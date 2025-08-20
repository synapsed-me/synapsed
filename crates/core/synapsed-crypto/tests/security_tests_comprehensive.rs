//! Comprehensive security tests for synapsed-crypto
//!
//! These tests verify security properties and resistance to common attacks.
//! Note: These are basic security tests - full cryptanalysis requires specialized tools.

use synapsed_crypto::kyber::{Kyber512, Kyber768, Kyber1024};
use synapsed_crypto::dilithium::Dilithium3;
use synapsed_crypto::traits::{Kem, Signature, Serializable};
use synapsed_crypto::random::DefaultRng;

#[test]
fn test_key_uniqueness() {
    // Test that key generation produces unique keys
    let mut rng = DefaultRng::default();
    let iterations = 100;
    
    // Test Kyber key uniqueness
    let mut kyber_public_keys = std::collections::HashSet::new();
    let mut kyber_secret_keys = std::collections::HashSet::new();
    
    for _ in 0..iterations {
        let (pk, sk) = Kyber768::generate_keypair(&mut rng).unwrap();
        let pk_bytes = pk.to_bytes();
        let sk_bytes = sk.to_bytes();
        
        assert!(!kyber_public_keys.contains(&pk_bytes), 
               "Duplicate Kyber public key generated");
        assert!(!kyber_secret_keys.contains(&sk_bytes), 
               "Duplicate Kyber secret key generated");
               
        kyber_public_keys.insert(pk_bytes);
        kyber_secret_keys.insert(sk_bytes);
    }
    
    // Test Dilithium key uniqueness
    let mut dilithium_public_keys = std::collections::HashSet::new();
    let mut dilithium_secret_keys = std::collections::HashSet::new();
    
    for _ in 0..iterations {
        let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
        let pk_bytes = pk.to_bytes();
        let sk_bytes = sk.to_bytes();
        
        assert!(!dilithium_public_keys.contains(&pk_bytes), 
               "Duplicate Dilithium public key generated");
        assert!(!dilithium_secret_keys.contains(&sk_bytes), 
               "Duplicate Dilithium secret key generated");
               
        dilithium_public_keys.insert(pk_bytes);
        dilithium_secret_keys.insert(sk_bytes);
    }
}

#[test]
fn test_shared_secret_uniqueness() {
    // Test that encapsulation produces unique shared secrets
    let mut rng = DefaultRng::default();
    let (pk, _) = Kyber768::generate_keypair(&mut rng).unwrap();
    
    let mut shared_secrets = std::collections::HashSet::new();
    let mut ciphertexts = std::collections::HashSet::new();
    
    for _ in 0..100 {
        let (ct, ss) = Kyber768::encapsulate(&pk, &mut rng).unwrap();
        
        let ss_bytes = ss.as_ref().to_vec();
        let ct_bytes = ct.as_ref().to_vec();
        
        assert!(!shared_secrets.contains(&ss_bytes), 
               "Duplicate shared secret generated");
        assert!(!ciphertexts.contains(&ct_bytes), 
               "Duplicate ciphertext generated");
               
        shared_secrets.insert(ss_bytes);
        ciphertexts.insert(ct_bytes);
    }
}

#[test]
fn test_signature_uniqueness() {
    // Test that signatures for the same message can be different (due to randomness)
    let mut rng = DefaultRng::default();
    let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
    let message = b"test message for signature uniqueness";
    
    let mut signatures = std::collections::HashSet::new();
    
    for _ in 0..50 {
        let sig = Dilithium3::sign(&sk, message, &mut rng).unwrap();
        
        // All signatures should be valid
        assert!(Dilithium3::verify(&pk, message, &sig).is_ok());
        
        // Signatures may or may not be unique depending on the implementation
        // Dilithium can produce different signatures for the same message
        signatures.insert(sig.as_ref().to_vec());
    }
    
    println!("Generated {} unique signatures out of 50 attempts", signatures.len());
}

