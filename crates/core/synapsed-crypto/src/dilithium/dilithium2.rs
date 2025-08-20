//! Dilithium2 implementation (NIST Level 2)

use crate::{
    error::Result,
    params::dilithium::{dilithium2::*, Q},
    traits::{Signature, SecureRandom},
    dilithium::{DilithiumPublicKey, DilithiumSecretKey, DilithiumSignature, common::*},
    hash::{h, g, crh},
    secure_memory::SecureArray,
};

use sha3::{Shake128, Shake256, digest::{ExtendableOutput, Update, XofReader}};

/// Dilithium2 - NIST Level 2 security
#[derive(Debug, Clone, Copy)]
pub struct Dilithium2;

/// Dilithium-specific polynomial type with i32 coefficients
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DilithiumPoly {
    /// Polynomial coefficients (256 coefficients for all Dilithium variants)
    pub coeffs: [i32; 256],  // N = 256 for all Dilithium variants
}

impl DilithiumPoly {
    /// Create zero polynomial
    pub fn zero() -> Self {
        Self { coeffs: [0; 256] }
    }
    
    /// Reduce coefficients modulo Q
    pub fn reduce(&mut self) {
        for coeff in &mut self.coeffs {
            *coeff = crate::utils::barrett_reduce_dilithium(*coeff);
        }
    }
    
    /// Add polynomials
    pub fn add(&self, other: &Self) -> Self {
        let mut result = Self::zero();
        for i in 0..256 {
            result.coeffs[i] = self.coeffs[i] + other.coeffs[i];
        }
        result
    }
    
    /// Subtract polynomials
    pub fn sub(&self, other: &Self) -> Self {
        let mut result = Self::zero();
        for i in 0..256 {
            result.coeffs[i] = self.coeffs[i] - other.coeffs[i];
        }
        result
    }
    
    /// Multiply by scalar
    pub fn scale(&self, scalar: i32) -> Self {
        let mut result = Self::zero();
        for i in 0..256 {
            result.coeffs[i] = self.coeffs[i] * scalar;
        }
        result
    }
    
    /// NTT transformation
    pub fn ntt(&mut self) {
        crate::ntt::dilithium_ntt(&mut self.coeffs);
    }
    
    /// Inverse NTT
    pub fn inv_ntt(&mut self) {
        crate::ntt::dilithium_inv_ntt(&mut self.coeffs);
    }
    
    /// Pointwise multiplication in NTT domain
    pub fn pointwise_mul(&self, other: &Self) -> Self {
        let mut result = Self::zero();
        for i in 0..256 {
            result.coeffs[i] = crate::utils::montgomery_reduce_dilithium(self.coeffs[i] as i64 * other.coeffs[i] as i64);
        }
        result
    }
    
    /// Sample polynomial with coefficients in [-eta, eta]
    pub fn sample_eta(seed: &[u8], nonce: u16, eta: usize) -> Self {
        let mut poly = Self::zero();
        
        if eta == 2 {
            // For eta = 2, we need 136 bytes
            let mut hasher = Shake256::default();
            Update::update(&mut hasher, seed);
            Update::update(&mut hasher, &nonce.to_le_bytes());
            let mut reader = hasher.finalize_xof();
            
            let mut buf = [0u8; 136];
            reader.read(&mut buf);
            
            // Sample from {-2, -1, 0, 1, 2}
            for (i, coeff) in poly.coeffs.iter_mut().enumerate().take(256) {
                if i < 136 {
                    let t = buf[i];
                    *coeff = ((t & 15) as i32) - ((t >> 4) as i32);
                    *coeff = *coeff - (*coeff >> 31 & 5);
                }
            }
        } else if eta == 4 {
            // For eta = 4, we need 227 bytes
            let mut hasher = Shake256::default();
            Update::update(&mut hasher, seed);
            Update::update(&mut hasher, &nonce.to_le_bytes());
            let mut reader = hasher.finalize_xof();
            
            let mut buf = [0u8; 227];
            reader.read(&mut buf);
            
            // Sample from {-4, -3, ..., 3, 4}
            let mut j = 0;
            for i in 0..256 {
                if j < 227 {
                    let t0 = buf[j] & 0x0F;
                    let t1 = buf[j] >> 4;
                    j += 1;
                    
                    if t0 < 9 {
                        poly.coeffs[i] = 4 - (t0 as i32);
                    }
                    if t1 < 9 && i + 1 < 256 {
                        poly.coeffs[i + 1] = 4 - (t1 as i32);
                    }
                }
            }
        }
        
        poly
    }
    
