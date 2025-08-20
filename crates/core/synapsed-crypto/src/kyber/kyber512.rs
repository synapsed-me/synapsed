//! Kyber512 implementation (NIST Level 1)
//!
//! This provides a concrete implementation of ML-KEM-512.

use crate::{
    error::{Error, Result},
    params::kyber::kyber512::*,
    params::kyber::{N, Q},
    poly::{Poly, PolyVec, PolyMat},
    traits::{Kem, SecureRandom},
    hash::{g, h, prf, expand_matrix_a},
    utils::{compress_poly, decompress_poly},
    kyber::{PublicKey, SecretKey, Ciphertext, SharedSecret},
};

/// Kyber512 implementation struct
#[derive(Debug, Clone)]
pub struct Kyber512;

impl Kyber512 {
    /// Create a new Kyber512 instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for Kyber512 {
    fn default() -> Self {
        Self::new()
    }
}

impl Kem for Kyber512 {
    type PublicKey = PublicKey<K>;
    type SecretKey = SecretKey<K>;
    type Ciphertext = Ciphertext<K>;
    type SharedSecret = SharedSecret;
    
    const PUBLIC_KEY_SIZE: usize = PUBLIC_KEY_SIZE;
    const SECRET_KEY_SIZE: usize = SECRET_KEY_SIZE;
    const CIPHERTEXT_SIZE: usize = CIPHERTEXT_SIZE;
    const SHARED_SECRET_SIZE: usize = SHARED_SECRET_SIZE;
    
    fn generate_keypair<R: SecureRandom>(rng: &mut R) -> Result<(Self::PublicKey, Self::SecretKey)> {
        // Generate random seed
        let mut d = [0u8; 32];
        rng.fill_bytes(&mut d);
        
        // Hash to get matrix seed and noise seed
        let g_output = g(&d);
        let rho = &g_output[..32];
        let sigma = &g_output[32..];
        
        // Generate matrix A
        let a_matrix = expand_matrix_a::<N, K, K>(rho, false)?;
        let mut a: PolyMat<N, K, K> = PolyMat::zero();
        for (_i, (row, matrix_row)) in a.rows.iter_mut().zip(a_matrix.iter()).enumerate().take(K) {
            for (_j, (poly, &coeffs)) in row.polys.iter_mut().zip(matrix_row.iter()).enumerate().take(K) {
                poly.coeffs = coeffs;
            }
        }
        
        // Sample secret vector s
        let mut s: PolyVec<N, K> = PolyVec::zero();
        let mut nonce = 0u8;
        for i in 0..K {
            let prf_output = prf(sigma, nonce);
            s.polys[i] = Poly::cbd(&prf_output, ETA1)?;
            nonce += 1;
        }
        
        // Sample error vector e
        let mut e: PolyVec<N, K> = PolyVec::zero();
        for i in 0..K {
            let prf_output = prf(sigma, nonce);
            e.polys[i] = Poly::cbd(&prf_output, ETA1)?;
            nonce += 1;
        }
        
        // Convert to NTT domain
        s.ntt();
        e.ntt();
        
        // Compute t = As + e
        let mut t = a.mul_vec(&s);
        t = t + e;
        
        // Pack public key
        let mut pk_bytes = Vec::with_capacity(PUBLIC_KEY_SIZE);
        for i in 0..K {
            let poly_bytes = t.polys[i].pack();
            pk_bytes.extend_from_slice(&poly_bytes);
        }
        pk_bytes.extend_from_slice(rho);
        
        // Pack secret key
        let mut sk_bytes = Vec::with_capacity(SECRET_KEY_SIZE);
        for i in 0..K {
            let poly_bytes = s.polys[i].pack();
            sk_bytes.extend_from_slice(&poly_bytes);
        }
        sk_bytes.extend_from_slice(&pk_bytes);
        sk_bytes.extend_from_slice(&h(&pk_bytes));
        let mut z = [0u8; 32];
        rng.fill_bytes(&mut z);
        sk_bytes.extend_from_slice(&z);
        
        Ok((
            PublicKey { bytes: pk_bytes },
            SecretKey { bytes: sk_bytes },
        ))
    }
    
