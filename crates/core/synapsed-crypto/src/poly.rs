//! Polynomial arithmetic operations
//!
//! This module implements polynomial operations over various rings
//! used in post-quantum cryptography algorithms.

use crate::utils::barrett_reduce;
use crate::error::{Error, Result};
use core::ops::{Add, Sub, Neg};
use zeroize::Zeroize;

/// Polynomial with coefficients in Z_q
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Poly<const N: usize> {
    /// Coefficients of the polynomial
    pub coeffs: [i16; N],
}

impl<const N: usize> Poly<N> {
    /// Create a new zero polynomial
    pub fn zero() -> Self {
        Self { coeffs: [0; N] }
    }
    
    /// Create a polynomial from coefficients
    pub fn from_coeffs(coeffs: [i16; N]) -> Self {
        Self { coeffs }
    }
    
    /// Reduce all coefficients modulo q
    pub fn reduce(&mut self) {
        for coeff in &mut self.coeffs {
            *coeff = barrett_reduce(*coeff);
        }
    }
    
    /// Add q if coefficient is negative (conditional add q)
    pub fn caddq(&mut self) {
        for coeff in &mut self.coeffs {
            *coeff = crate::constant_time::ct_caddq(*coeff);
        }
    }
    
    /// Convert to NTT domain
    pub fn ntt(&mut self) {
        // Placeholder - actual NTT implementation in ntt.rs
        crate::ntt::ntt(&mut self.coeffs);
    }
    
    /// Convert from NTT domain
    pub fn inv_ntt(&mut self) {
        // Placeholder - actual inverse NTT implementation in ntt.rs
        crate::ntt::inv_ntt(&mut self.coeffs);
    }
    
    /// Pointwise multiplication in NTT domain
    pub fn basemul(&self, other: &Self) -> Self {
        let mut result = Self::zero();
        crate::ntt::basemul(&mut result.coeffs, &self.coeffs, &other.coeffs);
        result
    }
    
    
    /// Sample polynomial from centered binomial distribution
    pub fn cbd(bytes: &[u8], eta: usize) -> Result<Self> {
        let mut poly = Self::zero();
        
        match eta {
            2 => cbd2(&mut poly.coeffs, bytes),
            3 => cbd3(&mut poly.coeffs, bytes),
            _ => return Err(Error::InvalidParameter),
        }
        
        Ok(poly)
    }
    
    /// Unpack polynomial from bytes (12-bit encoding)
    pub fn unpack(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 384 {
            return Err(Error::InvalidSize);
        }
        
        let mut poly = Self::zero();
        crate::utils::decode_poly(bytes, &mut poly.coeffs);
        Ok(poly)
    }
    
    /// Pack polynomial to bytes (12-bit encoding)
    pub fn pack(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; 384];
        crate::utils::encode_poly(&self.coeffs, &mut bytes);
        bytes
    }
}

/// Centered binomial distribution with eta=2
fn cbd2(coeffs: &mut [i16], bytes: &[u8]) {
    let num_blocks = coeffs.len() / 8;
    for i in 0..num_blocks {
        if 4*i + 3 >= bytes.len() {
            break;
        }
        let t = u32::from_le_bytes([
            bytes[4*i],
            bytes[4*i + 1],
            bytes[4*i + 2],
            bytes[4*i + 3],
        ]);
        
        let d = (0..8).map(|j| {
            let a = (t >> j) & 0x00010001;
            let b = (t >> (j + 1)) & 0x00010001;
            (a.wrapping_sub(b)) as i16
        });
        
        for (j, d_j) in d.enumerate() {
            if 8*i + j < coeffs.len() {
                coeffs[8*i + j] = d_j;
            }
        }
    }
}

/// Centered binomial distribution with eta=3
fn cbd3(coeffs: &mut [i16], bytes: &[u8]) {
    let num_blocks = coeffs.len() / 4;
    for i in 0..num_blocks {
        if 3*i + 2 >= bytes.len() {
            break;
        }
        let t = u32::from_le_bytes([
            bytes[3*i],
            bytes[3*i + 1],
            bytes[3*i + 2],
            0,
        ]) & 0x00FFFFFF;
        
        let d = (0..4).map(|j| {
            let a = (t >> j) & 0x00249249;
            let b = (t >> (j + 1)) & 0x00249249;
            let c = (t >> (j + 2)) & 0x00249249;
            
            let sum_a = a + (a >> 9) + (a >> 18);
            let sum_bc = b + c + ((b + c) >> 9) + ((b + c) >> 18);
            
            ((sum_a & 7) as i16) - ((sum_bc & 7) as i16)
        });
        
        for (j, d_j) in d.enumerate() {
            if 4*i + j < coeffs.len() {
                coeffs[4*i + j] = d_j;
            }
        }
    }
}

