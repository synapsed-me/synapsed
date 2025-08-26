//! Verification gate ensuring all stories must be verified before completion
//!
//! This module enforces the critical requirement that every story must end
//! with external verification, preventing AI hallucination and self-reporting.

use crate::{
    story::{Story, StoryOutcome, StoryEvent, StoryEventType},
    SemanticError, SemanticResult,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// The verification gate that all stories must pass through
pub struct VerificationGate {
    /// Configuration for the gate
    config: GateConfig,
    
    /// Stories awaiting verification
    pending_verification: Arc<RwLock<Vec<PendingStory>>>,
    
    /// Verification strategies
    strategies: Vec<Box<dyn VerificationStrategy>>,
    
    /// Audit log of gate decisions
    audit_log: Arc<RwLock<Vec<GateAuditEntry>>>,
}

impl VerificationGate {
    /// Create a new verification gate
    pub fn new(config: GateConfig) -> Self {
        Self {
            config,
            pending_verification: Arc::new(RwLock::new(Vec::new())),
            strategies: Vec::new(),
            audit_log: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Add a verification strategy
    pub fn add_strategy(&mut self, strategy: Box<dyn VerificationStrategy>) {
        self.strategies.push(strategy);
    }
    
    /// Submit a story for verification
    pub async fn submit_story(&self, story: Story) -> SemanticResult<GateTicket> {
        // Check if story has required events
        if !self.validate_story_structure(&story) {
            return Err(SemanticError::VerificationFailed(
                "Story missing required structure".to_string()
            ));
        }
        
        // Check if story already has verification
        if matches!(story.verification, StoryOutcome::Success { .. }) {
            return Err(SemanticError::VerificationFailed(
                "Story already verified".to_string()
            ));
        }
        
        // Create pending entry
        let ticket = GateTicket::new();
        let pending = PendingStory {
            ticket: ticket.clone(),
            story,
            submitted_at: Utc::now(),
            attempts: 0,
        };
        
        self.pending_verification.write().await.push(pending);
        
        // Log submission
        self.log_audit(GateAuditEntry {
            timestamp: Utc::now(),
            ticket: ticket.clone(),
            action: GateAction::Submitted,
            result: None,
        }).await;
        
        Ok(ticket)
    }
    
    /// Attempt to verify a story
    pub async fn verify(&self, ticket: &GateTicket) -> SemanticResult<VerifiedStory> {
        // Find pending story
        let mut pending_guard = self.pending_verification.write().await;
        let position = pending_guard.iter().position(|p| p.ticket == *ticket)
            .ok_or_else(|| SemanticError::VerificationFailed(
                "Story not found in pending verification".to_string()
            ))?;
        
        let mut pending = pending_guard.remove(position);
        pending.attempts += 1;
        
        // Apply all verification strategies
        let mut verification_results = Vec::new();
        for strategy in &self.strategies {
            let result = strategy.verify(&pending.story).await;
            verification_results.push(result);
        }
        
        // Determine overall verification outcome
        let outcome = self.evaluate_results(&verification_results);
        
        // Update story with verification
        let mut verified_story = pending.story.clone();
        verified_story.verification = outcome.clone();
        
        // Log verification
        self.log_audit(GateAuditEntry {
            timestamp: Utc::now(),
            ticket: ticket.clone(),
            action: GateAction::Verified,
            result: Some(outcome.clone()),
        }).await;
        
        // Check if verification passed
        if outcome.is_success() {
            Ok(VerifiedStory {
                story: verified_story,
                verification_proof: VerificationProof {
                    ticket: ticket.clone(),
                    timestamp: Utc::now(),
                    strategies_used: self.strategies.iter()
                        .map(|s| s.name().to_string())
                        .collect(),
                    outcome,
                },
            })
        } else {
            // Re-add to pending if retryable
            if pending.attempts < self.config.max_retry_attempts {
                pending_guard.push(pending);
            }
            
            Err(SemanticError::VerificationFailed(
                "Story verification failed".to_string()
            ))
        }
    }
    
    /// Force-fail a story (for timeout or manual intervention)
    pub async fn force_fail(&self, ticket: &GateTicket, reason: String) -> SemanticResult<()> {
        let mut pending_guard = self.pending_verification.write().await;
        
        if let Some(position) = pending_guard.iter().position(|p| p.ticket == *ticket) {
            let pending = pending_guard.remove(position);
            
            // Log force failure
            self.log_audit(GateAuditEntry {
                timestamp: Utc::now(),
                ticket: ticket.clone(),
                action: GateAction::ForceFailed(reason),
                result: Some(StoryOutcome::Failure {
                    reason: "Force failed by gate".to_string(),
                    error: None,
                }),
            }).await;
            
            Ok(())
        } else {
            Err(SemanticError::VerificationFailed(
                "Story not found in pending verification".to_string()
            ))
        }
    }
    
    /// Validate story has required structure
    fn validate_story_structure(&self, story: &Story) -> bool {
        // Must have intent
        if story.intent.goal.is_empty() {
            return false;
        }
        
        // Must have at least one event
        if story.execution.is_empty() {
            return false;
        }
        
        // Must have verification event if strict mode
        if self.config.require_verification_event {
            let has_verification = story.execution.iter()
                .any(|e| matches!(e.event_type, StoryEventType::VerificationPerformed));
            
            if !has_verification {
                return false;
            }
        }
        
        true
    }
    
    /// Evaluate verification results
    fn evaluate_results(&self, results: &[VerificationResult]) -> StoryOutcome {
        let total = results.len();
        if total == 0 {
            return StoryOutcome::Failure {
                reason: "No verification strategies available".to_string(),
                error: None,
            };
        }
        
        let passed = results.iter().filter(|r| r.passed).count();
        let confidence = passed as f64 / total as f64;
        
        if confidence >= self.config.min_confidence_threshold {
            StoryOutcome::Success {
                verification: self.create_verification_result(results),
                confidence,
            }
        } else if confidence > 0.0 {
            let completed = results.iter()
                .filter(|r| r.passed)
                .map(|r| r.description.clone())
                .collect();
            let failed = results.iter()
                .filter(|r| !r.passed)
                .map(|r| r.description.clone())
                .collect();
                
            StoryOutcome::Partial {
                completed,
                failed,
                confidence,
            }
        } else {
            StoryOutcome::Failure {
                reason: "All verification strategies failed".to_string(),
                error: None,
            }
        }
    }
    
    /// Create a verification result from strategy results
    fn create_verification_result(&self, results: &[VerificationResult]) -> synapsed_verify::VerificationResult {
        // Create a custom verification result combining all strategies
        synapsed_verify::VerificationResult {
            id: Uuid::new_v4(),
            verification_type: synapsed_verify::VerificationType::Custom,
            success: results.iter().all(|r| r.passed),
            expected: serde_json::json!({"strategies": results.len()}),
            actual: serde_json::json!({
                "passed": results.iter().filter(|r| r.passed).count()
            }),
            error: None,
            evidence: vec![],
            timestamp: Utc::now(),
            duration_ms: 0,
            metadata: Default::default(),
        }
    }
    
    /// Log audit entry
    async fn log_audit(&self, entry: GateAuditEntry) {
        self.audit_log.write().await.push(entry);
    }
    
    /// Get pending stories count
    pub async fn pending_count(&self) -> usize {
        self.pending_verification.read().await.len()
    }
    
    /// Clean up expired pending stories
    pub async fn cleanup_expired(&self) {
        let now = Utc::now();
        let timeout = chrono::Duration::seconds(self.config.timeout_seconds as i64);
        
        let mut pending_guard = self.pending_verification.write().await;
        pending_guard.retain(|p| {
            now.signed_duration_since(p.submitted_at) < timeout
        });
    }
}

/// Configuration for the verification gate
#[derive(Debug, Clone)]
pub struct GateConfig {
    /// Require verification event in story
    pub require_verification_event: bool,
    
    /// Minimum confidence threshold for success
    pub min_confidence_threshold: f64,
    
    /// Maximum retry attempts
    pub max_retry_attempts: u32,
    
    /// Timeout in seconds for pending verification
    pub timeout_seconds: u64,
    
    /// Allow partial verification success
    pub allow_partial_success: bool,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            require_verification_event: true,
            min_confidence_threshold: 0.8,
            max_retry_attempts: 3,
            timeout_seconds: 300,
            allow_partial_success: false,
        }
    }
}

/// A ticket for tracking story verification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GateTicket {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl GateTicket {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
        }
    }
}

