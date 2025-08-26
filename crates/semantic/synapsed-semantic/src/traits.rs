//! Core traits for semantic spacetime agents

use async_trait::async_trait;
use crate::{
    SemanticCoords, SemanticPosition, SemanticDistance,
    Story, StoryPath, Narrative,
    TrustScore, SemanticResult,
};
use synapsed_intent::Intent;
use synapsed_promise::{Promise, PromiseOutcome};
use synapsed_verify::VerificationResult;
use uuid::Uuid;
use std::collections::HashMap;

/// A semantic agent that exists in semantic spacetime
#[async_trait]
pub trait SemanticAgent: Send + Sync {
    /// Get the agent's unique identifier
    fn id(&self) -> Uuid;
    
    /// Get the agent's semantic coordinates in spacetime
    fn semantic_position(&self) -> SemanticCoords;
    
    /// Calculate affinity with an intent based on semantic distance
    fn story_affinity(&self, intent: &Intent) -> f64;
    
    /// Voluntarily promise to help fulfill an intent
    async fn voluntary_promise(&self, intent: &Intent) -> Option<Promise>;
    
    /// Update position based on story participation
    async fn update_position(&mut self, story: &Story) -> SemanticResult<()>;
    
    /// Get current trust relationships
    fn trust_relationships(&self) -> HashMap<Uuid, TrustScore>;
}

/// An agent that can tell and participate in stories
#[async_trait]
pub trait StoryTeller: SemanticAgent {
    /// Tell a story by executing an intent
    async fn tell_story(&self, intent: Intent) -> SemanticResult<Story>;
    
    /// Find stories similar to a goal
    async fn find_stories(&self, goal: &str) -> Vec<Story>;
    
    /// Suggest story paths for an intent
    async fn suggest_paths(&self, intent: &Intent) -> Vec<StoryPath>;
    
    /// Explain what this agent does through stories
    async fn explain_through_stories(&self) -> Narrative;
}

/// An agent that participates voluntarily (Promise Theory)
#[async_trait]
pub trait VoluntaryAgent: SemanticAgent {
    /// Evaluate whether to participate in a story
    async fn evaluate_participation(&self, intent: &Intent) -> SemanticResult<bool>;
    
    /// Negotiate terms of participation
    async fn negotiate_promise(&self, intent: &Intent) -> SemanticResult<Promise>;
    
    /// Execute promised actions
    async fn execute_promise(&self, promise: &Promise) -> SemanticResult<PromiseOutcome>;
    
    /// Verify promise fulfillment
    async fn verify_promise(&self, promise: &Promise, outcome: &PromiseOutcome) -> VerificationResult;
}

/// An agent that participates in narratives
#[async_trait]
pub trait NarrativeParticipant: StoryTeller + VoluntaryAgent {
    /// Join an ongoing narrative
    async fn join_narrative(&mut self, narrative: &Narrative) -> SemanticResult<()>;
    
    /// Contribute to a narrative
    async fn contribute(&self, narrative: &mut Narrative) -> SemanticResult<()>;
    
    /// Leave a narrative
    async fn leave_narrative(&mut self, narrative: &Narrative) -> SemanticResult<()>;
    
    /// Get current narrative context
    fn narrative_context(&self) -> Option<Narrative>;
}

/// Trait for modules that can be wrapped with semantic capabilities
pub trait SemanticWrappable {
    /// Wrap with semantic agent capabilities
    fn with_semantics(self) -> Box<dyn SemanticAgent>;
    
    /// Add story telling capabilities
    fn with_storytelling(self) -> Box<dyn StoryTeller>;
    
    /// Add voluntary cooperation
    fn with_voluntary_cooperation(self) -> Box<dyn VoluntaryAgent>;
    
    /// Full narrative participant
    fn as_narrative_participant(self) -> Box<dyn NarrativeParticipant>;
}