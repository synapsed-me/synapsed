//! Verification strategies for complex verification scenarios

use crate::{
    types::*, command::CommandVerifier, filesystem::FileSystemVerifier,
    network::NetworkVerifier, state::StateVerifier, Result, VerifyError
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

/// Trait for custom verifiers
#[async_trait]
pub trait CustomVerifier: Send + Sync {
    /// Performs verification
    async fn verify(&self, input: serde_json::Value) -> Result<VerificationResult>;
    
    /// Gets verifier name
    fn name(&self) -> &str;
    
    /// Gets verifier type
    fn verifier_type(&self) -> VerificationType {
        VerificationType::Custom
    }
}

/// Verification strategy
pub enum VerificationStrategy {
    /// Single verifier
    Single(Box<dyn CustomVerifier>),
    
    /// All verifiers must pass
    All(Vec<Box<dyn CustomVerifier>>),
    
    /// At least one verifier must pass
    Any(Vec<Box<dyn CustomVerifier>>),
    
    /// Consensus - majority must pass
    Consensus {
        verifiers: Vec<Box<dyn CustomVerifier>>,
        threshold: f64, // Percentage required (0.0 to 1.0)
    },
    
    /// Sequential - each depends on previous
    Sequential(Vec<Box<dyn CustomVerifier>>),
    
    /// Parallel - run all simultaneously
    Parallel(Vec<Box<dyn CustomVerifier>>),
}

/// Builder for verification strategies
pub struct StrategyBuilder {
    verifiers: Vec<Box<dyn CustomVerifier>>,
}

impl StrategyBuilder {
    /// Creates a new strategy builder
    pub fn new() -> Self {
        Self {
            verifiers: Vec::new(),
        }
    }
    
    /// Adds a custom verifier
    pub fn add_verifier(mut self, verifier: Box<dyn CustomVerifier>) -> Self {
        self.verifiers.push(verifier);
        self
    }
    
    /// Builds a strategy where all must pass
    pub fn all(self) -> VerificationStrategy {
        VerificationStrategy::All(self.verifiers)
    }
    
    /// Builds a strategy where any can pass
    pub fn any(self) -> VerificationStrategy {
        VerificationStrategy::Any(self.verifiers)
    }
    
    /// Builds a consensus strategy
    pub fn consensus(self, threshold: f64) -> VerificationStrategy {
        VerificationStrategy::Consensus {
            verifiers: self.verifiers,
            threshold,
        }
    }
    
    /// Builds a sequential strategy
    pub fn sequential(self) -> VerificationStrategy {
        VerificationStrategy::Sequential(self.verifiers)
    }
    
    /// Builds a parallel strategy
    pub fn parallel(self) -> VerificationStrategy {
        VerificationStrategy::Parallel(self.verifiers)
    }
}

/// Consensus verifier for multi-party verification
pub struct ConsensusVerifier {
    /// Participating verifiers
    verifiers: Vec<Arc<dyn CustomVerifier>>,
    /// Required consensus percentage
    threshold: f64,
    /// Results from each verifier
    results: Arc<RwLock<Vec<VerificationResult>>>,
}

impl ConsensusVerifier {
    /// Creates a new consensus verifier
    pub fn new(verifiers: Vec<Arc<dyn CustomVerifier>>, threshold: f64) -> Self {
        Self {
            verifiers,
            threshold,
            results: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Performs consensus verification
    pub async fn verify(&self, input: serde_json::Value) -> Result<VerificationResult> {
        let start = Utc::now();
        let mut all_results = Vec::new();
        
        // Run all verifiers in parallel
        let mut tasks = Vec::new();
        for verifier in &self.verifiers {
            let verifier = Arc::clone(verifier);
            let input = input.clone();
            
            tasks.push(tokio::spawn(async move {
                verifier.verify(input).await
            }));
        }
        
        // Collect results
        for task in tasks {
            match task.await {
                Ok(Ok(result)) => all_results.push(result),
                Ok(Err(e)) => {
                    // Create failed result for this verifier
                    all_results.push(VerificationResult::failure(
                        VerificationType::Custom,
                        input.clone(),
                        serde_json::json!({}),
                        format!("Verifier error: {}", e),
                    ));
                },
                Err(e) => {
                    // Task panic or cancellation
                    all_results.push(VerificationResult::failure(
                        VerificationType::Custom,
                        input.clone(),
                        serde_json::json!({}),
                        format!("Task error: {}", e),
                    ));
                }
            }
        }
        
        // Store results
        *self.results.write().await = all_results.clone();
        
        // Calculate consensus
        let total = all_results.len() as f64;
        let passed = all_results.iter().filter(|r| r.success).count() as f64;
        let consensus_reached = (passed / total) >= self.threshold;
        
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        let result = if consensus_reached {
            VerificationResult::success(
                VerificationType::Custom,
                serde_json::json!({
                    "consensus_threshold": self.threshold,
                    "verifiers": self.verifiers.len(),
                }),
                serde_json::json!({
                    "passed": passed as usize,
                    "failed": (total - passed) as usize,
                    "percentage": passed / total,
                }),
            )
        } else {
            VerificationResult::failure(
                VerificationType::Custom,
                serde_json::json!({
                    "consensus_threshold": self.threshold,
                    "verifiers": self.verifiers.len(),
                }),
                serde_json::json!({
                    "passed": passed as usize,
                    "failed": (total - passed) as usize,
                    "percentage": passed / total,
                }),
                format!("Consensus not reached: {:.1}% < {:.1}%", 
                    (passed / total) * 100.0,
                    self.threshold * 100.0
                ),
            )
        };
        
        let mut final_result = result;
        final_result.duration_ms = duration_ms;
        
        // Add evidence from all verifiers
        for (i, verifier_result) in all_results.iter().enumerate() {
            final_result.evidence.push(Evidence {
                evidence_type: EvidenceType::CustomThe user sent the following message:
please commit and push

Please address this message and continue with your tasks.
,
                data: serde_json::json!({
                    "verifier": i,
                    "success": verifier_result.success,
                }),
                source: format!("ConsensusVerifier[{}]", i),
                timestamp: Utc::now(),
            });
        }
        
        Ok(final_result)
    }
    
    /// Gets the results from all verifiers
    pub async fn get_results(&self) -> Vec<VerificationResult> {
        self.results.read().await.clone()
    }
}

/// Composite verifier that combines multiple verification types
pub struct CompositeVerifier {
    command: Option<CommandVerifier>,
    filesystem: Option<FileSystemVerifier>,
    network: Option<NetworkVerifier>,
    state: Option<StateVerifier>,
    custom: Vec<Box<dyn CustomVerifier>>,
}

impl CompositeVerifier {
    /// Creates a new composite verifier
    pub fn new() -> Self {
        Self {
            command: None,
            filesystem: None,
            network: None,
            state: None,
            custom: Vec::new(),
        }
    }
    
    /// Adds command verification
    pub fn with_command(mut self, verifier: CommandVerifier) -> Self {
        self.command = Some(verifier);
        self
    }
    
    /// Adds filesystem verification
    pub fn with_filesystem(mut self, verifier: FileSystemVerifier) -> Self {
        self.filesystem = Some(verifier);
        self
    }
    
    /// Adds network verification
    pub fn with_network(mut self, verifier: NetworkVerifier) -> Self {
        self.network = Some(verifier);
        self
    }
    
    /// Adds state verification
    pub fn with_state(mut self, verifier: StateVerifier) -> Self {
        self.state = Some(verifier);
        self
    }
    
    /// Adds custom verifier
    pub fn with_custom(mut self, verifier: Box<dyn CustomVerifier>) -> Self {
        self.custom.push(verifier);
        self
    }
    
    /// Performs composite verification
    pub async fn verify_all(&self, spec: VerificationSpec) -> Result<CompositeResult> {
        let mut results = Vec::new();
        let start = Utc::now();
        
        // Run command verification
        if let Some(ref verifier) = self.command {
            if let Some(ref cmd_spec) = spec.command {
                let result = verifier.verify(
                    &cmd_spec.command,
                    cmd_spec.expected_output.as_deref(),
                    cmd_spec.expected_exit_code,
                ).await?;
                results.push(result.result);
            }
        }
        
        // Run filesystem verification
        if let Some(ref verifier) = self.filesystem {
            if let Some(ref fs_spec) = spec.filesystem {
                let result = verifier.verify_files_exist(&fs_spec.files).await?;
                results.push(result);
            }
        }
        
        // Run network verification
        if let Some(ref verifier) = self.network {
            if let Some(ref net_spec) = spec.network {
                let result = verifier.verify_api(
                    &net_spec.url,
                    net_spec.expected_status,
                    net_spec.expected_body.clone(),
                ).await?;
                results.push(result.result);
            }
        }
        
        // Run custom verifiers
        for verifier in &self.custom {
            if let Some(ref custom_input) = spec.custom {
                let result = verifier.verify(custom_input.clone()).await?;
                results.push(result);
            }
        }
        
        let all_passed = results.iter().all(|r| r.success);
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        Ok(CompositeResult {
            success: all_passed,
            results,
            duration_ms,
        })
    }
}

/// Specification for composite verification
#[derive(Debug, Clone)]
pub struct VerificationSpec {
    pub command: Option<CommandSpec>,
    pub filesystem: Option<FileSystemSpec>,
    pub network: Option<NetworkSpec>,
    pub state: Option<StateSpec>,
    pub custom: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub command: String,
    pub expected_output: Option<String>,
    pub expected_exit_code: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct FileSystemSpec {
    pub files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NetworkSpec {
    pub url: String,
    pub expected_status: u16,
    pub expected_body: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct StateSpec {
    pub invariants: Vec<(String, serde_json::Value)>,
}

/// Result from composite verification
#[derive(Debug, Clone)]
pub struct CompositeResult {
    pub success: bool,
    pub results: Vec<VerificationResult>,
    pub duration_ms: u64,
}

impl Default for CompositeVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestVerifier {
        name: String,
        should_pass: bool,
    }
    
    #[async_trait]
    impl CustomVerifier for TestVerifier {
        async fn verify(&self, _input: serde_json::Value) -> Result<VerificationResult> {
            if self.should_pass {
                Ok(VerificationResult::success(
                    VerificationType::Custom,
                    serde_json::json!({}),
                    serde_json::json!({}),
                ))
            } else {
                Ok(VerificationResult::failure(
                    VerificationType::Custom,
                    serde_json::json!({}),
                    serde_json::json!({}),
                    "Test failure".to_string(),
                ))
            }
        }
        
        fn name(&self) -> &str {
            &self.name
        }
    }
    
    #[tokio::test]
    async fn test_consensus_verifier() {
        let verifiers: Vec<Arc<dyn CustomVerifier>> = vec![
            Arc::new(TestVerifier { name: "v1".to_string(), should_pass: true }),
            Arc::new(TestVerifier { name: "v2".to_string(), should_pass: true }),
            Arc::new(TestVerifier { name: "v3".to_string(), should_pass: false }),
        ];
        
        let consensus = ConsensusVerifier::new(verifiers, 0.6);
        let result = consensus.verify(serde_json::json!({})).await.unwrap();
        
        // 2 out of 3 pass, which is 66% > 60% threshold
        assert!(result.success);
    }
}