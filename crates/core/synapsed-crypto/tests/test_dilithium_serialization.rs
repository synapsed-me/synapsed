//! Test for Dilithium key serialization

use synapsed_crypto::{
    dilithium::{Dilithium2, Dilithium3, Dilithium5, PublicKey, SecretKey},
    traits::{Signature, Serializable},
    random::TestRng,
};

#[test]
fn test_dilithium2_key_serialization() {
    let mut rng = TestRng::new(42);
    
    // Generate keypair
    let (pk, sk) = Dilithium2::generate_keypair(&mut rng).unwrap();
    
    // Test public key serialization
    let pk_bytes = pk.to_bytes();
    assert_eq!(pk_bytes.len(), 1312); // Expected size for Dilithium2
    let recovered_pk = PublicKey::<4>::from_bytes(&pk_bytes).unwrap();
    assert_eq!(pk.to_bytes(), recovered_pk.to_bytes());
    
    // Test secret key serialization
    let sk_bytes = sk.to_bytes();
    assert_eq!(sk_bytes.len(), 2528); // Expected size for Dilithium2
    let recovered_sk = SecretKey::<4>::from_bytes(&sk_bytes).unwrap();
    assert_eq!(sk.to_bytes(), recovered_sk.to_bytes());
    
    // Test that recovered keys work for signing/verification
    let message = b"Test message for Dilithium2";
    let signature = Dilithium2::sign(&recovered_sk, message, &mut rng).unwrap();
    let valid = Dilithium2::verify(&recovered_pk, message, &signature).unwrap();
    assert!(valid);
}

#[test]
fn test_dilithium3_key_serialization() {
    let mut rng = TestRng::new(42);
    
    // Generate keypair
    let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
    
    // Test public key serialization
    let pk_bytes = pk.to_bytes();
    assert_eq!(pk_bytes.len(), 1952); // Expected size for Dilithium3
    let recovered_pk = PublicKey::<6>::from_bytes(&pk_bytes).unwrap();
    assert_eq!(pk.to_bytes(), recovered_pk.to_bytes());
    
    // Test secret key serialization
    let sk_bytes = sk.to_bytes();
    assert_eq!(sk_bytes.len(), 4000); // Expected size for Dilithium3
    let recovered_sk = SecretKey::<6>::from_bytes(&sk_bytes).unwrap();
    assert_eq!(sk.to_bytes(), recovered_sk.to_bytes());
    
    // Test that recovered keys work for signing/verification
    let message = b"Test message for Dilithium3";
    let signature = Dilithium3::sign(&recovered_sk, message, &mut rng).unwrap();
    let valid = Dilithium3::verify(&recovered_pk, message, &signature).unwrap();
    assert!(valid);
}

#[test]
fn test_dilithium5_key_serialization() {
    let mut rng = TestRng::new(42);
    
    // Generate keypair
    let (pk, sk) = Dilithium5::generate_keypair(&mut rng).unwrap();
    
    // Test public key serialization
    let pk_bytes = pk.to_bytes();
    assert_eq!(pk_bytes.len(), 2592); // Expected size for Dilithium5
    let recovered_pk = PublicKey::<8>::from_bytes(&pk_bytes).unwrap();
    assert_eq!(pk.to_bytes(), recovered_pk.to_bytes());
    
    // Test secret key serialization
    let sk_bytes = sk.to_bytes();
    assert_eq!(sk_bytes.len(), 4864); // Expected size for Dilithium5
    let recovered_sk = SecretKey::<8>::from_bytes(&sk_bytes).unwrap();
    assert_eq!(sk.to_bytes(), recovered_sk.to_bytes());
    
    // Test that recovered keys work for signing/verification
    let message = b"Test message for Dilithium5";
    let signature = Dilithium5::sign(&recovered_sk, message, &mut rng).unwrap();
    let valid = Dilithium5::verify(&recovered_pk, message, &signature).unwrap();
    assert!(valid);
}

#[test]
fn test_invalid_key_sizes() {
    // Test invalid public key sizes
    let invalid_pk_small = vec![0u8; 100];
    assert!(PublicKey::<4>::from_bytes(&invalid_pk_small).is_err());
    
    let invalid_pk_large = vec![0u8; 5000];
    assert!(PublicKey::<4>::from_bytes(&invalid_pk_large).is_err());
    
    // Test invalid secret key sizes
    let invalid_sk_small = vec![0u8; 100];
    assert!(SecretKey::<4>::from_bytes(&invalid_sk_small).is_err());
    
    let invalid_sk_large = vec![0u8; 10000];
    assert!(SecretKey::<4>::from_bytes(&invalid_sk_large).is_err());
}

#[test]
fn test_signature_serialization() {
    use synapsed_crypto::dilithium::DilithiumSignature;
    
    let mut rng = TestRng::new(42);
    let (_, sk) = Dilithium2::generate_keypair(&mut rng).unwrap();
    let message = b"Test message";
    
    // Generate signature
    let sig = Dilithium2::sign(&sk, message, &mut rng).unwrap();
    
    // Test signature serialization
    let sig_bytes = sig.to_bytes();
    assert_eq!(sig_bytes.len(), 2420); // Expected size for Dilithium2
    
    let recovered_sig = DilithiumSignature::from_bytes(&sig_bytes).unwrap();
    assert_eq!(sig.to_bytes(), recovered_sig.to_bytes());
}