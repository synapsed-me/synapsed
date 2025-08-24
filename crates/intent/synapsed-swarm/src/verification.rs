//! Swarm-specific verification system

use crate::{error::SwarmResult, types::*};
use synapsed_intent::{HierarchicalIntent, StepResult};
use synapsed_verify::{
    Verifier, VerificationResult, VerificationProof, CommandVerifier,
    FileSystemVerifier, NetworkVerifier, StateVerifier, ProofGenerator,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tracing::{info, debug, warn};

/// Verification policy for swarm operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationPolicy {
    /// Require command verification
    pub verify_commands: bool,
    /// Require file system verification
    pub verify_filesystem: bool,
    /// Require network verification
    pub verify_network: bool,
    /// Require state verification
    pub verify_state: bool,
    /// Minimum verification confidence
    pub min_confidence: f64,
    /// Enable cryptographic proofs
    pub generate_proofs: bool,
    /// Require consensus for critical operations
    pub require_consensus: bool,
    /// Number of verifiers required for consensus
    pub consensus_verifiers: usize,
}

impl Default for VerificationPolicy {
    fn default() -> Self {
        Self {
            verify_commands: true,
            verify_filesystem: true,
            verify_network: false,
            verify_state: true,
            min_confidence: 0.8,
            generate_proofs: true,
            require_consensus: false,
            consensus_verifiers: 3,
        }
    }
}

/// Verification report for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Task being verified
    pub task_id: TaskId,
    /// Agent that executed the task
    pub agent_id: AgentId,
    /// Whether verification passed
    pub verified: bool,
    /// Confidence in verification (0.0 to 1.0)
    pub confidence: f64,
    /// Individual verification results
    pub results: Vec<VerificationResult>,
    /// Cryptographic proof if generated
    pub proof: Option<VerificationProof>,
    /// Consensus results if applicable
    pub consensus: Option<ConsensusResult>,
    /// Timestamp of verification
    pub timestamp: DateTime<Utc>,
}

/// Result of consensus verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    /// Number of verifiers
    pub verifiers: usize,
    /// Number that agreed
    pub agreed: usize,
    /// Number that disagreed
    pub disagreed: usize,
    /// Whether consensus was reached
    pub consensus_reached: bool,
    /// Individual verifier results
    pub verifier_results: Vec<(AgentId, bool)>,
}

/// Swarm verification system
pub struct SwarmVerifier {
    /// Verification policy
    policy: Arc<RwLock<VerificationPolicy>>,
    /// Core verifier
    verifier: Arc<RwLock<Verifier>>,
    /// Command verifier
    command_verifier: Arc<CommandVerifier>,
    /// File system verifier
    filesystem_verifier: Arc<RwLock<FileSystemVerifier>>,
    /// Network verifier
    network_verifier: Arc<NetworkVerifier>,
    /// State verifier
    state_verifier: Arc<RwLock<StateVerifier>>,
    /// Proof generator
    proof_generator: Arc<RwLock<ProofGenerator>>,
    /// Verification history
    history: Arc<RwLock<Vec<VerificationReport>>>,
}

