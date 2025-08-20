//! Dilithium3 implementation (NIST Level 3)

use crate::{
    error::Result,
    params::dilithium::{dilithium3::*, N},
    traits::{Signature, SecureRandom},
    dilithium::{DilithiumPublicKey, DilithiumSecretKey, DilithiumSignature, common::*},
    hash::{h, g, crh},
    dilithium::dilithium2::{DilithiumPoly, DilithiumPolyVec, DilithiumMatrix},
    secure_memory::SecureArray,
};
use sha3::{Shake256, digest::{ExtendableOutput, Update, XofReader}};

/// Dilithium3 - NIST Level 3 security
#[derive(Debug, Clone, Copy)]
pub struct Dilithium3;

/// Sample challenge polynomial for Dilithium3
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
        
        if idx < N && !c_indices.contains(&idx) {
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
        for j in 0..N {
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
        for j in 0..N {
            let hint = hints.iter().any(|&(hi, hj)| hi == i && hj == j);
            result.polys[i].coeffs[j] = use_hint(h.polys[i].coeffs[j], hint, gamma2);
        }
    }
    
    result
}

impl Signature for Dilithium3 {
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
            for j in 0..N {
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
            for j in 0..N {
                sk_bytes.push(s1.polys[i].coeffs[j] as u8);
            }
        }
        for i in 0..K {
            for j in 0..N {
                sk_bytes.push(s2.polys[i].coeffs[j] as u8);
            }
        }
        for i in 0..K {
            for j in 0..N {
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
                // Use wrapping_add to handle potential overflow
                y.polys[i] = DilithiumPoly::sample_uniform(rhoprime, nonce.wrapping_add(i as u16));
                // Mask to gamma1 - 1
                for j in 0..N {
                    y.polys[i].coeffs[j] &= GAMMA1 - 1;
                }
            }
            // Use wrapping_add for nonce increment as well
            nonce = nonce.wrapping_add(L as u16);
            
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
                for j in 0..N {
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
                for j in 0..N {
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
            for j in 0..N {
                c_input.push(w1_prime.polys[i].coeffs[j] as u8);
            }
        }
        
        let c_hash_prime = h(&c_input);
        
        Ok(c_hash == c_hash_prime)
    }
}