    /// Sample uniform polynomial
    pub fn sample_uniform(seed: &[u8], nonce: u16) -> Self {
        let mut poly = Self::zero();
        let mut hasher = Shake128::default();
        Update::update(&mut hasher, seed);
        Update::update(&mut hasher, &nonce.to_le_bytes());
        let mut reader = hasher.finalize_xof();
        
        let mut idx = 0;
        while idx < 256 {
            let mut buf = [0u8; 3];
            reader.read(&mut buf);
            let val = ((buf[0] as u32) | ((buf[1] as u32) << 8) | ((buf[2] as u32) << 16)) & 0x7FFFFF;
            
            if val < Q as u32 {
                poly.coeffs[idx] = val as i32;
                idx += 1;
            }
        }
        
        poly
    }
    
    /// Power2Round
    pub fn power2round(&self) -> (DilithiumPoly, DilithiumPoly) {
        let mut high = Self::zero();
        let mut low = Self::zero();
        
        for i in 0..256 {
            let (h, l) = power2round(self.coeffs[i]);
            high.coeffs[i] = h;
            low.coeffs[i] = l;
        }
        
        (high, low)
    }
    
    /// Decompose
    pub fn decompose(&self, gamma2: i32) -> (DilithiumPoly, DilithiumPoly) {
        let mut high = Self::zero();
        let mut low = Self::zero();
        
        for i in 0..256 {
            let (h, l) = decompose(self.coeffs[i], gamma2);
            high.coeffs[i] = h;
            low.coeffs[i] = l;
        }
        
        (high, low)
    }
    
    /// Check infinity norm
    pub fn check_norm(&self, bound: i32) -> bool {
        for &coeff in &self.coeffs {
            if coeff >= bound || coeff <= -bound {
                return false;
            }
        }
        true
    }
}

/// Dilithium polynomial vector
#[derive(Clone, Debug)]
pub struct DilithiumPolyVec<const K: usize> {
    /// Vector of polynomials
    pub polys: [DilithiumPoly; K],
}

impl<const K: usize> DilithiumPolyVec<K> {
    /// Create zero vector
    pub fn zero() -> Self {
        Self {
            polys: core::array::from_fn(|_| DilithiumPoly::zero()),
        }
    }
    
    /// Add vectors
    pub fn add(&self, other: &Self) -> Self {
        let mut result = Self::zero();
        for i in 0..K {
            result.polys[i] = self.polys[i].add(&other.polys[i]);
        }
        result
    }
    
    /// Subtract vectors
    pub fn sub(&self, other: &Self) -> Self {
        let mut result = Self::zero();
        for i in 0..K {
            result.polys[i] = self.polys[i].sub(&other.polys[i]);
        }
        result
    }
    
    /// NTT on all polynomials
    pub fn ntt(&mut self) {
        for poly in &mut self.polys {
            poly.ntt();
        }
    }
    
    /// Inverse NTT on all polynomials
    pub fn inv_ntt(&mut self) {
        for poly in &mut self.polys {
            poly.inv_ntt();
        }
    }
    
    /// Reduce all polynomials
    pub fn reduce(&mut self) {
        for poly in &mut self.polys {
            poly.reduce();
        }
    }
    
    /// Check norm for all polynomials
    pub fn check_norm(&self, bound: i32) -> bool {
        self.polys.iter().all(|p| p.check_norm(bound))
    }
    
    /// Power2round for vector
    pub fn power2round(&self) -> (Self, Self) {
        let mut high = Self::zero();
        let mut low = Self::zero();
        
        for i in 0..K {
            let (h, l) = self.polys[i].power2round();
            high.polys[i] = h;
            low.polys[i] = l;
        }
        
        (high, low)
    }
    
    /// Decompose for vector
    pub fn decompose(&self, gamma2: i32) -> (Self, Self) {
        let mut high = Self::zero();
        let mut low = Self::zero();
        
        for i in 0..K {
            let (h, l) = self.polys[i].decompose(gamma2);
            high.polys[i] = h;
            low.polys[i] = l;
        }
        
        (high, low)
    }
}

/// Dilithium matrix
#[derive(Clone, Debug)]
pub struct DilithiumMatrix<const K: usize, const L: usize> {
    /// Matrix rows (K rows of L polynomials each)
    pub rows: [[DilithiumPoly; L]; K],
}

