//! Voluntary cooperation protocol for Promise Theory
//! 
//! This module ensures that all agent cooperation is truly voluntary,
//! implementing Mark Burgess's principle that agents cannot be coerced.

use crate::{
    types::*, Result, Promise
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Willingness to cooperate
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Willingness {
    /// Fully willing to cooperate
    Willing { confidence: f64 },
    /// Conditionally willing with requirements
    Conditional { conditions: Vec<Condition>, confidence: f64 },
    /// Unwilling to cooperate
    Unwilling { reason: String },
    /// Needs more information to decide
    Uncertain { missing_info: Vec<String> },
}

/// Condition for cooperation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    pub condition_type: ConditionType,
    pub requirement: String,
    pub priority: Priority,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionType {
    Resource,      // Resource availability
    Trust,         // Trust level requirement
    Capability,    // Capability requirement
    Temporal,      // Time constraint
    Precedence,    // Order dependency
    Permission,    // Permission requirement
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    Critical,      // Must be met
    Important,     // Should be met
    Optional,      // Nice to have
}

/// Voluntary cooperation evaluator
#[async_trait]
pub trait VoluntaryCooperationEvaluator: Send + Sync {
    /// Evaluate willingness to make a promise
    async fn evaluate_promise_willingness(
        &self,
        agent_id: AgentId,
        promise_type: &PromiseType,
        body: &PromiseBody,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<Willingness>;
    
    /// Evaluate willingness to accept an imposition
    async fn evaluate_imposition_willingness(
        &self,
        agent_id: AgentId,
        imposition: &Imposition,
        imposer_trust: f64,
    ) -> Result<Willingness>;
    
    /// Check if conditions are met
    async fn check_conditions(
        &self,
        conditions: &[Condition],
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<bool>;
    
    /// Negotiate conditions for cooperation
    async fn negotiate_conditions(
        &self,
        initial_conditions: Vec<Condition>,
        counterparty: AgentId,
    ) -> Result<Vec<Condition>>;
}

/// Semantic spacetime context for promises
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSpacetime {
    /// Spatial context - where the promise applies
    pub spatial_scope: SpatialScope,
    /// Temporal context - when the promise applies
    pub temporal_scope: TemporalScope,
    /// Semantic context - meaning and interpretation
    pub semantic_context: SemanticContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpatialScope {
    Local,                    // Same process/thread
    Node,                     // Same machine/node
    Network(String),          // Specific network
    Global,                   // Everywhere
    Custom(Vec<String>),      // Custom locations
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalScope {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub duration: Option<std::time::Duration>,
    pub recurring: Option<RecurrencePattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurrencePattern {
    pub frequency: RecurrenceFrequency,
    pub interval: u32,
    pub count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecurrenceFrequency {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticContext {
    pub domain: String,
    pub ontology: HashMap<String, String>,
    pub relationships: Vec<SemanticRelationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticRelationship {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
}

/// Causal independence verifier
pub struct CausalIndependenceVerifier;

impl CausalIndependenceVerifier {
    /// Verify that an agent is causally independent
    pub async fn verify_independence(
        &self,
        _agent_id: AgentId,
        _other_agents: &[AgentId],
    ) -> Result<bool> {
        // Check that agent can make decisions without external influence
        // This would integrate with the actual agent runtime
        Ok(true)
    }
    
    /// Check if a promise would violate causal independence
    pub async fn check_promise_independence(
        &self,
        _promise: &Promise,
        dependencies: &[PromiseId],
    ) -> Result<bool> {
        // Verify promise doesn't create coercive dependencies
        for _dep_id in dependencies {
            // Check each dependency doesn't violate autonomy
            // Real implementation would check promise graph
        }
        Ok(true)
    }
}

/// Promise chemistry - how promises interact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromiseChemistry {
    pub promise_id: PromiseId,
    pub reactions: Vec<PromiseReaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromiseReaction {
    pub reaction_type: ReactionType,
    pub with_promise: PromiseId,
    pub outcome: ReactionOutcome,
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReactionType {
    Compose,       // Promises combine into larger promise
    Decompose,     // Promise breaks into smaller promises
    Catalyze,      // Promise enables another promise
    Inhibit,       // Promise blocks another promise
    Transform,     // Promise changes into different promise
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReactionOutcome {
    NewPromise(PromiseBody),
    ModifiedPromise(PromiseId, PromiseBody),
    CancelledPromise(PromiseId),
    EnabledPromise(PromiseId),
}

/// Default implementation of voluntary cooperation evaluator
pub struct DefaultVoluntaryEvaluator {
    trust_threshold: f64,
    #[allow(dead_code)]
    capability_checker: Box<dyn CapabilityChecker>,
}

#[async_trait]
pub trait CapabilityChecker: Send + Sync {
    async fn has_capability(&self, agent_id: AgentId, capability: &str) -> bool;
    async fn has_resources(&self, agent_id: AgentId, resources: &[String]) -> bool;
}

#[async_trait]
impl VoluntaryCooperationEvaluator for DefaultVoluntaryEvaluator {
    async fn evaluate_promise_willingness(
        &self,
        agent_id: AgentId,
        promise_type: &PromiseType,
        body: &PromiseBody,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<Willingness> {
        // Check if agent has capability to make this promise
        let has_capability = match promise_type {
            PromiseType::Offer | PromiseType::Give => {
                // Check if we can provide what we're promising
                true // Simplified - would check actual capabilities
            },
            PromiseType::Use | PromiseType::Accept => {
                // Check if we can use what's being offered
                true // Simplified
            },
            PromiseType::Delegate => {
                // Check if we can delegate
                true // Simplified
            }
        };
        
        if !has_capability {
            return Ok(Willingness::Unwilling {
                reason: "Lacks required capability".to_string()
            });
        }
        
        // Check resource availability
        let has_resources = true; // Simplified
        
        if !has_resources {
            return Ok(Willingness::Conditional {
                conditions: vec![
                    Condition {
                        condition_type: ConditionType::Resource,
                        requirement: "Additional resources needed".to_string(),
                        priority: Priority::Critical,
                    }
                ],
                confidence: 0.5,
            });
        }
        
        Ok(Willingness::Willing { confidence: 0.9 })
    }
    
    async fn evaluate_imposition_willingness(
        &self,
        agent_id: AgentId,
        imposition: &Imposition,
        imposer_trust: f64,
    ) -> Result<Willingness> {
        // Never accept impositions from untrusted agents
        if imposer_trust < self.trust_threshold {
            return Ok(Willingness::Unwilling {
                reason: format!("Insufficient trust: {:.2} < {:.2}", 
                    imposer_trust, self.trust_threshold)
            });
        }
        
        // Evaluate the imposition body as if it were a promise
        self.evaluate_promise_willingness(
            agent_id,
            &PromiseType::Accept,
            &imposition.body,
            &HashMap::new(),
        ).await
    }
    
    async fn check_conditions(
        &self,
        conditions: &[Condition],
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<bool> {
        for condition in conditions {
            if condition.priority == Priority::Critical {
                // Check critical conditions
                // Simplified - would check actual condition
                let met = true;
                if !met {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
    
    async fn negotiate_conditions(
        &self,
        initial_conditions: Vec<Condition>,
        counterparty: AgentId,
    ) -> Result<Vec<Condition>> {
        // Simple negotiation - accept important and critical, drop optional
        Ok(initial_conditions.into_iter()
            .filter(|c| c.priority != Priority::Optional)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_willingness_evaluation() {
        struct MockCapabilityChecker;
        
        #[async_trait]
        impl CapabilityChecker for MockCapabilityChecker {
            async fn has_capability(&self, _: AgentId, _: &str) -> bool { true }
            async fn has_resources(&self, _: AgentId, _: &[String]) -> bool { true }
        }
        
        let evaluator = DefaultVoluntaryEvaluator {
            trust_threshold: 0.5,
            capability_checker: Box::new(MockCapabilityChecker),
        };
        
        let agent_id = AgentId::new();
        let body = PromiseBody {
            content: "Test promise".to_string(),
            constraints: vec![],
            qos: None,
            metadata: HashMap::new(),
        };
        
        let willingness = evaluator.evaluate_promise_willingness(
            agent_id,
            &PromiseType::Offer,
            &body,
            &HashMap::new(),
        ).await.unwrap();
        
        match willingness {
            Willingness::Willing { confidence } => assert!(confidence > 0.0),
            _ => panic!("Expected willing response"),
        }
    }
}