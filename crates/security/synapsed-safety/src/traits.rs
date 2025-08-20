//! Core traits for the Synapsed Safety system
//!
//! This module defines the key abstractions that enable pluggable and
//! extensible safety mechanisms throughout the system.

use crate::error::Result;
use crate::types::*;
use async_trait::async_trait;
use std::time::Duration;
use uuid::Uuid;

/// Core trait for safety monitoring systems
///
/// Implementors of this trait provide real-time monitoring capabilities
/// for safety-critical operations and can detect violations as they occur.
#[async_trait]
pub trait SafetyMonitor: Send + Sync {
    /// Start monitoring the system state
    async fn start_monitoring(&mut self) -> Result<()>;

    /// Stop monitoring and cleanup resources
    async fn stop_monitoring(&mut self) -> Result<()>;

    /// Check if monitor is currently active
    fn is_active(&self) -> bool;

    /// Get current system state
    async fn get_current_state(&self) -> Result<SafetyState>;

    /// Subscribe to state changes
    async fn subscribe_to_changes(&mut self, callback: Box<dyn StateChangeCallback>) -> Result<()>;

    /// Unsubscribe from state changes
    async fn unsubscribe_from_changes(&mut self) -> Result<()>;

    /// Get monitoring statistics
    async fn get_stats(&self) -> Result<MonitoringStats>;

    /// Set monitoring configuration
    async fn configure(&mut self, config: MonitorConfig) -> Result<()>;

    /// Perform health check on monitor
    async fn health_check(&self) -> Result<HealthStatus>;

    /// Get monitor metadata
    fn get_metadata(&self) -> MonitorMetadata;
}

/// Trait for constraint evaluation engines
///
/// Constraint engines are responsible for evaluating safety rules
/// against system state and determining violations.
#[async_trait]
pub trait ConstraintEngine: Send + Sync {
    /// Add a new constraint to the engine
    async fn add_constraint(&mut self, constraint: Constraint) -> Result<()>;

    /// Remove a constraint by ID
    async fn remove_constraint(&mut self, constraint_id: &ConstraintId) -> Result<()>;

    /// Update an existing constraint
    async fn update_constraint(&mut self, constraint: Constraint) -> Result<()>;

    /// Get constraint by ID
    async fn get_constraint(&self, constraint_id: &ConstraintId) -> Result<Option<Constraint>>;

    /// List all constraints
    async fn list_constraints(&self) -> Result<Vec<Constraint>>;

    /// Validate state against all active constraints
    async fn validate_state(&self, state: &SafetyState) -> Result<ValidationResult>;

    /// Validate state against specific constraints
    async fn validate_constraints(
        &self,
        state: &SafetyState,
        constraint_ids: &[ConstraintId],
    ) -> Result<ValidationResult>;

    /// Enable/disable a constraint
    async fn set_constraint_enabled(&mut self, constraint_id: &ConstraintId, enabled: bool) -> Result<()>;

    /// Get engine statistics
    async fn get_stats(&self) -> Result<EngineStats>;

    /// Optimize constraint evaluation order
    async fn optimize(&mut self) -> Result<()>;

    /// Export constraints to a serializable format
    async fn export_constraints(&self) -> Result<String>;

    /// Import constraints from serialized format
    async fn import_constraints(&mut self, data: &str) -> Result<()>;
}

/// Trait for rollback and checkpoint management
///
/// Rollback managers handle state snapshots and recovery operations
/// when safety violations are detected.
#[async_trait]
pub trait RollbackManager: Send + Sync {
    /// Create a new checkpoint of current state
    async fn create_checkpoint(&mut self, description: Option<String>) -> Result<CheckpointId>;

    /// Create a tagged checkpoint
    async fn create_tagged_checkpoint(
        &mut self,
        description: Option<String>,
        tags: Vec<String>,
    ) -> Result<CheckpointId>;

