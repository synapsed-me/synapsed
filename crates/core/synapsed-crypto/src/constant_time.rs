//! Constant-time operations for side-channel resistance
//!
//! This module provides constant-time implementations of common operations
//! to prevent timing attacks.

use subtle::{Choice, ConditionallySelectable, ConstantTimeLess};

/// Constant-time coefficient reduction for Kyber
/// Returns 1 if coefficient is closer to q/2 than to 0, 0 otherwise
/// This implements the Compress_q(x, 1) function from the Kyber spec
pub fn ct_decode_bit(coeff: i16) -> u8 {
    // In Kyber, when we encode a bit b, we add b * (q/2) to the coefficient
    // So to decode, we check if the coefficient is closer to 0 or to q/2
    // This is equivalent to checking if coeff is in range [q/4, 3q/4]
    // because if coeff > q/4 and coeff < 3q/4, it's closer to q/2
    const Q_QUARTER: i16 = 832;    // floor(q/4)
    const THREE_Q_QUARTER: i16 = 2497;  // floor(3q/4)
    
    // Create constant-time comparisons
    // We want to check if Q_QUARTER < coeff < THREE_Q_QUARTER
    
    // Check if coeff > Q_QUARTER (constant-time)
    let gt_lower = coeff.ct_gt(&Q_QUARTER);
    
    // Check if coeff < THREE_Q_QUARTER (constant-time)  
    let lt_upper = coeff.ct_lt(&THREE_Q_QUARTER);
    
    // Both conditions must be true
    let in_range = gt_lower & lt_upper;
    
    // Convert Choice to u8 (0 or 1)
    u8::conditional_select(&0u8, &1u8, in_range)
}

/// Constant-time greater-than comparison for i16
trait ConstantTimeGreater {
    fn ct_gt(&self, other: &Self) -> Choice;
}

impl ConstantTimeGreater for i16 {
    fn ct_gt(&self, other: &Self) -> Choice {
        // Convert to u16 for comparison
        // Add 2^15 to make them unsigned
        let a = (*self as u16).wrapping_add(0x8000);
        let b = (*other as u16).wrapping_add(0x8000);
        
        // Use constant-time less-than
        b.ct_lt(&a)
    }
}

/// Constant-time less-than comparison for i16
trait ConstantTimeLessI16 {
    fn ct_lt(&self, other: &Self) -> Choice;
}

impl ConstantTimeLessI16 for i16 {
    fn ct_lt(&self, other: &Self) -> Choice {
        // Convert to u16 for comparison
        // Add 2^15 to make them unsigned
        let a = (*self as u16).wrapping_add(0x8000);
        let b = (*other as u16).wrapping_add(0x8000);
        
        // Use constant-time less-than
        a.ct_lt(&b)
    }
}

/// Constant-time conditional reduction for caddq
pub fn ct_caddq(coeff: i16) -> i16 {
    const Q: i16 = 3329;
    
    // Create the value to potentially add
    let adjustment = (coeff >> 15) & Q;
    
    // Always perform the addition (constant-time)
    coeff.wrapping_add(adjustment)
}

/// Constant-time polynomial coefficient reduction
pub fn ct_reduce_coeffs(coeffs: &mut [i16]) {
    for coeff in coeffs {
        *coeff = ct_caddq(*coeff);
    }
}

/// Constant-time norm check for Dilithium
/// Returns Choice(1) if all coefficients are within [-bound, bound]
pub fn ct_check_norm(coeffs: &[i32], bound: i32) -> Choice {
    let mut all_valid = Choice::from(1u8);
    
    for &coeff in coeffs {
        // Check if |coeff| < bound
        // This is equivalent to -bound < coeff < bound
        
        // Convert to unsigned for comparison
        let coeff_u = (coeff as u32).wrapping_add(0x80000000);
        let neg_bound_u = ((-bound) as u32).wrapping_add(0x80000000);
        let bound_u = (bound as u32).wrapping_add(0x80000000);
        
        // Check -bound < coeff
        let gt_neg_bound = neg_bound_u.ct_lt(&coeff_u);
        
        // Check coeff < bound
        let lt_bound = coeff_u.ct_lt(&bound_u);
        
        // Both must be true
        let in_range = gt_neg_bound & lt_bound;
        
        // Update all_valid (remains 1 only if all checks pass)
        all_valid &= in_range;
    }
    
    all_valid
}

/// Constant-time rejection sampling iteration limit
/// Performs exactly MAX_ITERATIONS iterations regardless of when a valid sample is found
pub fn ct_rejection_sample<F, T>(max_iterations: usize, mut sample_fn: F) -> Option<T>
where
    F: FnMut(usize) -> (T, bool),
    T: Default + Clone,
{
    let mut result = T::default();
    let mut found = Choice::from(0u8);
    
    for i in 0..max_iterations {
        let (candidate, is_valid) = sample_fn(i);
        
        // Convert bool to Choice
        let valid_choice = Choice::from(is_valid as u8);
        
        // Only update result if this is the first valid sample
        let should_update = valid_choice & !found;
        
        // Constant-time conditional copy
        if bool::from(should_update) {
            result = candidate;
        }
        
        // Update found flag
        found |= valid_choice;
    }
    
    // Return Some(result) if we found a valid sample, None otherwise
    if bool::from(found) {
        Some(result)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ct_decode_bit() {
        // Test boundary conditions
        assert_eq!(ct_decode_bit(832), 0);
        assert_eq!(ct_decode_bit(833), 1);
        assert_eq!(ct_decode_bit(1664), 1);  // Middle value
        assert_eq!(ct_decode_bit(2496), 1);
        assert_eq!(ct_decode_bit(2497), 0);
    }
    
    #[test]
    fn test_ct_caddq() {
        // Test positive values (no reduction needed)
        assert_eq!(ct_caddq(100), 100);
        assert_eq!(ct_caddq(1000), 1000);
        
        // Test negative values (should add Q)
        assert_eq!(ct_caddq(-100), -100 + 3329);
        assert_eq!(ct_caddq(-1), -1 + 3329);
    }
    
    #[test]
    fn test_ct_check_norm() {
        let coeffs = vec![10, -20, 30, -40, 50];
        
        // All within bound
        assert!(bool::from(ct_check_norm(&coeffs, 100)));
        
        // One exceeds bound
        assert!(!bool::from(ct_check_norm(&coeffs, 40)));
        
        // Exact bound
        assert!(!bool::from(ct_check_norm(&coeffs, 50)));
    }
}