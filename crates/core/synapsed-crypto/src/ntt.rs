//! Number Theoretic Transform (NTT) operations
//!
//! This module implements the NTT and inverse NTT operations
//! used for efficient polynomial multiplication.

use crate::utils::montgomery_reduce;

/// Precomputed zetas for NTT
const ZETAS: [i16; 128] = [
    -1044, -758, -359, -1517, 1493, 1422, 287, 202,
    -171, 622, 1577, 182, 962, -1202, -1474, 1468,
    573, -1325, 264, 383, -829, 1458, -1602, -130,
    -681, 1017, 732, 608, -1542, 411, -205, -1571,
    1223, 652, -552, 1015, -1293, 1491, -282, -1544,
    516, -8, -320, -666, -1618, -1162, 126, 1469,
    -853, -90, -271, 830, 107, -1421, -247, -951,
    -398, 961, -1508, -725, 448, -1065, 677, -1275,
    -1103, 430, 555, 843, -1251, 871, 1550, 105,
    422, 587, 177, -235, -291, -460, 1574, 1653,
    -246, 778, 1159, -147, -777, 1483, -602, 1119,
    -1590, 644, -872, 349, 418, 329, -156, -75,
    817, 1097, 603, 610, 1322, -1285, -1465, 384,
    -1215, -136, 1218, -1335, -874, 220, -1187, -1659,
    -1185, -1530, -1278, 794, -1510, -854, -870, 478,
    -108, -308, 996, 991, 958, -1460, 1522, 1628
];

/// Perform forward NTT
pub fn ntt(coeffs: &mut [i16]) {
    let mut k = 1;
    let mut len = 128;
    
    while len >= 2 {
        for start in (0..256).step_by(2 * len) {
            let zeta = ZETAS[k];
            k += 1;
            
            for j in 0..len {
                let t = montgomery_reduce((zeta as i32).wrapping_mul(coeffs[start + j + len] as i32));
                coeffs[start + j + len] = coeffs[start + j].wrapping_sub(t);
                coeffs[start + j] = coeffs[start + j].wrapping_add(t);
            }
        }
        len >>= 1;
    }
}

/// Perform inverse NTT
pub fn inv_ntt(coeffs: &mut [i16]) {
    let mut k = 127;
    let mut len = 2;
    
    while len <= 128 {
        for start in (0..256).step_by(2 * len) {
            let zeta = -ZETAS[k];
            k -= 1;
            
            for j in 0..len {
                let t = coeffs[start + j];
                coeffs[start + j] = t.wrapping_add(coeffs[start + j + len]);
                coeffs[start + j + len] = montgomery_reduce((zeta as i32).wrapping_mul((coeffs[start + j + len].wrapping_sub(t)) as i32));
            }
        }
        len <<= 1;
    }
    
    // Final multiplication by n^{-1}
    const F: i16 = 1441; // n^{-1} mod q
    for coeff in coeffs {
        *coeff = montgomery_reduce((F as i32).wrapping_mul(*coeff as i32));
    }
}

/// Pointwise multiplication of polynomials in NTT domain
pub fn basemul(r: &mut [i16], a: &[i16], b: &[i16]) {
    const ZETA: i16 = -1044; // zeta^{2*64+1}
    
    for i in 0..64 {
        let (a0, a1) = (a[4*i], a[4*i + 1]);
        let (a2, a3) = (a[4*i + 2], a[4*i + 3]);
        let (b0, b1) = (b[4*i], b[4*i + 1]);
        let (b2, b3) = (b[4*i + 2], b[4*i + 3]);
        
        r[4*i] = montgomery_reduce((a0 as i32).wrapping_mul(b0 as i32).wrapping_add((ZETA as i32).wrapping_mul(a1 as i32).wrapping_mul(b1 as i32)));
        r[4*i + 1] = montgomery_reduce((a0 as i32).wrapping_mul(b1 as i32).wrapping_add((a1 as i32).wrapping_mul(b0 as i32)));
        r[4*i + 2] = montgomery_reduce((a2 as i32).wrapping_mul(b2 as i32).wrapping_add((ZETA as i32).wrapping_mul(a3 as i32).wrapping_mul(b3 as i32)));
        r[4*i + 3] = montgomery_reduce((a2 as i32).wrapping_mul(b3 as i32).wrapping_add((a3 as i32).wrapping_mul(b2 as i32)));
    }
}

