//! Test to demonstrate the compression/decompression asymmetry bug

#[test]
fn test_compression_asymmetry_bug() {
    // Test value that will show the bug
    let original_coeff = 1000i16;
    
    // Compression step (from compress_poly_10bit)
    let coeff_u32 = original_coeff as u32;
    let compressed = ((coeff_u32.wrapping_mul(1024).wrapping_add(1664)) / 3329) & 0x3FF;
    
    println!("Original coefficient: {original_coeff}");
    println!("After compression: {compressed}");
    println!("Compression calculation: ({original_coeff} * 1024 + 1664) / 3329 = {compressed}");
    
    // Decompression step (from decompress_poly_10bit) - WITH BUG
    let decompressed_with_bug = ((compressed * 3329 + 512) / 1024) as i16;
    println!("\nDecompression WITH BUG (using +512):");
    println!("({compressed} * 3329 + 512) / 1024 = {decompressed_with_bug}");
    
    // Decompression step - FIXED VERSION
    let decompressed_fixed = ((compressed * 3329 + 1664) / 1024) as i16;
    println!("\nDecompression FIXED (using +1664):");
    println!("({compressed} * 3329 + 1664) / 1024 = {decompressed_fixed}");
    
    println!("\nResults:");
    println!("Original:    {original_coeff}");
    println!("With bug:    {decompressed_with_bug} (difference: {})", 
             decompressed_with_bug - original_coeff);
    println!("Fixed:       {decompressed_fixed} (difference: {})",
             decompressed_fixed - original_coeff);
    
    // The bug causes incorrect decompression
    assert_ne!(decompressed_with_bug, original_coeff, "Bug reproduced!");
    // Fixed version should be much closer (may have small rounding error)
    assert!((decompressed_fixed - original_coeff).abs() <= 1, "Fixed version works!");
}

#[test]
fn test_full_poly_compression_bug() {
    use synapsed_crypto::utils::{compress_poly, decompress_poly};
    
    // Create a polynomial with various coefficients
    let mut original_coeffs = vec![0i16; 256];
    for (i, original_coeff) in original_coeffs.iter_mut().enumerate().take(256) {
        // Use a variety of values to test edge cases
        *original_coeff = ((i * 13) % 3329) as i16 - 1664;
    }
    
    // Compress with d=10
    let mut compressed = vec![0u8; 320]; // 256 * 10 / 8
    compress_poly(&original_coeffs, 10, &mut compressed).unwrap();
    
    // Decompress
    let mut decompressed_coeffs = vec![0i16; 256];
    decompress_poly(&compressed, 10, &mut decompressed_coeffs).unwrap();
    
    // Check differences
    let mut max_diff = 0i16;
    let mut total_diff = 0i64;
    let mut mismatches = 0;
    
    for i in 0..256 {
        let diff = (decompressed_coeffs[i] - original_coeffs[i]).abs();
        if diff > 0 {
            mismatches += 1;
        }
        max_diff = max_diff.max(diff);
        total_diff += diff as i64;
    }
    
    println!("\nFull polynomial compression test:");
    println!("Coefficients with differences: {mismatches}/256");
    println!("Maximum difference: {max_diff}");
    println!("Average difference: {:.2}", total_diff as f64 / 256.0);
    
    // With the bug, we expect significant differences
    assert!(mismatches > 100, "Bug causes widespread coefficient mismatches");
    assert!(max_diff > 2, "Bug causes large differences in some coefficients");
}

#[test]
fn test_compression_edge_cases() {
    // Test specific edge cases that trigger the bug
    let test_cases = [
        0i16,      // Zero
        1664,      // Half of q
        3328,      // q - 1
        -1664,     // Negative half
        1000,      // Example value
        -1000,     // Negative example
        2500,      // Large positive
        -2500,     // Large negative
    ];
    
    println!("\nEdge case analysis:");
    println!("{:>10} | {:>10} | {:>10} | {:>10}", "Original", "Compressed", "Bug Decomp", "Fixed Decomp");
    println!("{:-<50}", "");
    
    for &coeff in &test_cases {
        // Handle negative coefficients as in the actual code
        let coeff_u32 = if coeff < 0 {
            (coeff + 3329) as u32
        } else {
            coeff as u32
        };
        
        let compressed = ((coeff_u32.wrapping_mul(1024).wrapping_add(1664)) / 3329) & 0x3FF;
        let decompressed_bug = ((compressed * 3329 + 512) / 1024) as i16;
        let decompressed_fix = ((compressed * 3329 + 1664) / 1024) as i16;
        
        println!("{coeff:>10} | {compressed:>10} | {decompressed_bug:>10} | {decompressed_fix:>10}");
    }
}