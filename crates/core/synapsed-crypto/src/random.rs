//! Random number generation utilities
//!
//! This module provides secure random number generation
//! with support for both std and no-std environments.

use crate::traits::SecureRandom;
use rand_core::{RngCore, CryptoRng};
use core::fmt;

/// Default RNG implementation
#[cfg(feature = "std")]
pub struct DefaultRng {
    inner: rand_core::OsRng,
}

#[cfg(feature = "std")]
impl Default for DefaultRng {
    fn default() -> Self {
        Self {
            inner: rand_core::OsRng,
        }
    }
}

#[cfg(feature = "std")]
impl SecureRandom for DefaultRng {
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.inner.fill_bytes(dest);
    }
}

#[cfg(feature = "std")]
impl fmt::Debug for DefaultRng {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultRng")
            .field("inner", &"OsRng")
            .finish()
    }
}

/// Wrapper for any RngCore + CryptoRng
pub struct RngWrapper<R: RngCore + CryptoRng> {
    rng: R,
}

impl<R: RngCore + CryptoRng> RngWrapper<R> {
    /// Create a new RNG wrapper
    pub fn new(rng: R) -> Self {
        Self { rng }
    }
}

impl<R: RngCore + CryptoRng> SecureRandom for RngWrapper<R> {
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.rng.fill_bytes(dest);
    }
}

impl<R: RngCore + CryptoRng> fmt::Debug for RngWrapper<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RngWrapper")
            .field("rng", &"<RNG>")
            .finish()
    }
}

/// Get system random bytes (requires std feature)
/// 
/// # Errors
/// 
/// Returns an error if the system random number generator fails
#[cfg(feature = "std")]
pub fn system_random_bytes(dest: &mut [u8]) -> Result<(), getrandom::Error> {
    use getrandom::getrandom;
    getrandom(dest)?;
    Ok(())
}

/// Deterministic RNG for testing
/// Test RNG for deterministic testing
#[derive(Debug, Clone)]
pub struct TestRng {
    seed: u64,
}

impl TestRng {
    /// Create a new test RNG with the given seed
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }
}

impl SecureRandom for TestRng {
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        // Simple LCG for deterministic testing
        for byte in dest {
            self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *byte = (self.seed >> 24) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_test_rng() {
        let mut rng1 = TestRng::new(12345);
        let mut rng2 = TestRng::new(12345);
        
        let mut buf1 = [0u8; 32];
        let mut buf2 = [0u8; 32];
        
        rng1.fill_bytes(&mut buf1);
        rng2.fill_bytes(&mut buf2);
        
        assert_eq!(buf1, buf2);
    }
    
    #[cfg(feature = "std")]
    #[test]
    fn test_default_rng() {
        let mut rng = DefaultRng::default();
        let mut buf = [0u8; 32];
        
        rng.fill_bytes(&mut buf);
        
        // Check that we got non-zero output
        assert!(buf.iter().any(|&b| b != 0));
    }
}