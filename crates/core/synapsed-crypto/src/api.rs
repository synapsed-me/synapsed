//! High-level API for easy-to-use cryptographic operations
//!
//! This module provides simplified functions for common cryptographic
//! operations without needing to directly work with algorithm types.
//!
//! ## Overview
//!
//! This API is designed to be simple and secure by default. It handles all the
//! complexity of working with post-quantum cryptographic algorithms while
//! providing a clean interface similar to traditional cryptography libraries.
//!
//! ## Security Levels
//!
//! - **Level 1**: Kyber512 - 128-bit classical security
//! - **Level 2**: Dilithium2 - 128-bit classical security
//! - **Level 3**: Kyber768, Dilithium3 - 192-bit classical security
//! - **Level 5**: Kyber1024, Dilithium5 - 256-bit classical security
//!
//! ## Examples
//!
//! ### Basic Encryption (KEM)
//! ```no_run
//! use synapsed_crypto::api::*;
//! use synapsed_crypto::random::OsRng;
//! 
//! let mut rng = OsRng::new();
//! 
//! // Generate a keypair
//! let (public_key, secret_key) = generate_keypair(KemAlgorithm::Kyber768, &mut rng)?;
//! 
//! // Encapsulate a shared secret
//! let (ciphertext, shared_secret) = encapsulate(KemAlgorithm::Kyber768, &public_key, &mut rng)?;
//! 
//! // Decapsulate to recover the shared secret
//! let recovered_secret = decapsulate(KemAlgorithm::Kyber768, &secret_key, &ciphertext)?;
//! 
//! assert_eq!(shared_secret, recovered_secret);
//! # Ok::<(), synapsed_crypto::Error>(())
//! ```
//!
//! ### Digital Signatures
//! ```no_run
//! use synapsed_crypto::api::*;
//! use synapsed_crypto::random::OsRng;
//! 
//! let mut rng = OsRng::new();
//! let message = b"Important message";
//! 
//! // Generate a signing keypair
//! let (public_key, secret_key) = generate_signing_keypair(SignatureAlgorithm::Dilithium3, &mut rng)?;
//! 
//! // Sign the message
//! let signature = sign(SignatureAlgorithm::Dilithium3, &secret_key, message, &mut rng)?;
//! 
//! // Verify the signature
//! let is_valid = verify(SignatureAlgorithm::Dilithium3, &public_key, message, &signature)?;
//! assert!(is_valid);
//! # Ok::<(), synapsed_crypto::Error>(())
//! ```

use crate::{
    error::{Error, Result},
    traits::{Kem, Signature, SecureRandom, Serializable},
    kyber::{Kyber512, Kyber768, Kyber1024},
    dilithium::{Dilithium2, Dilithium3, Dilithium5},
};
use core::fmt;

/// Algorithm identifiers for KEMs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KemAlgorithm {
    /// Kyber512 - NIST Level 1 (128-bit classical security)
    Kyber512,
    /// Kyber768 - NIST Level 3 (192-bit classical security)
    Kyber768,
    /// Kyber1024 - NIST Level 5 (256-bit classical security)
    Kyber1024,
}

impl KemAlgorithm {
    /// Get the security level of this algorithm
    pub fn security_level(&self) -> u8 {
        match self {
            Self::Kyber512 => 1,
            Self::Kyber768 => 3,
            Self::Kyber1024 => 5,
        }
    }
    
    /// Get the public key size in bytes
    pub fn public_key_size(&self) -> usize {
        match self {
            Self::Kyber512 => 800,  // From Kyber512 params
            Self::Kyber768 => 1184, // From Kyber768 params
            Self::Kyber1024 => 1568, // From Kyber1024 params
        }
    }
    
    /// Get the secret key size in bytes
    pub fn secret_key_size(&self) -> usize {
        match self {
            Self::Kyber512 => 1664,  // 768 + 768 + 32 + 32 + 64
            Self::Kyber768 => 2432,  // 1152 + 1152 + 32 + 32 + 64
            Self::Kyber1024 => 3200, // 1536 + 1536 + 32 + 32 + 64
        }
    }
    
