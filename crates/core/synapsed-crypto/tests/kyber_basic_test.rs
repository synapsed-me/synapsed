//! Test basic Kyber operations to isolate the issue

use synapsed_crypto::{
    kyber::Kyber512,
    traits::Kem,
    random::TestRng,
};

#[test]
fn test_kyber_deterministic() {
    println!("\n=== Testing Kyber512 with Deterministic RNG ===");
    
    // Use the same seed for both keygen and encapsulation
    // This should give us reproducible results
    
    let mut rng1 = TestRng::new(42);
    let (pk1, sk1) = Kyber512::generate_keypair(&mut rng1).unwrap();
    
    let mut rng2 = TestRng::new(42); 
    let (pk2, sk2) = Kyber512::generate_keypair(&mut rng2).unwrap();
    
    // Keys should be identical with same RNG seed
    assert_eq!(pk1.as_ref(), pk2.as_ref(), "Public keys should match with same RNG");
    assert_eq!(sk1.as_ref(), sk2.as_ref(), "Secret keys should match with same RNG");
    println!("✓ Deterministic key generation verified");
    
    // Now test encapsulation with fresh deterministic RNG
    let mut rng_enc1 = TestRng::new(123);
    let (ct1, ss1) = Kyber512::encapsulate(&pk1, &mut rng_enc1).unwrap();
    
    let mut rng_enc2 = TestRng::new(123);
    let (ct2, _ss2) = Kyber512::encapsulate(&pk1, &mut rng_enc2).unwrap();
    
    // Ciphertexts should be identical with same RNG
    assert_eq!(ct1.as_ref(), ct2.as_ref(), "Ciphertexts should match with same RNG");
    println!("✓ Deterministic encapsulation verified");
    
    // Now the critical test - decapsulation
    let ss_dec1 = Kyber512::decapsulate(&sk1, &ct1).unwrap();
    let _ss_dec2 = Kyber512::decapsulate(&sk1, &ct2).unwrap();
    
    println!("\nShared secret from encapsulation: {:02x?}", &ss1.as_ref()[..8]);
    println!("Shared secret from decapsulation: {:02x?}", &ss_dec1.as_ref()[..8]);
    
    if ss1.as_ref() == ss_dec1.as_ref() {
        println!("\n✅ SUCCESS: Kyber is working correctly!");
    } else {
        println!("\n❌ FAILURE: Basic Kyber operations are broken");
        println!("This indicates a fundamental issue in the implementation");
    }
}

#[test]
fn test_multiple_encapsulations() {
    println!("\n=== Testing Multiple Encapsulations ===");
    
    let mut rng = TestRng::new(42);
    let (pk, sk) = Kyber512::generate_keypair(&mut rng).unwrap();
    
    let mut successes = 0;
    let mut failures = 0;
    
    for i in 0..10 {
        let mut rng_enc = TestRng::new(1000 + i);
        let (ct, ss_enc) = Kyber512::encapsulate(&pk, &mut rng_enc).unwrap();
        let ss_dec = Kyber512::decapsulate(&sk, &ct).unwrap();
        
        if ss_enc.as_ref() == ss_dec.as_ref() {
            successes += 1;
        } else {
            failures += 1;
            println!("Iteration {i}: MISMATCH");
            println!("  Enc: {:02x?}", &ss_enc.as_ref()[..8]);
            println!("  Dec: {:02x?}", &ss_dec.as_ref()[..8]);
        }
    }
    
    println!("\nResults: {successes} successes, {failures} failures");
    
    if failures > 0 {
        println!("❌ Kyber has a systematic error");
    } else {
        println!("✅ All encapsulations succeeded");
    }
}