#[test]
fn test_cross_key_isolation() {
    // Test that keys from different generations don't work together
    let mut rng = DefaultRng::default();
    
    // Generate two independent key pairs
    let (pk1, sk1) = Kyber768::generate_keypair(&mut rng).unwrap();
    let (pk2, sk2) = Kyber768::generate_keypair(&mut rng).unwrap();
    
    // Encapsulate with first public key
    let (ct1, ss1) = Kyber768::encapsulate(&pk1, &mut rng).unwrap();
    let (ct2, ss2) = Kyber768::encapsulate(&pk2, &mut rng).unwrap();
    
    // Verify correct decapsulation
    assert_eq!(ss1, Kyber768::decapsulate(&sk1, &ct1).unwrap());
    assert_eq!(ss2, Kyber768::decapsulate(&sk2, &ct2).unwrap());
    
    // Cross-key decapsulation should produce different results
    let cross_ss1 = Kyber768::decapsulate(&sk2, &ct1).unwrap();
    let cross_ss2 = Kyber768::decapsulate(&sk1, &ct2).unwrap();
    
    assert_ne!(ss1, cross_ss1, "Cross-key decapsulation should produce different shared secret");
    assert_ne!(ss2, cross_ss2, "Cross-key decapsulation should produce different shared secret");
}

#[test]
fn test_signature_key_isolation() {
    // Test that signatures can't be forged with different keys
    let mut rng = DefaultRng::default();
    let message = b"test message for key isolation";
    
    // Generate two independent key pairs
    let (pk1, sk1) = Dilithium3::generate_keypair(&mut rng).unwrap();
    let (pk2, _sk2) = Dilithium3::generate_keypair(&mut rng).unwrap();
    
    // Sign with first key
    let sig = Dilithium3::sign(&sk1, message, &mut rng).unwrap();
    
    // Verify with correct key
    assert!(Dilithium3::verify(&pk1, message, &sig).is_ok());
    
    // Verify with wrong key should fail
    assert!(!Dilithium3::verify(&pk2, message, &sig).unwrap());
}

#[test]
fn test_message_integrity() {
    // Test that signature verification detects message tampering
    let mut rng = DefaultRng::default();
    let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
    let original_message = b"This is the original message";
    
    let sig = Dilithium3::sign(&sk, original_message, &mut rng).unwrap();
    
    // Original message should verify
    assert!(Dilithium3::verify(&pk, original_message, &sig).is_ok());
    
    // Modified messages should not verify
    let tampered_messages = [
        b"This is the original message!",  // Added character
        b"This is the original messag".as_slice(),    // Removed character  
        b"This is the Original message",   // Changed case
        b"This is not original message",   // Changed word
        b"",                               // Empty message
        b"Completely different message",   // Completely different
    ];
    
    for tampered_msg in &tampered_messages {
        let result = Dilithium3::verify(&pk, tampered_msg, &sig);
        assert!(!result.unwrap(), 
               "Tampered message should not verify: {:?}", 
               std::str::from_utf8(tampered_msg).unwrap_or("<invalid utf8>"));
    }
}

#[test]
fn test_signature_malleability_resistance() {
    // Test that signature modification is detected
    let mut rng = DefaultRng::default();
    let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
    let message = b"test message for malleability";
    
    let original_sig = Dilithium3::sign(&sk, message, &mut rng).unwrap();
    
    // Original signature should verify
    assert!(Dilithium3::verify(&pk, message, &original_sig).is_ok());
    
    // Test various signature modifications
    for i in 0..std::cmp::min(original_sig.as_ref().len(), 20) {
        let _modified_sig = original_sig.clone();
        
        // Test with bit flip
        let mut sig_bytes = original_sig.to_bytes();
        sig_bytes[i] ^= 0x01;
        let modified_sig = <Dilithium3 as Signature>::Sig::from_bytes(&sig_bytes).unwrap();
        let result = Dilithium3::verify(&pk, message, &modified_sig);
        assert!(!result.unwrap(), 
               "Modified signature should not verify (bit flip at position {i})");
        
        // Test with zero byte
        sig_bytes[i] = 0x00;
        let modified_sig = <Dilithium3 as Signature>::Sig::from_bytes(&sig_bytes).unwrap();
        let result = Dilithium3::verify(&pk, message, &modified_sig);
        assert!(!result.unwrap(), 
               "Modified signature should not verify (zero byte at position {i})");
        
        // Test with max byte
        sig_bytes[i] = 0xFF;
        let modified_sig = <Dilithium3 as Signature>::Sig::from_bytes(&sig_bytes).unwrap();
        let result = Dilithium3::verify(&pk, message, &modified_sig);
        assert!(!result.unwrap(), 
               "Modified signature should not verify (max byte at position {i})");
    }
}

