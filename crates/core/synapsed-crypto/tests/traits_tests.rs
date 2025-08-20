//! Comprehensive unit tests for crypto traits
//!
//! These tests ensure all trait implementations work correctly
//! across different algorithms and parameter sets.

use synapsed_crypto::traits::{Kem, Signature, Serializable, SecureRandom};
use synapsed_crypto::kyber::{Kyber512, Kyber768, Kyber1024, PublicKey as KyberPub, SecretKey as KyberSec};
use synapsed_crypto::dilithium::{Dilithium2, Dilithium3, Dilithium5, PublicKey as DilPub, SecretKey as DilSec};
use synapsed_crypto::random::DefaultRng;
use synapsed_crypto::error::Result;

// Mock SecureRandom implementation for testing
struct MockRng {
    counter: u8,
}

impl MockRng {
    fn new() -> Self {
        Self { counter: 0 }
    }
}

impl SecureRandom for MockRng {
    fn fill_bytes(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.counter;
            self.counter = self.counter.wrapping_add(1);
        }
    }
}

#[test]
fn test_kem_trait_generic_usage() {
    // Test that KEM trait can be used generically
    test_kem_generic::<Kyber512>();
    test_kem_generic::<Kyber768>();
    test_kem_generic::<Kyber1024>();
}

fn test_kem_generic<K: Kem>() {
    let mut rng = DefaultRng::default();
    
    // Test basic KEM operations
    let (pk, sk) = K::generate_keypair(&mut rng).unwrap();
    let (ct, ss1) = K::encapsulate(&pk, &mut rng).unwrap();
    let ss2 = K::decapsulate(&sk, &ct).unwrap();
    
    assert_eq!(ss1, ss2);
}

#[test]
fn test_signature_trait_generic_usage() {
    // Test that Signature trait can be used generically
    test_signature_generic::<Dilithium2>();
    test_signature_generic::<Dilithium3>();
    test_signature_generic::<Dilithium5>();
}

fn test_signature_generic<S: Signature>() {
    let mut rng = DefaultRng::default();
    let msg = b"test message for generic signature";
    
    // Test basic signature operations
    let (pk, sk) = S::generate_keypair(&mut rng).unwrap();
    let sig = S::sign(&sk, msg, &mut rng).unwrap();
    assert!(S::verify(&pk, msg, &sig).is_ok());
}

#[test]
fn test_serializable_trait_kyber_keys() {
    let mut rng = DefaultRng::default();
    
    // Test Kyber512 keys
    test_kyber512_serialization(&mut rng);
    
    // Test Kyber768 keys
    test_kyber768_serialization(&mut rng);
    
    // Test Kyber1024 keys
    test_kyber1024_serialization(&mut rng);
}

fn test_kyber512_serialization<R: SecureRandom>(rng: &mut R) {
    let (pk, sk) = Kyber512::generate_keypair(rng).unwrap();
    
    // Test public key serialization
    let pk_bytes = pk.to_bytes();
    let pk_recovered = KyberPub::<2>::from_bytes(&pk_bytes).unwrap();
    assert_eq!(pk.to_bytes(), pk_recovered.to_bytes());
    
    // Test secret key serialization
    let sk_bytes = sk.to_bytes();
    let sk_recovered = KyberSec::<2>::from_bytes(&sk_bytes).unwrap();
    assert_eq!(sk.to_bytes(), sk_recovered.to_bytes());
    
    // Test that serialized keys still work
    let (ct, ss1) = Kyber512::encapsulate(&pk_recovered, rng).unwrap();
    let ss2 = Kyber512::decapsulate(&sk_recovered, &ct).unwrap();
    assert_eq!(ss1, ss2);
}

