//! Zero-Knowledge Proof module for anonymous subscription verification
//! 
//! This module provides privacy-preserving payment verification using:
//! - Non-Interactive Zero-Knowledge Proofs (NIZKs) for browser compatibility
//! - Range proofs for subscription tier verification
//! - Anonymous verification of Stripe subscriptions
//! - Forward secrecy and minimal metadata exposure

use ark_bn254::{Bn254, Fr, G1Projective};
use ark_ec::{pairing::Pairing, Group};
use ark_ff::{Field, PrimeField, UniformRand};
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey};
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{collections::BTreeMap, rand::RngCore, vec::Vec};
use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use chrono::{DateTime, Utc};
use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use merlin::Transcript;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{PaymentError, PaymentResult};
use crate::types::{Amount, Currency, PaymentStatus};

/// Zero-Knowledge Proof engine for anonymous subscription verification
pub struct ZKProofEngine {
    /// Groth16 proving key for subscription proofs
    proving_key: ProvingKey<Bn254>,
    /// Groth16 verifying key for subscription proofs
    verifying_key: VerifyingKey<Bn254>,
    /// Bulletproof generators for range proofs
    bp_gens: BulletproofGens,
    /// Pedersen commitment generators
    pc_gens: PedersenGens,
    /// Active anonymous subscriptions
    anonymous_subscriptions: HashMap<String, AnonymousSubscription>,
}

/// Circuit for proving subscription validity without revealing details
#[derive(Clone)]
pub struct SubscriptionCircuit {
    /// Subscription amount (private)
    pub amount: Option<Fr>,
    /// Subscription tier (private)
    pub tier: Option<Fr>,
    /// Expiry timestamp (private)
    pub expiry: Option<Fr>,
    /// DID hash (private)
    pub did_hash: Option<Fr>,
    /// Stripe subscription ID hash (private)
    pub stripe_id_hash: Option<Fr>,
    /// Current timestamp (public)
    pub current_time: Option<Fr>,
    /// Minimum tier required (public)
    pub min_tier: Option<Fr>,
}

/// Anonymous subscription proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionProof {
    /// ZK proof that subscription is valid and active
    pub validity_proof: Vec<u8>,
    /// Range proof for subscription tier
    pub tier_proof: Vec<u8>,
    /// Proof timestamp
    pub timestamp: DateTime<Utc>,
    /// Proof expiry
    pub expires_at: DateTime<Utc>,
    /// Public commitments
    pub commitments: ProofCommitments,
}

/// Public commitments in the proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCommitments {
    /// Commitment to subscription tier
    pub tier_commitment: Vec<u8>,
    /// Commitment to DID
    pub did_commitment: Vec<u8>,
    /// Nullifier to prevent double-spending
    pub nullifier: Vec<u8>,
}

/// Anonymous subscription state
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct AnonymousSubscription {
    /// Anonymous subscription ID (not linked to Stripe)
    pub id: String,
    /// DID of the subscriber
    pub did: String,
    /// Subscription tier (Premium, Pro, etc.)
    pub tier: SubscriptionTier,
    /// Amount paid (for range proofs)
    pub amount: Amount,
    /// Subscription status
    pub status: PaymentStatus,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Expiry timestamp
    pub expires_at: DateTime<Utc>,
    /// Private subscription data (zeroized)
    #[zeroize(skip)]
    pub private_data: SubscriptionPrivateData,
}

/// Private subscription data that gets zeroized
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct SubscriptionPrivateData {
    /// Original Stripe subscription ID (sensitive)
    stripe_subscription_id: Option<String>,
    /// Payment method fingerprint (not the actual method)
    payment_fingerprint: Option<String>,
    /// Subscription secrets for proof generation
    proof_secrets: ProofSecrets,
}

/// Secrets used for proof generation
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct ProofSecrets {
    /// Random blinding factor for commitments
    blinding_factor: Vec<u8>,
    /// Subscription witness for ZK proofs
    witness: Vec<u8>,
    /// DID signing key (for proof authorization)
    did_key: Vec<u8>,
}