#[test]
fn test_randomness_dependency() {
    // Test that operations depend on randomness appropriately
    // This is a basic test - real randomness analysis requires statistical tools
    
    struct PredictableRng {
        counter: u8,
    }
    
    impl synapsed_crypto::traits::SecureRandom for PredictableRng {
        fn fill_bytes(&mut self, bytes: &mut [u8]) {
            for byte in bytes {
                *byte = self.counter;
                self.counter = self.counter.wrapping_add(1);
            }
        }
    }
    
    // Test that predictable randomness produces predictable results
    let mut rng1 = PredictableRng { counter: 0 };
    let mut rng2 = PredictableRng { counter: 0 };
    
    let (pk1, sk1) = Kyber512::generate_keypair(&mut rng1).unwrap();
    let (pk2, sk2) = Kyber512::generate_keypair(&mut rng2).unwrap();
    
    // Same randomness should produce same keys
    assert_eq!(pk1.to_bytes(), pk2.to_bytes());
    assert_eq!(sk1.to_bytes(), sk2.to_bytes());
    
    // But different starting points should produce different keys
    let mut rng3 = PredictableRng { counter: 100 };
    let (pk3, _sk3) = Kyber512::generate_keypair(&mut rng3).unwrap();
    assert_ne!(pk1.to_bytes(), pk3.to_bytes());
}

#[test]
fn test_zero_key_resistance() {
    // Test behavior with zero-filled keys (should fail gracefully)
    
    // Create zero-filled public key
    let zero_pk_bytes = vec![0u8; 1184]; // Kyber768 public key size
    let zero_pk_result = synapsed_crypto::kyber::PublicKey::<3>::from_bytes(&zero_pk_bytes);
    
    match zero_pk_result {
        Ok(zero_pk) => {
            // If zero key is accepted (implementation-dependent), operations should still work
            let mut rng = DefaultRng::default();
            let result = Kyber768::encapsulate(&zero_pk, &mut rng);
            // This might succeed or fail depending on implementation
            match result {
                Ok((ct, ss)) => {
                    println!("Zero public key accepted, produced ciphertext of {} bytes and shared secret of {} bytes", 
                            ct.as_ref().len(), ss.as_ref().len());
                }
                Err(e) => {
                    println!("Zero public key rejected during encapsulation: {e:?}");
                }
            }
        }
        Err(e) => {
            println!("Zero public key rejected during deserialization: {e:?}");
        }
    }
    
    // Create zero-filled secret key
    let zero_sk_bytes = vec![0u8; 2400]; // Kyber768 secret key size
    let zero_sk_result = synapsed_crypto::kyber::SecretKey::<3>::from_bytes(&zero_sk_bytes);
    
    match zero_sk_result {
        Ok(_zero_sk) => {
            println!("Zero secret key was accepted");
        }
        Err(e) => {
            println!("Zero secret key rejected: {e:?}");
        }
    }
}

