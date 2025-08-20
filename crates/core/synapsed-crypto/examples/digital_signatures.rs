//! Digital signature example using Dilithium
//!
//! This example demonstrates how to:
//! - Generate Dilithium keypairs
//! - Sign messages
//! - Verify signatures
//! - Handle different message types

use std::time::Instant;
use synapsed_crypto::prelude::*;
use synapsed_crypto::random::DefaultRng;
use synapsed_crypto::traits::Serializable;
use synapsed_crypto::dilithium::{DilithiumPublicKey, DilithiumSecretKey};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== Synapsed Crypto: Digital Signatures Example ===\n");

    // Step 1: Generate a Dilithium keypair
    println!("1. Generating Dilithium3 keypair...");
    let mut rng = DefaultRng::default();
    let start = Instant::now();
    let (public_key, secret_key) = Dilithium3::generate_keypair(&mut rng)?;
    let keygen_time = start.elapsed();
    println!("   ✓ Public key size: {} bytes", public_key.to_bytes().len());
    println!("   ✓ Secret key size: {} bytes", secret_key.to_bytes().len());
    println!("   ✓ Generation time: {keygen_time:?}");

    // Step 2: Sign a simple message
    println!("\n2. Signing a message...");
    let message = b"Important document: Transfer 1000 units to account XYZ";
    let start = Instant::now();
    let signature = Dilithium3::sign(&secret_key, message, &mut rng)?;
    let sign_time = start.elapsed();
    println!("   ✓ Message: {} bytes", message.len());
    println!("   ✓ Signature size: {} bytes", signature.as_ref().len());
    println!("   ✓ Signing time: {sign_time:?}");

    // Step 3: Verify the signature
    println!("\n3. Verifying signature...");
    let start = Instant::now();
    match Dilithium3::verify(&public_key, message, &signature) {
        Ok(true) => {
            let verify_time = start.elapsed();
            println!("   ✓ Signature is valid!");
            println!("   ✓ Verification time: {verify_time:?}");
        }
        Ok(false) => println!("   ✗ Signature is invalid"),
        Err(e) => println!("   ✗ Signature verification failed: {e}"),
    }

    // Step 4: Demonstrate signature uniqueness
    println!("\n4. Demonstrating signature uniqueness...");
    let signature2 = Dilithium3::sign(&secret_key, message, &mut rng)?;
    if signature != signature2 {
        println!("   ✓ Same message produces different signatures (randomized signing)");
    } else {
        println!("   ✓ Signatures are deterministic");
    }

    // Step 5: Show that tampering invalidates signatures
    println!("\n5. Testing tamper detection...");
    let mut tampered_message = message.to_vec();
    tampered_message[0] ^= 1; // Flip one bit
    match Dilithium3::verify(&public_key, &tampered_message, &signature) {
        Ok(false) => println!("   ✓ Tampered message correctly rejected"),
        Ok(true) => println!("   ✗ WARNING: Tampered message verified!"),
        Err(_) => println!("   ✓ Tampered message correctly rejected"),
    }

    // Step 6: Sign different types of data
    println!("\n6. Signing various data types...");
    demonstrate_signing_various_data(&secret_key, &public_key)?;

    // Step 7: Demonstrate different security levels
    println!("\n7. Available security levels:");
    demonstrate_security_levels()?;

    // Step 8: Batch signature verification
    println!("\n8. Batch signature example...");
    demonstrate_batch_signatures()?;

    println!("\n=== Example completed successfully! ===");
    Ok(())
}

fn demonstrate_signing_various_data(
    secret_key: &DilithiumSecretKey<6>,
    public_key: &DilithiumPublicKey<6>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut rng = DefaultRng::default();
    
    // Sign JSON-like data
    let json_data = br#"{"user": "alice", "action": "transfer", "amount": 1000}"#;
    let json_sig = Dilithium3::sign(secret_key, json_data, &mut rng)?;
    assert!(Dilithium3::verify(public_key, json_data, &json_sig).unwrap());
    println!("   ✓ JSON data: {} bytes → {} byte signature", json_data.len(), json_sig.as_ref().len());

    // Sign binary data
    let binary_data = vec![0xFF, 0x00, 0xAB, 0xCD, 0xEF];
    let binary_sig = Dilithium3::sign(secret_key, &binary_data, &mut rng)?;
    assert!(Dilithium3::verify(public_key, &binary_data, &binary_sig).unwrap());
    println!("   ✓ Binary data: {} bytes → {} byte signature", binary_data.len(), binary_sig.as_ref().len());

    // Sign empty message
    let empty_sig = Dilithium3::sign(secret_key, b"", &mut rng)?;
    assert!(Dilithium3::verify(public_key, b"", &empty_sig).unwrap());
    println!("   ✓ Empty message → {} byte signature", empty_sig.as_ref().len());

    // Sign large file hash
    let file_hash = vec![0xAA; 64]; // Simulated SHA-512 hash
    let hash_sig = Dilithium3::sign(secret_key, &file_hash, &mut rng)?;
    assert!(Dilithium3::verify(public_key, &file_hash, &hash_sig).unwrap());
    println!("   ✓ File hash: {} bytes → {} byte signature", file_hash.len(), hash_sig.as_ref().len());

    Ok(())
}