/// Dilithium NTT constants
mod dilithium_constants {
    pub const DILITHIUM_N: usize = 256;
    #[allow(dead_code)]
    pub const DILITHIUM_Q: i32 = 8380417; // 2^23 - 2^13 + 1
    #[allow(dead_code)]
    pub const DILITHIUM_ROOT_OF_UNITY: i32 = 1753; // 256th root of unity
    
    /// Precomputed roots of unity for Dilithium NTT
    pub const DILITHIUM_ZETAS: [i32; 256] = [
        0, 25847, -2608894, -518909, 237124, -777960, -876248, 466468,
        1826347, 2353451, -359251, -2091905, 3119733, -2884855, 3111497, 2680103,
        2725464, 1024112, -1079900, 3585928, -549488, -1119584, 2619752, -2108549,
        -2118186, -3859737, -1399561, -3277672, 1757237, -19422, 4010497, 280005,
        2706023, 95776, 3077325, 3530437, -1661693, -3592148, -2537516, 3915439,
        -3861115, -3043716, 3574422, -2867647, 3539968, -300467, 2348700, -539299,
        -1699267, -1643818, 3505694, -3821735, 3507263, -2140649, -1600420, 3699596,
        811944, 531354, 954230, 3881043, 3900724, -2556880, 2071892, -2797779,
        -3930395, -1528703, -3677745, -3041255, -1452451, 3475950, 2176455, -1585221,
        -1257611, 1939314, -4083598, -1000202, -3190144, -3157330, -3632928, 126922,
        3412210, -983419, 2147896, 2715295, -2967645, -3693493, -411027, -2477047,
        -671102, -1228525, -22981, -1308169, -381987, 1349076, 1852771, -1430430,
        -3343383, 264944, 508951, 3097992, 44288, -1100098, 904516, 3958618,
        -3724342, -8578, 1653064, -3249728, 2389356, -210977, 759969, -1316856,
        189548, -3553272, 3159746, -1851402, -2409325, -177440, 1315589, 1341330,
        1285669, -1584928, -812732, -1439742, -3019102, -3881060, -3628969, 3839961,
        2091667, 3407706, 2316500, 3817976, -3342478, 2244091, -2446433, -3562462,
        266997, 2434439, -1235728, 3513181, -3520352, -3759364, -1197226, -3193378,
        900702, 1859098, 909542, 819034, 495491, -1613174, -43260, -522500,
        -655327, -3122442, 2031748, 3207046, -3556995, -525098, -768622, -3595838,
        342297, 286988, -2437823, 4108315, 3437287, -3342277, 1735879, 203044,
        2842341, 2691481, -2590150, 1265009, 4055324, 1247620, 2486353, 1595974,
        -3767016, 1250494, 2635921, -3548272, -2994039, 1869119, 1903435, -1050970,
        -1333058, 1237275, -3318210, -1430225, -451100, 1312455, 3306115, -1962642,
        -1279661, 1917081, -2546312, -1374803, 1500165, 777191, 2235880, 3406031,
        -542412, -2831860, -1671176, -1846953, -2584293, -3724270, 594136, -3776993,
        -2013608, 2432395, 2454455, -164721, 1957272, 3369112, 185531, -1207385,
        -3183426, 162844, 1616392, 3014001, 810149, 1652634, -3694233, -1799107,
        -3038916, 3523897, 3866901, 269760, 2213111, -975884, 1717735, 472078,
        -426683, 1723600, -1803090, 1910376, -1667432, -1104333, -260646, -3833893,
        -2939036, -2235985, -420899, -2286327, 183443, -976891, 1612842, -3545687,
        -554416, 3919660, -48306, -1362209, 3937738, 1400424, -846154, 1976782
    ];
}

use dilithium_constants::*;

/// Dilithium NTT forward transform  
pub fn dilithium_ntt(coeffs: &mut [i32]) {
    let mut k = 0;
    let mut len = 128;
    
    while len >= 1 {
        for start in (0..DILITHIUM_N).step_by(2 * len) {
            k += 1;
            let zeta = DILITHIUM_ZETAS[k];
            
            for j in 0..len {
                // Use Montgomery reduction since ZETAS are in Montgomery form
                let t = crate::utils::montgomery_reduce_dilithium(zeta as i64 * coeffs[start + j + len] as i64);
                coeffs[start + j + len] = coeffs[start + j] - t;
                coeffs[start + j] += t;
            }
        }
        len >>= 1;
    }
}