    fn encapsulate<R: SecureRandom>(public_key: &Self::PublicKey, rng: &mut R) -> Result<(Self::Ciphertext, Self::SharedSecret)> {
        // Generate random coins
        let mut m = [0u8; 32];
        rng.fill_bytes(&mut m);
        
        // Hash message
        let m_hash = h(&m);
        
        // Extract rho from public key
        let rho = &public_key.bytes[K * 384..];
        
        // Hash to get random coins
        let mut input = Vec::new();
        input.extend_from_slice(&m_hash);
        input.extend_from_slice(&h(&public_key.bytes));
        let kr = g(&input);
        let k = &kr[..32];
        let r = &kr[32..];
        
        // Generate matrix A  
        let a_matrix = expand_matrix_a::<N, K, K>(rho, true)?; // transposed
        let mut a: PolyMat<N, K, K> = PolyMat::zero();
        for (_i, (row, matrix_row)) in a.rows.iter_mut().zip(a_matrix.iter()).enumerate().take(K) {
            for (_j, (poly, &coeffs)) in row.polys.iter_mut().zip(matrix_row.iter()).enumerate().take(K) {
                poly.coeffs = coeffs;
            }
        }
        
        // Sample noise
        let mut r_vec: PolyVec<N, K> = PolyVec::zero();
        let mut nonce = 0u8;
        for i in 0..K {
            let prf_output = prf(r, nonce);
            r_vec.polys[i] = Poly::cbd(&prf_output, ETA1)?;
            nonce += 1;
        }
        
        let _prf_output = prf(r, nonce);
        let mut e1: PolyVec<N, K> = PolyVec::zero();
        for i in 0..K {
            let prf_output = prf(r, nonce);
            e1.polys[i] = Poly::cbd(&prf_output, ETA2)?;
            nonce += 1;
        }
        
        let prf_output = prf(r, nonce);
        let e2 = Poly::cbd(&prf_output, ETA2)?;
        
        // Convert to NTT
        r_vec.ntt();
        
        // Compute u = A^T * r + e1
        let mut u = a.mul_vec(&r_vec);
        u.inv_ntt();
        u = u + e1;
        
        // Unpack t from public key
        let mut t: PolyVec<N, K> = PolyVec::zero();
        for i in 0..K {
            let offset = i * 384;
            t.polys[i] = Poly::unpack(&public_key.bytes[offset..offset + 384])?;
        }
        
        // Compute v = t^T * r + e2 + decompress(m)
        t.ntt();
        let mut v = t.inner_product(&r_vec);
        v.inv_ntt();
        v = v + e2;
        
        // Add message
        let mut m_poly = Poly::zero();
        for i in 0..256 {
            m_poly.coeffs[i] = if (m_hash[i / 8] >> (i % 8)) & 1 == 1 {
                (Q + 1) / 2
            } else {
                0
            };
        }
        v = v + m_poly;
        
        // Compress and pack ciphertext
        let mut ct_bytes = Vec::with_capacity(CIPHERTEXT_SIZE);
        
        // Compress u
        for i in 0..K {
            let mut compressed_u = vec![0u8; DU * N / 8];
            compress_poly(&u.polys[i].coeffs, DU, &mut compressed_u)?;
            ct_bytes.extend_from_slice(&compressed_u);
        }
        
        // Compress v
        let mut compressed_v = vec![0u8; DV * N / 8];
        compress_poly(&v.coeffs, DV, &mut compressed_v)?;
        ct_bytes.extend_from_slice(&compressed_v);
        
        let k_array: [u8; 32] = k.try_into().map_err(|_| Error::CryptoError)?;
        Ok((
            Ciphertext { bytes: ct_bytes },
            SharedSecret { bytes: k_array },
        ))
    }
    
    fn decapsulate(secret_key: &Self::SecretKey, ciphertext: &Self::Ciphertext) -> Result<Self::SharedSecret> {
        // Extract s from secret key
        let mut s: PolyVec<N, K> = PolyVec::zero();
        for i in 0..K {
            let offset = i * 384;
            s.polys[i] = Poly::unpack(&secret_key.bytes[offset..offset + 384])?;
        }
        
        // Decompress ciphertext
        let mut u: PolyVec<N, K> = PolyVec::zero();
        for i in 0..K {
            let offset = i * DU * N / 8;
            decompress_poly(&ciphertext.bytes[offset..offset + DU * N / 8], DU, &mut u.polys[i].coeffs)?;
        }
        
        let mut v = Poly::zero();
        let v_offset = K * DU * N / 8;
        decompress_poly(&ciphertext.bytes[v_offset..], DV, &mut v.coeffs)?;
        
        // Compute m' = v - s^T * u
        u.ntt();
        let su = s.inner_product(&u);
        let mut su_normal = su;
        su_normal.inv_ntt();
        
        let m_prime = v - su_normal;
        
        // Extract message bits
        let mut m_prime_bytes = [0u8; 32];
        for i in 0..256 {
            let bit = if m_prime.coeffs[i].abs() < Q / 4 { 0 } else { 1 };
            m_prime_bytes[i / 8] |= bit << (i % 8);
        }
        
        // Get hash of public key from secret key
        let pk_start = K * 384;
        let pk_end = pk_start + PUBLIC_KEY_SIZE;
        let pk_hash_start = pk_end;
        let pk_hash = &secret_key.bytes[pk_hash_start..pk_hash_start + 32];
        
        // Compute K'
        let mut input = Vec::new();
        input.extend_from_slice(&m_prime_bytes);
        input.extend_from_slice(pk_hash);
        let kr_prime = g(&input);
        let k_prime = &kr_prime[..32];
        
        let k_prime_array: [u8; 32] = k_prime.try_into().map_err(|_| Error::CryptoError)?;
        Ok(SharedSecret { 
            bytes: k_prime_array 
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Kem;
    use crate::random::TestRng;
    
    #[test]
    fn test_kyber512_keygen() {
        let mut rng = TestRng::new(12345);
        let result = Kyber512::generate_keypair(&mut rng);
        assert!(result.is_ok());
        
        let (pk, sk) = result.unwrap();
        assert_eq!(pk.bytes.len(), PUBLIC_KEY_SIZE);
        assert_eq!(sk.bytes.len(), SECRET_KEY_SIZE);
    }
    
    #[test]
    fn test_kyber512_encapsulate_decapsulate() {
        let mut rng = TestRng::new(12345);
        
        // Generate keypair
        let (pk, sk) = Kyber512::generate_keypair(&mut rng).unwrap();
        
        // Encapsulate
        let (ct, ss1) = Kyber512::encapsulate(&pk, &mut rng).unwrap();
        assert_eq!(ct.bytes.len(), CIPHERTEXT_SIZE);
        assert_eq!(ss1.bytes.len(), SHARED_SECRET_SIZE);
        
        // Decapsulate
        let ss2 = Kyber512::decapsulate(&sk, &ct).unwrap();
        assert_eq!(ss2.bytes.len(), SHARED_SECRET_SIZE);
        
        // Shared secrets should match
        assert_eq!(ss1.bytes, ss2.bytes);
    }
}