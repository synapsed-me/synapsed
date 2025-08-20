//! Error types for the Synapsed Safety system
//!
//! This module defines comprehensive error handling for safety-critical operations,
//! ensuring that all failure modes are properly categorized and handled.

use thiserror::Error;
use std::fmt;
use uuid::Uuid;

/// Comprehensive error types for safety operations
#[derive(Error, Debug, Clone)]
pub enum SafetyError {
    /// Constraint violation detected
    #[error("Constraint violation: {message} (constraint: {constraint_id})")]
    ConstraintViolation {
        constraint_id: String,
        message: String,
        severity: crate::types::Severity,
    },

    /// Rollback operation failed
    #[error("Rollback failed: {reason} (checkpoint: {checkpoint_id})")]
    RollbackFailed {
        checkpoint_id: Uuid,
        reason: String,
    },

    /// Safety monitor error
    #[error("Monitor error: {message}")]
    MonitorError { message: String },

    /// Constraint engine error
    #[error("Constraint engine error: {message}")]
    ConstraintEngineError { message: String },

    /// Resource limit exceeded
    #[error("Resource limit exceeded: {resource} ({current}/{limit})")]
    ResourceLimitExceeded {
        resource: String,
        current: u64,
        limit: u64,
    },

    /// Timeout error
    #[error("Operation timed out after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    /// Emergency shutdown triggered
    #[error("Emergency shutdown: {reason}")]
    EmergencyShutdown { reason: String },

    /// Checkpoint corruption detected
    #[error("Checkpoint corrupted: {checkpoint_id}")]
    CheckpointCorrupted { checkpoint_id: Uuid },

    /// State inconsistency detected
    #[error("State inconsistent: {description}")]
    StateInconsistent { description: String },

    /// Formal verification failed
    #[cfg(feature = "formal-verification")]
    #[error("Formal verification failed: {property}")]
    VerificationFailed { property: String },

    /// Self-healing failed
    #[cfg(feature = "self-healing")]
    #[error("Self-healing failed: {strategy}")]
    HealingFailed { strategy: String },

    /// External dependency error
    #[error("External error: {source}")]
    External {
        #[from]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Configuration error
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Serialization error
    #[error("Serialization error: {message}")]
    Serialization { message: String },

    /// IO error
    #[error("IO error: {message}")]
    Io { message: String },

    /// Critical system error requiring immediate attention
    #[error("CRITICAL: {message}")]
    Critical { message: String },
}

impl SafetyError {
    /// Check if error is critical and requires immediate shutdown
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            SafetyError::Critical { .. }
                | SafetyError::EmergencyShutdown { .. }
                | SafetyError::ConstraintViolation {
                    severity: crate::types::Severity::Critical,
                    ..
                }
        )
    }

    /// Check if error is recoverable through rollback
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            SafetyError::ConstraintViolation { .. }
                | SafetyError::ResourceLimitExceeded { .. }
                | SafetyError::StateInconsistent { .. }
        )
    }

    /// Get error severity level
    pub fn severity(&self) -> crate::types::Severity {
        match self {
            SafetyError::Critical { .. } => crate::types::Severity::Critical,
            SafetyError::EmergencyShutdown { .. } => crate::types::Severity::Critical,
            SafetyError::ConstraintViolation { severity, .. } => *severity,
            SafetyError::RollbackFailed { .. } => crate::types::Severity::High,
            SafetyError::MonitorError { .. } => crate::types::Severity::Medium,
            SafetyError::ResourceLimitExceeded { .. } => crate::types::Severity::High,
            SafetyError::Timeout { .. } => crate::types::Severity::Medium,
            SafetyError::CheckpointCorrupted { .. } => crate::types::Severity::High,
            SafetyError::StateInconsistent { .. } => crate::types::Severity::High,
            #[cfg(feature = "formal-verification")]
            SafetyError::VerificationFailed { .. } => crate::types::Severity::High,
            #[cfg(feature = "self-healing")]
            SafetyError::HealingFailed { .. } => crate::types::Severity::Medium,
            _ => crate::types::Severity::Low,
        }
    }

    /// Create a constraint violation error
    pub fn constraint_violation(
        constraint_id: impl Into<String>,
        message: impl Into<String>,
        severity: crate::types::Severity,
    ) -> Self {
        Self::ConstraintViolation {
            constraint_id: constraint_id.into(),
            message: message.into(),
            severity,
        }
    }

    /// Create a rollback failure error
    pub fn rollback_failed(checkpoint_id: Uuid, reason: impl Into<String>) -> Self {
        Self::RollbackFailed {
            checkpoint_id,
            reason: reason.into(),
        }
    }

    /// Create a resource limit error
    pub fn resource_limit_exceeded(
        resource: impl Into<String>,
        current: u64,
        limit: u64,
    ) -> Self {
        Self::ResourceLimitExceeded {
            resource: resource.into(),
            current,
            limit,
        }
    }

    /// Create an emergency shutdown error
    pub fn emergency_shutdown(reason: impl Into<String>) -> Self {
        Self::EmergencyShutdown {
            reason: reason.into(),
        }
    }

    /// Create a critical error
    pub fn critical(message: impl Into<String>) -> Self {
        Self::Critical {
            message: message.into(),
        }
    }
}