    /// Rollback to a specific checkpoint
    async fn rollback_to_checkpoint(&mut self, checkpoint_id: &CheckpointId) -> Result<()>;

    /// Rollback to the most recent checkpoint
    async fn rollback_to_latest(&mut self) -> Result<CheckpointId>;

    /// Rollback to checkpoint with specific tag
    async fn rollback_to_tag(&mut self, tag: &str) -> Result<CheckpointId>;

    /// Delete a specific checkpoint
    async fn delete_checkpoint(&mut self, checkpoint_id: &CheckpointId) -> Result<()>;

    /// List all available checkpoints
    async fn list_checkpoints(&self) -> Result<Vec<CheckpointSummary>>;

    /// Get checkpoint details
    async fn get_checkpoint(&self, checkpoint_id: &CheckpointId) -> Result<Option<Checkpoint>>;

    /// Compress old checkpoints to save space
    async fn compress_checkpoints(&mut self, older_than: Duration) -> Result<CompressionStats>;

    /// Validate checkpoint integrity
    async fn validate_checkpoint(&self, checkpoint_id: &CheckpointId) -> Result<bool>;

    /// Get rollback statistics
    async fn get_stats(&self) -> Result<RollbackStats>;

    /// Set retention policy for checkpoints
    async fn set_retention_policy(&mut self, policy: RetentionPolicy) -> Result<()>;

    /// Export checkpoint to external storage
    async fn export_checkpoint(&self, checkpoint_id: &CheckpointId, destination: &str) -> Result<()>;

    /// Import checkpoint from external storage
    async fn import_checkpoint(&mut self, source: &str) -> Result<CheckpointId>;
}

/// Trait for resource limiting and sandboxing
///
/// Resource limiters enforce boundaries on system resource usage
/// to prevent resource exhaustion attacks or runaway processes.
#[async_trait]
pub trait ResourceLimiter: Send + Sync {
    /// Set memory limit
    async fn set_memory_limit(&mut self, limit_bytes: u64) -> Result<()>;

    /// Set CPU usage limit
    async fn set_cpu_limit(&mut self, limit_percentage: f64) -> Result<()>;

    /// Set network bandwidth limit
    async fn set_network_limit(&mut self, limit_bytes_per_sec: u64) -> Result<()>;

    /// Set disk I/O limit
    async fn set_disk_io_limit(&mut self, limit_bytes_per_sec: u64) -> Result<()>;

    /// Set file descriptor limit
    async fn set_file_descriptor_limit(&mut self, limit: u32) -> Result<()>;

    /// Set thread count limit
    async fn set_thread_limit(&mut self, limit: u32) -> Result<()>;

    /// Set custom resource limit
    async fn set_custom_limit(&mut self, resource: &str, limit: f64) -> Result<()>;

    /// Check if resource usage is within limits
    async fn check_resource_usage(&self) -> Result<ResourceUsage>;

    /// Enforce resource limits (kill processes if needed)
    async fn enforce_limits(&mut self) -> Result<EnforcementResult>;

    /// Get current resource limits
    async fn get_limits(&self) -> Result<ResourceLimits>;

    /// Subscribe to resource limit violations
    async fn subscribe_to_violations(&mut self, callback: Box<dyn ResourceViolationCallback>) -> Result<()>;

    /// Get resource usage history
    async fn get_usage_history(&self, duration: Duration) -> Result<Vec<ResourceUsageSnapshot>>;

    /// Predict resource usage trends
    async fn predict_usage(&self, horizon: Duration) -> Result<ResourceUsagePrediction>;
}

/// Trait for self-healing safety systems
///
/// Self-healing systems can automatically adapt to prevent
/// recurring safety violations and improve system resilience.
#[cfg(feature = "self-healing")]
#[async_trait]
pub trait SelfHealing: Send + Sync {
    /// Analyze violation patterns and suggest improvements
    async fn analyze_patterns(&self, violations: &[ConstraintViolation]) -> Result<Vec<HealingRecommendation>>;

