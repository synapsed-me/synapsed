//! Error types for swarm coordination

use thiserror::Error;

/// Result type for swarm operations
pub type SwarmResult<T> = std::result::Result<T, SwarmError>;

/// Errors that can occur during swarm coordination
#[derive(Debug, Error)]
pub enum SwarmError {
    /// Agent not found in swarm
    #[error("Agent not found: {0}")]
    AgentNotFound(uuid::Uuid),
    
    /// Intent execution failed
    #[error("Intent execution failed: {0}")]
    IntentFailed(String),
    
    /// Promise violation detected
    #[error("Promise violation by agent {agent}: {violation}")]
    PromiseViolation {
        agent: uuid::Uuid,
        violation: String,
    },
    
    /// Verification failed
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    
    /// Trust threshold not met
    #[error("Trust too low for agent {agent}: {score} < {threshold}")]
    InsufficientTrust {
        agent: uuid::Uuid,
        score: f64,
        threshold: f64,
    },
    
    /// Swarm size limit exceeded
    #[error("Swarm size limit exceeded: {current} >= {max}")]
    SwarmSizeLimitExceeded {
        current: usize,
        max: usize,
    },
    
    /// Protocol version mismatch
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    ProtocolMismatch {
        expected: String,
        actual: String,
    },
    
    /// Context propagation failed
    #[error("Failed to propagate context to agent: {0}")]
    ContextPropagationFailed(String),
    
    /// Coordination timeout
    #[error("Coordination timeout after {0} seconds")]
    CoordinationTimeout(u64),
    
    /// Communication error between agents
    #[error("Agent communication error: {0}")]
    CommunicationError(String),
    
    /// Consensus not reached
    #[error("Consensus not reached among agents: {reason}")]
    ConsensusFailure {
        reason: String,
    },
    
    /// Resource conflict between agents
    #[error("Resource conflict: {resource} requested by multiple agents")]
    ResourceConflict {
        resource: String,
    },
    
    /// Invalid agent configuration
    #[error("Invalid agent configuration: {0}")]
    InvalidConfiguration(String),
    
    /// Delegation failed
    #[error("Failed to delegate to sub-agent: {0}")]
    DelegationFailed(String),
    
    /// Recovery failed
    #[error("Failed to recover from error: {0}")]
    RecoveryFailed(String),
    
    /// Storage operation failed
    #[error("Storage error: {0}")]
    StorageError(String),
    
    /// Transaction failed
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    
    /// Concurrent access error
    #[error("Concurrent access error: {0}")]
    ConcurrencyError(String),
    
    /// Migration failed
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
    
    /// Backup/restore operation failed
    #[error("Backup operation failed: {0}")]
    BackupFailed(String),
    
    /// Monitoring system error
    #[error("Monitoring error: {0}")]
    MonitoringError(String),
    
    /// Intent error
    #[error("Intent error: {0}")]
    Intent(#[from] synapsed_intent::IntentError),
    
    /// Promise error
    #[error("Promise error: {0}")]
    Promise(#[from] synapsed_promise::PromiseError),
    
    /// Verification error
    #[error("Verification error: {0}")]
    Verify(#[from] synapsed_verify::VerifyError),
    
    /// Other errors
    #[error("Swarm error: {0}")]
    Other(#[from] anyhow::Error),
}

impl From<SwarmError> for synapsed_core::SynapsedError {
    fn from(err: SwarmError) -> Self {
        match err {
            SwarmError::AgentNotFound(_) => synapsed_core::SynapsedError::NotFound(err.to_string()),
            SwarmError::IntentFailed(msg) => synapsed_core::SynapsedError::Internal(msg),
            SwarmError::VerificationFailed(msg) => synapsed_core::SynapsedError::InvalidInput(msg),
            SwarmError::CoordinationTimeout(secs) => {
                synapsed_core::SynapsedError::Timeout(format!("{}s", secs))
            }
            SwarmError::CommunicationError(msg) => synapsed_core::SynapsedError::Network(msg),
            SwarmError::StorageError(msg) => synapsed_core::SynapsedError::Internal(format!("Storage: {}", msg)),
            SwarmError::TransactionFailed(msg) => synapsed_core::SynapsedError::Internal(format!("Transaction: {}", msg)),
            SwarmError::ConcurrencyError(msg) => synapsed_core::SynapsedError::Internal(format!("Concurrency: {}", msg)),
            SwarmError::MigrationFailed(msg) => synapsed_core::SynapsedError::Internal(format!("Migration: {}", msg)),
            SwarmError::BackupFailed(msg) => synapsed_core::SynapsedError::Internal(format!("Backup: {}", msg)),
            _ => synapsed_core::SynapsedError::Internal(err.to_string()),
        }
    }
}