//! Comprehensive TDD Tests for DID Rotation - RED PHASE
//!
//! These tests implement the full DID rotation algorithm specification
//! from did-rotation-algorithms.md. They are written FIRST to define
//! the expected behavior, following SPARC methodology.
//! 
//! Performance Requirements:
//! - DID generation: < 100ms
//! - ZK proof generation: < 500ms  
//! - Key rotation: no session interruption

use synapsed_identity::did::*;
use synapsed_identity::{Result, Error};
use chrono::{Utc, Duration};
use tokio::time::Instant;
use std::collections::HashMap;

#[cfg(test)]
mod did_rotation_tests {
    use super::*;

    /// RED PHASE: Test DID rotation with forward secrecy per Algorithm 2
    #[tokio::test]
    async fn test_rotate_did_keys_with_forward_secrecy() {
        // Setup: Create initial DID with keys
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let (did, mut hierarchy) = key_manager.generate_did_with_keys("key", "secure_password").await.unwrap();
        key_manager.initialize_hierarchy(&did, hierarchy.master_key.clone()).unwrap();

        // Get initial key IDs
        let initial_keys = hierarchy.get_active_key_ids();
        let initial_generation = hierarchy.current_generation;

        // Test manual rotation
        let rotation_result = key_manager.rotate_keys(&did, RotationReason::Manual).unwrap();

        // Verify rotation occurred
        assert!(rotation_result.rotated, "Rotation should have occurred");
        assert!(!rotation_result.new_keys.is_empty(), "New keys should be created");
        assert!(!rotation_result.deprecated_keys.is_empty(), "Old keys should be deprecated");
        assert!(rotation_result.updated_document.is_some(), "DID document should be updated");

        // Verify forward secrecy - old keys are deprecated  
        let updated_hierarchy = key_manager.hierarchies.get(&did).unwrap();
        assert_eq!(updated_hierarchy.current_generation, initial_generation + 1);
        
        // All initial keys should be in historical keys with revocation timestamp
        for old_key_id in &initial_keys {
            assert!(updated_hierarchy.historical_keys.contains_key(old_key_id), 
                   "Old key {} should be in historical keys", old_key_id);
            let historical_key = &updated_hierarchy.historical_keys[old_key_id];
            assert!(historical_key.revoked_at.is_some(), 
                   "Old key {} should have revocation timestamp", old_key_id);
        }

        // Active keys should be completely new
        let new_keys = updated_hierarchy.get_active_key_ids();
        for new_key_id in &new_keys {
            assert!(!initial_keys.contains(new_key_id), 
                   "New key {} should not be in initial keys", new_key_id);
        }
    }

    /// RED PHASE: Test scheduled rotation based on policy
    #[tokio::test]
    async fn test_scheduled_rotation_based_on_policy() {
        let mut policy = RotationPolicy::default();
        policy.max_key_age = Duration::seconds(1); // Very short for testing

        let mut key_manager = KeyRotationManager::new(
            policy,
            RecoveryMechanism::default()
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();
        
        // Initially should not need rotation
        let result = key_manager.rotate_keys(&did, RotationReason::Scheduled).unwrap();
        assert!(!result.rotated, "Fresh keys should not need rotation");

        // Wait for key age to exceed policy
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Now should need rotation
        let result = key_manager.rotate_keys(&did, RotationReason::Scheduled).unwrap();
        assert!(result.rotated, "Old keys should trigger scheduled rotation");
    }

    /// RED PHASE: Test compromise rotation (always rotates)
    #[tokio::test]
    async fn test_compromise_rotation_always_rotates() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();

        // Compromise rotation should always occur regardless of age
        let result = key_manager.rotate_keys(&did, RotationReason::Compromise).unwrap();
        assert!(result.rotated, "Compromise should always trigger rotation");
        
        // Immediately try again - should still rotate
        let result2 = key_manager.rotate_keys(&did, RotationReason::Compromise).unwrap();
        assert!(result2.rotated, "Multiple compromise rotations should be allowed");
    }

