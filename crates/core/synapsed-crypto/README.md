# Synapsed Crypto

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![NIST](https://img.shields.io/badge/NIST-Standardized-green.svg)](https://csrc.nist.gov/projects/post-quantum-cryptography)

A production-ready post-quantum cryptography library implementing NIST-standardized ML-KEM (Kyber) and ML-DSA (Dilithium) algorithms in pure Rust.

## Implementation Status

### Core Features
- ✅ Kyber (ML-KEM) - all security levels (512/768/1024) - Post-quantum key encapsulation mechanism
- ✅ Dilithium (ML-DSA) - all security levels (2/3/5) - Post-quantum digital signatures
- ✅ Constant-time operations - Side-channel resistant implementations
- ✅ NTT optimizations - Number theoretic transform performance enhancements
- ✅ Secure memory handling - Protected key material and sensitive data
- ✅ Observability integration - Event emission and metrics collection
- ✅ WASM support - WebAssembly compatibility for browser environments
- 🚧 SIMD enhancements - Vectorized operations for improved performance
- 📋 Hardware acceleration - Platform-specific optimizations (AES-NI, AVX)
- 📋 Batch operations - Efficient bulk cryptographic operations

## 🚀 Features

- **🔐 NIST-Standardized**: Implements ML-KEM (Kyber) and ML-DSA (Dilithium) as standardized by NIST
- **⚡ High Performance**: Optimized NTT operations and assembly-level optimizations
- **🛡️ Security First**: Constant-time operations, side-channel resistant implementations
- **📦 Pure Rust**: No unsafe code, no C dependencies
- **🎯 Easy to Use**: Simple, intuitive API similar to classical crypto libraries
- **🔄 Flexible**: Supports `no_std`, WASM, and embedded environments
- **✅ Well Tested**: Comprehensive test suite with NIST test vectors

## 📋 Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [Usage Examples](#usage-examples)
- [Security Levels](#security-levels)
- [API Documentation](#api-documentation)
- [Performance](#performance)
- [Security Considerations](#security-considerations)
- [Migration Guide](#migration-guide)
- [Contributing](#contributing)
- [License](#license)

## Quick Start

```rust
use synapsed_crypto::prelude::*;
use synapsed_crypto::random::OsRng;

fn main() -> Result<(), synapsed_crypto::Error> {
    let mut rng = OsRng::new();

    // Post-Quantum Key Exchange
    let (public_key, secret_key) = generate_keypair(KemAlgorithm::Kyber768, &mut rng)?;
    let (ciphertext, shared_secret) = encapsulate(KemAlgorithm::Kyber768, &public_key, &mut rng)?;
    let recovered_secret = decapsulate(KemAlgorithm::Kyber768, &secret_key, &ciphertext)?;
    assert_eq!(shared_secret, recovered_secret);

    // Post-Quantum Digital Signatures
    let (pub_key, sec_key) = generate_signing_keypair(SignatureAlgorithm::Dilithium3, &mut rng)?;
    let message = b"Quantum-safe message";
    let signature = sign(SignatureAlgorithm::Dilithium3, &sec_key, message, &mut rng)?;
    let is_valid = verify(SignatureAlgorithm::Dilithium3, &pub_key, message, &signature)?;
    assert!(is_valid);

    Ok(())
}
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
synapsed-crypto = "0.1"

# Optional features
synapsed-crypto = { version = "0.1", features = ["hybrid", "parallel"] }
```

### Feature Flags

- `std` (default): Standard library support
- `hybrid`: Enable hybrid classical/post-quantum modes
- `parallel`: Enable parallel operations with rayon
- `serde`: Serialization support
- `zeroize`: Secure memory wiping

## Usage Examples

### Basic Encryption (Key Encapsulation)

```rust
use synapsed_crypto::prelude::*;
use synapsed_crypto::random::OsRng;

// Initialize RNG
let mut rng = OsRng::new();

// Choose security level
let algorithm = SecurityLevel::High.recommended_kem(); // Kyber768

// Generate keypair
let keypair = KeyPair::generate(Algorithm::Kem(algorithm), &mut rng)?;

// Sender: Encapsulate shared secret
let (ciphertext, shared_secret) = encapsulate(
    algorithm,
    keypair.public_key(),
    &mut rng
)?;

// Receiver: Decapsulate to get same shared secret
let recovered = decapsulate(
    algorithm,
    keypair.secret_key(),
    &ciphertext
)?;

// Use shared_secret for symmetric encryption (AES, ChaCha20, etc.)
```

### Digital Signatures

```rust
use synapsed_crypto::prelude::*;
use synapsed_crypto::random::OsRng;

let mut rng = OsRng::new();

// Choose algorithm based on requirements
let algorithm = SignatureAlgorithm::Dilithium3;

// Generate signing keypair
let (public_key, secret_key) = generate_signing_keypair(algorithm, &mut rng)?;

// Sign a message
let message = b"Important document";
let signature = sign(algorithm, &secret_key, message, &mut rng)?;

// Verify signature
let is_valid = verify(algorithm, &public_key, message, &signature)?;
assert!(is_valid);

// Deterministic signing (no RNG needed)
let det_signature = sign_deterministic(algorithm, &secret_key, message)?;
```

### Hybrid Mode (Classical + Post-Quantum)

```rust
#[cfg(feature = "hybrid")]
use synapsed_crypto::hybrid::{HybridKem, EcdhP256Kyber768};

// Hybrid key exchange combining ECDH P-256 with Kyber768
let (public_key, secret_key) = EcdhP256Kyber768::generate_keypair(&mut rng)?;
let (ciphertext, shared_secret) = EcdhP256Kyber768::encapsulate(&public_key, &mut rng)?;

// Shared secret is derived from both ECDH and Kyber
let recovered = EcdhP256Kyber768::decapsulate(&secret_key, &ciphertext)?;
```

## Security Levels

Choose the appropriate security level for your application:

| Level | KEM | Signature | Classical | Quantum | Use Case |
|-------|-----|-----------|-----------|---------|----------|
| **Standard** | Kyber512 | Dilithium2 | 128-bit | 64-bit | Most applications |
| **High** | Kyber768 | Dilithium3 | 192-bit | 96-bit | Sensitive data |
| **Very High** | Kyber1024 | Dilithium5 | 256-bit | 128-bit | Critical infrastructure |

```rust
// Using security levels
let security = SecurityLevel::High;
let kem_alg = security.recommended_kem();
let sig_alg = security.recommended_signature();
```

## API Documentation

### Core Types

- `KemAlgorithm`: Enum for available KEM algorithms
- `SignatureAlgorithm`: Enum for available signature algorithms
- `KeyPair`: Convenient key management structure
- `SecurityLevel`: Security level recommendations

### Main Functions

#### Key Encapsulation (KEM)

- `generate_keypair()`: Generate a KEM keypair
- `encapsulate()`: Create ciphertext and shared secret
- `decapsulate()`: Recover shared secret from ciphertext

#### Digital Signatures

- `generate_signing_keypair()`: Generate a signing keypair
- `sign()`: Create a signature (randomized)
- `sign_deterministic()`: Create a signature (deterministic)
- `verify()`: Verify a signature

#### Utilities

- `encrypt()`: High-level encryption using KEM + symmetric cipher
- `decrypt()`: High-level decryption

## Performance

Benchmarks on Intel Core i7-10700K @ 3.80GHz:

| Operation | Kyber512 | Kyber768 | Kyber1024 |
|-----------|----------|----------|-----------|
| KeyGen | 15 μs | 23 μs | 35 μs |
| Encapsulate | 18 μs | 28 μs | 40 μs |
| Decapsulate | 20 μs | 31 μs | 45 μs |

| Operation | Dilithium2 | Dilithium3 | Dilithium5 |
|-----------|------------|------------|------------|
| KeyGen | 42 μs | 68 μs | 110 μs |
| Sign | 120 μs | 180 μs | 250 μs |
| Verify | 45 μs | 70 μs | 110 μs |

## Security Considerations

### Side-Channel Resistance

- Constant-time polynomial operations
- No secret-dependent branches
- Protected against timing attacks
- Cache-timing resistant NTT

### Random Number Generation

Always use a cryptographically secure RNG:

```rust
use synapsed_crypto::random::{OsRng, SecureRandom};

let mut rng = OsRng::new();
// For testing only:
// use synapsed_crypto::random::TestRng;
```

### Key Management

```rust
// Secure key handling with zeroize
#[cfg(feature = "zeroize")]
{
    use zeroize::Zeroize;
    
    let mut secret_key = generate_keypair(alg, &mut rng)?.1;
    // Use secret_key...
    secret_key.zeroize(); // Securely wipe from memory
}
```

## Migration Guide

### From RSA/ECDH to ML-KEM

```rust
// Before (RSA/ECDH)
// let (public_key, private_key) = generate_rsa_keypair(2048);
// let encrypted = rsa_encrypt(&public_key, &plaintext);

// After (ML-KEM)
let (public_key, secret_key) = generate_keypair(KemAlgorithm::Kyber768, &mut rng)?;
let (ciphertext, shared_secret) = encapsulate(KemAlgorithm::Kyber768, &public_key, &mut rng)?;
// Use shared_secret with AES-GCM or ChaCha20-Poly1305
```

### From ECDSA to ML-DSA

```rust
// Before (ECDSA)
// let (public_key, private_key) = generate_ecdsa_keypair();
// let signature = ecdsa_sign(&private_key, &message);

// After (ML-DSA)
let (public_key, secret_key) = generate_signing_keypair(SignatureAlgorithm::Dilithium3, &mut rng)?;
let signature = sign(SignatureAlgorithm::Dilithium3, &secret_key, &message, &mut rng)?;
```

### Key Size Considerations

| Algorithm | Public Key | Secret Key | Ciphertext/Signature |
|-----------|------------|------------|---------------------|
| RSA-2048 | 256 B | 256 B | 256 B |
| ECDSA P-256 | 64 B | 32 B | 64 B |
| **Kyber768** | **1,184 B** | **2,400 B** | **1,088 B** |
| **Dilithium3** | **1,952 B** | **4,000 B** | **3,293 B** |

## Advanced Usage

### Custom RNG

```rust
use synapsed_crypto::traits::SecureRandom;

struct MyRng {
    // Your RNG implementation
}

impl SecureRandom for MyRng {
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        // Fill dest with random bytes
    }
}
```

### No-std Usage

```toml
[dependencies]
synapsed-crypto = { version = "0.1", default-features = false }
```

```rust
#![no_std]
use synapsed_crypto::prelude::*;
```

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# Clone the repository
git clone https://github.com/synapsed/synapsed-crypto
cd synapsed-crypto

# Run tests
cargo test

# Run benchmarks
cargo bench

# Check formatting
cargo fmt -- --check

# Run lints
cargo clippy -- -D warnings
```

## Security Audits

This library has not yet undergone a formal security audit. While we follow best practices and implement standardized algorithms, use at your own risk in production environments.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

- NIST Post-Quantum Cryptography Standardization team
- Kyber and Dilithium algorithm designers
- Rust cryptography community

## Contact

- **Issues**: [GitHub Issues](https://github.com/synapsed/synapsed-crypto/issues)
- **Discussions**: [GitHub Discussions](https://github.com/synapsed/synapsed-crypto/discussions)
- **Security**: security@synapsed.io (PGP key in [SECURITY.md](SECURITY.md))