//! Error types for CRDT operations

use thiserror::Error;

/// CRDT operation errors
#[derive(Error, Debug, Clone, PartialEq)]
pub enum CrdtError {
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    
    #[error("Node not found: {id}")]
    NodeNotFound { id: String },
    
    #[error("Clock synchronization error: {0}")]
    ClockError(String),
    
    #[error("Merge conflict: {0}")]
    MergeConflict(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Actor ID mismatch: expected {expected}, got {actual}")]
    ActorIdMismatch { expected: String, actual: String },
    
    #[error("Version vector error: {0}")]
    VersionVectorError(String),
    
    #[error("Merkle tree error: {0}")]
    MerkleTreeError(String),
    
    #[error("Concurrent modification detected")]
    ConcurrentModification,
    
    #[error("Operation timeout")]
    Timeout,
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for CrdtError {
    fn from(err: serde_json::Error) -> Self {
        CrdtError::SerializationError(err.to_string())
    }
}

impl From<anyhow::Error> for CrdtError {
    fn from(err: anyhow::Error) -> Self {
        CrdtError::Internal(err.to_string())
    }
}

/// Result type for CRDT operations
pub type Result<T> = std::result::Result<T, CrdtError>;