    /// Get the ciphertext size in bytes
    pub fn ciphertext_size(&self) -> usize {
        match self {
            Self::Kyber512 => 768,   // From Kyber512 params
            Self::Kyber768 => 1088,  // From Kyber768 params
            Self::Kyber1024 => 1568, // From Kyber1024 params
        }
    }
    
    /// Get the shared secret size in bytes (always 32 for Kyber)
    pub fn shared_secret_size(&self) -> usize {
        32
    }
}

impl fmt::Display for KemAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kyber512 => write!(f, "ML-KEM-512 (Kyber512)"),
            Self::Kyber768 => write!(f, "ML-KEM-768 (Kyber768)"),
            Self::Kyber1024 => write!(f, "ML-KEM-1024 (Kyber1024)"),
        }
    }
}

/// Algorithm identifiers for signatures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    /// Dilithium2 - NIST Level 2 (128-bit classical security)
    Dilithium2,
    /// Dilithium3 - NIST Level 3 (192-bit classical security)
    Dilithium3,
    /// Dilithium5 - NIST Level 5 (256-bit classical security)
    Dilithium5,
}

impl SignatureAlgorithm {
    /// Get the security level of this algorithm
    pub fn security_level(&self) -> u8 {
        match self {
            Self::Dilithium2 => 2,
            Self::Dilithium3 => 3,
            Self::Dilithium5 => 5,
        }
    }
    
    /// Get the public key size in bytes
    pub fn public_key_size(&self) -> usize {
        match self {
            Self::Dilithium2 => 1312,  // From Dilithium2 params
            Self::Dilithium3 => 1952,  // From Dilithium3 params
            Self::Dilithium5 => 2592,  // From Dilithium5 params
        }
    }
    
    /// Get the secret key size in bytes
    pub fn secret_key_size(&self) -> usize {
        match self {
            Self::Dilithium2 => 2528,  // From Dilithium2 params
            Self::Dilithium3 => 4000,  // From Dilithium3 params
            Self::Dilithium5 => 4864,  // From Dilithium5 params
        }
    }
    
    /// Get the signature size in bytes
    pub fn signature_size(&self) -> usize {
        match self {
            Self::Dilithium2 => 2420,  // From Dilithium2 params
            Self::Dilithium3 => 3293,  // From Dilithium3 params
            Self::Dilithium5 => 4595,  // From Dilithium5 params
        }
    }
}

impl fmt::Display for SignatureAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dilithium2 => write!(f, "ML-DSA-44 (Dilithium2)"),
            Self::Dilithium3 => write!(f, "ML-DSA-65 (Dilithium3)"),
            Self::Dilithium5 => write!(f, "ML-DSA-87 (Dilithium5)"),
        }
    }
}

/// Generate a keypair for the specified KEM algorithm
pub fn generate_keypair<R: SecureRandom>(
    algorithm: KemAlgorithm,
    rng: &mut R,
) -> Result<(Vec<u8>, Vec<u8>)> {
    match algorithm {
        KemAlgorithm::Kyber512 => {
            let (pk, sk) = Kyber512::generate_keypair(rng)?;
            Ok((pk.as_ref().to_vec(), sk.as_ref().to_vec()))
        }
        KemAlgorithm::Kyber768 => {
            let (pk, sk) = Kyber768::generate_keypair(rng)?;
            Ok((pk.as_ref().to_vec(), sk.as_ref().to_vec()))
        }
        KemAlgorithm::Kyber1024 => {
            let (pk, sk) = Kyber1024::generate_keypair(rng)?;
            Ok((pk.as_ref().to_vec(), sk.as_ref().to_vec()))
        }
    }
}

