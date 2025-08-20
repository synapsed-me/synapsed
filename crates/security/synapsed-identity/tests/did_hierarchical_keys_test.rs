//! TDD Tests for DID Hierarchical Key Management 
//! 
//! Following SPARC methodology - these tests define the expected behavior
//! for hierarchical key generation and management according to the DID 
//! rotation algorithms specification.

use synapsed_identity::did::*;
use synapsed_identity::{Result, Error};
use chrono::{Utc, Duration};
use std::collections::HashMap;

#[cfg(test)]
mod hierarchical_key_tests {
    use super::*;

    /// Test DID generation with hierarchical keys per Algorithm 1
    #[tokio::test]
    async fn test_generate_did_with_hierarchical_keys() {
        // Test did:key method
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let result = key_manager.generate_did_with_keys("key", "secure_password").await;
        assert!(result.is_ok(), "DID generation should succeed");

        let (did, hierarchy) = result.unwrap();
        
        // Verify DID structure
        assert_eq!(did.scheme, "did");
        assert_eq!(did.method, "key");
        assert!(!did.method_specific_id.is_empty());
        
        // Verify hierarchical key structure
        assert_eq!(hierarchy.current_generation, 1);
        assert!(hierarchy.active_keys.len() >= 2); // At least signing + encryption
        
        // Verify required key types exist
        let active_key_ids: Vec<&String> = hierarchy.active_keys.keys().collect();
        assert!(active_key_ids.iter().any(|id| id.contains("signing")));
        assert!(active_key_ids.iter().any(|id| id.contains("encryption")));
        
        // Verify master key is properly initialized
        assert_eq!(hierarchy.master_key.key_bytes.len(), 32);
        assert_eq!(hierarchy.master_key.salt.len(), 32);
    }

    /// Test did:synapsed method generation
    #[tokio::test]
    async fn test_generate_synapsed_did_with_keys() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let result = key_manager.generate_did_with_keys("synapsed", "secure_password").await;
        assert!(result.is_ok());

        let (did, hierarchy) = result.unwrap();
        assert_eq!(did.method, "synapsed");
        
