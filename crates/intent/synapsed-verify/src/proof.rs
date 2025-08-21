//! Cryptographic proof generation for verification

use crate::{types::*, Result, VerifyError};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use blake3;
use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier as Ed25519Verifier};
use rand::rngs::OsRng;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Cryptographic proof of verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationProof {
    /// Proof ID
    pub id: Uuid,
    /// Verifications included in this proof
    pub verifications: Vec<VerificationSummary>,
    /// Merkle root of verifications
    pub merkle_root: String,
    /// Digital signature of the proof
    pub signature: Option<ProofSignature>,
    /// Timestamp when proof was generated
    pub timestamp: DateTime<Utc>,
    /// Agent that generated the proof
    pub prover: Option<String>,
    /// Proof metadata
    pub metadata: ProofMetadata,
}

/// Summary of a verification for proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationSummary {
    /// Verification ID
    pub id: Uuid,
    /// Type of verification
    pub verification_type: VerificationType,
    /// Whether it passed
    pub success: bool,
    /// Hash of the full verification
    pub hash: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Digital signature for a proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofSignature {
    /// Public key used for signing
    pub public_key: Vec<u8>,
    /// The signature
    pub signature: Vec<u8>,
    /// Algorithm used
    pub algorithm: SignatureAlgorithm,
}

/// Signature algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    Ed25519,
    // Future: Add post-quantum algorithms
}

/// Metadata about a proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofMetadata {
    /// Intent ID this proof relates to
    pub intent_id: Option<Uuid>,
    /// Agent context
    pub agent_context: Option<String>,
    /// Chain height (for proof chains)
    pub chain_height: u64,
    /// Previous proof in chain
    pub previous_proof: Option<Uuid>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Chain of proofs for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofChain {
    /// Chain ID
    pub id: Uuid,
    /// Genesis proof
    pub genesis: VerificationProof,
    /// All proofs in the chain
    pub proofs: Vec<VerificationProof>,
    /// Current chain head
    pub head: Uuid,
    /// Chain metadata
    pub metadata: ChainMetadata,
}

/// Metadata for a proof chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMetadata {
    /// When chain was created
    pub created_at: DateTime<Utc>,
    /// Last update time
    pub updated_at: DateTime<Utc>,
    /// Total verifications in chain
    pub total_verifications: usize,
    /// Chain purpose/context
    pub purpose: String,
}

/// Proof generator for creating cryptographic proofs
pub struct ProofGenerator {
    /// Signing keypair
    keypair: Option<Keypair>,
    /// Proof chains
    chains: HashMap<Uuid, ProofChain>,
    /// Individual proofs
    proofs: HashMap<Uuid, VerificationProof>,
}

impl ProofGenerator {
    /// Creates a new proof generator
    pub fn new() -> Self {
        Self {
            keypair: None,
            chains: HashMap::new(),
            proofs: HashMap::new(),
        }
    }
    
    /// Creates a proof generator with signing capability
    pub fn with_signing() -> Self {
        let mut csprng = OsRng;
        let keypair = Keypair::generate(&mut csprng);
        
        Self {
            keypair: Some(keypair),
            chains: HashMap::new(),
            proofs: HashMap::new(),
        }
    }
    
    /// Generates a proof for verifications
    pub async fn generate_proof(
        &mut self,
        verifications: Vec<VerificationResult>,
    ) -> Result<VerificationProof> {
        self.generate_proof_with_metadata(
            verifications,
            ProofMetadata {
                intent_id: None,
                agent_context: None,
                chain_height: 0,
                previous_proof: None,
                tags: Vec::new(),
            }
        ).await
    }
    
    /// Generates a proof with metadata
    pub async fn generate_proof_with_metadata(
        &mut self,
        verifications: Vec<VerificationResult>,
        metadata: ProofMetadata,
    ) -> Result<VerificationProof> {
        let id = Uuid::new_v4();
        let timestamp = Utc::now();
        
        // Create verification summaries
        let summaries: Vec<VerificationSummary> = verifications
            .iter()
            .map(|v| VerificationSummary {
                id: v.id,
                verification_type: v.verification_type,
                success: v.success,
                hash: Self::hash_verification(v),
                timestamp: v.timestamp,
            })
            .collect();
        
        // Calculate Merkle root
        let merkle_root = self.calculate_merkle_root(&summaries)?;
        
        // Sign if keypair available
        let signature = if let Some(ref keypair) = self.keypair {
            Some(self.sign_proof(&merkle_root, keypair)?)
        } else {
            None
        };
        
        let proof = VerificationProof {
            id,
            verifications: summaries,
            merkle_root,
            signature,
            timestamp,
            prover: metadata.agent_context.clone(),
            metadata,
        };
        
        self.proofs.insert(id, proof.clone());
        
        Ok(proof)
    }
    
