//! Algorithm parameters for Kyber and Dilithium
//!
//! This module contains the parameter sets for the NIST-standardized
//! post-quantum algorithms ML-KEM (Kyber) and ML-DSA (Dilithium).

/// Kyber parameter sets
pub mod kyber {
    /// Common Kyber parameters
    /// Polynomial degree (number of coefficients)
    pub const N: usize = 256;  // Polynomial degree
    
    /// Modulus for the polynomial ring
    pub const Q: i16 = 3329;   // Modulus
    
    /// Size of hashes and seeds in bytes
    pub const SYMBYTES: usize = 32;  // Size of hashes/seeds
    
    /// Kyber512 parameters (NIST Level 1)
    pub mod kyber512 {
        use super::*;
        
        /// Module dimension (number of polynomials in vector)
        pub const K: usize = 2;  // Module dimension
        
        /// Noise distribution parameter for key generation
        pub const ETA1: usize = 3;
        
        /// Noise distribution parameter for encryption
        pub const ETA2: usize = 2;
        
        /// Ciphertext compression parameter for u
        pub const DU: usize = 10;  // Ciphertext compression
        
        /// Ciphertext compression parameter for v
        pub const DV: usize = 4;   // Ciphertext compression
        
        // Derived sizes
        
        /// Size of polynomial vector in bytes
        pub const POLYVECBYTES: usize = K * 384;
        
        /// Size of compressed polynomial in bytes
        pub const POLYCOMPRESSEDBYTES: usize = 128;
        
        /// Size of compressed polynomial vector in bytes
        pub const POLYVECCOMPRESSEDBYTES: usize = K * 320;
        
        /// Public key size in bytes
        pub const PUBLIC_KEY_SIZE: usize = POLYVECBYTES + SYMBYTES;
        
        /// Secret key size in bytes
        pub const SECRET_KEY_SIZE: usize = POLYVECBYTES + PUBLIC_KEY_SIZE + 32 + 32;
        
        /// Ciphertext size in bytes
        pub const CIPHERTEXT_SIZE: usize = POLYVECCOMPRESSEDBYTES + POLYCOMPRESSEDBYTES;
        
        /// Shared secret size in bytes
        pub const SHARED_SECRET_SIZE: usize = 32;
    }
    
    /// Kyber768 parameters (NIST Level 3)
    pub mod kyber768 {
        use super::*;
        
        /// Module dimension (number of polynomials in the public key vector)
        pub const K: usize = 3;
        /// Noise distribution parameter for key generation (standard deviation bound)
        pub const ETA1: usize = 2;
        /// Noise distribution parameter for encryption (standard deviation bound)
        pub const ETA2: usize = 2;
        /// Ciphertext compression parameter for u component (bits per coefficient)
        pub const DU: usize = 10;
        /// Ciphertext compression parameter for v component (bits per coefficient)
        pub const DV: usize = 4;
        
        // Derived sizes
        /// Size of polynomial vector in bytes
        pub const POLYVECBYTES: usize = K * 384;
        /// Size of compressed polynomial in bytes
        pub const POLYCOMPRESSEDBYTES: usize = 128;
        /// Size of compressed polynomial vector in bytes
        pub const POLYVECCOMPRESSEDBYTES: usize = K * 320;
        
        /// Public key size in bytes
        pub const PUBLIC_KEY_SIZE: usize = POLYVECBYTES + SYMBYTES;
        /// Secret key size in bytes
        pub const SECRET_KEY_SIZE: usize = POLYVECBYTES + PUBLIC_KEY_SIZE + 32 + 32;
        /// Ciphertext size in bytes
        pub const CIPHERTEXT_SIZE: usize = POLYVECCOMPRESSEDBYTES + POLYCOMPRESSEDBYTES;
        /// Shared secret size in bytes
        pub const SHARED_SECRET_SIZE: usize = 32;
    }
    
    /// Kyber1024 parameters (NIST Level 5)
    pub mod kyber1024 {
        use super::*;
        
        /// Module dimension (number of polynomials in the public key vector)
        pub const K: usize = 4;
        /// Noise distribution parameter for key generation (standard deviation bound)
        pub const ETA1: usize = 2;
        /// Noise distribution parameter for encryption (standard deviation bound)
        pub const ETA2: usize = 2;
        /// Ciphertext compression parameter for u component (bits per coefficient)
        pub const DU: usize = 11;
        /// Ciphertext compression parameter for v component (bits per coefficient)
        pub const DV: usize = 5;
        