        // Synapsed DIDs should use SHA3-256 of public key
        assert!(did.method_specific_id.len() > 32); // Base58 encoded hash
    }

    /// Test invalid DID method handling
    #[tokio::test]
    async fn test_invalid_did_method() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let result = key_manager.generate_did_with_keys("invalid", "password").await;
        assert!(result.is_err());
        
        if let Err(Error::Configuration(msg)) = result {
            assert!(msg.contains("Unsupported DID method"));
        } else {
            panic!("Expected Configuration error for invalid method");
        }
    }

    /// Test did:web method requires domain
    #[tokio::test]
    async fn test_did_web_requires_domain() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let result = key_manager.generate_did_with_keys("web", "password").await;
        assert!(result.is_err());
        
        if let Err(Error::Configuration(msg)) = result {
            assert!(msg.contains("did:web requires domain specification"));
        } else {
            panic!("Expected Configuration error for did:web without domain");
        }
    }

    /// Test initial key generation creates proper key materials
    #[tokio::test]
    async fn test_initial_key_generation() {
        let did = Did::new("test", "example");
        let master_key = MasterKey::new("test_password", None).unwrap();
        let hierarchy = KeyHierarchy::new(did.clone(), master_key).unwrap();

        // Verify key structure
        assert_eq!(hierarchy.current_generation, 1);
        assert!(hierarchy.active_keys.contains_key("signing-1"));
        assert!(hierarchy.active_keys.contains_key("encryption-1"));
        
        // Verify key properties
        let signing_key = &hierarchy.active_keys["signing-1"];
        assert_eq!(signing_key.key_type, super::methods::KeyType::Ed25519);
        assert_eq!(signing_key.generation, 1);
        assert!(signing_key.private_key.is_some());
        assert!(signing_key.revoked_at.is_none());
        
        let encryption_key = &hierarchy.active_keys["encryption-1"];
        assert_eq!(encryption_key.key_type, super::methods::KeyType::X25519);
        assert_eq!(encryption_key.generation, 1);
        assert!(encryption_key.private_key.is_some());
        assert!(encryption_key.revoked_at.is_none());
    }

    /// Test post-quantum key generation when feature enabled
    #[cfg(feature = "post-quantum")]
    #[tokio::test]
    async fn test_post_quantum_key_generation() {
        let did = Did::new("test", "example");
        let master_key = MasterKey::new("test_password", None).unwrap();
        let hierarchy = KeyHierarchy::new(did.clone(), master_key).unwrap();

        // Should have post-quantum keys in addition to classical ones
        assert!(hierarchy.active_keys.len() >= 4); // signing, encryption, pq-signing, pq-kem
        assert!(hierarchy.active_keys.contains_key("pq-signing-1"));
        assert!(hierarchy.active_keys.contains_key("pq-kem-1"));
        
        let pq_sign_key = &hierarchy.active_keys["pq-signing-1"];
        assert_eq!(pq_sign_key.key_type, super::methods::KeyType::PostQuantumSign);
        
        let pq_kem_key = &hierarchy.active_keys["pq-kem-1"];
        assert_eq!(pq_kem_key.key_type, super::methods::KeyType::PostQuantumKem);
    }

    /// Test master key encryption/decryption of key materials
    #[tokio::test]
    async fn test_master_key_encryption() {
        let master_key = MasterKey::new("test_password", None).unwrap();
        
        // Create test key material
        let key_material = KeyMaterial {
            key_id: "test-key".to_string(),
            key_type: super::methods::KeyType::Ed25519,
            public_key_multibase: "zABC123".to_string(),
            private_key: Some(PrivateKeyMaterial {
                private_key_bytes: vec![1, 2, 3, 4, 5],
                encrypted: false,
            }),
            created_at: Utc::now(),
            expires_at: None,
            revoked_at: None,
            generation: 1,
        };

        // Test encryption
        let encrypted = master_key.encrypt_key_material(&key_material).unwrap();
        assert_eq!(encrypted.key_id, "test-key");
        assert_eq!(encrypted.algorithm, "ChaCha20Poly1305");
        assert!(!encrypted.ciphertext.is_empty());
        assert_eq!(encrypted.nonce.len(), 12);

        // Test decryption
        let decrypted = master_key.decrypt_key_material(encrypted).unwrap();
        assert_eq!(decrypted.key_id, key_material.key_id);
        assert_eq!(decrypted.key_type, key_material.key_type);
        assert_eq!(decrypted.private_key.as_ref().unwrap().private_key_bytes, 
                   key_material.private_key.as_ref().unwrap().private_key_bytes);
    }

    /// Test multibase encoding of public keys
    #[tokio::test]
    async fn test_multibase_encoding() {
        let did = Did::new("test", "example");
        let master_key = MasterKey::new("test_password", None).unwrap();
        let hierarchy = KeyHierarchy::new(did.clone(), master_key).unwrap();

        for (_, key_material) in &hierarchy.active_keys {
            // Verify multibase encoding format
            assert!(key_material.public_key_multibase.starts_with("z")); // Base58BTC prefix
            assert!(key_material.public_key_multibase.len() > 10);
        }
    }

    /// Test DID document generation from hierarchy
    #[tokio::test]
    async fn test_did_document_from_hierarchy() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let (did, hierarchy) = key_manager.generate_did_with_keys("key", "password").await.unwrap();
        key_manager.initialize_hierarchy(&did, hierarchy.master_key.clone()).unwrap();
        
        // Update DID document should create proper verification methods
        let updated_doc = key_manager.update_did_document(&did, &hierarchy).unwrap();
        
        assert_eq!(updated_doc.id, did);
        assert!(!updated_doc.verification_method.is_empty());
        
        // Should have authentication relationships
        assert!(!updated_doc.authentication.is_empty());
        
        // Should have capability invocation for signing keys
        assert!(!updated_doc.capability_invocation.is_empty());
        
        // Should have key agreement for encryption keys
        assert!(!updated_doc.key_agreement.is_empty());
    }

    /// Test key validity checking
    #[tokio::test]
    async fn test_key_validity_checking() {
        let did = Did::new("test", "example");
        let master_key = MasterKey::new("test_password", None).unwrap();
        let hierarchy = KeyHierarchy::new(did.clone(), master_key).unwrap();

        let key_id = "signing-1";
        let now = Utc::now();
        
        // Key should be valid at current time
        assert!(hierarchy.is_key_valid(key_id, now));
        
        // Key should not be valid before creation time
        let before_creation = now - Duration::hours(1);
        assert!(!hierarchy.is_key_valid(key_id, before_creation));
        
        // Key should not be valid for non-existent key
        assert!(!hierarchy.is_key_valid("non-existent", now));
    }

    /// Test key material creation with proper metadata
    #[tokio::test]
    async fn test_key_material_metadata() {
        let did = Did::new("test", "example");
        let master_key = MasterKey::new("test_password", None).unwrap();
        let hierarchy = KeyHierarchy::new(did.clone(), master_key).unwrap();

        for (key_id, key_material) in &hierarchy.active_keys {
            // Verify proper metadata
            assert_eq!(key_material.key_id, *key_id);
            assert_eq!(key_material.generation, 1);
            assert!(key_material.created_at <= Utc::now());
            assert!(key_material.expires_at.is_none()); // No expiration by default
            assert!(key_material.revoked_at.is_none()); // Not revoked initially
            
            // Verify private key material exists and is not encrypted initially
            let private_key = key_material.private_key.as_ref().unwrap();
            assert!(!private_key.private_key_bytes.is_empty());
            assert!(!private_key.encrypted); // Should not be encrypted in memory initially
        }
    }

    /// Test ScryptKDF parameters match specification
    #[tokio::test]
    async fn test_scrypt_parameters() {
        let master_key = MasterKey::new("test_password", None).unwrap();
        
        // Verify KDF parameters match specification constants
        assert_eq!(master_key.kdf_params.algorithm, "scrypt");
        assert_eq!(master_key.kdf_params.n, 32768); // SCRYPT_N
        assert_eq!(master_key.kdf_params.r, 8);     // SCRYPT_R  
        assert_eq!(master_key.kdf_params.p, 1);     // SCRYPT_P
        
        // Verify derived key size
        assert_eq!(master_key.key_bytes.len(), 32); // KEY_SIZE
        assert_eq!(master_key.salt.len(), 32);      // Salt size
    }
}