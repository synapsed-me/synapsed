# Synapsed Crypto Design Document

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Algorithm Implementation](#algorithm-implementation)
4. [Performance Optimizations](#performance-optimizations)
5. [Security Design](#security-design)
6. [API Design Philosophy](#api-design-philosophy)
7. [Testing Strategy](#testing-strategy)
8. [Future Considerations](#future-considerations)

## Overview

Synapsed Crypto is a post-quantum cryptography library implementing NIST-standardized ML-KEM (Kyber) and ML-DSA (Dilithium) algorithms. The design prioritizes:

- **Security**: Constant-time operations, side-channel resistance
- **Performance**: Optimized NTT, parallel operations where applicable
- **Usability**: Simple API, safe defaults, clear documentation
- **Portability**: Pure Rust, no_std support, WASM compatibility

## Architecture

### Module Structure

```
synapsed-crypto/
├── src/
│   ├── lib.rs           # Library root, public API
│   ├── api.rs           # High-level convenience API
│   ├── traits.rs        # Core trait definitions
│   ├── error.rs         # Error types and handling
│   ├── params.rs        # Algorithm parameters
│   │
│   ├── kyber/           # ML-KEM implementation
│   │   ├── mod.rs       # Kyber module interface
│   │   ├── kyber512.rs  # Level 1 security
│   │   ├── kyber768.rs  # Level 3 security
│   │   └── kyber1024.rs # Level 5 security
│   │
│   ├── dilithium/       # ML-DSA implementation
│   │   ├── mod.rs       # Dilithium module interface
│   │   ├── dilithium2.rs # Level 2 security
│   │   ├── dilithium3.rs # Level 3 security
│   │   └── dilithium5.rs # Level 5 security
│   │
│   ├── poly.rs          # Polynomial arithmetic
│   ├── ntt.rs           # Number Theoretic Transform
│   ├── hash.rs          # Hash functions (SHA3, SHAKE)
│   ├── random.rs        # RNG implementations
│   └── utils.rs         # Utility functions
```

### Core Traits

```rust
/// Key Encapsulation Mechanism trait
pub trait Kem {
    type PublicKey: Serializable;
    type SecretKey: Serializable;
    type Ciphertext: Serializable;
    type SharedSecret: AsRef<[u8]>;
    
    fn generate_keypair<R: SecureRandom>(rng: &mut R) 
        -> Result<(Self::PublicKey, Self::SecretKey)>;
    
    fn encapsulate<R: SecureRandom>(
        pk: &Self::PublicKey, 
        rng: &mut R
    ) -> Result<(Self::Ciphertext, Self::SharedSecret)>;
    
    fn decapsulate(
        sk: &Self::SecretKey, 
        ct: &Self::Ciphertext
    ) -> Result<Self::SharedSecret>;
}

/// Digital Signature trait
pub trait Signature {
    type PublicKey: Serializable;
    type SecretKey: Serializable;
    type Sig: Serializable;
    
    fn generate_keypair<R: SecureRandom>(rng: &mut R) 
        -> Result<(Self::PublicKey, Self::SecretKey)>;
    
    fn sign<R: SecureRandom>(
        sk: &Self::SecretKey,
        msg: &[u8],
        rng: &mut R
    ) -> Result<Self::Sig>;
    
    fn verify(
        pk: &Self::PublicKey,
        msg: &[u8],
        sig: &Self::Sig
    ) -> Result<bool>;
}
```

### Design Principles

1. **Type Safety**: Algorithm parameters encoded in types
2. **Zero-Copy**: Minimize allocations, use stack arrays where possible
3. **Modularity**: Each algorithm variant is independent
4. **Testability**: Every component is unit-testable
5. **No Unsafe**: Pure safe Rust implementation

## Algorithm Implementation

### ML-KEM (Kyber)

Kyber is a module-lattice-based KEM built on the hardness of the Module-LWE problem.

#### Key Components

1. **Polynomial Rings**: R_q = Z_q[X]/(X^256 + 1)
2. **NTT Operations**: Fast polynomial multiplication
3. **Compression**: Reduce ciphertext size
4. **CBD Sampling**: Centered Binomial Distribution

#### Implementation Details

```rust
// Kyber polynomial representation
#[derive(Clone)]
pub struct Poly {
    coeffs: [i16; 256],  // Coefficients in Z_q
}

// Module structure (vector of polynomials)
pub struct PolyVec<const K: usize> {
    vec: [Poly; K],
}

// Matrix of polynomials
pub struct PolyMat<const K: usize> {
    mat: [[Poly; K]; K],
}
```

#### Key Generation

```
1. Sample A ← R_q^{k×k} from seed (using SHAKE128)
2. Sample (s, e) ← CBD^k × CBD^k
3. Compute t = As + e
4. Return pk = (t, seed), sk = s
```

#### Encapsulation

```
1. Hash message m to get (K̄, r)
2. Sample (r', e1, e2) using r
3. Compute u = A^T r' + e1
4. Compute v = t^T r' + e2 + Decompress(m)
5. Return ct = (u, v), ss = K̄
```

### ML-DSA (Dilithium)

Dilithium is a module-lattice-based signature scheme based on the Fiat-Shamir with Aborts paradigm.

#### Key Components

1. **Polynomial Rings**: R_q = Z_q[X]/(X^256 + 1)
2. **Rejection Sampling**: Ensure signature doesn't leak secret
3. **Hint Mechanism**: Reduce signature size
4. **Deterministic Signing**: Option for reproducible signatures

#### Implementation Details

```rust
// Dilithium-specific polynomial operations
impl Poly {
    // Power of 2 rounding
    pub fn power2round(&self, d: u32) -> (Poly, Poly) {
        // Split into high and low parts
    }
    
    // Hint generation for size reduction
    pub fn make_hint(&self, other: &Poly) -> Poly {
        // Generate hints for signature compression
    }
}
```

#### Signing Process

```
1. Expand matrix A from seed
2. Sample y from large domain
3. Compute w = Ay
4. Create challenge c = H(μ || w1)
5. Compute z = y + cs
6. Rejection sampling: restart if ||z|| too large
7. Create hints for w - cs2
8. Return signature (z, h, c)
```

## Performance Optimizations

### 1. Number Theoretic Transform (NTT)

The NTT is critical for performance, enabling O(n log n) polynomial multiplication.

```rust
// Optimized NTT implementation
pub fn ntt(poly: &mut Poly) {
    let mut k = 1;
    let mut len = 128;
    
    while len >= 2 {
        for start in (0..256).step_by(2 * len) {
            let zeta = ZETAS[k];
            k += 1;
            
            for j in start..start + len {
                let t = montgomery_reduce(zeta as i32 * poly.coeffs[j + len] as i32);
                poly.coeffs[j + len] = poly.coeffs[j] - t;
                poly.coeffs[j] = poly.coeffs[j] + t;
            }
        }
        len >>= 1;
    }
}
```

### 2. Montgomery Arithmetic

Use Montgomery form for efficient modular arithmetic:

```rust
const MONT: i32 = 2285; // 2^16 mod q
const QINV: i32 = 62209; // q^{-1} mod 2^16

fn montgomery_reduce(a: i32) -> i16 {
    let t = (a as i16).wrapping_mul(QINV as i16) as i32;
    ((a - t * Q) >> 16) as i16
}
```

### 3. Vectorization Opportunities

Where available, use SIMD for parallel operations:

```rust
#[cfg(target_feature = "avx2")]
fn poly_add_avx2(a: &Poly, b: &Poly) -> Poly {
    // AVX2 implementation for x86_64
}

#[cfg(not(target_feature = "avx2"))]
fn poly_add_scalar(a: &Poly, b: &Poly) -> Poly {
    // Scalar fallback
}
```

### 4. Memory Layout

Optimize cache usage with careful data layout:

```rust
// Good: Sequential memory access
#[repr(C, align(32))]
pub struct AlignedPoly {
    coeffs: [i16; 256],
}

// Minimize cache misses in matrix operations
pub fn matrix_mult<const K: usize>(
    result: &mut PolyVec<K>,
    matrix: &PolyMat<K>,
    vector: &PolyVec<K>,
) {
    // Process in cache-friendly order
}
```

## Security Design

### Constant-Time Operations

All secret-dependent operations must be constant-time:

```rust
/// Constant-time polynomial comparison
pub fn ct_eq(a: &Poly, b: &Poly) -> bool {
    let mut diff = 0u16;
    for i in 0..256 {
        diff |= (a.coeffs[i] ^ b.coeffs[i]) as u16;
    }
    diff == 0
}

/// Constant-time conditional move
pub fn ct_cmov(dest: &mut [u8], src: &[u8], condition: bool) {
    let mask = (condition as u8).wrapping_neg();
    for (d, s) in dest.iter_mut().zip(src.iter()) {
        *d ^= mask & (*d ^ *s);
    }
}
```

### Side-Channel Mitigations

1. **No Secret-Dependent Branches**
   ```rust
   // Bad: Leaks information through timing
   if secret_bit == 1 {
       do_something();
   }
   
   // Good: Constant-time selection
   let result = ct_select(option_a, option_b, secret_bit);
   ```

2. **No Secret-Dependent Memory Access**
   ```rust
   // Bad: Cache timing reveals index
   let value = table[secret_index];
   
   // Good: Access all elements
   let mut value = 0;
   for (i, &elem) in table.iter().enumerate() {
       value |= ct_eq(i, secret_index) as u8 * elem;
   }
   ```

3. **Protect Against Fault Attacks**
   ```rust
   // Verify critical operations
   let shared_secret = decapsulate(sk, ct)?;
   let verification = encapsulate_deterministic(pk, shared_secret)?;
   if !ct_eq(&verification, ct) {
       return Err(Error::FaultDetected);
   }
   ```

### Randomness Security

```rust
pub trait SecureRandom {
    fn fill_bytes(&mut self, dest: &mut [u8]);
    
    /// Ensure non-zero output for nonce generation
    fn fill_bytes_non_zero(&mut self, dest: &mut [u8]) {
        loop {
            self.fill_bytes(dest);
            if !dest.iter().all(|&b| b == 0) {
                break;
            }
        }
    }
}
```

## API Design Philosophy

### 1. Progressive Disclosure

Simple things should be simple, complex things should be possible:

```rust
// Level 1: Dead simple
let (pk, sk) = generate_keypair(KemAlgorithm::Kyber768, &mut rng)?;

// Level 2: More control
let keypair = KeyPair::generate(Algorithm::Kem(KemAlgorithm::Kyber768), &mut rng)?;

// Level 3: Full control
let kyber = Kyber768::new();
let (pk, sk) = kyber.generate_keypair_with_seed(&seed)?;
```

### 2. Safe Defaults

```rust
impl SecurityLevel {
    /// Recommended for most applications
    pub fn default() -> Self {
        SecurityLevel::High // Kyber768/Dilithium3
    }
}
```

### 3. Explicit Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid key size: expected {expected}, got {actual}")]
    InvalidKeySize { expected: usize, actual: usize },
    
    #[error("Decryption failed")]
    DecryptionFailed, // Generic to avoid leaking information
    
    #[error("RNG failure")]
    RngError(#[from] RngError),
}
```

### 4. Type Safety

Use the type system to prevent misuse:

```rust
// Can't mix algorithms at compile time
impl Kyber768 {
    pub fn encapsulate(
        pk: &Kyber768PublicKey, // Type-specific
        rng: &mut impl SecureRandom
    ) -> Result<(Kyber768Ciphertext, SharedSecret)> {
        // Implementation
    }
}
```

## Testing Strategy

### 1. Unit Tests

Every component has comprehensive unit tests:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_ntt_inverse() {
        let mut poly = random_poly();
        let original = poly.clone();
        
        ntt(&mut poly);
        inv_ntt(&mut poly);
        
        assert_eq!(poly, original);
    }
}
```

### 2. Known Answer Tests (KATs)

Validate against NIST test vectors:

```rust
#[test]
fn test_kyber768_nist_vectors() {
    for vector in load_nist_vectors("kyber768") {
        let (pk, sk) = generate_keypair_deterministic(&vector.seed);
        assert_eq!(pk.as_bytes(), &vector.expected_pk);
        assert_eq!(sk.as_bytes(), &vector.expected_sk);
    }
}
```

### 3. Property-Based Testing

Use proptest for invariant checking:

```rust
proptest! {
    #[test]
    fn test_kem_correctness(seed: [u8; 32]) {
        let mut rng = TestRng::from_seed(seed);
        let (pk, sk) = generate_keypair(KemAlgorithm::Kyber768, &mut rng)?;
        let (ct, ss1) = encapsulate(KemAlgorithm::Kyber768, &pk, &mut rng)?;
        let ss2 = decapsulate(KemAlgorithm::Kyber768, &sk, &ct)?;
        prop_assert_eq!(ss1, ss2);
    }
}
```

### 4. Fuzzing

Continuous fuzzing for edge cases:

```rust
#[cfg(fuzzing)]
fuzz_target!(|data: &[u8]| {
    if let Ok(pk) = PublicKey::from_bytes(data) {
        let _ = validate_public_key(&pk);
    }
});
```

### 5. Benchmarking

Track performance across changes:

```rust
#[bench]
fn bench_kyber768_keygen(b: &mut Bencher) {
    let mut rng = OsRng::new();
    b.iter(|| {
        generate_keypair(KemAlgorithm::Kyber768, &mut rng)
    });
}
```

## Future Considerations

### 1. Algorithm Agility

Design allows easy addition of new algorithms:

```rust
// Future: Add NTRU Prime
pub enum KemAlgorithm {
    Kyber512,
    Kyber768,
    Kyber1024,
    #[cfg(feature = "experimental")]
    NtruPrime,
}
```

### 2. Hardware Acceleration

Structure supports future hardware optimizations:

```rust
// Future: Hardware security module support
pub trait HardwareKem {
    fn generate_keypair_hsm(&mut self, slot: u32) -> Result<KeyHandle>;
    fn encapsulate_hsm(&mut self, key: KeyHandle) -> Result<Vec<u8>>;
}
```

### 3. Batch Operations

API extensible for batch processing:

```rust
// Future: Batch verification for efficiency
pub fn verify_batch(
    messages: &[(&[u8], &PublicKey, &Signature)],
) -> Result<Vec<bool>> {
    // Optimized batch verification
}
```

### 4. Threshold Cryptography

Foundation for advanced protocols:

```rust
// Future: Threshold signatures
pub trait ThresholdSignature: Signature {
    fn generate_shares(n: usize, t: usize) -> Vec<Share>;
    fn sign_share(share: &Share, msg: &[u8]) -> PartialSig;
    fn combine_signatures(sigs: &[PartialSig]) -> Self::Sig;
}
```

## Conclusion

Synapsed Crypto's design balances security, performance, and usability. The architecture supports:

- Safe, efficient implementations of post-quantum algorithms
- Protection against implementation attacks
- Easy integration into existing systems
- Future extensibility for new algorithms and features

The modular design ensures each component can be independently verified, tested, and optimized while maintaining the security properties of the overall system.