/// Dilithium inverse NTT
pub fn dilithium_inv_ntt(coeffs: &mut [i32]) {
    let mut k = 256;
    let mut len = 1;
    
    while len < DILITHIUM_N {
        for start in (0..DILITHIUM_N).step_by(2 * len) {
            k -= 1;
            let zeta = -DILITHIUM_ZETAS[k];
            
            for j in 0..len {
                let t = coeffs[start + j];
                coeffs[start + j] = t + coeffs[start + j + len];
                coeffs[start + j + len] = crate::utils::montgomery_reduce_dilithium(
                    zeta as i64 * (t - coeffs[start + j + len]) as i64
                );
            }
        }
        len <<= 1;
    }
    
    // Multiply by N^{-1} in Montgomery form
    // We need N^{-1} * R^{-1} mod Q where R = 2^32
    const F: i32 = 41978; // 256^{-1} * 2^{-32} mod Q
    
    for coeff in coeffs {
        *coeff = crate::utils::montgomery_reduce_dilithium(F as i64 * *coeff as i64);
    }
}

/// Pointwise multiplication of polynomials in NTT domain for Dilithium
pub fn dilithium_basemul(r: &mut [i32], a: &[i32], b: &[i32]) {
    // For Dilithium, we do simple pointwise multiplication since NTT is already applied
    for i in 0..DILITHIUM_N {
        r[i] = crate::utils::montgomery_reduce_dilithium(a[i] as i64 * b[i] as i64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::kyber::Q;
    
    #[test]
    fn test_ntt_inv_ntt() {
        let mut coeffs = [0i16; 256];
        let original = coeffs;
        
        // NTT followed by inverse NTT should give back original
        ntt(&mut coeffs);
        inv_ntt(&mut coeffs);
        
        // Account for scaling
        for i in 0..256 {
            coeffs[i] = ((coeffs[i] as i32 % Q as i32 + Q as i32) % Q as i32) as i16;
            assert_eq!(coeffs[i], original[i]);
        }
    }
    
    #[test] 
    fn test_dilithium_ntt_zero() {
        let mut coeffs = [0i32; 256];
        let original = coeffs;
        
        // NTT of zero should be zero
        dilithium_ntt(&mut coeffs);
        dilithium_inv_ntt(&mut coeffs);
        
        for i in 0..256 {
            assert_eq!(coeffs[i], original[i], "Failed at index {i}");
        }
    }

    #[test]
    fn test_n_inv() {
        // Test that N_INV is correct
        const N_INV: i32 = 8347681;
        let product = ((256i64 * N_INV as i64) % DILITHIUM_Q as i64) as i32;
        println!("256 * {N_INV} mod {DILITHIUM_Q} = {product}");
        assert_eq!(product, 1, "N_INV is incorrect");
    }
    
    #[test]
    fn test_dilithium_ntt_simple() {
        // Test with all ones to check scaling
        let mut coeffs = [1i32; 256];
        let _original = coeffs;
        
        // Apply NTT and inverse NTT
        dilithium_ntt(&mut coeffs);
        dilithium_inv_ntt(&mut coeffs);
        
        // Check if all coefficients are scaled by the same factor
        let scale_factor = coeffs[0];
        println!("Scale factor appears to be: {scale_factor}");
        
        // Verify all coefficients have same scale
        for (i, &coeff) in coeffs.iter().enumerate().take(256) {
            assert_eq!(coeff, scale_factor, "Coefficient {i} differs");
        }
    }

    #[test]
    fn test_dilithium_ntt_inv_ntt() {
        // Test proper NTT round-trip
        let mut coeffs = [0i32; 256];
        for (i, coeff) in coeffs.iter_mut().enumerate().take(256) {
            *coeff = i as i32;
        }
        let original = coeffs;
        
        // NTT followed by inverse NTT should give back original
        dilithium_ntt(&mut coeffs);
        dilithium_inv_ntt(&mut coeffs);
        
        // Check results
        for i in 0..256 {
            assert_eq!(coeffs[i], original[i], "Failed at index {i}");
        }
    }
}