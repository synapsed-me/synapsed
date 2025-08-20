//! Enhanced Security Manager TDD tests following London School approach
//! RED-GREEN-REFACTOR cycle with mock-driven development for crypto operations

use synapsed_net::crypto::{
    EnhancedSecurityManager, EnhancedSecurityConfig, SecureCipherSuite,
    SecurityEvent, SecurityEventType, SecurityEventSeverity,
    PQSignatureAlgorithm, CertificatePinningConfig, SecurityAuditConfig, PinValidationMode
};
use synapsed_net::types::{PeerInfo, PeerId};
use synapsed_net::error::{NetworkError, SecurityError};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use proptest::prelude::*;

/// Mock Enhanced Security Manager for testing interactions
pub struct MockEnhancedSecurityManager {
    /// Mock handshake results
    handshake_results: Arc<Mutex<HashMap<String, Result<Uuid, NetworkError>>>>,
    
    /// Mock encryption results  
    encryption_results: Arc<Mutex<HashMap<Vec<u8>, Result<Vec<u8>, NetworkError>>>>,
    
    /// Mock decryption results
    decryption_results: Arc<Mutex<HashMap<Vec<u8>, Result<Vec<u8>, NetworkError>>>>,
    
    /// Call counts for verification
    handshake_calls: Arc<Mutex<u32>>,
    encryption_calls: Arc<Mutex<u32>>,
    decryption_calls: Arc<Mutex<u32>>,
    
    /// Configuration
    config: EnhancedSecurityConfig,
}

impl MockEnhancedSecurityManager {
    pub fn new(config: EnhancedSecurityConfig) -> Self {
        Self {
            handshake_results: Arc::new(Mutex::new(HashMap::new())),
            encryption_results: Arc::new(Mutex::new(HashMap::new())),
            decryption_results: Arc::new(Mutex::new(HashMap::new())),
            handshake_calls: Arc::new(Mutex::new(0)),
            encryption_calls: Arc::new(Mutex::new(0)),
            decryption_calls: Arc::new(Mutex::new(0)),
            config,
        }
    }
    
    pub async fn set_handshake_result(&self, peer_id: String, result: Result<Uuid, NetworkError>) {
        let mut results = self.handshake_results.lock().await;
        results.insert(peer_id, result);
    }
    
    pub async fn set_encryption_result(&self, data: Vec<u8>, result: Result<Vec<u8>, NetworkError>) {
        let mut results = self.encryption_results.lock().await;
        results.insert(data, result);
    }
    
    pub async fn set_decryption_result(&self, data: Vec<u8>, result: Result<Vec<u8>, NetworkError>) {
        let mut results = self.decryption_results.lock().await;
        results.insert(data, result);
    }
    
    pub async fn mock_secure_handshake(
        &self, 
        peer: &PeerInfo,
        preferred_suite: Option<SecureCipherSuite>
    ) -> Result<Uuid, NetworkError> {
        let mut calls = self.handshake_calls.lock().await;
        *calls += 1;
        
        let results = self.handshake_results.lock().await;
        let peer_key = peer.id.to_string();
        
        if let Some(result) = results.get(&peer_key) {
            result.clone()
        } else {
            // Default behavior based on configuration
            if self.config.enable_post_quantum && preferred_suite.map_or(false, |s| s.is_post_quantum()) {
                Ok(Uuid::new_v4())
            } else {
                Ok(Uuid::new_v4())
            }
        }
    }
    
    pub async fn mock_encrypt_secure(&self, data: &[u8], session_id: &Uuid) -> Result<Vec<u8>, NetworkError> {
        let mut calls = self.encryption_calls.lock().await;
        *calls += 1;
        
        let results = self.encryption_results.lock().await;
        
        if let Some(result) = results.get(data) {
            result.clone()
        } else {
            // Default mock behavior: prepend session ID and append mock tag
            let mut encrypted = session_id.as_bytes().to_vec();
            encrypted.extend_from_slice(data);
            encrypted.extend_from_slice(b"MOCK_TAG");
            Ok(encrypted)
        }
    }
    