    /// RED PHASE: Test device rotation based on policy
    #[tokio::test]
    async fn test_device_rotation_policy() {
        let mut policy = RotationPolicy::default();
        policy.rotate_on_device_change = true;

        let mut key_manager = KeyRotationManager::new(
            policy,
            RecoveryMechanism::default()
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();

        // Device rotation should occur when policy allows
        let result = key_manager.rotate_keys(&did, RotationReason::Device).unwrap();
        assert!(result.rotated, "Device change should trigger rotation when policy allows");

        // Test with policy disabled
        let mut policy_disabled = RotationPolicy::default();
        policy_disabled.rotate_on_device_change = false;

        let mut key_manager_no_device = KeyRotationManager::new(
            policy_disabled,
            RecoveryMechanism::default()
        );

        let (did2, _) = key_manager_no_device.generate_did_with_keys("key", "password").await.unwrap();
        let result = key_manager_no_device.rotate_keys(&did2, RotationReason::Device).unwrap();
        assert!(!result.rotated, "Device change should not trigger rotation when policy disabled");
    }

    /// RED PHASE: Test rotation history tracking
    #[tokio::test]
    async fn test_rotation_history_tracking() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();

        // Perform multiple rotations with different reasons
        key_manager.rotate_keys(&did, RotationReason::Manual).unwrap();
        key_manager.rotate_keys(&did, RotationReason::Compromise).unwrap();
        key_manager.rotate_keys(&did, RotationReason::Device).unwrap();

        let hierarchy = key_manager.hierarchies.get(&did).unwrap();
        assert_eq!(hierarchy.rotation_history.len(), 3, "Should track all rotation events");

        // Verify rotation events have proper data
        for (i, event) in hierarchy.rotation_history.iter().enumerate() {
            assert!(event.timestamp <= Utc::now(), "Event timestamp should be valid");
            assert_eq!(event.generation, (i + 2) as u32, "Generation should increment");
            assert!(!event.rotated_keys.is_empty(), "Rotated keys should be recorded");
        }

        // Verify reasons are tracked correctly
        assert_eq!(hierarchy.rotation_history[0].reason, RotationReason::Manual);
        assert_eq!(hierarchy.rotation_history[1].reason, RotationReason::Compromise);
        assert_eq!(hierarchy.rotation_history[2].reason, RotationReason::Device);
    }

    /// RED PHASE: Test DID document update after rotation
    #[tokio::test]
    async fn test_did_document_update_after_rotation() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();

        // Get initial document
        let hierarchy = key_manager.hierarchies.get(&did).unwrap();
        let initial_doc = key_manager.update_did_document(&did, hierarchy).unwrap();
        let initial_vm_count = initial_doc.verification_method.len();

        // Rotate keys
        let rotation_result = key_manager.rotate_keys(&did, RotationReason::Manual).unwrap();
        let updated_doc = rotation_result.updated_document.unwrap();

        // Verify document structure
        assert_eq!(updated_doc.id, did, "Document ID should match DID");
        assert_eq!(updated_doc.verification_method.len(), initial_vm_count, 
                  "Should have same number of verification methods");

        // Verify verification methods reference new keys
        let updated_hierarchy = key_manager.hierarchies.get(&did).unwrap();
        let new_key_ids = updated_hierarchy.get_active_key_ids();
        
        for vm in &updated_doc.verification_method {
            let key_id = vm.id.split('#').last().unwrap();
            assert!(new_key_ids.contains(&key_id.to_string()), 
                   "Verification method should reference new key: {}", key_id);
        }

