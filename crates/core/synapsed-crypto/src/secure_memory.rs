//! Secure memory handling for cryptographic operations
//!
//! This module provides utilities for secure handling of sensitive data,
//! ensuring proper zeroing of memory when data is no longer needed.

use zeroize::{Zeroize, ZeroizeOnDrop};
use core::fmt;

/// Wrapper for sensitive byte arrays that automatically zeros memory on drop
#[derive(Clone)]
pub struct SecureBytes {
    data: Vec<u8>,
}

impl SecureBytes {
    /// Create new secure bytes from a vector
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
    
    /// Create new secure bytes with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }
    
    /// Get the length of the data
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Check if data is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    /// Extend from slice
    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        self.data.extend_from_slice(slice);
    }
    
    /// Clear the data (zeros it)
    pub fn clear(&mut self) {
        self.data.zeroize();
        self.data.clear();
    }
}

impl Zeroize for SecureBytes {
    fn zeroize(&mut self) {
        self.data.zeroize();
    }
}

impl Drop for SecureBytes {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for SecureBytes {}

impl fmt::Debug for SecureBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecureBytes")
            .field("len", &self.data.len())
            .field("data", &"[REDACTED]")
            .finish()
    }
}

impl AsRef<[u8]> for SecureBytes {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl AsMut<[u8]> for SecureBytes {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

/// Wrapper for fixed-size sensitive arrays
#[derive(Clone)]
pub struct SecureArray<const N: usize> {
    data: [u8; N],
}

impl<const N: usize> SecureArray<N> {
    /// Create new secure array
    pub fn new(data: [u8; N]) -> Self {
        Self { data }
    }
    
    /// Create zeroed secure array
    pub fn zero() -> Self {
        Self { data: [0u8; N] }
    }
    
    /// Copy from slice
    pub fn copy_from_slice(&mut self, slice: &[u8]) {
        self.data.copy_from_slice(slice);
    }
}

impl<const N: usize> Zeroize for SecureArray<N> {
    fn zeroize(&mut self) {
        self.data.zeroize();
    }
}

impl<const N: usize> Drop for SecureArray<N> {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl<const N: usize> ZeroizeOnDrop for SecureArray<N> {}

impl<const N: usize> fmt::Debug for SecureArray<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecureArray")
            .field("size", &N)
            .field("data", &"[REDACTED]")
            .finish()
    }
}

impl<const N: usize> AsRef<[u8]> for SecureArray<N> {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl<const N: usize> AsMut<[u8]> for SecureArray<N> {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

/// Secure temporary buffer that zeros on drop
pub struct SecureBuffer<const N: usize> {
    buffer: [u8; N],
}

impl<const N: usize> SecureBuffer<N> {
    /// Create new secure buffer
    pub fn new() -> Self {
        Self { buffer: [0u8; N] }
    }
    
    /// Get buffer as slice
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }
    
    /// Get mutable buffer
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.buffer
    }
}

impl<const N: usize> Zeroize for SecureBuffer<N> {
    fn zeroize(&mut self) {
        self.buffer.zeroize();
    }
}

impl<const N: usize> Drop for SecureBuffer<N> {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl<const N: usize> ZeroizeOnDrop for SecureBuffer<N> {}

impl<const N: usize> Default for SecureBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> fmt::Debug for SecureBuffer<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecureBuffer")
            .field("size", &N)
            .field("buffer", &"[REDACTED]")
            .finish()
    }
}

/// Secure scope for temporary sensitive data
/// 
/// Usage:
/// ```
/// secure_scope(|buffer: &mut [u8; 32]| {
///     // Use buffer for sensitive operations
///     // Buffer will be automatically zeroed when scope ends
/// });
/// ```
pub fn secure_scope<const N: usize, F, R>(f: F) -> R
where
    F: FnOnce(&mut [u8; N]) -> R,
{
    let mut buffer = [0u8; N];
    let result = f(&mut buffer);
    buffer.zeroize();
    result
}

/// Execute a closure with a dynamically sized secure buffer
pub fn secure_vec_scope<F, R>(size: usize, f: F) -> R
where
    F: FnOnce(&mut Vec<u8>) -> R,
{
    let mut buffer = vec![0u8; size];
    let result = f(&mut buffer);
    buffer.zeroize();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_secure_bytes() {
        let mut secure = SecureBytes::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(secure.len(), 5);
        assert_eq!(secure.as_ref(), &[1, 2, 3, 4, 5]);
        
        secure.clear();
        assert!(secure.is_empty());
    }
    
    #[test]
    fn test_secure_array() {
        let mut secure = SecureArray::<32>::zero();
        secure.copy_from_slice(&[42u8; 32]);
        assert_eq!(secure.as_ref(), &[42u8; 32]);
    }
    
    #[test]
    fn test_secure_scope() {
        let result = secure_scope(|buffer: &mut [u8; 16]| {
            buffer[0] = 42;
            buffer[0]
        });
        assert_eq!(result, 42);
    }
}