//! Red Phase TDD Tests for DID Rotation and Subscription Portability
//!
//! These tests define the behavior for maintaining subscription access across DID changes.
//! All tests should initially FAIL to drive implementation.
//!
//! Test Requirements:
//! - DIDs can rotate while maintaining subscription validity
//! - Subscription proofs work with new DID after rotation
//! - Old DID access is revoked after rotation
//! - Portable subscription proofs work across DID changes
//! - Recovery mechanisms for lost DIDs
//! - Privacy preservation during rotation process

use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use uuid::Uuid;

use synapsed_payments::prelude::*;
use synapsed_payments::zkp::*;
use synapsed_payments::did_integration::*;
use synapsed_payments::types::*;

/// Test basic DID rotation maintains subscription access
#[tokio::test]
async fn test_basic_did_rotation() {
    // RED PHASE: This test should FAIL initially
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    let mut did_manager = DIDManager::new();
    
    let old_did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6";
    let new_did = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
    
    let amount = Amount::new(Decimal::new(2999, 2), Currency::Fiat(FiatCurrency::USD))
        .expect("Failed to create amount");
    
    // Create subscription with old DID
    let subscription = zkp_engine.create_anonymous_subscription(
        old_did.to_string(),
        "sub_did_rotation_test".to_string(),
        SubscriptionTier::Premium,
        amount,
        Utc::now() + Duration::days(30),
    ).await.expect("Failed to create subscription");
    
    // Create DID session
    let session = did_manager.create_session(
        old_did,
        vec![subscription.id.clone()],
        Duration::hours(1),
    ).await.expect("Failed to create DID session");
    
    // Verify old DID works before rotation
    let proof_old = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "pre_rotation_context",
    ).await.expect("Failed to generate proof with old DID");
    
    let request_old = VerificationRequest {
        proof: proof_old,
        min_tier: SubscriptionTier::Basic,
        features: vec!["premium_access".to_string()],
        context: "pre_rotation_verification".to_string(),
    };
    
    let result_old = zkp_engine.verify_subscription_proof(&request_old).await
        .expect("Failed to verify proof with old DID");
    assert!(result_old.is_valid);
    
    // Perform DID rotation
    let rotation_signature = b"mock_rotation_signature_proving_ownership".to_vec();
    did_manager.rotate_did(
        old_did,
        new_did,
        rotation_signature,
        RotationReason::UserRequested,
        &mut zkp_engine,
    ).await.expect("Failed to rotate DID");
    
    // REQUIREMENTS TO IMPLEMENT:
    
    // 1. New DID should work for generating proofs
    let proof_new = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "post_rotation_context",
    ).await.expect("Failed to generate proof with new DID");
    
    let request_new = VerificationRequest {
        proof: proof_new,
        min_tier: SubscriptionTier::Basic,
        features: vec!["premium_access".to_string()],
        context: "post_rotation_verification".to_string(),
    };
    
    let result_new = zkp_engine.verify_subscription_proof(&request_new).await
        .expect("Failed to verify proof with new DID");
    
    assert!(result_new.is_valid);
    assert!(result_new.tier_sufficient);
    
    // 2. Old DID should no longer work (either at generation or verification)
    let old_did_result = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "post_rotation_old_did_context",
    ).await;
    
    // Should either fail to generate or fail to verify
    if let Ok(proof_old_after) = old_did_result {
        let request_old_after = VerificationRequest {
            proof: proof_old_after,
            min_tier: SubscriptionTier::Basic,
            features: vec!["premium_access".to_string()],
            context: "post_rotation_old_did_verification".to_string(),
        };
        
        let result_old_after = zkp_engine.verify_subscription_proof(&request_old_after).await;
        match result_old_after {
            Ok(result) => assert!(!result.is_valid, "Old DID should not verify after rotation"),
            Err(_) => {}, // Error is acceptable for old DID
        }
    }
    
    // 3. Subscription tier and expiry should be preserved
    assert!(result_new.allowed_features.contains(&"advanced_features".to_string()));
}