        // Verify verification relationships are properly set
        assert!(!updated_doc.authentication.is_empty(), "Should have authentication methods");
        assert!(!updated_doc.assertion_method.is_empty(), "Should have assertion methods");
        assert!(!updated_doc.capability_invocation.is_empty(), "Should have capability invocation");
        assert!(!updated_doc.key_agreement.is_empty(), "Should have key agreement");
    }

    /// RED PHASE: Test key generation increments properly
    #[tokio::test]
    async fn test_key_generation_increments() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();

        // Verify initial generation
        let hierarchy = key_manager.hierarchies.get(&did).unwrap();
        assert_eq!(hierarchy.current_generation, 1);

        for (_, key_material) in &hierarchy.active_keys {
            assert_eq!(key_material.generation, 1, "Initial keys should be generation 1");
        }

        // Rotate and verify generation increment
        key_manager.rotate_keys(&did, RotationReason::Manual).unwrap();
        
        let updated_hierarchy = key_manager.hierarchies.get(&did).unwrap();
        assert_eq!(updated_hierarchy.current_generation, 2);

        for (_, key_material) in &updated_hierarchy.active_keys {
            assert_eq!(key_material.generation, 2, "Rotated keys should be generation 2");
        }

        // Rotate again
        key_manager.rotate_keys(&did, RotationReason::Manual).unwrap();
        
        let final_hierarchy = key_manager.hierarchies.get(&did).unwrap();
        assert_eq!(final_hierarchy.current_generation, 3);

        for (_, key_material) in &final_hierarchy.active_keys {
            assert_eq!(key_material.generation, 3, "Second rotation keys should be generation 3");
        }
    }

    /// RED PHASE: Test key material encryption during rotation
    #[tokio::test]
    async fn test_key_material_encryption_during_rotation() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();

        // Test encryption of key materials
        let hierarchy = key_manager.hierarchies.get(&did).unwrap();
        let master_key = &hierarchy.master_key;

        // Get a key material to encrypt
        let (_, key_material) = hierarchy.active_keys.iter().next().unwrap();
        
        // Test encryption
        let encrypted = master_key.encrypt_key_material(key_material).unwrap();
        assert_eq!(encrypted.algorithm, "ChaCha20Poly1305");
        assert!(!encrypted.ciphertext.is_empty());
        assert_eq!(encrypted.nonce.len(), 12);

        // Test decryption
        let decrypted = master_key.decrypt_key_material(encrypted).unwrap();
        assert_eq!(decrypted.key_id, key_material.key_id);
        assert_eq!(decrypted.key_type, key_material.key_type);
    }

    /// RED PHASE: Test historical key access and validation
    #[tokio::test]
    async fn test_historical_key_access_and_validation() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();

        // Get initial key for later validation
        let hierarchy = key_manager.hierarchies.get(&did).unwrap();
        let initial_key_id = hierarchy.get_active_key_ids()[0].clone();
        let creation_time = Utc::now();

        // Rotate keys
        key_manager.rotate_keys(&did, RotationReason::Manual).unwrap();

        let updated_hierarchy = key_manager.hierarchies.get(&did).unwrap();

        // Test access to historical private key
        let historical_private_key = updated_hierarchy.get_private_key(&initial_key_id);
        assert!(historical_private_key.is_ok(), "Should be able to access historical private key");

        // Test key validity at different times
        assert!(updated_hierarchy.is_key_valid(&initial_key_id, creation_time), 
               "Key should be valid at creation time");
        
        assert!(!updated_hierarchy.is_key_valid(&initial_key_id, Utc::now()), 
               "Key should be invalid after revocation");
    }

    /// RED PHASE: Test multibase encoding consistency across rotations
    #[tokio::test]
    async fn test_multibase_encoding_consistency() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();

        // Perform multiple rotations and verify multibase consistency
        for _ in 0..3 {
            key_manager.rotate_keys(&did, RotationReason::Manual).unwrap();
            
            let hierarchy = key_manager.hierarchies.get(&did).unwrap();
            for (_, key_material) in &hierarchy.active_keys {
                // All public keys should use Base58BTC encoding (starts with 'z')
                assert!(key_material.public_key_multibase.starts_with("z"), 
                       "Multibase should use Base58BTC encoding");
                
                // Should be decodable
                let decode_result = multibase::decode(&key_material.public_key_multibase);
                assert!(decode_result.is_ok(), "Multibase should be decodable");
                
                let (base, _data) = decode_result.unwrap();
                assert_eq!(base, multibase::Base::Base58Btc, "Should use Base58BTC");
            }
        }
    }

    /// RED PHASE: Test performance requirements for DID generation
    #[tokio::test]
    async fn test_did_generation_performance() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let start = Instant::now();
        let result = key_manager.generate_did_with_keys("key", "secure_password").await;
        let duration = start.elapsed();

        assert!(result.is_ok(), "DID generation should succeed");
        assert!(duration.as_millis() < 100, 
               "DID generation should complete in <100ms, took {}ms", 
               duration.as_millis());
    }

    /// RED PHASE: Test performance requirements for key rotation
    #[tokio::test]
    async fn test_key_rotation_performance() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();

        let start = Instant::now();
        let result = key_manager.rotate_keys(&did, RotationReason::Manual);
        let duration = start.elapsed();

        assert!(result.is_ok(), "Key rotation should succeed");
        // Note: This should be very fast as it's just key generation and updates
        assert!(duration.as_millis() < 50, 
               "Key rotation should complete quickly, took {}ms", 
               duration.as_millis());
    }

    /// RED PHASE: Test error handling for non-existent DID rotation
    #[tokio::test]
    async fn test_rotation_error_handling_nonexistent_did() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let non_existent_did = Did::new("key", "nonexistent");
        let result = key_manager.rotate_keys(&non_existent_did, RotationReason::Manual);

        assert!(result.is_err(), "Should error for non-existent DID");
        if let Err(Error::KeyManagementError(msg)) = result {
            assert!(msg.contains("Key hierarchy not found"));
        } else {
            panic!("Expected KeyManagementError for non-existent DID");
        }
    }

    /// RED PHASE: Test concurrent rotation safety
    #[tokio::test]
    async fn test_concurrent_rotation_safety() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();

        // This test would require Arc<Mutex<KeyRotationManager>> in real implementation
        // For now, we test that rotation maintains consistency
        let initial_generation = key_manager.hierarchies.get(&did).unwrap().current_generation;

        // Perform sequential rotations (simulating concurrent access patterns)
        for i in 1..=5 {
            let result = key_manager.rotate_keys(&did, RotationReason::Manual).unwrap();
            assert!(result.rotated, "Rotation {} should succeed", i);
            
            let hierarchy = key_manager.hierarchies.get(&did).unwrap();
            assert_eq!(hierarchy.current_generation, initial_generation + i, 
                      "Generation should increment consistently");
        }
    }

    /// RED PHASE: Test key ID generation follows specification format
    #[tokio::test]
    async fn test_key_id_generation_format() {
        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            RecoveryMechanism::default()
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();

        // Verify initial key ID format
        let hierarchy = key_manager.hierarchies.get(&did).unwrap();
        let active_keys = hierarchy.get_active_key_ids();
        
        // Should have signing and encryption keys
        assert!(active_keys.iter().any(|id| id == "signing-1"), "Should have signing-1 key");
        assert!(active_keys.iter().any(|id| id == "encryption-1"), "Should have encryption-1 key");

        // After rotation, should have generation-2 keys
        key_manager.rotate_keys(&did, RotationReason::Manual).unwrap();
        
        let updated_hierarchy = key_manager.hierarchies.get(&did).unwrap();
        let new_keys = updated_hierarchy.get_active_key_ids();
        
        assert!(new_keys.iter().any(|id| id == "signing-2"), "Should have signing-2 key after rotation");
        assert!(new_keys.iter().any(|id| id == "encryption-2"), "Should have encryption-2 key after rotation");
    }
}