    /// Apply healing recommendations
    async fn apply_healing(&mut self, recommendations: &[HealingRecommendation]) -> Result<HealingResult>;

    /// Learn from successful/failed healing attempts
    async fn learn_from_outcome(&mut self, healing_id: Uuid, outcome: HealingOutcome) -> Result<()>;

    /// Get healing statistics
    async fn get_healing_stats(&self) -> Result<HealingStats>;

    /// Enable/disable specific healing strategies
    async fn configure_strategies(&mut self, strategies: Vec<HealingStrategy>) -> Result<()>;

    /// Preview healing recommendations without applying
    async fn preview_healing(&self, violations: &[ConstraintViolation]) -> Result<Vec<HealingRecommendation>>;
}

/// Trait for formal verification systems
///
/// Formal verification provides mathematical proofs of safety properties
/// using theorem provers and model checkers.
#[cfg(feature = "formal-verification")]
#[async_trait]
pub trait FormalVerifier: Send + Sync {
    /// Verify safety property using formal methods
    async fn verify_property(&self, property: &FormalProperty) -> Result<VerificationResult>;

    /// Generate proof for safety property
    async fn generate_proof(&self, property: &FormalProperty) -> Result<Proof>;

    /// Validate existing proof
    async fn validate_proof(&self, proof: &Proof) -> Result<bool>;

    /// Find counterexamples for false properties
    async fn find_counterexample(&self, property: &FormalProperty) -> Result<Option<Counterexample>>;

    /// Convert constraints to formal properties
    async fn constraints_to_properties(&self, constraints: &[Constraint]) -> Result<Vec<FormalProperty>>;

    /// Get verification statistics
    async fn get_verification_stats(&self) -> Result<VerificationStats>;
}

/// Callback trait for state change notifications
#[async_trait]
pub trait StateChangeCallback: Send + Sync {
    /// Called when system state changes
    async fn on_state_change(&mut self, old_state: &SafetyState, new_state: &SafetyState) -> Result<()>;

    /// Called when a constraint violation is detected
    async fn on_violation(&mut self, violation: &ConstraintViolation) -> Result<()>;

    /// Called when a checkpoint is created
    async fn on_checkpoint_created(&mut self, checkpoint_id: &CheckpointId) -> Result<()>;

    /// Called when a rollback occurs
    async fn on_rollback(&mut self, checkpoint_id: &CheckpointId) -> Result<()>;
}

/// Callback trait for resource violation notifications
#[async_trait]
pub trait ResourceViolationCallback: Send + Sync {
    /// Called when resource limit is exceeded
    async fn on_resource_violation(&mut self, resource: &str, usage: f64, limit: f64) -> Result<()>;

    /// Called when resource usage is approaching limit
    async fn on_resource_warning(&mut self, resource: &str, usage: f64, limit: f64, threshold: f64) -> Result<()>;
}

// Supporting types for trait implementations

/// Statistics about monitoring operations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonitoringStats {
    pub states_monitored: u64,
    pub violations_detected: u64,
    pub avg_check_duration_ms: f64,
    pub uptime_ms: u64,
    pub memory_usage_bytes: u64,
}

/// Configuration for safety monitors
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonitorConfig {
    pub check_interval_ms: u64,
    pub memory_limit_bytes: u64,
    pub enable_predictive_analysis: bool,
    pub violation_threshold: f64,
    pub custom_settings: std::collections::HashMap<String, String>,
}

/// Health status of a monitor
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub issues: Vec<String>,
    pub performance_score: f64,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Metadata about a monitor
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonitorMetadata {
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub supported_constraints: Vec<String>,
}

/// Statistics about constraint engine operations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EngineStats {
    pub constraints_count: u32,
    pub evaluations_performed: u64,
    pub violations_found: u64,
    pub avg_evaluation_time_ms: f64,
    pub optimization_level: f64,
}