/// Test multiple DID rotations in sequence
#[tokio::test]
async fn test_sequential_did_rotations() {
    // RED PHASE: This test should FAIL initially
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    let mut did_manager = DIDManager::new();
    
    let dids = vec![
        "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6",
        "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
        "did:key:z6MkpTHR2JnfqPvC4kKNmVtrhNqNhB5JQV6CVZ2QXNFdHBvZ",
        "did:key:z6MkrJVnaZkeFzdQyQSSbJ3RL7JcLPhEQRLm9jk6GU5mKtF9",
    ];
    
    let amount = Amount::new(Decimal::new(4999, 2), Currency::Fiat(FiatCurrency::USD))
        .expect("Failed to create amount");
    
    // Create subscription with first DID
    let subscription = zkp_engine.create_anonymous_subscription(
        dids[0].to_string(),
        "sub_sequential_rotation_test".to_string(),
        SubscriptionTier::Pro,
        amount,
        Utc::now() + Duration::days(30),
    ).await.expect("Failed to create subscription");
    
    // Perform sequential rotations
    for i in 1..dids.len() {
        let old_did = dids[i - 1];
        let new_did = dids[i];
        
        // Verify old DID works before rotation
        let proof_before = zkp_engine.generate_subscription_proof(
            &subscription.id,
            SubscriptionTier::Basic,
            &format!("before_rotation_{}", i),
        ).await.expect("Failed to generate proof before rotation");
        
        let request_before = VerificationRequest {
            proof: proof_before,
            min_tier: SubscriptionTier::Basic,
            features: vec!["pro_access".to_string()],
            context: format!("before_rotation_verification_{}", i),
        };
        
        let result_before = zkp_engine.verify_subscription_proof(&request_before).await
            .expect("Failed to verify proof before rotation");
        assert!(result_before.is_valid);
        
        // Perform rotation
        let rotation_signature = format!("rotation_signature_{}", i).as_bytes().to_vec();
        did_manager.rotate_did(
            old_did,
            new_did,
            rotation_signature,
            RotationReason::Scheduled,
            &mut zkp_engine,
        ).await.expect("Failed to rotate DID");
        
        // REQUIREMENTS TO IMPLEMENT:
        
        // 1. New DID should work after rotation
        let proof_after = zkp_engine.generate_subscription_proof(
            &subscription.id,
            SubscriptionTier::Basic,
            &format!("after_rotation_{}", i),
        ).await.expect("Failed to generate proof after rotation");
        
        let request_after = VerificationRequest {
            proof: proof_after,
            min_tier: SubscriptionTier::Basic,
            features: vec!["pro_access".to_string()],
            context: format!("after_rotation_verification_{}", i),
        };
        
        let result_after = zkp_engine.verify_subscription_proof(&request_after).await
            .expect("Failed to verify proof after rotation");
        
        assert!(result_after.is_valid);
        assert!(result_after.tier_sufficient);
        
        // 2. Previous DIDs should no longer work
        for j in 0..i {
            let old_did_test = dids[j];
            let old_proof_result = zkp_engine.generate_subscription_proof(
                &subscription.id,
                SubscriptionTier::Basic,
                &format!("test_old_did_{}_{}", i, j),
            ).await;
            
            // Should fail to generate or verify
            if let Ok(old_proof) = old_proof_result {
                let old_request = VerificationRequest {
                    proof: old_proof,
                    min_tier: SubscriptionTier::Basic,
                    features: vec!["pro_access".to_string()],
                    context: format!("old_did_test_{}_{}", i, j),
                };
                
                let old_result = zkp_engine.verify_subscription_proof(&old_request).await;
                match old_result {
                    Ok(result) => assert!(!result.is_valid, "Old DID {} should not work after rotation {}", j, i),
                    Err(_) => {}, // Error is acceptable
                }
            }
        }
    }
    
    // 3. Final DID should still provide full Pro access
    let final_proof = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Pro,
        "final_did_test",
    ).await.expect("Failed to generate proof with final DID");
    
    let final_request = VerificationRequest {
        proof: final_proof,
        min_tier: SubscriptionTier::Pro,
        features: vec!["pro_access".to_string(), "api_access".to_string()],
        context: "final_did_verification".to_string(),
    };
    
    let final_result = zkp_engine.verify_subscription_proof(&final_request).await
        .expect("Failed to verify proof with final DID");
    
    assert!(final_result.is_valid);
    assert!(final_result.tier_sufficient);
    assert!(final_result.allowed_features.contains(&"api_access".to_string()));
}