#[cfg(test)]
mod zero_knowledge_proof_tests {
    use super::*;

    /// RED PHASE: Test zero-knowledge proof generation for subscriptions
    #[tokio::test]
    async fn test_subscription_proof_generation() {
        // This will fail until we implement ZK proof functionality
        let subscription = AnonymousSubscription {
            id: "sub_test_123".to_string(),
            did: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            tier: SubscriptionTier::Premium,
            amount: Amount { value: 1999, currency: "USD".to_string() },
            status: PaymentStatus::Active,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(30),
            private_data: SubscriptionPrivateData {
                stripe_subscription_id: "sub_stripe_xyz".to_string(),
                payment_method_id: "pm_test_card".to_string(),
            },
        };

        let start = Instant::now();
        let result = generate_subscription_proof(
            &subscription, 
            SubscriptionTier::Basic, 
            "test_context"
        ).await;
        let duration = start.elapsed();

        assert!(result.is_ok(), "ZK proof generation should succeed");
        assert!(duration.as_millis() < 500, 
               "ZK proof generation should complete in <500ms, took {}ms", 
               duration.as_millis());

        let proof = result.unwrap();
        assert!(!proof.validity_proof.is_empty(), "Should have validity proof");
        assert!(!proof.tier_proof.is_empty(), "Should have tier proof");
        assert!(proof.expires_at > Utc::now(), "Proof should not be expired");
        assert!(!proof.commitments.nullifier.is_empty(), "Should have nullifier");
    }

