//! Generic Kyber implementation using trait-based parameters
//!
//! This module provides a generic implementation of the ML-KEM (Kyber) algorithm
//! that works for all security levels using trait-based parameter sets.

use crate::{
    error::{Error, Result},
    params::kyber::{N, Q},
    poly::{Poly, PolyVec, PolyMat},
    traits::{Kem, SecureRandom, Serializable},
    hash::{g, h, kdf, prf, expand_matrix_a},
    utils::{compress_poly, decompress_poly, compress_polyvec, decompress_polyvec},
    kyber::{PublicKey, SecretKey, Ciphertext, SharedSecret},
    constant_time,
};
use zeroize::Zeroize;

/// Trait defining Kyber parameters for different security levels
pub trait KyberParams: Send + Sync + 'static {
    /// Module dimension (number of polynomials in vector/matrix)
    const K: usize;
    
    /// Noise distribution parameter for key generation
    const ETA1: usize;
    
    /// Noise distribution parameter for encryption  
    const ETA2: usize;
    
    /// Ciphertext compression parameter for u component
    const DU: usize;
    
    /// Ciphertext compression parameter for v component
    const DV: usize;
    
    /// Size of polynomial vector in bytes
    const POLYVEC_BYTES: usize;
    
    /// Size of compressed polynomial in bytes
    const POLY_COMPRESSED_BYTES: usize;
    
    /// Size of compressed polynomial vector in bytes
    const POLYVEC_COMPRESSED_BYTES: usize;
    
    /// Public key size in bytes
    const PUBLIC_KEY_SIZE: usize;
    
    /// Secret key size in bytes
    const SECRET_KEY_SIZE: usize;
    
    /// Ciphertext size in bytes
    const CIPHERTEXT_SIZE: usize;
    
    /// Shared secret size in bytes
    const SHARED_SECRET_SIZE: usize = 32;
    
    /// Get parameter name for debugging
    const NAME: &'static str;
}

/// Generic Kyber implementation
#[derive(Debug, Clone)]
pub struct GenericKyber<P: KyberParams> {
    _phantom: core::marker::PhantomData<P>,
}

impl<P: KyberParams> GenericKyber<P> {
    /// Create a new instance
    pub fn new() -> Self {
        Self {
            _phantom: core::marker::PhantomData,
        }
    }
    
    /// Generate a keypair
    fn generate_keypair_impl<R: SecureRandom, const K: usize>(
        rng: &mut R
    ) -> Result<(PublicKey<K>, SecretKey<K>)>
    where
        P: KyberParams<K = K>,
    {
        // Generate random seeds
        let mut d = [0u8; 32];
        rng.fill_bytes(&mut d);
        
        // Compute matrix A seed and polynomial vector seeds
        let (rho, sigma) = g(&d);
        
        // Generate matrix A
        let a = expand_matrix_a::<N, {P::K}, {P::K}>(&rho, false)?;
        let mut a_mat = PolyMat::zero();
        for i in 0..P::K {
            for j in 0..P::K {
                a_mat.rows[i].polys[j].coeffs = a[i][j];
            }
        }
        
        // Sample secret vector s
        let mut s = PolyVec::zero();
        let mut nonce = 0u8;
        for i in 0..P::K {
            let prf_output = prf(&sigma, nonce);
            s.polys[i] = Poly::cbd(&prf_output, P::ETA1)?;
            nonce += 1;
        }
        
        // Sample error vector e
        let mut e = PolyVec::zero();
        for i in 0..P::K {
            let prf_output = prf(&sigma, nonce);
            e.polys[i] = Poly::cbd(&prf_output, P::ETA1)?;
            nonce += 1;
        }
        
        // Convert to NTT form
        for i in 0..P::K {
            s.polys[i].ntt();
            e.polys[i].ntt();
        }
        for i in 0..P::K {
            for j in 0..P::K {
                a_mat.rows[i].polys[j].ntt();
            }
        }
        
        // Compute t = As + e
        let mut t = PolyVec::zero();
        for i in 0..P::K {
            for j in 0..P::K {
                let mut tmp = a_mat.rows[i].polys[j].clone();
                tmp.pointwise_montgomery(&s.polys[j]);
                t.polys[i] = t.polys[i].add(&tmp);
            }
            t.polys[i] = t.polys[i].add(&e.polys[i]);
            t.polys[i].reduce();
        }
        
        // Pack public key
        let mut pk_bytes = Vec::with_capacity(P::PUBLIC_KEY_SIZE);
        
        // Convert t back from NTT for packing
        let mut t_normal = t.clone();
        for i in 0..P::K {
            t_normal.polys[i].inv_ntt();
        }
        
        // Pack t
        for i in 0..P::K {
            let packed = t_normal.polys[i].pack();
            pk_bytes.extend_from_slice(&packed);
        }
        
        // Append rho
        pk_bytes.extend_from_slice(&rho);
        
        // Pack secret key
        let mut sk_bytes = Vec::with_capacity(P::SECRET_KEY_SIZE);
        
        // Convert s back from NTT for packing
        let mut s_normal = s.clone();
        for i in 0..P::K {
            s_normal.polys[i].inv_ntt();
        }
        
        // Pack s
        for i in 0..P::K {
            let packed = s_normal.polys[i].pack();
            sk_bytes.extend_from_slice(&packed);
        }
        
        // Append public key
        sk_bytes.extend_from_slice(&pk_bytes);
        
        // Append H(pk)
        let h_pk = h(&pk_bytes);
        sk_bytes.extend_from_slice(&h_pk);
        
        // Append z (random value for implicit rejection)
        let mut z = [0u8; 32];
        rng.fill_bytes(&mut z);
        sk_bytes.extend_from_slice(&z);
        
        Ok((
            PublicKey { bytes: pk_bytes },
            SecretKey { bytes: sk_bytes }
        ))
    }
    