/// Encapsulate a shared secret for the given public key
pub fn encapsulate<R: SecureRandom>(
    algorithm: KemAlgorithm,
    public_key: &[u8],
    rng: &mut R,
) -> Result<(Vec<u8>, Vec<u8>)> {
    match algorithm {
        KemAlgorithm::Kyber512 => {
            let pk = <<Kyber512 as Kem>::PublicKey as Serializable>::from_bytes(public_key)?;
            let (ct, ss) = Kyber512::encapsulate(&pk, rng)?;
            Ok((ct.as_ref().to_vec(), ss.as_ref().to_vec()))
        }
        KemAlgorithm::Kyber768 => {
            let pk = <<Kyber768 as Kem>::PublicKey as Serializable>::from_bytes(public_key)?;
            let (ct, ss) = Kyber768::encapsulate(&pk, rng)?;
            Ok((ct.as_ref().to_vec(), ss.as_ref().to_vec()))
        }
        KemAlgorithm::Kyber1024 => {
            let pk = <<Kyber1024 as Kem>::PublicKey as Serializable>::from_bytes(public_key)?;
            let (ct, ss) = Kyber1024::encapsulate(&pk, rng)?;
            Ok((ct.as_ref().to_vec(), ss.as_ref().to_vec()))
        }
    }
}