        // Derived sizes
        /// Size of polynomial vector in bytes
        pub const POLYVECBYTES: usize = K * 384;
        /// Size of compressed polynomial in bytes
        pub const POLYCOMPRESSEDBYTES: usize = 160;
        /// Size of compressed polynomial vector in bytes
        pub const POLYVECCOMPRESSEDBYTES: usize = K * 352;
        
        /// Public key size in bytes
        pub const PUBLIC_KEY_SIZE: usize = POLYVECBYTES + SYMBYTES;
        /// Secret key size in bytes
        pub const SECRET_KEY_SIZE: usize = POLYVECBYTES + PUBLIC_KEY_SIZE + 32 + 32;
        /// Ciphertext size in bytes
        pub const CIPHERTEXT_SIZE: usize = POLYVECCOMPRESSEDBYTES + POLYCOMPRESSEDBYTES;
        /// Shared secret size in bytes
        pub const SHARED_SECRET_SIZE: usize = 32;
    }
}

/// Dilithium parameter sets
pub mod dilithium {
    /// Common Dilithium parameters
    pub const N: usize = 256;  // Polynomial degree
    /// Prime modulus q = 2^23 - 2^13 + 1 = 8380417
    pub const Q: i32 = 8380417;
    /// Number of bits dropped from t when computing w1 and w0
    pub const D: usize = 13;
    /// 256th root of unity modulo q
    pub const ROOT_OF_UNITY: i32 = 1753;
    /// Size of hashes and seeds in bytes
    pub const SYMBYTES: usize = 32;
    /// Size of collision-resistant hash output in bytes
    pub const CRHBYTES: usize = 64;
    /// Size of commitment hash output in bytes
    pub const TRBYTES: usize = 64;
    
    /// Dilithium2 parameters (NIST Level 2)
    pub mod dilithium2 {
        use super::*;
        
        /// Dimension of matrix A (number of columns)
        pub const K: usize = 4;
        /// Dimension of matrix A (number of rows)
        pub const L: usize = 4;
        /// Noise distribution parameter (standard deviation bound)
        pub const ETA: usize = 2;
        /// Number of ±1's in the challenge polynomial c
        pub const TAU: usize = 39;
        /// Maximum L∞ norm of c*s1 and c*s2
        pub const BETA: usize = 78;
        /// Low-order rounding parameter γ1 = 2^17
        pub const GAMMA1: i32 = 131072;
        /// High-order rounding parameter γ2 = (q-1)/88
        pub const GAMMA2: i32 = 95232;
        /// Maximum number of 1's in the hint vector h
        pub const OMEGA: usize = 80;
        
        // Derived sizes
        /// Size of packed t1 polynomial in bytes
        pub const POLYT1_PACKEDBYTES: usize = 320;
        /// Size of packed t0 polynomial in bytes
        pub const POLYT0_PACKEDBYTES: usize = 416;
        /// Size of packed hint vector h in bytes
        pub const POLYVECH_PACKEDBYTES: usize = OMEGA + K;
        /// Size of packed z polynomial in bytes
        pub const POLYZ_PACKEDBYTES: usize = 576;
        /// Size of packed w1 polynomial in bytes
        pub const POLYW1_PACKEDBYTES: usize = 192;
        /// Size of packed eta polynomial in bytes
        pub const POLYETA_PACKEDBYTES: usize = 96;
        
        /// Public key size in bytes
        pub const PUBLIC_KEY_SIZE: usize = SYMBYTES + K * POLYT1_PACKEDBYTES;
        /// Secret key size in bytes
        pub const SECRET_KEY_SIZE: usize = 2 * SYMBYTES + TRBYTES 
            + L * POLYETA_PACKEDBYTES 
            + K * POLYETA_PACKEDBYTES 
            + K * POLYT0_PACKEDBYTES;
        /// Signature size in bytes
        pub const SIGNATURE_SIZE: usize = SYMBYTES + L * POLYZ_PACKEDBYTES + POLYVECH_PACKEDBYTES;
    }
    
    /// Dilithium3 parameters (NIST Level 3)
    pub mod dilithium3 {
        use super::*;
        
