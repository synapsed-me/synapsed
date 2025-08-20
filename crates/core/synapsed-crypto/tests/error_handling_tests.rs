//! Comprehensive error handling tests for synapsed-crypto
//!
//! These tests ensure all error conditions are properly handled
//! and appropriate error types are returned.

use synapsed_crypto::error::{Error, Result};
use synapsed_crypto::kyber::Kyber768;
use synapsed_crypto::dilithium::Dilithium3;
use synapsed_crypto::traits::{Kem, Signature, Serializable};
use synapsed_crypto::random::DefaultRng;

#[test]
fn test_error_display_and_debug() {
    let errors = [
        Error::InvalidKeySize,
        Error::InvalidCiphertext,
        Error::InvalidSignature,
        Error::InvalidParameter,
        Error::InvalidInput,
        Error::InvalidEncoding,
        Error::RandomnessError,
        Error::PolynomialError,
        Error::NttError,
        Error::HashError,
        Error::SerializationError,
        Error::CryptoError,
        Error::UnsupportedCompression,
    ];
    
    for error in errors {
        // Test Display implementation
        let display_str = format!("{error}");
        assert!(!display_str.is_empty());
        
        // Test Debug implementation
        let debug_str = format!("{error:?}");
        assert!(!debug_str.is_empty());
    }
}

#[test]
fn test_error_equality() {
    // Test PartialEq implementation
    assert_eq!(Error::InvalidKeySize, Error::InvalidKeySize);
    assert_ne!(Error::InvalidKeySize, Error::InvalidCiphertext);
    
    // Test that different errors are not equal
    assert_ne!(Error::InvalidCiphertext, Error::InvalidSignature);
}

#[test]
fn test_kyber_invalid_key_sizes() {
    let mut rng = DefaultRng::default();
    
    // Generate valid keys for reference
    let (valid_pk, valid_sk) = Kyber768::generate_keypair(&mut rng).unwrap();
    
    // Test invalid public key deserialization
    let invalid_pk_bytes = vec![0u8; 100]; // Wrong size
    let result = synapsed_crypto::kyber::PublicKey::<3>::from_bytes(&invalid_pk_bytes);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    // Test invalid secret key deserialization
    let invalid_sk_bytes = vec![0u8; 100]; // Wrong size
    let result = synapsed_crypto::kyber::SecretKey::<3>::from_bytes(&invalid_sk_bytes);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    // Test decapsulation with invalid ciphertext (Vec instead of proper type)
    let invalid_ct_bytes = vec![0u8; 100]; // Wrong size
    let invalid_ct = synapsed_crypto::kyber::Ciphertext::<3>::from_bytes(&invalid_ct_bytes);
    assert!(matches!(invalid_ct, Err(Error::InvalidKeySize))); // Size validation happens at construction
    
    // Test decapsulation with corrupted ciphertext (correct size but invalid content)
    let (valid_ct, _) = Kyber768::encapsulate(&valid_pk, &mut rng).unwrap();
    let mut corrupted_ct_bytes = valid_ct.to_bytes().to_vec();
    // Corrupt all bytes
    corrupted_ct_bytes.fill(0xFF);
    
    // Try to create corrupted ciphertext and decapsulate
    if let Ok(corrupted_ct) = synapsed_crypto::kyber::Ciphertext::<3>::from_bytes(&corrupted_ct_bytes) {
        let result = Kyber768::decapsulate(&valid_sk, &corrupted_ct);
        // Decapsulation might succeed with Kyber (it's designed to be robust)
        // so we just ensure it doesn't panic
        let _ = result;
    }
}