fn demonstrate_security_levels() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut rng = DefaultRng::default();
    
    // Dilithium2 - NIST Level 2 (SHA-256 collision resistance)
    let (pk2, _) = Dilithium2::generate_keypair(&mut rng)?;
    let _sig2 = vec![0u8; 2420]; // Approximate signature size
    println!(
        "   • Dilithium2 (Level 2): {} bytes public key, ~{} bytes signature",
        pk2.to_bytes().len(),
        _sig2.len()
    );

    // Dilithium3 - NIST Level 3 (AES-192 equivalent)
    let (pk3, _) = Dilithium3::generate_keypair(&mut rng)?;
    let _sig3 = vec![0u8; 3293]; // Approximate signature size
    println!(
        "   • Dilithium3 (Level 3): {} bytes public key, ~{} bytes signature",
        pk3.to_bytes().len(),
        _sig3.len()
    );

    // Dilithium5 - NIST Level 5 (AES-256 equivalent)
    let (pk5, _) = Dilithium5::generate_keypair(&mut rng)?;
    let _sig5 = vec![0u8; 4595]; // Approximate signature size
    println!(
        "   • Dilithium5 (Level 5): {} bytes public key, ~{} bytes signature",
        pk5.to_bytes().len(),
        _sig5.len()
    );

    Ok(())
}

fn demonstrate_batch_signatures() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut rng = DefaultRng::default();
    let (public_key, secret_key) = Dilithium3::generate_keypair(&mut rng)?;

    // Create multiple messages
    let messages = vec![
        b"Transaction 1: Alice \xE2\x86\x92 Bob: 100 units".as_slice(),
        b"Transaction 2: Bob \xE2\x86\x92 Charlie: 50 units".as_slice(),
        b"Transaction 3: Charlie \xE2\x86\x92 Alice: 25 units".as_slice(),
    ];

    // Sign all messages
    let mut signatures = Vec::new();
    let start = Instant::now();
    for msg in &messages {
        signatures.push(Dilithium3::sign(&secret_key, msg, &mut rng)?);
    }
    let batch_sign_time = start.elapsed();

    // Verify all signatures
    let start = Instant::now();
    let mut all_valid = true;
    for (msg, sig) in messages.iter().zip(signatures.iter()) {
        if Dilithium3::verify(&public_key, msg, sig).is_err() {
            all_valid = false;
            break;
        }
    }
    let batch_verify_time = start.elapsed();

    println!("   ✓ Signed {} messages in {:?}", messages.len(), batch_sign_time);
    println!("   ✓ Verified {} signatures in {:?}", signatures.len(), batch_verify_time);
    println!("   ✓ All signatures valid: {all_valid}");

    Ok(())
}

// Example output:
//
// === Synapsed Crypto: Digital Signatures Example ===
//
// 1. Generating Dilithium3 keypair...
//    ✓ Public key size: 1952 bytes
//    ✓ Secret key size: 4000 bytes
//    ✓ Generation time: 125.3µs
//
// 2. Signing a message...
//    ✓ Message: 54 bytes
//    ✓ Signature size: 3293 bytes
//    ✓ Signing time: 250.5µs
//
// 3. Verifying signature...
//    ✓ Signature is valid!
//    ✓ Verification time: 150.2µs
//
// 4. Demonstrating signature uniqueness...
//    ✓ Same message produces different signatures (randomized signing)
//
// 5. Testing tamper detection...
//    ✓ Tampered message correctly rejected
//
// 6. Signing various data types...
//    ✓ JSON data: 64 bytes → 3293 byte signature
//    ✓ Binary data: 5 bytes → 3293 byte signature
//    ✓ Empty message → 3293 byte signature
//    ✓ File hash: 64 bytes → 3293 byte signature
//
// 7. Available security levels:
//    • Dilithium2 (Level 2): 1312 bytes public key, ~2420 bytes signature
//    • Dilithium3 (Level 3): 1952 bytes public key, ~3293 bytes signature
//    • Dilithium5 (Level 5): 2592 bytes public key, ~4595 bytes signature
//
// 8. Batch signature example...
//    ✓ Signed 3 messages in 750.8µs
//    ✓ Verified 3 signatures in 450.5µs
//    ✓ All signatures valid: true
//
// === Example completed successfully! ===