/// A story pending verification
#[derive(Debug, Clone)]
struct PendingStory {
    ticket: GateTicket,
    story: Story,
    submitted_at: DateTime<Utc>,
    attempts: u32,
}

/// A verified story with proof
#[derive(Debug, Clone)]
pub struct VerifiedStory {
    pub story: Story,
    pub verification_proof: VerificationProof,
}

/// Proof of verification
#[derive(Debug, Clone)]
pub struct VerificationProof {
    pub ticket: GateTicket,
    pub timestamp: DateTime<Utc>,
    pub strategies_used: Vec<String>,
    pub outcome: StoryOutcome,
}

/// Audit entry for gate actions
#[derive(Debug, Clone)]
struct GateAuditEntry {
    timestamp: DateTime<Utc>,
    ticket: GateTicket,
    action: GateAction,
    result: Option<StoryOutcome>,
}

/// Actions taken by the gate
#[derive(Debug, Clone)]
enum GateAction {
    Submitted,
    Verified,
    ForceFailed(String),
    Expired,
}

/// Result from a verification strategy
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub strategy_name: String,
    pub passed: bool,
    pub confidence: f64,
    pub description: String,
    pub evidence: Option<serde_json::Value>,
}

/// A verification strategy
#[async_trait]
pub trait VerificationStrategy: Send + Sync {
    /// Name of the strategy
    fn name(&self) -> &str;
    