#[test]
fn test_dilithium_invalid_key_sizes() {
    let mut rng = DefaultRng::default();
    
    // Generate valid keys for reference
    let (valid_pk, valid_sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
    
    // Test invalid public key deserialization
    let invalid_pk_bytes = vec![0u8; 100]; // Wrong size
    let result = synapsed_crypto::dilithium::DilithiumPublicKey::<6>::from_bytes(&invalid_pk_bytes);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    // Test invalid secret key deserialization
    let invalid_sk_bytes = vec![0u8; 100]; // Wrong size
    let result = synapsed_crypto::dilithium::DilithiumSecretKey::<6>::from_bytes(&invalid_sk_bytes);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    // Test verification with corrupted signature (we'll corrupt the bytes directly)
    let msg = b"test message";
    let valid_sig = Dilithium3::sign(&valid_sk, msg, &mut rng).unwrap();
    let mut corrupted_sig_bytes = valid_sig.to_bytes().to_vec();
    // Corrupt all bytes
    corrupted_sig_bytes.fill(0xFF);
    
    // Create corrupted signature and verify
    if let Ok(corrupted_sig) = synapsed_crypto::dilithium::DilithiumSignature::from_bytes(&corrupted_sig_bytes) {
        let result = Dilithium3::verify(&valid_pk, msg, &corrupted_sig);
        assert!(matches!(result, Err(Error::InvalidSignature)));
    }
}

#[test]
fn test_signature_verification_failures() {
    let mut rng = DefaultRng::default();
    
    // Test with Dilithium3
    let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
    let msg = b"original message";
    let sig = Dilithium3::sign(&sk, msg, &mut rng).unwrap();
    
    // Verify original works
    assert!(Dilithium3::verify(&pk, msg, &sig).is_ok());
    
    // Test with wrong message
    let wrong_msg = b"wrong message";
    let result = Dilithium3::verify(&pk, wrong_msg, &sig);
    assert!(matches!(result, Err(Error::InvalidSignature)));
    
    // Test with corrupted signature (single bit flip)
    let mut corrupted_sig_bytes = sig.to_bytes().to_vec();
    if !corrupted_sig_bytes.is_empty() {
        corrupted_sig_bytes[0] ^= 0x01;
        if let Ok(corrupted_sig) = synapsed_crypto::dilithium::DilithiumSignature::from_bytes(&corrupted_sig_bytes) {
            let result = Dilithium3::verify(&pk, msg, &corrupted_sig);
            assert!(matches!(result, Err(Error::InvalidSignature)));
        }
    }
}

#[test]
fn test_serialization_errors() {
    let mut rng = DefaultRng::default();
    
    // Test Kyber serialization errors
    let (kyber_pk, kyber_sk) = Kyber768::generate_keypair(&mut rng).unwrap();
    
    // Test deserialization with truncated data
    let pk_bytes = kyber_pk.to_bytes();
    let truncated = &pk_bytes[..pk_bytes.len() - 10];
    let result = synapsed_crypto::kyber::PublicKey::<3>::from_bytes(truncated);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    let sk_bytes = kyber_sk.to_bytes();
    let truncated = &sk_bytes[..sk_bytes.len() - 10];
    let result = synapsed_crypto::kyber::SecretKey::<3>::from_bytes(truncated);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    // Test Dilithium serialization errors
    let (dilithium_pk, dilithium_sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
    
    let pk_bytes = dilithium_pk.to_bytes();
    let truncated = &pk_bytes[..pk_bytes.len() - 10];
    let result = synapsed_crypto::dilithium::DilithiumPublicKey::<6>::from_bytes(truncated);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    let sk_bytes = dilithium_sk.to_bytes();
    let truncated = &sk_bytes[..sk_bytes.len() - 10];
    let result = synapsed_crypto::dilithium::DilithiumSecretKey::<6>::from_bytes(truncated);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
}

#[test]
fn test_empty_input_handling() {
    let mut rng = DefaultRng::default();
    
    // Test Kyber with empty ciphertext
    let empty_ct_bytes = vec![];
    let result = synapsed_crypto::kyber::Ciphertext::<3>::from_bytes(&empty_ct_bytes);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    // Test Dilithium with empty signature
    let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
    let _msg = b"test";
    let empty_sig_bytes = vec![];
    let result = synapsed_crypto::dilithium::DilithiumSignature::from_bytes(&empty_sig_bytes);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    // Test signing empty message (this should work)
    let empty_msg = b"";
    let sig = Dilithium3::sign(&sk, empty_msg, &mut rng).unwrap();
    assert!(Dilithium3::verify(&pk, empty_msg, &sig).is_ok());
}

#[test]
fn test_error_chain_compatibility() {
    // Test that our errors work with error handling patterns
    
    fn operation_that_fails() -> Result<()> {
        Err(Error::InvalidKeySize)
    }
    
    fn handle_error() -> Result<String> {
        match operation_that_fails() {
            Ok(_) => Ok("success".to_string()),
            Err(Error::InvalidKeySize) => Ok("handled key size error".to_string()),
            Err(e) => Err(e),
        }
    }
    
    let result = handle_error().unwrap();
    assert_eq!(result, "handled key size error");
}

#[test]
fn test_concurrent_error_handling() {
    use std::thread;
    use std::sync::Arc;
    
    // Test that errors work correctly in concurrent scenarios
    let error = Arc::new(Error::InvalidKeySize);
    
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let error = Arc::clone(&error);
            thread::spawn(move || {
                // Each thread should see the same error
                matches!(&*error, Error::InvalidKeySize)
            })
        })
        .collect();
    
    for handle in handles {
        assert!(handle.join().unwrap());
    }
}

#[test]
fn test_result_type_ergonomics() {
    // Test that Result<T> works ergonomically with ? operator
    
    fn may_fail(should_fail: bool) -> Result<i32> {
        if should_fail {
            Err(Error::CryptoError)
        } else {
            Ok(42)
        }
    }
    
    fn chain_operations() -> Result<i32> {
        let x = may_fail(false)?;  // Should succeed
        let y = may_fail(false)?;  // Should succeed
        Ok(x + y)
    }
    
    fn chain_with_failure() -> Result<i32> {
        let x = may_fail(false)?;  // Should succeed
        let y = may_fail(true)?;   // Should fail
        Ok(x + y)  // Never reached
    }
    
    assert_eq!(chain_operations().unwrap(), 84);
    assert!(matches!(chain_with_failure(), Err(Error::CryptoError)));
}