    /// RED PHASE: Test subscription proof verification
    #[tokio::test]
    async fn test_subscription_proof_verification() {
        // Generate a proof first
        let subscription = AnonymousSubscription {
            id: "sub_test_123".to_string(),
            did: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            tier: SubscriptionTier::Premium,
            amount: Amount { value: 1999, currency: "USD".to_string() },
            status: PaymentStatus::Active,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(30),
            private_data: SubscriptionPrivateData {
                stripe_subscription_id: "sub_stripe_xyz".to_string(),
                payment_method_id: "pm_test_card".to_string(),
            },
        };

        let proof = generate_subscription_proof(
            &subscription, 
            SubscriptionTier::Basic, 
            "test_context"
        ).await.unwrap();

        // Test verification
        let verification_result = verify_subscription_proof(
            &proof,
            SubscriptionTier::Basic,
            "test_context"
        ).await;

        assert!(verification_result.is_ok(), "Proof verification should succeed");
        let result = verification_result.unwrap();
        
        assert!(result.is_valid, "Proof should be valid");
        assert!(result.tier_sufficient, "Tier should be sufficient");
        assert!(!result.allowed_features.is_empty(), "Should have allowed features");
    }

    /// RED PHASE: Test proof expiry handling
    #[tokio::test]
    async fn test_proof_expiry_handling() {
        // Create expired subscription
        let expired_subscription = AnonymousSubscription {
            id: "sub_expired".to_string(),
            did: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            tier: SubscriptionTier::Premium,
            amount: Amount { value: 1999, currency: "USD".to_string() },
            status: PaymentStatus::Active,
            created_at: Utc::now() - Duration::days(60),
            expires_at: Utc::now() - Duration::days(1), // Expired
            private_data: SubscriptionPrivateData {
                stripe_subscription_id: "sub_stripe_expired".to_string(),
                payment_method_id: "pm_test_card".to_string(),
            },
        };

        let result = generate_subscription_proof(
            &expired_subscription, 
            SubscriptionTier::Basic, 
            "test_context"
        ).await;

        assert!(result.is_err(), "Should not generate proof for expired subscription");
        if let Err(Error::SubscriptionError(msg)) = result {
            assert!(msg.contains("expired"));
        } else {
            panic!("Expected subscription error for expired subscription");
        }
    }

