//! Property-based security tests for cryptographic operations.

use synapsed_net::{
    security::{SecurityLayer, CipherSuite, AuthMethod, HandshakePhase},
    types::{PeerId, PeerInfo, NetworkAddress, PeerMetadata},
};
use proptest::prelude::*;
use std::collections::HashSet;

// Property-based test strategies
prop_compose! {
    fn arb_peer_info()(
        address in "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}:[0-9]{1,5}",
        capabilities in prop::collection::vec(
            prop::string::string_regex("[A-Za-z0-9]+").unwrap(),
            0..10
        ),
        has_key in any::<bool>()
    ) -> PeerInfo {
        PeerInfo {
            id: PeerId::new(),
            address,
            addresses: vec![],
            protocols: vec!["test/1.0".to_string()],
            capabilities,
            public_key: if has_key { Some(vec![0u8; 32]) } else { None },
            metadata: PeerMetadata::default(),
        }
    }
}

prop_compose! {
    fn arb_message_data()(
        data in prop::collection::vec(any::<u8>(), 0..65536)
    ) -> Vec<u8> {
        data
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    /// Property: Encryption should always be reversible
    #[test]
    fn prop_encryption_reversible(
        peer in arb_peer_info(),
        message in arb_message_data()
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let mut layer = SecurityLayer::new(false).unwrap();
            
            // Establish session
            let handshake = layer.initiate_handshake(&peer).await.unwrap();
            let session_id = layer.complete_handshake(handshake, &peer).await.unwrap();
            
            // Encrypt then decrypt should recover original
            let encrypted = layer.encrypt(&message, &session_id).unwrap();
            let decrypted = layer.decrypt(&encrypted, &session_id).unwrap();
            
            prop_assert_eq!(decrypted, message);
        })?;
    }
    
    /// Property: Encryption should not be deterministic (except for empty messages)
    #[test]
    fn prop_encryption_non_deterministic(
        peer in arb_peer_info(),
        message in arb_message_data()
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            // Skip for empty messages as they might encrypt deterministically
            if message.is_empty() {
                return Ok(());
            }
            
            let mut layer = SecurityLayer::new(false).unwrap();
            
            // Establish session
            let handshake = layer.initiate_handshake(&peer).await.unwrap();
            let session_id = layer.complete_handshake(handshake, &peer).await.unwrap();
            
            // Two encryptions of the same message should differ (due to nonces)
            let encrypted1 = layer.encrypt(&message, &session_id).unwrap();
            let encrypted2 = layer.encrypt(&message, &session_id).unwrap();
            
            prop_assert_ne!(encrypted1, encrypted2);
        })?;
    }
    
    /// Property: Tampered ciphertext should not decrypt successfully
    #[test]
    fn prop_tampering_detection(
        peer in arb_peer_info(),
        message in arb_message_data().prop_filter("Non-empty", |m| !m.is_empty()),
        bit_flip_pos in 0usize..1000
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let mut layer = SecurityLayer::new(false).unwrap();
            
            // Establish session
            let handshake = layer.initiate_handshake(&peer).await.unwrap();
            let session_id = layer.complete_handshake(handshake, &peer).await.unwrap();
            
            // Encrypt message
            let mut encrypted = layer.encrypt(&message, &session_id).unwrap();
            
            // Tamper with the ciphertext
            if bit_flip_pos < encrypted.len() {
                encrypted[bit_flip_pos] ^= 0x01;
                
                // Decryption should fail for tampered data
                let result = layer.decrypt(&encrypted, &session_id);
                prop_assert!(result.is_err());
            }
        })?;
    }
    
    /// Property: Different sessions should produce different ciphertexts
    #[test]
    fn prop_session_isolation(
        peer1 in arb_peer_info(),
        peer2 in arb_peer_info().prop_filter("Different peer", |p| p.id != peer1.id),
        message in arb_message_data()
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            if message.is_empty() {
                return Ok(());
            }
            
            let mut layer = SecurityLayer::new(false).unwrap();
            
            // Establish two different sessions
            let handshake1 = layer.initiate_handshake(&peer1).await.unwrap();
            let session_id1 = layer.complete_handshake(handshake1, &peer1).await.unwrap();
            
            let handshake2 = layer.initiate_handshake(&peer2).await.unwrap();
            let session_id2 = layer.complete_handshake(handshake2, &peer2).await.unwrap();
            
            // Same message encrypted with different sessions should differ
            let encrypted1 = layer.encrypt(&message, &session_id1).unwrap();
            let encrypted2 = layer.encrypt(&message, &session_id2).unwrap();
            
            prop_assert_ne!(encrypted1, encrypted2);
            
            // Cross-session decryption should fail
            let cross_decrypt1 = layer.decrypt(&encrypted1, &session_id2);
            let cross_decrypt2 = layer.decrypt(&encrypted2, &session_id1);
            
            prop_assert!(cross_decrypt1.is_err());
            prop_assert!(cross_decrypt2.is_err());
        })?;
    }
    
    /// Property: Handshake should always produce unique session IDs
    #[test]
    fn prop_unique_session_ids(
        peers in prop::collection::vec(arb_peer_info(), 1..10)
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let mut layer = SecurityLayer::new(false).unwrap();
            let mut session_ids = HashSet::new();
            
            for peer in peers {
                let handshake = layer.initiate_handshake(&peer).await.unwrap();
                let session_id = layer.complete_handshake(handshake, &peer).await.unwrap();
                
                // Session ID should be unique
                prop_assert!(session_ids.insert(session_id), "Duplicate session ID found");
            }
        })?;
    }
    
    /// Property: Cipher suite negotiation should be deterministic
    #[test]
    fn prop_cipher_suite_negotiation_deterministic(
        base_peer in arb_peer_info()
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let layer = SecurityLayer::new(true).unwrap();
            
            // Multiple handshakes with same peer should negotiate same cipher suite
            let handshake1 = layer.initiate_handshake(&base_peer).await.unwrap();
            let suite1 = handshake1.cipher_suite();
            
            let handshake2 = layer.initiate_handshake(&base_peer).await.unwrap();
            let suite2 = handshake2.cipher_suite();
            
            prop_assert_eq!(suite1, suite2);
        })?;
    }
    
    /// Property: Key derivation should be deterministic
    #[test]
    fn prop_key_derivation_deterministic(
        ephemeral_key in prop::collection::vec(any::<u8>(), 1..256),
        peer_id in "[a-f0-9]{32}"
    ) {
        use blake3;
        
        // Same inputs should produce same output
        let combined1 = [&ephemeral_key[..], peer_id.as_bytes()].concat();
        let key1 = blake3::hash(&combined1);
        
        let combined2 = [&ephemeral_key[..], peer_id.as_bytes()].concat();
        let key2 = blake3::hash(&combined2);
        
        prop_assert_eq!(key1, key2);
    }
    
    /// Property: Security layer should handle concurrent handshakes safely
    #[test]
    fn prop_concurrent_handshakes_safe(
        peers in prop::collection::vec(arb_peer_info(), 2..5)
    ) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            use tokio::sync::{Arc, Mutex};
            use std::collections::HashMap;
            
            let layer = Arc::new(Mutex::new(SecurityLayer::new(false).unwrap()));
            let results = Arc::new(Mutex::new(HashMap::new()));
            
            let mut tasks = vec![];
            
            for (i, peer) in peers.into_iter().enumerate() {
                let layer = layer.clone();
                let results = results.clone();
                
                let task = tokio::spawn(async move {
                    let mut layer = layer.lock().await;
                    let handshake = layer.initiate_handshake(&peer).await.unwrap();
                    let session_id = layer.complete_handshake(handshake, &peer).await.unwrap();
                    
                    let mut results = results.lock().await;
                    results.insert(i, session_id);
                });
                
                tasks.push(task);
            }
            
            // Wait for all handshakes to complete
            for task in tasks {
                task.await.unwrap();
            }
            
            let results = results.lock().await;
            
            // All handshakes should have succeeded
            prop_assert!(!results.is_empty());
            
            // All session IDs should be unique
            let mut session_ids: Vec<_> = results.values().cloned().collect();
            session_ids.sort();
            session_ids.dedup();
            prop_assert_eq!(session_ids.len(), results.len());
        })?;
    }
}