impl<const N: usize> Add for Poly<N> {
    type Output = Self;
    
    fn add(mut self, other: Self) -> Self {
        for i in 0..N {
            self.coeffs[i] = self.coeffs[i].wrapping_add(other.coeffs[i]);
        }
        self
    }
}

impl<const N: usize> Sub for Poly<N> {
    type Output = Self;
    
    fn sub(mut self, other: Self) -> Self {
        for i in 0..N {
            self.coeffs[i] = self.coeffs[i].wrapping_sub(other.coeffs[i]);
        }
        self
    }
}

impl<const N: usize> Neg for Poly<N> {
    type Output = Self;
    
    fn neg(mut self) -> Self {
        for coeff in &mut self.coeffs {
            *coeff = -*coeff;
        }
        self
    }
}

impl<const N: usize> Zeroize for Poly<N> {
    fn zeroize(&mut self) {
        self.coeffs.zeroize();
    }
}

/// Vector of polynomials
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolyVec<const N: usize, const K: usize> {
    /// Vector of polynomials
    pub polys: [Poly<N>; K],
}

impl<const N: usize, const K: usize> PolyVec<N, K> {
    /// Create a new zero polynomial vector
    pub fn zero() -> Self {
        Self {
            polys: core::array::from_fn(|_| Poly::zero()),
        }
    }
    
    /// Reduce all polynomials
    pub fn reduce(&mut self) {
        for poly in &mut self.polys {
            poly.reduce();
        }
    }
    
    /// Convert all polynomials to NTT domain
    pub fn ntt(&mut self) {
        for poly in &mut self.polys {
            poly.ntt();
        }
    }
    
    /// Convert all polynomials from NTT domain
    pub fn inv_ntt(&mut self) {
        for poly in &mut self.polys {
            poly.inv_ntt();
        }
    }
    
    /// Add q if coefficient is negative for all polynomials
    pub fn caddq(&mut self) {
        for poly in &mut self.polys {
            poly.caddq();
        }
    }
    
    /// Inner product of two polynomial vectors
    pub fn inner_product(&self, other: &Self) -> Poly<N> {
        let mut result = Poly::zero();
        
        for i in 0..K {
            let prod = self.polys[i].basemul(&other.polys[i]);
            result = result + prod;
        }
        
        result
    }
}

impl<const N: usize, const K: usize> Add for PolyVec<N, K> {
    type Output = Self;
    
    fn add(mut self, other: Self) -> Self {
        for i in 0..K {
            self.polys[i] = self.polys[i].clone() + other.polys[i].clone();
        }
        self
    }
}

impl<const N: usize, const K: usize> Sub for PolyVec<N, K> {
    type Output = Self;
    
    fn sub(mut self, other: Self) -> Self {
        for i in 0..K {
            self.polys[i] = self.polys[i].clone() - other.polys[i].clone();
        }
        self
    }
}

impl<const N: usize, const K: usize> Zeroize for PolyVec<N, K> {
    fn zeroize(&mut self) {
        for poly in &mut self.polys {
            poly.zeroize();
        }
    }
}

/// Matrix of polynomials
#[derive(Clone, Debug)]
pub struct PolyMat<const N: usize, const K: usize, const L: usize> {
    /// Matrix of polynomials (row-major order)
    pub rows: [PolyVec<N, L>; K],
}

impl<const N: usize, const K: usize, const L: usize> PolyMat<N, K, L> {
    /// Create a new zero polynomial matrix
    pub fn zero() -> Self {
        Self {
            rows: core::array::from_fn(|_| PolyVec::zero()),
        }
    }
    
    /// Matrix-vector multiplication
    pub fn mul_vec(&self, vec: &PolyVec<N, L>) -> PolyVec<N, K> {
        let mut result = PolyVec::zero();
        
        for i in 0..K {
            result.polys[i] = self.rows[i].inner_product(vec);
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_poly_add() {
        let mut a = Poly::<256>::zero();
        let mut b = Poly::<256>::zero();
        
        a.coeffs[0] = 100;
        b.coeffs[0] = 200;
        
        let c = a + b;
        assert_eq!(c.coeffs[0], 300);
    }
    
    #[test]
    fn test_poly_sub() {
        let mut a = Poly::<256>::zero();
        let mut b = Poly::<256>::zero();
        
        a.coeffs[0] = 300;
        b.coeffs[0] = 100;
        
        let c = a - b;
        assert_eq!(c.coeffs[0], 200);
    }
}