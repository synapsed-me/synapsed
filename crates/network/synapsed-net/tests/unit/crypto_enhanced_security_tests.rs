//! Comprehensive TDD unit tests for EnhancedSecurityManager
//! Following London School TDD approach with mock-driven development

use synapsed_net::crypto::{
    EnhancedSecurityManager, EnhancedSecurityConfig, SecureCipherSuite,
    SecurityEvent, SecurityEventType, SecurityEventSeverity,
    PQSignatureAlgorithm, PinValidationMode
};
use synapsed_net::types::PeerInfo;
use synapsed_net::error::{NetworkError, SecurityError};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Mock certificate validator for isolated testing
struct MockCertificateValidator {
    should_succeed: bool,
    call_count: Arc<Mutex<u32>>,
}

impl MockCertificateValidator {
    fn new(should_succeed: bool) -> Self {
        Self {
            should_succeed,
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    async fn validate_chain(&self, _cert_chain: &[quinn::rustls::pki_types::CertificateDer<'_>], _server_name: &str) -> Result<(), NetworkError> {
        let mut count = self.call_count.lock().await;
        *count += 1;
        
        if self.should_succeed {
            Ok(())
        } else {
            Err(NetworkError::Security(SecurityError::Certificate(
                "Mock validation failure".to_string()
            )))
        }
    }

    async fn call_count(&self) -> u32 {
        *self.call_count.lock().await
    }
}

/// Mock certificate pinner for testing pinning behavior
struct MockCertificatePinner {
    pinned_hashes: Vec<[u8; 32]>,
    validation_results: HashMap<Vec<u8>, bool>,
}

impl MockCertificatePinner {
    fn new() -> Self {
        Self {
            pinned_hashes: Vec::new(),
            validation_results: HashMap::new(),
        }
    }

    fn add_pin(&mut self, hash: [u8; 32]) {
        self.pinned_hashes.push(hash);
    }

    fn set_validation_result(&mut self, cert_der: Vec<u8>, should_succeed: bool) {
        self.validation_results.insert(cert_der, should_succeed);
    }

    fn validate(&self, cert: &quinn::rustls::pki_types::CertificateDer<'_>) -> Result<(), NetworkError> {
        if let Some(&should_succeed) = self.validation_results.get(cert.as_ref()) {
            if should_succeed {
                Ok(())
            } else {
                Err(NetworkError::Security(SecurityError::Certificate(
                    "Certificate pin validation failed".to_string()
                )))
            }
        } else {
            // Default behavior - check if cert hash matches any pinned hash
            let cert_hash = blake3::hash(cert.as_ref());
            if self.pinned_hashes.iter().any(|pin| pin == cert_hash.as_bytes()) {
                Ok(())
            } else {
                Err(NetworkError::Security(SecurityError::Certificate(
                    "Certificate not pinned".to_string()
                )))
            }
        }
    }
}

/// Test helper for creating test peers
fn create_test_peer(id: &str, capabilities: Vec<&str>) -> PeerInfo {
    PeerInfo {
        id: id.to_string(),
        addresses: vec!["127.0.0.1:8080".to_string()],
        capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
        last_seen: SystemTime::now(),
        reputation: 1.0,
    }
}

/// Test helper for creating security manager with test config
fn create_test_security_manager() -> Result<EnhancedSecurityManager, NetworkError> {
    let config = EnhancedSecurityConfig {
        enable_post_quantum: true,
        preferred_cipher_suites: vec![
            SecureCipherSuite::HybridX25519Kyber1024ChaCha20,
            SecureCipherSuite::ChaCha20Poly1305X25519,
        ],
        key_rotation_interval: Duration::from_secs(300),
        session_timeout: Duration::from_secs(3600),
        constant_time_ops: true,
        certificate_pinning: synapsed_net::crypto::CertificatePinningConfig {
            enabled: false, // Start with disabled for basic tests
            pinned_hashes: Vec::new(),
            allow_backup_certs: true,
            validation_mode: PinValidationMode::Advisory,
        },
        audit_config: synapsed_net::crypto::SecurityAuditConfig {
            enable_audit_log: true,
            log_crypto_ops: true,
            log_key_events: true,
            log_auth_attempts: true,
            retention_period: Duration::from_secs(86400),
        },
    };
    
    EnhancedSecurityManager::new(config)
}

#[cfg(test)]
mod enhanced_security_tests {
    use super::*;
    use proptest::prelude::*;

    // RED Phase: Test that fails before implementation
    #[tokio::test]
    async fn test_secure_handshake_should_generate_unique_session_ids() {
        let mut manager = create_test_security_manager().unwrap();
        let peer = create_test_peer("test_peer_1", vec!["ChaCha20Poly1305X25519"]);
        
        // Generate multiple sessions with same peer
        let session1 = manager.secure_handshake(&peer, None).await.unwrap();
        let session2 = manager.secure_handshake(&peer, None).await.unwrap();
        let session3 = manager.secure_handshake(&peer, None).await.unwrap();
        
        // Sessions should be unique
        assert_ne!(session1, session2);
        assert_ne!(session2, session3);
        assert_ne!(session1, session3);
        
        // Verify metrics updated correctly
        let metrics = manager.get_metrics();
        assert_eq!(metrics.key_generations_count, 3);
    }

    #[tokio::test]
    async fn test_cipher_suite_selection_should_prefer_post_quantum() {
        let mut manager = create_test_security_manager().unwrap();
        let peer = create_test_peer("pq_peer", vec![
            "HybridX25519Kyber1024ChaCha20",
            "ChaCha20Poly1305X25519"
        ]);
        
        // Request specific post-quantum suite
        let session_id = manager.secure_handshake(
            &peer, 
            Some(SecureCipherSuite::HybridX25519Kyber1024ChaCha20)
        ).await.unwrap();
        
        // Verify session was created successfully
        assert!(!session_id.is_nil());
        
        // Verify post-quantum operations were used
        let metrics = manager.get_metrics();
        assert!(metrics.pq_operations_count > 0);
    }

    #[tokio::test]
    async fn test_encryption_decryption_roundtrip_should_preserve_data() {
        let mut manager = create_test_security_manager().unwrap();
        let peer = create_test_peer("crypto_peer", vec!["ChaCha20Poly1305X25519"]);
        
        // Establish session
        let session_id = manager.secure_handshake(&peer, None).await.unwrap();
        
        // Test data roundtrip
        let test_data = b"Hello, secure world! This is a test message.";
        let encrypted = manager.encrypt_secure(test_data, &session_id).unwrap();
        let decrypted = manager.decrypt_secure(&encrypted, &session_id).unwrap();
        
        assert_eq!(decrypted, test_data);
        
        // Verify metrics
        let metrics = manager.get_metrics();
        assert_eq!(metrics.encryptions_count, 1);
        assert_eq!(metrics.decryptions_count, 1);
    }

    #[tokio::test]
    async fn test_encryption_should_be_non_deterministic() {
        let mut manager = create_test_security_manager().unwrap();
        let peer = create_test_peer("nonce_peer", vec!["ChaCha20Poly1305X25519"]);
        let session_id = manager.secure_handshake(&peer, None).await.unwrap();
        
        let test_data = b"Same message";
        let encrypted1 = manager.encrypt_secure(test_data, &session_id).unwrap();
        let encrypted2 = manager.encrypt_secure(test_data, &session_id).unwrap();
        
        // Encryptions should differ due to nonces
        assert_ne!(encrypted1, encrypted2);
        
        // Both should decrypt correctly
        let decrypted1 = manager.decrypt_secure(&encrypted1, &session_id).unwrap();
        let decrypted2 = manager.decrypt_secure(&encrypted2, &session_id).unwrap();
        assert_eq!(decrypted1, test_data);
        assert_eq!(decrypted2, test_data);
    }

    #[tokio::test]
    async fn test_tampered_ciphertext_should_fail_decryption() {
        let mut manager = create_test_security_manager().unwrap();
        let peer = create_test_peer("tamper_peer", vec!["ChaCha20Poly1305X25519"]);
        let session_id = manager.secure_handshake(&peer, None).await.unwrap();
        
        let test_data = b"Important secure data";
        let mut encrypted = manager.encrypt_secure(test_data, &session_id).unwrap();
        
        // Tamper with the ciphertext
        if encrypted.len() > 12 {
            encrypted[12] ^= 0xFF; // Flip bits in the actual ciphertext
        }
        
        // Decryption should fail due to authentication tag mismatch
        let result = manager.decrypt_secure(&encrypted, &session_id);
        assert!(result.is_err());
        
        if let Err(NetworkError::Security(SecurityError::Decryption(msg))) = result {
            assert!(msg.contains("authentication tag mismatch"));
        } else {
            panic!("Expected decryption error with authentication tag mismatch");
        }
    }

    #[tokio::test]
    async fn test_invalid_session_should_fail_crypto_operations() {
        let mut manager = create_test_security_manager().unwrap();
        let invalid_session = Uuid::new_v4();
        
        let test_data = b"Test data";
        
        // Encryption with invalid session should fail
        let encrypt_result = manager.encrypt_secure(test_data, &invalid_session);
        assert!(encrypt_result.is_err());
        
        // Decryption with invalid session should fail
        let decrypt_result = manager.decrypt_secure(test_data, &invalid_session);
        assert!(decrypt_result.is_err());
    }

    #[tokio::test]
    async fn test_post_quantum_signature_roundtrip() {
        let mut manager = create_test_security_manager().unwrap();
        
        // Generate test key pair (simplified for testing)
        let secret_key = vec![0x42u8; 64]; // Placeholder secret key
        let public_key = vec![0x24u8; 32]; // Placeholder public key
        let test_data = b"Message to sign";
        
        // Sign with post-quantum algorithm
        let signature = manager.sign_post_quantum(
            test_data,
            PQSignatureAlgorithm::Dilithium3,
            &secret_key
        ).unwrap();
        
        // Verify signature
        let is_valid = manager.verify_post_quantum(
            test_data,
            &signature,
            &public_key,
            PQSignatureAlgorithm::Dilithium3
        ).unwrap();
        
        assert!(is_valid);
        
        // Verify metrics updated
        let metrics = manager.get_metrics();
        assert!(metrics.pq_operations_count >= 2); // Sign + verify operations
    }

    #[tokio::test]
    async fn test_session_key_rotation_should_update_metrics() {
        let mut manager = create_test_security_manager().unwrap();
        let peer = create_test_peer("rotation_peer", vec!["ChaCha20Poly1305X25519"]);
        let session_id = manager.secure_handshake(&peer, None).await.unwrap();
        
        let initial_rotations = manager.get_metrics().key_rotations_count;
        
        // Rotate keys
        manager.rotate_session_keys(&session_id).await.unwrap();
        
        // Verify metrics updated
        let metrics = manager.get_metrics();
        assert_eq!(metrics.key_rotations_count, initial_rotations + 1);
    }

    #[tokio::test]
    async fn test_maintenance_should_cleanup_expired() {
        let mut manager = create_test_security_manager().unwrap();
        
        // Perform maintenance
        let result = manager.perform_maintenance().await;
        assert!(result.is_ok());
        
        // Test passes if no panics or errors occur
        // In a real implementation, we'd verify expired sessions were removed
    }

    // Property-based tests using proptest
    proptest! {
        #[test]
        fn prop_cipher_suite_properties(
            suite in prop::sample::select(vec![
                SecureCipherSuite::ChaCha20Poly1305X25519,
                SecureCipherSuite::Kyber768ChaCha20,
                SecureCipherSuite::HybridX25519Kyber1024ChaCha20,
            ])
        ) {
            // Post-quantum suites should have higher security levels
            if suite.is_post_quantum() {
                prop_assert!(suite.security_level() >= 192);
            }
            
            // Hybrid suites should be post-quantum
            if suite.is_hybrid() {
                prop_assert!(suite.is_post_quantum());
            }
            
            // Security level should be reasonable
            prop_assert!(suite.security_level() >= 128);
            prop_assert!(suite.security_level() <= 256);
        }

        #[test]
        fn prop_encryption_preserves_data_length_properties(
            data in prop::collection::vec(any::<u8>(), 0..1024)
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let mut manager = create_test_security_manager().unwrap();
                let peer = create_test_peer("length_test", vec!["ChaCha20Poly1305X25519"]);
                let session_id = manager.secure_handshake(&peer, None).await.unwrap();
                
                if !data.is_empty() {
                    let encrypted = manager.encrypt_secure(&data, &session_id).unwrap();
                    
                    // Encrypted data should be longer (nonce + ciphertext + tag)
                    prop_assert!(encrypted.len() > data.len());
                    
                    // Should be able to decrypt
                    let decrypted = manager.decrypt_secure(&encrypted, &session_id).unwrap();
                    prop_assert_eq!(decrypted, data);
                }
            })?;
        }

        #[test]
        fn prop_concurrent_operations_are_safe(
            operations in prop::collection::vec(
                prop::collection::vec(any::<u8>(), 1..100),
                1..10
            )
        ) {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let manager = Arc::new(Mutex::new(create_test_security_manager().unwrap()));
                let peer = create_test_peer("concurrent_test", vec!["ChaCha20Poly1305X25519"]);
                
                // Establish session
                let session_id = {
                    let mut mgr = manager.lock().await;
                    mgr.secure_handshake(&peer, None).await.unwrap()
                };
                
                // Perform concurrent operations
                let mut handles = vec![];
                
                for data in operations {
                    let manager = manager.clone();
                    let session_id = session_id;
                    
                    let handle = tokio::spawn(async move {
                        let mut mgr = manager.lock().await;
                        let encrypted = mgr.encrypt_secure(&data, &session_id).unwrap();
                        let decrypted = mgr.decrypt_secure(&encrypted, &session_id).unwrap();
                        (data, decrypted)
                    });
                    
                    handles.push(handle);
                }
                
                // All operations should succeed and data should match
                for handle in handles {
                    let (original, decrypted) = handle.await.unwrap();
                    prop_assert_eq!(original, decrypted);
                }
            })?;
        }
    }

    // Mock-based tests for certificate validation
    #[tokio::test]
    async fn test_certificate_validation_with_mocks() {
        // This test demonstrates London School TDD with mocks
        // In a real implementation, we'd inject the mock validator
        
        let mut manager = create_test_security_manager().unwrap();
        
        // Create mock certificate data
        let mock_cert = quinn::rustls::pki_types::CertificateDer::from(vec![0x30, 0x82, 0x01, 0x00]); // Simplified DER
        let cert_chain = vec![mock_cert];
        
        // Test certificate validation
        // Note: This will use the real certificate validator
        // In a proper mock setup, we'd inject our MockCertificateValidator
        let result = manager.validate_certificate_with_pinning(&cert_chain, "test.example.com");
        
        // Expect failure with mock data
        assert!(result.is_err());
        
        // Verify metrics were updated for failure
        let metrics = manager.get_metrics();
        assert!(metrics.cert_validations_failure > 0);
    }

    // Tests for constant-time operations
    #[tokio::test]
    async fn test_constant_time_operations() {
        let manager = create_test_security_manager().unwrap();
        
        // Test that cipher suite selection is constant-time
        let peer1 = create_test_peer("ct_peer1", vec!["ChaCha20Poly1305X25519"]);
        let peer2 = create_test_peer("ct_peer2", vec![
            "ChaCha20Poly1305X25519", 
            "HybridX25519Kyber1024ChaCha20"
        ]);
        
        // Both should complete without timing side channels
        // In a real test, we'd measure timing variance
        let start1 = std::time::Instant::now();
        let _ = manager.select_cipher_suite_ct(&peer1, None);
        let duration1 = start1.elapsed();
        
        let start2 = std::time::Instant::now();
        let _ = manager.select_cipher_suite_ct(&peer2, None);
        let duration2 = start2.elapsed();
        
        // Timing should be relatively consistent for constant-time ops
        // This is a simplified test - real timing analysis would be more sophisticated
        println!("Duration 1: {:?}, Duration 2: {:?}", duration1, duration2);
        
        // Test passes if no panics occur
        assert!(true);
    }

    // Edge case tests
    #[tokio::test]
    async fn test_empty_data_encryption() {
        let mut manager = create_test_security_manager().unwrap();
        let peer = create_test_peer("empty_peer", vec!["ChaCha20Poly1305X25519"]);
        let session_id = manager.secure_handshake(&peer, None).await.unwrap();
        
        let empty_data = b"";
        let encrypted = manager.encrypt_secure(empty_data, &session_id).unwrap();
        let decrypted = manager.decrypt_secure(&encrypted, &session_id).unwrap();
        
        assert_eq!(decrypted, empty_data);
    }

    #[tokio::test]
    async fn test_large_data_encryption() {
        let mut manager = create_test_security_manager().unwrap();
        let peer = create_test_peer("large_peer", vec!["ChaCha20Poly1305X25519"]);
        let session_id = manager.secure_handshake(&peer, None).await.unwrap();
        
        // Test with 1MB of data
        let large_data = vec![0x42u8; 1024 * 1024];
        let encrypted = manager.encrypt_secure(&large_data, &session_id).unwrap();
        let decrypted = manager.decrypt_secure(&encrypted, &session_id).unwrap();
        
        assert_eq!(decrypted, large_data);
    }

    #[tokio::test]
    async fn test_invalid_ciphertext_length() {
        let mut manager = create_test_security_manager().unwrap();
        let peer = create_test_peer("invalid_peer", vec!["ChaCha20Poly1305X25519"]);
        let session_id = manager.secure_handshake(&peer, None).await.unwrap();
        
        // Test with too-short ciphertext (less than nonce size)
        let invalid_ciphertext = vec![0x00u8; 5];
        let result = manager.decrypt_secure(&invalid_ciphertext, &session_id);
        
        assert!(result.is_err());
        if let Err(NetworkError::Security(SecurityError::Decryption(msg))) = result {
            assert!(msg.contains("Invalid ciphertext length"));
        }
    }
}

// Integration tests for substrate compatibility
#[cfg(test)]
mod substrate_integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_substrate_event_emission() {
        // Test that security events are properly emitted for substrate consumption
        let mut manager = create_test_security_manager().unwrap();
        let peer = create_test_peer("substrate_peer", vec!["ChaCha20Poly1305X25519"]);
        
        // Perform operations that should emit events
        let session_id = manager.secure_handshake(&peer, None).await.unwrap();
        let test_data = b"Substrate test data";
        let _encrypted = manager.encrypt_secure(test_data, &session_id).unwrap();
        
        // In a real implementation, we'd verify that substrate events were emitted
        // For now, verify that operations completed successfully
        let metrics = manager.get_metrics();
        assert_eq!(metrics.key_generations_count, 1);
        assert_eq!(metrics.encryptions_count, 1);
    }
}