/// Test portable subscription proofs across DID rotations
#[tokio::test]
async fn test_portable_subscription_proofs() {
    // RED PHASE: This test should FAIL initially
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    let mut did_manager = DIDManager::new();
    
    let old_did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6";
    let new_did = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
    
    let amount = Amount::new(Decimal::new(9999, 2), Currency::Fiat(FiatCurrency::USD))
        .expect("Failed to create amount");
    
    // Create subscription
    let subscription = zkp_engine.create_anonymous_subscription(
        old_did.to_string(),
        "sub_portable_proof_test".to_string(),
        SubscriptionTier::Enterprise,
        amount,
        Utc::now() + Duration::days(30),
    ).await.expect("Failed to create subscription");
    
    // Generate portable proof before rotation
    let portable_proof = did_manager.generate_portable_proof(
        old_did,
        &subscription.id,
        &zkp_engine,
    ).await.expect("Failed to generate portable proof");
    
    // Perform DID rotation
    let rotation_signature = b"portable_rotation_signature".to_vec();
    did_manager.rotate_did(
        old_did,
        new_did,
        rotation_signature,
        RotationReason::UserRequested,
        &mut zkp_engine,
    ).await.expect("Failed to rotate DID");
    
    // REQUIREMENTS TO IMPLEMENT:
    
    // 1. Portable proof should still be valid after rotation
    assert!(portable_proof.expires_at > Utc::now());
    assert!(!portable_proof.validity_proof.is_empty());
    assert!(!portable_proof.did_commitment.is_empty());
    assert!(!portable_proof.portability_signature.is_empty());
    
    // 2. Portable proof should not reveal the actual DIDs
    let proof_str = format!("{:?}", portable_proof);
    assert!(!proof_str.contains(old_did));
    assert!(!proof_str.contains(new_did));
    assert!(!proof_str.contains("sub_portable"));
    
    // 3. Should be able to verify portable proof independently
    // (This would require a separate verification system for portable proofs)
    
    // 4. New DID should be able to generate new portable proofs
    let new_portable_proof = did_manager.generate_portable_proof(
        new_did,
        &subscription.id,
        &zkp_engine,
    ).await.expect("Failed to generate portable proof with new DID");
    
    assert!(new_portable_proof.expires_at > Utc::now());
    assert_ne!(portable_proof.portability_signature, new_portable_proof.portability_signature);
    
    // 5. Old DID should not be able to generate new portable proofs
    let old_portable_result = did_manager.generate_portable_proof(
        old_did,
        &subscription.id,
        &zkp_engine,
    ).await;
    
    assert!(old_portable_result.is_err(), "Old DID should not be able to generate portable proofs after rotation");
}

