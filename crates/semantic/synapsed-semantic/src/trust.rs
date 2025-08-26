//! Trust management in semantic spacetime

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Trust score between agents (0-1)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrustScore(f64);

impl TrustScore {
    /// Create a new trust score
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }
    
    /// Get the raw value
    pub fn value(&self) -> f64 {
        self.0
    }
    
    /// Check if trust is above threshold
    pub fn is_trusted(&self, threshold: f64) -> bool {
        self.0 >= threshold
    }
    
    /// Increase trust
    pub fn increase(&mut self, amount: f64) {
        self.0 = (self.0 + amount).min(1.0);
    }
    
    /// Decrease trust
    pub fn decrease(&mut self, amount: f64) {
        self.0 = (self.0 - amount).max(0.0);
    }
    
    /// Apply exponential decay
    pub fn decay(&mut self, rate: f64) {
        self.0 *= (1.0 - rate);
    }
}

impl Default for TrustScore {
    fn default() -> Self {
        Self(0.5) // Start with neutral trust
    }
}

/// Trust relationship between two agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustRelationship {
    /// The trustor (who trusts)
    pub from: Uuid,
    
    /// The trustee (who is trusted)
    pub to: Uuid,
    
    /// Current trust score
    pub score: TrustScore,
    
    /// Number of interactions
    pub interaction_count: u64,
    
    /// Success rate
    pub success_rate: f64,
    
    /// Last interaction time
    pub last_interaction: DateTime<Utc>,
    
    /// Trust category
    pub category: TrustCategory,
}

impl TrustRelationship {
    /// Create a new trust relationship
    pub fn new(from: Uuid, to: Uuid) -> Self {
        Self {
            from,
            to,
            score: TrustScore::default(),
            interaction_count: 0,
            success_rate: 0.0,
            last_interaction: Utc::now(),
            category: TrustCategory::Neutral,
        }
    }
    
    /// Record an interaction
    pub fn record_interaction(&mut self, success: bool) {
        self.interaction_count += 1;
        self.last_interaction = Utc::now();
        
        // Update success rate with exponential moving average
        let alpha = 0.1;
        let success_value = if success { 1.0 } else { 0.0 };
        self.success_rate = alpha * success_value + (1.0 - alpha) * self.success_rate;
        
        // Update trust score
        if success {
            self.score.increase(0.05);
        } else {
            self.score.decrease(0.1); // Failures hurt more
        }
        
        // Update category
        self.update_category();
    }
    
    /// Update trust category based on score
    fn update_category(&mut self) {
        self.category = match self.score.value() {
            x if x >= 0.8 => TrustCategory::High,
            x if x >= 0.6 => TrustCategory::Good,
            x if x >= 0.4 => TrustCategory::Neutral,
            x if x >= 0.2 => TrustCategory::Low,
            _ => TrustCategory::Untrusted,
        };
    }
    
    /// Calculate confidence in the trust score
    pub fn confidence(&self) -> f64 {
        // Confidence increases with more interactions
        (self.interaction_count as f64 / (self.interaction_count as f64 + 10.0)).min(0.95)
    }
}

/// Categories of trust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustCategory {
    /// Highly trusted (0.8-1.0)
    High,
    /// Good trust (0.6-0.8)
    Good,
    /// Neutral trust (0.4-0.6)
    Neutral,
    /// Low trust (0.2-0.4)
    Low,
    /// Untrusted (0.0-0.2)
    Untrusted,
}

impl TrustCategory {
    /// Get minimum score for this category
    pub fn min_score(&self) -> f64 {
        match self {
            Self::High => 0.8,
            Self::Good => 0.6,
            Self::Neutral => 0.4,
            Self::Low => 0.2,
            Self::Untrusted => 0.0,
        }
    }
}

/// Trust network managing all trust relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustNetwork {
    /// All trust relationships
    relationships: HashMap<(Uuid, Uuid), TrustRelationship>,
    
    /// Global trust scores (reputation)
    reputation: HashMap<Uuid, f64>,
    
    /// Trust decay rate
    decay_rate: f64,
}

impl TrustNetwork {
    /// Create a new trust network
    pub fn new() -> Self {
        Self {
            relationships: HashMap::new(),
            reputation: HashMap::new(),
            decay_rate: 0.01,
        }
    }
    
    /// Get or create trust relationship
    pub fn get_or_create(&mut self, from: Uuid, to: Uuid) -> &mut TrustRelationship {
        self.relationships
            .entry((from, to))
            .or_insert_with(|| TrustRelationship::new(from, to))
    }
    
    /// Get trust score between agents
    pub fn get_trust(&self, from: Uuid, to: Uuid) -> TrustScore {
        self.relationships
            .get(&(from, to))
            .map(|r| r.score)
            .unwrap_or_default()
    }
    
    /// Record an interaction between agents
    pub fn record_interaction(&mut self, from: Uuid, to: Uuid, success: bool) {
        let relationship = self.get_or_create(from, to);
        relationship.record_interaction(success);
        
        // Update reputation
        self.update_reputation(to, success);
    }
    
    /// Update global reputation
    fn update_reputation(&mut self, agent: Uuid, success: bool) {
        let current = self.reputation.get(&agent).copied().unwrap_or(0.5);
        let delta = if success { 0.01 } else { -0.02 };
        let new_reputation = (current + delta).clamp(0.0, 1.0);
        self.reputation.insert(agent, new_reputation);
    }
    