    /// Encapsulate a shared secret
    fn encapsulate_impl<R: SecureRandom>(
        public_key: &PublicKey<{P::K}>,
        rng: &mut R
    ) -> Result<(Ciphertext<{P::K}>, SharedSecret)> {
        // Generate random message
        let mut m = [0u8; 32];
        rng.fill_bytes(&mut m);
        
        // Compute H(pk)
        let h_pk = h(&public_key.bytes);
        
        // Compute (K', r) = G(m||H(pk))
        let mut input = Vec::with_capacity(64);
        input.extend_from_slice(&m);
        input.extend_from_slice(&h_pk);
        let (k_prime, r) = g(&input);
        
        // Unpack public key
        let (t_hat, rho) = Self::unpack_public_key(public_key)?;
        
        // Generate matrix A from rho
        let a = expand_matrix_a::<N, {P::K}, {P::K}>(&rho, true)?; // transposed
        let mut a_t = PolyMat::zero();
        for i in 0..P::K {
            for j in 0..P::K {
                a_t.rows[i].polys[j].coeffs = a[i][j];
                a_t.rows[i].polys[j].ntt();
            }
        }
        
        // Sample error vectors r, e1, e2
        let mut r_vec = PolyVec::zero();
        let mut e1 = PolyVec::zero();
        let mut e2 = Poly::zero();
        
        let mut nonce = 0u8;
        for i in 0..P::K {
            let prf_output = prf(&r, nonce);
            r_vec.polys[i] = Poly::cbd(&prf_output, P::ETA1)?;
            r_vec.polys[i].ntt();
            nonce += 1;
        }
        
        for i in 0..P::K {
            let prf_output = prf(&r, nonce);  
            e1.polys[i] = Poly::cbd(&prf_output, P::ETA2)?;
            nonce += 1;
        }
        
        let prf_output = prf(&r, nonce);
        e2 = Poly::cbd(&prf_output, P::ETA2)?;
        
        // Compute u = A^T * r + e1
        let mut u = PolyVec::zero();
        for i in 0..P::K {
            for j in 0..P::K {
                let mut tmp = a_t.rows[j].polys[i].clone();
                tmp.pointwise_montgomery(&r_vec.polys[j]);
                u.polys[i] = u.polys[i].add(&tmp);
            }
            u.polys[i].inv_ntt();
            u.polys[i] = u.polys[i].add(&e1.polys[i]);
            u.polys[i].reduce();
        }
        
        // Compute v = t^T * r + e2 + Decompress_q(Encode(m))
        let mut v = Poly::zero();
        for i in 0..P::K {
            let mut tmp = t_hat.polys[i].clone();
            tmp.pointwise_montgomery(&r_vec.polys[i]);
            v = v.add(&tmp);
        }
        v.inv_ntt();
        v = v.add(&e2);
        
        // Add message
        let m_poly = Self::encode_message(&m);
        v = v.add(&m_poly);
        v.reduce();
        
        // Pack ciphertext
        let ct_bytes = Self::pack_ciphertext(&u, &v)?;
        
        // Compute shared secret
        let ss_bytes = kdf(&k_prime, &h(&ct_bytes));
        
        Ok((
            Ciphertext { bytes: ct_bytes },
            SharedSecret { bytes: ss_bytes }
        ))
    }
    