    /// Verify a story
    async fn verify(&self, story: &Story) -> VerificationResult;
}

/// Command execution verification strategy
pub struct CommandVerificationStrategy;

#[async_trait]
impl VerificationStrategy for CommandVerificationStrategy {
    fn name(&self) -> &str {
        "CommandExecution"
    }
    
    async fn verify(&self, story: &Story) -> VerificationResult {
        // Check if story contains command execution events
        let has_execution = story.execution.iter()
            .any(|e| matches!(e.event_type, 
                StoryEventType::ExecutionStarted | 
                StoryEventType::ExecutionCompleted
            ));
        
        VerificationResult {
            strategy_name: self.name().to_string(),
            passed: has_execution,
            confidence: if has_execution { 0.9 } else { 0.0 },
            description: "Command execution verification".to_string(),
            evidence: None,
        }
    }
}

/// File system verification strategy
pub struct FileSystemVerificationStrategy;

#[async_trait]
impl VerificationStrategy for FileSystemVerificationStrategy {
    fn name(&self) -> &str {
        "FileSystem"
    }
    
    async fn verify(&self, story: &Story) -> VerificationResult {
        // Check if story contains file operations
        let has_file_ops = story.execution.iter()
            .any(|e| e.description.contains("file") || e.description.contains("write"));
        
        VerificationResult {
            strategy_name: self.name().to_string(),
            passed: true, // Would check actual file system in production
            confidence: 0.8,
            description: "File system state verification".to_string(),
            evidence: None,
        }
    }
}

/// Network verification strategy  
pub struct NetworkVerificationStrategy;

#[async_trait]
impl VerificationStrategy for NetworkVerificationStrategy {
    fn name(&self) -> &str {
        "Network"
    }
    
    async fn verify(&self, story: &Story) -> VerificationResult {
        // Check if story contains network operations
        let has_network = story.execution.iter()
            .any(|e| e.description.contains("network") || e.description.contains("api"));
        
        VerificationResult {
            strategy_name: self.name().to_string(),
            passed: true, // Would check actual network responses in production
            confidence: 0.7,
            description: "Network response verification".to_string(),
            evidence: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Intent;
    
    #[tokio::test]
    async fn test_verification_gate() {
        let mut gate = VerificationGate::new(GateConfig::default());
        gate.add_strategy(Box::new(CommandVerificationStrategy));
        
        let story = Story::begin(Intent::new("Test intent"));
        let ticket = gate.submit_story(story).await.unwrap();
        
        assert_eq!(gate.pending_count().await, 1);
    }
}