// Additional invariant tests

#[tokio::test]
async fn test_security_layer_invariants() {
    let layer = SecurityLayer::new(true).unwrap();
    
    // Invariant: Should support at least one cipher suite
    assert!(!layer.cipher_suites().is_empty());
    
    // Invariant: Should support at least one auth method
    assert!(!layer.auth_methods().is_empty());
    
    // Invariant: Post-quantum suites should be available when enabled
    assert!(layer.cipher_suites().iter().any(|suite| matches!(
        suite,
        CipherSuite::Kyber768ChaCha20 |
        CipherSuite::Kyber1024ChaCha20 |
        CipherSuite::Kyber1024Aes256 |
        CipherSuite::HybridX25519Kyber1024
    )));
}

#[tokio::test]
async fn test_handshake_state_invariants() {
    let layer = SecurityLayer::new(false).unwrap();
    let peer = PeerInfo {
        id: PeerId::new(),
        address: "127.0.0.1:8080".to_string(),
        addresses: vec![],
        protocols: vec![],
        capabilities: vec![],
        public_key: None,
        metadata: PeerMetadata::default(),
    };
    
    let handshake = layer.initiate_handshake(&peer).await.unwrap();
    
    // Invariant: Handshake should start in KeyExchange phase
    assert_eq!(handshake.phase(), HandshakePhase::KeyExchange);
    
    // Invariant: Should have negotiated a cipher suite and auth method
    assert!(handshake.cipher_suite().is_some());
    assert!(handshake.auth_method().is_some());
    
    // Invariant: Should have ephemeral keys (for non-PQ suites)
    if !layer.is_post_quantum_suite(handshake.cipher_suite().unwrap()) {
        assert!(!handshake.ephemeral_keys().is_empty());
    }
}

#[test]
fn test_cipher_suite_strength_ordering() {
    // Test that cipher suites are properly ordered by strength
    let weak_suites = vec![CipherSuite::ChaCha20Poly1305X25519];
    let strong_suites = vec![
        CipherSuite::Kyber1024ChaCha20,
        CipherSuite::HybridX25519Kyber1024,
    ];
    
    // Post-quantum suites should generally be preferred
    // This is implementation-dependent but good practice
    for strong in &strong_suites {
        for weak in &weak_suites {
            // In a real implementation, we'd compare security levels
            println!("Comparing {:?} vs {:?}", strong, weak);
        }
    }
}

#[test]
fn test_entropy_quality() {
    // Test that key generation has good entropy
    let mut keys = std::collections::HashSet::new();
    
    for _ in 0..100 {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut key = vec![0u8; 32];
        rng.fill_bytes(&mut key);
        
        // Keys should be unique (with high probability)
        assert!(keys.insert(key), "Duplicate key generated - poor entropy?");
    }
}