    /// Decapsulate a shared secret
    fn decapsulate_impl(
        secret_key: &SecretKey<{P::K}>,
        ciphertext: &Ciphertext<{P::K}>
    ) -> Result<SharedSecret> {
        // Unpack secret key
        let (s_hat, pk, h_pk, z) = Self::unpack_secret_key(secret_key)?;
        
        // Unpack ciphertext
        let (u, v) = Self::unpack_ciphertext(ciphertext)?;
        
        // Compute m' = Encode(Decompress_q(v - s^T * u))
        let mut s_t_u = Poly::zero();
        for i in 0..P::K {
            let mut tmp = s_hat.polys[i].clone();
            tmp.pointwise_montgomery(&u.polys[i]);
            s_t_u = s_t_u.add(&tmp);
        }
        s_t_u.inv_ntt();
        
        let mut m_recovered = v.sub(&s_t_u);
        m_recovered.reduce();
        
        let m_prime = Self::decode_message(&m_recovered);
        
        // Compute (K'', r') = G(m'||H(pk))
        let mut input = Vec::with_capacity(64);
        input.extend_from_slice(&m_prime);
        input.extend_from_slice(&h_pk);
        let (k_prime_prime, r_prime) = g(&input);
        
        // Re-encrypt with m' to get c'
        let (ct_prime, _) = Self::encrypt_with_coins(&pk, &m_prime, &r_prime)?;
        
        // Check if c == c'
        let ct_eq = constant_time::ct_memcmp(&ciphertext.bytes, &ct_prime.bytes);
        
        // Compute shared secrets
        let k_normal = kdf(&k_prime_prime, &h(&ciphertext.bytes));
        let k_rejection = kdf(&z, &h(&ciphertext.bytes));
        
        // Use constant-time selection
        let mut ss_bytes = [0u8; 32];
        for i in 0..32 {
            ss_bytes[i] = constant_time::ct_select_u8(ct_eq, k_normal[i], k_rejection[i]);
        }
        
        Ok(SharedSecret { bytes: ss_bytes })
    }
    
    /// Unpack public key
    fn unpack_public_key(pk: &PublicKey<{P::K}>) -> Result<(PolyVec<N, {P::K}>, [u8; 32])> {
        if pk.bytes.len() != P::PUBLIC_KEY_SIZE {
            return Err(Error::InvalidKeySize);
        }
        
        let mut t_hat = PolyVec::zero();
        let mut offset = 0;
        
        // Unpack t
        for i in 0..P::K {
            t_hat.polys[i] = Poly::unpack(&pk.bytes[offset..offset + 384])?;
            t_hat.polys[i].ntt();
            offset += 384;
        }
        
        // Extract rho
        let mut rho = [0u8; 32];
        rho.copy_from_slice(&pk.bytes[offset..offset + 32]);
        
        Ok((t_hat, rho))
    }
    
    /// Unpack secret key
    fn unpack_secret_key(sk: &SecretKey<{P::K}>) -> Result<(PolyVec<N, {P::K}>, Vec<u8>, [u8; 32], [u8; 32])> {
        if sk.bytes.len() != P::SECRET_KEY_SIZE {
            return Err(Error::InvalidKeySize);
        }
        
        let mut s_hat = PolyVec::zero();
        let mut offset = 0;
        
        // Unpack s
        for i in 0..P::K {
            s_hat.polys[i] = Poly::unpack(&sk.bytes[offset..offset + 384])?;
            s_hat.polys[i].ntt();
            offset += 384;
        }
        
        // Extract public key
        let pk_len = P::PUBLIC_KEY_SIZE;
        let pk = sk.bytes[offset..offset + pk_len].to_vec();
        offset += pk_len;
        
        // Extract H(pk)
        let mut h_pk = [0u8; 32];
        h_pk.copy_from_slice(&sk.bytes[offset..offset + 32]);
        offset += 32;
        
        // Extract z
        let mut z = [0u8; 32];
        z.copy_from_slice(&sk.bytes[offset..offset + 32]);
        
        Ok((s_hat, pk, h_pk, z))
    }
    