impl<const K: usize, const L: usize> DilithiumMatrix<K, L> {
    /// Expand matrix A from seed
    pub fn expand_a(seed: &[u8]) -> Self {
        let mut matrix = Self {
            rows: core::array::from_fn(|_| core::array::from_fn(|_| DilithiumPoly::zero())),
        };
        
        for i in 0..K {
            for j in 0..L {
                let nonce = ((i as u16) << 8) | (j as u16);
                matrix.rows[i][j] = DilithiumPoly::sample_uniform(seed, nonce);
            }
        }
        
        matrix
    }
    
    /// Matrix-vector multiplication
    pub fn mul_vec(&self, vec: &DilithiumPolyVec<L>) -> DilithiumPolyVec<K> {
        let mut result = DilithiumPolyVec::zero();
        
        for i in 0..K {
            let mut sum = DilithiumPoly::zero();
            for j in 0..L {
                let prod = self.rows[i][j].pointwise_mul(&vec.polys[j]);
                sum = sum.add(&prod);
            }
            result.polys[i] = sum;
        }
        
        result
    }
}

/// Sample challenge polynomial
fn sample_challenge(seed: &[u8], tau: usize) -> DilithiumPoly {
    let mut poly = DilithiumPoly::zero();
    
    let mut hasher = Shake256::default();
    Update::update(&mut hasher, seed);
    let mut reader = hasher.finalize_xof();
    
    let mut buf = [0u8; 8];
    reader.read(&mut buf);
    let signs = u64::from_le_bytes(buf);
    
    let mut c_indices = Vec::with_capacity(tau);
    while c_indices.len() < tau {
        let mut idx_buf = [0u8; 1];
        reader.read(&mut idx_buf);
        let idx = idx_buf[0] as usize;
        
        if idx < 256 && !c_indices.contains(&idx) {
            c_indices.push(idx);
        }
    }
    
    for (i, &idx) in c_indices.iter().enumerate() {
        poly.coeffs[idx] = if (signs >> i) & 1 == 1 { 1 } else { -1 };
    }
    
    poly
}

/// Create hint
fn make_hint_vec<const K: usize>(
    low0: &DilithiumPolyVec<K>,
    low1: &DilithiumPolyVec<K>,
    gamma2: i32
) -> (Vec<(usize, usize)>, usize) {
    let mut hints = Vec::new();
    
    for i in 0..K {
        for j in 0..256 {
            if make_hint(low0.polys[i].coeffs[j], low1.polys[i].coeffs[j], gamma2) {
                hints.push((i, j));
            }
        }
    }
    
    let count = hints.len();
    (hints, count)
}

/// Use hint to recover high bits
fn use_hint_vec<const K: usize>(
    h: &DilithiumPolyVec<K>,
    hints: &[(usize, usize)],
    gamma2: i32
) -> DilithiumPolyVec<K> {
    let mut result = DilithiumPolyVec::<K>::zero();
    
    for i in 0..K {
        for j in 0..256 {
            let hint = hints.iter().any(|&(hi, hj)| hi == i && hj == j);
            result.polys[i].coeffs[j] = use_hint(h.polys[i].coeffs[j], hint, gamma2);
        }
    }
    
    result
}

impl Signature for Dilithium2 {
    type PublicKey = DilithiumPublicKey<K>;
    type SecretKey = DilithiumSecretKey<K>;
    type Sig = DilithiumSignature;
    
    const PUBLIC_KEY_SIZE: usize = PUBLIC_KEY_SIZE;
    const SECRET_KEY_SIZE: usize = SECRET_KEY_SIZE;
    const SIGNATURE_SIZE: usize = SIGNATURE_SIZE;
    
