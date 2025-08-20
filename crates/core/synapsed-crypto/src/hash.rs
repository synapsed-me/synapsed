//! Hash functions and XOF implementations
//!
//! This module provides hash functions and extendable output functions (XOF)
//! used in post-quantum cryptography algorithms.

use sha3::{
    Digest, Sha3_256, Sha3_512,
    Shake128, Shake256,
    digest::{ExtendableOutput, Update, XofReader},
};
use core::fmt;
use crate::error::{Error, Result};

/// Hash function H (SHA3-256)
pub fn h(input: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    Digest::update(&mut hasher, input);
    hasher.finalize().into()
}

/// Hash function G (SHA3-512)
pub fn g(input: &[u8]) -> [u8; 64] {
    let mut hasher = Sha3_512::new();
    Digest::update(&mut hasher, input);
    hasher.finalize().into()
}

/// PRF using SHAKE256
pub fn prf(key: &[u8], nonce: u8) -> [u8; 32] {
    let mut hasher = Shake256::default();
    Update::update(&mut hasher, key);
    Update::update(&mut hasher, &[nonce]);
    
    let mut output = [0u8; 32];
    hasher.finalize_xof().read(&mut output);
    output
}

/// XOF for matrix generation using SHAKE128
pub struct Xof {
    reader: sha3::Shake128Reader,
}

impl Xof {
    /// Create new XOF instance with seed and indices
    pub fn new(seed: &[u8], i: u8, j: u8) -> Self {
        let mut hasher = Shake128::default();
        Update::update(&mut hasher, seed);
        Update::update(&mut hasher, &[i, j]);
        
        Self {
            reader: hasher.finalize_xof(),
        }
    }
    
    /// Read bytes from XOF
    pub fn read(&mut self, buf: &mut [u8]) {
        self.reader.read(buf);
    }
    
    /// Read and squeeze more bytes
    pub fn squeeze(&mut self, buf: &mut [u8]) {
        self.read(buf);
    }
}

impl fmt::Debug for Xof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Xof")
            .field("reader", &"<Shake128Reader>")
            .finish()
    }
}

/// Sample uniform random field element from XOF output
pub fn sample_uniform(xof: &mut Xof, modulus: i32) -> Result<Option<i32>> {
    // For Kyber (q = 3329)
    if modulus == 3329 {
        let mut buf = [0u8; 3];
        xof.read(&mut buf);
        
        let val = ((buf[0] as u32) | ((buf[1] as u32) << 8) | ((buf[2] as u32) << 16)) & 0xFFF;
        
        if val < 3329 {
            Ok(Some(val as i32))
        } else {
            Ok(None)
        }
    }
    // For Dilithium (q = 8380417)
    else if modulus == 8380417 {
        let mut buf = [0u8; 3];
        xof.read(&mut buf);
        
        let val = ((buf[0] as u32) | ((buf[1] as u32) << 8) | ((buf[2] as u32) << 16)) & 0x7FFFFF;
        
        if val < 8380417 {
            Ok(Some(val as i32))
        } else {
            Ok(None)
        }
    } else {
        Err(Error::UnsupportedModulus)
    }
}

/// Expand seed to polynomial matrix A
pub fn expand_matrix_a<const N: usize, const K: usize, const L: usize>(
    seed: &[u8],
    transposed: bool,
) -> Result<Vec<Vec<[i16; N]>>> {
    let mut matrix = vec![vec![[0i16; N]; L]; K];
    
    for (i, row) in matrix.iter_mut().enumerate().take(K) {
        for (j, poly) in row.iter_mut().enumerate().take(L) {
            let (xof_row, xof_col) = if transposed { (j, i) } else { (i, j) };
            let mut xof = Xof::new(seed, xof_row as u8, xof_col as u8);
            
            let mut poly_idx = 0;
            while poly_idx < N {
                if let Some(val) = sample_uniform(&mut xof, 3329)? {
                    poly[poly_idx] = val as i16;
                    poly_idx += 1;
                }
            }
        }
    }
    
    Ok(matrix)
}

