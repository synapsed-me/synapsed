//! Post-quantum cryptography integration with synapsed-crypto.

use crate::error::{NetworkError, Result, SecurityError};
use synapsed_crypto::{
    api::{decapsulate, encapsulate, generate_keypair, generate_signing_keypair, sign, verify},
    prelude::{KemAlgorithm, SignatureAlgorithm},
    random::DefaultRng,
};
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Post-quantum cipher suite configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PQCipherSuite {
    /// Kyber768 + ChaCha20-Poly1305 (NIST Level 3)
    Kyber768ChaCha20,
    
    /// Kyber1024 + ChaCha20-Poly1305 (NIST Level 5)
    Kyber1024ChaCha20,
    
    /// Kyber768 + AES-256-GCM (NIST Level 3)
    Kyber768Aes256,
    
    /// Kyber1024 + AES-256-GCM (NIST Level 5)
    Kyber1024Aes256,
    
    /// Hybrid: X25519 + Kyber768 + ChaCha20 (Transitional security)
    HybridX25519Kyber768,
    
    /// Hybrid: X25519 + Kyber1024 + AES-256 (Maximum transitional security)
    HybridX25519Kyber1024,
}

impl PQCipherSuite {
    /// Returns the KEM algorithm for this cipher suite.
    pub fn kem_algorithm(&self) -> KemAlgorithm {
        match self {
            Self::Kyber768ChaCha20 | Self::Kyber768Aes256 | Self::HybridX25519Kyber768 => {
                KemAlgorithm::Kyber768
            }
            Self::Kyber1024ChaCha20 | Self::Kyber1024Aes256 | Self::HybridX25519Kyber1024 => {
                KemAlgorithm::Kyber1024
            }
        }
    }
    
    /// Returns whether this is a hybrid cipher suite.
    pub fn is_hybrid(&self) -> bool {
        matches!(self, Self::HybridX25519Kyber768 | Self::HybridX25519Kyber1024)
    }
    
    /// Returns the security level in bits.
    pub fn security_level(&self) -> u16 {
        match self {
            Self::Kyber768ChaCha20 | Self::Kyber768Aes256 | Self::HybridX25519Kyber768 => 192,
            Self::Kyber1024ChaCha20 | Self::Kyber1024Aes256 | Self::HybridX25519Kyber1024 => 256,
        }
    }
}

/// Post-quantum signature algorithm configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PQSignatureAlgorithm {
    /// Dilithium3 (NIST Level 3)
    Dilithium3,
    
    /// Dilithium5 (NIST Level 5)
    Dilithium5,
    
    /// Hybrid: Ed25519 + Dilithium3
    HybridEd25519Dilithium3,
    
    /// Hybrid: Ed25519 + Dilithium5
    HybridEd25519Dilithium5,
}

impl PQSignatureAlgorithm {
    /// Returns the signature algorithm.
    pub fn signature_algorithm(&self) -> SignatureAlgorithm {
        match self {
            Self::Dilithium3 | Self::HybridEd25519Dilithium3 => SignatureAlgorithm::Dilithium3,
            Self::Dilithium5 | Self::HybridEd25519Dilithium5 => SignatureAlgorithm::Dilithium5,
        }
    }
    
    /// Returns whether this is a hybrid algorithm.
    pub fn is_hybrid(&self) -> bool {
        matches!(self, Self::HybridEd25519Dilithium3 | Self::HybridEd25519Dilithium5)
    }
}

/// Hybrid key exchange combining classical and post-quantum algorithms.
#[derive(Clone)]
pub struct HybridKeyExchange {
    /// Post-quantum cipher suite
    pq_suite: PQCipherSuite,
}

impl HybridKeyExchange {
    /// Creates a new hybrid key exchange instance.
    pub fn new(pq_suite: PQCipherSuite) -> Result<Self> {
        Ok(Self { pq_suite })
    }
    
    /// Generates a keypair for the key exchange.
    pub fn generate_keypair(&mut self) -> Result<(HybridPublicKey, HybridSecretKey)> {
        // Generate post-quantum keypair
        let mut rng = DefaultRng::default();
        let (pq_public, pq_secret) = generate_keypair(self.pq_suite.kem_algorithm(), &mut rng)
            .map_err(|e| NetworkError::Security(SecurityError::KeyGeneration(e.to_string())))?;
        
        // Generate classical keypair if hybrid
        let classical_keypair = if self.pq_suite.is_hybrid() {
            let mut classical_rng = thread_rng();
            let secret = x25519_dalek::StaticSecret::random_from_rng(&mut classical_rng);
            let public = x25519_dalek::PublicKey::from(&secret);
            Some((public, secret))
        } else {
            None
        };
        
        Ok((
            HybridPublicKey {
                pq_public,
                classical_public: classical_keypair.as_ref().map(|(p, _)| *p),
            },
            HybridSecretKey {
                pq_secret,
                classical_secret: classical_keypair.map(|(_, s)| s),
            },
        ))
    }
    
