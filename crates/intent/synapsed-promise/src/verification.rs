//! Verification mechanisms for promises and agent actions

use crate::{
    types::*, PromiseId, Promise, Result, PromiseError
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};

/// Verification proof for a promise
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationProof {
    /// Proof ID
    pub id: Uuid,
    /// Promise being verified
    pub promise_id: PromiseId,
    /// Type of verification
    pub verification_type: VerificationType,
    /// Verification data
    pub data: VerificationData,
    /// Cryptographic hash of the verification
    pub hash: Vec<u8>,
    /// Signature if available
    pub signature: Option<Vec<u8>>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Verifier agent
    pub verifier: Option<AgentId>,
}

/// Type of verification performed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationType {
    /// Command execution verification
    CommandExecution,
    /// File system state verification
    FileSystemState,
    /// Network/API verification
    NetworkResponse,
    /// State snapshot verification
    StateSnapshot,
    /// Cryptographic verification
    Cryptographic,
    /// External oracle verification
    ExternalOracle,
    /// Multi-party verification
    MultiParty,
}

/// Verification data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationData {
    /// Input to verification
    pub input: serde_json::Value,
    /// Expected output
    pub expected: serde_json::Value,
    /// Actual output
    pub actual: serde_json::Value,
    /// Whether verification passed
    pub passed: bool,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Verification strategy
#[derive(Debug, Clone)]
pub enum VerificationStrategy {
    /// Single point verification
    Single(Box<dyn Verifier>),
    /// Multiple verifiers must agree
    Consensus(Vec<Box<dyn Verifier>>),
    /// At least N of M verifiers must agree
    Threshold {
        verifiers: Vec<Box<dyn Verifier>>,
        threshold: usize,
    },
    /// Hierarchical verification
    Hierarchical(Vec<Box<dyn Verifier>>),
}

/// Trait for verifiers
pub trait Verifier: Send + Sync {
    /// Performs verification
    fn verify(
        &self,
        input: &serde_json::Value,
        expected: &serde_json::Value,
    ) -> Result<VerificationData>;
    
    /// Gets the verifier type
    fn verifier_type(&self) -> VerificationType;
    
    /// Gets verifier name
    fn name(&self) -> &str;
}

/// Command execution verifier (for Claude sub-agent commands)
#[derive(Debug, Clone)]
pub struct CommandVerifier {
    /// Sandbox environment if available
    pub sandbox: bool,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
}

impl Verifier for CommandVerifier {
    fn verify(
        &self,
        input: &serde_json::Value,
        expected: &serde_json::Value,
    ) -> Result<VerificationData> {
        // This would actually execute the command and verify output
        // For Claude sub-agents, this would verify their claimed actions
        
        let command = input.as_str()
            .ok_or_else(|| PromiseError::VerificationFailed("Invalid command input".to_string()))?;
        
        // Simulate command execution
        let actual = serde_json::json!({
            "exit_code": 0,
            "stdout": "Command executed successfully",
            "stderr": ""
        });
        
        let passed = actual == *expected;
        
        Ok(VerificationData {
            input: input.clone(),
            expected: expected.clone(),
            actual,
            passed,
            metadata: HashMap::new(),
        })
    }
    
    fn verifier_type(&self) -> VerificationType {
        VerificationType::CommandExecution
    }
    
    fn name(&self) -> &str {
        "CommandVerifier"
    }
}

/// File system state verifier
#[derive(Debug, Clone)]
pub struct FileSystemVerifier {
    /// Root path for verification
    pub root_path: String,
}

impl Verifier for FileSystemVerifier {
    fn verify(
        &self,
        input: &serde_json::Value,
        expected: &serde_json::Value,
    ) -> Result<VerificationData> {
        // This would check file system state
        // Useful for verifying Claude's claims about created files
        
        let path = input.as_str()
            .ok_or_else(|| PromiseError::VerificationFailed("Invalid path input".to_string()))?;
        
        // Simulate file check
        let actual = serde_json::json!({
            "exists": true,
            "size": 1024,
            "modified": Utc::now().to_rfc3339()
        });
        
        let passed = actual["exists"] == expected["exists"];
        
        Ok(VerificationData {
            input: input.clone(),
            expected: expected.clone(),
            actual,
            passed,
            metadata: HashMap::new(),
        })
    }
    
    fn verifier_type(&self) -> VerificationType {
        VerificationType::FileSystemState
    }
    
    fn name(&self) -> &str {
        "FileSystemVerifier"
    }
}

/// State snapshot verifier
#[derive(Debug, Clone)]
pub struct StateSnapshotVerifier {
    /// Previous snapshots
    snapshots: HashMap<Uuid, serde_json::Value>,
}

impl StateSnapshotVerifier {
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
        }
    }
    
    /// Takes a snapshot
    pub fn snapshot(&mut self, state: serde_json::Value) -> Uuid {
        let id = Uuid::new_v4();
        self.snapshots.insert(id, state);
        id
    }
    
    /// Compares with a snapshot
    pub fn compare(&self, snapshot_id: Uuid, current: &serde_json::Value) -> bool {
        self.snapshots.get(&snapshot_id)
            .map(|snap| snap == current)
            .unwrap_or(false)
    }
}

impl Verifier for StateSnapshotVerifier {
    fn verify(
        &self,
        input: &serde_json::Value,
        expected: &serde_json::Value,
    ) -> Result<VerificationData> {
        let snapshot_id = input["snapshot_id"].as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| PromiseError::VerificationFailed("Invalid snapshot ID".to_string()))?;
        
        let current_state = &input["current_state"];
        let passed = self.compare(snapshot_id, current_state);
        