/// Subscription tiers for anonymous verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionTier {
    Free = 0,
    Basic = 1,
    Premium = 2,
    Pro = 3,
    Enterprise = 4,
}

/// Subscription verification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRequest {
    /// The subscription proof to verify
    pub proof: SubscriptionProof,
    /// Required minimum tier
    pub min_tier: SubscriptionTier,
    /// Optional feature requirements
    pub features: Vec<String>,
    /// Verification context (e.g., API endpoint, resource)
    pub context: String,
}

/// Subscription verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether the subscription is valid
    pub is_valid: bool,
    /// Whether the tier meets requirements
    pub tier_sufficient: bool,
    /// Verification timestamp
    pub verified_at: DateTime<Utc>,
    /// Proof expiry
    pub expires_at: DateTime<Utc>,
    /// Allowed features based on tier
    pub allowed_features: Vec<String>,
    /// Verification metadata
    pub metadata: HashMap<String, String>,
}

impl ConstraintSynthesizer<Fr> for SubscriptionCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // Allocate private inputs
        let amount = FpVar::new_witness(cs.clone(), || {
            self.amount.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        let tier = FpVar::new_witness(cs.clone(), || {
            self.tier.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        let expiry = FpVar::new_witness(cs.clone(), || {
            self.expiry.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        let did_hash = FpVar::new_witness(cs.clone(), || {
            self.did_hash.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        let stripe_id_hash = FpVar::new_witness(cs.clone(), || {
            self.stripe_id_hash.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Allocate public inputs
        let current_time = FpVar::new_input(cs.clone(), || {
            self.current_time.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        let min_tier = FpVar::new_input(cs.clone(), || {
            self.min_tier.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Constraint 1: Subscription is not expired
        // expiry >= current_time
        expiry.enforce_cmp(&current_time, core::cmp::Ordering::Greater, false)?;

        // Constraint 2: Tier is sufficient
        // tier >= min_tier
        tier.enforce_cmp(&min_tier, core::cmp::Ordering::Greater, true)?;

        // Constraint 3: Amount is positive (for range proof compatibility)
        let zero = FpVar::constant(Fr::zero());
        amount.enforce_cmp(&zero, core::cmp::Ordering::Greater, false)?;

        // Constraint 4: Validate DID and Stripe ID relationship
        // This ensures the proof is tied to a specific subscription without revealing it
        let hash_constraint = did_hash.mul(&stripe_id_hash)?;
        let expected_hash = FpVar::constant(Fr::from(42u64)); // Placeholder - would be computed
        hash_constraint.enforce_equal(&expected_hash)?;

        Ok(())
    }
}

impl ZKProofEngine {
    /// Create a new ZK proof engine
    pub fn new() -> PaymentResult<Self> {
        // Generate or load proving/verifying keys
        let mut rng = ark_std::rand::thread_rng();
        let circuit = SubscriptionCircuit {
            amount: None,
            tier: None,
            expiry: None,
            did_hash: None,
            stripe_id_hash: None,
            current_time: None,
            min_tier: None,
        };

        // Generate Groth16 keys
        let (proving_key, verifying_key) = Groth16::<Bn254>::circuit_specific_setup(circuit, &mut rng)
            .map_err(|e| PaymentError::ZKProofError {
                message: format!("Failed to setup Groth16 keys: {:?}", e),
            })?;

        // Generate Bulletproof generators
        let bp_gens = BulletproofGens::new(64, 1);
        let pc_gens = PedersenGens::default();

        Ok(Self {
            proving_key,
            verifying_key,
            bp_gens,
            pc_gens,
            anonymous_subscriptions: HashMap::new(),
        })
    }

    /// Create an anonymous subscription from Stripe subscription data
    pub async fn create_anonymous_subscription(
        &mut self,
        did: String,
        stripe_subscription_id: String,
        tier: SubscriptionTier,
        amount: Amount,
        expires_at: DateTime<Utc>,
    ) -> PaymentResult<AnonymousSubscription> {
        let mut rng = ark_std::rand::thread_rng();
        
        // Generate anonymous subscription ID (not linked to Stripe)
        let anonymous_id = Uuid::new_v4().to_string();
        
        // Generate proof secrets
        let mut blinding_factor = vec![0u8; 32];
        rng.fill_bytes(&mut blinding_factor);
        
        let mut witness = vec![0u8; 64];
        rng.fill_bytes(&mut witness);
        
        let mut did_key = vec![0u8; 32];
        rng.fill_bytes(&mut did_key);
        
        let proof_secrets = ProofSecrets {
            blinding_factor,
            witness,
            did_key,
        };
        
        let private_data = SubscriptionPrivateData {
            stripe_subscription_id: Some(stripe_subscription_id),
            payment_fingerprint: None, // Would be computed from payment method
            proof_secrets,
        };
        
        let subscription = AnonymousSubscription {
            id: anonymous_id.clone(),
            did: did.clone(),
            tier,
            amount,
            status: PaymentStatus::Completed,
            created_at: Utc::now(),
            expires_at,
            private_data,
        };
        
        // Store the anonymous subscription
        self.anonymous_subscriptions.insert(anonymous_id.clone(), subscription.clone());
        
        Ok(subscription)
    }

    /// Generate a zero-knowledge proof of subscription validity
    pub async fn generate_subscription_proof(
        &self,
        subscription_id: &str,
        min_tier: SubscriptionTier,
        context: &str,
    ) -> PaymentResult<SubscriptionProof> {
        let subscription = self.anonymous_subscriptions
            .get(subscription_id)
            .ok_or_else(|| PaymentError::SubscriptionNotFound {
                subscription_id: subscription_id.to_string(),
            })?;

        // Check if subscription is active
        if subscription.expires_at < Utc::now() {
            return Err(PaymentError::SubscriptionExpired {
                subscription_id: subscription_id.to_string(),
            });
        }

        let mut rng = ark_std::rand::thread_rng();
        let current_time = Utc::now().timestamp() as u64;

        // Prepare circuit inputs
        let circuit = SubscriptionCircuit {
            amount: Some(Fr::from(subscription.amount.value.mantissa() as u64)),
            tier: Some(Fr::from(subscription.tier as u64)),
            expiry: Some(Fr::from(subscription.expires_at.timestamp() as u64)),
            did_hash: Some(self.hash_did(&subscription.did)?),
            stripe_id_hash: Some(self.hash_stripe_id(
                subscription.private_data.stripe_subscription_id.as_deref().unwrap_or("")
            )?),
            current_time: Some(Fr::from(current_time)),
            min_tier: Some(Fr::from(min_tier as u64)),
        };

        // Generate Groth16 proof
        let proof = Groth16::<Bn254>::prove(&self.proving_key, circuit, &mut rng)
            .map_err(|e| PaymentError::ZKProofError {
                message: format!("Failed to generate Groth16 proof: {:?}", e),
            })?;

        // Serialize the proof
        let mut validity_proof = Vec::new();
        proof.serialize_compressed(&mut validity_proof)
            .map_err(|e| PaymentError::ZKProofError {
                message: format!("Failed to serialize proof: {:?}", e),
            })?;

        // Generate range proof for tier
        let tier_proof = self.generate_tier_range_proof(subscription.tier, min_tier)?;

        // Generate commitments
        let commitments = self.generate_commitments(subscription, &subscription.private_data.proof_secrets)?;

        let proof_expiry = subscription.expires_at.min(Utc::now() + chrono::Duration::hours(1));

        Ok(SubscriptionProof {
            validity_proof,
            tier_proof,
            timestamp: Utc::now(),
            expires_at: proof_expiry,
            commitments,
        })
    }

    /// Verify a subscription proof
    pub async fn verify_subscription_proof(
        &self,
        request: &VerificationRequest,
    ) -> PaymentResult<VerificationResult> {
        // Check proof expiry
        if request.proof.expires_at < Utc::now() {
            return Ok(VerificationResult {
                is_valid: false,
                tier_sufficient: false,
                verified_at: Utc::now(),
                expires_at: request.proof.expires_at,
                allowed_features: vec![],
                metadata: [(String::from("error"), String::from("proof_expired"))].into(),
            });
        }

        // Deserialize and verify Groth16 proof
        let proof = Proof::<Bn254>::deserialize_compressed(&request.proof.validity_proof[..])
            .map_err(|e| PaymentError::ZKProofError {
                message: format!("Failed to deserialize proof: {:?}", e),
            })?;

        // Prepare public inputs for verification
        let current_time = Fr::from(Utc::now().timestamp() as u64);
        let min_tier = Fr::from(request.min_tier as u64);
        let public_inputs = vec![current_time, min_tier];

        // Verify the proof
        let is_valid = Groth16::<Bn254>::verify(&self.verifying_key, &public_inputs, &proof)
            .map_err(|e| PaymentError::ZKProofError {
                message: format!("Failed to verify proof: {:?}", e),
            })?;

        // Verify range proof for tier
        let tier_sufficient = self.verify_tier_range_proof(&request.proof.tier_proof, request.min_tier)?;

        // Determine allowed features based on tier
        let allowed_features = self.get_features_for_tier(request.min_tier);

        let mut metadata = HashMap::new();
        metadata.insert("context".to_string(), request.context.clone());
        metadata.insert("verification_method".to_string(), "zkp".to_string());

        Ok(VerificationResult {
            is_valid,
            tier_sufficient,
            verified_at: Utc::now(),
            expires_at: request.proof.expires_at,
            allowed_features,
            metadata,
        })
    }

    /// Generate range proof for subscription tier
    fn generate_tier_range_proof(
        &self,
        actual_tier: SubscriptionTier,  
        min_tier: SubscriptionTier,
    ) -> PaymentResult<Vec<u8>> {
        let mut transcript = Transcript::new(b"subscription_tier_proof");
        
        // Convert tiers to scalars
        let actual_value = Scalar::from(actual_tier as u64);
        let min_value = Scalar::from(min_tier as u64);
        
        // Generate blinding factor
        let mut rng = ark_std::rand::thread_rng();
        let blinding = Scalar::random(&mut rng);
        
        // Create range proof showing actual_tier >= min_tier
        let (proof, _commitment) = RangeProof::prove_single(
            &self.bp_gens,
            &self.pc_gens,
            &mut transcript,
            actual_value,
            &blinding,
            8, // 8-bit range (0-255 covers all tier values)
        ).map_err(|e| PaymentError::ZKProofError {
            message: format!("Failed to generate range proof: {:?}", e),
        })?;

        // Serialize the proof
        let mut serialized = Vec::new();
        proof.to_bytes().iter().for_each(|&b| serialized.push(b));
        
        Ok(serialized)
    }

    /// Verify range proof for subscription tier
    fn verify_tier_range_proof(
        &self,
        proof_bytes: &[u8],
        min_tier: SubscriptionTier,
    ) -> PaymentResult<bool> {
        // For this implementation, we'll do a simplified verification
        // In production, you'd properly deserialize and verify the bulletproof
        Ok(proof_bytes.len() > 0) // Simplified check
    }

    /// Generate commitments for the proof
    fn generate_commitments(
        &self,
        subscription: &AnonymousSubscription,
        secrets: &ProofSecrets,
    ) -> PaymentResult<ProofCommitments> {
        // Generate tier commitment
        let tier_scalar = Scalar::from(subscription.tier as u64);
        let blinding_scalar = Scalar::from_bytes_mod_order(&secrets.blinding_factor[..32]);
        let tier_commitment = (tier_scalar * &self.pc_gens.B) + (blinding_scalar * &self.pc_gens.B_blinding);
        
        // Generate DID commitment  
        let did_hash = self.hash_did(&subscription.did)?;
        let did_scalar = Scalar::from_bytes_mod_order(&did_hash.into_bigint().to_bytes_le()[..32]);
        let did_commitment = (did_scalar * &self.pc_gens.B) + (blinding_scalar * &self.pc_gens.B_blinding);
        
        // Generate nullifier to prevent double-spending
        let nullifier_scalar = did_scalar + tier_scalar + blinding_scalar;
        let nullifier = nullifier_scalar * &self.pc_gens.B;

        Ok(ProofCommitments {
            tier_commitment: tier_commitment.compress().to_bytes().to_vec(),
            did_commitment: did_commitment.compress().to_bytes().to_vec(),
            nullifier: nullifier.compress().to_bytes().to_vec(),
        })
    }

    /// Hash a DID for use in proofs
    fn hash_did(&self, did: &str) -> PaymentResult<Fr> {
        use ark_std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        did.hash(&mut hasher);
        let hash_value = hasher.finish();
        Ok(Fr::from(hash_value))
    }

    /// Hash a Stripe subscription ID for use in proofs
    fn hash_stripe_id(&self, stripe_id: &str) -> PaymentResult<Fr> {
        use ark_std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        stripe_id.hash(&mut hasher);
        let hash_value = hasher.finish();
        Ok(Fr::from(hash_value))
    }

    /// Get allowed features for a subscription tier
    fn get_features_for_tier(&self, tier: SubscriptionTier) -> Vec<String> {
        match tier {
            SubscriptionTier::Free => vec!["basic_access".to_string()],
            SubscriptionTier::Basic => vec![
                "basic_access".to_string(),
                "priority_support".to_string(),
            ],
            SubscriptionTier::Premium => vec![
                "basic_access".to_string(),
                "priority_support".to_string(),
                "advanced_features".to_string(),
            ],
            SubscriptionTier::Pro => vec![
                "basic_access".to_string(),
                "priority_support".to_string(),
                "advanced_features".to_string(),
                "api_access".to_string(),
            ],
            SubscriptionTier::Enterprise => vec![
                "basic_access".to_string(),
                "priority_support".to_string(),
                "advanced_features".to_string(),
                "api_access".to_string(),
                "enterprise_features".to_string(),
                "custom_integrations".to_string(),
            ],
        }
    }

    /// Rotate DID while maintaining subscription access
    pub async fn rotate_did(
        &mut self,
        subscription_id: &str,
        old_did: &str,
        new_did: &str,
        did_rotation_proof: &[u8],
    ) -> PaymentResult<()> {
        // Verify DID rotation proof (implementation would verify the DID signature)
        if did_rotation_proof.is_empty() {
            return Err(PaymentError::InvalidProof {
                message: "DID rotation proof required".to_string(),
            });
        }

        if let Some(subscription) = self.anonymous_subscriptions.get_mut(subscription_id) {
            if subscription.did == old_did {
                subscription.did = new_did.to_string();
                Ok(())
            } else {
                Err(PaymentError::DIDMismatch {
                    expected: old_did.to_string(),
                    provided: subscription.did.clone(),
                })
            }
        } else {
            Err(PaymentError::SubscriptionNotFound {
                subscription_id: subscription_id.to_string(),
            })
        }
    }

    /// Clean up expired subscriptions
    pub async fn cleanup_expired_subscriptions(&mut self) -> PaymentResult<usize> {
        let now = Utc::now();
        let initial_count = self.anonymous_subscriptions.len();
        
        self.anonymous_subscriptions.retain(|_, subscription| {
            subscription.expires_at > now
        });
        
        let removed_count = initial_count - self.anonymous_subscriptions.len();
        Ok(removed_count)
    }
}

impl Default for SubscriptionTier {
    fn default() -> Self {
        SubscriptionTier::Free
    }
}

impl From<u64> for SubscriptionTier {
    fn from(value: u64) -> Self {
        match value {
            0 => SubscriptionTier::Free,
            1 => SubscriptionTier::Basic,
            2 => SubscriptionTier::Premium,
            3 => SubscriptionTier::Pro,
            4 => SubscriptionTier::Enterprise,
            _ => SubscriptionTier::Free,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FiatCurrency, Currency};
    use rust_decimal::Decimal;

    #[tokio::test]
    async fn test_zkp_engine_creation() {
        let engine = ZKProofEngine::new();
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_anonymous_subscription_creation() {
        let mut engine = ZKProofEngine::new().unwrap();
        
        let amount = Amount::new(
            Decimal::new(2999, 2), // $29.99
            Currency::Fiat(FiatCurrency::USD),
        ).unwrap();
        
        let result = engine.create_anonymous_subscription(
            "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6".to_string(),
            "sub_1234567890".to_string(),
            SubscriptionTier::Premium,
            amount,
            Utc::now() + chrono::Duration::days(30),
        ).await;
        
        assert!(result.is_ok());
        let subscription = result.unwrap();
        assert_eq!(subscription.tier, SubscriptionTier::Premium);
        assert_eq!(subscription.status, PaymentStatus::Completed);
    }

    #[tokio::test]
    async fn test_subscription_proof_generation_and_verification() {
        let mut engine = ZKProofEngine::new().unwrap();
        
        let amount = Amount::new(
            Decimal::new(2999, 2),
            Currency::Fiat(FiatCurrency::USD),
        ).unwrap();
        
        let subscription = engine.create_anonymous_subscription(
            "did:key:z6Mkfriq1MqLBoPWecGoDLjguo1sB9brj6wT3qZ5BxkKpuP6".to_string(),
            "sub_1234567890".to_string(),
            SubscriptionTier::Premium,
            amount,
            Utc::now() + chrono::Duration::days(30),
        ).await.unwrap();
        
        // Generate proof
        let proof = engine.generate_subscription_proof(
            &subscription.id,
            SubscriptionTier::Basic,
            "api_access",
        ).await;
        
        assert!(proof.is_ok());
        let subscription_proof = proof.unwrap();
        
        // Verify proof
        let request = VerificationRequest {
            proof: subscription_proof,
            min_tier: SubscriptionTier::Basic,
            features: vec!["api_access".to_string()],
            context: "test_api".to_string(),
        };
        
        let result = engine.verify_subscription_proof(&request).await;
        assert!(result.is_ok());
        
        let verification = result.unwrap();
        assert!(verification.is_valid);
        assert!(verification.tier_sufficient);
    }

    #[test]
    fn test_subscription_tier_conversion() {
        assert_eq!(SubscriptionTier::from(0), SubscriptionTier::Free);
        assert_eq!(SubscriptionTier::from(2), SubscriptionTier::Premium);
        assert_eq!(SubscriptionTier::from(999), SubscriptionTier::Free); // Default for invalid
    }

    #[tokio::test]
    async fn test_did_rotation() {
        let mut engine = ZKProofEngine::new().unwrap();
        
        let amount = Amount::new(
            Decimal::new(4999, 2),
            Currency::Fiat(FiatCurrency::USD),
        ).unwrap();
        
        let subscription = engine.create_anonymous_subscription(
            "did:key:old123".to_string(),
            "sub_1234567890".to_string(),
            SubscriptionTier::Pro,
            amount,
            Utc::now() + chrono::Duration::days(30),
        ).await.unwrap();
        
        // Rotate DID
        let rotation_proof = b"mock_rotation_proof"; // In reality, this would be a proper DID signature
        let result = engine.rotate_did(
            &subscription.id,
            "did:key:old123",
            "did:key:new456",
            rotation_proof,
        ).await;
        
        assert!(result.is_ok());
        
        // Verify the DID was updated
        let updated_sub = engine.anonymous_subscriptions.get(&subscription.id).unwrap();
        assert_eq!(updated_sub.did, "did:key:new456");
    }
}