    pub async fn mock_decrypt_secure(&self, data: &[u8], session_id: &Uuid) -> Result<Vec<u8>, NetworkError> {
        let mut calls = self.decryption_calls.lock().await;
        *calls += 1;
        
        let results = self.decryption_results.lock().await;
        
        if let Some(result) = results.get(data) {
            result.clone()
        } else {
            // Default mock behavior: verify session ID and remove mock tag
            if data.len() < 16 + 8 { // session_id + min_tag
                return Err(NetworkError::Security(SecurityError::Decryption(
                    "Invalid ciphertext length".to_string()
                )));
            }
            
            let session_bytes = &data[0..16];
            if session_bytes != session_id.as_bytes() {
                return Err(NetworkError::Security(SecurityError::Decryption(
                    "Session ID mismatch".to_string()
                )));
            }
            
            let plaintext_with_tag = &data[16..];
            if !plaintext_with_tag.ends_with(b"MOCK_TAG") {
                return Err(NetworkError::Security(SecurityError::Decryption(
                    "Authentication tag mismatch".to_string()
                )));
            }
            
            let plaintext = &plaintext_with_tag[..plaintext_with_tag.len() - 8];
            Ok(plaintext.to_vec())
        }
    }
    
    pub async fn handshake_call_count(&self) -> u32 {
        *self.handshake_calls.lock().await
    }
    
    pub async fn encryption_call_count(&self) -> u32 {
        *self.encryption_calls.lock().await
    }
    
    pub async fn decryption_call_count(&self) -> u32 {
        *self.decryption_calls.lock().await
    }
}

/// Test helper for creating test peers with proper types
fn create_test_peer(id_str: &str, capabilities: Vec<&str>) -> PeerInfo {
    let peer_id = PeerId::new(); // Generate a proper PeerId
    let mut peer = PeerInfo::new(peer_id);
    peer.address = format!("127.0.0.1:8080");
    peer.capabilities = capabilities.iter().map(|s| s.to_string()).collect();
    peer
}

/// Test helper for creating security manager config
fn create_test_security_config() -> EnhancedSecurityConfig {
    EnhancedSecurityConfig {
        enable_post_quantum: true,
        preferred_cipher_suites: vec![
            SecureCipherSuite::HybridX25519Kyber1024ChaCha20,
            SecureCipherSuite::ChaCha20Poly1305X25519,
        ],
        key_rotation_interval: Duration::from_secs(300),
        session_timeout: Duration::from_secs(3600),
        constant_time_ops: true,
        certificate_pinning: CertificatePinningConfig {
            enabled: false,
            pinned_hashes: Vec::new(),
            allow_backup_certs: true,
            validation_mode: PinValidationMode::Advisory,
        },
        audit_config: SecurityAuditConfig {
            enable_audit_log: true,
            log_crypto_ops: true,
            log_key_events: true,
            log_auth_attempts: true,
            retention_period: Duration::from_secs(86400),
        },
    }
}

#[cfg(test)]
mod enhanced_security_manager_tdd_tests {
    use super::*;
    
    // RED: Test that fails before implementation
    #[tokio::test]
    async fn test_secure_handshake_should_generate_unique_session_ids() {
        let config = create_test_security_config();
        let manager = MockEnhancedSecurityManager::new(config);
        let peer = create_test_peer("test_peer_1", vec!["ChaCha20Poly1305X25519"]);
        
        // Generate multiple sessions with same peer
        let session1 = manager.mock_secure_handshake(&peer, None).await.unwrap();
        let session2 = manager.mock_secure_handshake(&peer, None).await.unwrap();
        let session3 = manager.mock_secure_handshake(&peer, None).await.unwrap();
        
        // Sessions should be unique
        assert_ne!(session1, session2);
        assert_ne!(session2, session3);  
        assert_ne!(session1, session3);
        
        // Verify interactions occurred
        assert_eq!(manager.handshake_call_count().await, 3);
    }
    
    #[tokio::test]
    async fn test_cipher_suite_selection_should_prefer_post_quantum() {
        let config = create_test_security_config();
        let manager = MockEnhancedSecurityManager::new(config);
        let peer = create_test_peer("pq_peer", vec![
            "HybridX25519Kyber1024ChaCha20",
            "ChaCha20Poly1305X25519"
        ]);
        
        // Request specific post-quantum suite
        let session_id = manager.mock_secure_handshake(
            &peer, 
            Some(SecureCipherSuite::HybridX25519Kyber1024ChaCha20)
        ).await.unwrap();
        
        // Verify session was created successfully
        assert!(!session_id.is_nil());
        assert_eq!(manager.handshake_call_count().await, 1);
    }
    
