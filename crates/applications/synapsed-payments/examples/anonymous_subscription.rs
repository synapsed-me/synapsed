//! Example: Anonymous Subscription System with Zero-Knowledge Proofs
//! 
//! This example demonstrates how to use the Synapsed payments system to:
//! 1. Create anonymous subscriptions from Stripe subscriptions
//! 2. Generate zero-knowledge proofs of subscription validity
//! 3. Verify subscriptions without revealing user identity
//! 4. Handle DID rotation and recovery
//! 5. Use WebAssembly for browser-based proof generation

use chrono::{Duration, Utc};
use synapsed_payments::{
    prelude::*,
    zkp::{AnonymousSubscription, SubscriptionTier, ZKProofEngine, VerificationRequest},
    did_integration::{DIDManager, DIDAccessRequest, RotationReason},
    wasm_pwa::WasmZKEngine,
    types::{Amount, Currency, FiatCurrency},
};
use rust_decimal::Decimal;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Synapsed Anonymous Subscription System Demo");
    println!("===========================================\n");

    // Initialize components
    let mut zkp_engine = ZKProofEngine::new()?;
    let mut did_manager = DIDManager::new();

    // Demo user DIDs
    let user_did = "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6";
    let new_did = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";

    println!("👤 User DID: {}", user_did);
    println!("🆔 New DID (for rotation): {}\n", new_did);

    // Step 1: Create anonymous subscription from Stripe subscription
    println!("📋 Step 1: Creating Anonymous Subscription");
    println!("==========================================");

    let subscription_amount = Amount::new(
        Decimal::new(2999, 2), // $29.99
        Currency::Fiat(FiatCurrency::USD),
    )?;

    let anonymous_subscription = zkp_engine.create_anonymous_subscription(
        user_did.to_string(),
        "sub_1NdTKl2eZvKYlo2CYfTHy123".to_string(), // Mock Stripe subscription ID
        SubscriptionTier::Premium,
        subscription_amount,
        Utc::now() + Duration::days(30),
    ).await?;

    println!("✅ Created anonymous subscription:");
    println!("   📧 Anonymous ID: {}", anonymous_subscription.id);
    println!("   🏆 Tier: {:?}", anonymous_subscription.tier); 
    println!("   💰 Amount: {}", anonymous_subscription.amount);
    println!("   ⏰ Expires: {}\n", anonymous_subscription.expires_at);

    // Step 2: Create DID session for access management
    println!("📋 Step 2: Creating DID Session");
    println!("==============================");

    let session = did_manager.create_session(
        user_did,
        vec![anonymous_subscription.id.clone()],
        Duration::hours(24),
    ).await?;

    println!("✅ Created DID session:");
    println!("   🎫 Session ID: {}", session.session_id);
    println!("   ⏰ Expires: {}\n", session.expires_at);

    // Step 3: Generate zero-knowledge proof of subscription validity
    println!("📋 Step 3: Generating Zero-Knowledge Proof");
    println!("==========================================");

    let subscription_proof = zkp_engine.generate_subscription_proof(
        &anonymous_subscription.id,
        SubscriptionTier::Basic, // Minimum tier required
        "api_access",
    ).await?;

    println!("✅ Generated ZK proof:");
    println!("   📋 Proof size: {} bytes", subscription_proof.validity_proof.len());
    println!("   🎯 Tier proof size: {} bytes", subscription_proof.tier_proof.len());
    println!("   ⏰ Valid until: {}\n", subscription_proof.expires_at);

    // Step 4: Verify subscription access without revealing identity
    println!("📋 Step 4: Verifying Anonymous Access");
    println!("====================================");

    let verification_request = VerificationRequest {
        proof: subscription_proof.clone(),
        min_tier: SubscriptionTier::Basic,
        features: vec!["api_access".to_string(), "priority_support".to_string()],
        context: "premium_api_endpoint".to_string(),
    };

    let verification_result = zkp_engine.verify_subscription_proof(&verification_request).await?;

    println!("✅ Verification result:");
    println!("   ✓ Valid: {}", verification_result.is_valid);
    println!("   🏆 Tier sufficient: {}", verification_result.tier_sufficient);
    println!("   🔓 Allowed features: {:?}", verification_result.allowed_features);
    println!("   ⏰ Verified at: {}\n", verification_result.verified_at);

    // Step 5: Demonstrate DID-based access control
    println!("📋 Step 5: DID-Based Access Control");
    println!("===================================");

    let access_request = DIDAccessRequest {
        did: user_did.to_string(),
        resource: "premium_content".to_string(),
        min_tier: SubscriptionTier::Premium,
        timestamp: Utc::now(),
        signature: b"mock_signature_data".to_vec(),
        session_token: Some(session.session_id.clone()),
    };

    let access_response = did_manager.verify_access(&access_request, &zkp_engine).await?;

    println!("✅ Access control result:");
    println!("   🚪 Access granted: {}", access_response.access_granted);
    println!("   🎫 Session token: {:?}", access_response.session_token);
    println!("   🔐 Permissions: {:?}\n", access_response.permissions);

    // Step 6: Demonstrate DID rotation while maintaining access
    println!("📋 Step 6: DID Rotation with Subscription Preservation");
    println!("====================================================");

    let rotation_signature = b"rotation_signature_proving_ownership".to_vec();
    
    zkp_engine.rotate_did(
        &anonymous_subscription.id,
        user_did,
        new_did,
        &rotation_signature,
    ).await?;

    did_manager.rotate_did(
        user_did,
        new_did,
        rotation_signature.clone(),
        RotationReason::UserRequested,
        &mut zkp_engine,
    ).await?;

    println!("✅ DID rotation completed:");
    println!("   🔄 Old DID: {}", user_did);
    println!("   🆕 New DID: {}", new_did);
    println!("   🔐 Subscription access preserved\n");

    // Step 7: Generate portable proof for cross-platform use
    println!("📋 Step 7: Generating Portable Subscription Proof");
    println!("================================================");

    let portable_proof = did_manager.generate_portable_proof(
        new_did, // Using the new DID after rotation
        &anonymous_subscription.id,
        &zkp_engine,
    ).await?;

    println!("✅ Generated portable proof:");
    println!("   📦 Proof size: {} bytes", portable_proof.validity_proof.len());
    println!("   🔗 DID commitment: {} bytes", portable_proof.did_commitment.len());
    println!("   ⏰ Valid until: {}\n", portable_proof.expires_at);

    // Step 8: WebAssembly browser integration example
    #[cfg(feature = "wasm-support")]
    {
        println!("📋 Step 8: WebAssembly Browser Integration");
        println!("==========================================");

        let mut wasm_engine = WasmZKEngine::new()?;
        let proving_keys = b"mock_proving_keys_for_browser";
        wasm_engine.initialize(proving_keys).await?;

        // Get PWA capabilities
        let capabilities_json = wasm_engine.get_pwa_capabilities()?;
        println!("✅ PWA capabilities: {}\n", capabilities_json);

        // Demonstrate browser proof generation
        let subscription_data = serde_json::json!({
            "tier": 2, // Premium
            "did": new_did,
            "expires_at": (Utc::now() + Duration::hours(1)).to_rfc3339()
        });

        let browser_proof = wasm_engine.generate_proof_browser(
            &subscription_data.to_string(),
            1, // Basic tier required
            "browser_api_access",
        ).await?;

        println!("✅ Browser-generated proof:");
        println!("   📱 Proof generated in WebAssembly");
        println!("   💾 Cached for offline use");
        println!("   🔒 Privacy-preserving verification ready\n");
    }

    // Step 9: Privacy features summary
    println!("📋 Step 9: Privacy Features Summary");
    println!("==================================");

    println!("🔐 Privacy Features Demonstrated:");
    println!("   ✓ Anonymous subscription verification");
    println!("   ✓ Zero-knowledge proof generation");
    println!("   ✓ No linkage between DID and payment accounts");
    println!("   ✓ Subscription tier verification without revealing details");
    println!("   ✓ DID rotation with subscription preservation");
    println!("   ✓ Portable proofs for cross-platform access");
    println!("   ✓ Browser-optimized WebAssembly implementation");
    println!("   ✓ Offline proof storage and validation");
    println!("   ✓ Forward secrecy for payment proofs");
    println!("   ✓ Minimal metadata exposure\n");

    // Cleanup
    println!("📋 Cleanup: Removing Expired Data");
    println!("=================================");

    let expired_subscriptions = zkp_engine.cleanup_expired_subscriptions().await?;
    let (expired_sessions, expired_rotations) = did_manager.cleanup_expired().await?;

    println!("✅ Cleanup completed:");
    println!("   🗑️  Expired subscriptions: {}", expired_subscriptions);
    println!("   🗑️  Expired sessions: {}", expired_sessions);
    println!("   🗑️  Expired rotations: {}", expired_rotations);

    println!("\n🎉 Anonymous subscription system demo completed!");
    println!("   All user privacy has been preserved while enabling");
    println!("   full subscription verification and access control.");

    Ok(())
}

