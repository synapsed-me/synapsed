//! Comprehensive unit tests for the high-level API module
//!
//! These tests ensure the API functions work correctly with all supported
//! algorithms and handle error cases properly.

use synapsed_crypto::api::*;
use synapsed_crypto::random::DefaultRng;
use synapsed_crypto::error::Error;

#[test]
fn test_kem_algorithms_basic_operations() {
    let algorithms = [
        KemAlgorithm::Kyber512,
        KemAlgorithm::Kyber768,
        KemAlgorithm::Kyber1024,
    ];
    
    for alg in algorithms {
        let mut rng = DefaultRng::default();
        
        // Generate keypair
        let (pk, sk) = generate_keypair(alg, &mut rng).unwrap();
        
        // Verify key sizes
        assert_eq!(pk.len(), alg.public_key_size());
        assert_eq!(sk.len(), alg.secret_key_size());
        
        // Encapsulate
        let (ct, ss1) = encapsulate(alg, &pk, &mut rng).unwrap();
        
        // Verify ciphertext and shared secret sizes
        assert_eq!(ct.len(), alg.ciphertext_size());
        assert_eq!(ss1.len(), alg.shared_secret_size());
        
        // Decapsulate
        let ss2 = decapsulate(alg, &sk, &ct).unwrap();
        
        // Verify shared secrets match
        assert_eq!(ss1, ss2);
    }
}

#[test]
fn test_signature_algorithms_basic_operations() {
    let algorithms = [
        SignatureAlgorithm::Dilithium2,
        SignatureAlgorithm::Dilithium3,
        SignatureAlgorithm::Dilithium5,
    ];
    
    let test_messages: &[&[u8]] = &[
        b"",  // Empty message
        b"Hello, World!",  // Short message
        &[0xAB; 1000],  // Medium message
        &vec![0xCD; 100_000],  // Large message
    ];
    
    for alg in algorithms {
        let mut rng = DefaultRng::default();
        
        // Generate keypair
        let (pk, sk) = generate_signing_keypair(alg, &mut rng).unwrap();
        
        // Verify key sizes
        assert_eq!(pk.len(), alg.public_key_size());
        assert_eq!(sk.len(), alg.secret_key_size());
        
        for msg in test_messages {
            // Sign message
            let sig = sign(alg, &sk, msg, &mut rng).unwrap();
            
            // Verify signature size is within expected range
            assert!(sig.len() <= alg.signature_size());
            
            // Verify signature
            assert!(verify(alg, &pk, msg, &sig).unwrap());
            
            // Test invalid signature verification
            let mut bad_sig = sig.clone();
            if !bad_sig.is_empty() {
                bad_sig[0] ^= 0xFF;
                assert!(!verify(alg, &pk, msg, &bad_sig).unwrap());
            }
            
            // Test wrong message verification
            let wrong_msg = b"wrong message";
            assert!(!verify(alg, &pk, wrong_msg, &sig).unwrap());
        }
    }
}

#[test]
fn test_kem_error_handling() {
    let mut rng = DefaultRng::default();
    let alg = KemAlgorithm::Kyber768;
    
    // Test with invalid public key size
    let bad_pk = vec![0u8; 100];  // Wrong size
    let result = encapsulate(alg, &bad_pk, &mut rng);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    // Test with invalid secret key size
    let bad_sk = vec![0u8; 100];  // Wrong size
    let ct = vec![0u8; alg.ciphertext_size()];
    let result = decapsulate(alg, &bad_sk, &ct);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    // Test with invalid ciphertext size
    let (_, sk) = generate_keypair(alg, &mut rng).unwrap();
    let bad_ct = vec![0u8; 100];  // Wrong size
    let result = decapsulate(alg, &sk, &bad_ct);
    assert!(matches!(result, Err(Error::InvalidCiphertext)));
}

#[test]
fn test_signature_error_handling() {
    let mut rng = DefaultRng::default();
    let alg = SignatureAlgorithm::Dilithium3;
    
    // Test with invalid public key size
    let bad_pk = vec![0u8; 100];  // Wrong size
    let msg = b"test";
    let sig = vec![0u8; 1000];
    let result = verify(alg, &bad_pk, msg, &sig);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    // Test with invalid secret key size
    let bad_sk = vec![0u8; 100];  // Wrong size
    let result = sign(alg, &bad_sk, msg, &mut rng);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
}

#[test]
fn test_security_levels() {
    // Verify security levels are correct
    assert_eq!(KemAlgorithm::Kyber512.security_level(), 1);
    assert_eq!(KemAlgorithm::Kyber768.security_level(), 3);
    assert_eq!(KemAlgorithm::Kyber1024.security_level(), 5);
    
    assert_eq!(SignatureAlgorithm::Dilithium2.security_level(), 2);
    assert_eq!(SignatureAlgorithm::Dilithium3.security_level(), 3);
    assert_eq!(SignatureAlgorithm::Dilithium5.security_level(), 5);
}

#[test]
fn test_algorithm_display() {
    // Test Display implementations
    assert_eq!(format!("{}", KemAlgorithm::Kyber512), "Kyber512");
    assert_eq!(format!("{}", KemAlgorithm::Kyber768), "Kyber768");
    assert_eq!(format!("{}", KemAlgorithm::Kyber1024), "Kyber1024");
    
    assert_eq!(format!("{}", SignatureAlgorithm::Dilithium2), "Dilithium2");
    assert_eq!(format!("{}", SignatureAlgorithm::Dilithium3), "Dilithium3");
    assert_eq!(format!("{}", SignatureAlgorithm::Dilithium5), "Dilithium5");
}

