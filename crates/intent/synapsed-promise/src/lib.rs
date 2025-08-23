//! # Synapsed Promise
//! 
//! Promise Theory implementation for autonomous agent cooperation and verification.
//! Based on Mark Burgess's Promise Theory, providing voluntary cooperation between
//! autonomous agents that cannot be coerced.
//!
//! ## Key Concepts
//! 
//! - **Autonomy**: Agents are causally independent and self-determined
//! - **Promises**: Voluntary declarations of intent by agents about their own behavior
//! - **Impositions**: External expectations that must be accepted to become promises
//! - **Trust**: Reputation-based trust model for agent cooperation
//! - **Cooperation**: Voluntary coordination without command-and-control

pub mod agent;
pub mod promise;
pub mod trust;
pub mod cooperation;
pub mod verification;
pub mod types;
pub mod voluntary;
pub mod acl;

pub use agent::{AutonomousAgent, AgentCapabilities, AgentState};
pub use promise::{Promise, PromiseContract, PromiseState, PromiseOutcome};
pub use trust::{TrustModel, TrustLevel, Reputation};
pub use cooperation::{CooperationProtocol, CooperationRequest, CooperationResponse};
pub use verification::{PromiseVerifier, VerificationProof};
pub use types::*;
pub use voluntary::{
    Willingness, VoluntaryCooperationEvaluator, SemanticSpacetime,
    CausalIndependenceVerifier, PromiseChemistry
};
pub use acl::{
    Performative, ACLMessage, ACLMessageBuilder, InteractionProtocol,
    ConversationManager, ConversationState, MessageContent
};

/// Result type for promise operations
pub type Result<T> = std::result::Result<T, PromiseError>;

/// Promise-specific errors
#[derive(Debug, thiserror::Error)]
pub enum PromiseError {
    #[error("Promise validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Promise execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Trust violation: {0}")]
    TrustViolation(String),
    
    #[error("Agent not found: {0}")]
    AgentNotFound(uuid::Uuid),
    
    #[error("Promise conflict: {0}")]
    Conflict(String),
    
    #[error("Cooperation failed: {0}")]
    CooperationFailed(String),
    
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    
    #[error("Autonomy violation: {0}")]
    AutonomyViolation(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

impl From<PromiseError> for synapsed_core::SynapsedError {
    fn from(err: PromiseError) -> Self {
        match err {
            PromiseError::ValidationFailed(msg) => synapsed_core::SynapsedError::InvalidInput(msg),
            PromiseError::ExecutionFailed(msg) => synapsed_core::SynapsedError::Internal(msg),
            PromiseError::Network(msg) => synapsed_core::SynapsedError::Network(msg),
            _ => synapsed_core::SynapsedError::Internal(err.to_string()),
        }
    }
}