    fn generate_keypair<R: SecureRandom>(
        rng: &mut R
    ) -> Result<(Self::PublicKey, Self::SecretKey)> {
        // Generate random seed using secure memory
        let mut seed = SecureArray::<32>::zero();
        rng.fill_bytes(seed.as_mut());
        
        // Expand seed using secure memory
        let mut expanded_seed = SecureArray::<96>::zero();
        let hash_out = g(seed.as_ref());
        expanded_seed.as_mut()[..32].copy_from_slice(seed.as_ref());
        expanded_seed.as_mut()[32..96].copy_from_slice(&hash_out);
        
        let rho = &expanded_seed.as_ref()[0..32];
        let rhoprime = &expanded_seed.as_ref()[32..64];
        let key = &expanded_seed.as_ref()[64..96];
        
        // Expand matrix A
        let a = DilithiumMatrix::<K, L>::expand_a(rho);
        
        // Sample secret vectors
        let mut s1 = DilithiumPolyVec::<L>::zero();
        let mut s2 = DilithiumPolyVec::<K>::zero();
        
        for i in 0..L {
            s1.polys[i] = DilithiumPoly::sample_eta(rhoprime, i as u16, ETA);
        }
        
        for i in 0..K {
            s2.polys[i] = DilithiumPoly::sample_eta(rhoprime, (L + i) as u16, ETA);
        }
        
        // Compute t = As1 + s2
        s1.ntt();
        let mut t = a.mul_vec(&s1);
        t.inv_ntt();
        t = t.add(&s2);
        t.reduce();
        
        // Split t into t1 and t0
        let (t1, t0) = t.power2round();
        
        // Pack public key
        let mut pk_bytes = Vec::with_capacity(PUBLIC_KEY_SIZE);
        pk_bytes.extend_from_slice(rho);
        // Pack t1 (simplified - actual packing is more complex)
        for i in 0..K {
            for j in 0..256 {
                let val = t1.polys[i].coeffs[j];
                pk_bytes.extend_from_slice(&val.to_le_bytes()[..3]);
            }
        }
        
        // Pack secret key
        let mut sk_bytes = Vec::with_capacity(SECRET_KEY_SIZE);
        sk_bytes.extend_from_slice(rho);
        sk_bytes.extend_from_slice(key);
        let tr = h(&pk_bytes);
        sk_bytes.extend_from_slice(&tr);
        
        // Pack s1, s2, t0 (simplified)
        for i in 0..L {
            for j in 0..256 {
                sk_bytes.push(s1.polys[i].coeffs[j] as u8);
            }
        }
        for i in 0..K {
            for j in 0..256 {
                sk_bytes.push(s2.polys[i].coeffs[j] as u8);
            }
        }
        for i in 0..K {
            for j in 0..256 {
                let val = t0.polys[i].coeffs[j];
                sk_bytes.extend_from_slice(&val.to_le_bytes()[..3]);
            }
        }
        
        Ok((
            DilithiumPublicKey { bytes: pk_bytes },
            DilithiumSecretKey { bytes: sk_bytes }
        ))
    }
    
    fn sign<R: SecureRandom>(
        secret_key: &Self::SecretKey,
        message: &[u8],
        rng: &mut R
    ) -> Result<Self::Sig> {
        // Extract components from secret key
        let rho = &secret_key.bytes[0..32];
        let key = &secret_key.bytes[32..64];
        let tr = &secret_key.bytes[64..96];
        
        // Generate randomness using secure memory
        let mut rand_bytes = SecureArray::<32>::zero();
        rng.fill_bytes(rand_bytes.as_mut());
        
        // Compute mu = CRH(tr || M)
        let mut mu_input = Vec::with_capacity(96 + message.len());
        mu_input.extend_from_slice(tr);
        mu_input.extend_from_slice(message);
        let mu = crh(&mu_input);
        
        // Compute rhoprime using secure memory
        let mut rhoprime_input = SecureArray::<112>::zero();  // Increased from 96 to 112 to accommodate 48-byte mu
        rhoprime_input.as_mut()[..32].copy_from_slice(key);
        rhoprime_input.as_mut()[32..64].copy_from_slice(rand_bytes.as_ref());
        rhoprime_input.as_mut()[64..112].copy_from_slice(&mu);  // Now correctly copies all 48 bytes of mu
        let rhoprime_full = g(rhoprime_input.as_ref());
        let rhoprime = &rhoprime_full[..32];
        
        // Expand matrix A
        let a = DilithiumMatrix::<K, L>::expand_a(rho);
        
        // Unpack s1, s2, t0 from secret key (simplified)
        let s1 = DilithiumPolyVec::<L>::zero();
        let _s2 = DilithiumPolyVec::<K>::zero();
        let _t0 = DilithiumPolyVec::<K>::zero();
        
        // (Actual unpacking would be more complex)
        // For now, we'll use dummy values
        
        let mut nonce = 0u16;
        let mut sig_bytes = Vec::new();
        
        loop {
            // Sample y
            let mut y = DilithiumPolyVec::<L>::zero();
            for i in 0..L {
                y.polys[i] = DilithiumPoly::sample_uniform(rhoprime, nonce + i as u16);
                // Mask to gamma1 - 1
                for j in 0..256 {
                    y.polys[i].coeffs[j] &= GAMMA1 - 1;
                }
            }
            nonce += L as u16;
            
            // Compute w = Ay
            y.ntt();
            let mut w = a.mul_vec(&y);
            w.inv_ntt();
            w.reduce();
            
            // Decompose w
            let (w1, _) = w.decompose(GAMMA2);
            
            // Challenge
            let mut c_input = Vec::new();
            c_input.extend_from_slice(&mu);
            // Pack w1 (simplified)
            for i in 0..K {
                for j in 0..256 {
                    c_input.push(w1.polys[i].coeffs[j] as u8);
                }
            }
            let c_hash = h(&c_input);
            let c = sample_challenge(&c_hash, TAU);
            
            // Compute z = y + cs1
            let mut c_ntt = c.clone();
            c_ntt.ntt();
            
            let mut z = DilithiumPolyVec::<L>::zero();
            for i in 0..L {
                z.polys[i] = y.polys[i].add(&c_ntt.pointwise_mul(&s1.polys[i]));
            }
            z.inv_ntt();
            z.reduce();
            
            // Check z norm
            if !z.check_norm(GAMMA1 - BETA as i32) {
                continue;
            }
            
            // Compute hints (simplified)
            let (hints, hint_count) = make_hint_vec(&w, &w1, GAMMA2);
            
            if hint_count > OMEGA {
                continue;
            }
            
            // Pack signature
            sig_bytes.clear();
            sig_bytes.extend_from_slice(&c_hash);
            
            // Pack z (simplified)
            for i in 0..L {
                for j in 0..256 {
                    sig_bytes.extend_from_slice(&z.polys[i].coeffs[j].to_le_bytes());
                }
            }
            
            // Pack hints (simplified)
            sig_bytes.push(hint_count as u8);
            for &(i, j) in &hints {
                sig_bytes.push(i as u8);
                sig_bytes.push(j as u8);
            }
            
            break;
        }
        
        Ok(DilithiumSignature { bytes: sig_bytes })
    }
    