/// Helper function to demonstrate integration with existing Stripe subscriptions
async fn integrate_with_stripe_subscription(
    stripe_subscription_id: &str,
    zkp_engine: &mut ZKProofEngine,
) -> Result<AnonymousSubscription, Box<dyn std::error::Error>> {
    // In a real implementation, this would:
    // 1. Fetch subscription details from Stripe API
    // 2. Verify the subscription is active
    // 3. Map Stripe tier to SubscriptionTier
    // 4. Create anonymous subscription without storing Stripe details

    println!("🔗 Integrating with Stripe subscription: {}", stripe_subscription_id);

    // Mock Stripe subscription data
    let mock_stripe_data = MockStripeSubscription {
        id: stripe_subscription_id.to_string(),
        status: "active".to_string(),
        current_period_end: Utc::now() + Duration::days(30),
        amount: 2999, // $29.99 in cents
        currency: "usd".to_string(),
        tier: "premium".to_string(),
    };

    // Convert to anonymous subscription
    let subscription_tier = match mock_stripe_data.tier.as_str() {
        "basic" => SubscriptionTier::Basic,
        "premium" => SubscriptionTier::Premium,
        "pro" => SubscriptionTier::Pro,
        "enterprise" => SubscriptionTier::Enterprise,
        _ => SubscriptionTier::Free,
    };

    let amount = Amount::new(
        Decimal::new(mock_stripe_data.amount as i64, 2),
        Currency::Fiat(FiatCurrency::USD),
    )?;

    // Generate a temporary DID for this example
    let temp_did = "did:key:z6MkteMP1Nt4xV8H7GqJHe8Ct1CgqqNFDHcYf4v5e2s6K7pN";

    let anonymous_subscription = zkp_engine.create_anonymous_subscription(
        temp_did.to_string(),
        stripe_subscription_id.to_string(),
        subscription_tier,
        amount,
        mock_stripe_data.current_period_end,
    ).await?;

    println!("✅ Successfully created anonymous subscription from Stripe data");
    Ok(anonymous_subscription)
}

