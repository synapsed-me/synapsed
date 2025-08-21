//! Core types for verification

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Result of a verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Unique ID for this verification
    pub id: Uuid,
    /// Type of verification performed
    pub verification_type: VerificationType,
    /// Whether verification passed
    pub success: bool,
    /// Expected value/state
    pub expected: serde_json::Value,
    /// Actual value/state found
    pub actual: serde_json::Value,
    /// Error message if failed
    pub error: Option<String>,
    /// Evidence collected
    pub evidence: Vec<Evidence>,
    /// Timestamp of verification
    pub timestamp: DateTime<Utc>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl VerificationResult {
    /// Creates a successful verification result
    pub fn success(
        verification_type: VerificationType,
        expected: serde_json::Value,
        actual: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            verification_type,
            success: true,
            expected,
            actual,
            error: None,
            evidence: Vec::new(),
            timestamp: Utc::now(),
            duration_ms: 0,
            metadata: HashMap::new(),
        }
    }
    
    /// Creates a failed verification result
    pub fn failure(
        verification_type: VerificationType,
        expected: serde_json::Value,
        actual: serde_json::Value,
        error: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            verification_type,
            success: false,
            expected,
            actual,
            error: Some(error),
            evidence: Vec::new(),
            timestamp: Utc::now(),
            duration_ms: 0,
            metadata: HashMap::new(),
        }
    }
}

/// Type of verification performed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationType {
    /// Command execution verification
    Command,
    /// File system verification
    FileSystem,
    /// Network/API verification
    Network,
    /// State verification
    State,
    /// Cryptographic verification
    Cryptographic,
    /// Custom verification
    Custom,
}

/// Evidence collected during verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Type of evidence
    pub evidence_type: EvidenceType,
    /// Evidence data
    pub data: serde_json::Value,
    /// Source of evidence
    pub source: String,
    /// Timestamp when collected
    pub timestamp: DateTime<Utc>,
}

/// Type of evidence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceType {
    /// Command output
    CommandOutput,
    /// File content
    FileContent,
    /// File metadata
    FileMetadata,
    /// Network response
    NetworkResponse,
    /// State snapshot
    StateSnapshot,
    /// Log entry
    LogEntry,
    /// Screenshot
    Screenshot,
    /// Cryptographic hash
    Hash,
}

/// Verification context for Claude sub-agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationContext {
    /// Agent ID making claims
    pub agent_id: String,
    /// Parent agent ID (if sub-agent)
    pub parent_agent_id: Option<String>,
    /// Claimed action
    pub claimed_action: String,
    /// Context injected into agent
    pub injected_context: HashMap<String, serde_json::Value>,
    /// Verification requirements
    pub requirements: Vec<VerificationRequirement>,
    /// Timeout for verification
    pub timeout_ms: u64,
}

/// A verification requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRequirement {
    /// Name of the requirement
    pub name: String,
    /// Description
    pub description: String,
    /// Type of verification needed
    pub verification_type: VerificationType,
    /// Expected result
    pub expected: serde_json::Value,
    /// Whether this is critical (must pass)
    pub critical: bool,
}

/// Verification checkpoint for rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCheckpoint {
    /// Checkpoint ID
    pub id: Uuid,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// State at this checkpoint
    pub state: serde_json::Value,
    /// Verifications performed up to this point
    pub verifications: Vec<VerificationResult>,
    /// Whether all verifications passed
    pub all_passed: bool,
}

/// Verification report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Report ID
    pub id: Uuid,
    /// Context of verification
    pub context: VerificationContext,
    /// All verification results
    pub results: Vec<VerificationResult>,
    /// Overall success
    pub success: bool,
    /// Number of passed verifications
    pub passed: usize,
    /// Number of failed verifications
    pub failed: usize,
    /// Total duration
    pub total_duration_ms: u64,
    /// Checkpoints
    pub checkpoints: Vec<VerificationCheckpoint>,
    /// Generated at
    pub generated_at: DateTime<Utc>,
}

impl VerificationReport {
    /// Creates a new verification report
    pub fn new(context: VerificationContext, results: Vec<VerificationResult>) -> Self {
        let passed = results.iter().filter(|r| r.success).count();
        let failed = results.iter().filter(|r| !r.success).count();
        let success = failed == 0;
        let total_duration_ms = results.iter().map(|r| r.duration_ms).sum();
        
        Self {
            id: Uuid::new_v4(),
            context,
            results,
            success,
            passed,
            failed,
            total_duration_ms,
            checkpoints: Vec::new(),
            generated_at: Utc::now(),
        }
    }
    
    /// Adds a checkpoint to the report
    pub fn add_checkpoint(&mut self, checkpoint: VerificationCheckpoint) {
        self.checkpoints.push(checkpoint);
    }
    
    /// Gets a summary of the report
    pub fn summary(&self) -> String {
        format!(
            "Verification Report: {} ({}/{} passed) - Duration: {}ms",
            if self.success { "SUCCESS" } else { "FAILED" },
            self.passed,
            self.passed + self.failed,
            self.total_duration_ms
        )
    }
}