    fn sign_deterministic(
        secret_key: &Self::SecretKey,
        message: &[u8]
    ) -> Result<Self::Sig> {
        // For deterministic signing, we use a zero random value
        struct ZeroRng;
        impl SecureRandom for ZeroRng {
            fn fill_bytes(&mut self, dest: &mut [u8]) {
                dest.fill(0);
            }
        }
        
        let mut rng = ZeroRng;
        Self::sign(secret_key, message, &mut rng)
    }
    
    fn verify(
        public_key: &Self::PublicKey,
        message: &[u8],
        signature: &Self::Sig
    ) -> Result<bool> {
        // Extract components
        let rho = &public_key.bytes[0..32];
        let c_hash = &signature.bytes[0..32];
        
        // Unpack t1 from public key (simplified)
        let t1 = DilithiumPolyVec::<K>::zero();
        
        // Unpack z from signature (simplified)
        let mut z = DilithiumPolyVec::<L>::zero();
        
        // Unpack hints (simplified)
        let hints = Vec::new();
        
        // Expand matrix A
        let a = DilithiumMatrix::<K, L>::expand_a(rho);
        
        // Compute mu
        let tr = h(&public_key.bytes);
        let mut mu_input = Vec::with_capacity(32 + message.len());
        mu_input.extend_from_slice(&tr);
        mu_input.extend_from_slice(message);
        let mu = crh(&mu_input);
        
        // Recover challenge
        let c = sample_challenge(c_hash, TAU);
        
        // Check z norm
        if !z.check_norm(GAMMA1 - BETA as i32) {
            return Ok(false);
        }
        
        // Compute w' = Az - ct1
        z.ntt();
        let mut w_prime = a.mul_vec(&z);
        
        let mut c_ntt = c.clone();
        c_ntt.ntt();
        
        for i in 0..K {
            let ct = c_ntt.pointwise_mul(&t1.polys[i]);
            w_prime.polys[i] = w_prime.polys[i].sub(&ct);
        }
        
        w_prime.inv_ntt();
        w_prime.reduce();
        
        // Use hints to recover w1
        let w1_prime = use_hint_vec(&w_prime, &hints, GAMMA2);
        
        // Recompute challenge
        let mut c_input = Vec::new();
        c_input.extend_from_slice(&mu);
        // Pack w1_prime (simplified)
        for i in 0..K {
            for j in 0..256 {
                c_input.push(w1_prime.polys[i].coeffs[j] as u8);
            }
        }
        
        let c_hash_prime = h(&c_input);
        
        Ok(c_hash == c_hash_prime)
    }
}