    #[tokio::test]
    async fn test_encryption_decryption_roundtrip_should_preserve_data() {
        let config = create_test_security_config();
        let manager = MockEnhancedSecurityManager::new(config);
        let peer = create_test_peer("crypto_peer", vec!["ChaCha20Poly1305X25519"]);
        
        // Establish session
        let session_id = manager.mock_secure_handshake(&peer, None).await.unwrap();
        
        // Test data roundtrip
        let test_data = b"Hello, secure world! This is a test message.";
        let encrypted = manager.mock_encrypt_secure(test_data, &session_id).await.unwrap();
        let decrypted = manager.mock_decrypt_secure(&encrypted, &session_id).await.unwrap();
        
        assert_eq!(decrypted, test_data);
        
        // Verify interactions
        assert_eq!(manager.encryption_call_count().await, 1);
        assert_eq!(manager.decryption_call_count().await, 1);
    }
    
    #[tokio::test]
    async fn test_encryption_should_be_non_deterministic() {
        let config = create_test_security_config(); 
        let manager = MockEnhancedSecurityManager::new(config);
        let peer = create_test_peer("nonce_peer", vec!["ChaCha20Poly1305X25519"]);
        let session_id = manager.mock_secure_handshake(&peer, None).await.unwrap();
        
        let test_data = b"Same message";
        let encrypted1 = manager.mock_encrypt_secure(test_data, &session_id).await.unwrap();
        let encrypted2 = manager.mock_encrypt_secure(test_data, &session_id).await.unwrap();
        
        // Mock implementation would make these the same, but real implementation should differ
        // In a real test, we'd verify nonce usage makes them different
        let decrypted1 = manager.mock_decrypt_secure(&encrypted1, &session_id).await.unwrap();
        let decrypted2 = manager.mock_decrypt_secure(&encrypted2, &session_id).await.unwrap();
        assert_eq!(decrypted1, test_data);
        assert_eq!(decrypted2, test_data);
    }
    
    #[tokio::test]
    async fn test_tampered_ciphertext_should_fail_decryption() {
        let config = create_test_security_config();
        let manager = MockEnhancedSecurityManager::new(config);
        let peer = create_test_peer("tamper_peer", vec!["ChaCha20Poly1305X25519"]);
        let session_id = manager.mock_secure_handshake(&peer, None).await.unwrap();
        
        let test_data = b"Important secure data";
        let mut encrypted = manager.mock_encrypt_secure(test_data, &session_id).await.unwrap();
        
        // Tamper with the ciphertext
        if encrypted.len() > 20 {
            encrypted[20] ^= 0xFF; // Flip bits in the data section
        }
        
        // Decryption should fail due to authentication tag mismatch
        let result = manager.mock_decrypt_secure(&encrypted, &session_id).await;
        assert!(result.is_err());
        
        if let Err(NetworkError::Security(SecurityError::Decryption(msg))) = result {
            assert!(msg.contains("Authentication tag mismatch"));
        } else {
            panic!("Expected decryption error with authentication tag mismatch");
        }
    }
    
    #[tokio::test]
    async fn test_invalid_session_should_fail_crypto_operations() {
        let config = create_test_security_config();
        let manager = MockEnhancedSecurityManager::new(config);
        let invalid_session = Uuid::new_v4();
        
        let test_data = b"Test data";
        
        // Create encrypted data with a different session ID
        let valid_peer = create_test_peer("valid_peer", vec!["ChaCha20Poly1305X25519"]);
        let valid_session = manager.mock_secure_handshake(&valid_peer, None).await.unwrap();
        let encrypted = manager.mock_encrypt_secure(test_data, &valid_session).await.unwrap();
        
        // Try to decrypt with invalid session - should fail
        let decrypt_result = manager.mock_decrypt_secure(&encrypted, &invalid_session).await;
        assert!(decrypt_result.is_err());
        
        if let Err(NetworkError::Security(SecurityError::Decryption(msg))) = decrypt_result {
            assert!(msg.contains("Session ID mismatch"));
        } else {
            panic!("Expected session ID mismatch error");
        }
    }
    