fn test_kyber768_serialization<R: SecureRandom>(rng: &mut R) {
    let (pk, sk) = Kyber768::generate_keypair(rng).unwrap();
    
    // Test public key serialization
    let pk_bytes = pk.to_bytes();
    let pk_recovered = KyberPub::<3>::from_bytes(&pk_bytes).unwrap();
    assert_eq!(pk.to_bytes(), pk_recovered.to_bytes());
    
    // Test secret key serialization
    let sk_bytes = sk.to_bytes();
    let sk_recovered = KyberSec::<3>::from_bytes(&sk_bytes).unwrap();
    assert_eq!(sk.to_bytes(), sk_recovered.to_bytes());
    
    // Test that serialized keys still work
    let (ct, ss1) = Kyber768::encapsulate(&pk_recovered, rng).unwrap();
    let ss2 = Kyber768::decapsulate(&sk_recovered, &ct).unwrap();
    assert_eq!(ss1, ss2);
}

fn test_kyber1024_serialization<R: SecureRandom>(rng: &mut R) {
    let (pk, sk) = Kyber1024::generate_keypair(rng).unwrap();
    
    // Test public key serialization
    let pk_bytes = pk.to_bytes();
    let pk_recovered = KyberPub::<4>::from_bytes(&pk_bytes).unwrap();
    assert_eq!(pk.to_bytes(), pk_recovered.to_bytes());
    
    // Test secret key serialization
    let sk_bytes = sk.to_bytes();
    let sk_recovered = KyberSec::<4>::from_bytes(&sk_bytes).unwrap();
    assert_eq!(sk.to_bytes(), sk_recovered.to_bytes());
    
    // Test that serialized keys still work
    let (ct, ss1) = Kyber1024::encapsulate(&pk_recovered, rng).unwrap();
    let ss2 = Kyber1024::decapsulate(&sk_recovered, &ct).unwrap();
    assert_eq!(ss1, ss2);
}

#[test]
fn test_serializable_trait_dilithium_keys() {
    let mut rng = DefaultRng::default();
    
    // Test Dilithium2 keys
    test_dilithium2_serialization(&mut rng);
    
    // Test Dilithium3 keys
    test_dilithium3_serialization(&mut rng);
    
    // Test Dilithium5 keys
    test_dilithium5_serialization(&mut rng);
}

fn test_dilithium2_serialization<R: SecureRandom>(rng: &mut R) {
    let (pk, sk) = Dilithium2::generate_keypair(rng).unwrap();
    
    // Test public key serialization
    let pk_bytes = pk.to_bytes();
    let pk_recovered = DilPub::<4>::from_bytes(&pk_bytes).unwrap();
    assert_eq!(pk.to_bytes(), pk_recovered.to_bytes());
    
    // Test secret key serialization
    let sk_bytes = sk.to_bytes();
    let sk_recovered = DilSec::<4>::from_bytes(&sk_bytes).unwrap();
    assert_eq!(sk.to_bytes(), sk_recovered.to_bytes());
    
    // Test that serialized keys still work
    let msg = b"test message";
    let sig = Dilithium2::sign(&sk_recovered, msg, rng).unwrap();
    assert!(Dilithium2::verify(&pk_recovered, msg, &sig).is_ok());
}

fn test_dilithium3_serialization<R: SecureRandom>(rng: &mut R) {
    let (pk, sk) = Dilithium3::generate_keypair(rng).unwrap();
    
    // Test public key serialization
    let pk_bytes = pk.to_bytes();
    let pk_recovered = DilPub::<6>::from_bytes(&pk_bytes).unwrap();
    assert_eq!(pk.to_bytes(), pk_recovered.to_bytes());
    
    // Test secret key serialization
    let sk_bytes = sk.to_bytes();
    let sk_recovered = DilSec::<6>::from_bytes(&sk_bytes).unwrap();
    assert_eq!(sk.to_bytes(), sk_recovered.to_bytes());
    
    // Test that serialized keys still work
    let msg = b"test message";
    let sig = Dilithium3::sign(&sk_recovered, msg, rng).unwrap();
    assert!(Dilithium3::verify(&pk_recovered, msg, &sig).is_ok());
}