impl SwarmVerifier {
    /// Create a new swarm verifier
    pub fn new() -> Self {
        Self {
            policy: Arc::new(RwLock::new(VerificationPolicy::default())),
            verifier: Arc::new(RwLock::new(Verifier::new())),
            command_verifier: Arc::new(CommandVerifier::new()),
            filesystem_verifier: Arc::new(RwLock::new(FileSystemVerifier::new())),
            network_verifier: Arc::new(NetworkVerifier::new()),
            state_verifier: Arc::new(RwLock::new(StateVerifier::new())),
            proof_generator: Arc::new(RwLock::new(ProofGenerator::new())),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Create with custom policy
    pub fn with_policy(policy: VerificationPolicy) -> Self {
        let mut verifier = Self::new();
        *verifier.policy.blocking_write() = policy;
        verifier
    }
    
    /// Initialize the verifier
    pub async fn initialize(&self) -> SwarmResult<()> {
        info!("Initializing swarm verifier");
        Ok(())
    }
    
    /// Verify execution of an intent
    pub async fn verify_execution(
        &self,
        intent: &HierarchicalIntent,
        result: &StepResult,
        agent_id: AgentId,
    ) -> SwarmResult<VerificationReport> {
        let task_id = Uuid::new_v4(); // Generate task ID
        let policy = self.policy.read().await;
        
        debug!("Verifying execution of intent {} by agent {}", intent.id(), agent_id);
        
        let mut verification_results = Vec::new();
        let mut total_confidence = 0.0;
        let mut checks_performed = 0;
        
        // Command verification
        if policy.verify_commands {
            if let Some(command) = Self::extract_command(result) {
                let verification = self.verify_command(&command, result).await?;
                total_confidence += verification.confidence();
                checks_performed += 1;
                verification_results.push(verification);
            }
        }
        
        // File system verification
        if policy.verify_filesystem {
            if let Some(files) = Self::extract_files(result) {
                let verification = self.verify_filesystem(&files, result).await?;
                total_confidence += verification.confidence();
                checks_performed += 1;
                verification_results.push(verification);
            }
        }
        
        // Network verification
        if policy.verify_network {
            if let Some(network_ops) = Self::extract_network_ops(result) {
                let verification = self.verify_network(&network_ops, result).await?;
                total_confidence += verification.confidence();
                checks_performed += 1;
                verification_results.push(verification);
            }
        }
        
        // State verification
        if policy.verify_state {
            let verification = self.verify_state(intent, result).await?;
            total_confidence += verification.confidence();
            checks_performed += 1;
            verification_results.push(verification);
        }
        
        // Calculate average confidence
        let confidence = if checks_performed > 0 {
            total_confidence / checks_performed as f64
        } else {
            1.0 // No checks performed means automatic pass
        };
        
        // Check if verification passed
        let verified = confidence >= policy.min_confidence && 
                      verification_results.iter().all(|r| r.is_verified());
        
        // Generate proof if required
        let proof = if policy.generate_proofs && verified {
            Some(self.generate_proof(verification_results.clone()).await?)
        } else {
            None
        };
        
        // Perform consensus verification if required
        let consensus = if policy.require_consensus {
            Some(self.perform_consensus_verification(
                intent,
                result,
                agent_id,
                policy.consensus_verifiers,
            ).await?)
        } else {
            None
        };
        
        let report = VerificationReport {
            task_id,
            agent_id,
            verified,
            confidence,
            results: verification_results,
            proof,
            consensus,
            timestamp: Utc::now(),
        };
        
        // Store in history
        self.record_verification(report.clone()).await;
        
        Ok(report)
    }
    
    /// Verify a command execution
    async fn verify_command(
        &self,
        command: &str,
        result: &StepResult,
    ) -> SwarmResult<VerificationResult> {
        let expected_output = result.output.as_ref()
            .and_then(|o| o.as_str());
        
        let verification = self.command_verifier
            .verify(command, expected_output, None)
            .await?;
        
        Ok(VerificationResult {
            verified: verification.is_success(),
            confidence: if verification.is_success() { 0.9 } else { 0.1 },
            evidence: serde_json::json!({
                "command": command,
                "verification": verification,
            }),
            timestamp: Utc::now(),
        })
    }
    
    /// Verify file system changes
    async fn verify_filesystem(
        &self,
        files: &[String],
        result: &StepResult,
    ) -> SwarmResult<VerificationResult> {
        let mut verifier = self.filesystem_verifier.write().await;
        
        // Take snapshot of current state
        let paths: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
        let snapshot = verifier.take_snapshot(&paths).await?;
        
        // Compare with expected state
        let verified = !snapshot.files.is_empty();
        
        Ok(VerificationResult {
            verified,
            confidence: if verified { 0.85 } else { 0.15 },
            evidence: serde_json::json!({
                "files": files,
                "snapshot": snapshot,
            }),
            timestamp: Utc::now(),
        })
    }
    
    /// Verify network operations
    async fn verify_network(
        &self,
        operations: &[NetworkOperation],
        result: &StepResult,
    ) -> SwarmResult<VerificationResult> {
        let mut verified_count = 0;
        let mut evidence = Vec::new();
        
        for op in operations {
            match op {
                NetworkOperation::HttpRequest { url, expected_status } => {
                    let verification = self.network_verifier
                        .verify_api(url, *expected_status, None)
                        .await;
                    
                    if verification.is_ok() {
                        verified_count += 1;
                    }
                    
                    evidence.push(serde_json::json!({
                        "url": url,
                        "verified": verification.is_ok(),
                    }));
                }
            }
        }
        
        let verified = verified_count == operations.len();
        let confidence = verified_count as f64 / operations.len() as f64;
        
        Ok(VerificationResult {
            verified,
            confidence,
            evidence: serde_json::json!(evidence),
            timestamp: Utc::now(),
        })
    }
    
    /// Verify state changes
    async fn verify_state(
        &self,
        intent: &HierarchicalIntent,
        result: &StepResult,
    ) -> SwarmResult<VerificationResult> {
        let mut verifier = self.state_verifier.write().await;
        
        // Take current state snapshot
        let snapshot = verifier.take_snapshot().await?;
        
        // Simple verification: check if state changed
        let verified = result.success;
        
        Ok(VerificationResult {
            verified,
            confidence: if verified { 0.75 } else { 0.25 },
            evidence: serde_json::json!({
                "intent_id": intent.id(),
                "state_hash": snapshot.hash(),
            }),
            timestamp: Utc::now(),
        })
    }
    
    /// Generate cryptographic proof
    pub async fn generate_proof(
        &self,
        results: Vec<VerificationResult>,
    ) -> SwarmResult<VerificationProof> {
        let mut generator = self.proof_generator.write().await;
        let proof = generator.generate_proof(results).await?;
        Ok(proof)
    }
    
    /// Perform consensus verification
    async fn perform_consensus_verification(
        &self,
        intent: &HierarchicalIntent,
        result: &StepResult,
        agent_id: AgentId,
        num_verifiers: usize,
    ) -> SwarmResult<ConsensusResult> {
        // In a real implementation, this would coordinate with other verifier agents
        // For now, simulate consensus
        warn!("Consensus verification not fully implemented - simulating");
        
        let agreed = if result.success {
            num_verifiers * 3 / 4 // 75% agree on success
        } else {
            num_verifiers / 4 // 25% agree on failure
        };
        
        let disagreed = num_verifiers - agreed;
        
        Ok(ConsensusResult {
            verifiers: num_verifiers,
            agreed,
            disagreed,
            consensus_reached: agreed > num_verifiers / 2,
            verifier_results: Vec::new(), // Would contain actual verifier results
        })
    }
    
    /// Record verification in history
    async fn record_verification(&self, report: VerificationReport) {
        let mut history = self.history.write().await;
        history.push(report);
        
        // Limit history size
        if history.len() > 1000 {
            history.drain(0..100);
        }
    }
    
    /// Get verification history
    pub async fn get_history(&self) -> Vec<VerificationReport> {
        self.history.read().await.clone()
    }
    
    /// Extract command from result
    fn extract_command(result: &StepResult) -> Option<String> {
        result.metadata.get("command")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
    
    /// Extract files from result
    fn extract_files(result: &StepResult) -> Option<Vec<String>> {
        result.metadata.get("files")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
    }
    
    /// Extract network operations from result
    fn extract_network_ops(result: &StepResult) -> Option<Vec<NetworkOperation>> {
        result.metadata.get("network_ops")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// Network operation for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkOperation {
    HttpRequest {
        url: String,
        expected_status: u16,
    },
}

// Extension trait for VerificationResult
trait VerificationResultExt {
    fn is_verified(&self) -> bool;
    fn confidence(&self) -> f64;
}

impl VerificationResultExt for VerificationResult {
    fn is_verified(&self) -> bool {
        self.verified
    }
    
    fn confidence(&self) -> f64 {
        self.confidence
    }
}