//! Story representation for execution narratives in semantic spacetime

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use crate::{SemanticCoords, SemanticLink, TrustScore};
use synapsed_intent::Intent;
use synapsed_promise::{Promise, PromiseOutcome};
use synapsed_verify::VerificationResult;

/// A complete story from intent to verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Story {
    /// Unique story identifier
    pub id: Uuid,
    
    /// The beginning - what we wanted to achieve
    pub intent: Intent,
    
    /// The middle - promises made by participants
    pub promises: Vec<Promise>,
    
    /// The action - actual execution events
    pub execution: Vec<StoryEvent>,
    
    /// The end - verification of outcomes
    pub verification: StoryOutcome,
    
    /// Trust updates resulting from this story
    pub trust_updates: Vec<TrustDelta>,
    
    /// Semantic path taken through spacetime
    pub path: StoryPath,
    
    /// Context in which story occurred
    pub context: StoryContext,
    
    /// Timestamp when story began
    pub started_at: DateTime<Utc>,
    
    /// Timestamp when story ended
    pub ended_at: Option<DateTime<Utc>>,
    
    /// Metadata about the story
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Story {
    /// Create a new story from an intent
    pub fn begin(intent: Intent) -> Self {
        Self {
            id: Uuid::new_v4(),
            intent,
            promises: Vec::new(),
            execution: Vec::new(),
            verification: StoryOutcome::Pending,
            trust_updates: Vec::new(),
            path: StoryPath::new(),
            context: StoryContext::default(),
            started_at: Utc::now(),
            ended_at: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Add a promise to the story
    pub fn add_promise(&mut self, promise: Promise) {
        self.promises.push(promise);
    }
    
    /// Record an execution event
    pub fn record_event(&mut self, event: StoryEvent) {
        self.execution.push(event);
    }
    
    /// Complete the story with verification
    pub fn complete(&mut self, verification: VerificationResult) {
        self.verification = StoryOutcome::from_verification(verification);
        self.ended_at = Some(Utc::now());
        self.calculate_trust_updates();
    }
    
    /// Calculate trust updates based on promise fulfillment
    fn calculate_trust_updates(&mut self) {
        for promise in &self.promises {
            let fulfilled = self.verification.is_success();
            let delta = if fulfilled {
                TrustDelta::increase(promise.promisor_id(), 0.1)
            } else {
                TrustDelta::decrease(promise.promisor_id(), 0.2)
            };
            self.trust_updates.push(delta);
        }
    }
    
    /// Get story duration
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.ended_at.map(|end| end - self.started_at)
    }
    
    /// Check if story was successful
    pub fn is_successful(&self) -> bool {
        self.verification.is_success()
    }
    
    /// Get participating agent IDs
    pub fn participants(&self) -> Vec<Uuid> {
        self.promises.iter()
            .map(|p| p.promisor_id())
            .collect()
    }
}

/// An event that occurs during story execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryEvent {
    /// Event ID
    pub id: Uuid,
    
    /// Agent that caused this event
    pub agent_id: Uuid,
    
    /// Type of event
    pub event_type: StoryEventType,
    
    /// Event description
    pub description: String,
    
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Semantic position when event occurred
    pub position: SemanticCoords,
    
    /// Additional event data
    pub data: serde_json::Value,
}

/// Types of events that can occur in a story
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoryEventType {
    /// Intent was declared
    IntentDeclared,
    /// Promise was made
    PromiseMade,
    /// Promise was accepted
    PromiseAccepted,
    /// Promise was rejected
    PromiseRejected,
    /// Execution started
    ExecutionStarted,
    /// Module was invoked
    ModuleInvoked,
    /// Data was transformed
    DataTransformed,
    /// Error occurred
    ErrorOccurred,
    /// Execution completed
    ExecutionCompleted,
    /// Verification performed
    VerificationPerformed,
}

/// The outcome of a story
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoryOutcome {
    /// Story is still in progress
    Pending,
    /// Story completed successfully
    Success {
        verification: VerificationResult,
        confidence: f64,
    },
    /// Story failed
    Failure {
        reason: String,
        error: Option<String>,
    },
    /// Story was partially successful
    Partial {
        completed: Vec<String>,
        failed: Vec<String>,
        confidence: f64,
    },
}

impl StoryOutcome {
    /// Create from verification result
    pub fn from_verification(verification: VerificationResult) -> Self {
        if verification.is_verified {
            Self::Success {
                verification,
                confidence: verification.confidence,
            }
        } else {
            Self::Failure {
                reason: "Verification failed".to_string(),
                error: None,
            }
        }
    }
    
    /// Check if outcome is successful
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }
}