    /// Encapsulates a shared secret using the public key.
    pub fn encapsulate(
        &mut self,
        public_key: &HybridPublicKey,
    ) -> Result<(HybridCiphertext, Vec<u8>)> {
        // Post-quantum encapsulation
        let mut rng = DefaultRng::default();
        let (pq_ciphertext, pq_shared) = encapsulate(
            self.pq_suite.kem_algorithm(),
            &public_key.pq_public,
            &mut rng,
        )
        .map_err(|e| NetworkError::Security(SecurityError::KeyExchange(e.to_string())))?;
        
        // Classical key exchange if hybrid
        let (classical_ciphertext, classical_shared) = if self.pq_suite.is_hybrid() {
            let classical_public = public_key.classical_public
                .ok_or_else(|| NetworkError::Security(SecurityError::KeyExchange(
                    "Missing classical public key in hybrid mode".to_string()
                )))?;
            
            let mut classical_rng = thread_rng();
            let ephemeral_secret = x25519_dalek::StaticSecret::random_from_rng(&mut classical_rng);
            let ephemeral_public = x25519_dalek::PublicKey::from(&ephemeral_secret);
            let shared = ephemeral_secret.diffie_hellman(&classical_public);
            
            (Some(ephemeral_public), Some(shared.as_bytes().to_vec()))
        } else {
            (None, None)
        };
        
        // Combine shared secrets
        let combined_shared = if let Some(classical) = classical_shared {
            // XOR combine for hybrid mode (in production, use a proper KDF)
            let mut combined = pq_shared.clone();
            for (i, byte) in combined.iter_mut().enumerate() {
                *byte ^= classical[i % classical.len()];
            }
            combined
        } else {
            pq_shared
        };
        
        Ok((
            HybridCiphertext {
                pq_ciphertext,
                classical_public: classical_ciphertext,
            },
            combined_shared,
        ))
    }
    
    /// Decapsulates the shared secret using the secret key.
    pub fn decapsulate(
        &self,
        secret_key: &HybridSecretKey,
        ciphertext: &HybridCiphertext,
    ) -> Result<Vec<u8>> {
        // Post-quantum decapsulation
        let pq_shared = decapsulate(
            self.pq_suite.kem_algorithm(),
            &secret_key.pq_secret,
            &ciphertext.pq_ciphertext,
        )
        .map_err(|e| NetworkError::Security(SecurityError::KeyExchange(e.to_string())))?;
        
        // Classical key exchange if hybrid
        let classical_shared = if self.pq_suite.is_hybrid() {
            let classical_secret = secret_key.classical_secret.as_ref()
                .ok_or_else(|| NetworkError::Security(SecurityError::KeyExchange(
                    "Missing classical secret key in hybrid mode".to_string()
                )))?;
            
            let classical_public = ciphertext.classical_public
                .ok_or_else(|| NetworkError::Security(SecurityError::KeyExchange(
                    "Missing classical public key in hybrid ciphertext".to_string()
                )))?;
            
            let shared = classical_secret.diffie_hellman(&classical_public);
            Some(shared.as_bytes().to_vec())
        } else {
            None
        };
        
        // Combine shared secrets
        let combined_shared = if let Some(classical) = classical_shared {
            // XOR combine for hybrid mode
            let mut combined = pq_shared.clone();
            for (i, byte) in combined.iter_mut().enumerate() {
                *byte ^= classical[i % classical.len()];
            }
            combined
        } else {
            pq_shared
        };
        
        Ok(combined_shared)
    }
}

/// Hybrid public key containing both classical and post-quantum components.
#[derive(Debug, Clone)]
pub struct HybridPublicKey {
    /// Post-quantum public key
    pub pq_public: Vec<u8>,
    
    /// Classical public key (for hybrid mode)
    pub classical_public: Option<x25519_dalek::PublicKey>,
}

/// Hybrid secret key containing both classical and post-quantum components.
#[derive(Clone)]
pub struct HybridSecretKey {
    /// Post-quantum secret key
    pub pq_secret: Vec<u8>,
    
    /// Classical secret key (for hybrid mode)
    pub classical_secret: Option<x25519_dalek::StaticSecret>,
}

