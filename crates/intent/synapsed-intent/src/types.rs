//! Core types for hierarchical intent system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Unique identifier for an intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntentId(pub Uuid);

impl IntentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for IntentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Status of an intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentStatus {
    /// Intent has been created but not started
    Pending,
    /// Intent is currently being planned
    Planning,
    /// Intent is ready to execute
    Ready,
    /// Intent is currently executing
    Executing,
    /// Intent execution is paused
    Paused,
    /// Intent completed successfully
    Completed,
    /// Intent failed during execution
    Failed,
    /// Intent was cancelled
    Cancelled,
    /// Intent has been rolled back
    RolledBack,
}

/// Priority level for intent execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// A step in an intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// Step ID
    pub id: Uuid,
    /// Step name
    pub name: String,
    /// Step description
    pub description: Option<String>,
    /// Command or action to execute
    pub action: StepAction,
    /// Pre-conditions that must be met
    pub preconditions: Vec<Condition>,
    /// Post-conditions that should be true after
    pub postconditions: Vec<Condition>,
    /// Dependencies on other steps
    pub dependencies: Vec<Uuid>,
    /// Verification requirements
    pub verification: Option<VerificationRequirement>,
    /// Status of the step
    pub status: StepStatus,
    /// Result of execution
    pub result: Option<StepResult>,
}

/// Action to be performed in a step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepAction {
    /// Execute a command
    Command(String),
    /// Call a function
    Function(String, Vec<serde_json::Value>),
    /// Delegate to sub-agent
    Delegate(DelegationSpec),
    /// Composite action
    Composite(Vec<StepAction>),
    /// Custom action
    Custom(serde_json::Value),
}

/// Specification for delegating to a sub-agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationSpec {
    /// Target agent ID
    pub agent_id: Option<String>,
    /// Task to delegate
    pub task: String,
    /// Context to pass to sub-agent
    pub context: HashMap<String, serde_json::Value>,
    /// Timeout for delegation
    pub timeout_ms: u64,
    /// Whether to wait for completion
    pub wait_for_completion: bool,
}

/// Status of a step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
    RolledBack,
}

/// Result from step execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Whether the step succeeded
    pub success: bool,
    /// Output from the step
    pub output: Option<serde_json::Value>,
    /// Error if failed
    pub error: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Verification result
    pub verification: Option<VerificationOutcome>,
}

/// Condition that must be met
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// Condition type
    pub condition_type: ConditionType,
    /// Expected value or state
    pub expected: serde_json::Value,
    /// Whether this is critical
    pub critical: bool,
    /// Description
    pub description: Option<String>,
}

/// Type of condition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionType {
    /// File must exist
    FileExists,
    /// Command must succeed
    CommandSuccess,
    /// State must match
    StateMatch,
    /// Custom condition
    Custom,
}

/// Verification requirement for a step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRequirement {
    /// Type of verification needed
    pub verification_type: VerificationType,
    /// Expected outcome
    pub expected: serde_json::Value,
    /// Whether verification is mandatory
    pub mandatory: bool,
    /// Verification strategy
    pub strategy: VerificationStrategy,
}

/// Type of verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationType {
    Command,
    FileSystem,
    Network,
    State,
    Custom,
}

/// Strategy for verification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStrategy {
    /// Single verification
    Single,
    /// Multiple verifiers must agree
    Consensus(u32),
    /// At least N must pass
    Threshold(u32),
    /// All must pass
    All,
}

/// Outcome of verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationOutcome {
    /// Whether verification passed
    pub passed: bool,
    /// Details of the verification
    pub details: serde_json::Value,
    /// Proof ID if generated
    pub proof_id: Option<Uuid>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Metadata for an intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentMetadata {
    /// Who created the intent
    pub creator: String,
    /// When it was created
    pub created_at: DateTime<Utc>,
    /// Last modified
    pub modified_at: DateTime<Utc>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Parent intent if this is a sub-intent
    pub parent_intent: Option<IntentId>,
    /// Agent context
    pub agent_context: Option<String>,
    /// Priority level
    pub priority: Priority,
    /// Estimated duration in milliseconds
    pub estimated_duration_ms: Option<u64>,
}

/// Configuration for intent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Whether to stop on first failure
    pub stop_on_failure: bool,
    /// Whether to enable rollback
    pub enable_rollback: bool,
    /// Whether to verify each step
    pub verify_steps: bool,
    /// Maximum retries for failed steps
    pub max_retries: u32,
    /// Timeout for entire intent
    pub timeout_ms: Option<u64>,
    /// Parallelization strategy
    pub parallelization: ParallelizationStrategy,
    /// Whether to generate proofs
    pub generate_proofs: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            stop_on_failure: true,
            enable_rollback: true,
            verify_steps: true,
            max_retries: 0,
            timeout_ms: Some(300000), // 5 minutes
            parallelization: ParallelizationStrategy::Sequential,
            generate_proofs: true,
        }
    }
}

/// Strategy for parallel execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParallelizationStrategy {
    /// Execute steps sequentially
    Sequential,
    /// Execute independent steps in parallel
    Parallel,
    /// Use specified concurrency limit
    Limited(usize),
}

/// Context bounds for intent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBounds {
    /// Allowed file system paths
    pub allowed_paths: Vec<String>,
    /// Allowed commands
    pub allowed_commands: Vec<String>,
    /// Allowed network endpoints
    pub allowed_endpoints: Vec<String>,
    /// Maximum memory usage
    pub max_memory_bytes: Option<usize>,
    /// Maximum CPU time
    pub max_cpu_seconds: Option<u64>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
}

impl Default for ContextBounds {
    fn default() -> Self {
        Self {
            allowed_paths: Vec::new(),
            allowed_commands: Vec::new(),
            allowed_endpoints: Vec::new(),
            max_memory_bytes: Some(512 * 1024 * 1024), // 512MB
            max_cpu_seconds: Some(300), // 5 minutes
            env_vars: HashMap::new(),
        }
    }
}

/// Event emitted during intent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentEvent {
    /// Event ID
    pub id: Uuid,
    /// Intent ID
    pub intent_id: IntentId,
    /// Event type
    pub event_type: EventType,
    /// Event data
    pub data: serde_json::Value,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Type of event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    /// Intent started
    Started,
    /// Step started
    StepStarted,
    /// Step completed
    StepCompleted,
    /// Step failed
    StepFailed,
    /// Verification performed
    VerificationPerformed,
    /// Checkpoint created
    CheckpointCreated,
    /// Rollback initiated
    RollbackInitiated,
    /// Intent completed
    Completed,
    /// Intent failed
    Failed,
}

/// Checkpoint data for rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointData {
    /// Checkpoint ID
    pub id: Uuid,
    /// Intent ID
    pub intent_id: IntentId,
    /// Step at which checkpoint was taken
    pub step_id: Uuid,
    /// State snapshot
    pub state: HashMap<String, serde_json::Value>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether this is a safe rollback point
    pub safe_rollback: bool,
}