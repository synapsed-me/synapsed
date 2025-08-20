//! Key derivation functions and utilities.

use crate::error::{NetworkError, Result, SecurityError};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroize;

/// Key derivation function algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
pub enum KeyDerivationFunction {
    /// HKDF with SHA-256
    HkdfSha256,
    
    /// HKDF with SHA-384
    HkdfSha384,
    
    /// HKDF with SHA-512
    HkdfSha512,
}

impl KeyDerivationFunction {
    /// Returns the output length for this KDF.
    pub fn output_len(&self) -> usize {
        match self {
            Self::HkdfSha256 => 32,
            Self::HkdfSha384 => 48,
            Self::HkdfSha512 => 64,
        }
    }
}

/// Session keys derived from a master secret.
#[derive(Clone)]
pub struct SessionKeys {
    /// Encryption key for sending data
    pub client_write_key: Vec<u8>,
    
    /// Encryption key for receiving data
    pub server_write_key: Vec<u8>,
    
    /// Authentication key for sending data
    pub client_mac_key: Vec<u8>,
    
    /// Authentication key for receiving data
    pub server_mac_key: Vec<u8>,
    
    /// IV for client writes
    pub client_iv: Vec<u8>,
    
    /// IV for server writes
    pub server_iv: Vec<u8>,
}

impl SessionKeys {
    /// Zeros out all key material.
    pub fn zeroize(&mut self) {
        use zeroize::Zeroize;
        
        self.client_write_key.zeroize();
        self.server_write_key.zeroize();
        self.client_mac_key.zeroize();
        self.server_mac_key.zeroize();
        self.client_iv.zeroize();
        self.server_iv.zeroize();
    }
}

impl Drop for SessionKeys {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Derives session keys from a master secret using HKDF.
pub fn derive_session_keys(
    kdf: KeyDerivationFunction,
    master_secret: &[u8],
    salt: Option<&[u8]>,
    info: &[u8],
    key_size: usize,
    iv_size: usize,
) -> Result<SessionKeys> {
    // Calculate total output needed
    let total_size = key_size * 4 + iv_size * 2; // 4 keys + 2 IVs
    let mut output = vec![0u8; total_size];
    
    // Perform key derivation
    match kdf {
        KeyDerivationFunction::HkdfSha256 => {
            let hk = Hkdf::<sha2::Sha256>::new(salt, master_secret);
            hk.expand(info, &mut output)
                .map_err(|_| NetworkError::Security(SecurityError::KeyDerivation(
                    "HKDF-SHA256 expansion failed".to_string()
                )))?;
        }
        KeyDerivationFunction::HkdfSha384 => {
            let hk = Hkdf::<sha2::Sha384>::new(salt, master_secret);
            hk.expand(info, &mut output)
                .map_err(|_| NetworkError::Security(SecurityError::KeyDerivation(
                    "HKDF-SHA384 expansion failed".to_string()
                )))?;
        }
        KeyDerivationFunction::HkdfSha512 => {
            let hk = Hkdf::<sha2::Sha512>::new(salt, master_secret);
            hk.expand(info, &mut output)
                .map_err(|_| NetworkError::Security(SecurityError::KeyDerivation(
                    "HKDF-SHA512 expansion failed".to_string()
                )))?;
        }
    }
    
    // Split the output into individual keys
    let mut offset = 0;
    let client_write_key = output[offset..offset + key_size].to_vec();
    offset += key_size;
    
    let server_write_key = output[offset..offset + key_size].to_vec();
    offset += key_size;
    
    let client_mac_key = output[offset..offset + key_size].to_vec();
    offset += key_size;
    
    let server_mac_key = output[offset..offset + key_size].to_vec();
    offset += key_size;
    
    let client_iv = output[offset..offset + iv_size].to_vec();
    offset += iv_size;
    
    let server_iv = output[offset..offset + iv_size].to_vec();
    
    // Zero out the temporary buffer
    use zeroize::Zeroize;
    output.zeroize();
    
    Ok(SessionKeys {
        client_write_key,
        server_write_key,
        client_mac_key,
        server_mac_key,
        client_iv,
        server_iv,
    })
}

/// Key ratcheting for forward secrecy.
pub struct KeyRatchet {
    /// Current chain key
    chain_key: Vec<u8>,
    
    /// KDF algorithm
    kdf: KeyDerivationFunction,
    
    /// Generation counter
    generation: u64,
}

impl KeyRatchet {
    /// Creates a new key ratchet.
    pub fn new(initial_key: Vec<u8>, kdf: KeyDerivationFunction) -> Self {
        Self {
            chain_key: initial_key,
            kdf,
            generation: 0,
        }
    }
    