    /// RED PHASE: Test nullifier uniqueness prevents double-spending
    #[tokio::test]
    async fn test_nullifier_prevents_double_spending() {
        let subscription = AnonymousSubscription {
            id: "sub_test_123".to_string(),
            did: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
            tier: SubscriptionTier::Premium,
            amount: Amount { value: 1999, currency: "USD".to_string() },
            status: PaymentStatus::Active,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(30),
            private_data: SubscriptionPrivateData {
                stripe_subscription_id: "sub_stripe_xyz".to_string(),
                payment_method_id: "pm_test_card".to_string(),
            },
        };

        // Generate first proof
        let proof1 = generate_subscription_proof(
            &subscription, 
            SubscriptionTier::Basic, 
            "test_context"
        ).await.unwrap();

        // First verification should succeed
        let result1 = verify_subscription_proof(
            &proof1,
            SubscriptionTier::Basic,
            "test_context"
        ).await.unwrap();
        assert!(result1.is_valid, "First verification should succeed");

        // Generate second proof (should have same nullifier)
        let proof2 = generate_subscription_proof(
            &subscription, 
            SubscriptionTier::Basic, 
            "test_context"
        ).await.unwrap();

        // Second verification should fail due to nullifier reuse
        let result2 = verify_subscription_proof(
            &proof2,
            SubscriptionTier::Basic,
            "test_context"
        ).await.unwrap();
        
        assert!(!result2.is_valid, "Second verification should fail");
        assert!(result2.metadata.get("error").unwrap().as_str().unwrap().contains("nullifier_already_used"));
    }
}

#[cfg(test)]
mod key_recovery_tests {
    use super::*;

    /// RED PHASE: Test recovery information generation
    #[tokio::test]
    async fn test_recovery_info_generation() {
        let recovery_mechanism = RecoveryMechanism {
            recovery_phrase_length: 24,
            social_recovery_threshold: Some(3),
            hardware_recovery: true,
        };

        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            recovery_mechanism
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();
        
        // Generate recovery info
        let hierarchy = key_manager.hierarchies.get(&did).unwrap();
        let recovery_info = generate_recovery_info(hierarchy, &key_manager.recovery).await;

        assert!(recovery_info.is_ok(), "Recovery info generation should succeed");
        let info = recovery_info.unwrap();

        assert!(info.recovery_phrase.is_some(), "Should have recovery phrase");
        assert!(!info.social_recovery_contacts.is_empty(), "Should have social recovery contacts");
        assert!(info.hardware_recovery_data.is_some(), "Should have hardware recovery data");
    }

    /// RED PHASE: Test BIP39 mnemonic recovery
    #[tokio::test]
    async fn test_bip39_mnemonic_recovery() {
        let recovery_mechanism = RecoveryMechanism {
            recovery_phrase_length: 24,
            social_recovery_threshold: None,
            hardware_recovery: false,
        };

        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            recovery_mechanism
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();
        
        // Generate recovery phrase
        let hierarchy = key_manager.hierarchies.get(&did).unwrap();
        let recovery_info = generate_recovery_info(hierarchy, &key_manager.recovery).await.unwrap();
        let recovery_phrase = recovery_info.recovery_phrase.unwrap();

        // Test recovery from mnemonic
        let recovery_data = RecoveryData {
            recovery_method: RecoveryMethod::RecoveryPhrase,
            recovery_phrase: Some(recovery_phrase),
            social_shares: None,
            hardware_data: None,
            encrypted_hierarchy: vec![], // Will be populated by actual implementation
        };

        let recovery_result = key_manager.recover_keys(&did, recovery_data).await;
        assert!(recovery_result.is_ok(), "Mnemonic recovery should succeed");
    }

    /// RED PHASE: Test social recovery with Shamir secret sharing
    #[tokio::test]
    async fn test_social_recovery_shamir_shares() {
        let recovery_mechanism = RecoveryMechanism {
            recovery_phrase_length: 0,
            social_recovery_threshold: Some(3),
            hardware_recovery: false,
        };

        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            recovery_mechanism
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();
        
        // Generate social recovery shares
        let hierarchy = key_manager.hierarchies.get(&did).unwrap();
        let recovery_info = generate_recovery_info(hierarchy, &key_manager.recovery).await.unwrap();

        // Should have 5 contacts (threshold + 2 extra shares)
        assert_eq!(recovery_info.social_recovery_contacts.len(), 5);

        // Simulate collecting shares from 3 out of 5 contacts
        let shares = vec![
            SecretShare { x: 1, y: vec![1, 2, 3] },
            SecretShare { x: 2, y: vec![4, 5, 6] },
            SecretShare { x: 3, y: vec![7, 8, 9] },
        ];

        let recovery_data = RecoveryData {
            recovery_method: RecoveryMethod::SocialRecovery,
            recovery_phrase: None,
            social_shares: Some(shares),
            hardware_data: None,
            encrypted_hierarchy: vec![],
        };

        let recovery_result = key_manager.recover_keys(&did, recovery_data).await;
        assert!(recovery_result.is_ok(), "Social recovery should succeed with sufficient shares");
    }

