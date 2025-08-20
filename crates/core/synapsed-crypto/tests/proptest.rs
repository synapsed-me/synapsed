//! Property-based tests for synapsed-crypto
//!
//! These tests use proptest to verify correctness properties across
//! a wide range of randomly generated inputs.

use proptest::prelude::*;
use synapsed_crypto::{kyber::{Kyber512, Kyber768, Kyber1024}, dilithium::{Dilithium2, Dilithium3, Dilithium5}};
use synapsed_crypto::traits::{Kem, Signature, Serializable, SecureRandom};

// Strategy for generating random byte arrays of specific sizes
fn byte_array_strategy(size: usize) -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), size..=size)
}

// Strategy for generating messages of various sizes
fn message_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..10000)
}

proptest! {
    #[test]
    fn test_kyber_correctness(seed in byte_array_strategy(32)) {
        // Test that encapsulation/decapsulation always produces matching secrets
        let mut rng = synapsed_crypto::random::TestRng::new(
            u64::from_le_bytes(seed.as_slice().get(..8).unwrap_or(&[0u8; 8]).try_into().unwrap_or([0u8; 8]))
        );
        
        // Test all Kyber variants
        test_kyber_variant::<Kyber512>(&mut rng)?;
        test_kyber_variant::<Kyber768>(&mut rng)?;
        test_kyber_variant::<Kyber1024>(&mut rng)?;
    }
    
    #[test]
    fn test_dilithium_correctness(
        seed in byte_array_strategy(32),
        message in message_strategy()
    ) {
        // Test that signatures always verify correctly
        let mut rng = synapsed_crypto::random::TestRng::new(
            u64::from_le_bytes(seed.as_slice().get(..8).unwrap_or(&[0u8; 8]).try_into().unwrap_or([0u8; 8]))
        );
        
        // Test all Dilithium variants
        test_dilithium_variant::<Dilithium2>(&mut rng, &message)?;
        test_dilithium_variant::<Dilithium3>(&mut rng, &message)?;
        test_dilithium_variant::<Dilithium5>(&mut rng, &message)?;
    }
    
    #[test]
    fn test_kyber_determinism(seed in byte_array_strategy(32)) {
        // Test that key generation is deterministic given the same RNG seed
        let mut rng1 = synapsed_crypto::random::TestRng::new(
            u64::from_le_bytes(seed.as_slice().get(..8).unwrap_or(&[0u8; 8]).try_into().unwrap_or([0u8; 8]))
        );
        let mut rng2 = synapsed_crypto::random::TestRng::new(
            u64::from_le_bytes(seed.as_slice().get(..8).unwrap_or(&[0u8; 8]).try_into().unwrap_or([0u8; 8]))
        );
        
        // Generate keys with same seed
        let (pk1, sk1) = Kyber768::generate_keypair(&mut rng1).unwrap();
        let (pk2, sk2) = Kyber768::generate_keypair(&mut rng2).unwrap();
        
        // Keys should be identical
        prop_assert_eq!(pk1.to_bytes(), pk2.to_bytes());
        prop_assert_eq!(sk1.to_bytes(), sk2.to_bytes());
    }
    
    #[test]
    fn test_dilithium_determinism(seed in byte_array_strategy(32)) {
        // Test that key generation is deterministic given the same RNG seed
        let mut rng1 = synapsed_crypto::random::TestRng::new(
            u64::from_le_bytes(seed.as_slice().get(..8).unwrap_or(&[0u8; 8]).try_into().unwrap_or([0u8; 8]))
        );
        let mut rng2 = synapsed_crypto::random::TestRng::new(
            u64::from_le_bytes(seed.as_slice().get(..8).unwrap_or(&[0u8; 8]).try_into().unwrap_or([0u8; 8]))
        );
        
        // Generate keys with same seed
        let (pk1, sk1) = Dilithium3::generate_keypair(&mut rng1).unwrap();
        let (pk2, sk2) = Dilithium3::generate_keypair(&mut rng2).unwrap();
        
        // Keys should be identical
        prop_assert_eq!(pk1.to_bytes(), pk2.to_bytes());
        prop_assert_eq!(sk1.to_bytes(), sk2.to_bytes());
    }
    
    #[test]
    fn test_serialization_round_trip_kyber(seed in byte_array_strategy(32)) {
        // Test that serialization/deserialization preserves key functionality
        let mut rng = synapsed_crypto::random::TestRng::new(
            u64::from_le_bytes(seed.as_slice().get(..8).unwrap_or(&[0u8; 8]).try_into().unwrap_or([0u8; 8]))
        );
        
        let (pk, sk) = Kyber768::generate_keypair(&mut rng).unwrap();
        
        // Serialize and deserialize
        let pk_bytes = pk.to_bytes();
        let sk_bytes = sk.to_bytes();
        
        let pk_restored = <Kyber768 as Kem>::PublicKey::from_bytes(&pk_bytes).unwrap();
        let sk_restored = <Kyber768 as Kem>::SecretKey::from_bytes(&sk_bytes).unwrap();
        
        // Test that restored keys work correctly
        let (ct, ss1) = Kyber768::encapsulate(&pk_restored, &mut rng).unwrap();
        let ss2 = Kyber768::decapsulate(&sk_restored, &ct).unwrap();
        
        prop_assert_eq!(ss1, ss2);
    }
    
    #[test]
    fn test_serialization_round_trip_dilithium(
        seed in byte_array_strategy(32),
        message in message_strategy()
    ) {
        // Test that serialization/deserialization preserves key functionality
        let mut rng = synapsed_crypto::random::TestRng::new(
            u64::from_le_bytes(seed.as_slice().get(..8).unwrap_or(&[0u8; 8]).try_into().unwrap_or([0u8; 8]))
        );
        
        let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
        
        // Serialize and deserialize
        let pk_bytes = pk.to_bytes();
        let sk_bytes = sk.to_bytes();
        
        let pk_restored = <Dilithium3 as Signature>::PublicKey::from_bytes(&pk_bytes).unwrap();
        let sk_restored = <Dilithium3 as Signature>::SecretKey::from_bytes(&sk_bytes).unwrap();
        
        // Test that restored keys work correctly
        let sig = Dilithium3::sign(&sk_restored, &message, &mut rng).unwrap();
        prop_assert!(Dilithium3::verify(&pk_restored, &message, &sig).unwrap());
    }
    
    #[test]
    fn test_invalid_ciphertext_rejection(
        seed in byte_array_strategy(32),
        corruption_index in 0usize..1000
    ) {
        // Test that corrupted ciphertexts are handled correctly
        let mut rng = synapsed_crypto::random::TestRng::new(
            u64::from_le_bytes(seed.as_slice().get(..8).unwrap_or(&[0u8; 8]).try_into().unwrap_or([0u8; 8]))
        );
        
        let (pk, sk) = Kyber768::generate_keypair(&mut rng).unwrap();
        let (ct, original_ss) = Kyber768::encapsulate(&pk, &mut rng).unwrap();
        
        // Corrupt the ciphertext if index is within bounds
        if corruption_index < ct.as_ref().len() {
            let mut ct_bytes = ct.to_bytes();
            ct_bytes[corruption_index] ^= 0xFF;
            let corrupted_ct = <Kyber768 as Kem>::Ciphertext::from_bytes(&ct_bytes).unwrap();
            
            // Decapsulation should either fail or produce a different shared secret
            match Kyber768::decapsulate(&sk, &corrupted_ct) {
                Ok(corrupted_ss) => {
                    // In implicit rejection mode, this might succeed but produce different secret
                    prop_assert_ne!(original_ss.as_ref(), corrupted_ss.as_ref());
                }
                Err(_) => {
                    // Explicit rejection is also acceptable
                }
            }
        }
    }
    
    #[test]
    fn test_invalid_signature_rejection(
        seed in byte_array_strategy(32),
        message in message_strategy(),
        corruption_index in 0usize..3000
    ) {
        // Test that corrupted signatures are rejected
        let mut rng = synapsed_crypto::random::TestRng::new(
            u64::from_le_bytes(seed.as_slice().get(..8).unwrap_or(&[0u8; 8]).try_into().unwrap_or([0u8; 8]))
        );
        
        let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
        let sig = Dilithium3::sign(&sk, &message, &mut rng).unwrap();
        
        // Corrupt the signature if index is within bounds
        if corruption_index < sig.as_ref().len() {
            let mut sig_bytes = sig.to_bytes();
            sig_bytes[corruption_index] ^= 0xFF;
            let corrupted_sig = <Dilithium3 as Signature>::Sig::from_bytes(&sig_bytes).unwrap();
            
            // Verification should fail
            prop_assert!(!Dilithium3::verify(&pk, &message, &corrupted_sig).unwrap());
        }
    }
    
    #[test]
    fn test_different_messages_different_signatures(
        seed in byte_array_strategy(32),
        message1 in message_strategy(),
        message2 in message_strategy()
    ) {
        // Test that different messages produce different signatures
        prop_assume!(message1 != message2);
        
        let mut rng = synapsed_crypto::random::TestRng::new(
            u64::from_le_bytes(seed.as_slice().get(..8).unwrap_or(&[0u8; 8]).try_into().unwrap_or([0u8; 8]))
        );
        
        let (_, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
        
        let sig1 = Dilithium3::sign(&sk, &message1, &mut rng).unwrap();
        let sig2 = Dilithium3::sign(&sk, &message2, &mut rng).unwrap();
        
        // Signatures should be different for different messages
        prop_assert_ne!(sig1.as_ref(), sig2.as_ref());
    }
    
    #[test]
    fn test_encapsulation_produces_different_secrets(seed in byte_array_strategy(32)) {
        // Test that multiple encapsulations produce different shared secrets
        let mut rng = synapsed_crypto::random::TestRng::new(
            u64::from_le_bytes(seed.as_slice().get(..8).unwrap_or(&[0u8; 8]).try_into().unwrap_or([0u8; 8]))
        );
        
        let (pk, _) = Kyber768::generate_keypair(&mut rng).unwrap();
        
        let (ct1, ss1) = Kyber768::encapsulate(&pk, &mut rng).unwrap();
        let (ct2, ss2) = Kyber768::encapsulate(&pk, &mut rng).unwrap();
        
        // Different encapsulations should produce different ciphertexts and secrets
        prop_assert_ne!(ct1.as_ref(), ct2.as_ref());
        prop_assert_ne!(ss1.as_ref(), ss2.as_ref());
    }
}

// Helper functions for property tests
fn test_kyber_variant<T>(rng: &mut impl SecureRandom) -> std::result::Result<(), TestCaseError>
where
    T: Kem,
    T::SharedSecret: PartialEq,
{
    let (pk, sk) = T::generate_keypair(rng)
        .map_err(|_| TestCaseError::fail("Key generation failed"))?;
    
    let (ct, ss1) = T::encapsulate(&pk, rng)
        .map_err(|_| TestCaseError::fail("Encapsulation failed"))?;
    
    let ss2 = T::decapsulate(&sk, &ct)
        .map_err(|_| TestCaseError::fail("Decapsulation failed"))?;
    
    prop_assert_eq!(ss1.as_ref(), ss2.as_ref());
    Ok(())
}

fn test_dilithium_variant<T>(
    rng: &mut impl SecureRandom,
    message: &[u8]
) -> std::result::Result<(), TestCaseError>
where
    T: Signature,
{
    let (pk, sk) = T::generate_keypair(rng)
        .map_err(|_| TestCaseError::fail("Key generation failed"))?;
    
    let sig = T::sign(&sk, message, rng)
        .map_err(|_| TestCaseError::fail("Signing failed"))?;
    
    let result = T::verify(&pk, message, &sig)
        .map_err(|_| TestCaseError::fail("Verification failed"))?;
    
    prop_assert!(result);
    Ok(())
}