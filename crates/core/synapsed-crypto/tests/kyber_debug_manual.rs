//! Manual debug test for Kyber to isolate the issue

use synapsed_crypto::{
    kyber::Kyber512,
    traits::{Kem, Serializable, SecureRandom},
    random::TestRng,
};

#[test]
fn debug_kyber512_step_by_step() {
    let mut rng = TestRng::new(42);
    
    println!("=== Kyber512 Debug Test ===");
    
    // Generate keypair
    let (pk, sk) = Kyber512::generate_keypair(&mut rng).unwrap();
    println!("✓ Generated keypair");
    println!("  Public key size: {}", pk.to_bytes().len());
    println!("  Secret key size: {}", sk.to_bytes().len());
    
    // Reset RNG to ensure deterministic behavior  
    let mut rng2 = TestRng::new(42); // Fresh RNG for encapsulation
    rng2.fill_bytes(&mut [0u8; 100]); // Advance to post-keygen state
    
    // Encapsulate
    let (ct, ss1) = Kyber512::encapsulate(&pk, &mut rng2).unwrap();
    println!("✓ Encapsulation complete");
    println!("  Ciphertext size: {}", ct.to_bytes().len());
    println!("  Shared secret 1: {:02x?}", &ss1.bytes[..8]);
    
    // Decapsulate
    let ss2 = Kyber512::decapsulate(&sk, &ct).unwrap();
    println!("✓ Decapsulation complete");
    println!("  Shared secret 2: {:02x?}", &ss2.bytes[..8]);
    
    // Check if they match
    let matches = ss1.bytes == ss2.bytes;
    println!("✓ Shared secrets match: {matches}");
    
    if !matches {
        println!("❌ MISMATCH DETECTED");
        println!("  SS1 full: {:02x?}", ss1.bytes);
        println!("  SS2 full: {:02x?}", ss2.bytes);
        
        // Compare byte by byte
        for (i, (b1, b2)) in ss1.bytes.iter().zip(ss2.bytes.iter()).enumerate() {
            if b1 != b2 {
                println!("  Differ at byte {i}: {b1:02x} vs {b2:02x}");
            }
        }
    }
    
    assert_eq!(ss1.bytes, ss2.bytes, "Shared secrets must match!");
}