    /// RED PHASE: Test insufficient shares for social recovery
    #[tokio::test]
    async fn test_insufficient_shares_social_recovery() {
        let recovery_mechanism = RecoveryMechanism {
            recovery_phrase_length: 0,
            social_recovery_threshold: Some(3),
            hardware_recovery: false,
        };

        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            recovery_mechanism
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();

        // Only 2 shares (insufficient for threshold of 3)
        let shares = vec![
            SecretShare { x: 1, y: vec![1, 2, 3] },
            SecretShare { x: 2, y: vec![4, 5, 6] },
        ];

        let recovery_data = RecoveryData {
            recovery_method: RecoveryMethod::SocialRecovery,
            recovery_phrase: None,
            social_shares: Some(shares),
            hardware_data: None,
            encrypted_hierarchy: vec![],
        };

        let recovery_result = key_manager.recover_keys(&did, recovery_data).await;
        assert!(recovery_result.is_err(), "Social recovery should fail with insufficient shares");
    }

    /// RED PHASE: Test hardware recovery mechanism
    #[tokio::test]
    async fn test_hardware_recovery() {
        let recovery_mechanism = RecoveryMechanism {
            recovery_phrase_length: 0,
            social_recovery_threshold: None,
            hardware_recovery: true,
        };

        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            recovery_mechanism
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();
        
        // Generate hardware recovery data
        let hierarchy = key_manager.hierarchies.get(&did).unwrap();
        let recovery_info = generate_recovery_info(hierarchy, &key_manager.recovery).await.unwrap();
        let hardware_data = recovery_info.hardware_recovery_data.unwrap();

        let recovery_data = RecoveryData {
            recovery_method: RecoveryMethod::HardwareRecovery,
            recovery_phrase: None,
            social_shares: None,
            hardware_data: Some(hardware_data),
            encrypted_hierarchy: vec![],
        };

        let recovery_result = key_manager.recover_keys(&did, recovery_data).await;
        assert!(recovery_result.is_ok(), "Hardware recovery should succeed");
    }

    /// RED PHASE: Test combined recovery methods
    #[tokio::test]
    async fn test_combined_recovery_methods() {
        let recovery_mechanism = RecoveryMechanism {
            recovery_phrase_length: 12,
            social_recovery_threshold: Some(2),
            hardware_recovery: true,
        };

        let mut key_manager = KeyRotationManager::new(
            RotationPolicy::default(),
            recovery_mechanism
        );

        let (did, _) = key_manager.generate_did_with_keys("key", "password").await.unwrap();
        
        // Generate all recovery types
        let hierarchy = key_manager.hierarchies.get(&did).unwrap();
        let recovery_info = generate_recovery_info(hierarchy, &key_manager.recovery).await.unwrap();

        assert!(recovery_info.recovery_phrase.is_some(), "Should have recovery phrase");
        assert!(!recovery_info.social_recovery_contacts.is_empty(), "Should have social contacts");
        assert!(recovery_info.hardware_recovery_data.is_some(), "Should have hardware data");

        // Test combined recovery (e.g., mnemonic + hardware)
        let recovery_data = RecoveryData {
            recovery_method: RecoveryMethod::CombinedRecovery,
            recovery_phrase: recovery_info.recovery_phrase,
            social_shares: None, // Not using social in this test
            hardware_data: recovery_info.hardware_recovery_data,
            encrypted_hierarchy: vec![],
        };

        let recovery_result = key_manager.recover_keys(&did, recovery_data).await;
        assert!(recovery_result.is_ok(), "Combined recovery should succeed");
    }
}

// All types are now implemented in the main modules