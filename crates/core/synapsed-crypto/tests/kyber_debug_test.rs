//! Debug test for Kyber to verify the implementation

use synapsed_crypto::{
    kyber::Kyber512,
    traits::{Kem, Serializable},
    random::TestRng,
};

#[test]
fn test_kyber512_basic() {
    let mut rng = TestRng::new(42);
    
    // Generate keypair
    let (pk, sk) = Kyber512::generate_keypair(&mut rng).unwrap();
    println!("Public key size: {}", pk.to_bytes().len());
    println!("Secret key size: {}", sk.to_bytes().len());
    
    // Encapsulate
    let (ct, ss1) = Kyber512::encapsulate(&pk, &mut rng).unwrap();
    println!("Ciphertext size: {}", ct.to_bytes().len());
    println!("Shared secret 1: {:?}", ss1.bytes);
    
    // Decapsulate
    let ss2 = Kyber512::decapsulate(&sk, &ct).unwrap();
    println!("Shared secret 2: {:?}", ss2.bytes);
    
    // They should match
    assert_eq!(ss1.bytes, ss2.bytes, "Shared secrets don't match!");
}

#[test]
fn test_kyber512_corrupted_ciphertext() {
    let mut rng = TestRng::new(42);
    
    // Generate keypair
    let (pk, sk) = Kyber512::generate_keypair(&mut rng).unwrap();
    
    // Encapsulate
    let (ct, ss1) = Kyber512::encapsulate(&pk, &mut rng).unwrap();
    
    // Corrupt the ciphertext
    let mut ct_corrupted = ct.clone();
    ct_corrupted.bytes[0] ^= 0xFF;
    
    // Decapsulate with corrupted ciphertext
    let ss2 = Kyber512::decapsulate(&sk, &ct_corrupted).unwrap();
    
    // They should NOT match
    assert_ne!(ss1.bytes, ss2.bytes, "Shared secrets should not match with corrupted ciphertext!");
}