    /// Pack ciphertext
    fn pack_ciphertext(u: &PolyVec<N, {P::K}>, v: &Poly<N>) -> Result<Vec<u8>> {
        let mut ct = Vec::with_capacity(P::CIPHERTEXT_SIZE);
        
        // Compress and pack u
        for i in 0..P::K {
            let compressed = compress_polyvec(&u.polys[i], P::DU)?;
            ct.extend_from_slice(&compressed);
        }
        
        // Compress and pack v
        let compressed_v = compress_poly(&v.coeffs, P::DV)?;
        ct.extend_from_slice(&compressed_v);
        
        Ok(ct)
    }
    
    /// Unpack ciphertext
    fn unpack_ciphertext(ct: &Ciphertext<{P::K}>) -> Result<(PolyVec<N, {P::K}>, Poly<N>)> {
        if ct.bytes.len() != P::CIPHERTEXT_SIZE {
            return Err(Error::InvalidCiphertext);
        }
        
        let mut u = PolyVec::zero();
        let mut offset = 0;
        
        // Decompress and unpack u
        let du_bytes = P::DU * N / 8;
        for i in 0..P::K {
            let mut coeffs = [0i16; N];
            decompress_poly(&ct.bytes[offset..offset + du_bytes], P::DU, &mut coeffs)?;
            u.polys[i] = Poly::from_coeffs(coeffs);
            u.polys[i].ntt();
            offset += du_bytes;
        }
        
        // Decompress and unpack v
        let dv_bytes = P::DV * N / 8;
        let mut v_coeffs = [0i16; N];
        decompress_poly(&ct.bytes[offset..], P::DV, &mut v_coeffs)?;
        let v = Poly::from_coeffs(v_coeffs);
        
        Ok((u, v))
    }
    
    /// Encode message as polynomial
    fn encode_message(m: &[u8; 32]) -> Poly<N> {
        let mut poly = Poly::zero();
        for i in 0..32 {
            for j in 0..8 {
                let bit = (m[i] >> j) & 1;
                poly.coeffs[8 * i + j] = (bit as i16) * (Q / 2);
            }
        }
        poly
    }
    
    /// Decode message from polynomial
    fn decode_message(poly: &Poly<N>) -> [u8; 32] {
        let mut m = [0u8; 32];
        for i in 0..32 {
            for j in 0..8 {
                let bit = constant_time::ct_compress_q(poly.coeffs[8 * i + j], 1);
                m[i] |= (bit as u8) << j;
            }
        }
        m
    }
    
    /// Encrypt with specific coins (for re-encryption in decapsulation)
    fn encrypt_with_coins(
        pk_bytes: &[u8],
        m: &[u8; 32],
        coins: &[u8; 32]
    ) -> Result<(Ciphertext<{P::K}>, SharedSecret)> {
        let pk = PublicKey { bytes: pk_bytes.to_vec() };
        
        // Use a deterministic RNG based on coins
        struct DeterministicRng<'a> {
            coins: &'a [u8; 32],
            counter: usize,
        }
        
        impl<'a> SecureRandom for DeterministicRng<'a> {
            fn fill_bytes(&mut self, dest: &mut [u8]) {
                for i in 0..dest.len() {
                    dest[i] = self.coins[self.counter % 32];
                    self.counter += 1;
                }
            }
        }
        
        let mut rng = DeterministicRng { coins, counter: 0 };
        
        // Use the message directly instead of generating random
        let h_pk = h(&pk.bytes);
        let mut input = Vec::with_capacity(64);
        input.extend_from_slice(m);
        input.extend_from_slice(&h_pk);
        let (k_prime, _) = g(&input);
        
        // Perform encryption with the given coins
        let (ct, _) = Self::encapsulate_impl(&pk, &mut rng)?;
        let ss = SharedSecret { bytes: k_prime };
        
        Ok((ct, ss))
    }
}

impl<P: KyberParams> Kem for GenericKyber<P> {
    type PublicKey = PublicKey<{P::K}>;
    type SecretKey = SecretKey<{P::K}>;
    type Ciphertext = Ciphertext<{P::K}>;
    type SharedSecret = SharedSecret;
    
