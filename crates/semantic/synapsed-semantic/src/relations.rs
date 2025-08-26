//! Semantic relations based on Burgess's Semantic Spacetime theory

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The four fundamental relation types in Semantic Spacetime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationType {
    /// SIMILARITY - agents with similar capabilities or properties
    /// (Burgess's "Near" relation - spatial proximity in semantic space)
    Similarity,
    
    /// SEQUENCE - temporal/causal ordering between agents
    /// (Burgess's "LeadsTo" relation - process flow)
    Sequence,
    
    /// CONTAINMENT - hierarchical composition
    /// (Burgess's "Contains" relation - part-whole relationships)
    Containment,
    
    /// EXPRESSION - manifestation of intent or meaning
    /// (Burgess's "Express" relation - semantic interpretation)
    Expression,
}

impl RelationType {
    /// Get the inverse relation type
    pub fn inverse(&self) -> Self {
        match self {
            Self::Similarity => Self::Similarity, // Symmetric
            Self::Sequence => Self::Sequence,     // Asymmetric but same type
            Self::Containment => Self::Containment, // Inverse is "contained by"
            Self::Expression => Self::Expression,   // Inverse is "expressed by"
        }
    }
    
    /// Check if this relation type is symmetric
    pub fn is_symmetric(&self) -> bool {
        matches!(self, Self::Similarity)
    }
    
    /// Get semantic weight for this relation type
    pub fn semantic_weight(&self) -> f64 {
        match self {
            Self::Similarity => 0.7,   // Moderate weight
            Self::Sequence => 0.9,      // High weight - critical for flow
            Self::Containment => 0.8,   // High weight - structural
            Self::Expression => 1.0,    // Highest weight - meaning
        }
    }
}

/// A semantic relation between two agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticRelation {
    /// Type of relation
    pub relation_type: RelationType,
    
    /// Specific relation name (e.g., "then", "contains", "implements")
    pub name: String,
    
    /// Short form of the relation name
    pub short_name: String,
    
    /// Inverse relation name (e.g., "preceded by" for "then")
    pub inverse_name: String,
    
    /// Strength of the relation (0-1)
    pub strength: f64,
    
    /// Context in which this relation applies
    pub context: Vec<String>,
    
    /// Whether this relation is transitive
    pub is_transitive: bool,
}

impl SemanticRelation {
    /// Create a new similarity relation
    pub fn similarity(name: impl Into<String>, strength: f64) -> Self {
        let name = name.into();
        let short_name = Self::abbreviate(&name);
        
        Self {
            relation_type: RelationType::Similarity,
            name: name.clone(),
            short_name,
            inverse_name: name, // Symmetric
            strength: strength.clamp(0.0, 1.0),
            context: Vec::new(),
            is_transitive: false,
        }
    }
    
    /// Create a new sequence relation
    pub fn sequence(name: impl Into<String>, inverse: impl Into<String>, strength: f64) -> Self {
        let name = name.into();
        let inverse_name = inverse.into();
        
        Self {
            relation_type: RelationType::Sequence,
            name: name.clone(),
            short_name: Self::abbreviate(&name),
            inverse_name,
            strength: strength.clamp(0.0, 1.0),
            context: Vec::new(),
            is_transitive: true,
        }
    }
    
    /// Create a new containment relation
    pub fn containment(name: impl Into<String>, inverse: impl Into<String>, strength: f64) -> Self {
        let name = name.into();
        let inverse_name = inverse.into();
        
        Self {
            relation_type: RelationType::Containment,
            name: name.clone(),
            short_name: Self::abbreviate(&name),
            inverse_name,
            strength: strength.clamp(0.0, 1.0),
            context: Vec::new(),
            is_transitive: true,
        }
    }
    
    /// Create a new expression relation
    pub fn expression(name: impl Into<String>, inverse: impl Into<String>, strength: f64) -> Self {
        let name = name.into();
        let inverse_name = inverse.into();
        
        Self {
            relation_type: RelationType::Expression,
            name: name.clone(),
            short_name: Self::abbreviate(&name),
            inverse_name,
            strength: strength.clamp(0.0, 1.0),
            context: Vec::new(),
            is_transitive: false,
        }
    }
    
    /// Add context to this relation
    pub fn with_context(mut self, context: Vec<String>) -> Self {
        self.context = context;
        self
    }
    