    /// Advances the ratchet and derives a new key.
    pub fn advance(&mut self) -> Result<Vec<u8>> {
        let info = format!("ratchet-{}", self.generation).into_bytes();
        let mut output = vec![0u8; self.kdf.output_len() * 2];
        
        // Derive next chain key and output key
        match self.kdf {
            KeyDerivationFunction::HkdfSha256 => {
                let hk = Hkdf::<sha2::Sha256>::new(None, &self.chain_key);
                hk.expand(&info, &mut output)
                    .map_err(|_| NetworkError::Security(SecurityError::KeyDerivation(
                        "Ratchet HKDF expansion failed".to_string()
                    )))?;
            }
            KeyDerivationFunction::HkdfSha384 => {
                let hk = Hkdf::<sha2::Sha384>::new(None, &self.chain_key);
                hk.expand(&info, &mut output)
                    .map_err(|_| NetworkError::Security(SecurityError::KeyDerivation(
                        "Ratchet HKDF expansion failed".to_string()
                    )))?;
            }
            KeyDerivationFunction::HkdfSha512 => {
                let hk = Hkdf::<sha2::Sha512>::new(None, &self.chain_key);
                hk.expand(&info, &mut output)
                    .map_err(|_| NetworkError::Security(SecurityError::KeyDerivation(
                        "Ratchet HKDF expansion failed".to_string()
                    )))?;
            }
        }
        
        // Update chain key and return output key
        let key_len = self.kdf.output_len();
        self.chain_key = output[..key_len].to_vec();
        let output_key = output[key_len..key_len * 2].to_vec();
        
        self.generation += 1;
        
        // Zero out temporary buffer
        use zeroize::Zeroize;
        output.zeroize();
        
        Ok(output_key)
    }
    
    /// Returns the current generation number.
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

impl Drop for KeyRatchet {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.chain_key.zeroize();
    }
}

/// Derives a key using PBKDF2 for password-based scenarios.
pub fn derive_key_from_password(
    password: &[u8],
    salt: &[u8],
    iterations: u32,
    key_len: usize,
) -> Result<Vec<u8>> {
    use ring::pbkdf2;
    
    let mut key = vec![0u8; key_len];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        std::num::NonZeroU32::new(iterations).unwrap(),
        salt,
        password,
        &mut key,
    );
    
    Ok(key)
}

impl fmt::Display for KeyDerivationFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HkdfSha256 => write!(f, "HKDF-SHA256"),
            Self::HkdfSha384 => write!(f, "HKDF-SHA384"),
            Self::HkdfSha512 => write!(f, "HKDF-SHA512"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_key_derivation() {
        let master_secret = b"master secret";
        let salt = b"salt";
        let info = b"session keys";
        
        let keys = derive_session_keys(
            KeyDerivationFunction::HkdfSha256,
            master_secret,
            Some(salt),
            info,
            32, // key size
            16, // IV size
        ).unwrap();
        
        // Verify all keys are different
        assert_ne!(keys.client_write_key, keys.server_write_key);
        assert_ne!(keys.client_mac_key, keys.server_mac_key);
        assert_ne!(keys.client_iv, keys.server_iv);
        
        // Verify key sizes
        assert_eq!(keys.client_write_key.len(), 32);
        assert_eq!(keys.client_iv.len(), 16);
    }
    
    #[test]
    fn test_key_ratchet() {
        let initial_key = vec![0u8; 32];
        let mut ratchet = KeyRatchet::new(initial_key, KeyDerivationFunction::HkdfSha256);
        
        // Generate multiple keys
        let key1 = ratchet.advance().unwrap();
        let key2 = ratchet.advance().unwrap();
        let key3 = ratchet.advance().unwrap();
        
        // All keys should be different
        assert_ne!(key1, key2);
        assert_ne!(key2, key3);
        assert_ne!(key1, key3);
        
        // Check generation counter
        assert_eq!(ratchet.generation(), 3);
    }
    
    #[test]
    fn test_password_derivation() {
        let password = b"secure password";
        let salt = b"random salt";
        
        let key = derive_key_from_password(password, salt, 100_000, 32).unwrap();
        
        assert_eq!(key.len(), 32);
        
        // Same inputs should produce same key
        let key2 = derive_key_from_password(password, salt, 100_000, 32).unwrap();
        assert_eq!(key, key2);
        
        // Different salt should produce different key
        let key3 = derive_key_from_password(password, b"different salt", 100_000, 32).unwrap();
        assert_ne!(key, key3);
    }
}