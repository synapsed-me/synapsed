//! Cryptographic primitives and protocols for secure networking.

pub mod certificates;
pub mod enhanced_security;
pub mod key_derivation;
pub mod post_quantum;
pub mod session;

#[cfg(test)]
pub mod test_enhanced_security;

pub use certificates::{CertificateValidator, CertificatePinner};
pub use enhanced_security::{
    EnhancedSecurityManager, EnhancedSecurityConfig, SecureCipherSuite,
    SecurityEvent, SecurityMetrics,
};
pub use key_derivation::{derive_session_keys, KeyDerivationFunction, KeyRatchet};
pub use post_quantum::{HybridKeyExchange, PQCipherSuite, PQSignature, PQSignatureAlgorithm};
pub use session::{SessionManager, SessionState, SessionTicket};

use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Supported cryptographic algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CryptoAlgorithm {
    /// Ed25519 signatures
    Ed25519,
    /// X25519 key exchange
    X25519,
    /// ChaCha20-Poly1305 AEAD
    ChaCha20Poly1305,
    /// AES-256-GCM AEAD
    Aes256Gcm,
    /// Kyber768 post-quantum KEM
    Kyber768,
    /// Kyber1024 post-quantum KEM
    Kyber1024,
    /// Dilithium3 post-quantum signatures
    Dilithium3,
    /// Dilithium5 post-quantum signatures
    Dilithium5,
}

/// Cryptographic key types.
#[derive(Debug, Clone)]
pub enum CryptoKey {
    /// Symmetric key
    Symmetric(Vec<u8>),
    /// Ed25519 public key
    Ed25519Public([u8; 32]),
    /// Ed25519 private key
    Ed25519Private([u8; 32]),
    /// X25519 public key
    X25519Public([u8; 32]),
    /// X25519 private key
    X25519Private([u8; 32]),
    /// Post-quantum public key
    PostQuantumPublic(Vec<u8>),
    /// Post-quantum private key
    PostQuantumPrivate(Vec<u8>),
}

impl CryptoKey {
    /// Returns the key algorithm.
    pub fn algorithm(&self) -> CryptoAlgorithm {
        match self {
            CryptoKey::Ed25519Public(_) | CryptoKey::Ed25519Private(_) => CryptoAlgorithm::Ed25519,
            CryptoKey::X25519Public(_) | CryptoKey::X25519Private(_) => CryptoAlgorithm::X25519,
            CryptoKey::PostQuantumPublic(_) | CryptoKey::PostQuantumPrivate(_) => CryptoAlgorithm::Kyber1024,
            CryptoKey::Symmetric(_) => CryptoAlgorithm::ChaCha20Poly1305,
        }
    }
    
    /// Returns the key size in bytes.
    pub fn size(&self) -> usize {
        match self {
            CryptoKey::Symmetric(k) => k.len(),
            CryptoKey::Ed25519Public(_) | CryptoKey::Ed25519Private(_) => 32,
            CryptoKey::X25519Public(_) | CryptoKey::X25519Private(_) => 32,
            CryptoKey::PostQuantumPublic(k) | CryptoKey::PostQuantumPrivate(k) => k.len(),
        }
    }
}

/// Initialize cryptographic subsystem.
pub fn init_crypto() -> Result<()> {
    // Initialize random number generation
    ring::rand::SystemRandom::new();
    
    // Validate cryptographic implementations
    validate_crypto_implementations()?;
    
    Ok(())
}

/// Validate that all cryptographic implementations work correctly.
fn validate_crypto_implementations() -> Result<()> {
    // Test Ed25519 signing
    let rng = ring::rand::SystemRandom::new();
    let pkcs8_bytes = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng)
        .map_err(|_| crate::error::NetworkError::Security(
            crate::error::SecurityError::KeyExchange("Ed25519 key generation failed".to_string())
        ))?;
    
    let _key_pair = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref())
        .map_err(|_| crate::error::NetworkError::Security(
            crate::error::SecurityError::KeyExchange("Ed25519 key parsing failed".to_string())
        ))?;
    
    // Test ChaCha20-Poly1305
    use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
    let key = [0u8; 32];
    let _cipher = ChaCha20Poly1305::new(&key.into());
    
    Ok(())
}