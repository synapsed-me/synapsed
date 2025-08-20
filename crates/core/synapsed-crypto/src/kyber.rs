//! ML-KEM (Kyber) implementation
//!
//! This module implements the Module-Lattice-Based Key Encapsulation Mechanism
//! as specified in NIST FIPS 203.

// Temporarily disable generic module due to const generics issues
// pub mod generic;
pub mod kyber512;
pub mod kyber768;
pub mod kyber1024;

// Re-export the main types  
pub use kyber512::Kyber512;
pub use kyber768::Kyber768;
pub use kyber1024::Kyber1024;

use crate::{
    error::{Error, Result},
    params::kyber::*,
    poly::{Poly, PolyVec, PolyMat},
    traits::Serializable,
    hash::{prf, expand_matrix_a},
    utils::{compress_poly, decompress_poly},
};
use zeroize::Zeroize;

/// Common Kyber functionality
pub(crate) mod common {
    use super::*;
    
    /// Generate matrix A from seed
    pub fn _gen_matrix<const K: usize>(seed: &[u8], transposed: bool) -> Result<PolyMat<N, K, K>> {
        let matrix_vec = expand_matrix_a::<N, K, K>(seed, transposed)?;
        let mut matrix = PolyMat::zero();
        
        for (_i, (row, matrix_row)) in matrix.rows.iter_mut().zip(matrix_vec.iter()).enumerate().take(K) {
            for (_j, (poly, &coeffs)) in row.polys.iter_mut().zip(matrix_row.iter()).enumerate().take(K) {
                poly.coeffs = coeffs;
            }
        }
        
        Ok(matrix)
    }
    
    /// Sample polynomial vector from seed
    pub fn _sample_poly_vec<const K: usize>(
        seed: &[u8],
        nonce: u8,
        eta: usize
    ) -> Result<PolyVec<N, K>> {
        let mut vec = PolyVec::zero();
        
        for i in 0..K {
            // Dynamic allocation for variable eta
            let buf_size = 64 * eta;
            let mut buf = vec![0u8; buf_size];
            let prf_output = prf(seed, nonce + i as u8);
            buf[..32].copy_from_slice(&prf_output);
            
            if eta > 2 {
                let prf_output2 = prf(seed, nonce + i as u8 + K as u8);
                buf[32..64].copy_from_slice(&prf_output2);
            }
            
            vec.polys[i] = Poly::cbd(&buf[..eta * 64 / 4], eta)?;
        }
        
        Ok(vec)
    }
    
    /// Pack ciphertext
    pub fn _pack_ciphertext<const K: usize>(
        b: &PolyVec<N, K>,
        v: &Poly<N>,
        du: usize,
        dv: usize,
    ) -> Result<Vec<u8>> {
        let mut ct = vec![0u8; K * du * N / 8 + dv * N / 8];
        
        // Compress and pack b
        for (i, poly) in b.polys.iter().enumerate().take(K) {
            let offset = i * du * N / 8;
            compress_poly(&poly.coeffs, du, &mut ct[offset..offset + du * N / 8])?;
        }
        
        // Compress and pack v
        let offset = K * du * N / 8;
        compress_poly(&v.coeffs, dv, &mut ct[offset..])?;
        
        Ok(ct)
    }
    
    /// Unpack ciphertext
    pub fn _unpack_ciphertext<const K: usize>(
        ct: &[u8],
        du: usize,
        dv: usize,
    ) -> Result<(PolyVec<N, K>, Poly<N>)> {
        if ct.len() != K * du * N / 8 + dv * N / 8 {
            return Err(Error::InvalidCiphertext);
        }
        
        let mut b = PolyVec::zero();
        let mut v = Poly::zero();
        
        // Decompress and unpack b
        for (i, poly) in b.polys.iter_mut().enumerate().take(K) {
            let offset = i * du * N / 8;
            decompress_poly(&ct[offset..offset + du * N / 8], du, &mut poly.coeffs)?;
        }
        
        // Decompress and unpack v
        let offset = K * du * N / 8;
        decompress_poly(&ct[offset..], dv, &mut v.coeffs)?;
        
        Ok((b, v))
    }
}

/// Kyber public key
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKey<const K: usize> {
    /// Packed public key bytes
    pub bytes: Vec<u8>,
}

impl<const K: usize> AsRef<[u8]> for PublicKey<K> {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl<const K: usize> Serializable for PublicKey<K> {
    fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
    
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Calculate expected size based on K
        let expected_size = match K {
            2 => 800,   // Kyber512: POLYVECBYTES (2*384) + SYMBYTES (32)
            3 => 1184,  // Kyber768: POLYVECBYTES (3*384) + SYMBYTES (32)
            4 => 1568,  // Kyber1024: POLYVECBYTES (4*384) + SYMBYTES (32)
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

/// Kyber secret key
#[derive(Clone, Debug)]
pub struct SecretKey<const K: usize> {
    /// Packed secret key bytes
    pub bytes: Vec<u8>,
}

impl<const K: usize> AsRef<[u8]> for SecretKey<K> {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl<const K: usize> Zeroize for SecretKey<K> {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl<const K: usize> Drop for SecretKey<K> {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl<const K: usize> Serializable for SecretKey<K> {
    fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
    
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Calculate expected size based on K
        // SECRET_KEY_SIZE = POLYVECBYTES + PUBLIC_KEY_SIZE + 32 + 32
        let expected_size = match K {
            2 => 1632,  // Kyber512: 768 + 800 + 32 + 32
            3 => 2400,  // Kyber768: 1152 + 1184 + 32 + 32  
            4 => 3168,  // Kyber1024: 1536 + 1568 + 32 + 32
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

/// Kyber ciphertext
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ciphertext<const K: usize> {
    /// Packed ciphertext bytes
    pub bytes: Vec<u8>,
}

impl<const K: usize> AsRef<[u8]> for Ciphertext<K> {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl<const K: usize> Serializable for Ciphertext<K> {
    fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
    
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // Calculate expected size based on K
        // CIPHERTEXT_SIZE = POLYVECCOMPRESSEDBYTES + POLYCOMPRESSEDBYTES
        let expected_size = match K {
            2 => 768,   // Kyber512: 2*320 + 128
            3 => 1088,  // Kyber768: 3*320 + 128
            4 => 1568,  // Kyber1024: 4*352 + 160
            _ => return Err(Error::InvalidParameter),
        };
        
        if bytes.len() != expected_size {
            return Err(Error::InvalidCiphertext);
        }
        
        Ok(Self {
            bytes: bytes.to_vec(),
        })
    }
}

/// Kyber shared secret
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedSecret {
    /// Shared secret bytes
    pub bytes: [u8; 32],
}

impl AsRef<[u8]> for SharedSecret {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl Zeroize for SharedSecret {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for SharedSecret {
    fn drop(&mut self) {
        self.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_public_key_serialization() {
        // Create a properly sized public key for Kyber512 (K=2)
        let pk = PublicKey::<2> {
            bytes: vec![0u8; 800], // Kyber512 public key size
        };
        
        let bytes = pk.to_bytes();
        let pk2 = PublicKey::<2>::from_bytes(&bytes).unwrap();
        
        assert_eq!(pk, pk2);
    }
    
    #[test]
    fn test_invalid_public_key_size() {
        // Test that invalid sizes are rejected
        let bytes = vec![1, 2, 3, 4, 5];
        let result = PublicKey::<2>::from_bytes(&bytes);
        assert!(result.is_err());
    }
}