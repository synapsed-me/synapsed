//! # Synapsed Intent
//! 
//! Hierarchical intent trees with verification for AI agent systems.
//! Implements HTN (Hierarchical Task Network) planning with observable execution.

pub mod intent;
pub mod tree;
pub mod context;
pub mod checkpoint;
pub mod types;

pub use intent::{HierarchicalIntent, IntentBuilder};
pub use tree::{IntentTree, IntentForest, IntentRelation};
pub use context::{IntentContext, ContextBuilder};
pub use checkpoint::{IntentCheckpoint, CheckpointManager};
pub use types::*;

// Re-export commonly used types
pub use crate::types::Step;

/// Result type for intent operations
pub type Result<T> = std::result::Result<T, IntentError>;

/// Intent-specific errors
#[derive(Debug, thiserror::Error)]
pub enum IntentError {
    #[error("Intent validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Intent execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Intent not found: {0}")]
    NotFound(uuid::Uuid),
    
    #[error("Context violation: {0}")]
    ContextViolation(String),
    
    #[error("Dependency failed: {0}")]
    DependencyFailed(String),
    
    #[error("Observable error: {0}")]
    ObservableError(String),
    
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}