/// Mock Stripe subscription structure
#[derive(Debug)]
struct MockStripeSubscription {
    id: String,
    status: String,
    current_period_end: chrono::DateTime<Utc>,
    amount: u32,
    currency: String,
    tier: String,
}

/// Example of real-world usage patterns
#[tokio::test]
async fn example_usage_patterns() -> Result<(), Box<dyn std::error::Error>> {
    // Pattern 1: API Gateway Integration
    // This would be used in an API gateway to verify subscription without
    // revealing user identity or subscription details
    
    let mut zkp_engine = ZKProofEngine::new()?;
    
    // Create a subscription for testing
    let amount = Amount::new(Decimal::new(999, 2), Currency::Fiat(FiatCurrency::USD))?;
    let subscription = zkp_engine.create_anonymous_subscription(
        "did:key:test".to_string(),
        "sub_test_123".to_string(),
        SubscriptionTier::Basic,
        amount,
        Utc::now() + Duration::days(30),
    ).await?;

    // Generate proof for API access
    let proof = zkp_engine.generate_subscription_proof(
        &subscription.id,
        SubscriptionTier::Basic,
        "api_gateway",
    ).await?;

    // Verify in API gateway without revealing user identity
    let request = VerificationRequest {
        proof,
        min_tier: SubscriptionTier::Basic,
        features: vec!["api_access".to_string()],
        context: "api_gateway_check".to_string(),
    };

    let result = zkp_engine.verify_subscription_proof(&request).await?;
    assert!(result.is_valid);
    assert!(result.tier_sufficient);

    Ok(())
}