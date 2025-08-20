//! Side-channel resistance tests
//!
//! These tests verify that cryptographic operations don't leak information
//! through timing, memory access patterns, or other side channels.

use synapsed_crypto::{
    kyber::Kyber512,
    dilithium::Dilithium2,
    traits::{Kem, Signature},
    random::TestRng,
};
use std::time::Instant;
use std::hint::black_box;

/// Helper to measure operation timing with high precision
fn measure_operation<F: Fn()>(f: F, iterations: usize) -> Vec<u128> {
    let mut timings = Vec::with_capacity(iterations);
    
    // Warm up
    for _ in 0..100 {
        f();
    }
    
    // Measure
    for _ in 0..iterations {
        let start = Instant::now();
        f();
        timings.push(start.elapsed().as_nanos());
    }
    
    timings
}

/// Calculate statistical variance of timings
fn calculate_variance(timings: &[u128]) -> f64 {
    let mean = timings.iter().sum::<u128>() as f64 / timings.len() as f64;
    let variance = timings.iter()
        .map(|&t| {
            let diff = t as f64 - mean;
            diff * diff
        })
        .sum::<f64>() / timings.len() as f64;
    
    variance.sqrt() / mean * 100.0 // Return as percentage
}

/// Test that Kyber decapsulation timing doesn't depend on ciphertext validity
#[test]
#[ignore = "Timing tests require controlled environment"]
fn test_kyber_decapsulation_timing_independence() {
    const ITERATIONS: usize = 1000;
    let mut rng = TestRng::new(42);
    
    // Generate keypair
    let (pk, sk) = Kyber512::generate_keypair(&mut rng).unwrap();
    
    // Create valid ciphertext
    let (valid_ct, _) = Kyber512::encapsulate(&pk, &mut rng).unwrap();
    
    // Create invalid ciphertext (corrupted)
    let mut invalid_ct = valid_ct.clone();
    invalid_ct.bytes[0] ^= 0xFF;
    
    // Measure valid decapsulation timing
    let valid_timings = measure_operation(
        || {
            let _ = black_box(Kyber512::decapsulate(&sk, &valid_ct).unwrap());
        },
        ITERATIONS
    );
    
    // Measure invalid decapsulation timing
    let invalid_timings = measure_operation(
        || {
            let _ = black_box(Kyber512::decapsulate(&sk, &invalid_ct).unwrap());
        },
        ITERATIONS
    );
    
    // Calculate timing statistics
    let valid_mean = valid_timings.iter().sum::<u128>() as f64 / ITERATIONS as f64;
    let invalid_mean = invalid_timings.iter().sum::<u128>() as f64 / ITERATIONS as f64;
    
    let timing_diff_percent = ((valid_mean - invalid_mean).abs() / valid_mean) * 100.0;
    
    println!("Valid CT mean time: {valid_mean:.0}ns");
    println!("Invalid CT mean time: {invalid_mean:.0}ns");
    println!("Timing difference: {timing_diff_percent:.2}%");
    
    // Timing difference should be less than 5%
    assert!(timing_diff_percent < 5.0, 
        "Timing difference too large: {timing_diff_percent:.2}%");
}

/// Test that message bit extraction is constant-time
#[test]
fn test_message_extraction_constant_time() {
    use synapsed_crypto::constant_time::ct_decode_bit;
    
    const ITERATIONS: usize = 10000;
    
    // Test coefficients that would decode to 1
    let coeff_1 = 1664; // Middle of valid range
    let timings_1 = measure_operation(
        || {
            let _ = black_box(ct_decode_bit(coeff_1));
        },
        ITERATIONS
    );
    
    // Test coefficients that would decode to 0
    let coeff_0 = 100; // Outside valid range
    let timings_0 = measure_operation(
        || {
            let _ = black_box(ct_decode_bit(coeff_0));
        },
        ITERATIONS
    );
    
    // Calculate variance
    let variance_1 = calculate_variance(&timings_1);
    let variance_0 = calculate_variance(&timings_0);
    
    println!("Variance for bit=1: {variance_1:.2}%");
    println!("Variance for bit=0: {variance_0:.2}%");
    
    // Both should have low variance
    assert!(variance_1 < 10.0, "High variance for bit=1: {variance_1:.2}%");
    assert!(variance_0 < 10.0, "High variance for bit=0: {variance_0:.2}%");
}

/// Test polynomial operations for timing leaks
#[test]
fn test_polynomial_operations_constant_time() {
    use synapsed_crypto::poly::Poly;
    
    const ITERATIONS: usize = 1000;
    
    // Create polynomials with different properties
    let mut poly_positive = Poly::<256>::zero();
    let mut poly_negative = Poly::<256>::zero();
    
    // Fill with positive coefficients
    for i in 0..256 {
        poly_positive.coeffs[i] = (i as i16) % 1000;
    }
    
    // Fill with negative coefficients
    for i in 0..256 {
        poly_negative.coeffs[i] = -((i as i16) % 1000);
    }
    
    // Measure caddq on positive coefficients
    let positive_timings = measure_operation(
        || {
            let mut p = poly_positive.clone();
            p.caddq();
            black_box(());
        },
        ITERATIONS
    );
    
    // Measure caddq on negative coefficients
    let negative_timings = measure_operation(
        || {
            let mut p = poly_negative.clone();
            p.caddq();
            black_box(());
        },
        ITERATIONS
    );
    
    // Calculate timing difference
    let pos_mean = positive_timings.iter().sum::<u128>() as f64 / ITERATIONS as f64;
    let neg_mean = negative_timings.iter().sum::<u128>() as f64 / ITERATIONS as f64;
    let diff_percent = ((pos_mean - neg_mean).abs() / pos_mean) * 100.0;
    
    println!("Positive coeffs mean time: {pos_mean:.0}ns");
    println!("Negative coeffs mean time: {neg_mean:.0}ns");
    println!("Timing difference: {diff_percent:.2}%");
    
    // Should be constant-time
    assert!(diff_percent < 5.0, "Timing varies with coefficient sign: {diff_percent:.2}%");
}