    const PUBLIC_KEY_SIZE: usize = P::PUBLIC_KEY_SIZE;
    const SECRET_KEY_SIZE: usize = P::SECRET_KEY_SIZE;
    const CIPHERTEXT_SIZE: usize = P::CIPHERTEXT_SIZE;
    const SHARED_SECRET_SIZE: usize = P::SHARED_SECRET_SIZE;
    
    fn generate_keypair<R: SecureRandom>(
        rng: &mut R
    ) -> Result<(Self::PublicKey, Self::SecretKey)> {
        Self::generate_keypair_impl(rng)
    }
    
    fn encapsulate<R: SecureRandom>(
        public_key: &Self::PublicKey,
        rng: &mut R
    ) -> Result<(Self::Ciphertext, Self::SharedSecret)> {
        Self::encapsulate_impl(public_key, rng)
    }
    
    fn decapsulate(
        secret_key: &Self::SecretKey,
        ciphertext: &Self::Ciphertext
    ) -> Result<Self::SharedSecret> {
        Self::decapsulate_impl(secret_key, ciphertext)
    }
}

impl<P: KyberParams> Default for GenericKyber<P> {
    fn default() -> Self {
        Self::new()
    }
}

// Implement Send and Sync for GenericKyber
unsafe impl<P: KyberParams> Send for GenericKyber<P> {}
unsafe impl<P: KyberParams> Sync for GenericKyber<P> {}

/// Kyber512 parameter set
pub struct Kyber512Params;

impl KyberParams for Kyber512Params {
    const K: usize = 2;
    const ETA1: usize = 3;
    const ETA2: usize = 2;
    const DU: usize = 10;
    const DV: usize = 4;
    const POLYVEC_BYTES: usize = Self::K * 384;
    const POLY_COMPRESSED_BYTES: usize = 128;
    const POLYVEC_COMPRESSED_BYTES: usize = Self::K * 320;
    const PUBLIC_KEY_SIZE: usize = Self::POLYVEC_BYTES + SYMBYTES;
    const SECRET_KEY_SIZE: usize = Self::POLYVEC_BYTES + Self::PUBLIC_KEY_SIZE + 32 + 32;
    const CIPHERTEXT_SIZE: usize = Self::POLYVEC_COMPRESSED_BYTES + Self::POLY_COMPRESSED_BYTES;
    const NAME: &'static str = "Kyber512";
}

/// Kyber768 parameter set
pub struct Kyber768Params;

impl KyberParams for Kyber768Params {
    const K: usize = 3;
    const ETA1: usize = 2;
    const ETA2: usize = 2;
    const DU: usize = 10;
    const DV: usize = 4;
    const POLYVEC_BYTES: usize = Self::K * 384;
    const POLY_COMPRESSED_BYTES: usize = 128;
    const POLYVEC_COMPRESSED_BYTES: usize = Self::K * 320;
    const PUBLIC_KEY_SIZE: usize = Self::POLYVEC_BYTES + SYMBYTES;
    const SECRET_KEY_SIZE: usize = Self::POLYVEC_BYTES + Self::PUBLIC_KEY_SIZE + 32 + 32;
    const CIPHERTEXT_SIZE: usize = Self::POLYVEC_COMPRESSED_BYTES + Self::POLY_COMPRESSED_BYTES;
    const NAME: &'static str = "Kyber768";
}

/// Kyber1024 parameter set
pub struct Kyber1024Params;

impl KyberParams for Kyber1024Params {
    const K: usize = 4;
    const ETA1: usize = 2;
    const ETA2: usize = 2;
    const DU: usize = 11;
    const DV: usize = 5;
    const POLYVEC_BYTES: usize = Self::K * 384;
    const POLY_COMPRESSED_BYTES: usize = 160;
    const POLYVEC_COMPRESSED_BYTES: usize = Self::K * 352;
    const PUBLIC_KEY_SIZE: usize = Self::POLYVEC_BYTES + SYMBYTES;
    const SECRET_KEY_SIZE: usize = Self::POLYVEC_BYTES + Self::PUBLIC_KEY_SIZE + 32 + 32;
    const CIPHERTEXT_SIZE: usize = Self::POLYVEC_COMPRESSED_BYTES + Self::POLY_COMPRESSED_BYTES;
    const NAME: &'static str = "Kyber1024";
}

