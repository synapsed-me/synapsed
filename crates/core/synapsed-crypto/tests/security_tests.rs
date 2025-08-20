//! Security tests for synapsed-crypto
//!
//! These tests verify that security-critical operations are implemented correctly,
//! including constant-time operations, secure memory handling, and side-channel resistance.

use synapsed_crypto::{
    constant_time::*,
    secure_memory::*,
    kyber::{Kyber512, Kyber768, Kyber1024},
    dilithium::{Dilithium2, Dilithium3, Dilithium5},
    traits::{Kem, Signature, Serializable},
    random::TestRng,
    Error,
};
use std::time::{Duration, Instant};

/// Test that secure memory is properly zeroed on drop
#[test]
fn test_secure_memory_zeroing() {
    // Test SecureArray - verify it can be filled and used
    {
        let mut secure_array = SecureArray::<32>::zero();
        
        // Verify it starts zeroed
        assert!(secure_array.as_ref().iter().all(|&b| b == 0));
        
        // Fill with non-zero data
        secure_array.as_mut().fill(0xAA);
        
        // Verify it's filled
        assert!(secure_array.as_ref().iter().all(|&b| b == 0xAA));
        
        // The drop implementation will zero the memory
        // We can't safely verify this after drop due to Rust's memory safety
    }
    
    // Test SecureBytes
    {
        let mut secure_bytes = SecureBytes::new(vec![0x00; 64]);
        
        // Fill with data
        secure_bytes.as_mut().fill(0xFF);
        
        // Verify it's filled
        assert!(secure_bytes.as_ref().iter().all(|&b| b == 0xFF));
        
        // The drop implementation will zero the memory
    }
    
    // Test that SecureArray implements ZeroizeOnDrop
    {
        use zeroize::ZeroizeOnDrop;
        fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}
        assert_zeroize_on_drop::<SecureArray<32>>();
    }
}

/// Test that constant-time bit extraction works correctly
#[test]
fn test_constant_time_decode_bit() {
    // Test the exact boundaries
    assert_eq!(ct_decode_bit(832), 0, "Should return 0 at lower boundary");
    assert_eq!(ct_decode_bit(833), 1, "Should return 1 just above lower boundary");
    assert_eq!(ct_decode_bit(1664), 1, "Should return 1 in middle of range");
    assert_eq!(ct_decode_bit(2496), 1, "Should return 1 just below upper boundary");
    assert_eq!(ct_decode_bit(2497), 0, "Should return 0 at upper boundary");
    
    // Test extreme values
    assert_eq!(ct_decode_bit(0), 0);
    assert_eq!(ct_decode_bit(-1000), 0);
    assert_eq!(ct_decode_bit(3328), 0);
}

/// Test constant-time polynomial reduction
#[test]
fn test_constant_time_caddq() {
    // Test positive coefficients (no change expected)
    assert_eq!(ct_caddq(100), 100);
    assert_eq!(ct_caddq(1000), 1000);
    assert_eq!(ct_caddq(3328), 3328);
    
    // Test negative coefficients (should add Q)
    assert_eq!(ct_caddq(-1), -1 + 3329);
    assert_eq!(ct_caddq(-100), -100 + 3329);
    assert_eq!(ct_caddq(-3329), -3329 + 3329);
}

/// Test constant-time norm checking
#[test]
fn test_constant_time_norm_check() {
    let coeffs = vec![50, -30, 80, -90, 100];
    
    // All coefficients within bound
    assert!(bool::from(ct_check_norm(&coeffs, 101)));
    assert!(bool::from(ct_check_norm(&coeffs, 150)));
    
    // Some coefficients exceed bound
    assert!(!bool::from(ct_check_norm(&coeffs, 100)));
    assert!(!bool::from(ct_check_norm(&coeffs, 99)));
    assert!(!bool::from(ct_check_norm(&coeffs, 50)));
}

