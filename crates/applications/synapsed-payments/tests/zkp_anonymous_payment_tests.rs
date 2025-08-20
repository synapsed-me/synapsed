//! Red Phase TDD Tests for Anonymous Payment Verification with Zero-Knowledge Proofs
//!
//! These tests define the desired behavior for anonymous subscription verification
//! without revealing user identity. All tests should initially FAIL to drive implementation.
//!
//! Test Requirements:
//! - Users can prove valid subscription without revealing identity
//! - DIDs can rotate while maintaining subscription validity  
//! - Relay servers can verify subscriptions without user data
//! - Prevent linkability between payments and DIDs
//! - ZK proof generation < 500ms, verification < 50ms
//! - Support for 10,000+ concurrent verifications

use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use std::collections::HashSet;
use std::time::Instant;
use tokio::task::JoinSet;
use uuid::Uuid;

use synapsed_payments::prelude::*;
use synapsed_payments::zkp::*;
use synapsed_payments::did_integration::*;
use synapsed_payments::types::*;

/// Test anonymous subscription creation without linking to Stripe
#[tokio::test]
async fn test_anonymous_subscription_creation_unlinkable() {
    // RED PHASE: This test should FAIL initially
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    
    let did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6";
    let stripe_subscription_id = "sub_1234567890abcdef";
    
    let amount = Amount::new(
        Decimal::new(2999, 2), // $29.99
        Currency::Fiat(FiatCurrency::USD),
    ).expect("Failed to create amount");
    
    // Create anonymous subscription
    let anonymous_sub = zkp_engine.create_anonymous_subscription(
        did.to_string(),
        stripe_subscription_id.to_string(),
        SubscriptionTier::Premium,
        amount,
        Utc::now() + Duration::days(30),
    ).await.expect("Failed to create anonymous subscription");
    
    // REQUIREMENTS TO IMPLEMENT:
    
    // 1. Anonymous ID should NOT be derivable from Stripe ID
    assert!(!anonymous_sub.id.contains("sub_"));
    assert!(!anonymous_sub.id.contains("1234567890"));
    assert_ne!(anonymous_sub.id, stripe_subscription_id);
    
    // 2. Should not contain any Stripe-linkable metadata in public fields
    assert!(!anonymous_sub.id.contains("stripe"));
    assert!(!format!("{:?}", anonymous_sub.tier).contains("stripe"));
    
    // 3. Private data should be properly encrypted/hidden
    assert!(anonymous_sub.private_data.stripe_subscription_id.is_some());
    
    // 4. Should generate unique proof secrets
    assert!(!anonymous_sub.private_data.proof_secrets.blinding_factor.is_empty());
    assert!(!anonymous_sub.private_data.proof_secrets.witness.is_empty());
    assert!(!anonymous_sub.private_data.proof_secrets.did_key.is_empty());
    
    // 5. Status should be active
    assert_eq!(anonymous_sub.status, PaymentStatus::Completed);
}

/// Test zero-knowledge proof generation without revealing subscription details
#[tokio::test]
async fn test_zkp_generation_privacy_preserving() {
    // RED PHASE: This test should FAIL initially
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    
    let did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6";
    let stripe_id = "sub_sensitive_data_12345";
    
    let amount = Amount::new(
        Decimal::new(4999, 2), // $49.99
        Currency::Fiat(FiatCurrency::USD),
    ).expect("Failed to create amount");
    
    let subscription = zkp_engine.create_anonymous_subscription(
        did.to_string(),
        stripe_id.to_string(),
        SubscriptionTier::Pro,
        amount,
        Utc::now() + Duration::days(30),
    ).await.expect("Failed to create subscription");
    
    // Generate ZK proof
    let proof = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic, // Verifying against Basic tier
        "api_access_context",
    ).await.expect("Failed to generate proof");
    
    // REQUIREMENTS TO IMPLEMENT:
    
    // 1. Proof should not contain sensitive data
    let proof_str = format!("{:?}", proof);
    assert!(!proof_str.contains("sub_sensitive"));
    assert!(!proof_str.contains("12345"));
    assert!(!proof_str.contains(did));
    assert!(!proof_str.contains("4999")); // Amount should not be revealed
    
    // 2. Proof should contain necessary commitments
    assert!(!proof.commitments.tier_commitment.is_empty());
    assert!(!proof.commitments.did_commitment.is_empty());
    assert!(!proof.commitments.nullifier.is_empty());
    
    // 3. Proof should have proper validity period
    assert!(proof.expires_at > Utc::now());
    assert!(proof.expires_at <= Utc::now() + Duration::hours(2));
    
    // 4. Validity proof should be non-empty
    assert!(!proof.validity_proof.is_empty());
    assert!(!proof.tier_proof.is_empty());
}

