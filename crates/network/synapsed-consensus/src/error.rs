//! Error types for consensus operations

use thiserror::Error;

/// Result type for consensus operations
pub type Result<T> = std::result::Result<T, ConsensusError>;

/// Errors that can occur during consensus operations
#[derive(Error, Debug)]
pub enum ConsensusError {
    /// Invalid configuration
    #[error("Invalid consensus configuration: {0}")]
    InvalidConfiguration(String),
    
    /// Cryptographic operation failed
    #[error("Cryptographic error: {0}")]
    CryptographicError(String),
    
    /// Network communication error
    #[error("Network error: {0}")]
    NetworkError(String),
    
    /// Invalid vote received
    #[error("Invalid vote: {0}")]
    InvalidVote(String),
    
    /// Invalid block proposal
    #[error("Invalid block proposal: {0}")]
    InvalidProposal(String),
    
    /// View change error
    #[error("View change error: {0}")]
    ViewChangeError(String),
    
    /// Timeout occurred
    #[error("Timeout: {0}")]
    Timeout(String),
    
    /// State machine error
    #[error("State machine error: {0}")]
    StateMachineError(String),
    
    /// Insufficient votes for quorum (legacy)
    #[error("Insufficient votes: got {got}, need {needed}")]
    InsufficientVotesLegacy { got: usize, needed: usize },
    
    /// Node not in validator set
    #[error("Node {0} not in validator set")]
    UnknownValidator(crate::NodeId),
    
    /// Consensus protocol already started
    #[error("Consensus protocol already started")]
    AlreadyStarted,
    
    /// Consensus protocol not started
    #[error("Consensus protocol not started")]
    NotStarted,
    
    /// Invalid view number
    #[error("Invalid view number: {0}")]
    InvalidView(u64),
    
    /// Block validation failed
    #[error("Block validation failed: {0}")]
    ValidationFailed(String),
    
    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
    
    /// Invalid leader for current view
    #[error("Invalid leader")]
    InvalidLeader,
    
    /// Invalid block received
    #[error("Invalid block")]
    InvalidBlock,
    
    /// Invalid signature
    #[error("Invalid signature")]
    InvalidSignature,
    
    /// Node is not the leader
    #[error("Not leader")]
    NotLeader,
    
    /// Already running
    #[error("Already running")]
    AlreadyRunning,
    
    /// Invalid timestamp
    #[error("Invalid timestamp")]
    InvalidTimestamp,
    
    /// Insufficient votes for quorum
    #[error("Insufficient votes: required {required}, received {received}")]
    InsufficientVotes { required: usize, received: usize },
    
    /// Empty quorum certificate
    #[error("Empty quorum certificate")]
    EmptyQuorumCertificate,
    
    /// Inconsistent votes in QC
    #[error("Inconsistent votes")]
    InconsistentVotes,
    
    /// Duplicate voter in QC
    #[error("Duplicate voter: {0}")]
    DuplicateVoter(crate::NodeId),
}

impl From<serde_json::Error> for ConsensusError {
    fn from(err: serde_json::Error) -> Self {
        ConsensusError::SerializationError(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for ConsensusError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        ConsensusError::Timeout(err.to_string())
    }
}