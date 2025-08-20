# Security Policy

## Overview

Synapsed Crypto implements post-quantum cryptographic algorithms following NIST standards. This document outlines security considerations, best practices, and our vulnerability disclosure process.

## Security Guarantees

### Algorithm Security

1. **ML-KEM (Kyber)**: NIST-standardized Module-Lattice-Based Key-Encapsulation Mechanism
   - Based on the hardness of the Module Learning With Errors (MLWE) problem
   - Provides IND-CCA2 security
   - Quantum-resistant with configurable security levels

2. **ML-DSA (Dilithium)**: NIST-standardized Module-Lattice-Based Digital Signature Algorithm
   - Based on the hardness of MLWE and Module-SIS problems
   - Provides EUF-CMA security
   - Strongly unforgeable signatures

### Implementation Security

#### Constant-Time Operations

Critical operations are implemented in constant time to prevent timing attacks:

```rust
// Example: Constant-time polynomial comparison
pub fn ct_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    
    diff == 0
}
```

#### Side-Channel Resistance

- **No secret-dependent branches**: Control flow doesn't depend on secret data
- **No secret-dependent memory access**: Array indices don't depend on secrets
- **Constant-time arithmetic**: Polynomial operations use constant-time algorithms
- **Cache-timing resistance**: NTT operations access memory in predictable patterns

## Security Best Practices

### 1. Random Number Generation

**ALWAYS** use a cryptographically secure random number generator:

```rust
use synapsed_crypto::random::OsRng;

// Good: OS-provided CSPRNG
let mut rng = OsRng::new();

// Bad: Never use for production
// let mut rng = TestRng::new(12345);
```

### 2. Key Management

#### Key Generation

- Generate keys in a secure environment
- Use appropriate security levels for your threat model
- Never reuse keys across different protocols

```rust
// Generate keys with appropriate security level
let security = SecurityLevel::High;
let alg = security.recommended_kem();
let (pk, sk) = generate_keypair(alg, &mut rng)?;
```

#### Key Storage

- Store secret keys encrypted at rest
- Use hardware security modules (HSM) when available
- Implement key rotation policies

```rust
// Example: Secure key storage pattern
#[cfg(feature = "zeroize")]
{
    let mut secret_key = keypair.secret_key;
    
    // Use the key
    let result = process_with_key(&secret_key);
    
    // Securely wipe from memory
    secret_key.zeroize();
}
```

#### Key Serialization

- Always validate deserialized keys
- Use authenticated encryption for key transport
- Check key sizes match expected values

```rust
// Safe key deserialization
fn deserialize_public_key(data: &[u8]) -> Result<PublicKey> {
    if data.len() != KemAlgorithm::Kyber768.public_key_size() {
        return Err(Error::InvalidKeySize);
    }
    
    PublicKey::from_bytes(data)
}
```

### 3. Protocol Design

#### Hybrid Modes

During the transition to post-quantum cryptography, use hybrid modes:

```rust
#[cfg(feature = "hybrid")]
use synapsed_crypto::hybrid::EcdhP256Kyber768;

// Combines classical ECDH with post-quantum Kyber
let (pk, sk) = EcdhP256Kyber768::generate_keypair(&mut rng)?;
```

#### Domain Separation

Use different keys for different purposes:

```rust
// Good: Separate keys for encryption and signing
let encrypt_keys = generate_keypair(KemAlgorithm::Kyber768, &mut rng)?;
let sign_keys = generate_signing_keypair(SignatureAlgorithm::Dilithium3, &mut rng)?;

// Bad: Don't derive signing keys from encryption keys
```

### 4. Error Handling

Never reveal information about secret data through error messages:

```rust
// Good: Generic error
pub fn decrypt(sk: &SecretKey, ct: &[u8]) -> Result<Vec<u8>> {
    validate_ciphertext(ct).map_err(|_| Error::DecryptionFailed)?;
    // ...
}

// Bad: Detailed error reveals information
pub fn bad_decrypt(sk: &SecretKey, ct: &[u8]) -> Result<Vec<u8>> {
    if ct[0] != expected_tag {
        return Err(Error::InvalidTag(ct[0])); // Don't do this!
    }
    // ...
}
```

## Common Vulnerabilities and Mitigations

### 1. Decryption Failures

**Vulnerability**: Decryption failure attacks can leak information about the secret key.

