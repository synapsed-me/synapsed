//! Minimal test to verify Kyber fix

use synapsed_crypto::{
    kyber::Kyber512,
    traits::Kem,
    random::TestRng,
};

#[test]
fn test_kyber512_fix() {
    println!("\n=== Testing Kyber512 Fix ===");
    
    // Print constants
    println!("PUBLIC_KEY_SIZE: {}", Kyber512::PUBLIC_KEY_SIZE);
    println!("SECRET_KEY_SIZE: {}", Kyber512::SECRET_KEY_SIZE);
    println!("Expected SECRET_KEY_SIZE: 1632");
    
    let mut rng = TestRng::new(42);
    
    // Generate keypair
    let (pk, sk) = match Kyber512::generate_keypair(&mut rng) {
        Ok(kp) => kp,
        Err(e) => {
            panic!("Keypair generation failed: {e:?}");
        }
    };
    
    println!("✓ Keypair generated successfully");
    println!("  PK size: {}", pk.as_ref().len());
    println!("  SK size: {}", sk.as_ref().len());
    
    // Create fresh RNG for encapsulation
    let mut rng2 = TestRng::new(123);
    
    // Encapsulate
    let (ct, ss1) = match Kyber512::encapsulate(&pk, &mut rng2) {
        Ok(r) => r,
        Err(e) => {
            panic!("Encapsulation failed: {e:?}");
        }
    };
    
    println!("✓ Encapsulation successful");
    
    // Decapsulate
    let ss2 = match Kyber512::decapsulate(&sk, &ct) {
        Ok(ss) => ss,
        Err(e) => {
            panic!("Decapsulation failed: {e:?}");
        }
    };
    
    println!("✓ Decapsulation successful");
    
    // Check shared secrets
    let ss1_bytes = ss1.as_ref();
    let ss2_bytes = ss2.as_ref();
    
    if ss1_bytes == ss2_bytes {
        println!("\n✅ SUCCESS: Shared secrets match!");
        println!("   The Kyber bug has been fixed!");
    } else {
        println!("\n❌ FAILURE: Shared secrets don't match!");
        println!("   SS1: {:02x?}", &ss1_bytes[..8]);
        println!("   SS2: {:02x?}", &ss2_bytes[..8]);
        panic!("Kyber shared secret mismatch");
    }
}