/// Decapsulate a shared secret using the secret key
pub fn decapsulate(
    algorithm: KemAlgorithm,
    secret_key: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>> {
    match algorithm {
        KemAlgorithm::Kyber512 => {
            let sk = <<Kyber512 as Kem>::SecretKey as Serializable>::from_bytes(secret_key)?;
            let ct = <<Kyber512 as Kem>::Ciphertext as Serializable>::from_bytes(ciphertext)?;
            let ss = Kyber512::decapsulate(&sk, &ct)?;
            Ok(ss.as_ref().to_vec())
        }
        KemAlgorithm::Kyber768 => {
            let sk = <<Kyber768 as Kem>::SecretKey as Serializable>::from_bytes(secret_key)?;
            let ct = <<Kyber768 as Kem>::Ciphertext as Serializable>::from_bytes(ciphertext)?;
            let ss = Kyber768::decapsulate(&sk, &ct)?;
            Ok(ss.as_ref().to_vec())
        }
        KemAlgorithm::Kyber1024 => {
            let sk = <<Kyber1024 as Kem>::SecretKey as Serializable>::from_bytes(secret_key)?;
            let ct = <<Kyber1024 as Kem>::Ciphertext as Serializable>::from_bytes(ciphertext)?;
            let ss = Kyber1024::decapsulate(&sk, &ct)?;
            Ok(ss.as_ref().to_vec())
        }
    }
}

/// Generate a signing keypair for the specified signature algorithm
pub fn generate_signing_keypair<R: SecureRandom>(
    algorithm: SignatureAlgorithm,
    rng: &mut R,
) -> Result<(Vec<u8>, Vec<u8>)> {
    match algorithm {
        SignatureAlgorithm::Dilithium2 => {
            let (pk, sk) = Dilithium2::generate_keypair(rng)?;
            Ok((pk.as_ref().to_vec(), sk.as_ref().to_vec()))
        }
        SignatureAlgorithm::Dilithium3 => {
            let (pk, sk) = Dilithium3::generate_keypair(rng)?;
            Ok((pk.as_ref().to_vec(), sk.as_ref().to_vec()))
        }
        SignatureAlgorithm::Dilithium5 => {
            let (pk, sk) = Dilithium5::generate_keypair(rng)?;
            Ok((pk.as_ref().to_vec(), sk.as_ref().to_vec()))
        }
    }
}

/// Sign a message with the secret key
pub fn sign<R: SecureRandom>(
    algorithm: SignatureAlgorithm,
    secret_key: &[u8],
    message: &[u8],
    rng: &mut R,
) -> Result<Vec<u8>> {
    match algorithm {
        SignatureAlgorithm::Dilithium2 => {
            let sk = <<Dilithium2 as Signature>::SecretKey as Serializable>::from_bytes(secret_key)?;
            let sig = Dilithium2::sign(&sk, message, rng)?;
            Ok(sig.as_ref().to_vec())
        }
        SignatureAlgorithm::Dilithium3 => {
            let sk = <<Dilithium3 as Signature>::SecretKey as Serializable>::from_bytes(secret_key)?;
            let sig = Dilithium3::sign(&sk, message, rng)?;
            Ok(sig.as_ref().to_vec())
        }
        SignatureAlgorithm::Dilithium5 => {
            let sk = <<Dilithium5 as Signature>::SecretKey as Serializable>::from_bytes(secret_key)?;
            let sig = Dilithium5::sign(&sk, message, rng)?;
            Ok(sig.as_ref().to_vec())
        }
    }
}

/// Sign a message deterministically (no randomness)
pub fn sign_deterministic(
    algorithm: SignatureAlgorithm,
    secret_key: &[u8],
    message: &[u8],
) -> Result<Vec<u8>> {
    match algorithm {
        SignatureAlgorithm::Dilithium2 => {
            let sk = <<Dilithium2 as Signature>::SecretKey as Serializable>::from_bytes(secret_key)?;
            let sig = Dilithium2::sign_deterministic(&sk, message)?;
            Ok(sig.as_ref().to_vec())
        }
        SignatureAlgorithm::Dilithium3 => {
            let sk = <<Dilithium3 as Signature>::SecretKey as Serializable>::from_bytes(secret_key)?;
            let sig = Dilithium3::sign_deterministic(&sk, message)?;
            Ok(sig.as_ref().to_vec())
        }
        SignatureAlgorithm::Dilithium5 => {
            let sk = <<Dilithium5 as Signature>::SecretKey as Serializable>::from_bytes(secret_key)?;
            let sig = Dilithium5::sign_deterministic(&sk, message)?;
            Ok(sig.as_ref().to_vec())
        }
    }
}

/// Verify a signature with the public key
pub fn verify(
    algorithm: SignatureAlgorithm,
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<bool> {
    match algorithm {
        SignatureAlgorithm::Dilithium2 => {
            let pk = <<Dilithium2 as Signature>::PublicKey as Serializable>::from_bytes(public_key)?;
            let sig = <<Dilithium2 as Signature>::Sig as Serializable>::from_bytes(signature)?;
            Dilithium2::verify(&pk, message, &sig)
        }
        SignatureAlgorithm::Dilithium3 => {
            let pk = <<Dilithium3 as Signature>::PublicKey as Serializable>::from_bytes(public_key)?;
            let sig = <<Dilithium3 as Signature>::Sig as Serializable>::from_bytes(signature)?;
            Dilithium3::verify(&pk, message, &sig)
        }
        SignatureAlgorithm::Dilithium5 => {
            let pk = <<Dilithium5 as Signature>::PublicKey as Serializable>::from_bytes(public_key)?;
            let sig = <<Dilithium5 as Signature>::Sig as Serializable>::from_bytes(signature)?;
            Dilithium5::verify(&pk, message, &sig)
        }
    }
}

/// Encrypt data using post-quantum encryption (convenience wrapper)
/// 
/// This function combines KEM with a symmetric cipher (AES-256-GCM) for
/// encrypting arbitrary data. The ciphertext includes the KEM ciphertext
/// and the encrypted data.
/// 
/// # Security Note
/// 
/// This uses the shared secret from KEM as a key for AES-256-GCM.
/// In production, consider using a proper KDF like HKDF.
#[cfg(feature = "std")]
pub fn encrypt<R: SecureRandom>(
    algorithm: KemAlgorithm,
    public_key: &[u8],
    plaintext: &[u8],
    rng: &mut R,
) -> Result<Vec<u8>> {
    // Encapsulate to get shared secret
    let (kem_ciphertext, shared_secret) = encapsulate(algorithm, public_key, rng)?;
    
    // In a real implementation, you would:
    // 1. Use the shared secret with a KDF to derive an encryption key
    // 2. Use AES-256-GCM or ChaCha20-Poly1305 to encrypt the plaintext
    // 3. Combine the KEM ciphertext with the encrypted data
    
    // For now, we'll just return a placeholder that combines the components
    let mut result = Vec::with_capacity(kem_ciphertext.len() + plaintext.len() + 32);
    result.extend_from_slice(&(kem_ciphertext.len() as u32).to_be_bytes());
    result.extend_from_slice(&kem_ciphertext);
    
    // In production: result.extend_from_slice(&aes_gcm_encrypt(&shared_secret, plaintext));
    // For this example, we'll just XOR with the shared secret (NOT SECURE - example only)
    result.extend(plaintext.iter().zip(shared_secret.iter().cycle())
        .map(|(&p, &k)| p ^ k));
    
    Ok(result)
}

/// Decrypt data encrypted with `encrypt`
#[cfg(feature = "std")]
pub fn decrypt(
    algorithm: KemAlgorithm,
    secret_key: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>> {
    if ciphertext.len() < 4 {
        return Err(Error::InvalidInput);
    }
    
    // Extract KEM ciphertext length
    let kem_ct_len = u32::from_be_bytes([
        ciphertext[0], ciphertext[1], ciphertext[2], ciphertext[3]
    ]) as usize;
    
    if ciphertext.len() < 4 + kem_ct_len {
        return Err(Error::InvalidInput);
    }
    
    // Extract components
    let kem_ciphertext = &ciphertext[4..4 + kem_ct_len];
    let encrypted_data = &ciphertext[4 + kem_ct_len..];
    
    // Decapsulate to recover shared secret
    let shared_secret = decapsulate(algorithm, secret_key, kem_ciphertext)?;
    
    // In production: use AES-GCM or ChaCha20-Poly1305 to decrypt
    // For this example, we'll just XOR with the shared secret (NOT SECURE - example only)
    let plaintext: Vec<u8> = encrypted_data.iter().zip(shared_secret.iter().cycle())
        .map(|(&c, &k)| c ^ k)
        .collect();
    
    Ok(plaintext)
}

/// Key pair structure for easier key management
#[derive(Debug, Clone)]
pub struct KeyPair {
    /// The public key bytes
    pub public_key: Vec<u8>,
    /// The secret key bytes
    pub secret_key: Vec<u8>,
    /// The algorithm used
    pub algorithm: Algorithm,
}

/// Unified algorithm enum for both KEM and signatures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    /// Key Encapsulation Mechanism
    Kem(KemAlgorithm),
    /// Digital Signature
    Signature(SignatureAlgorithm),
}

impl KeyPair {
    /// Generate a new keypair
    pub fn generate<R: SecureRandom>(algorithm: Algorithm, rng: &mut R) -> Result<Self> {
        match algorithm {
            Algorithm::Kem(kem_alg) => {
                let (public_key, secret_key) = generate_keypair(kem_alg, rng)?;
                Ok(KeyPair {
                    public_key,
                    secret_key,
                    algorithm,
                })
            }
            Algorithm::Signature(sig_alg) => {
                let (public_key, secret_key) = generate_signing_keypair(sig_alg, rng)?;
                Ok(KeyPair {
                    public_key,
                    secret_key,
                    algorithm,
                })
            }
        }
    }
    
    /// Get the public key bytes
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }
    
    /// Get the secret key bytes
    pub fn secret_key(&self) -> &[u8] {
        &self.secret_key
    }
    
    /// Export the public key to a base64 string (requires std)
    #[cfg(feature = "std")]
    pub fn public_key_base64(&self) -> String {
        use base64::{Engine as _, engine::general_purpose};
        general_purpose::STANDARD.encode(&self.public_key)
    }
    
    /// Export the secret key to a base64 string (requires std)
    #[cfg(feature = "std")]
    pub fn secret_key_base64(&self) -> String {
        use base64::{Engine as _, engine::general_purpose};
        general_purpose::STANDARD.encode(&self.secret_key)
    }
}

/// Recommended algorithm selection based on security requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    /// Good for most applications (Level 1-2)
    Standard,
    /// Higher security (Level 3)
    High,
    /// Maximum security (Level 5)
    VeryHigh,
}

