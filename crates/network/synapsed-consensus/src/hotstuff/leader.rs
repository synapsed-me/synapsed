//! Leader election implementations for HotStuff

use crate::{NodeId, ViewNumber};
use crate::traits::LeaderElection;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Round-robin leader election strategy
#[derive(Debug, Clone)]
pub struct RoundRobinLeaderElection {
    /// Seed for deterministic randomization
    seed: u64,
}

impl RoundRobinLeaderElection {
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    pub fn with_seed(seed: u64) -> Self {
        Self { seed }
    }
}

impl LeaderElection for RoundRobinLeaderElection {
    fn get_leader(&self, view: ViewNumber, validators: &[NodeId]) -> NodeId {
        if validators.is_empty() {
            panic!("No validators available for leader election");
        }

        let index = (view.as_u64() + self.seed) as usize % validators.len();
        validators[index].clone()
    }

    fn is_leader(&self, node: &NodeId, view: ViewNumber, validators: &[NodeId]) -> bool {
        let leader = self.get_leader(view, validators);
        leader == *node
    }
}

/// Hash-based leader election for better randomization
#[derive(Debug, Clone)]
pub struct HashBasedLeaderElection {
    /// Network identifier for consistent hashing
    network_id: String,
}

impl HashBasedLeaderElection {
    pub fn new(network_id: String) -> Self {
        Self { network_id }
    }
}

impl LeaderElection for HashBasedLeaderElection {
    fn get_leader(&self, view: ViewNumber, validators: &[NodeId]) -> NodeId {
        if validators.is_empty() {
            panic!("No validators available for leader election");
        }

        // Create deterministic hash from view and network ID
        let mut hasher = DefaultHasher::new();
        self.network_id.hash(&mut hasher);
        view.as_u64().hash(&mut hasher);
        let hash = hasher.finish();

        let index = hash as usize % validators.len();
        validators[index].clone()
    }

    fn is_leader(&self, node: &NodeId, view: ViewNumber, validators: &[NodeId]) -> bool {
        let leader = self.get_leader(view, validators);
        leader == *node
    }
}

/// Weighted leader election based on stake or reputation
#[derive(Debug, Clone)]
pub struct WeightedLeaderElection {
    /// Weights for each validator
    weights: std::collections::HashMap<NodeId, u64>,
    /// Total weight
    total_weight: u64,
}

impl WeightedLeaderElection {
    pub fn new(weights: std::collections::HashMap<NodeId, u64>) -> Self {
        let total_weight = weights.values().sum();
        Self {
            weights,
            total_weight,
        }
    }

    /// Update weights for validators
    pub fn update_weights(&mut self, weights: std::collections::HashMap<NodeId, u64>) {
        self.total_weight = weights.values().sum();
        self.weights = weights;
    }
}

impl LeaderElection for WeightedLeaderElection {
    fn get_leader(&self, view: ViewNumber, validators: &[NodeId]) -> NodeId {
        if validators.is_empty() || self.total_weight == 0 {
            panic!("No validators or weights available for leader election");
        }

        // Deterministic selection based on view number
        let target = (view.as_u64() * 7919) % self.total_weight; // 7919 is a prime
        let mut cumulative_weight = 0;

        for validator in validators {
            if let Some(&weight) = self.weights.get(validator) {
                cumulative_weight += weight;
                if cumulative_weight > target {
                    return validator.clone();
                }
            }
        }

        // Fallback to first validator
        validators[0].clone()
    }

    fn is_leader(&self, node: &NodeId, view: ViewNumber, validators: &[NodeId]) -> bool {
        let leader = self.get_leader(view, validators);
        leader == *node
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_validators() -> Vec<NodeId> {
        (0..4).map(|_| NodeId::new()).collect()
    }

    #[test]
    fn test_round_robin_leader_election() {
        let election = RoundRobinLeaderElection::new();
        let validators = create_test_validators();
        
        // Test that leader cycles through validators
        let leader1 = election.get_leader(ViewNumber::new(0), &validators);
        let leader2 = election.get_leader(ViewNumber::new(1), &validators);
        let leader3 = election.get_leader(ViewNumber::new(4), &validators); // Should wrap around
        
        assert_eq!(leader1, validators[0]);
        assert_eq!(leader2, validators[1]);
        assert_eq!(leader3, validators[0]);
    }

    #[test]
    fn test_hash_based_leader_election() {
        let election = HashBasedLeaderElection::new("test-network".to_string());
        let validators = create_test_validators();
        
        // Test determinism
        let leader1a = election.get_leader(ViewNumber::new(1), &validators);
        let leader1b = election.get_leader(ViewNumber::new(1), &validators);
        assert_eq!(leader1a, leader1b);
        
        // Test different views produce potentially different leaders
        let leader1 = election.get_leader(ViewNumber::new(1), &validators);
        let leader2 = election.get_leader(ViewNumber::new(2), &validators);
        // They might be the same, but the selection should be deterministic
        assert!(validators.contains(&leader1));
        assert!(validators.contains(&leader2));
    }

    #[test]
    fn test_weighted_leader_election() {
        let validators = create_test_validators();
        let mut weights = std::collections::HashMap::new();
        weights.insert(validators[0].clone(), 100);
        weights.insert(validators[1].clone(), 200);
        weights.insert(validators[2].clone(), 300);
        weights.insert(validators[3].clone(), 400);
        
        let election = WeightedLeaderElection::new(weights);
        
        // Higher weight nodes should be selected more often over many views
        let mut selection_count = std::collections::HashMap::new();
        for view in 0..1000 {
            let leader = election.get_leader(ViewNumber::new(view), &validators);
            *selection_count.entry(leader).or_insert(0) += 1;
        }
        
        // Validator with weight 400 should be selected most often
        let max_selections = selection_count.values().max().unwrap();
        let most_selected = selection_count.iter()
            .find(|(_, &count)| count == *max_selections)
            .unwrap().0;
        
        // The validator with highest weight should be selected most (with high probability)
        assert_eq!(*most_selected, validators[3]);
    }

    #[test]
    fn test_is_leader() {
        let election = RoundRobinLeaderElection::new();
        let validators = create_test_validators();
        
        assert!(election.is_leader(&validators[0], ViewNumber::new(0), &validators));
        assert!(!election.is_leader(&validators[1], ViewNumber::new(0), &validators));
        assert!(election.is_leader(&validators[1], ViewNumber::new(1), &validators));
    }
}