/// Test subscription verification without revealing subscriber identity
#[tokio::test]
async fn test_anonymous_subscription_verification() {
    // RED PHASE: This test should FAIL initially
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    
    let subscriber_did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6";
    
    // Create subscription
    let amount = Amount::new(Decimal::new(9999, 2), Currency::Fiat(FiatCurrency::USD))
        .expect("Failed to create amount");
    
    let subscription = zkp_engine.create_anonymous_subscription(
        subscriber_did.to_string(),
        "sub_enterprise_plan".to_string(),
        SubscriptionTier::Enterprise,
        amount,
        Utc::now() + Duration::days(30),
    ).await.expect("Failed to create subscription");
    
    // Generate proof
    let proof = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Premium, // Verifying Premium access
        "enterprise_api_context",
    ).await.expect("Failed to generate proof");
    
    // Verify as relay server (should not know subscriber identity)
    let verification_request = VerificationRequest {
        proof,
        min_tier: SubscriptionTier::Premium,
        features: vec!["enterprise_api".to_string(), "advanced_features".to_string()],
        context: "relay_server_verification".to_string(),
    };
    
    let result = zkp_engine.verify_subscription_proof(&verification_request).await
        .expect("Failed to verify subscription proof");
    
    // REQUIREMENTS TO IMPLEMENT:
    
    // 1. Verification should succeed without knowing subscriber identity
    assert!(result.is_valid);
    assert!(result.tier_sufficient);
    
    // 2. Should provide appropriate features for tier
    assert!(result.allowed_features.contains(&"enterprise_features".to_string()));
    assert!(result.allowed_features.contains(&"api_access".to_string()));
    
    // 3. Should not reveal subscriber information
    assert!(!result.metadata.values().any(|v| v.contains(subscriber_did)));
    assert!(!result.metadata.values().any(|v| v.contains("sub_enterprise")));
    
    // 4. Should have proper expiry
    assert!(result.expires_at > Utc::now());
    
    // 5. Should indicate verification method
    assert_eq!(result.metadata.get("verification_method"), Some(&"zkp".to_string()));
}

/// Test nullifier prevents double-spending of subscription proofs
#[tokio::test]
async fn test_nullifier_prevents_double_spending() {
    // RED PHASE: This test should FAIL initially
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    
    let did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6";
    let amount = Amount::new(Decimal::new(1999, 2), Currency::Fiat(FiatCurrency::USD))
        .expect("Failed to create amount");
    
    let subscription = zkp_engine.create_anonymous_subscription(
        did.to_string(),
        "sub_basic_plan".to_string(),
        SubscriptionTier::Basic,
        amount,
        Utc::now() + Duration::days(30),
    ).await.expect("Failed to create subscription");
    
    // Generate first proof
    let proof1 = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "first_access_context",
    ).await.expect("Failed to generate first proof");
    
    // Generate second proof (should have different nullifier or be rejected)
    let proof2 = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "second_access_context", 
    ).await.expect("Failed to generate second proof");
    
    // REQUIREMENTS TO IMPLEMENT:
    
    // 1. Nullifiers should prevent replay attacks
    // Either nullifiers should be the same (preventing reuse), or
    // the system should track used nullifiers
    let nullifier1 = &proof1.commitments.nullifier;
    let nullifier2 = &proof2.commitments.nullifier;
    
    // Option A: Same nullifier (would need tracking in verifier)
    // Option B: Different nullifiers but system tracks usage
    // For this test, we'll expect the system to handle double-spending prevention
    
    // 2. Both proofs should be valid individually
    let request1 = VerificationRequest {
        proof: proof1,
        min_tier: SubscriptionTier::Basic,
        features: vec!["basic_access".to_string()],
        context: "first_verification".to_string(),
    };
    
    let request2 = VerificationRequest {
        proof: proof2,
        min_tier: SubscriptionTier::Basic,
        features: vec!["basic_access".to_string()],
        context: "second_verification".to_string(),
    };
    
    let result1 = zkp_engine.verify_subscription_proof(&request1).await
        .expect("Failed to verify first proof");
    let result2 = zkp_engine.verify_subscription_proof(&request2).await
        .expect("Failed to verify second proof");
    
    assert!(result1.is_valid);
    assert!(result2.is_valid);
    
    // 3. System should implement nullifier tracking (to be implemented)
    // This will be implemented in the verification system
}