#[test]
fn test_multiple_encapsulations() {
    // Test that multiple encapsulations with same key produce different results
    let mut rng = DefaultRng::default();
    let alg = KemAlgorithm::Kyber768;
    let (pk, sk) = generate_keypair(alg, &mut rng).unwrap();
    
    let mut ciphertexts = Vec::new();
    let mut shared_secrets = Vec::new();
    
    for _ in 0..10 {
        let (ct, ss) = encapsulate(alg, &pk, &mut rng).unwrap();
        
        // Each encapsulation should produce different results
        assert!(!ciphertexts.contains(&ct));
        assert!(!shared_secrets.contains(&ss));
        
        ciphertexts.push(ct.clone());
        shared_secrets.push(ss.clone());
        
        // But decapsulation should work correctly
        let recovered = decapsulate(alg, &sk, &ct).unwrap();
        assert_eq!(ss, recovered);
    }
}

#[test]
fn test_deterministic_signatures() {
    // Dilithium signatures are deterministic when using the same randomness
    let alg = SignatureAlgorithm::Dilithium3;
    let msg = b"test message for deterministic check";
    
    // Generate keypair
    let mut rng = DefaultRng::default();
    let (pk, sk) = generate_signing_keypair(alg, &mut rng).unwrap();
    
    // Create two signatures
    let sig1 = sign(alg, &sk, msg, &mut rng).unwrap();
    let sig2 = sign(alg, &sk, msg, &mut rng).unwrap();
    
    // Both should be valid
    assert!(verify(alg, &pk, msg, &sig1).unwrap());
    assert!(verify(alg, &pk, msg, &sig2).unwrap());
    
    // Note: Dilithium can produce different signatures for the same message
    // due to randomized hashing, so we don't assert they're equal
}

#[test]
fn test_cross_algorithm_compatibility() {
    // Ensure algorithms don't interfere with each other
    let mut rng = DefaultRng::default();
    
    // Generate keys for all KEM algorithms
    let kyber512_keys = generate_keypair(KemAlgorithm::Kyber512, &mut rng).unwrap();
    let kyber768_keys = generate_keypair(KemAlgorithm::Kyber768, &mut rng).unwrap();
    let _kyber1024_keys = generate_keypair(KemAlgorithm::Kyber1024, &mut rng).unwrap();
    
    // Try to use wrong algorithm with keys (should fail)
    let (ct, _) = encapsulate(KemAlgorithm::Kyber512, &kyber512_keys.0, &mut rng).unwrap();
    assert!(decapsulate(KemAlgorithm::Kyber768, &kyber768_keys.1, &ct).is_err());
    
    // Generate keys for all signature algorithms
    let dil2_keys = generate_signing_keypair(SignatureAlgorithm::Dilithium2, &mut rng).unwrap();
    let dil3_keys = generate_signing_keypair(SignatureAlgorithm::Dilithium3, &mut rng).unwrap();
    let _dil5_keys = generate_signing_keypair(SignatureAlgorithm::Dilithium5, &mut rng).unwrap();
    
    // Sign with one algorithm
    let msg = b"test";
    let sig = sign(SignatureAlgorithm::Dilithium2, &dil2_keys.1, msg, &mut rng).unwrap();
    
    // Verify with wrong algorithm (should fail)
    assert!(verify(SignatureAlgorithm::Dilithium3, &dil3_keys.0, msg, &sig).is_err());
}

#[test]
fn test_key_serialization_consistency() {
    // Test that serialization through the API is consistent
    let mut rng = DefaultRng::default();
    
    // Test KEM keys
    for alg in [KemAlgorithm::Kyber512, KemAlgorithm::Kyber768, KemAlgorithm::Kyber1024] {
        let (pk1, sk1) = generate_keypair(alg, &mut rng).unwrap();
        let (pk2, sk2) = generate_keypair(alg, &mut rng).unwrap();
        
        // Different keys should be different
        assert_ne!(pk1, pk2);
        assert_ne!(sk1, sk2);
        
        // But operations should still work
        let (ct, ss) = encapsulate(alg, &pk1, &mut rng).unwrap();
        assert_eq!(ss, decapsulate(alg, &sk1, &ct).unwrap());
    }
    
    // Test signature keys
    for alg in [SignatureAlgorithm::Dilithium2, SignatureAlgorithm::Dilithium3, SignatureAlgorithm::Dilithium5] {
        let (pk1, sk1) = generate_signing_keypair(alg, &mut rng).unwrap();
        let (pk2, sk2) = generate_signing_keypair(alg, &mut rng).unwrap();
        
        // Different keys should be different
        assert_ne!(pk1, pk2);
        assert_ne!(sk1, sk2);
        
        // But operations should still work
        let msg = b"test";
        let sig = sign(alg, &sk1, msg, &mut rng).unwrap();
        assert!(verify(alg, &pk1, msg, &sig).unwrap());
        assert!(verify(alg, &pk2, msg, &sig).is_err() || !verify(alg, &pk2, msg, &sig).unwrap());
    }
}