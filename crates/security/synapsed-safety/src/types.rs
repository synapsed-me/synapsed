//! Core types for the Synapsed Safety system
//!
//! This module defines the fundamental data structures used throughout
//! the safety system, including constraints, states, and safety metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Severity levels for safety violations and events
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Low severity - informational or minor issues
    Low = 1,
    /// Medium severity - requires attention but not critical
    Medium = 2,
    /// High severity - requires immediate attention
    High = 3,
    /// Critical severity - system-threatening issues
    Critical = 4,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Low => write!(f, "LOW"),
            Severity::Medium => write!(f, "MEDIUM"),
            Severity::High => write!(f, "HIGH"),
            Severity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Unique identifier for checkpoints
pub type CheckpointId = Uuid;

/// Unique identifier for constraints
pub type ConstraintId = String;

/// Safety state representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyState {
    /// Unique state identifier
    pub id: Uuid,
    /// Timestamp when state was captured
    pub timestamp: DateTime<Utc>,
    /// Current system values
    pub values: HashMap<String, StateValue>,
    /// Active constraints
    pub active_constraints: Vec<ConstraintId>,
    /// Resource usage metrics
    pub resource_usage: ResourceUsage,
    /// System health indicators
    pub health_indicators: HealthIndicators,
    /// Metadata about the state
    pub metadata: StateMetadata,
}

/// Possible values in system state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateValue {
    /// Integer value
    Integer(i64),
    /// Floating point value
    Float(f64),
    /// String value
    String(String),
    /// Boolean value
    Boolean(bool),
    /// Nested object
    Object(HashMap<String, StateValue>),
    /// Array of values
    Array(Vec<StateValue>),
    /// Null value
    Null,
}

impl StateValue {
    /// Convert to integer if possible
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            StateValue::Integer(v) => Some(*v),
            StateValue::Float(v) => Some(*v as i64),
            _ => None,
        }
    }

    /// Convert to float if possible
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            StateValue::Float(v) => Some(*v),
            StateValue::Integer(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// Convert to string if possible
    pub fn as_str(&self) -> Option<&str> {
        match self {
            StateValue::String(v) => Some(v),
            _ => None,
        }
    }

    /// Convert to boolean if possible
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            StateValue::Boolean(v) => Some(*v),
            _ => None,
        }
    }
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage (0.0 to 1.0)
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Maximum memory allowed
    pub memory_limit: u64,
    /// Network bandwidth usage in bytes/sec
    pub network_usage: u64,
    /// Disk I/O usage in bytes/sec
    pub disk_io: u64,
    /// File descriptor count
    pub file_descriptors: u32,
    /// Thread count
    pub thread_count: u32,
    /// Custom resource metrics
    pub custom_resources: HashMap<String, f64>,
}

impl ResourceUsage {
    /// Check if memory usage is within limits
    pub fn is_memory_within_limits(&self) -> bool {
        self.memory_usage <= self.memory_limit
    }

    /// Get memory usage percentage
    pub fn memory_usage_percentage(&self) -> f64 {
        if self.memory_limit == 0 {
            0.0
        } else {
            self.memory_usage as f64 / self.memory_limit as f64
        }
    }

    /// Check if any resource is over threshold
    pub fn is_over_threshold(&self, threshold: f64) -> bool {
        self.cpu_usage > threshold
            || self.memory_usage_percentage() > threshold
            || self.custom_resources.values().any(|&v| v > threshold)
    }
}

/// System health indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthIndicators {
    /// Overall system health score (0.0 to 1.0)
    pub overall_health: f64,
    /// Component health scores
    pub component_health: HashMap<String, f64>,
    /// Error rates
    pub error_rates: HashMap<String, f64>,
    /// Response times
    pub response_times: HashMap<String, f64>,
    /// Availability metrics
    pub availability: HashMap<String, f64>,
    /// Performance indicators
    pub performance_indicators: HashMap<String, f64>,
}

/// Metadata about a safety state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMetadata {
    /// Source of the state
    pub source: String,
    /// Version of the state format
    pub version: String,
    /// Checksum of the state data
    pub checksum: String,
    /// Size of serialized state in bytes
    pub size_bytes: u64,
    /// Compression ratio if compressed
    pub compression_ratio: Option<f64>,
    /// Tags associated with the state
    pub tags: Vec<String>,
    /// Additional properties
    pub properties: HashMap<String, String>,
}