/// Test DID rotation maintains subscription access
#[tokio::test]
async fn test_did_rotation_maintains_subscription() {
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
        "sub_premium_plan".to_string(),
        SubscriptionTier::Premium,
        amount,
        Utc::now() + Duration::days(30),
    ).await.expect("Failed to create subscription");
    
    // Verify access with old DID works
    let proof_old = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "pre_rotation_access",
    ).await.expect("Failed to generate proof with old DID");
    
    let request_old = VerificationRequest {
        proof: proof_old,
        min_tier: SubscriptionTier::Basic,
        features: vec!["premium_access".to_string()],
        context: "pre_rotation".to_string(),
    };
    
    let result_old = zkp_engine.verify_subscription_proof(&request_old).await
        .expect("Failed to verify with old DID");
    assert!(result_old.is_valid);
    
    // Perform DID rotation
    let rotation_signature = b"mock_rotation_signature_proving_ownership".to_vec();
    zkp_engine.rotate_did(
        &subscription.id,
        old_did,
        new_did,
        &rotation_signature,
    ).await.expect("Failed to rotate DID");
    
    // REQUIREMENTS TO IMPLEMENT:
    
    // 1. After rotation, old DID should no longer work for new proofs
    let proof_old_after_rotation = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "post_rotation_old_did",
    ).await;
    
    // This should either fail or the proof should not verify
    // (Implementation detail: may fail at generation or verification)
    
    // 2. New DID should work for generating proofs
    let proof_new = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "post_rotation_new_did",
    ).await.expect("Failed to generate proof with new DID");
    
    let request_new = VerificationRequest {
        proof: proof_new,
        min_tier: SubscriptionTier::Basic,
        features: vec!["premium_access".to_string()],
        context: "post_rotation".to_string(),
    };
    
    let result_new = zkp_engine.verify_subscription_proof(&request_new).await
        .expect("Failed to verify with new DID");
    
    assert!(result_new.is_valid);
    assert!(result_new.tier_sufficient);
    
    // 3. Subscription should maintain same tier and expiry
    // (This would be verified through the proof verification)
}

/// Test performance requirements for ZK proof operations
#[tokio::test]
async fn test_zkp_performance_requirements() {
    // RED PHASE: This test should FAIL initially due to unoptimized implementation
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    
    let did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6";
    let amount = Amount::new(Decimal::new(1999, 2), Currency::Fiat(FiatCurrency::USD))
        .expect("Failed to create amount");
    
    let subscription = zkp_engine.create_anonymous_subscription(
        did.to_string(),
        "sub_performance_test".to_string(),
        SubscriptionTier::Basic,
        amount,
        Utc::now() + Duration::days(30),
    ).await.expect("Failed to create subscription");
    
    // REQUIREMENT 1: ZK proof generation < 500ms
    let start_generation = Instant::now();
    let proof = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "performance_test_context",
    ).await.expect("Failed to generate proof");
    let generation_time = start_generation.elapsed();
    
    assert!(
        generation_time.as_millis() < 500,
        "ZK proof generation took {}ms, requirement is <500ms",
        generation_time.as_millis()
    );
    
    // REQUIREMENT 2: ZK proof verification < 50ms
    let request = VerificationRequest {
        proof,
        min_tier: SubscriptionTier::Basic,
        features: vec!["performance_test".to_string()],
        context: "performance_verification".to_string(),
    };
    
    let start_verification = Instant::now();
    let result = zkp_engine.verify_subscription_proof(&request).await
        .expect("Failed to verify proof");
    let verification_time = start_verification.elapsed();
    
    assert!(result.is_valid);
    assert!(
        verification_time.as_millis() < 50,
        "ZK proof verification took {}ms, requirement is <50ms",
        verification_time.as_millis()
    );
}