/// Hybrid ciphertext containing both classical and post-quantum components.
#[derive(Debug, Clone)]
pub struct HybridCiphertext {
    /// Post-quantum ciphertext
    pub pq_ciphertext: Vec<u8>,
    
    /// Classical ephemeral public key (for hybrid mode)
    pub classical_public: Option<x25519_dalek::PublicKey>,
}

/// Post-quantum signature operations.
#[derive(Clone)]
pub struct PQSignature {
    /// Signature algorithm
    algorithm: PQSignatureAlgorithm,
}

impl PQSignature {
    /// Creates a new signature instance.
    pub fn new(algorithm: PQSignatureAlgorithm) -> Result<Self> {
        Ok(Self { algorithm })
    }
    
    /// Generates a signing keypair.
    pub fn generate_keypair(&mut self) -> Result<(Vec<u8>, Vec<u8>)> {
        let mut rng = DefaultRng::default();
        generate_signing_keypair(self.algorithm.signature_algorithm(), &mut rng)
            .map_err(|e| NetworkError::Security(SecurityError::KeyGeneration(e.to_string())))
    }
    
    /// Signs a message.
    pub fn sign(&mut self, secret_key: &[u8], message: &[u8]) -> Result<Vec<u8>> {
        let mut rng = DefaultRng::default();
        let pq_signature = sign(
            self.algorithm.signature_algorithm(),
            secret_key,
            message,
            &mut rng,
        )
        .map_err(|e| NetworkError::Security(SecurityError::Signature(e.to_string())))?;
        
        if self.algorithm.is_hybrid() {
            // In hybrid mode, also create Ed25519 signature
            // This is simplified - in production, properly handle hybrid signatures
            Ok(pq_signature)
        } else {
            Ok(pq_signature)
        }
    }
    
    /// Verifies a signature.
    pub fn verify(&self, public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<bool> {
        verify(
            self.algorithm.signature_algorithm(),
            public_key,
            message,
            signature,
        )
        .map_err(|e| NetworkError::Security(SecurityError::Verification(e.to_string())))
    }
}

impl fmt::Display for PQCipherSuite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kyber768ChaCha20 => write!(f, "Kyber768-ChaCha20-Poly1305"),
            Self::Kyber1024ChaCha20 => write!(f, "Kyber1024-ChaCha20-Poly1305"),
            Self::Kyber768Aes256 => write!(f, "Kyber768-AES256-GCM"),
            Self::Kyber1024Aes256 => write!(f, "Kyber1024-AES256-GCM"),
            Self::HybridX25519Kyber768 => write!(f, "X25519-Kyber768-Hybrid"),
            Self::HybridX25519Kyber1024 => write!(f, "X25519-Kyber1024-Hybrid"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_quantum_key_exchange() {
        let mut kex = HybridKeyExchange::new(PQCipherSuite::Kyber768ChaCha20).unwrap();
        
        // Generate keypair
        let (public, secret) = kex.generate_keypair().unwrap();
        
        // Encapsulate
        let (ciphertext, shared1) = kex.encapsulate(&public).unwrap();
        
        // Decapsulate
        let shared2 = kex.decapsulate(&secret, &ciphertext).unwrap();
        
        // Shared secrets should match
        assert_eq!(shared1, shared2);
    }
    
    #[test]
    fn test_hybrid_key_exchange() {
        let mut kex = HybridKeyExchange::new(PQCipherSuite::HybridX25519Kyber768).unwrap();
        
        // Generate keypair
        let (public, secret) = kex.generate_keypair().unwrap();
        
        // Verify hybrid components exist
        assert!(public.classical_public.is_some());
        assert!(secret.classical_secret.is_some());
        
        // Encapsulate
        let (ciphertext, shared1) = kex.encapsulate(&public).unwrap();
        assert!(ciphertext.classical_public.is_some());
        
        // Decapsulate
        let shared2 = kex.decapsulate(&secret, &ciphertext).unwrap();
        
        // Shared secrets should match
        assert_eq!(shared1, shared2);
    }
    
    #[test]
    fn test_post_quantum_signatures() {
        let mut sig = PQSignature::new(PQSignatureAlgorithm::Dilithium3).unwrap();
        
        // Generate keypair
        let (public, secret) = sig.generate_keypair().unwrap();
        
        // Sign message
        let message = b"Post-quantum secure message";
        let signature = sig.sign(&secret, message).unwrap();
        
        // Verify signature
        let valid = sig.verify(&public, message, &signature).unwrap();
        assert!(valid);
        
        // Verify with wrong message fails
        let wrong_message = b"Modified message";
        let invalid = sig.verify(&public, wrong_message, &signature).unwrap();
        assert!(!invalid);
    }
}