        /// Dimension of matrix A (number of columns)
        pub const K: usize = 6;
        /// Dimension of matrix A (number of rows)
        pub const L: usize = 5;
        /// Noise distribution parameter (standard deviation bound)
        pub const ETA: usize = 4;
        /// Number of ±1's in the challenge polynomial c
        pub const TAU: usize = 49;
        /// Maximum L∞ norm of c*s1 and c*s2
        pub const BETA: usize = 196;
        /// Low-order rounding parameter γ1 = 2^19
        pub const GAMMA1: i32 = 524288;
        /// High-order rounding parameter γ2 = (q-1)/32
        pub const GAMMA2: i32 = 261888;
        /// Maximum number of 1's in the hint vector h
        pub const OMEGA: usize = 55;
        
        // Derived sizes
        /// Size of packed t1 polynomial in bytes
        pub const POLYT1_PACKEDBYTES: usize = 320;
        /// Size of packed t0 polynomial in bytes
        pub const POLYT0_PACKEDBYTES: usize = 416;
        /// Size of packed hint vector h in bytes
        pub const POLYVECH_PACKEDBYTES: usize = OMEGA + K;
        /// Size of packed z polynomial in bytes
        pub const POLYZ_PACKEDBYTES: usize = 640;
        /// Size of packed w1 polynomial in bytes
        pub const POLYW1_PACKEDBYTES: usize = 128;
        /// Size of packed eta polynomial in bytes
        pub const POLYETA_PACKEDBYTES: usize = 128;
        
        /// Public key size in bytes
        pub const PUBLIC_KEY_SIZE: usize = SYMBYTES + K * POLYT1_PACKEDBYTES;
        /// Secret key size in bytes
        pub const SECRET_KEY_SIZE: usize = 2 * SYMBYTES + TRBYTES 
            + L * POLYETA_PACKEDBYTES 
            + K * POLYETA_PACKEDBYTES 
            + K * POLYT0_PACKEDBYTES;
        /// Signature size in bytes
        pub const SIGNATURE_SIZE: usize = SYMBYTES + L * POLYZ_PACKEDBYTES + POLYVECH_PACKEDBYTES;
    }
    
    /// Dilithium5 parameters (NIST Level 5)
    pub mod dilithium5 {
        use super::*;
        
        /// Dimension of matrix A (number of columns)
        pub const K: usize = 8;
        /// Dimension of matrix A (number of rows)
        pub const L: usize = 7;
        /// Noise distribution parameter (standard deviation bound)
        pub const ETA: usize = 2;
        /// Number of ±1's in the challenge polynomial c
        pub const TAU: usize = 60;
        /// Maximum L∞ norm of c*s1 and c*s2
        pub const BETA: usize = 120;
        /// Low-order rounding parameter γ1 = 2^19
        pub const GAMMA1: i32 = 524288;
        /// High-order rounding parameter γ2 = (q-1)/32
        pub const GAMMA2: i32 = 261888;
        /// Maximum number of 1's in the hint vector h
        pub const OMEGA: usize = 75;
        
        // Derived sizes
        /// Size of packed t1 polynomial in bytes
        pub const POLYT1_PACKEDBYTES: usize = 320;
        /// Size of packed t0 polynomial in bytes
        pub const POLYT0_PACKEDBYTES: usize = 416;
        /// Size of packed hint vector h in bytes
        pub const POLYVECH_PACKEDBYTES: usize = OMEGA + K;
        /// Size of packed z polynomial in bytes
        pub const POLYZ_PACKEDBYTES: usize = 640;
        /// Size of packed w1 polynomial in bytes
        pub const POLYW1_PACKEDBYTES: usize = 128;
        /// Size of packed eta polynomial in bytes
        pub const POLYETA_PACKEDBYTES: usize = 96;
        
        /// Public key size in bytes
        pub const PUBLIC_KEY_SIZE: usize = SYMBYTES + K * POLYT1_PACKEDBYTES;
        /// Secret key size in bytes
        pub const SECRET_KEY_SIZE: usize = 2 * SYMBYTES + TRBYTES 
            + L * POLYETA_PACKEDBYTES 
            + K * POLYETA_PACKEDBYTES 
            + K * POLYT0_PACKEDBYTES;
        /// Signature size in bytes
        pub const SIGNATURE_SIZE: usize = SYMBYTES + L * POLYZ_PACKEDBYTES + POLYVECH_PACKEDBYTES;
    }
}