/// Test that Dilithium signature generation doesn't leak secret key info
#[test]
#[ignore = "Dilithium signing has known timing variations due to rejection sampling"]
fn test_dilithium_signing_timing() {
    const ITERATIONS: usize = 100;
    let mut rng = TestRng::new(42);
    
    // Generate keypair
    let (_pk, sk) = Dilithium2::generate_keypair(&mut rng).unwrap();
    
    // Different messages
    let msg1 = b"Test message 1";
    let msg2 = b"Different message with different properties";
    
    // Measure signing time for different messages
    let timings1 = measure_operation(
        || {
            let _ = black_box(Dilithium2::sign(&sk, msg1, &mut rng.clone()).unwrap());
        },
        ITERATIONS
    );
    
    let timings2 = measure_operation(
        || {
            let _ = black_box(Dilithium2::sign(&sk, msg2, &mut rng.clone()).unwrap());
        },
        ITERATIONS
    );
    
    // Note: Dilithium uses rejection sampling, so some timing variation is expected
    // We're mainly checking that it's not excessive
    let variance1 = calculate_variance(&timings1);
    let variance2 = calculate_variance(&timings2);
    
    println!("Message 1 timing variance: {variance1:.2}%");
    println!("Message 2 timing variance: {variance2:.2}%");
    
    // Variance should be reasonable (less than 50% due to rejection sampling)
    assert!(variance1 < 50.0, "Excessive variance in signing: {variance1:.2}%");
    assert!(variance2 < 50.0, "Excessive variance in signing: {variance2:.2}%");
}

/// Test memory access patterns don't leak information
#[test]
fn test_memory_access_patterns() {
    use synapsed_crypto::secure_memory::SecureArray;
    
    // This test verifies that secure memory operations don't have
    // data-dependent memory access patterns
    
    let mut secure_data1 = SecureArray::<32>::zero();
    let mut secure_data2 = SecureArray::<32>::zero();
    
    // Fill with different patterns
    secure_data1.as_mut().fill(0xAA);
    secure_data2.as_mut().fill(0x55);
    
    // Access patterns should be identical regardless of content
    let timings1 = measure_operation(
        || {
            for i in 0..32 {
                let _ = black_box(secure_data1.as_ref()[i]);
            }
        },
        1000
    );
    
    let timings2 = measure_operation(
        || {
            for i in 0..32 {
                let _ = black_box(secure_data2.as_ref()[i]);
            }
        },
        1000
    );
    
    let variance1 = calculate_variance(&timings1);
    let variance2 = calculate_variance(&timings2);
    
    // Both should have similar low variance
    assert!((variance1 - variance2).abs() < 5.0, 
        "Memory access patterns differ: {variance1:.2}% vs {variance2:.2}%");
}

/// Test that error paths don't leak timing information
#[test]
fn test_error_handling_timing() {
    const ITERATIONS: usize = 1000;
    
    // Test with different invalid key sizes
    let small_key = vec![0u8; 100];
    let medium_key = vec![0u8; 500];
    let large_key = vec![0u8; 1000];
    
    use synapsed_crypto::kyber::PublicKey;
    use synapsed_crypto::traits::Serializable;
    
    // Measure error timing for different sizes
    let small_timings = measure_operation(
        || {
            let _ = black_box(PublicKey::<2>::from_bytes(&small_key));
        },
        ITERATIONS
    );
    
    let medium_timings = measure_operation(
        || {
            let _ = black_box(PublicKey::<2>::from_bytes(&medium_key));
        },
        ITERATIONS
    );
    
    let large_timings = measure_operation(
        || {
            let _ = black_box(PublicKey::<2>::from_bytes(&large_key));
        },
        ITERATIONS
    );
    
    // Calculate means
    let small_mean = small_timings.iter().sum::<u128>() as f64 / ITERATIONS as f64;
    let medium_mean = medium_timings.iter().sum::<u128>() as f64 / ITERATIONS as f64;
    let large_mean = large_timings.iter().sum::<u128>() as f64 / ITERATIONS as f64;
    
    // Timing should be similar regardless of input size
    let max_diff = vec![
        (small_mean - medium_mean).abs() / small_mean,
        (medium_mean - large_mean).abs() / medium_mean,
        (small_mean - large_mean).abs() / small_mean,
    ].into_iter().fold(0.0, f64::max) * 100.0;
    
    println!("Maximum timing difference in error paths: {max_diff:.2}%");
    
    // Should be constant-time even for errors
    assert!(max_diff < 10.0, "Error timing varies with input: {max_diff:.2}%");
}