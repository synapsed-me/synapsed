//! # Synapsed Semantic Spacetime
//! 
//! Core semantic layer for the living codebase architecture based on
//! Mark Burgess's Semantic Spacetime theory and Promise Theory.
//!
//! This crate provides the foundational traits and types for treating
//! code modules as autonomous agents in semantic space that tell stories
//! through voluntary cooperation.

pub mod coordinates;
pub mod traits;
pub mod relations;
pub mod story;
pub mod navigation;
pub mod trust;
pub mod chemistry;

pub use coordinates::{SemanticCoords, SemanticPosition, SemanticDistance};
pub use traits::{SemanticAgent, StoryTeller, VoluntaryAgent, NarrativeParticipant};
pub use relations::{SemanticRelation, RelationType, SemanticLink};
pub use story::{Story, StoryPath, StoryFragment, Narrative, StoryOutcome, StoryEvent, StoryContext, TrustDelta};
pub use navigation::{SemanticNavigator, AgentNode, PathMetrics};
pub use trust::{TrustScore, TrustNetwork, TrustRelationship, TrustCategory, TrustDecision};
pub use chemistry::{PromiseChemistry, AffinityBond, CollaborationSuggestion};

use thiserror::Error;

/// Semantic spacetime specific errors
#[derive(Debug, Error)]
pub enum SemanticError {
    #[error("Semantic distance calculation failed: {0}")]
    DistanceCalculation(String),
    
    #[error("Story path not found from {from} to {to}")]
    NoStoryPath { from: String, to: String },
    
    #[error("Trust violation: {0}")]
    TrustViolation(String),
    
    #[error("Semantic drift detected: {0}")]
    SemanticDrift(String),
    
    #[error("Navigation failed: {0}")]
    NavigationFailed(String),
    
    #[error("Story recording failed: {0}")]
    StoryRecordingFailed(String),
    
    #[error("Promise chemistry unstable: {0}")]
    UnstableChemistry(String),
}

/// Result type for semantic operations
pub type SemanticResult<T> = Result<T, SemanticError>;

/// Core configuration for semantic spacetime
#[derive(Debug, Clone)]
pub struct SemanticConfig {
    /// Maximum semantic distance for voluntary cooperation
    pub max_cooperation_distance: f64,
    
    /// Trust threshold for promise acceptance
    pub trust_threshold: f64,
    
    /// Story retention period
    pub story_retention_days: u32,
    
    /// Enable automatic trust updates
    pub auto_trust_updates: bool,
    
    /// Semantic drift tolerance
    pub drift_tolerance: f64,
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            max_cooperation_distance: 0.7,
            trust_threshold: 0.5,
            story_retention_days: 90,
            auto_trust_updates: true,
            drift_tolerance: 0.1,
        }
    }
}