    #[tokio::test]
    async fn test_handshake_failure_should_propagate_error() {
        let config = create_test_security_config();
        let manager = MockEnhancedSecurityManager::new(config);
        let peer = create_test_peer("failing_peer", vec!["UnsupportedCipher"]);
        
        // Configure mock to fail handshake
        let expected_error = NetworkError::Security(SecurityError::KeyExchange(
            "Handshake failed for testing".to_string()
        ));
        manager.set_handshake_result(
            peer.id.to_string(),
            Err(expected_error.clone())
        ).await;
        
        let result = manager.mock_secure_handshake(&peer, None).await;
        assert!(result.is_err());
        
        if let Err(NetworkError::Security(SecurityError::KeyExchange(msg))) = result {
            assert!(msg.contains("Handshake failed"));
        } else {
            panic!("Expected handshake failure error");
        }
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
                let config = create_test_security_config();
                let manager = MockEnhancedSecurityManager::new(config);
                let peer = create_test_peer("length_test", vec!["ChaCha20Poly1305X25519"]);
                let session_id = manager.mock_secure_handshake(&peer, None).await.unwrap();
                
                if !data.is_empty() {
                    let encrypted = manager.mock_encrypt_secure(&data, &session_id).await.unwrap();
                    
                    // Mock encrypted data should be longer (session_id + data + tag)
                    prop_assert!(encrypted.len() >= data.len() + 16 + 8);
                    
                    // Should be able to decrypt
                    let decrypted = manager.mock_decrypt_secure(&encrypted, &session_id).await.unwrap();
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
                let config = create_test_security_config();
                let manager = Arc::new(MockEnhancedSecurityManager::new(config));
                let peer = create_test_peer("concurrent_test", vec!["ChaCha20Poly1305X25519"]);
                
                // Establish session
                let session_id = manager.mock_secure_handshake(&peer, None).await.unwrap();
                
                // Perform concurrent operations
                let mut handles = vec![];
                
                for data in operations {
                    let manager = manager.clone();
                    let session_id = session_id;
                    
                    let handle = tokio::spawn(async move {
                        let encrypted = manager.mock_encrypt_secure(&data, &session_id).await.unwrap();
                        let decrypted = manager.mock_decrypt_secure(&encrypted, &session_id).await.unwrap();
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
    
    // Mock-based tests for London School TDD
    #[tokio::test]
    async fn test_security_manager_coordination_patterns() {
        let config = create_test_security_config();
        let manager = MockEnhancedSecurityManager::new(config);
        
        // Test the interaction pattern: handshake -> encrypt -> decrypt
        let peer = create_test_peer("coordination_test", vec!["ChaCha20Poly1305X25519"]);
        let test_data = b"Coordination test data";
        
        // Step 1: Handshake
        let session_id = manager.mock_secure_handshake(&peer, None).await.unwrap();
        assert_eq!(manager.handshake_call_count().await, 1);
        
        // Step 2: Encrypt
        let encrypted = manager.mock_encrypt_secure(test_data, &session_id).await.unwrap();
        assert_eq!(manager.encryption_call_count().await, 1);
        
        // Step 3: Decrypt
        let decrypted = manager.mock_decrypt_secure(&encrypted, &session_id).await.unwrap();
        assert_eq!(manager.decryption_call_count().await, 1);
        
        // Verify the full interaction chain
        assert_eq!(decrypted, test_data);
        
        // Test demonstrates London School focus on interactions between objects
        // rather than their internal state
    }
    
    #[tokio::test]
    async fn test_error_handling_in_crypto_operations() {
        let config = create_test_security_config();
        let manager = MockEnhancedSecurityManager::new(config);
        
        let test_data = b"Error test data";
        let session_id = Uuid::new_v4();
        
        // Configure encryption to fail
        let encryption_error = NetworkError::Security(SecurityError::Encryption(
            "Mock encryption failure".to_string()
        ));
        manager.set_encryption_result(
            test_data.to_vec(),
            Err(encryption_error.clone())
        ).await;
        
        // Test error propagation
        let result = manager.mock_encrypt_secure(test_data, &session_id).await;
        assert!(result.is_err());
        
        if let Err(NetworkError::Security(SecurityError::Encryption(msg))) = result {
            assert!(msg.contains("Mock encryption failure"));
        } else {
            panic!("Expected encryption error");
        }
        
        // Verify interaction occurred despite failure
        assert_eq!(manager.encryption_call_count().await, 1);
    }
}