/// A path through semantic spacetime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryPath {
    /// Sequence of positions visited
    pub positions: Vec<SemanticCoords>,
    
    /// Links traversed between positions
    pub links: Vec<SemanticLink>,
    
    /// Total distance traveled
    pub total_distance: f64,
    
    /// Average trust along the path
    pub average_trust: f64,
}

impl StoryPath {
    /// Create a new empty path
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            links: Vec::new(),
            total_distance: 0.0,
            average_trust: 1.0,
        }
    }
    
    /// Add a position to the path
    pub fn add_position(&mut self, position: SemanticCoords) {
        if let Some(last) = self.positions.last() {
            self.total_distance += last.distance_to(&position);
        }
        self.positions.push(position);
    }
    
    /// Add a link to the path
    pub fn add_link(&mut self, link: SemanticLink) {
        self.links.push(link);
        self.recalculate_trust();
    }
    
    /// Recalculate average trust
    fn recalculate_trust(&mut self) {
        if self.links.is_empty() {
            self.average_trust = 1.0;
        } else {
            let sum: f64 = self.links.iter()
                .map(|l| l.success_rate)
                .sum();
            self.average_trust = sum / self.links.len() as f64;
        }
    }
    
    /// Get path length
    pub fn length(&self) -> usize {
        self.positions.len()
    }
}

impl Default for StoryPath {
    fn default() -> Self {
        Self::new()
    }
}

/// Context in which a story occurs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryContext {
    /// Chapter or section name
    pub chapter: String,
    
    /// Active themes
    pub themes: Vec<String>,
    
    /// Parent story ID if this is a sub-story
    pub parent_story: Option<Uuid>,
    
    /// Related stories
    pub related_stories: Vec<Uuid>,
    
    /// Environmental context
    pub environment: HashMap<String, String>,
}

impl Default for StoryContext {
    fn default() -> Self {
        Self {
            chapter: "default".to_string(),
            themes: Vec::new(),
            parent_story: None,
            related_stories: Vec::new(),
            environment: HashMap::new(),
        }
    }
}

/// A fragment of a story (can be composed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryFragment {
    /// Fragment ID
    pub id: Uuid,
    
    /// Fragment content
    pub content: String,
    
    /// Position in larger narrative
    pub sequence: usize,
    
    /// Semantic position
    pub position: SemanticCoords,
    
    /// Can connect to these fragments
    pub connects_to: Vec<Uuid>,
}

/// A collection of related stories forming a narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Narrative {
    /// Narrative ID
    pub id: Uuid,
    
    /// Narrative title
    pub title: String,
    
    /// Stories in this narrative
    pub stories: Vec<Story>,
    
    /// Overall theme
    pub theme: String,
    
    /// Narrative arc type
    pub arc: NarrativeArc,
    
    /// Key participants
    pub protagonists: Vec<Uuid>,
}

/// Types of narrative arcs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NarrativeArc {
    /// Linear progression
    Linear,
    /// Cyclical/repeating pattern
    Cyclical,
    /// Branching paths
    Branching,
    /// Converging paths
    Converging,
    /// Emergent/unplanned
    Emergent,
}

/// Trust changes resulting from a story
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustDelta {
    /// Agent whose trust is updated
    pub agent_id: Uuid,
    
    /// Change in trust score
    pub delta: f64,
    
    /// Reason for change
    pub reason: String,
    
    /// Timestamp of update
    pub timestamp: DateTime<Utc>,
}

impl TrustDelta {
    /// Create a trust increase
    pub fn increase(agent_id: Uuid, amount: f64) -> Self {
        Self {
            agent_id,
            delta: amount.abs(),
            reason: "Promise fulfilled".to_string(),
            timestamp: Utc::now(),
        }
    }
    
    /// Create a trust decrease
    pub fn decrease(agent_id: Uuid, amount: f64) -> Self {
        Self {
            agent_id,
            delta: -amount.abs(),
            reason: "Promise broken".to_string(),
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_story_creation() {
        let intent = Intent::new("Test intent");
        let story = Story::begin(intent);
        
        assert_eq!(story.promises.len(), 0);
        assert_eq!(story.execution.len(), 0);
        assert!(matches!(story.verification, StoryOutcome::Pending));
    }
    
    #[test]
    fn test_story_path() {
        let mut path = StoryPath::new();
        
        path.add_position(SemanticCoords::new(0.0, 0.0, 0.0, 0.0));
        path.add_position(SemanticCoords::new(1.0, 0.0, 0.0, 0.0));
        
        assert_eq!(path.length(), 2);
        assert!(path.total_distance > 0.0);
    }
}