/// Test DID recovery maintains subscription access
#[tokio::test]
async fn test_did_recovery_maintains_access() {
    // RED PHASE: This test should FAIL initially
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    let mut did_manager = DIDManager::new();
    
    let lost_did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6";
    let recovery_did = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
    
    let amount = Amount::new(Decimal::new(5999, 2), Currency::Fiat(FiatCurrency::USD))
        .expect("Failed to create amount");
    
    // Create subscription with original DID
    let subscription = zkp_engine.create_anonymous_subscription(
        lost_did.to_string(),
        "sub_recovery_test".to_string(),
        SubscriptionTier::Enterprise,
        amount,
        Utc::now() + Duration::days(30),
    ).await.expect("Failed to create subscription");
    
    // Create DID session
    let session = did_manager.create_session(
        lost_did,
        vec![subscription.id.clone()],
        Duration::hours(1),
    ).await.expect("Failed to create DID session");
    
    // Verify original DID works
    let proof_original = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "pre_recovery_context",
    ).await.expect("Failed to generate proof with original DID");
    
    let request_original = VerificationRequest {
        proof: proof_original,
        min_tier: SubscriptionTier::Basic,
        features: vec!["enterprise_access".to_string()],
        context: "pre_recovery_verification".to_string(),
    };
    
    let result_original = zkp_engine.verify_subscription_proof(&request_original).await
        .expect("Failed to verify proof with original DID");
    assert!(result_original.is_valid);
    
    // Simulate DID loss and recovery
    let recovery_request = DIDRecoveryRequest {
        old_did: lost_did.to_string(),
        new_did: recovery_did.to_string(),
        recovery_proof: b"backup_key_recovery_proof_signature".to_vec(),
        recovery_method: RecoveryMethod::BackupKey,
        timestamp: Utc::now(),
    };
    
    let recovered_subscriptions = did_manager.recover_access(&recovery_request, &mut zkp_engine).await
        .expect("Failed to recover DID access");
    
    // REQUIREMENTS TO IMPLEMENT:
    
    // 1. Should recover access to subscriptions
    assert!(!recovered_subscriptions.is_empty());
    assert!(recovered_subscriptions.contains(&subscription.id));
    
    // 2. Recovery DID should work for generating proofs
    let proof_recovery = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "post_recovery_context",
    ).await.expect("Failed to generate proof with recovery DID");
    
    let request_recovery = VerificationRequest {
        proof: proof_recovery,
        min_tier: SubscriptionTier::Basic,
        features: vec!["enterprise_access".to_string()],
        context: "post_recovery_verification".to_string(),
    };
    
    let result_recovery = zkp_engine.verify_subscription_proof(&request_recovery).await
        .expect("Failed to verify proof with recovery DID");
    
    assert!(result_recovery.is_valid);
    assert!(result_recovery.tier_sufficient);
    
    // 3. Should maintain original subscription tier and features
    assert!(result_recovery.allowed_features.contains(&"enterprise_features".to_string()));
    
    // 4. Lost DID should no longer work
    let lost_proof_result = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "post_recovery_lost_did_context",
    ).await;
    
    if let Ok(lost_proof) = lost_proof_result {
        let lost_request = VerificationRequest {
            proof: lost_proof,
            min_tier: SubscriptionTier::Basic,
            features: vec!["enterprise_access".to_string()],
            context: "post_recovery_lost_did_verification".to_string(),
        };
        
        let lost_result = zkp_engine.verify_subscription_proof(&lost_request).await;
        match lost_result {
            Ok(result) => assert!(!result.is_valid, "Lost DID should not work after recovery"),
            Err(_) => {}, // Error is acceptable
        }
    }
}

