//! Tests for Number Theoretic Transform operations

use synapsed_crypto::ntt::{ntt, inv_ntt, basemul};

#[test]
fn test_ntt_inverse_identity() {
    // Test that NTT followed by inverse NTT gives back the original
    let original = [1i16, 2, 3, 4, 5, 6, 7, 8, 9, 10, 
                    11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                    21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
                    31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
                    41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
                    51, 52, 53, 54, 55, 56, 57, 58, 59, 60,
                    61, 62, 63, 64, 65, 66, 67, 68, 69, 70,
                    71, 72, 73, 74, 75, 76, 77, 78, 79, 80,
                    81, 82, 83, 84, 85, 86, 87, 88, 89, 90,
                    91, 92, 93, 94, 95, 96, 97, 98, 99, 100,
                    101, 102, 103, 104, 105, 106, 107, 108, 109, 110,
                    111, 112, 113, 114, 115, 116, 117, 118, 119, 120,
                    121, 122, 123, 124, 125, 126, 127, 128, 129, 130,
                    131, 132, 133, 134, 135, 136, 137, 138, 139, 140,
                    141, 142, 143, 144, 145, 146, 147, 148, 149, 150,
                    151, 152, 153, 154, 155, 156, 157, 158, 159, 160,
                    161, 162, 163, 164, 165, 166, 167, 168, 169, 170,
                    171, 172, 173, 174, 175, 176, 177, 178, 179, 180,
                    181, 182, 183, 184, 185, 186, 187, 188, 189, 190,
                    191, 192, 193, 194, 195, 196, 197, 198, 199, 200,
                    201, 202, 203, 204, 205, 206, 207, 208, 209, 210,
                    211, 212, 213, 214, 215, 216, 217, 218, 219, 220,
                    221, 222, 223, 224, 225, 226, 227, 228, 229, 230,
                    231, 232, 233, 234, 235, 236, 237, 238, 239, 240,
                    241, 242, 243, 244, 245, 246, 247, 248, 249, 250,
                    251, 252, 253, 254, 255, 256];
    
    let mut coeffs = original;
    
    // Apply NTT
    ntt(&mut coeffs);
    
    // Apply inverse NTT
    inv_ntt(&mut coeffs);
    
    // Check that we get back the original (modulo q)
    for i in 0..256 {
        let reduced = (coeffs[i] % 3329 + 3329) % 3329;
        let original_reduced = (original[i] % 3329 + 3329) % 3329;
        assert_eq!(reduced, original_reduced, "Mismatch at index {i}");
    }
}

#[test]
fn test_ntt_zero_polynomial() {
    let mut coeffs = [0i16; 256];
    
    // NTT of zero polynomial should be zero
    ntt(&mut coeffs);
    
    for &coeff in &coeffs {
        assert_eq!(coeff, 0);
    }
    
    // Inverse NTT of zero should also be zero
    inv_ntt(&mut coeffs);
    
    for &coeff in &coeffs {
        assert_eq!(coeff, 0);
    }
}

#[test]
fn test_basemul_identity() {
    // Test that multiplying by 1 in NTT domain gives the same result
    let mut a = [0i16; 256];
    let mut one = [0i16; 256];
    let mut result = [0i16; 256];
    
    // Set a to some values
    for (i, item) in a.iter_mut().enumerate() {
        *item = (i as i16) % 100;
    }
    
    // Set one to the NTT of polynomial 1
    one[0] = 1;
    ntt(&mut one);
    
    // Transform a to NTT domain
    let mut a_ntt = a;
    ntt(&mut a_ntt);
    
    // Multiply
    basemul(&mut result, &a_ntt, &one);
    
    // Transform back
    inv_ntt(&mut result);
    
    // Should get back a (modulo q)
    for i in 0..256 {
        let result_reduced = (result[i] % 3329 + 3329) % 3329;
        let a_reduced = (a[i] % 3329 + 3329) % 3329;
        assert_eq!(result_reduced, a_reduced, "Mismatch at index {i}");
    }
}

#[test]
fn test_basemul_commutativity() {
    // Test that a * b = b * a in NTT domain
    let mut a = [0i16; 256];
    let mut b = [0i16; 256];
    let mut result1 = [0i16; 256];
    let mut result2 = [0i16; 256];
    
    // Initialize with some values
    for i in 0..256 {
        a[i] = ((i * 7 + 13) % 3329) as i16;
        b[i] = ((i * 11 + 17) % 3329) as i16;
    }
    
    // Transform to NTT domain
    ntt(&mut a);
    ntt(&mut b);
    
    // Compute a * b
    basemul(&mut result1, &a, &b);
    
    // Compute b * a
    basemul(&mut result2, &b, &a);
    
    // Results should be the same
    for i in 0..256 {
        assert_eq!(result1[i], result2[i], "Mismatch at index {i}");
    }
}

#[test]
fn test_ntt_linearity() {
    // Test that NTT(a + b) = NTT(a) + NTT(b)
    let mut a = [0i16; 256];
    let mut b = [0i16; 256];
    let mut sum = [0i16; 256];
    
    // Initialize with some values
    for i in 0..256 {
        a[i] = ((i * 3) % 3329) as i16;
        b[i] = ((i * 5) % 3329) as i16;
        sum[i] = (a[i] + b[i]) % 3329;
    }
    
    // Compute NTT(a + b)
    let mut sum_ntt = sum;
    ntt(&mut sum_ntt);
    
    // Compute NTT(a) + NTT(b)
    let mut a_ntt = a;
    let mut b_ntt = b;
    ntt(&mut a_ntt);
    ntt(&mut b_ntt);
    
    let mut ntt_sum = [0i16; 256];
    for i in 0..256 {
        ntt_sum[i] = (a_ntt[i] + b_ntt[i]) % 3329;
    }
    
    // Should be equal
    for i in 0..256 {
        let sum_reduced = (sum_ntt[i] % 3329 + 3329) % 3329;
        let ntt_sum_reduced = (ntt_sum[i] % 3329 + 3329) % 3329;
        assert_eq!(sum_reduced, ntt_sum_reduced, "Mismatch at index {i}");
    }
}

#[test]
fn test_ntt_bounds() {
    // Test that NTT maintains coefficient bounds
    let mut coeffs = [0i16; 256];
    
    // Initialize with maximum positive values
    for coeff in coeffs.iter_mut() {
        *coeff = 1664; // q/2
    }
    
    // Apply NTT
    ntt(&mut coeffs);
    
    // Check bounds
    for &coeff in &coeffs {
        assert!(coeff.abs() < 3329, "Coefficient out of bounds: {coeff}");
    }
    
    // Apply inverse NTT
    inv_ntt(&mut coeffs);
    
    // Check bounds again
    for &coeff in &coeffs {
        assert!(coeff.abs() < 3329, "Coefficient out of bounds after inv_ntt: {coeff}");
    }
}