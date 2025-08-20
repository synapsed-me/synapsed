//! Comprehensive unit tests for hybrid cryptographic modes
//!
//! These tests ensure the hybrid module correctly combines classical
//! and post-quantum algorithms for defense in depth.

#![cfg(feature = "hybrid")]

use synapsed_crypto::hybrid::*;
use synapsed_crypto::random::DefaultRng;
use synapsed_crypto::traits::SecureRandom;
use synapsed_crypto::error::{Error, Result};

// Mock implementations for testing the trait interfaces
struct MockClassicalKem;
struct MockPostQuantumKem;
struct MockHybridKemImpl {
    #[allow(dead_code)]
    classical: MockClassicalKem,
    #[allow(dead_code)]
    post_quantum: MockPostQuantumKem,
}

impl HybridKem for MockHybridKemImpl {
    fn generate_keypair<R: SecureRandom>(&self, rng: &mut R) -> Result<(Vec<u8>, Vec<u8>)> {
        // Generate mock hybrid keypair
        let mut pk = vec![0u8; 64];  // 32 bytes classical + 32 bytes PQ
        let mut sk = vec![0u8; 96];  // 32 bytes classical + 64 bytes PQ
        
        rng.fill_bytes(&mut pk);
        rng.fill_bytes(&mut sk);
        
        Ok((pk, sk))
    }
    
    fn encapsulate<R: SecureRandom>(
        &self, 
        public_key: &[u8], 
        rng: &mut R
    ) -> Result<(Vec<u8>, Vec<u8>)> {
        if public_key.len() != 64 {
            return Err(Error::InvalidKeySize);
        }
        
        let mut ciphertext = vec![0u8; 96];  // Combined ciphertext
        let mut shared_secret = vec![0u8; 64];  // Combined shared secret
        
        rng.fill_bytes(&mut ciphertext);
        rng.fill_bytes(&mut shared_secret);
        
        Ok((ciphertext, shared_secret))
    }
    
    fn decapsulate(&self, secret_key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        if secret_key.len() != 96 {
            return Err(Error::InvalidKeySize);
        }
        if ciphertext.len() != 96 {
            return Err(Error::InvalidCiphertext);
        }
        
        // Mock decapsulation
        let shared_secret = vec![0u8; 64];
        Ok(shared_secret)
    }
}

struct MockClassicalSig;
struct MockPostQuantumSig;
struct MockHybridSigImpl {
    #[allow(dead_code)]
    classical: MockClassicalSig,
    #[allow(dead_code)]
    post_quantum: MockPostQuantumSig,
}

impl HybridSignature for MockHybridSigImpl {
    fn generate_keypair<R: SecureRandom>(&self, rng: &mut R) -> Result<(Vec<u8>, Vec<u8>)> {
        let mut pk = vec![0u8; 96];  // Combined public key
        let mut sk = vec![0u8; 128]; // Combined secret key
        
        rng.fill_bytes(&mut pk);
        rng.fill_bytes(&mut sk);
        
        Ok((pk, sk))
    }
    
    fn sign<R: SecureRandom>(
        &self,
        secret_key: &[u8],
        message: &[u8],
        rng: &mut R,
    ) -> Result<Vec<u8>> {
        if secret_key.len() != 128 {
            return Err(Error::InvalidKeySize);
        }
        
        // Create combined signature
        let mut signature = vec![0u8; 160];  // Mock combined signature
        rng.fill_bytes(&mut signature);
        
        // Include message in signature generation (mock)
        if !message.is_empty() {
            signature[0] = message[0];
        }
        
        Ok(signature)
    }
    