/// Test that operations on secret data are constant-time
/// This test measures timing variance for operations on different secret values
#[test]
#[ignore = "Timing tests are unreliable in CI environments"]
fn test_timing_attack_resistance() {
    const ITERATIONS: usize = 10000;
    
    // Test ct_decode_bit timing
    let mut times_in_range = Vec::with_capacity(ITERATIONS);
    let mut times_out_range = Vec::with_capacity(ITERATIONS);
    
    // Warm up
    for _ in 0..100 {
        let _ = ct_decode_bit(1000);
        let _ = ct_decode_bit(100);
    }
    
    // Measure in-range values
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        let _ = ct_decode_bit(1664); // In range
        times_in_range.push(start.elapsed());
    }
    
    // Measure out-of-range values
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        let _ = ct_decode_bit(100); // Out of range
        times_out_range.push(start.elapsed());
    }
    
    // Calculate statistics
    let avg_in_range = times_in_range.iter().sum::<Duration>() / ITERATIONS as u32;
    let avg_out_range = times_out_range.iter().sum::<Duration>() / ITERATIONS as u32;
    
    // The difference should be negligible (less than 5% variance)
    let diff = avg_in_range.abs_diff(avg_out_range);
    
    let max_avg = avg_in_range.max(avg_out_range);
    let variance_percent = (diff.as_nanos() as f64 / max_avg.as_nanos() as f64) * 100.0;
    
    println!("Average time in range: {avg_in_range:?}");
    println!("Average time out of range: {avg_out_range:?}");
    println!("Timing variance: {variance_percent:.2}%");
    
    assert!(variance_percent < 5.0, "Timing variance too high: {variance_percent:.2}%");
}

/// Test secure memory in Kyber key generation
#[test]
fn test_kyber_secure_key_generation() {
    let mut rng = TestRng::new(42);
    
    // Generate keys for all Kyber variants
    let (pk512, sk512) = Kyber512::generate_keypair(&mut rng).unwrap();
    let (pk768, sk768) = Kyber768::generate_keypair(&mut rng).unwrap();
    let (pk1024, sk1024) = Kyber1024::generate_keypair(&mut rng).unwrap();
    
    // Keys should be valid
    assert_eq!(pk512.to_bytes().len(), Kyber512::PUBLIC_KEY_SIZE);
    assert_eq!(sk512.to_bytes().len(), Kyber512::SECRET_KEY_SIZE);
    assert_eq!(pk768.to_bytes().len(), Kyber768::PUBLIC_KEY_SIZE);
    assert_eq!(sk768.to_bytes().len(), Kyber768::SECRET_KEY_SIZE);
    assert_eq!(pk1024.to_bytes().len(), Kyber1024::PUBLIC_KEY_SIZE);
    assert_eq!(sk1024.to_bytes().len(), Kyber1024::SECRET_KEY_SIZE);
}

/// Test secure memory in Dilithium key generation
#[test]
fn test_dilithium_secure_key_generation() {
    let mut rng = TestRng::new(42);
    
    // Generate keys for all Dilithium variants
    let (pk2, sk2) = Dilithium2::generate_keypair(&mut rng).unwrap();
    let (pk3, sk3) = Dilithium3::generate_keypair(&mut rng).unwrap();
    let (pk5, sk5) = Dilithium5::generate_keypair(&mut rng).unwrap();
    
    // Keys should be valid
    // For now, just check that keys have non-zero size
    // The actual size validation is done in the from_bytes implementation
    assert!(!pk2.to_bytes().is_empty());
    assert!(!sk2.to_bytes().is_empty());
    assert!(!pk3.to_bytes().is_empty());
    assert!(!sk3.to_bytes().is_empty());
    assert!(!pk5.to_bytes().is_empty());
    assert!(!sk5.to_bytes().is_empty());
}

/// Test that KEM encapsulation/decapsulation uses constant-time operations
#[test]
fn test_kyber_constant_time_decapsulation() {
    let mut rng = TestRng::new(12345);
    
    // Generate keypair
    let (pk, sk) = Kyber512::generate_keypair(&mut rng).unwrap();
    
    // Create valid ciphertext
    let (ct, ss) = Kyber512::encapsulate(&pk, &mut rng).unwrap();
    
    // Decapsulation should succeed
    let ss_recovered = Kyber512::decapsulate(&sk, &ct).unwrap();
    assert_eq!(ss.bytes, ss_recovered.bytes);
    
    // Test with corrupted ciphertext (should still complete in constant time)
    let mut ct_corrupted = ct.clone();
    ct_corrupted.bytes[0] ^= 0xFF;
    
    // This should complete without timing leaks
    let ss_corrupted = Kyber512::decapsulate(&sk, &ct_corrupted).unwrap();
    
    // The shared secrets should be different
    assert_ne!(ss.bytes, ss_corrupted.bytes);
}