    /// Create an inverse of this relation
    pub fn inverse(&self) -> Self {
        Self {
            relation_type: self.relation_type,
            name: self.inverse_name.clone(),
            short_name: Self::abbreviate(&self.inverse_name),
            inverse_name: self.name.clone(),
            strength: self.strength,
            context: self.context.clone(),
            is_transitive: self.is_transitive,
        }
    }
    
    /// Abbreviate a relation name for compact representation
    fn abbreviate(name: &str) -> String {
        // Take first letter of each word, or first 3 chars if single word
        let words: Vec<&str> = name.split_whitespace().collect();
        if words.len() > 1 {
            words.iter()
                .map(|w| w.chars().next().unwrap_or('_'))
                .collect()
        } else {
            name.chars().take(3).collect()
        }
    }
}

/// A semantic link between two agents (directed edge in the graph)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticLink {
    /// Source agent ID
    pub from: Uuid,
    
    /// Target agent ID
    pub to: Uuid,
    
    /// The relation between them
    pub relation: SemanticRelation,
    
    /// Weight of this specific link
    pub weight: f64,
    
    /// Timestamp when this link was established
    pub established: chrono::DateTime<chrono::Utc>,
    
    /// Number of times this link has been traversed
    pub traversal_count: u64,
    
    /// Success rate of traversals (0-1)
    pub success_rate: f64,
}

impl SemanticLink {
    /// Create a new semantic link
    pub fn new(from: Uuid, to: Uuid, relation: SemanticRelation) -> Self {
        Self {
            from,
            to,
            relation,
            weight: 1.0,
            established: chrono::Utc::now(),
            traversal_count: 0,
            success_rate: 1.0,
        }
    }
    
    /// Record a traversal of this link
    pub fn record_traversal(&mut self, success: bool) {
        self.traversal_count += 1;
        
        // Update success rate with exponential moving average
        let alpha = 0.1; // Learning rate
        let success_value = if success { 1.0 } else { 0.0 };
        self.success_rate = alpha * success_value + (1.0 - alpha) * self.success_rate;
        
        // Update weight based on success rate and traversal count
        self.update_weight();
    }
    
    /// Update the weight based on success rate and usage
    fn update_weight(&mut self) {
        // Weight increases with success rate and usage
        let usage_factor = (self.traversal_count as f64).ln().max(1.0) / 10.0;
        self.weight = (self.success_rate * (1.0 + usage_factor)).min(2.0);
    }
    
    /// Get the effective strength of this link
    pub fn effective_strength(&self) -> f64 {
        self.relation.strength * self.weight * self.success_rate
    }
    
    /// Create an inverse link
    pub fn inverse(&self) -> Self {
        Self {
            from: self.to,
            to: self.from,
            relation: self.relation.inverse(),
            weight: self.weight,
            established: self.established,
            traversal_count: self.traversal_count,
            success_rate: self.success_rate,
        }
    }
}

/// Common semantic relations for code modules
pub mod common {
    use super::SemanticRelation;
    
    /// Create a "then" sequence relation
    pub fn then() -> SemanticRelation {
        SemanticRelation::sequence("then", "preceded by", 0.9)
    }
    
    /// Create a "calls" sequence relation
    pub fn calls() -> SemanticRelation {
        SemanticRelation::sequence("calls", "called by", 0.8)
    }
    
    /// Create a "depends on" sequence relation
    pub fn depends_on() -> SemanticRelation {
        SemanticRelation::sequence("depends on", "required by", 0.85)
    }
    
    /// Create a "contains" containment relation
    pub fn contains() -> SemanticRelation {
        SemanticRelation::containment("contains", "contained in", 0.9)
    }
    
    /// Create an "implements" expression relation
    pub fn implements() -> SemanticRelation {
        SemanticRelation::expression("implements", "implemented by", 0.95)
    }
    
    /// Create a "similar to" similarity relation
    pub fn similar_to() -> SemanticRelation {
        SemanticRelation::similarity("similar to", 0.7)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_relation_types() {
        assert!(RelationType::Similarity.is_symmetric());
        assert!(!RelationType::Sequence.is_symmetric());
    }
    
    #[test]
    fn test_semantic_link_traversal() {
        let mut link = SemanticLink::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            common::then(),
        );
        
        link.record_traversal(true);
        link.record_traversal(true);
        link.record_traversal(false);
        
        assert_eq!(link.traversal_count, 3);
        assert!(link.success_rate < 1.0);
        assert!(link.success_rate > 0.5);
    }
}