    /// Get global reputation
    pub fn get_reputation(&self, agent: Uuid) -> f64 {
        self.reputation.get(&agent).copied().unwrap_or(0.5)
    }
    
    /// Apply time-based decay to all relationships
    pub fn apply_decay(&mut self) {
        for relationship in self.relationships.values_mut() {
            relationship.score.decay(self.decay_rate);
            relationship.update_category();
        }
    }
    
    /// Find most trusted agents
    pub fn most_trusted(&self, limit: usize) -> Vec<(Uuid, f64)> {
        let mut scores: Vec<(Uuid, f64)> = self.reputation.iter()
            .map(|(&id, &score)| (id, score))
            .collect();
        
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores.truncate(limit);
        scores
    }
    
    /// Find agents trusted by a specific agent
    pub fn trusted_by(&self, agent: Uuid, threshold: f64) -> Vec<Uuid> {
        self.relationships
            .iter()
            .filter(|((from, _), rel)| {
                *from == agent && rel.score.is_trusted(threshold)
            })
            .map(|((_, to), _)| *to)
            .collect()
    }
    
    /// Calculate transitive trust
    pub fn transitive_trust(&self, from: Uuid, to: Uuid, via: Uuid) -> TrustScore {
        let trust1 = self.get_trust(from, via);
        let trust2 = self.get_trust(via, to);
        
        // Multiply trust scores for transitive trust
        TrustScore::new(trust1.value() * trust2.value() * 0.9) // 0.9 discount factor
    }
    
    /// Find trust path between agents
    pub fn find_trust_path(&self, from: Uuid, to: Uuid, min_trust: f64) -> Option<Vec<Uuid>> {
        // Simple BFS to find a path with minimum trust
        use std::collections::VecDeque;
        
        let mut visited = std::collections::HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent_map = HashMap::new();
        
        queue.push_back(from);
        visited.insert(from);
        
        while let Some(current) = queue.pop_front() {
            if current == to {
                // Reconstruct path
                let mut path = vec![to];
                let mut node = to;
                
                while let Some(&parent) = parent_map.get(&node) {
                    path.push(parent);
                    node = parent;
                }
                
                path.reverse();
                return Some(path);
            }
            
            // Find neighbors with sufficient trust
            for ((f, t), rel) in &self.relationships {
                if *f == current && !visited.contains(t) && rel.score.value() >= min_trust {
                    visited.insert(*t);
                    parent_map.insert(*t, current);
                    queue.push_back(*t);
                }
            }
        }
        
        None
    }
}

impl Default for TrustNetwork {
    fn default() -> Self {
        Self::new()
    }
}

/// Trust-based decision making
pub struct TrustDecision {
    /// Minimum trust required
    pub min_trust: f64,
    
    /// Minimum confidence required
    pub min_confidence: f64,
    
    /// Whether to allow transitive trust
    pub allow_transitive: bool,
    
    /// Maximum transitive hops
    pub max_hops: usize,
}

impl TrustDecision {
    /// Create a strict trust decision
    pub fn strict() -> Self {
        Self {
            min_trust: 0.8,
            min_confidence: 0.7,
            allow_transitive: false,
            max_hops: 0,
        }
    }
    
    /// Create a moderate trust decision
    pub fn moderate() -> Self {
        Self {
            min_trust: 0.6,
            min_confidence: 0.5,
            allow_transitive: true,
            max_hops: 1,
        }
    }
    
    /// Create a lenient trust decision
    pub fn lenient() -> Self {
        Self {
            min_trust: 0.4,
            min_confidence: 0.3,
            allow_transitive: true,
            max_hops: 2,
        }
    }
    
    /// Evaluate if an agent should be trusted
    pub fn should_trust(
        &self,
        network: &TrustNetwork,
        from: Uuid,
        to: Uuid,
    ) -> bool {
        // Check direct trust
        if let Some(rel) = network.relationships.get(&(from, to)) {
            if rel.score.value() >= self.min_trust 
                && rel.confidence() >= self.min_confidence {
                return true;
            }
        }
        
        // Check transitive trust if allowed
        if self.allow_transitive && self.max_hops > 0 {
            // Try to find a trust path
            if let Some(path) = network.find_trust_path(from, to, self.min_trust) {
                if path.len() <= self.max_hops + 2 { // +2 for source and target
                    return true;
                }
            }
        }
        
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_trust_score() {
        let mut trust = TrustScore::new(0.5);
        trust.increase(0.3);
        assert_eq!(trust.value(), 0.8);
        
        trust.decrease(0.5);
        assert_eq!(trust.value(), 0.3);
        
        trust.decay(0.1);
        assert!((trust.value() - 0.27).abs() < 0.001);
    }
    
    #[test]
    fn test_trust_relationship() {
        let mut rel = TrustRelationship::new(Uuid::new_v4(), Uuid::new_v4());
        
        rel.record_interaction(true);
        rel.record_interaction(true);
        rel.record_interaction(false);
        
        assert!(rel.score.value() > 0.5);
        assert_eq!(rel.interaction_count, 3);
    }
    
    #[test]
    fn test_trust_network() {
        let mut network = TrustNetwork::new();
        let agent1 = Uuid::new_v4();
        let agent2 = Uuid::new_v4();
        
        network.record_interaction(agent1, agent2, true);
        network.record_interaction(agent1, agent2, true);
        
        let trust = network.get_trust(agent1, agent2);
        assert!(trust.value() > 0.5);
        
        let rep = network.get_reputation(agent2);
        assert!(rep > 0.5);
    }
}