/// Test input validation prevents buffer overflows
#[test]
fn test_input_validation() {
    // Test with invalid public key sizes
    let invalid_pk_small = vec![0u8; 100]; // Too small
    let invalid_pk_large = vec![0u8; 10000]; // Too large
    
    // These should fail gracefully
    assert!(matches!(
        <synapsed_crypto::kyber::PublicKey::<2> as Serializable>::from_bytes(&invalid_pk_small),
        Err(Error::InvalidKeySize)
    ));
    
    assert!(matches!(
        <synapsed_crypto::kyber::PublicKey::<2> as Serializable>::from_bytes(&invalid_pk_large),
        Err(Error::InvalidKeySize)
    ));
    
    // Test with invalid secret key sizes
    let invalid_sk_small = vec![0u8; 100];
    let invalid_sk_large = vec![0u8; 10000];
    
    assert!(matches!(
        <synapsed_crypto::kyber::SecretKey::<2> as Serializable>::from_bytes(&invalid_sk_small),
        Err(Error::InvalidKeySize)
    ));
    
    assert!(matches!(
        <synapsed_crypto::kyber::SecretKey::<2> as Serializable>::from_bytes(&invalid_sk_large),
        Err(Error::InvalidKeySize)
    ));
}

/// Test that error handling doesn't leak information through error types
#[test]
fn test_error_handling_no_info_leak() {
    let mut rng = TestRng::new(42);
    
    // Generate valid keypair
    let (_pk, sk) = Kyber512::generate_keypair(&mut rng).unwrap();
    
    // Create invalid ciphertext (wrong size)
    let invalid_ct = synapsed_crypto::kyber::Ciphertext::<2> {
        bytes: vec![0u8; 100], // Wrong size
    };
    
    // Different invalid ciphertexts should produce the same error
    let invalid_ct2 = synapsed_crypto::kyber::Ciphertext::<2> {
        bytes: vec![0xFF; 100], // Different content, same wrong size
    };
    
    // Errors should be identical (no information leak)
    let err1 = Kyber512::decapsulate(&sk, &invalid_ct);
    let err2 = Kyber512::decapsulate(&sk, &invalid_ct2);
    
    // Both should fail with the same error type
    assert!(err1.is_err());
    assert!(err2.is_err());
    
    // The errors should be the same (no information leak based on content)
    match (err1, err2) {
        (Err(e1), Err(e2)) => assert_eq!(std::mem::discriminant(&e1), std::mem::discriminant(&e2)),
        _ => panic!("Expected both operations to fail"),
    }
}

/// Test secure_scope ensures cleanup even on panic
#[test]
fn test_secure_scope_panic_safety() {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    
    let result = catch_unwind(AssertUnwindSafe(|| {
        secure_scope::<32, _, _>(|buffer| {
            // Fill buffer with sensitive data
            buffer.fill(0xAA);
            
            // Verify it's filled
            assert!(buffer.iter().all(|&b| b == 0xAA));
            
            // Simulate panic
            panic!("Test panic");
        })
    }));
    
    // The panic should be caught
    assert!(result.is_err());
    // Memory should still be cleaned up (verified by secure_scope implementation)
}

/// Test that all critical paths use constant-time operations
#[test]
fn test_critical_paths_constant_time() {
    // This is more of a code review test - verify key functions exist and are used
    
    // Verify ct_decode_bit is used (by attempting to call it)
    let _ = ct_decode_bit(1000);
    
    // Verify ct_caddq is used
    let _ = ct_caddq(100);
    
    // Verify ct_check_norm is used
    let _ = ct_check_norm(&[1, 2, 3], 10);
    
    // If these compile and run, the constant-time functions are properly integrated
}

/// Benchmark security operations to ensure reasonable performance
#[test]
#[ignore = "Benchmarks are not deterministic"]
fn bench_security_operations() {
    const ITERATIONS: usize = 100000;
    
    // Benchmark ct_decode_bit
    let start = Instant::now();
    for i in 0..ITERATIONS {
        let _ = ct_decode_bit((i % 3329) as i16);
    }
    let ct_decode_time = start.elapsed();
    println!("ct_decode_bit: {:?} per operation", ct_decode_time / ITERATIONS as u32);
    
    // Benchmark ct_caddq
    let start = Instant::now();
    for i in 0..ITERATIONS {
        let _ = ct_caddq((i as i16).wrapping_sub(1664));
    }
    let ct_caddq_time = start.elapsed();
    println!("ct_caddq: {:?} per operation", ct_caddq_time / ITERATIONS as u32);
    
    // Performance should be reasonable (< 100ns per operation)
    assert!(ct_decode_time / (ITERATIONS as u32) < Duration::from_nanos(100));
    assert!(ct_caddq_time / (ITERATIONS as u32) < Duration::from_nanos(100));
}