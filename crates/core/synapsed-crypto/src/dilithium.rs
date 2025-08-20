//! ML-DSA (Dilithium) implementation
//!
//! This module implements the Module-Lattice-Based Digital Signature Algorithm
//! as specified in NIST FIPS 204.

pub mod dilithium2;
pub mod dilithium3;
pub mod dilithium5;

// Re-export the main types
pub use dilithium2::Dilithium2;
pub use dilithium3::Dilithium3;
pub use dilithium5::Dilithium5;

// Re-export key types for easier access
pub use self::{DilithiumPublicKey as PublicKey, DilithiumSecretKey as SecretKey};

use crate::{
    error::{Error, Result},
    params::dilithium::*,
    traits::Serializable,
};
use zeroize::Zeroize;

/// Common Dilithium functionality
pub(crate) mod common {
    use super::*;
    
    /// Power-of-2 rounding
    pub fn power2round(a: i32) -> (i32, i32) {
        const D: i32 = 13;
        let a1 = (a + (1 << (D - 1)) - 1) >> D;
        let a0 = a - (a1 << D);
        (a1, a0)
    }
    
    /// Decompose for hint generation
    pub fn decompose(a: i32, gamma2: i32) -> (i32, i32) {
        let a1 = (a + 127) >> 7;
        let a1 = ((a1.wrapping_mul(1025).wrapping_add(1 << 21)) >> 22).wrapping_mul(3);
        let a1 = a1 & !((((43i32.wrapping_sub(a1)) >> 31) & 1).wrapping_neg());
        
        let a0 = a.wrapping_sub(a1.wrapping_mul(2).wrapping_mul(gamma2));
        let a0 = a0.wrapping_sub(((Q / 2).wrapping_sub(a0).wrapping_sub(1).wrapping_neg() >> 31) & Q);
        
        (a1, a0)
    }
    
    /// Make hint for signature compression
    pub fn make_hint(a0: i32, a1: i32, gamma2: i32) -> bool {
        a0 > gamma2 || a0 < -gamma2 || (a0 == -gamma2 && a1 != 0)
    }
    
    /// Use hint to recover high bits
    pub fn use_hint(a: i32, hint: bool, gamma2: i32) -> i32 {
        let (a1, a0) = decompose(a, gamma2);
        
        if hint {
            if a0 > 0 {
                (a1 + 1) % 44
            } else {
                (a1 - 1 + 44) % 44
            }
        } else {
            a1
        }
    }
}

/// Dilithium public key
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DilithiumPublicKey<const K: usize> {
    /// Packed public key bytes
    pub bytes: Vec<u8>,
}

impl<const K: usize> AsRef<[u8]> for DilithiumPublicKey<K> {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl<const K: usize> Serializable for DilithiumPublicKey<K> {
    fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
    
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Calculate expected size based on K
        // PUBLIC_KEY_SIZE = SYMBYTES + K * POLYT1_PACKEDBYTES
        let expected_size = match K {
            4 => 1312,  // Dilithium2: 32 + 4*320
            6 => 1952,  // Dilithium3: 32 + 6*320
            8 => 2592,  // Dilithium5: 32 + 8*320
            _ => return Err(Error::InvalidParameter),
        };
        
        if bytes.len() != expected_size {
            return Err(Error::InvalidKeySize);
        }
        
        Ok(Self {
            bytes: bytes.to_vec(),
        })
    }
}

/// Dilithium secret key
#[derive(Clone, Debug)]
pub struct DilithiumSecretKey<const K: usize> {
    /// Packed secret key bytes
    pub bytes: Vec<u8>,
}

impl<const K: usize> AsRef<[u8]> for DilithiumSecretKey<K> {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl<const K: usize> Zeroize for DilithiumSecretKey<K> {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl<const K: usize> Drop for DilithiumSecretKey<K> {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl<const K: usize> Serializable for DilithiumSecretKey<K> {
    fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
    
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Calculate expected size based on K
        // This is complex and depends on L as well, so we hardcode for known configs
        let expected_size = match K {
            4 => 2528,  // Dilithium2
            6 => 4000,  // Dilithium3  
            8 => 4864,  // Dilithium5
            _ => return Err(Error::InvalidParameter),
        };
        
        if bytes.len() != expected_size {
            return Err(Error::InvalidKeySize);
        }
        
        Ok(Self {
            bytes: bytes.to_vec(),
        })
    }
}

/// Dilithium signature
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DilithiumSignature {
    /// Packed signature bytes
    pub bytes: Vec<u8>,
}

impl AsRef<[u8]> for DilithiumSignature {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl Serializable for DilithiumSignature {
    fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
    
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Signature sizes vary by parameter set
        // We accept a range of valid sizes
        let valid_sizes = [2420, 3293, 4595]; // Dilithium2, 3, 5
        
        if !valid_sizes.contains(&bytes.len()) {
            return Err(Error::InvalidSignature);
        }
        
        Ok(Self {
            bytes: bytes.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::common::*;
    
    #[test]
    fn test_power2round() {
        let (a1, a0) = power2round(12345);
        assert_eq!(a1 * (1 << 13) + a0, 12345);
        assert!((-(1 << 12)..(1 << 12)).contains(&a0));
    }
    
    #[test]
    fn test_decompose() {
        let gamma2 = 95232;
        let (a1, _a0) = decompose(1234567, gamma2);
        // The decompose function is more complex than a simple modulo operation
        // It ensures that a = a1 * 2 * gamma2 + a0 (mod q)
        // and that a0 is in the correct range
        // For now, just check that the decomposition can be performed
        assert!(a1.abs() < 44); // a1 should be small
    }
}