fn test_dilithium5_serialization<R: SecureRandom>(rng: &mut R) {
    let (pk, sk) = Dilithium5::generate_keypair(rng).unwrap();
    
    // Test public key serialization
    let pk_bytes = pk.to_bytes();
    let pk_recovered = DilPub::<8>::from_bytes(&pk_bytes).unwrap();
    assert_eq!(pk.to_bytes(), pk_recovered.to_bytes());
    
    // Test secret key serialization
    let sk_bytes = sk.to_bytes();
    let sk_recovered = DilSec::<8>::from_bytes(&sk_bytes).unwrap();
    assert_eq!(sk.to_bytes(), sk_recovered.to_bytes());
    
    // Test that serialized keys still work
    let msg = b"test message";
    let sig = Dilithium5::sign(&sk_recovered, msg, rng).unwrap();
    assert!(Dilithium5::verify(&pk_recovered, msg, &sig).is_ok());
}

#[test]
fn test_secure_random_trait() {
    let mut os_rng = DefaultRng::default();
    let mut mock_rng = MockRng::new();
    
    // Test OsRng implementation
    let mut buffer1 = [0u8; 32];
    os_rng.fill_bytes(&mut buffer1);
    let mut buffer2 = [0u8; 32];
    os_rng.fill_bytes(&mut buffer2);
    // OS random should produce different results
    assert_ne!(buffer1, buffer2);
    
    // Test MockRng implementation
    let mut buffer3 = [0u8; 10];
    mock_rng.fill_bytes(&mut buffer3);
    assert_eq!(buffer3, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    
    // Test that MockRng continues sequence
    let mut buffer4 = [0u8; 5];
    mock_rng.fill_bytes(&mut buffer4);
    assert_eq!(buffer4, [10, 11, 12, 13, 14]);
}

#[test]
fn test_trait_object_compatibility() {
    // Test that our traits can be used as trait objects where appropriate
    
    // We can't easily test KEM and Signature as trait objects because they have
    // generic methods, but we can test SecureRandom
    
    let mut os_rng = DefaultRng::default();
    let rng_ref: &mut dyn SecureRandom = &mut os_rng;
    
    let mut buffer = [0u8; 16];
    rng_ref.fill_bytes(&mut buffer);
    
    // Buffer should be filled with non-zero values (high probability)
    assert!(buffer.iter().any(|&b| b != 0));
}

#[test]
fn test_trait_bounds_and_generics() {
    // Test that we can write generic functions with trait bounds
    
    fn encrypt_and_decrypt<K: Kem, R: SecureRandom>(rng: &mut R) -> Result<bool> {
        let (pk, sk) = K::generate_keypair(rng)?;
        let (ct, ss1) = K::encapsulate(&pk, rng)?;
        let ss2 = K::decapsulate(&sk, &ct)?;
        Ok(ss1 == ss2)
    }
    
    fn sign_and_verify<S: Signature, R: SecureRandom>(rng: &mut R, msg: &[u8]) -> Result<bool> {
        let (pk, sk) = S::generate_keypair(rng)?;
        let sig = S::sign(&sk, msg, rng)?;
        S::verify(&pk, msg, &sig)
    }
    
    let mut rng = DefaultRng::default();
    let msg = b"test message for generic functions";
    
    // Test with different KEM algorithms
    assert!(encrypt_and_decrypt::<Kyber512, _>(&mut rng).unwrap());
    assert!(encrypt_and_decrypt::<Kyber768, _>(&mut rng).unwrap());
    assert!(encrypt_and_decrypt::<Kyber1024, _>(&mut rng).unwrap());
    
    // Test with different signature algorithms
    assert!(sign_and_verify::<Dilithium2, _>(&mut rng, msg).unwrap());
    assert!(sign_and_verify::<Dilithium3, _>(&mut rng, msg).unwrap());
    assert!(sign_and_verify::<Dilithium5, _>(&mut rng, msg).unwrap());
}

#[test]
fn test_serializable_edge_cases() {
    let mut rng = DefaultRng::default();
    
    // Test multiple serialization/deserialization cycles
    let (pk, sk) = Kyber768::generate_keypair(&mut rng).unwrap();
    
    let mut current_pk = pk;
    let mut current_sk = sk;
    
    for _ in 0..10 {
        // Serialize and deserialize public key
        let pk_bytes = current_pk.to_bytes();
        current_pk = KyberPub::<3>::from_bytes(&pk_bytes).unwrap();
        
        // Serialize and deserialize secret key
        let sk_bytes = current_sk.to_bytes();
        current_sk = KyberSec::<3>::from_bytes(&sk_bytes).unwrap();
        
        // Verify keys still work after each cycle
        let (ct, ss1) = Kyber768::encapsulate(&current_pk, &mut rng).unwrap();
        let ss2 = Kyber768::decapsulate(&current_sk, &ct).unwrap();
        assert_eq!(ss1, ss2);
    }
}

#[test]
fn test_trait_method_consistency() {
    // Test that trait methods are consistent across implementations
    let mut rng = DefaultRng::default();
    
    // Test KEM consistency
    for _ in 0..10 {
        let (pk, sk) = Kyber768::generate_keypair(&mut rng).unwrap();
        let (ct, ss1) = Kyber768::encapsulate(&pk, &mut rng).unwrap();
        let ss2 = Kyber768::decapsulate(&sk, &ct).unwrap();
        
        // Shared secrets must always match
        assert_eq!(ss1, ss2);
        
        // Key and ciphertext sizes should be consistent
        assert!(!pk.to_bytes().is_empty());
        assert!(!sk.to_bytes().is_empty());
        assert!(!ct.as_ref().is_empty());
        assert!(!ss1.as_ref().is_empty());
    }
    
    // Test Signature consistency
    let msg = b"consistency test message";
    for _ in 0..10 {
        let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
        let sig = Dilithium3::sign(&sk, msg, &mut rng).unwrap();
        
        // Verification must always succeed for valid signatures
        assert!(Dilithium3::verify(&pk, msg, &sig).is_ok());
        
        // Key and signature sizes should be reasonable
        assert!(!pk.to_bytes().is_empty());
        assert!(!sk.to_bytes().is_empty());
        assert!(!sig.as_ref().is_empty());
    }
}

#[test]
fn test_trait_lifetime_handling() {
    // Test that traits work correctly with different lifetimes
    
    fn process_with_borrowed_rng<K: Kem, R: SecureRandom>(rng: &mut R) {
        let (pk, sk) = K::generate_keypair(rng).unwrap();
        let (ct, ss1) = K::encapsulate(&pk, rng).unwrap();
        let ss2 = K::decapsulate(&sk, &ct).unwrap();
        assert_eq!(ss1, ss2);
    }
    
    fn process_with_owned_keys<K: Kem, R: SecureRandom>(pk: K::PublicKey, sk: K::SecretKey, rng: &mut R) {
        let (ct, ss1) = K::encapsulate(&pk, rng).unwrap();
        let ss2 = K::decapsulate(&sk, &ct).unwrap();
        assert_eq!(ss1, ss2);
    }
    
    let mut rng = DefaultRng::default();
    
    // Test with borrowed RNG
    process_with_borrowed_rng::<Kyber768, _>(&mut rng);
    
    // Test with owned keys
    let (pk, sk) = Kyber512::generate_keypair(&mut rng).unwrap();
    process_with_owned_keys::<Kyber512, _>(pk, sk, &mut rng);
}

#[test]
fn test_send_sync_bounds() {
    // Test that our types are Send + Sync where appropriate
    fn assert_send_sync<T: Send + Sync>() {}
    
    // These should compile if the types are Send + Sync
    assert_send_sync::<synapsed_crypto::error::Error>();
    // Note: Key types contain secret data, so they may not be Send + Sync
    // This is a security feature to prevent accidental sharing across threads
}