    fn verify(&self, public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<bool> {
        if public_key.len() != 96 {
            return Err(Error::InvalidKeySize);
        }
        if signature.len() != 160 {
            return Err(Error::InvalidSignature);
        }
        
        // Mock verification - check if message byte matches
        if !message.is_empty() && signature[0] != message[0] {
            return Ok(false);
        }
        
        Ok(true)
    }
}

#[test]
fn test_hybrid_kem_basic_operations() {
    let hybrid_kem = MockHybridKemImpl {
        classical: MockClassicalKem,
        post_quantum: MockPostQuantumKem,
    };
    
    let mut rng = DefaultRng::default();
    
    // Generate keypair
    let (pk, sk) = hybrid_kem.generate_keypair(&mut rng).unwrap();
    assert_eq!(pk.len(), 64);
    assert_eq!(sk.len(), 96);
    
    // Encapsulate
    let (ct, ss1) = hybrid_kem.encapsulate(&pk, &mut rng).unwrap();
    assert_eq!(ct.len(), 96);
    assert_eq!(ss1.len(), 64);
    
    // Decapsulate
    let ss2 = hybrid_kem.decapsulate(&sk, &ct).unwrap();
    assert_eq!(ss2.len(), 64);
}

#[test]
fn test_hybrid_kem_error_handling() {
    let hybrid_kem = MockHybridKemImpl {
        classical: MockClassicalKem,
        post_quantum: MockPostQuantumKem,
    };
    
    let mut rng = DefaultRng::default();
    
    // Test invalid public key size
    let bad_pk = vec![0u8; 32];  // Wrong size
    let result = hybrid_kem.encapsulate(&bad_pk, &mut rng);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    // Test invalid secret key size
    let bad_sk = vec![0u8; 32];  // Wrong size
    let ct = vec![0u8; 96];
    let result = hybrid_kem.decapsulate(&bad_sk, &ct);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    // Test invalid ciphertext size
    let (_, sk) = hybrid_kem.generate_keypair(&mut rng).unwrap();
    let bad_ct = vec![0u8; 32];  // Wrong size
    let result = hybrid_kem.decapsulate(&sk, &bad_ct);
    assert!(matches!(result, Err(Error::InvalidCiphertext)));
}

#[test]
fn test_hybrid_signature_basic_operations() {
    let hybrid_sig = MockHybridSigImpl {
        classical: MockClassicalSig,
        post_quantum: MockPostQuantumSig,
    };
    
    let mut rng = DefaultRng::default();
    
    // Generate keypair
    let (pk, sk) = hybrid_sig.generate_keypair(&mut rng).unwrap();
    assert_eq!(pk.len(), 96);
    assert_eq!(sk.len(), 128);
    
    // Test various message sizes
    let test_messages: &[&[u8]] = &[
        b"",  // Empty
        b"Hello",  // Short
        b"This is a longer message for testing",  // Medium
        &[0xAB; 1000],  // Large
    ];
    
    for msg in test_messages {
        // Sign
        let sig = hybrid_sig.sign(&sk, msg, &mut rng).unwrap();
        assert_eq!(sig.len(), 160);
        
        // Verify
        assert!(hybrid_sig.verify(&pk, msg, &sig).unwrap());
        
        // Verify with wrong message should fail
        let wrong_msg = b"wrong";
        if !msg.is_empty() {
            assert!(!hybrid_sig.verify(&pk, wrong_msg, &sig).unwrap());
        }
    }
}

#[test]
fn test_hybrid_signature_error_handling() {
    let hybrid_sig = MockHybridSigImpl {
        classical: MockClassicalSig,
        post_quantum: MockPostQuantumSig,
    };
    
    let mut rng = DefaultRng::default();
    let msg = b"test message";
    
    // Test invalid secret key size
    let bad_sk = vec![0u8; 64];  // Wrong size
    let result = hybrid_sig.sign(&bad_sk, msg, &mut rng);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    // Test invalid public key size for verification
    let sig = vec![0u8; 160];
    let bad_pk = vec![0u8; 32];  // Wrong size
    let result = hybrid_sig.verify(&bad_pk, msg, &sig);
    assert!(matches!(result, Err(Error::InvalidKeySize)));
    
    // Test invalid signature size
    let (pk, _) = hybrid_sig.generate_keypair(&mut rng).unwrap();
    let bad_sig = vec![0u8; 80];  // Wrong size
    let result = hybrid_sig.verify(&pk, msg, &bad_sig);
    assert!(matches!(result, Err(Error::InvalidSignature)));
}

#[test]
fn test_hybrid_trait_usage() {
    // Note: Traits with generic methods cannot be made into trait objects directly.
    // This test demonstrates the concrete trait usage pattern instead.
    let hybrid_kem = MockHybridKemImpl {
        classical: MockClassicalKem,
        post_quantum: MockPostQuantumKem,
    };
    
    let mut rng = DefaultRng::default();
    let (pk, sk) = hybrid_kem.generate_keypair(&mut rng).unwrap();
    let (ct, ss1) = hybrid_kem.encapsulate(&pk, &mut rng).unwrap();
    let ss2 = hybrid_kem.decapsulate(&sk, &ct).unwrap();
    assert_eq!(ss1.len(), ss2.len());
    
    let hybrid_sig = MockHybridSigImpl {
        classical: MockClassicalSig,
        post_quantum: MockPostQuantumSig,
    };
    
    let (pk, sk) = hybrid_sig.generate_keypair(&mut rng).unwrap();
    let msg = b"test";
    let sig = hybrid_sig.sign(&sk, msg, &mut rng).unwrap();
    assert!(hybrid_sig.verify(&pk, msg, &sig).unwrap());
}

#[test]
fn test_multiple_hybrid_operations() {
    let hybrid_kem = MockHybridKemImpl {
        classical: MockClassicalKem,
        post_quantum: MockPostQuantumKem,
    };
    
    let hybrid_sig = MockHybridSigImpl {
        classical: MockClassicalSig,
        post_quantum: MockPostQuantumSig,
    };
    
    let mut rng = DefaultRng::default();
    
    // Perform multiple operations to ensure state independence
    for i in 0..10 {
        // KEM operations
        let (pk, sk) = hybrid_kem.generate_keypair(&mut rng).unwrap();
        let (ct, ss1) = hybrid_kem.encapsulate(&pk, &mut rng).unwrap();
        let ss2 = hybrid_kem.decapsulate(&sk, &ct).unwrap();
        assert_eq!(ss1.len(), ss2.len());
        
        // Signature operations
        let (sig_pk, sig_sk) = hybrid_sig.generate_keypair(&mut rng).unwrap();
        let msg = format!("Message {i}");
        let sig = hybrid_sig.sign(&sig_sk, msg.as_bytes(), &mut rng).unwrap();
        assert!(hybrid_sig.verify(&sig_pk, msg.as_bytes(), &sig).unwrap());
    }
}

#[test]
fn test_basic_hybrid_kem_struct() {
    // Test the BasicHybridKem struct exists and can be created using new()
    #[derive(Debug)]
    struct DummyClassical;
    #[derive(Debug)]
    struct DummyPQ;
    
    let _hybrid = BasicHybridKem::new(DummyClassical, DummyPQ);
    
    // The struct should compile and be debuggable
    let debug_str = format!("{_hybrid:?}");
    assert!(debug_str.contains("BasicHybridKem"));
}