/// Type aliases for the specific Kyber variants
pub type GenericKyber512 = GenericKyber<Kyber512Params>;
pub type GenericKyber768 = GenericKyber<Kyber768Params>;  
pub type GenericKyber1024 = GenericKyber<Kyber1024Params>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::TestRng;
    
    #[test]
    fn test_generic_kyber512() {
        let mut rng = TestRng::new(42);
        let kyber = GenericKyber512::new();
        
        // Generate keypair
        let (pk, sk) = GenericKyber512::generate_keypair(&mut rng).unwrap();
        
        // Test sizes
        assert_eq!(pk.bytes.len(), GenericKyber512::PUBLIC_KEY_SIZE);
        assert_eq!(sk.bytes.len(), GenericKyber512::SECRET_KEY_SIZE);
        
        // Encapsulate
        let (ct, ss1) = GenericKyber512::encapsulate(&pk, &mut rng).unwrap();
        assert_eq!(ct.bytes.len(), GenericKyber512::CIPHERTEXT_SIZE);
        assert_eq!(ss1.bytes.len(), GenericKyber512::SHARED_SECRET_SIZE);
        
        // Decapsulate
        let ss2 = GenericKyber512::decapsulate(&sk, &ct).unwrap();
        assert_eq!(ss2.bytes.len(), GenericKyber512::SHARED_SECRET_SIZE);
        
        // Shared secrets should match
        assert_eq!(ss1.bytes, ss2.bytes);
    }
    
    #[test]
    fn test_generic_kyber768() {
        let mut rng = TestRng::new(123);
        
        // Generate keypair
        let (pk, sk) = GenericKyber768::generate_keypair(&mut rng).unwrap();
        
        // Test sizes
        assert_eq!(pk.bytes.len(), GenericKyber768::PUBLIC_KEY_SIZE);
        assert_eq!(sk.bytes.len(), GenericKyber768::SECRET_KEY_SIZE);
        
        // Encapsulate
        let (ct, ss1) = GenericKyber768::encapsulate(&pk, &mut rng).unwrap();
        assert_eq!(ct.bytes.len(), GenericKyber768::CIPHERTEXT_SIZE);
        
        // Decapsulate
        let ss2 = GenericKyber768::decapsulate(&sk, &ct).unwrap();
        
        // Shared secrets should match
        assert_eq!(ss1.bytes, ss2.bytes);
    }
    
    #[test]
    fn test_generic_kyber1024() {
        let mut rng = TestRng::new(456);
        
        // Generate keypair
        let (pk, sk) = GenericKyber1024::generate_keypair(&mut rng).unwrap();
        
        // Test sizes
        assert_eq!(pk.bytes.len(), GenericKyber1024::PUBLIC_KEY_SIZE);
        assert_eq!(sk.bytes.len(), GenericKyber1024::SECRET_KEY_SIZE); 
        
        // Encapsulate
        let (ct, ss1) = GenericKyber1024::encapsulate(&pk, &mut rng).unwrap();
        assert_eq!(ct.bytes.len(), GenericKyber1024::CIPHERTEXT_SIZE);
        
        // Decapsulate
        let ss2 = GenericKyber1024::decapsulate(&sk, &ct).unwrap();
        
        // Shared secrets should match
        assert_eq!(ss1.bytes, ss2.bytes);
    }
    
    #[test]
    fn test_parameter_consistency() {
        // Test that parameters match expected values
        assert_eq!(Kyber512Params::K, 2);
        assert_eq!(Kyber512Params::PUBLIC_KEY_SIZE, 800);
        assert_eq!(Kyber512Params::SECRET_KEY_SIZE, 1632);
        assert_eq!(Kyber512Params::CIPHERTEXT_SIZE, 768);
        
        assert_eq!(Kyber768Params::K, 3);
        assert_eq!(Kyber768Params::PUBLIC_KEY_SIZE, 1184);
        assert_eq!(Kyber768Params::SECRET_KEY_SIZE, 2400);
        assert_eq!(Kyber768Params::CIPHERTEXT_SIZE, 1088);
        
        assert_eq!(Kyber1024Params::K, 4);
        assert_eq!(Kyber1024Params::PUBLIC_KEY_SIZE, 1568);
        assert_eq!(Kyber1024Params::SECRET_KEY_SIZE, 3168);
        assert_eq!(Kyber1024Params::CIPHERTEXT_SIZE, 1568);
    }
}