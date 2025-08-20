//! Core traits for cryptographic operations
//!
//! This module defines the fundamental traits that all cryptographic
//! algorithms in this library implement.

use crate::error::Result;
use core::fmt::Debug;
use zeroize::Zeroize;

/// Trait for Key Encapsulation Mechanisms (KEMs)
pub trait Kem: Debug + Send + Sync {
    /// Type representing a public key
    type PublicKey: AsRef<[u8]> + Debug + Clone + PartialEq + Eq;
    
    /// Type representing a secret key
    type SecretKey: AsRef<[u8]> + Debug + Clone + Zeroize;
    
    /// Type representing a ciphertext
    type Ciphertext: AsRef<[u8]> + Debug + Clone + PartialEq + Eq;
    
    /// Type representing a shared secret
    type SharedSecret: AsRef<[u8]> + Debug + Clone + Zeroize + PartialEq;
    
    /// The size of public keys in bytes
    const PUBLIC_KEY_SIZE: usize;
    
    /// The size of secret keys in bytes
    const SECRET_KEY_SIZE: usize;
    
    /// The size of ciphertexts in bytes
    const CIPHERTEXT_SIZE: usize;
    
    /// The size of shared secrets in bytes
    const SHARED_SECRET_SIZE: usize;
    
    /// Generate a new keypair
    fn generate_keypair<R: SecureRandom>(
        rng: &mut R
    ) -> Result<(Self::PublicKey, Self::SecretKey)>;
    
    /// Encapsulate a shared secret for the given public key
    fn encapsulate<R: SecureRandom>(
        public_key: &Self::PublicKey,
        rng: &mut R
    ) -> Result<(Self::Ciphertext, Self::SharedSecret)>;
    
    /// Decapsulate a shared secret using the secret key
    fn decapsulate(
        secret_key: &Self::SecretKey,
        ciphertext: &Self::Ciphertext
    ) -> Result<Self::SharedSecret>;
}

/// Trait for Digital Signature Algorithms
pub trait Signature: Debug + Send + Sync {
    /// Type representing a public key
    type PublicKey: AsRef<[u8]> + Debug + Clone + PartialEq + Eq;
    
    /// Type representing a secret key
    type SecretKey: AsRef<[u8]> + Debug + Clone + Zeroize;
    
    /// Type representing a signature
    type Sig: AsRef<[u8]> + Debug + Clone + PartialEq + Eq;
    
    /// The size of public keys in bytes
    const PUBLIC_KEY_SIZE: usize;
    
    /// The size of secret keys in bytes
    const SECRET_KEY_SIZE: usize;
    
    /// The maximum size of signatures in bytes
    const SIGNATURE_SIZE: usize;
    
    /// Generate a new keypair
    fn generate_keypair<R: SecureRandom>(
        rng: &mut R
    ) -> Result<(Self::PublicKey, Self::SecretKey)>;
    
    /// Sign a message with the secret key
    fn sign<R: SecureRandom>(
        secret_key: &Self::SecretKey,
        message: &[u8],
        rng: &mut R
    ) -> Result<Self::Sig>;
    
    /// Sign a message deterministically (no randomness)
    fn sign_deterministic(
        secret_key: &Self::SecretKey,
        message: &[u8]
    ) -> Result<Self::Sig>;
    
    /// Verify a signature with the public key
    fn verify(
        public_key: &Self::PublicKey,
        message: &[u8],
        signature: &Self::Sig
    ) -> Result<bool>;
}

/// Trait for secure random number generation
pub trait SecureRandom {
    /// Fill the given buffer with random bytes
    fn fill_bytes(&mut self, dest: &mut [u8]);
    
    /// Generate a random u32
    fn next_u32(&mut self) -> u32 {
        let mut buf = [0u8; 4];
        self.fill_bytes(&mut buf);
        u32::from_le_bytes(buf)
    }
    
    /// Generate a random u64
    fn next_u64(&mut self) -> u64 {
        let mut buf = [0u8; 8];
        self.fill_bytes(&mut buf);
        u64::from_le_bytes(buf)
    }
}

/// Trait for types that can be serialized to/from bytes
pub trait Serializable: Sized {
    /// Serialize to bytes
    fn to_bytes(&self) -> Vec<u8>;
    
    /// Deserialize from bytes
    fn from_bytes(bytes: &[u8]) -> Result<Self>;
}

/// Trait for types that can be encoded/decoded to/from hex
#[cfg(feature = "std")]
pub trait HexEncodable: Serializable {
    /// Encode to hexadecimal string
    fn to_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }
    
    /// Decode from hexadecimal string
    fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes = hex::decode(hex_str)
            .map_err(|_| crate::error::Error::InvalidEncoding)?;
        Self::from_bytes(&bytes)
    }
}