/// Test concurrent verification performance (10,000+ verifications)
#[tokio::test]
async fn test_concurrent_verification_performance() {
    // RED PHASE: This test should FAIL initially due to unoptimized concurrent handling
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    
    let did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6";
    let amount = Amount::new(Decimal::new(2999, 2), Currency::Fiat(FiatCurrency::USD))
        .expect("Failed to create amount");
    
    let subscription = zkp_engine.create_anonymous_subscription(
        did.to_string(),
        "sub_concurrent_test".to_string(),
        SubscriptionTier::Premium,
        amount,
        Utc::now() + Duration::days(30),
    ).await.expect("Failed to create subscription");
    
    // Generate a proof to reuse for concurrent verification
    let proof = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "concurrent_test_context",
    ).await.expect("Failed to generate proof");
    
    // REQUIREMENT: Support 10,000+ concurrent verifications
    const CONCURRENT_VERIFICATIONS: usize = 10_000;
    let start_time = Instant::now();
    
    let mut join_set = JoinSet::new();
    
    for i in 0..CONCURRENT_VERIFICATIONS {
        let proof_clone = proof.clone();
        let zkp_engine_ref = &zkp_engine; // Note: This may need Arc<Mutex<>> in implementation
        
        join_set.spawn(async move {
            let request = VerificationRequest {
                proof: proof_clone,
                min_tier: SubscriptionTier::Basic,
                features: vec![format!("concurrent_test_{}", i)],
                context: format!("concurrent_verification_{}", i),
            };
            
            zkp_engine_ref.verify_subscription_proof(&request).await
        });
    }
    
    let mut successful_verifications = 0;
    let mut failed_verifications = 0;
    
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(verification_result)) => {
                if verification_result.is_valid {
                    successful_verifications += 1;
                } else {
                    failed_verifications += 1;
                }
            }
            _ => failed_verifications += 1,
        }
    }
    
    let total_time = start_time.elapsed();
    
    // REQUIREMENTS TO IMPLEMENT:
    
    // 1. All verifications should succeed
    assert_eq!(successful_verifications, CONCURRENT_VERIFICATIONS);
    assert_eq!(failed_verifications, 0);
    
    // 2. Average verification time should be reasonable
    let avg_time_per_verification = total_time.as_millis() / CONCURRENT_VERIFICATIONS as u128;
    assert!(
        avg_time_per_verification < 100, // Allow some overhead for concurrency
        "Average verification time {}ms is too high for concurrent processing",
        avg_time_per_verification
    );
    
    // 3. Total time should indicate good parallelization
    // (This is more of a performance indicator than a hard requirement)
    println!(
        "Completed {} concurrent verifications in {}ms (avg: {}ms per verification)",
        CONCURRENT_VERIFICATIONS,
        total_time.as_millis(),
        avg_time_per_verification
    );
}

/// Test subscription expiry handling in ZK proofs
#[tokio::test]
async fn test_subscription_expiry_in_zkp() {
    // RED PHASE: This test should FAIL initially
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    
    let did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6";
    let amount = Amount::new(Decimal::new(999, 2), Currency::Fiat(FiatCurrency::USD))
        .expect("Failed to create amount");
    
    // Create subscription that expires soon
    let short_expiry = Utc::now() + Duration::seconds(2);
    let subscription = zkp_engine.create_anonymous_subscription(
        did.to_string(),
        "sub_expiry_test".to_string(),
        SubscriptionTier::Basic,
        amount,
        short_expiry,
    ).await.expect("Failed to create subscription");
    
    // Generate proof before expiry
    let proof_before = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "before_expiry_context",
    ).await.expect("Failed to generate proof before expiry");
    
    // Verify proof before expiry
    let request_before = VerificationRequest {
        proof: proof_before,
        min_tier: SubscriptionTier::Basic,
        features: vec!["basic_access".to_string()],
        context: "before_expiry_verification".to_string(),
    };
    
    let result_before = zkp_engine.verify_subscription_proof(&request_before).await
        .expect("Failed to verify proof before expiry");
    assert!(result_before.is_valid);
    
    // Wait for subscription to expire
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // REQUIREMENTS TO IMPLEMENT:
    
    // 1. Proof generation should fail for expired subscription
    let proof_after_result = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "after_expiry_context",
    ).await;
    
    assert!(proof_after_result.is_err());
    if let Err(e) = proof_after_result {
        assert!(format!("{:?}", e).contains("expired") || format!("{:?}", e).contains("Expired"));
    }
    
    // 2. Previously generated proof should also fail verification if checked against current time
    // (This depends on implementation - proof might embed expiry time)
}