/// Summary information about a checkpoint
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckpointSummary {
    pub id: CheckpointId,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub description: String,
    pub tags: Vec<String>,
    pub size_bytes: u64,
    pub compressed: bool,
}

/// Statistics about compression operations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompressionStats {
    pub checkpoints_compressed: u32,
    pub bytes_saved: u64,
    pub compression_ratio: f64,
    pub duration_ms: u64,
}

/// Statistics about rollback operations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RollbackStats {
    pub checkpoints_created: u32,
    pub rollbacks_performed: u32,
    pub avg_checkpoint_size_bytes: u64,
    pub avg_rollback_time_ms: f64,
    pub success_rate: f64,
}

/// Policy for checkpoint retention
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetentionPolicy {
    pub max_checkpoints: u32,
    pub max_age_hours: u32,
    pub max_total_size_bytes: u64,
    pub compress_after_hours: u32,
    pub delete_compressed_after_days: u32,
}

/// Resource limits configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceLimits {
    pub memory_bytes: Option<u64>,
    pub cpu_percentage: Option<f64>,
    pub network_bytes_per_sec: Option<u64>,
    pub disk_io_bytes_per_sec: Option<u64>,
    pub file_descriptors: Option<u32>,
    pub threads: Option<u32>,
    pub custom_limits: std::collections::HashMap<String, f64>,
}

/// Result of resource limit enforcement
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnforcementResult {
    pub actions_taken: Vec<EnforcementAction>,
    pub violations_resolved: u32,
    pub processes_terminated: u32,
    pub resources_freed: ResourceUsage,
}

/// Actions taken during enforcement
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EnforcementAction {
    ProcessTerminated { pid: u32, resource: String },
    ResourceThrottled { resource: String, new_limit: f64 },
    MemoryFreed { bytes: u64 },
    ConnectionsClosed { count: u32 },
    FilesUnlocked { count: u32 },
}

/// Snapshot of resource usage at a point in time
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceUsageSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub usage: ResourceUsage,
}

/// Prediction of future resource usage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceUsagePrediction {
    pub predicted_usage: ResourceUsage,
    pub confidence: f64,
    pub trend: ResourceTrend,
    pub risk_factors: Vec<String>,
}

/// Trend in resource usage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ResourceTrend {
    Increasing,
    Decreasing,
    Stable,
    Oscillating,
    Unknown,
}

// Self-healing types (feature gated)
#[cfg(feature = "self-healing")]
pub mod healing {
    use super::*;

    /// Recommendation for healing system issues
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct HealingRecommendation {
        pub id: Uuid,
        pub priority: Severity,
        pub description: String,
        pub action: HealingAction,
        pub expected_benefit: f64,
        pub risk_level: f64,
        pub estimated_cost: HealingCost,
    }

    /// Actions that can be taken for healing
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub enum HealingAction {
        AdjustConstraint { constraint_id: String, new_threshold: f64 },
        AddConstraint { constraint: Constraint },
        ModifyResource { resource: String, new_limit: f64 },
        RestartComponent { component: String },
        ScaleResource { resource: String, scale_factor: f64 },
        UpdateConfiguration { key: String, value: String },
    }

    /// Cost estimate for healing actions
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct HealingCost {
        pub cpu_cost: f64,
        pub memory_cost: f64,
        pub downtime_ms: u64,
        pub risk_score: f64,
    }

    /// Result of applying healing recommendations
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct HealingResult {
        pub applied_recommendations: Vec<Uuid>,
        pub failed_recommendations: Vec<(Uuid, String)>,
        pub improvement_score: f64,
        pub side_effects: Vec<String>,
    }

    /// Outcome of a healing attempt
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub enum HealingOutcome {
        Success { improvement: f64 },
        Failure { reason: String },
        PartialSuccess { improvement: f64, issues: Vec<String> },
    }