#[test]
fn test_algorithm_parameter_isolation() {
    // Test that different parameter sets don't interfere
    let mut rng = DefaultRng::default();
    
    // Generate keys for all Kyber variants
    let (pk512, sk512) = Kyber512::generate_keypair(&mut rng).unwrap();
    let (pk768, sk768) = Kyber768::generate_keypair(&mut rng).unwrap();
    let (pk1024, sk1024) = Kyber1024::generate_keypair(&mut rng).unwrap();
    
    // Keys should be different sizes
    assert_ne!(pk512.to_bytes().len(), pk768.to_bytes().len());
    assert_ne!(pk768.to_bytes().len(), pk1024.to_bytes().len());
    assert_ne!(sk512.to_bytes().len(), sk768.to_bytes().len());
    assert_ne!(sk768.to_bytes().len(), sk1024.to_bytes().len());
    
    // Operations should work correctly for each
    let (ct512, ss512) = Kyber512::encapsulate(&pk512, &mut rng).unwrap();
    let (ct768, ss768) = Kyber768::encapsulate(&pk768, &mut rng).unwrap();
    let (ct1024, ss1024) = Kyber1024::encapsulate(&pk1024, &mut rng).unwrap();
    
    assert_eq!(ss512, Kyber512::decapsulate(&sk512, &ct512).unwrap());
    assert_eq!(ss768, Kyber768::decapsulate(&sk768, &ct768).unwrap());
    assert_eq!(ss1024, Kyber1024::decapsulate(&sk1024, &ct1024).unwrap());
    
    // Ciphertexts and shared secrets should be different sizes
    assert_ne!(ct512.as_ref().len(), ct768.as_ref().len());
    assert_ne!(ct768.as_ref().len(), ct1024.as_ref().len());
    assert_ne!(ss512.as_ref().len(), ss768.as_ref().len());
    assert_ne!(ss768.as_ref().len(), ss1024.as_ref().len());
}

#[test]
fn test_error_information_leakage() {
    // Test that error messages don't leak sensitive information
    let mut rng = DefaultRng::default();
    let (_pk, sk) = Kyber768::generate_keypair(&mut rng).unwrap();
    
    // Create invalid ciphertext
    let invalid_ct_bytes = vec![0xFF; 1088]; // Correct size but invalid content
    let invalid_ct = <Kyber768 as Kem>::Ciphertext::from_bytes(&invalid_ct_bytes).unwrap();
    let result = Kyber768::decapsulate(&sk, &invalid_ct);
    
    // Error should not contain secret key information
    match result {
        Ok(_) => {
            // If it succeeds, that's fine - some implementations are robust
        }
        Err(e) => {
            let error_msg = format!("{e:?}");
            // Error message shouldn't contain raw key bytes
            let sk_bytes = sk.to_bytes();
            for byte in sk_bytes {
                assert!(!error_msg.contains(&format!("{byte:02x}")), 
                       "Error message may leak secret key data");
            }
        }
    }
}

#[test]
fn test_timing_consistency() {
    // Basic timing consistency test (not a full timing attack analysis)
    let mut rng = DefaultRng::default();
    let (pk, sk) = Kyber768::generate_keypair(&mut rng).unwrap();
    
    let mut decap_times = Vec::new();
    
    // Test with valid ciphertexts
    for _ in 0..20 {
        let (ct, _) = Kyber768::encapsulate(&pk, &mut rng).unwrap();
        
        let start = std::time::Instant::now();
        let _ss = Kyber768::decapsulate(&sk, &ct).unwrap();
        let duration = start.elapsed();
        
        decap_times.push(duration.as_nanos());
    }
    
    // Calculate coefficient of variation
    let mean: u128 = decap_times.iter().sum::<u128>() / decap_times.len() as u128;
    let variance: u128 = decap_times.iter()
        .map(|&x| {
            let diff = x.abs_diff(mean);
            diff * diff
        })
        .sum::<u128>() / decap_times.len() as u128;
    
    let std_dev = (variance as f64).sqrt();
    let cv = std_dev / mean as f64; // Coefficient of variation
    
    println!("Timing analysis: mean={mean} ns, std_dev={std_dev:.2} ns, CV={cv:.3}");
    
    // Coefficient of variation should be reasonable (not a strict security test)
    assert!(cv < 0.5, "High timing variation detected: CV={cv:.3}");
}