**Mitigation**: Our implementation uses implicit rejection to prevent these attacks:
- Failed decryptions return a pseudorandom value
- No information about failure reason is revealed

### 2. Side-Channel Attacks

**Vulnerability**: Timing, power, or electromagnetic emanations can leak secrets.

**Mitigations**:
- Constant-time implementations
- No secret-dependent branches
- Regular memory access patterns
- Consider additional hardware countermeasures

### 3. Fault Attacks

**Vulnerability**: Induced faults during computation can reveal secrets.

**Mitigations**:
- Validate all inputs and outputs
- Use redundant computations for critical operations
- Implement checksums on secret data

### 4. API Misuse

**Vulnerability**: Incorrect API usage can compromise security.

**Mitigations**:
- Safe defaults (e.g., recommended security levels)
- Clear documentation with examples
- Compile-time type safety
- Runtime parameter validation

## Cryptographic Agility

Design your systems to support algorithm changes:

```rust
// Good: Algorithm-agnostic design
pub struct CryptoConfig {
    kem_algorithm: KemAlgorithm,
    sig_algorithm: SignatureAlgorithm,
}

impl CryptoConfig {
    pub fn recommended() -> Self {
        Self {
            kem_algorithm: SecurityLevel::High.recommended_kem(),
            sig_algorithm: SecurityLevel::High.recommended_signature(),
        }
    }
    
    pub fn migrate_to_stronger(&mut self) {
        // Easy migration path
        self.kem_algorithm = KemAlgorithm::Kyber1024;
        self.sig_algorithm = SignatureAlgorithm::Dilithium5;
    }
}
```

## Security Levels and Recommendations

| Use Case | KEM | Signature | Rationale |
|----------|-----|-----------|-----------|
| Web Services | Kyber768 | Dilithium3 | Balance of security and performance |
| Financial Systems | Kyber1024 | Dilithium5 | Maximum security for high-value targets |
| IoT Devices | Kyber512 | Dilithium2 | Reduced key sizes for constrained devices |
| Long-term Archives | Kyber1024 | Dilithium5 | Future-proof against quantum advances |

## Testing and Validation

### Test Vectors

All implementations are validated against NIST test vectors:

```bash
cargo test --features test-vectors
```

### Fuzzing

Regular fuzzing helps find edge cases:

```bash
cargo fuzz run kem_fuzzer
cargo fuzz run signature_fuzzer
```

### Formal Verification

Key properties that should be verified:
- Constant-time execution
- Memory safety
- Functional correctness
- Side-channel resistance

## Vulnerability Disclosure

### Reporting Security Issues

**DO NOT** open public issues for security vulnerabilities.

Instead, please email security@synapsed.io with:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fixes (if any)

### Our Commitment

- Acknowledge receipt within 48 hours
- Provide regular updates on our progress
- Credit researchers in our security advisories
- Coordinate disclosure timeline

### PGP Key

For encrypted communications, use our PGP key:

```
-----BEGIN PGP PUBLIC KEY BLOCK-----

[PGP key would be inserted here]

-----END PGP PUBLIC KEY BLOCK-----
```

## Security Checklist

Before deploying Synapsed Crypto:

- [ ] Using latest version of the library
- [ ] Cryptographically secure RNG configured
- [ ] Appropriate security levels selected
- [ ] Key management procedures in place
- [ ] Error handling doesn't leak information
- [ ] Hybrid mode considered for transition period
- [ ] Monitoring for security advisories
- [ ] Incident response plan prepared

## References

1. [NIST Post-Quantum Cryptography](https://csrc.nist.gov/projects/post-quantum-cryptography)
2. [FIPS 203: Module-Lattice-Based Key-Encapsulation Mechanism](https://csrc.nist.gov/publications/detail/fips/203/final)
3. [FIPS 204: Module-Lattice-Based Digital Signature Algorithm](https://csrc.nist.gov/publications/detail/fips/204/final)
4. [Kyber Specification](https://pq-crystals.org/kyber/)
5. [Dilithium Specification](https://pq-crystals.org/dilithium/)

## Contact

- **Security Issues**: security@synapsed.io
- **General Questions**: [GitHub Discussions](https://github.com/synapsed/synapsed-crypto/discussions)
- **Updates**: [Security Advisories](https://github.com/synapsed/synapsed-crypto/security/advisories)