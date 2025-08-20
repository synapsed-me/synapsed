//! Detailed debugging of Kyber to understand the mismatch

use synapsed_crypto::{
    kyber::Kyber512,
    traits::Kem,
    random::TestRng,
};

#[test]
fn debug_kyber_detailed() {
    println!("\n=== Detailed Kyber512 Debug ===");
    
    let mut rng = TestRng::new(42);
    
    // Generate keypair
    let (pk, sk) = Kyber512::generate_keypair(&mut rng).unwrap();
    println!("✓ Keypair generated");
    
    // Extract z from secret key for inspection
    let sk_bytes = sk.as_ref();
    let z = &sk_bytes[1600..]; // Last 32 bytes
    println!("\nOriginal z from keygen: {:02x?}", &z[..8]);
    
    // Create fresh RNG for encapsulation
    let mut rng2 = TestRng::new(123);
    
    // Encapsulate
    let (ct, ss1) = Kyber512::encapsulate(&pk, &mut rng2).unwrap();
    println!("\n✓ Encapsulation complete");
    println!("SS1: {:02x?}", &ss1.as_ref()[..8]);
    
    // Decapsulate
    let ss2 = Kyber512::decapsulate(&sk, &ct).unwrap();
    println!("\n✓ Decapsulation complete");
    println!("SS2: {:02x?}", &ss2.as_ref()[..8]);
    
    // Check
    if ss1.as_ref() == ss2.as_ref() {
        println!("\n✅ SUCCESS: Shared secrets match!");
    } else {
        println!("\n❌ FAILURE: Shared secrets don't match!");
        
        // Debug: Let's manually check what should happen
        // In the FO transform, if re-encryption matches, we use the decapsulated key
        // If not, we use z as the shared secret seed
        println!("\nThis suggests the re-encryption check is failing");
        println!("When re-encryption fails, Kyber uses z to derive the shared secret");
        println!("This is the implicit rejection mechanism for CCA security");
    }
}