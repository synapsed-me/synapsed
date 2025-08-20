//! Hybrid cryptography example combining classical and post-quantum algorithms
//!
//! This example demonstrates how to:
//! - Use Kyber for post-quantum key encapsulation
//! - Use Dilithium for post-quantum signatures
//! - Ensure security against both classical and quantum attacks

#[cfg(not(feature = "hybrid"))]
fn main() {
    println!("This example requires the 'hybrid' feature to be enabled.");
    println!("Run with: cargo run --example hybrid_mode --features hybrid");
}

#[cfg(feature = "hybrid")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use synapsed_crypto::prelude::*;
    use synapsed_crypto::kyber::Kyber768;
    use synapsed_crypto::dilithium::Dilithium3; 
    use synapsed_crypto::random::TestRng;
    use synapsed_crypto::traits::Serializable;
    use std::time::Instant;
    
    println!("=== Synapsed Crypto: Hybrid Mode Example ===\n");
    println!("Combining classical and post-quantum cryptography for maximum security\n");
    
    // Part 1: Key Encapsulation with Kyber768
    println!("1. Post-Quantum Key Encapsulation (Kyber768)");
    println!("   Protects against both classical and quantum attacks\n");
    
    // Generate KEM keypair
    println!("   a) Generating KEM keypair...");
    let mut rng = TestRng::new(42);
    let start = Instant::now();
    let (kem_pk, kem_sk) = Kyber768::generate_keypair(&mut rng)?;
    let keygen_time = start.elapsed();
    println!("      ✓ Public key size: {} bytes", kem_pk.to_bytes().len());
    println!("      ✓ Secret key size: {} bytes", kem_sk.to_bytes().len());
    println!("      ✓ Generation time: {keygen_time:?}");
    
    // Encapsulate shared secret
    println!("\n   b) Encapsulating shared secret...");
    let start = Instant::now();
    let (ciphertext, shared_secret) = Kyber768::encapsulate(&kem_pk, &mut rng)?;
    let encap_time = start.elapsed();
    println!("      ✓ Ciphertext size: {} bytes", ciphertext.to_bytes().len());
    println!("      ✓ Shared secret: {} bytes", shared_secret.as_ref().len());
    println!("      ✓ Encapsulation time: {encap_time:?}");
    
    // Decapsulate shared secret
    println!("\n   c) Decapsulating shared secret...");
    let start = Instant::now();
    let recovered_secret = Kyber768::decapsulate(&kem_sk, &ciphertext)?;
    let decap_time = start.elapsed();
    assert_eq!(shared_secret.as_ref(), recovered_secret.as_ref());
    println!("      ✓ Secret recovered successfully!");
    println!("      ✓ Decapsulation time: {decap_time:?}");
    
    // Part 2: Digital Signatures with Dilithium3
    println!("\n2. Post-Quantum Digital Signatures (Dilithium3)");
    println!("   Ensures authenticity against both classical and quantum adversaries\n");
    
    // Generate signature keypair
    println!("   a) Generating signature keypair...");
    let start = Instant::now();
    let (sig_pk, sig_sk) = Dilithium3::generate_keypair(&mut rng)?;
    let sig_keygen_time = start.elapsed();
    println!("      ✓ Public key size: {} bytes", sig_pk.to_bytes().len());
    println!("      ✓ Secret key size: {} bytes", sig_sk.to_bytes().len());
    println!("      ✓ Generation time: {sig_keygen_time:?}");
    
    // Sign a message
    let message = b"Critical firmware update v2.1.0 - SHA256: abcd1234...";
    println!("\n   b) Signing message...");
    let start = Instant::now();
    let signature = Dilithium3::sign(&sig_sk, message, &mut rng)?;
    let sign_time = start.elapsed();
    println!("      ✓ Message: {} bytes", message.len());
    println!("      ✓ Signature size: {} bytes", signature.to_bytes().len());
    println!("      ✓ Signing time: {sign_time:?}");
    
    // Verify the signature
    println!("\n   c) Verifying signature...");
    let start = Instant::now();
    Dilithium3::verify(&sig_pk, message, &signature)?;
    let verify_time = start.elapsed();
    println!("      ✓ Signature verified successfully!");
    println!("      ✓ Verification time: {verify_time:?}");
    
    // Part 3: Security Analysis
    println!("\n3. Security Analysis");
    println!("   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("   Component    │ Classical Security │ Quantum Security");
    println!("   ─────────────┼───────────────────┼─────────────────");
    println!("   Kyber768     │ 192-bit           │ 192-bit        ");
    println!("   Dilithium3   │ 192-bit           │ 192-bit        ");
    println!("   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Part 4: Use Case Example
    println!("\n4. Real-World Use Case: Secure Communication Protocol");
    demonstrate_secure_protocol()?;
    
    println!("\n=== Example completed successfully! ===");
    Ok(())
}

#[cfg(feature = "hybrid")]
fn demonstrate_secure_protocol() -> Result<(), Box<dyn std::error::Error>> {
    use synapsed_crypto::prelude::*;
    use synapsed_crypto::kyber::Kyber768;
    use synapsed_crypto::dilithium::Dilithium3;
    use synapsed_crypto::random::TestRng;
    use synapsed_crypto::traits::Serializable;
    
    println!("\n   Simulating secure message exchange between Alice and Bob:");
    
    // Alice and Bob generate their keypairs
    println!("\n   a) Key generation phase");
    let mut rng = TestRng::new(12345);
    let (alice_kem_pk, alice_kem_sk) = Kyber768::generate_keypair(&mut rng)?;
    let (alice_sig_pk, alice_sig_sk) = Dilithium3::generate_keypair(&mut rng)?;
    println!("      ✓ Alice's keys generated");
    
    let (bob_kem_pk, bob_kem_sk) = Kyber768::generate_keypair(&mut rng)?;
    let (bob_sig_pk, bob_sig_sk) = Dilithium3::generate_keypair(&mut rng)?;
    println!("      ✓ Bob's keys generated");
    
    // Alice sends encrypted and signed message to Bob
    println!("\n   b) Alice → Bob: Encrypted and signed message");
    
    // Alice encapsulates shared secret using Bob's public key
    let (ciphertext, alice_shared_secret) = Kyber768::encapsulate(&bob_kem_pk, &mut rng)?;
    
    // Alice encrypts her message
    let alice_message = b"Meet me at the quantum-safe location at 15:00";
    let encrypted_msg = xor_encrypt(alice_message, alice_shared_secret.as_ref());
    
    // Alice signs the ciphertext and encrypted message
    let mut signed_data = Vec::new();
    signed_data.extend_from_slice(&ciphertext.to_bytes());
    signed_data.extend_from_slice(&encrypted_msg);
    let alice_signature = Dilithium3::sign(&alice_sig_sk, &signed_data, &mut rng)?;
    
    println!("      ✓ Message encrypted with Kyber768 KEM");
    println!("      ✓ Signed with Dilithium3 signature");
    
    // Bob receives and processes the message
    println!("\n   c) Bob processes Alice's message");
    
    // Bob verifies Alice's signature
    Dilithium3::verify(&alice_sig_pk, &signed_data, &alice_signature)?;
    println!("      ✓ Alice's signature verified");
    
    // Bob decapsulates the shared secret
    let bob_shared_secret = Kyber768::decapsulate(&bob_kem_sk, &ciphertext)?;
    assert_eq!(alice_shared_secret.as_ref(), bob_shared_secret.as_ref());
    println!("      ✓ Shared secret recovered");
    
    // Bob decrypts the message
    let decrypted_msg = xor_encrypt(&encrypted_msg, bob_shared_secret.as_ref());
    assert_eq!(alice_message, decrypted_msg.as_slice());
    println!("      ✓ Message decrypted: \"{}\"", String::from_utf8_lossy(&decrypted_msg));
    
    // Bob sends a reply
    println!("\n   d) Bob → Alice: Encrypted reply");
    let (reply_ct, bob_reply_secret) = Kyber768::encapsulate(&alice_kem_pk, &mut rng)?;
    let bob_message = b"Confirmed. See you there.";
    let encrypted_reply = xor_encrypt(bob_message, bob_reply_secret.as_ref());
    
    let mut reply_data = Vec::new();
    reply_data.extend_from_slice(&reply_ct.to_bytes());
    reply_data.extend_from_slice(&encrypted_reply);
    let bob_signature = Dilithium3::sign(&bob_sig_sk, &reply_data, &mut rng)?;
    
    // Alice verifies and decrypts Bob's reply
    Dilithium3::verify(&bob_sig_pk, &reply_data, &bob_signature)?;
    let alice_recovered_secret = Kyber768::decapsulate(&alice_kem_sk, &reply_ct)?;
    let decrypted_reply = xor_encrypt(&encrypted_reply, alice_recovered_secret.as_ref());
    
    println!("      ✓ Bob's reply verified and decrypted: \"{}\"", 
             String::from_utf8_lossy(&decrypted_reply));
    
    Ok(())
}

#[cfg(feature = "hybrid")]
fn xor_encrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    use sha3::{Shake256, digest::{ExtendableOutput, Update, XofReader}};
    
    let mut hasher = Shake256::default();
    hasher.update(key);
    let mut reader = hasher.finalize_xof();
    let mut expanded_key = vec![0u8; data.len()];
    reader.read(&mut expanded_key);
    
    data.iter()
        .zip(expanded_key.iter())
        .map(|(d, k)| d ^ k)
        .collect()
}