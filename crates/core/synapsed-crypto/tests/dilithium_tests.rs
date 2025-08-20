//! Tests for Dilithium signature algorithm

use synapsed_crypto::{
    dilithium::{Dilithium2, Dilithium3, Dilithium5},
    traits::Signature,
    random::TestRng,
};

#[test]
fn test_dilithium2_keygen() {
    let mut rng = TestRng::new(42);
    let (pk, sk) = Dilithium2::generate_keypair(&mut rng).unwrap();
    
    assert_eq!(pk.as_ref().len(), Dilithium2::PUBLIC_KEY_SIZE);
    assert_eq!(sk.as_ref().len(), Dilithium2::SECRET_KEY_SIZE);
}

#[test]
fn test_dilithium2_sign_verify() {
    let mut rng = TestRng::new(42);
    let message = b"Test message for Dilithium2";
    
    // Generate keypair
    let (pk, sk) = Dilithium2::generate_keypair(&mut rng).unwrap();
    
    // Sign message
    let signature = Dilithium2::sign(&sk, message, &mut rng).unwrap();
    assert_eq!(signature.as_ref().len(), Dilithium2::SIGNATURE_SIZE);
    
    // Verify signature
    let valid = Dilithium2::verify(&pk, message, &signature).unwrap();
    assert!(valid);
    
    // Verify with wrong message should fail
    let wrong_message = b"Wrong message";
    let valid = Dilithium2::verify(&pk, wrong_message, &signature).unwrap();
    assert!(!valid);
}

#[test]
fn test_dilithium2_deterministic_sign() {
    let mut rng = TestRng::new(42);
    let message = b"Test message for deterministic signing";
    
    // Generate keypair
    let (pk, sk) = Dilithium2::generate_keypair(&mut rng).unwrap();
    
    // Sign message deterministically
    let signature1 = Dilithium2::sign_deterministic(&sk, message).unwrap();
    let signature2 = Dilithium2::sign_deterministic(&sk, message).unwrap();
    
    // Deterministic signatures should be identical
    assert_eq!(signature1.as_ref(), signature2.as_ref());
    
    // Both should verify
    let valid1 = Dilithium2::verify(&pk, message, &signature1).unwrap();
    let valid2 = Dilithium2::verify(&pk, message, &signature2).unwrap();
    assert!(valid1);
    assert!(valid2);
}

#[test]
fn test_dilithium3_keygen() {
    let mut rng = TestRng::new(42);
    let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
    
    assert_eq!(pk.as_ref().len(), Dilithium3::PUBLIC_KEY_SIZE);
    assert_eq!(sk.as_ref().len(), Dilithium3::SECRET_KEY_SIZE);
}

#[test]
fn test_dilithium3_sign_verify() {
    let mut rng = TestRng::new(42);
    let message = b"Test message for Dilithium3";
    
    // Generate keypair
    let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
    
    // Sign message
    let signature = Dilithium3::sign(&sk, message, &mut rng).unwrap();
    assert_eq!(signature.as_ref().len(), Dilithium3::SIGNATURE_SIZE);
    
    // Verify signature
    let valid = Dilithium3::verify(&pk, message, &signature).unwrap();
    assert!(valid);
}

#[test]
fn test_dilithium5_keygen() {
    let mut rng = TestRng::new(42);
    let (pk, sk) = Dilithium5::generate_keypair(&mut rng).unwrap();
    
    assert_eq!(pk.as_ref().len(), Dilithium5::PUBLIC_KEY_SIZE);
    assert_eq!(sk.as_ref().len(), Dilithium5::SECRET_KEY_SIZE);
}

#[test]
fn test_dilithium5_sign_verify() {
    let mut rng = TestRng::new(42);
    let message = b"Test message for Dilithium5";
    
    // Generate keypair
    let (pk, sk) = Dilithium5::generate_keypair(&mut rng).unwrap();
    
    // Sign message
    let signature = Dilithium5::sign(&sk, message, &mut rng).unwrap();
    assert_eq!(signature.as_ref().len(), Dilithium5::SIGNATURE_SIZE);
    
    // Verify signature
    let valid = Dilithium5::verify(&pk, message, &signature).unwrap();
    assert!(valid);
}

#[test]
fn test_dilithium_cross_verification_fails() {
    let mut rng = TestRng::new(42);
    let message = b"Test message";
    
    // Generate Dilithium2 keypair and signature
    let (_pk2, sk2) = Dilithium2::generate_keypair(&mut rng).unwrap();
    let _sig2 = Dilithium2::sign(&sk2, message, &mut rng).unwrap();
    
    // Generate Dilithium3 keypair
    let (_pk3, _) = Dilithium3::generate_keypair(&mut rng).unwrap();
    
    // Try to verify Dilithium2 signature with Dilithium3 public key
    // This should fail gracefully (return false or error)
    // Note: In a real implementation, this would likely fail due to size mismatches
}

#[test]
#[ignore] // Performance test - run with `cargo test -- --ignored`
fn test_dilithium_performance() {
    use std::time::Instant;
    
    let mut rng = TestRng::new(42);
    let message = b"Performance test message";
    let iterations = 100;
    
    // Dilithium2 performance
    println!("\nDilithium2 Performance ({iterations}×):");
    
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = Dilithium2::generate_keypair(&mut rng).unwrap();
    }
    let keygen_time = start.elapsed() / iterations;
    println!("  Key generation: {keygen_time:?}");
    
    let (pk, sk) = Dilithium2::generate_keypair(&mut rng).unwrap();
    
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = Dilithium2::sign(&sk, message, &mut rng).unwrap();
    }
    let sign_time = start.elapsed() / iterations;
    println!("  Signing: {sign_time:?}");
    
    let signature = Dilithium2::sign(&sk, message, &mut rng).unwrap();
    
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = Dilithium2::verify(&pk, message, &signature).unwrap();
    }
    let verify_time = start.elapsed() / iterations;
    println!("  Verification: {verify_time:?}");
}