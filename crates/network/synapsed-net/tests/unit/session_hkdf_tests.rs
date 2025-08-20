//! Tests for session HKDF issues with blake3
//! These tests target the trait bound errors in session.rs

use synapsed_net::session::{SessionManager, SessionConfig};
use synapsed_net::error::{NetworkError, SessionError};
use std::time::Duration;

#[tokio::test]
async fn test_hkdf_blake3_trait_bounds() {
    // This test targets the HKDF trait bound issues with blake3
    // Error: the trait bound `blake3::Hasher: sha2::digest::OutputSizeUser` is not satisfied
    
    let config = SessionConfig {
        timeout: Duration::from_secs(300),
        max_sessions: 1000,
        cleanup_interval: Duration::from_secs(60),
    };
    
    let manager = SessionManager::new(config);
    
    // Try to create a session which will trigger HKDF key derivation
    let peer_id = "test_peer".to_string();
    let initial_key = vec![0u8; 32]; // 256-bit key
    
    // This should fail at compile time due to HKDF<blake3::Hasher> trait issues
    let result = manager.create_session(&peer_id, &initial_key).await;
    
    // If it compiles and runs, verify the error
    match result {
        Ok(_) => panic!("Expected session creation to fail with current HKDF issues"),
        Err(e) => {
            // Verify we get a proper error
            println!("Got expected error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_session_key_derivation() {
    // Test the key derivation functionality that's broken
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);
    
    // Test data
    let peer_id = "key_derive_peer".to_string();
    let shared_secret = vec![0x42u8; 32];
    
    // Try to create session with key derivation
    let result = manager.create_session(&peer_id, &shared_secret).await;
    
    // Should fail due to HKDF trait bounds
    assert!(result.is_err());
}

#[tokio::test]
async fn test_session_encryption_with_broken_hkdf() {
    // Test that encryption fails when HKDF is broken
    let config = SessionConfig::default();
    let mut manager = SessionManager::new(config);
    
    // Even if we could create a session, encryption would fail
    let test_data = b"Test encryption data";
    let fake_session_id = uuid::Uuid::new_v4();
    
    // Try to encrypt - should fail
    let encrypt_result = manager.encrypt_message(test_data, &fake_session_id).await;
    assert!(encrypt_result.is_err());
    
    // Try to decrypt - should also fail
    let decrypt_result = manager.decrypt_message(test_data, &fake_session_id).await;
    assert!(decrypt_result.is_err());
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn prop_session_config_invariants(
            timeout_secs in 1u64..3600u64,
            max_sessions in 1usize..10000usize,
            cleanup_secs in 1u64..3600u64,
        ) {
            let config = SessionConfig {
                timeout: Duration::from_secs(timeout_secs),
                max_sessions,
                cleanup_interval: Duration::from_secs(cleanup_secs),
            };
            
            // Timeout should be reasonable
            prop_assert!(config.timeout.as_secs() >= 1);
            prop_assert!(config.timeout.as_secs() <= 3600);
            
            // Max sessions should be reasonable
            prop_assert!(config.max_sessions >= 1);
            prop_assert!(config.max_sessions <= 10000);
            
            // Cleanup interval should be less than or equal to timeout
            prop_assert!(config.cleanup_interval <= config.timeout);
        }
        
        #[test]
        fn prop_key_size_validation(
            key_size in 0usize..1024usize
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let config = SessionConfig::default();
                let manager = SessionManager::new(config);
                
                let peer_id = "test_peer".to_string();
                let key = vec![0u8; key_size];
                
                let result = manager.create_session(&peer_id, &key).await;
                
                // Currently all should fail due to HKDF issues
                prop_assert!(result.is_err());
                Ok(())
            })?;
        }
    }
}

// Integration test for session manager with other components
#[tokio::test]
async fn test_session_manager_integration() {
    // This would test integration with transport and crypto
    // Currently expected to fail due to HKDF issues
    
    let config = SessionConfig::default();
    let manager = SessionManager::new(config);
    
    // Test concurrent session creation
    let mut handles = vec![];
    
    for i in 0..10 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let peer_id = format!("peer_{}", i);
            let key = vec![i as u8; 32];
            manager_clone.create_session(&peer_id, &key).await
        });
        handles.push(handle);
    }
    
    // All should fail with current HKDF issues
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_err());
    }
}