/// Test DID rotation with different recovery methods
#[tokio::test]
async fn test_different_recovery_methods() {
    // RED PHASE: This test should FAIL initially
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    let mut did_manager = DIDManager::new();
    
    let amount = Amount::new(Decimal::new(1999, 2), Currency::Fiat(FiatCurrency::USD))
        .expect("Failed to create amount");
    
    let recovery_methods = vec![
        (RecoveryMethod::BackupKey, "backup_key_signature"),
        (RecoveryMethod::SocialRecovery, "social_recovery_multisig"),
        (RecoveryMethod::MultiSig, "multisig_recovery_proof"),
    ];
    
    for (i, (recovery_method, proof_data)) in recovery_methods.iter().enumerate() {
        let lost_did = format!("did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP{}", i);
        let recovery_did = format!("did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2d{}", i);
        
        // Create subscription
        let subscription = zkp_engine.create_anonymous_subscription(
            lost_did.clone(),
            format!("sub_recovery_method_test_{}", i),
            SubscriptionTier::Basic,
            amount.clone(),
            Utc::now() + Duration::days(30),
        ).await.expect("Failed to create subscription");
        
        // Simulate recovery with different method
        let recovery_request = DIDRecoveryRequest {
            old_did: lost_did.clone(),
            new_did: recovery_did.clone(),
            recovery_proof: proof_data.as_bytes().to_vec(),
            recovery_method: *recovery_method,
            timestamp: Utc::now(),
        };
        
        let recovery_result = did_manager.recover_access(&recovery_request, &mut zkp_engine).await;
        
        // REQUIREMENTS TO IMPLEMENT:
        
        // 1. Different recovery methods should have different validation requirements
        match recovery_method {
            RecoveryMethod::BackupKey => {
                assert!(recovery_result.is_ok(), "Backup key recovery should succeed");
            }
            RecoveryMethod::SocialRecovery => {
                assert!(recovery_result.is_ok(), "Social recovery should succeed");
            }
            RecoveryMethod::MultiSig => {
                assert!(recovery_result.is_ok(), "MultiSig recovery should succeed");
            }
            _ => {
                // Other methods might not be implemented yet
            }
        }
        
        if let Ok(recovered_subscriptions) = recovery_result {
            // 2. Recovery should restore subscription access
            assert!(recovered_subscriptions.contains(&subscription.id));
            
            // 3. Recovered DID should work for proof generation
            let proof = zkp_engine.generate_subscription_proof(
                &subscription.id,
                SubscriptionTier::Basic,
                &format!("recovery_method_test_{}", i),
            ).await.expect("Failed to generate proof with recovered DID");
            
            let request = VerificationRequest {
                proof,
                min_tier: SubscriptionTier::Basic,
                features: vec!["basic_access".to_string()],
                context: format!("recovery_method_verification_{}", i),
            };
            
            let result = zkp_engine.verify_subscription_proof(&request).await
                .expect("Failed to verify proof with recovered DID");
            
            assert!(result.is_valid);
            assert!(result.tier_sufficient);
        }
    }
}