    /// Creates a new proof chain
    pub async fn create_chain(
        &mut self,
        purpose: String,
        genesis_verifications: Vec<VerificationResult>,
    ) -> Result<ProofChain> {
        let chain_id = Uuid::new_v4();
        let now = Utc::now();
        
        // Create genesis proof
        let genesis = self.generate_proof_with_metadata(
            genesis_verifications,
            ProofMetadata {
                intent_id: None,
                agent_context: None,
                chain_height: 0,
                previous_proof: None,
                tags: vec!["genesis".to_string()],
            }
        ).await?;
        
        let chain = ProofChain {
            id: chain_id,
            genesis: genesis.clone(),
            proofs: vec![genesis.clone()],
            head: genesis.id,
            metadata: ChainMetadata {
                created_at: now,
                updated_at: now,
                total_verifications: genesis.verifications.len(),
                purpose,
            },
        };
        
        self.chains.insert(chain_id, chain.clone());
        
        Ok(chain)
    }
    
    /// Adds a proof to a chain
    pub async fn add_to_chain(
        &mut self,
        chain_id: Uuid,
        verifications: Vec<VerificationResult>,
    ) -> Result<VerificationProof> {
        let chain = self.chains.get_mut(&chain_id)
            .ok_or_else(|| VerifyError::ProofError("Chain not found".to_string()))?;
        
        let previous_proof = chain.head;
        let chain_height = chain.proofs.len() as u64;
        
        let proof = self.generate_proof_with_metadata(
            verifications,
            ProofMetadata {
                intent_id: None,
                agent_context: None,
                chain_height,
                previous_proof: Some(previous_proof),
                tags: vec!["chain".to_string()],
            }
        ).await?;
        
        chain.proofs.push(proof.clone());
        chain.head = proof.id;
        chain.metadata.updated_at = Utc::now();
        chain.metadata.total_verifications += proof.verifications.len();
        
        Ok(proof)
    }
    
    /// Verifies a proof signature
    pub fn verify_proof(&self, proof: &VerificationProof) -> Result<bool> {
        if let Some(ref sig) = proof.signature {
            if sig.algorithm != SignatureAlgorithm::Ed25519 {
                return Err(VerifyError::ProofError(
                    "Unsupported signature algorithm".to_string()
                ));
            }
            
            let public_key = PublicKey::from_bytes(&sig.public_key)
                .map_err(|e| VerifyError::ProofError(format!("Invalid public key: {}", e)))?;
            
            let signature = Signature::from_bytes(&sig.signature)
                .map_err(|e| VerifyError::ProofError(format!("Invalid signature: {}", e)))?;
            
            let message = proof.merkle_root.as_bytes();
            
            Ok(public_key.verify(message, &signature).is_ok())
        } else {
            // No signature to verify
            Ok(true)
        }
    }
    
    /// Verifies an entire proof chain
    pub fn verify_chain(&self, chain: &ProofChain) -> Result<bool> {
        // Verify genesis
        if !self.verify_proof(&chain.genesis)? {
            return Ok(false);
        }
        
        // Verify each proof in sequence
        let mut previous_id = chain.genesis.id;
        
        for (i, proof) in chain.proofs.iter().enumerate() {
            if i == 0 {
                continue; // Skip genesis, already verified
            }
            
            // Check chain linkage
            if proof.metadata.previous_proof != Some(previous_id) {
                return Ok(false);
            }
            
            // Check height
            if proof.metadata.chain_height != i as u64 {
                return Ok(false);
            }
            
            // Verify proof signature
            if !self.verify_proof(proof)? {
                return Ok(false);
            }
            
            previous_id = proof.id;
        }
        
        Ok(true)
    }
    
    // Helper methods
    