/// Test error handling and edge cases
#[tokio::test]
async fn test_zkp_error_handling() {
    // RED PHASE: This test should FAIL initially
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    
    // REQUIREMENT 1: Invalid subscription ID should fail gracefully
    let invalid_proof_result = zkp_engine.generate_subscription_proof(
        "nonexistent_subscription_id",
        SubscriptionTier::Basic,
        "invalid_context",
    ).await;
    
    assert!(invalid_proof_result.is_err());
    
    // REQUIREMENT 2: Corrupted proof should fail verification
    let did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6";
    let amount = Amount::new(Decimal::new(1999, 2), Currency::Fiat(FiatCurrency::USD))
        .expect("Failed to create amount");
    
    let subscription = zkp_engine.create_anonymous_subscription(
        did.to_string(),
        "sub_error_test".to_string(),
        SubscriptionTier::Basic,
        amount,
        Utc::now() + Duration::days(30),
    ).await.expect("Failed to create subscription");
    
    let mut proof = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "error_test_context",
    ).await.expect("Failed to generate proof");
    
    // Corrupt the proof
    if !proof.validity_proof.is_empty() {
        proof.validity_proof[0] = proof.validity_proof[0].wrapping_add(1);
    }
    
    let corrupted_request = VerificationRequest {
        proof,
        min_tier: SubscriptionTier::Basic,
        features: vec!["error_test".to_string()],
        context: "corrupted_proof_test".to_string(),
    };
    
    let corrupted_result = zkp_engine.verify_subscription_proof(&corrupted_request).await;
    
    // Should either return error or invalid result
    match corrupted_result {
        Ok(result) => assert!(!result.is_valid),
        Err(_) => {}, // Both outcomes are acceptable
    }
    
    // REQUIREMENT 3: Insufficient tier should be properly handled
    let high_tier_request = VerificationRequest {
        proof: zkp_engine.generate_subscription_proof(
            &subscription.id,
            SubscriptionTier::Basic,
            "tier_test_context",
        ).await.expect("Failed to generate proof for tier test"),
        min_tier: SubscriptionTier::Enterprise, // Requiring higher tier than subscription has
        features: vec!["enterprise_access".to_string()],
        context: "insufficient_tier_test".to_string(),
    };
    
    let tier_result = zkp_engine.verify_subscription_proof(&high_tier_request).await
        .expect("Failed to verify proof for tier test");
    
    assert!(tier_result.is_valid); // Proof itself is valid
    assert!(!tier_result.tier_sufficient); // But tier is insufficient
}

/// Integration test with synapsed-identity DID management
#[tokio::test]
async fn test_integration_with_synapsed_identity() {
    // RED PHASE: This test should FAIL initially due to missing integration
    let mut zkp_engine = ZKProofEngine::new().expect("Failed to create ZK engine");
    let mut did_manager = DIDManager::new();
    
    let did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6";
    let amount = Amount::new(Decimal::new(4999, 2), Currency::Fiat(FiatCurrency::USD))
        .expect("Failed to create amount");
    
    // Create subscription
    let subscription = zkp_engine.create_anonymous_subscription(
        did.to_string(),
        "sub_integration_test".to_string(),
        SubscriptionTier::Pro,
        amount,
        Utc::now() + Duration::days(30),
    ).await.expect("Failed to create subscription");
    
    // Create DID session
    let session = did_manager.create_session(
        did,
        vec![subscription.id.clone()],
        Duration::hours(1),
    ).await.expect("Failed to create DID session");
    
    // Create access request using DID
    let access_request = DIDAccessRequest {
        did: did.to_string(),
        resource: "api_endpoint".to_string(),
        min_tier: SubscriptionTier::Basic,
        timestamp: Utc::now(),
        signature: b"mock_did_signature".to_vec(),
        session_token: Some(session.session_id.clone()),
    };
    
    // Verify access (should integrate with ZKP engine)
    let access_response = did_manager.verify_access(&access_request, &zkp_engine).await
        .expect("Failed to verify DID access");
    
    // REQUIREMENTS TO IMPLEMENT:
    
    // 1. Access should be granted for valid DID with valid subscription
    assert!(access_response.access_granted);
    assert!(access_response.session_token.is_some());
    
    // 2. Should provide appropriate permissions for Pro tier
    assert!(access_response.permissions.contains(&"api".to_string()));
    assert!(access_response.permissions.contains(&"admin".to_string()));
    
    // 3. Should not reveal subscription details in response
    let response_str = format!("{:?}", access_response);
    assert!(!response_str.contains("sub_integration"));
    assert!(!response_str.contains("4999"));
}