/// Safety constraint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Unique constraint identifier
    pub id: ConstraintId,
    /// Human-readable name
    pub name: String,
    /// Detailed description
    pub description: String,
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Severity level
    pub severity: Severity,
    /// Whether constraint is enabled
    pub enabled: bool,
    /// Constraint rule definition
    pub rule: ConstraintRule,
    /// Actions to take on violation
    pub actions: Vec<ConstraintAction>,
    /// Metadata about the constraint
    pub metadata: ConstraintMetadata,
}

/// Types of constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    /// Invariant constraint (always true)
    Invariant,
    /// Pre-condition constraint
    Precondition,
    /// Post-condition constraint
    Postcondition,
    /// Temporal constraint (time-based)
    Temporal,
    /// Resource constraint
    Resource,
    /// Custom constraint type
    Custom(String),
}

/// Constraint rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintRule {
    /// Rule expression
    pub expression: String,
    /// Rule parameters
    pub parameters: HashMap<String, StateValue>,
    /// Evaluation context
    pub context: RuleContext,
    /// Timeout for evaluation
    pub timeout_ms: Option<u64>,
}

/// Context for constraint rule evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleContext {
    /// Variables available in the rule
    pub variables: HashMap<String, StateValue>,
    /// Functions available in the rule
    pub functions: Vec<String>,
    /// Scope of the rule evaluation
    pub scope: String,
}

/// Actions to take when constraint is violated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintAction {
    /// Log the violation
    Log {
        level: String,
        message: String,
    },
    /// Send alert/notification
    Alert {
        channel: String,
        message: String,
        urgency: Severity,
    },
    /// Trigger rollback to checkpoint
    Rollback {
        checkpoint_id: Option<CheckpointId>,
        automatic: bool,
    },
    /// Execute custom action
    Execute {
        command: String,
        parameters: HashMap<String, String>,
    },
    /// Throttle operations
    Throttle {
        rate_limit: f64,
        duration_ms: u64,
    },
    /// Shutdown component
    Shutdown {
        component: String,
        graceful: bool,
    },
    /// Emergency stop
    EmergencyStop,
}

/// Metadata about a constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintMetadata {
    /// When constraint was created
    pub created_at: DateTime<Utc>,
    /// Who created the constraint
    pub created_by: String,
    /// When constraint was last modified
    pub modified_at: DateTime<Utc>,
    /// Version of the constraint
    pub version: u32,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom properties
    pub properties: HashMap<String, String>,
}

/// Result of constraint validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub passed: bool,
    /// Violated constraints
    pub violations: Vec<ConstraintViolation>,
    /// Warnings (non-critical issues)
    pub warnings: Vec<ConstraintWarning>,
    /// Validation metadata
    pub metadata: ValidationMetadata,
}

/// Details of a constraint violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintViolation {
    /// Constraint that was violated
    pub constraint_id: ConstraintId,
    /// Severity of the violation
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
    /// Current state value that caused violation
    pub actual_value: StateValue,
    /// Expected value or range
    pub expected_value: Option<StateValue>,
    /// Timestamp of violation
    pub timestamp: DateTime<Utc>,
    /// Context information
    pub context: HashMap<String, String>,
}

/// Warning about potential constraint issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintWarning {
    /// Constraint that generated warning
    pub constraint_id: ConstraintId,
    /// Warning message
    pub message: String,
    /// Suggested action
    pub suggested_action: Option<String>,
    /// Timestamp of warning
    pub timestamp: DateTime<Utc>,
}

/// Metadata about validation process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMetadata {
    /// How long validation took
    pub duration_ms: u64,
    /// Number of constraints evaluated
    pub constraints_evaluated: u32,
    /// Evaluation engine used
    pub engine: String,
    /// Additional metrics
    pub metrics: HashMap<String, f64>,
}

/// Checkpoint data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique checkpoint identifier
    pub id: CheckpointId,
    /// When checkpoint was created
    pub timestamp: DateTime<Utc>,
    /// System state at checkpoint
    pub state: SafetyState,
    /// Description of checkpoint
    pub description: String,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Size of checkpoint data
    pub size_bytes: u64,
    /// Compression used
    pub compression: Option<String>,
    /// Integrity hash
    pub integrity_hash: String,
}