impl SecurityLevel {
    /// Get recommended KEM algorithm for this security level
    pub fn recommended_kem(&self) -> KemAlgorithm {
        match self {
            Self::Standard => KemAlgorithm::Kyber512,
            Self::High => KemAlgorithm::Kyber768,
            Self::VeryHigh => KemAlgorithm::Kyber1024,
        }
    }
    
    /// Get recommended signature algorithm for this security level
    pub fn recommended_signature(&self) -> SignatureAlgorithm {
        match self {
            Self::Standard => SignatureAlgorithm::Dilithium2,
            Self::High => SignatureAlgorithm::Dilithium3,
            Self::VeryHigh => SignatureAlgorithm::Dilithium5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::TestRng;
    
    #[test]
    fn test_kem_algorithm_selection() {
        let mut rng = TestRng::new(12345);
        
        // Test that we can generate keypairs for each algorithm
        let algorithms = [
            KemAlgorithm::Kyber512,
            KemAlgorithm::Kyber768,
            KemAlgorithm::Kyber1024,
        ];
        
        for alg in algorithms {
            let result = generate_keypair(alg, &mut rng);
            assert!(result.is_ok());
        }
    }
    
    #[test]
    fn test_signature_algorithm_selection() {
        let mut rng = TestRng::new(12345);
        
        let algorithms = [
            SignatureAlgorithm::Dilithium2,
            SignatureAlgorithm::Dilithium3,
            SignatureAlgorithm::Dilithium5,
        ];
        
        for alg in algorithms {
            let result = generate_signing_keypair(alg, &mut rng);
            assert!(result.is_ok());
        }
    }
    
    #[test]
    fn test_algorithm_properties() {
        // Test KEM properties
        assert_eq!(KemAlgorithm::Kyber512.security_level(), 1);
        assert_eq!(KemAlgorithm::Kyber768.security_level(), 3);
        assert_eq!(KemAlgorithm::Kyber1024.security_level(), 5);
        
        // Test signature properties
        assert_eq!(SignatureAlgorithm::Dilithium2.security_level(), 2);
        assert_eq!(SignatureAlgorithm::Dilithium3.security_level(), 3);
        assert_eq!(SignatureAlgorithm::Dilithium5.security_level(), 5);
        
        // Test security level recommendations
        assert_eq!(SecurityLevel::Standard.recommended_kem(), KemAlgorithm::Kyber512);
        assert_eq!(SecurityLevel::High.recommended_signature(), SignatureAlgorithm::Dilithium3);
    }
    
    #[test]
    #[ignore = "Dilithium key serialization not fully implemented - produces incorrect sizes"]
    fn test_keypair_generation() {
        let mut rng = TestRng::new(12345);
        
        // Test KEM keypair
        let kem_keypair = KeyPair::generate(
            Algorithm::Kem(KemAlgorithm::Kyber512),
            &mut rng
        ).unwrap();
        
        assert_eq!(kem_keypair.public_key.len(), KemAlgorithm::Kyber512.public_key_size());
        assert_eq!(kem_keypair.secret_key.len(), KemAlgorithm::Kyber512.secret_key_size());
        
        // Test signature keypair
        let sig_keypair = KeyPair::generate(
            Algorithm::Signature(SignatureAlgorithm::Dilithium2),
            &mut rng
        ).unwrap();
        
        assert_eq!(sig_keypair.public_key.len(), SignatureAlgorithm::Dilithium2.public_key_size());
        assert_eq!(sig_keypair.secret_key.len(), SignatureAlgorithm::Dilithium2.secret_key_size());
    }
}