    /// Statistics about healing operations
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct HealingStats {
        pub recommendations_generated: u64,
        pub recommendations_applied: u64,
        pub success_rate: f64,
        pub avg_improvement: f64,
        pub avg_healing_time_ms: f64,
    }

    /// Strategy for self-healing
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct HealingStrategy {
        pub name: String,
        pub enabled: bool,
        pub priority: i32,
        pub trigger_conditions: Vec<String>,
        pub parameters: std::collections::HashMap<String, String>,
    }
}

// Formal verification types (feature gated)
#[cfg(feature = "formal-verification")]
pub mod formal {
    use super::*;

    /// Formal property for verification
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct FormalProperty {
        pub name: String,
        pub formula: String,
        pub logic: Logic,
        pub variables: std::collections::HashMap<String, VariableType>,
        pub assumptions: Vec<String>,
    }

    /// Logical systems supported
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub enum Logic {
        PropositionalLogic,
        FirstOrderLogic,
        TemporalLogic,
        LinearArithmetic,
        BitVector,
    }

    /// Types of variables in formal properties
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub enum VariableType {
        Integer,
        Real,
        Boolean,
        BitVector { width: u32 },
        Array { index_type: Box<VariableType>, element_type: Box<VariableType> },
    }

    /// Result of formal verification
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct VerificationResult {
        pub property_name: String,
        pub result: VerificationOutcome,
        pub duration_ms: u64,
        pub solver_info: SolverInfo,
    }

    /// Outcome of verification
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub enum VerificationOutcome {
        Valid,
        Invalid { counterexample: Counterexample },
        Unknown { reason: String },
        Timeout,
        Error { message: String },
    }

    /// Proof of a property
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct Proof {
        pub property_name: String,
        pub proof_steps: Vec<ProofStep>,
        pub assumptions_used: Vec<String>,
        pub proof_system: String,
    }

    /// Step in a proof
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ProofStep {
        pub step_number: u32,
        pub rule: String,
        pub premises: Vec<String>,
        pub conclusion: String,
        pub justification: String,
    }

    /// Counterexample for invalid properties
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct Counterexample {
        pub variable_assignments: std::collections::HashMap<String, String>,
        pub execution_trace: Vec<TraceStep>,
        pub violated_assertions: Vec<String>,
    }

    /// Step in execution trace
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct TraceStep {
        pub step: u32,
        pub action: String,
        pub state: std::collections::HashMap<String, String>,
    }

    /// Information about the solver used
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct SolverInfo {
        pub name: String,
        pub version: String,
        pub configuration: std::collections::HashMap<String, String>,
    }

    /// Statistics about verification operations
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct VerificationStats {
        pub properties_verified: u64,
        pub properties_valid: u64,
        pub properties_invalid: u64,
        pub avg_verification_time_ms: f64,
        pub solver_timeouts: u64,
    }
}

// Re-export feature-gated modules
#[cfg(feature = "self-healing")]
pub use healing::*;

#[cfg(feature = "formal-verification")]
pub use formal::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits {
            memory_bytes: Some(1024 * 1024),
            cpu_percentage: Some(80.0),
            network_bytes_per_sec: None,
            disk_io_bytes_per_sec: None,
            file_descriptors: Some(1000),
            threads: Some(100),
            custom_limits: std::collections::HashMap::new(),
        };

        assert_eq!(limits.memory_bytes, Some(1024 * 1024));
        assert_eq!(limits.cpu_percentage, Some(80.0));
        assert!(limits.network_bytes_per_sec.is_none());
    }

    #[test]
    fn test_checkpoint_summary() {
        let summary = CheckpointSummary {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            description: "Test checkpoint".to_string(),
            tags: vec!["test".to_string()],
            size_bytes: 1024,
            compressed: true,
        };

        assert_eq!(summary.description, "Test checkpoint");
        assert!(summary.compressed);
        assert_eq!(summary.tags.len(), 1);
    }
}