/// Configuration for safety system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Maximum number of checkpoints to keep
    pub max_checkpoints: u32,
    /// Interval between automatic checkpoints
    pub checkpoint_interval_ms: u64,
    /// Constraint check interval
    pub constraint_check_interval_ms: u64,
    /// Memory limit for safety system
    pub memory_limit_bytes: u64,
    /// Enable compression for checkpoints
    pub compression_enabled: bool,
    /// Compression algorithm to use
    pub compression_algorithm: String,
    /// Enable formal verification
    pub formal_verification_enabled: bool,
    /// Enable self-healing
    pub self_healing_enabled: bool,
    /// Custom configuration properties
    pub custom_properties: HashMap<String, String>,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            max_checkpoints: 100,
            checkpoint_interval_ms: 60_000, // 1 minute
            constraint_check_interval_ms: 1_000, // 1 second
            memory_limit_bytes: 100 * 1024 * 1024, // 100MB
            compression_enabled: true,
            compression_algorithm: "zstd".to_string(),
            formal_verification_enabled: false,
            self_healing_enabled: true,
            custom_properties: HashMap::new(),
        }
    }
}

/// Statistics about safety system operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyStats {
    /// Total constraints evaluated
    pub constraints_evaluated: u64,
    /// Total violations detected
    pub violations_detected: u64,
    /// Total rollbacks performed
    pub rollbacks_performed: u32,
    /// Total checkpoints created
    pub checkpoints_created: u32,
    /// Average constraint evaluation time
    pub avg_evaluation_time_ms: f64,
    /// System uptime
    pub uptime_ms: u64,
    /// Memory usage statistics
    pub memory_stats: MemoryStats,
    /// Performance metrics
    pub performance_metrics: HashMap<String, f64>,
}

/// Memory usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Current memory usage
    pub current_usage_bytes: u64,
    /// Peak memory usage
    pub peak_usage_bytes: u64,
    /// Average memory usage
    pub avg_usage_bytes: u64,
    /// Memory allocated for checkpoints
    pub checkpoint_memory_bytes: u64,
    /// Memory allocated for constraints
    pub constraint_memory_bytes: u64,
}

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
    fn test_state_value_conversions() {
        let int_val = StateValue::Integer(42);
        assert_eq!(int_val.as_i64(), Some(42));
        assert_eq!(int_val.as_f64(), Some(42.0));
        assert_eq!(int_val.as_str(), None);

        let str_val = StateValue::String("hello".to_string());
        assert_eq!(str_val.as_str(), Some("hello"));
        assert_eq!(str_val.as_i64(), None);
    }

    #[test]
    fn test_resource_usage() {
        let usage = ResourceUsage {
            cpu_usage: 0.5,
            memory_usage: 512,
            memory_limit: 1024,
            network_usage: 100,
            disk_io: 50,
            file_descriptors: 10,
            thread_count: 5,
            custom_resources: HashMap::new(),
        };

        assert!(usage.is_memory_within_limits());
        assert_eq!(usage.memory_usage_percentage(), 0.5);
        assert!(!usage.is_over_threshold(0.6));
        assert!(usage.is_over_threshold(0.4));
    }

    #[test]
    fn test_safety_config_default() {
        let config = SafetyConfig::default();
        assert_eq!(config.max_checkpoints, 100);
        assert_eq!(config.compression_algorithm, "zstd");
        assert!(config.compression_enabled);
        assert!(!config.formal_verification_enabled);
        assert!(config.self_healing_enabled);
    }

    #[test]
    fn test_constraint_creation() {
        let constraint = Constraint {
            id: "test_constraint".to_string(),
            name: "Test Constraint".to_string(),
            description: "A test constraint".to_string(),
            constraint_type: ConstraintType::Invariant,
            severity: Severity::High,
            enabled: true,
            rule: ConstraintRule {
                expression: "value > 0".to_string(),
                parameters: HashMap::new(),
                context: RuleContext {
                    variables: HashMap::new(),
                    functions: vec![],
                    scope: "global".to_string(),
                },
                timeout_ms: Some(1000),
            },
            actions: vec![ConstraintAction::Log {
                level: "error".to_string(),
                message: "Constraint violated".to_string(),
            }],
            metadata: ConstraintMetadata {
                created_at: Utc::now(),
                created_by: "test".to_string(),
                modified_at: Utc::now(),
                version: 1,
                tags: vec!["test".to_string()],
                properties: HashMap::new(),
            },
        };

        assert_eq!(constraint.id, "test_constraint");
        assert_eq!(constraint.severity, Severity::High);
        assert!(constraint.enabled);
    }
}