/// Result type for safety operations
pub type Result<T> = std::result::Result<T, SafetyError>;

/// Error context for detailed error reporting
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Operation being performed
    pub operation: String,
    /// Timestamp of error
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Additional context data
    pub context_data: std::collections::HashMap<String, String>,
    /// Stack trace if available
    pub stack_trace: Option<String>,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            timestamp: chrono::Utc::now(),
            context_data: std::collections::HashMap::new(),
            stack_trace: None,
        }
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context_data.insert(key.into(), value.into());
        self
    }

    pub fn with_stack_trace(mut self, stack_trace: String) -> Self {
        self.stack_trace = Some(stack_trace);
        self
    }
}

/// Enhanced error with context information
#[derive(Debug, Clone)]
pub struct ContextualError {
    pub error: SafetyError,
    pub context: ErrorContext,
}

impl fmt::Display for ContextualError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} - {}",
            self.context.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            self.context.operation,
            self.error
        )
    }
}

impl std::error::Error for ContextualError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Macro for creating contextual errors
#[macro_export]
macro_rules! safety_error {
    ($error:expr, $operation:expr) => {
        $crate::error::ContextualError {
            error: $error,
            context: $crate::error::ErrorContext::new($operation),
        }
    };
    ($error:expr, $operation:expr, $($key:expr => $value:expr),+) => {
        $crate::error::ContextualError {
            error: $error,
            context: $crate::error::ErrorContext::new($operation)
                $(.with_context($key, $value))+,
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Severity;

    #[test]
    fn test_error_severity() {
        let critical_error = SafetyError::critical("System failure");
        assert!(critical_error.is_critical());
        assert_eq!(critical_error.severity(), Severity::Critical);

        let constraint_error = SafetyError::constraint_violation(
            "balance_check",
            "Negative balance",
            Severity::High,
        );
        assert!(!constraint_error.is_critical());
        assert!(constraint_error.is_recoverable());
        assert_eq!(constraint_error.severity(), Severity::High);
    }

    #[test]
    fn test_error_context() {
        let context = ErrorContext::new("test_operation")
            .with_context("user_id", "12345")
            .with_context("request_id", "req_67890");

        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.context_data.get("user_id"), Some(&"12345".to_string()));
        assert_eq!(context.context_data.get("request_id"), Some(&"req_67890".to_string()));
    }

    #[test]
    fn test_contextual_error() {
        let error = SafetyError::resource_limit_exceeded("memory", 1024, 512);
        let contextual = ContextualError {
            error,
            context: ErrorContext::new("memory_allocation")
                .with_context("process_id", "1234"),
        };

        let error_string = contextual.to_string();
        assert!(error_string.contains("memory_allocation"));
        assert!(error_string.contains("Resource limit exceeded"));
    }

    #[test]
    fn test_safety_error_macro() {
        let error = SafetyError::timeout(5000);
        let contextual = safety_error!(
            error,
            "database_query",
            "table" => "users",
            "timeout" => "5000ms"
        );

        assert_eq!(contextual.context.operation, "database_query");
        assert_eq!(contextual.context.context_data.get("table"), Some(&"users".to_string()));
    }
}

// Implement missing timeout method for SafetyError
impl SafetyError {
    pub fn timeout(duration_ms: u64) -> Self {
        Self::Timeout { duration_ms }
    }
}