    fn hash_verification(verification: &VerificationResult) -> String {
        let mut hasher = blake3::Hasher::new();
        let json = serde_json::to_string(verification).unwrap_or_default();
        hasher.update(json.as_bytes());
        hasher.finalize().to_hex().to_string()
    }
    
    fn calculate_merkle_root(&self, summaries: &[VerificationSummary]) -> Result<String> {
        if summaries.is_empty() {
            return Ok(String::from("0000000000000000000000000000000000000000000000000000000000000000"));
        }
        
        // Get leaf hashes
        let mut hashes: Vec<Vec<u8>> = summaries
            .iter()
            .map(|s| hex::decode(&s.hash).unwrap_or_default())
            .collect();
        
        // Build Merkle tree
        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            
            for pair in hashes.chunks(2) {
                let mut hasher = Sha256::new();
                hasher.update(&pair[0]);
                if pair.len() > 1 {
                    hasher.update(&pair[1]);
                } else {
                    hasher.update(&pair[0]); // Duplicate for odd number
                }
                next_level.push(hasher.finalize().to_vec());
            }
            
            hashes = next_level;
        }
        
        Ok(hex::encode(&hashes[0]))
    }
    
    fn sign_proof(&self, merkle_root: &str, keypair: &Keypair) -> Result<ProofSignature> {
        let signature = keypair.sign(merkle_root.as_bytes());
        
        Ok(ProofSignature {
            public_key: keypair.public.to_bytes().to_vec(),
            signature: signature.to_bytes().to_vec(),
            algorithm: SignatureAlgorithm::Ed25519,
        })
    }
    
    /// Gets a proof by ID
    pub fn get_proof(&self, id: Uuid) -> Option<&VerificationProof> {
        self.proofs.get(&id)
    }
    
    /// Gets a chain by ID
    pub fn get_chain(&self, id: Uuid) -> Option<&ProofChain> {
        self.chains.get(&id)
    }
    
    /// Exports a proof to JSON
    pub fn export_proof(&self, proof: &VerificationProof) -> Result<String> {
        serde_json::to_string_pretty(proof)
            .map_err(|e| VerifyError::ProofError(format!("Failed to export: {}", e)))
    }
    
    /// Imports a proof from JSON
    pub fn import_proof(&mut self, json: &str) -> Result<VerificationProof> {
        let proof: VerificationProof = serde_json::from_str(json)
            .map_err(|e| VerifyError::ProofError(format!("Failed to import: {}", e)))?;
        
        self.proofs.insert(proof.id, proof.clone());
        
        Ok(proof)
    }
}

impl Default for ProofGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_proof_generation() {
        let mut generator = ProofGenerator::new();
        
        let verifications = vec![
            VerificationResult::success(
                VerificationType::Command,
                serde_json::json!({"command": "test"}),
                serde_json::json!({"result": "success"}),
            ),
        ];
        
        let proof = generator.generate_proof(verifications).await.unwrap();
        
        assert_eq!(proof.verifications.len(), 1);
        assert!(!proof.merkle_root.is_empty());
    }
    
    #[tokio::test]
    async fn test_proof_signing() {
        let mut generator = ProofGenerator::with_signing();
        
        let verifications = vec![
            VerificationResult::success(
                VerificationType::Command,
                serde_json::json!({}),
                serde_json::json!({}),
            ),
        ];
        
        let proof = generator.generate_proof(verifications).await.unwrap();
        
        assert!(proof.signature.is_some());
        assert!(generator.verify_proof(&proof).unwrap());
    }
    
    #[tokio::test]
    async fn test_proof_chain() {
        let mut generator = ProofGenerator::new();
        
        let initial_verifications = vec![
            VerificationResult::success(
                VerificationType::State,
                serde_json::json!({}),
                serde_json::json!({}),
            ),
        ];
        
        let chain = generator.create_chain(
            "test_chain".to_string(),
            initial_verifications
        ).await.unwrap();
        
        assert_eq!(chain.proofs.len(), 1);
        
        // Add another proof
        let more_verifications = vec![
            VerificationResult::success(
                VerificationType::FileSystem,
                serde_json::json!({}),
                serde_json::json!({}),
            ),
        ];
        
        let proof = generator.add_to_chain(chain.id, more_verifications).await.unwrap();
        
        let updated_chain = generator.get_chain(chain.id).unwrap();
        assert_eq!(updated_chain.proofs.len(), 2);
        assert_eq!(updated_chain.head, proof.id);
    }
}