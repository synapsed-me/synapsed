//! Basic encryption example using Kyber key encapsulation
//!
//! This example demonstrates how to:
//! - Generate Kyber keypairs
//! - Encapsulate a shared secret
//! - Use the shared secret for symmetric encryption
//! - Decapsulate and decrypt

use synapsed_crypto::prelude::*;
use synapsed_crypto::random::DefaultRng;
use synapsed_crypto::traits::Serializable;
use sha3::{Shake256, digest::{ExtendableOutput, Update}};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== Synapsed Crypto: Basic Encryption Example ===\n");
    
    // Step 1: Generate a Kyber keypair
    println!("1. Generating Kyber768 keypair...");
    let mut rng = DefaultRng::default();
    let (public_key, secret_key) = Kyber768::generate_keypair(&mut rng)?;
    println!("   ✓ Public key size: {} bytes", public_key.to_bytes().len());
    println!("   ✓ Secret key size: {} bytes", secret_key.to_bytes().len());
    
    // Step 2: Encapsulate a shared secret
    println!("\n2. Encapsulating shared secret...");
    let (ciphertext, shared_secret_sender) = Kyber768::encapsulate(&public_key, &mut rng)?;
    println!("   ✓ Ciphertext size: {} bytes", ciphertext.to_bytes().len());
    println!("   ✓ Shared secret: {} bytes", shared_secret_sender.as_ref().len());
    
    // Step 3: Simulate sending the ciphertext
    println!("\n3. Sending ciphertext to recipient...");
    let transmitted_ciphertext = ciphertext.clone();
    
    // Step 4: Recipient decapsulates the shared secret
    println!("\n4. Recipient decapsulating shared secret...");
    let shared_secret_recipient = Kyber768::decapsulate(&secret_key, &transmitted_ciphertext)?;
    println!("   ✓ Shared secret recovered: {} bytes", shared_secret_recipient.as_ref().len());
    
    // Step 5: Verify both parties have the same shared secret
    println!("\n5. Verifying shared secrets match...");
    assert_eq!(shared_secret_sender.as_ref(), shared_secret_recipient.as_ref());
    println!("   ✓ Success! Both parties have the same shared secret");
    
    // Step 6: Use the shared secret for symmetric encryption
    println!("\n6. Using shared secret for symmetric encryption...");
    let message = b"Hello, Post-Quantum World!";
    let encrypted = simple_xor_encrypt(message, shared_secret_sender.as_ref());
    println!("   ✓ Encrypted message: {} bytes", encrypted.len());
    
    // Step 7: Decrypt the message
    println!("\n7. Decrypting message...");
    let decrypted = simple_xor_encrypt(&encrypted, shared_secret_recipient.as_ref());
    let decrypted_str = String::from_utf8_lossy(&decrypted);
    println!("   ✓ Decrypted message: \"{decrypted_str}\"");
    
    // Step 8: Demonstrate different security levels
    println!("\n8. Available security levels:");
    demonstrate_security_levels()?;
    
    println!("\n=== Example completed successfully! ===");
    Ok(())
}

// Simple XOR encryption for demonstration (NOT for production use!)
fn simple_xor_encrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    // Expand key using SHAKE256 to match data length
    let mut hasher = Shake256::default();
    hasher.update(key);
    let mut reader = hasher.finalize_xof();
    let mut expanded_key = vec![0u8; data.len()];
    sha3::digest::XofReader::read(&mut reader, &mut expanded_key);
    
    data.iter()
        .zip(expanded_key.iter())
        .map(|(d, k)| d ^ k)
        .collect()
}

fn demonstrate_security_levels() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut rng = DefaultRng::default();
    
    // Kyber512 - NIST Level 1 (AES-128 equivalent)
    let (pk512, _) = Kyber512::generate_keypair(&mut rng)?;
    println!("   • Kyber512 (Level 1): {} bytes public key", pk512.to_bytes().len());
    
    // Kyber768 - NIST Level 3 (AES-192 equivalent)
    let (pk768, _) = Kyber768::generate_keypair(&mut rng)?;
    println!("   • Kyber768 (Level 3): {} bytes public key", pk768.to_bytes().len());
    
    // Kyber1024 - NIST Level 5 (AES-256 equivalent)
    let (pk1024, _) = Kyber1024::generate_keypair(&mut rng)?;
    println!("   • Kyber1024 (Level 5): {} bytes public key", pk1024.to_bytes().len());
    
    Ok(())
}

// Example output:
//
// === Synapsed Crypto: Basic Encryption Example ===
//
// 1. Generating Kyber768 keypair...
//    ✓ Public key size: 1184 bytes
//    ✓ Secret key size: 2400 bytes
//
// 2. Encapsulating shared secret...
//    ✓ Ciphertext size: 1088 bytes
//    ✓ Shared secret: 32 bytes
//
// 3. Sending ciphertext to recipient...
//
// 4. Recipient decapsulating shared secret...
//    ✓ Shared secret recovered: 32 bytes
//
// 5. Verifying shared secrets match...
//    ✓ Success! Both parties have the same shared secret
//
// 6. Using shared secret for symmetric encryption...
//    ✓ Encrypted message: 26 bytes
//
// 7. Decrypting message...
//    ✓ Decrypted message: "Hello, Post-Quantum World!"
//
// 8. Available security levels:
//    • Kyber512 (Level 1): 800 bytes public key
//    • Kyber768 (Level 3): 1184 bytes public key
//    • Kyber1024 (Level 5): 1568 bytes public key
//
// === Example completed successfully! ===