/// KDF function using SHAKE256
pub fn kdf(input: &[u8], output_len: usize) -> Vec<u8> {
    let mut hasher = Shake256::default();
    Update::update(&mut hasher, input);
    
    let mut output = vec![0u8; output_len];
    hasher.finalize_xof().read(&mut output);
    output
}

/// CRH (collision-resistant hash) for Dilithium
pub fn crh(input: &[u8]) -> [u8; 48] {
    let mut hasher = Shake256::default();
    Update::update(&mut hasher, input);
    
    let mut output = [0u8; 48];
    hasher.finalize_xof().read(&mut output);
    output
}

/// Hint bit packing for Dilithium
/// 
/// Packs hint bits into a byte array according to Dilithium specification.
/// The hint vector h contains positions where hints are 1.
/// The function returns exactly omega bytes containing the positions of set hints,
/// padded with zeros.
pub fn pack_hint_bits(h: &[u8], omega: usize) -> Vec<u8> {
    let mut packed = Vec::with_capacity(omega);
    
    // Collect all hint positions (indices where h[i] == 1)
    let mut hint_count = 0;
    
    for (idx, &byte) in h.iter().enumerate() {
        if byte != 0 {
            // In Dilithium, hints are binary (0 or 1)
            // Store the position of each hint
            if hint_count < omega {
                packed.push(idx as u8);
                hint_count += 1;
            } else {
                // If we exceed omega, stop processing
                break;
            }
        }
    }
    
    // Pad with zeros to reach exactly omega bytes
    packed.resize(omega, 0);
    
    packed
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hash_functions() {
        let input = b"test input";
        
        let h_output = h(input);
        assert_eq!(h_output.len(), 32);
        
        let g_output = g(input);
        assert_eq!(g_output.len(), 64);
    }
    
    #[test]
    fn test_xof() {
        let seed = [0u8; 32];
        let mut xof = Xof::new(&seed, 0, 0);
        
        let mut buf = [0u8; 100];
        xof.read(&mut buf);
        
        // Check that we got non-zero output
        assert!(buf.iter().any(|&b| b != 0));
    }
    
    #[test]
    fn test_pack_hint_bits() {
        // Test case 1: Empty hints
        let h1 = vec![0u8; 256];
        let packed1 = pack_hint_bits(&h1, 80);
        assert_eq!(packed1.len(), 80); // exactly omega bytes
        assert!(packed1.iter().all(|&b| b == 0));
        
        // Test case 2: Some hints set
        let mut h2 = vec![0u8; 256];
        h2[10] = 1;
        h2[50] = 1;
        h2[100] = 1;
        h2[200] = 1;
        let packed2 = pack_hint_bits(&h2, 80);
        assert_eq!(packed2.len(), 80);
        assert_eq!(packed2[0], 10);
        assert_eq!(packed2[1], 50);
        assert_eq!(packed2[2], 100);
        assert_eq!(packed2[3], 200);
        assert_eq!(packed2[4], 0); // Rest should be padded with zeros
        
        // Test case 3: Different omega value
        let h3 = vec![0u8; 256];
        let packed3 = pack_hint_bits(&h3, 55);
        assert_eq!(packed3.len(), 55); // exactly omega bytes
        
        // Test case 4: Exceeding omega
        let h4 = vec![1u8; 100]; // All ones
        let packed4 = pack_hint_bits(&h4, 10);
        assert_eq!(packed4.len(), 10); // exactly omega bytes
        // Should only pack first 10 positions
        for (i, &val) in packed4.iter().enumerate().take(10) {
            assert_eq!(val, i as u8);
        }
        
        // Test case 5: Verify padding works correctly
        let mut h5 = vec![0u8; 256];
        h5[5] = 1;
        h5[15] = 1;
        let packed5 = pack_hint_bits(&h5, 20);
        assert_eq!(packed5.len(), 20);
        assert_eq!(packed5[0], 5);
        assert_eq!(packed5[1], 15);
        // Rest should be zeros
        for &val in packed5.iter().skip(2).take(18) {
            assert_eq!(val, 0);
        }
    }
}