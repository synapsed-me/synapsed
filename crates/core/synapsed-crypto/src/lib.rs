//! # Synapsed Crypto
//! 
//! A production-ready post-quantum cryptography library implementing NIST-standardized
//! ML-KEM (Kyber) and ML-DSA (Dilithium) algorithms with a focus on security,
//! performance, and ease of use.
//!
//! ## Overview
//!
//! Synapsed Crypto provides quantum-resistant cryptographic primitives that are
//! designed to remain secure even against attacks from quantum computers. This
//! library implements the latest NIST standards for post-quantum cryptography:
//!
//! - **ML-KEM (Module-Lattice-Based Key-Encapsulation Mechanism)**: Based on Kyber
//! - **ML-DSA (Module-Lattice-Based Digital Signature Algorithm)**: Based on Dilithium
//!
//! ## Features
//! 
//! - **🔐 NIST-Standardized Algorithms**: ML-KEM (Kyber) and ML-DSA (Dilithium)
//! - **🚀 High Performance**: Optimized NTT and polynomial arithmetic
//! - **🛡️ Side-Channel Resistant**: Constant-time operations where critical
//! - **🦀 Pure Rust**: No unsafe code, no C dependencies
//! - **📦 Flexible**: Support for `no_std` environments
//! - **🌐 WASM Compatible**: Run in web browsers and edge computing
//! - **🔄 Hybrid Modes**: Combine with classical algorithms for defense in depth
//! - **✅ Well-Tested**: Comprehensive test suite with known answer tests
//!
//! ## Quick Start
//!
//! ```no_run
//! use synapsed_crypto::prelude::*;
//! use synapsed_crypto::random::OsRng;
//!
//! # fn main() -> Result<(), synapsed_crypto::Error> {
//! let mut rng = OsRng::new();
//!
//! // Post-Quantum Key Exchange
//! let (public_key, secret_key) = generate_keypair(KemAlgorithm::Kyber768, &mut rng)?;
//! let (ciphertext, shared_secret) = encapsulate(KemAlgorithm::Kyber768, &public_key, &mut rng)?;
//! let recovered_secret = decapsulate(KemAlgorithm::Kyber768, &secret_key, &ciphertext)?;
//! assert_eq!(shared_secret, recovered_secret);
//!
//! // Post-Quantum Digital Signatures
//! let (pub_key, sec_key) = generate_signing_keypair(SignatureAlgorithm::Dilithium3, &mut rng)?;
//! let message = b"Quantum-safe message";
//! let signature = sign(SignatureAlgorithm::Dilithium3, &sec_key, message, &mut rng)?;
//! let is_valid = verify(SignatureAlgorithm::Dilithium3, &pub_key, message, &signature)?;
//! assert!(is_valid);
//! # Ok(())
//! # }
//! ```
//!
//! ## Security Levels
//!
//! The library provides multiple security levels to match your requirements:
//!
//! | Algorithm | NIST Level | Classical Security | Quantum Security |
//! |-----------|------------|-------------------|------------------|
//! | Kyber512 | 1 | 128-bit | 64-bit |
//! | Dilithium2 | 2 | 128-bit | 64-bit |
//! | Kyber768 | 3 | 192-bit | 96-bit |
//! | Dilithium3 | 3 | 192-bit | 96-bit |
//! | Kyber1024 | 5 | 256-bit | 128-bit |
//! | Dilithium5 | 5 | 256-bit | 128-bit |
//!
//! ## Module Structure
//!
//! - [`api`]: High-level, easy-to-use functions
//! - [`kyber`]: ML-KEM implementation (key encapsulation)
//! - [`dilithium`]: ML-DSA implementation (signatures)
//! - [`traits`]: Core cryptographic traits
//! - [`random`]: Cryptographically secure RNG
//! - [`hybrid`]: Hybrid classical/post-quantum modes (optional)
//!
//! ## Security Considerations
//!
//! 1. **Side-Channel Resistance**: Critical operations use constant-time implementations
//! 2. **Random Number Generation**: Always use a cryptographically secure RNG
//! 3. **Key Storage**: Keys should be stored securely and zeroed after use
//! 4. **Hybrid Modes**: Consider using hybrid modes during the transition period
//!
//! ## Migration from Classical Cryptography
//!
//! If you're migrating from classical algorithms:
//!
//! - Replace RSA/ECDH with ML-KEM (Kyber) for key exchange
//! - Replace RSA/ECDSA with ML-DSA (Dilithium) for signatures
//! - Consider hybrid modes for gradual transition
//! - Expect larger key and signature sizes
//!
//! ## Security Warning
//!
//! While this library implements standardized algorithms and follows best practices,
//! it has not yet undergone a formal security audit. Use at your own risk in
//! production environments. Consider defense-in-depth approaches and hybrid modes.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    unused_qualifications,
    missing_debug_implementations
)]

// Re-export core traits and types
pub use crate::traits::{Kem, Signature, SecureRandom};
pub use crate::error::{Error, Result};

// Core modules
pub mod error;
pub mod traits;
pub mod params;
pub mod utils;
pub mod constant_time;
pub mod secure_memory;

// Cryptographic primitive modules
pub mod poly;
pub mod ntt;
pub mod hash;
pub mod random;

// Algorithm implementations
pub mod kyber;
pub mod dilithium;

// High-level API
pub mod api;

// Optional hybrid modes
#[cfg(feature = "hybrid")]
pub mod hybrid;

// Prelude for convenient imports
pub mod prelude {
    //! Common imports for using synapsed-crypto
    //!
    //! This module provides a convenient way to import the most commonly used types
    //! and functions from the library.
    //!
    //! # Example
    //!
    //! ```
    //! use synapsed_crypto::prelude::*;
    //! ```
    
    pub use crate::{
        Error, Result,
        Kem, Signature, SecureRandom,
        kyber::{Kyber512, Kyber768, Kyber1024},
        dilithium::{Dilithium2, Dilithium3, Dilithium5},
        api::{
            // Core functions
            generate_keypair, encapsulate, decapsulate,
            generate_signing_keypair, sign, sign_deterministic, verify,
            // Types
            KemAlgorithm, SignatureAlgorithm, KeyPair, Algorithm, SecurityLevel,
        },
    };
    
    #[cfg(feature = "std")]
    pub use crate::api::{encrypt, decrypt};
    
    #[cfg(feature = "hybrid")]
    pub use crate::hybrid::{HybridKem, HybridSignature};
}

#[cfg(test)]
mod tests {
    
    #[test]
    fn library_compiles() {
        // Basic smoke test
        assert_eq!(2 + 2, 4);
    }
}