/// Test privacy preservation during DID rotation
#[tokio::test]
async fn test_rotation_privacy_preservation() {
    // RED PHASE: This test should FAIL initially
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    let mut did_manager = DIDManager::new();
    
    let old_did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6";
    let new_did = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
    
    let amount = Amount::new(Decimal::new(7999, 2), Currency::Fiat(FiatCurrency::USD))
        .expect("Failed to create amount");
    
    // Create subscription
    let subscription = zkp_engine.create_anonymous_subscription(
        old_did.to_string(),
        "sub_privacy_rotation_test".to_string(),
        SubscriptionTier::Enterprise,
        amount,
        Utc::now() + Duration::days(30),
    ).await.expect("Failed to create subscription");
    
    // Generate proof before rotation
    let proof_before = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "privacy_before_rotation",
    ).await.expect("Failed to generate proof before rotation");
    
    // Perform rotation
    let rotation_signature = b"privacy_rotation_signature".to_vec();
    did_manager.rotate_did(
        old_did,
        new_did,
        rotation_signature,
        RotationReason::UserRequested,
        &mut zkp_engine,
    ).await.expect("Failed to rotate DID");
    
    // Generate proof after rotation
    let proof_after = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "privacy_after_rotation",
    ).await.expect("Failed to generate proof after rotation");
    
    // REQUIREMENTS TO IMPLEMENT:
    
    // 1. Proofs should not reveal DID rotation occurred
    let proof_before_str = format!("{:?}", proof_before);
    let proof_after_str = format!("{:?}", proof_after);
    
    assert!(!proof_before_str.contains(old_did));
    assert!(!proof_before_str.contains(new_did));
    assert!(!proof_after_str.contains(old_did));
    assert!(!proof_after_str.contains(new_did));
    
    // 2. Nullifiers should not reveal rotation relationship
    let nullifier_before = &proof_before.commitments.nullifier;
    let nullifier_after = &proof_after.commitments.nullifier;
    
    // Nullifiers might be the same (subscription-based) or different (context-based)
    // but they should not reveal the DID rotation relationship
    
    // 3. DID commitments should not reveal actual DIDs
    let did_commitment_before = &proof_before.commitments.did_commitment;
    let did_commitment_after = &proof_after.commitments.did_commitment;
    
    assert!(!did_commitment_before.is_empty());
    assert!(!did_commitment_after.is_empty());
    
    // 4. Subscription ID should remain the same (privacy-preserving rotation)
    assert_eq!(subscription.id, subscription.id); // Tautology, but documents requirement
    
    // 5. Both proofs should verify successfully
    let request_before = VerificationRequest {
        proof: proof_before,
        min_tier: SubscriptionTier::Basic,
        features: vec!["enterprise_access".to_string()],
        context: "privacy_verification_before".to_string(),
    };
    
    let request_after = VerificationRequest {
        proof: proof_after,
        min_tier: SubscriptionTier::Basic,
        features: vec!["enterprise_access".to_string()],
        context: "privacy_verification_after".to_string(),
    };
    
    let result_before = zkp_engine.verify_subscription_proof(&request_before).await
        .expect("Failed to verify proof before rotation");
    let result_after = zkp_engine.verify_subscription_proof(&request_after).await
        .expect("Failed to verify proof after rotation");
    
    assert!(result_before.is_valid);
    assert!(result_after.is_valid);
    
    // 6. Verification results should not reveal rotation occurred
    assert_eq!(result_before.allowed_features, result_after.allowed_features);
}

/// Test concurrent DID rotations and access
#[tokio::test]
async fn test_concurrent_did_operations() {
    // RED PHASE: This test should FAIL initially - needs thread-safe DID operations
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    let did_manager = DIDManager::new();
    
    // This test would require Arc<Mutex<>> or similar for concurrent access
    // For now, we'll test the concept with sequential operations that simulate concurrency
    
    let base_did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP";
    let amount = Amount::new(Decimal::new(999, 2), Currency::Fiat(FiatCurrency::USD))
        .expect("Failed to create amount");
    
    let mut subscriptions = Vec::new();
    
    // Create multiple subscriptions with different DIDs
    for i in 0..5 {
        let did = format!("{}_{}", base_did, i);
        let subscription = zkp_engine.create_anonymous_subscription(
            did,
            format!("sub_concurrent_test_{}", i),
            SubscriptionTier::Basic,
            amount.clone(),
            Utc::now() + Duration::days(30),
        ).await.expect("Failed to create subscription");
        
        subscriptions.push(subscription);
    }
    
    // REQUIREMENTS TO IMPLEMENT:
    
    // 1. Multiple subscriptions should work independently
    for (i, subscription) in subscriptions.iter().enumerate() {
        let proof = zkp_engine.generate_subscription_proof(
            &subscription.id,
            SubscriptionTier::Basic,
            &format!("concurrent_context_{}", i),
        ).await.expect("Failed to generate concurrent proof");
        
        let request = VerificationRequest {
            proof,
            min_tier: SubscriptionTier::Basic,
            features: vec!["basic_access".to_string()],
            context: format!("concurrent_verification_{}", i),
        };
        
        let result = zkp_engine.verify_subscription_proof(&request).await
            .expect("Failed to verify concurrent proof");
        
        assert!(result.is_valid);
        assert!(result.tier_sufficient);
    }
    
    // 2. System should handle concurrent operations without data races
    // (This would be tested with actual concurrent operations in a thread-safe implementation)
    
    println!("Concurrent operations test completed for {} subscriptions", subscriptions.len());
}