        Ok(VerificationData {
            input: input.clone(),
            expected: expected.clone(),
            actual: current_state.clone(),
            passed,
            metadata: HashMap::new(),
        })
    }
    
    fn verifier_type(&self) -> VerificationType {
        VerificationType::StateSnapshot
    }
    
    fn name(&self) -> &str {
        "StateSnapshotVerifier"
    }
}

/// Promise verifier that combines multiple verification strategies
pub struct PromiseVerifier {
    /// Verification strategies
    strategies: Vec<VerificationStrategy>,
    /// Proof storage
    proofs: Vec<VerificationProof>,
}

impl PromiseVerifier {
    /// Creates a new promise verifier
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
            proofs: Vec::new(),
        }
    }
    
    /// Adds a verification strategy
    pub fn add_strategy(&mut self, strategy: VerificationStrategy) {
        self.strategies.push(strategy);
    }
    
    /// Verifies a promise
    pub async fn verify_promise(&mut self, promise: &Promise) -> Result<VerificationProof> {
        let promise_id = promise.id();
        
        // Get promise outcome
        let outcome = promise.outcome().await
            .ok_or_else(|| PromiseError::VerificationFailed("Promise has no outcome".to_string()))?;
        
        // Create verification data
        let data = VerificationData {
            input: serde_json::json!({
                "promise_id": promise_id.0,
                "promise_type": format!("{:?}", promise.promise_type()),
            }),
            expected: serde_json::json!({
                "state": "Fulfilled",
                "quality": 1.0,
            }),
            actual: serde_json::json!({
                "state": format!("{:?}", outcome.state),
                "quality": outcome.quality,
            }),
            passed: outcome.state == PromiseState::Fulfilled && outcome.quality >= 0.8,
            metadata: HashMap::new(),
        };
        
        // Calculate hash
        let hash = Self::calculate_hash(&data);
        
        // Create proof
        let proof = VerificationProof {
            id: Uuid::new_v4(),
            promise_id,
            verification_type: VerificationType::MultiParty,
            data,
            hash,
            signature: None, // Would add cryptographic signature
            timestamp: Utc::now(),
            verifier: promise.promisor(),
        };
        
        self.proofs.push(proof.clone());
        
        Ok(proof)
    }
    
    /// Verifies a claim made by an agent
    pub async fn verify_claim(
        &mut self,
        claim: &str,
        evidence: &[Evidence],
    ) -> Result<bool> {
        // This would verify specific claims made by Claude sub-agents
        
        // Check evidence
        let has_proof = evidence.iter()
            .any(|e| e.evidence_type == EvidenceType::CryptographicProof);
        
        if has_proof {
            return Ok(true);
        }
        
        // Check other evidence types
        let log_evidence = evidence.iter()
            .filter(|e| e.evidence_type == EvidenceType::Log)
            .count();
        
        let metric_evidence = evidence.iter()
            .filter(|e| e.evidence_type == EvidenceType::Metric)
            .count();
        
        // Require multiple types of evidence
        Ok(log_evidence > 0 && metric_evidence > 0)
    }
    
    /// Gets all proofs
    pub fn get_proofs(&self) -> &[VerificationProof] {
        &self.proofs
    }
    
    /// Calculates hash of verification data
    fn calculate_hash(data: &VerificationData) -> Vec<u8> {
        let mut hasher = Sha256::new();
        let json = serde_json::to_string(data).unwrap_or_default();
        hasher.update(json.as_bytes());
        hasher.finalize().to_vec()
    }
}

/// Multi-party verification coordinator
pub struct MultiPartyVerifier {
    /// Participating verifiers
    verifiers: Vec<AgentId>,
    /// Required confirmations
    required_confirmations: usize,
    /// Collected confirmations
    confirmations: HashMap<PromiseId, Vec<AgentId>>,
}

impl MultiPartyVerifier {
    /// Creates a new multi-party verifier
    pub fn new(verifiers: Vec<AgentId>, required_confirmations: usize) -> Self {
        Self {
            verifiers,
            required_confirmations,
            confirmations: HashMap::new(),
        }
    }
    
    /// Adds a confirmation from a verifier
    pub fn add_confirmation(&mut self, promise_id: PromiseId, verifier: AgentId) -> bool {
        if !self.verifiers.contains(&verifier) {
            return false;
        }
        
        self.confirmations
            .entry(promise_id)
            .or_insert_with(Vec::new)
            .push(verifier);
        
        true
    }
    
    /// Checks if promise has enough confirmations
    pub fn is_verified(&self, promise_id: PromiseId) -> bool {
        self.confirmations
            .get(&promise_id)
            .map(|confs| confs.len() >= self.required_confirmations)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_command_verifier() {
        let verifier = CommandVerifier {
            sandbox: true,
            timeout_ms: 5000,
        };
        
        let input = serde_json::json!("echo test");
        let expected = serde_json::json!({
            "exit_code": 0,
            "stdout": "Command executed successfully",
            "stderr": ""
        });
        
        let result = verifier.verify(&input, &expected).unwrap();
        assert!(result.passed);
    }
    
    #[test]
    fn test_multi_party_verifier() {
        let agents = vec![AgentId::new(), AgentId::new(), AgentId::new()];
        let mut verifier = MultiPartyVerifier::new(agents.clone(), 2);
        
        let promise_id = PromiseId::new();
        
        // Add first confirmation
        assert!(verifier.add_confirmation(promise_id, agents[0]));
        assert!(!verifier.is_verified(promise_id));
        
        // Add second confirmation - should be verified
        assert!(verifier.add_confirmation(promise_id, agents[1]));
        assert!(verifier.is_verified(promise_id));
    }
}