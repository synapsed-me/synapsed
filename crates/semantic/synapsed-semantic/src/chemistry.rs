//! Promise chemistry - affinity between agents based on successful cooperation

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use crate::{SemanticCoords, TrustScore, SemanticDistance};

/// Affinity between agents (like chemical bonds)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromiseChemistry {
    /// Affinity scores between agent pairs
    affinity_map: HashMap<(Uuid, Uuid), AffinityBond>,
    
    /// Semantic positions of agents
    positions: HashMap<Uuid, SemanticCoords>,
    
    /// Distance metric for calculations
    distance_metric: SemanticDistance,
    
    /// Chemistry configuration
    config: ChemistryConfig,
}

impl PromiseChemistry {
    /// Create new promise chemistry
    pub fn new() -> Self {
        Self {
            affinity_map: HashMap::new(),
            positions: HashMap::new(),
            distance_metric: SemanticDistance::Euclidean,
            config: ChemistryConfig::default(),
        }
    }
    
    /// Register an agent with its semantic position
    pub fn register_agent(&mut self, agent_id: Uuid, position: SemanticCoords) {
        self.positions.insert(agent_id, position);
    }
    
    /// Update agent position
    pub fn update_position(&mut self, agent_id: Uuid, new_position: SemanticCoords) {
        self.positions.insert(agent_id, new_position);
        
        // Recalculate affinities involving this agent
        self.recalculate_affinities_for(agent_id);
    }
    
    /// Strengthen bond between agents (successful cooperation)
    pub fn strengthen_bond(&mut self, from: Uuid, to: Uuid, amount: f64) {
        let bond = self.affinity_map
            .entry((from, to))
            .or_insert_with(|| AffinityBond::new(from, to));
        
        bond.strengthen(amount);
        
        // Bonds are bidirectional but asymmetric
        let reverse_bond = self.affinity_map
            .entry((to, from))
            .or_insert_with(|| AffinityBond::new(to, from));
        
        reverse_bond.strengthen(amount * 0.8); // Slightly less in reverse
    }
    
    /// Weaken bond between agents (failed cooperation)
    pub fn weaken_bond(&mut self, from: Uuid, to: Uuid, amount: f64) {
        let bond = self.affinity_map
            .entry((from, to))
            .or_insert_with(|| AffinityBond::new(from, to));
        
        bond.weaken(amount);
        
        // Weaken reverse bond more
        let reverse_bond = self.affinity_map
            .entry((to, from))
            .or_insert_with(|| AffinityBond::new(to, from));
        
        reverse_bond.weaken(amount * 1.2);
    }
    
    /// Get affinity between agents
    pub fn get_affinity(&self, from: Uuid, to: Uuid) -> f64 {
        self.affinity_map
            .get(&(from, to))
            .map(|b| b.strength)
            .unwrap_or(self.calculate_base_affinity(from, to))
    }
    
    /// Calculate base affinity from semantic distance
    fn calculate_base_affinity(&self, from: Uuid, to: Uuid) -> f64 {
        if let (Some(pos1), Some(pos2)) = (self.positions.get(&from), self.positions.get(&to)) {
            // Closer agents have higher base affinity
            pos1.similarity_to(pos2) * self.config.base_affinity_factor
        } else {
            self.config.default_affinity
        }
    }
    
    /// Recalculate affinities for an agent
    fn recalculate_affinities_for(&mut self, agent_id: Uuid) {
        let keys_to_update: Vec<(Uuid, Uuid)> = self.affinity_map
            .keys()
            .filter(|(from, to)| *from == agent_id || *to == agent_id)
            .cloned()
            .collect();
        
        for (from, to) in keys_to_update {
            if let Some(bond) = self.affinity_map.get_mut(&(from, to)) {
                bond.update_semantic_factor(self.calculate_base_affinity(from, to));
            }
        }
    }
    
    /// Find agents with high affinity to given agent
    pub fn find_compatible_agents(&self, agent_id: Uuid, threshold: f64) -> Vec<(Uuid, f64)> {
        let mut compatible: Vec<(Uuid, f64)> = Vec::new();
        
        for ((from, to), bond) in &self.affinity_map {
            if *from == agent_id && bond.strength >= threshold {
                compatible.push((*to, bond.strength));
            }
        }
        
        compatible.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        compatible
    }
    
    /// Suggest collaboration partners
    pub fn suggest_collaborators(
        &self,
        agent_id: Uuid,
        intent_coords: SemanticCoords,
        limit: usize,
    ) -> Vec<CollaborationSuggestion> {
        let mut suggestions = Vec::new();
        
        for (&other_id, other_pos) in &self.positions {
            if other_id == agent_id {
                continue;
            }
            
            let affinity = self.get_affinity(agent_id, other_id);
            let intent_similarity = other_pos.similarity_to(&intent_coords);
            let collaboration_score = affinity * 0.6 + intent_similarity * 0.4;
            
            suggestions.push(CollaborationSuggestion {
                agent_id: other_id,
                affinity,
                intent_match: intent_similarity,
                overall_score: collaboration_score,
            });
        }
        
        suggestions.sort_by(|a, b| b.overall_score.partial_cmp(&a.overall_score).unwrap());
        suggestions.truncate(limit);
        suggestions
    }
    
    /// Apply decay to all bonds
    pub fn apply_decay(&mut self) {
        for bond in self.affinity_map.values_mut() {
            bond.decay(self.config.decay_rate);
        }
        
        // Remove very weak bonds
        self.affinity_map.retain(|_, bond| bond.strength > self.config.min_bond_strength);
    }
    
    /// Calculate network cohesion
    pub fn calculate_cohesion(&self) -> f64 {
        if self.affinity_map.is_empty() {
            return 0.0;
        }
        
        let total_strength: f64 = self.affinity_map.values()
            .map(|b| b.strength)
            .sum();
        
        let max_possible = self.affinity_map.len() as f64;
        total_strength / max_possible
    }
    
    /// Find clusters of high affinity
    pub fn find_affinity_clusters(&self, min_cluster_size: usize) -> Vec<Vec<Uuid>> {
        use std::collections::{HashSet, VecDeque};
        
        let mut clusters = Vec::new();
        let mut visited = HashSet::new();
        
        for &agent_id in self.positions.keys() {
            if visited.contains(&agent_id) {
                continue;
            }
            
            let mut cluster = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back(agent_id);
            visited.insert(agent_id);
            
            while let Some(current) = queue.pop_front() {
                cluster.push(current);
                
                // Find strongly connected agents
                for ((from, to), bond) in &self.affinity_map {
                    if *from == current 
                        && !visited.contains(to) 
                        && bond.strength >= self.config.cluster_threshold {
                        visited.insert(*to);
                        queue.push_back(*to);
                    }
                }
            }
            
            if cluster.len() >= min_cluster_size {
                clusters.push(cluster);
            }
        }
        
        clusters
    }
}

impl Default for PromiseChemistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A bond between two agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityBond {
    /// Source agent
    pub from: Uuid,
    
    /// Target agent
    pub to: Uuid,
    
    /// Bond strength (0-1)
    pub strength: f64,
    
    /// Number of successful cooperations
    pub success_count: u64,
    
    /// Number of failed cooperations
    pub failure_count: u64,
    
    /// Semantic similarity factor
    pub semantic_factor: f64,
    
    /// Bond type
    pub bond_type: BondType,
}

impl AffinityBond {
    /// Create a new affinity bond
    pub fn new(from: Uuid, to: Uuid) -> Self {
        Self {
            from,
            to,
            strength: 0.5,
            success_count: 0,
            failure_count: 0,
            semantic_factor: 0.5,
            bond_type: BondType::Neutral,
        }
    }
    
    /// Strengthen the bond
    pub fn strengthen(&mut self, amount: f64) {
        self.strength = (self.strength + amount).min(1.0);
        self.success_count += 1;
        self.update_type();
    }
    
    /// Weaken the bond
    pub fn weaken(&mut self, amount: f64) {
        self.strength = (self.strength - amount).max(0.0);
        self.failure_count += 1;
        self.update_type();
    }
    
    /// Apply decay
    pub fn decay(&mut self, rate: f64) {
        self.strength *= (1.0 - rate);
    }
    
    /// Update semantic factor
    pub fn update_semantic_factor(&mut self, factor: f64) {
        self.semantic_factor = factor;
        // Adjust strength based on semantic alignment
        self.strength = self.strength * 0.8 + self.semantic_factor * 0.2;
    }
    
    /// Update bond type based on strength
    fn update_type(&mut self) {
        self.bond_type = match self.strength {
            x if x >= 0.8 => BondType::Strong,
            x if x >= 0.6 => BondType::Good,
            x if x >= 0.4 => BondType::Neutral,
            x if x >= 0.2 => BondType::Weak,
            _ => BondType::Broken,
        };
    }
    
    /// Get cooperation success rate
    pub fn success_rate(&self) -> f64 {
        let total = (self.success_count + self.failure_count) as f64;
        if total == 0.0 {
            0.5
        } else {
            self.success_count as f64 / total
        }
    }
}

/// Types of affinity bonds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BondType {
    /// Very strong bond (0.8-1.0)
    Strong,
    /// Good bond (0.6-0.8)
    Good,
    /// Neutral bond (0.4-0.6)
    Neutral,
    /// Weak bond (0.2-0.4)
    Weak,
    /// Broken bond (0.0-0.2)
    Broken,
}

/// Collaboration suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationSuggestion {
    /// Suggested agent
    pub agent_id: Uuid,
    
    /// Existing affinity
    pub affinity: f64,
    
    /// Match with intent
    pub intent_match: f64,
    
    /// Overall collaboration score
    pub overall_score: f64,
}

/// Chemistry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemistryConfig {
    /// Base affinity factor for semantic similarity
    pub base_affinity_factor: f64,
    
    /// Default affinity for unknown pairs
    pub default_affinity: f64,
    
    /// Decay rate for bonds
    pub decay_rate: f64,
    
    /// Minimum bond strength to maintain
    pub min_bond_strength: f64,
    
    /// Threshold for cluster formation
    pub cluster_threshold: f64,
}

impl Default for ChemistryConfig {
    fn default() -> Self {
        Self {
            base_affinity_factor: 0.7,
            default_affinity: 0.3,
            decay_rate: 0.01,
            min_bond_strength: 0.1,
            cluster_threshold: 0.6,
        }
    }
}

/// Chemical reaction between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemicalReaction {
    /// Reactant agents
    pub reactants: Vec<Uuid>,
    
    /// Product (result of cooperation)
    pub product: ReactionProduct,
    
    /// Energy released (trust/affinity changes)
    pub energy: f64,
    
    /// Whether reaction was successful
    pub successful: bool,
}

/// Product of a chemical reaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReactionProduct {
    /// Successful story completion
    Story(uuid::Uuid),
    
    /// New emergent capability
    Capability(String),
    
    /// Strengthened bond
    Bond(AffinityBond),
    
    /// Failed reaction
    Failure(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_promise_chemistry() {
        let mut chemistry = PromiseChemistry::new();
        let agent1 = Uuid::new_v4();
        let agent2 = Uuid::new_v4();
        
        chemistry.register_agent(agent1, SemanticCoords::new(0.5, 0.5, 0.5, 0.5));
        chemistry.register_agent(agent2, SemanticCoords::new(0.6, 0.6, 0.6, 0.6));
        
        chemistry.strengthen_bond(agent1, agent2, 0.1);
        
        let affinity = chemistry.get_affinity(agent1, agent2);
        assert!(affinity > 0.5);
    }
    
    #[test]
    fn test_affinity_bond() {
        let mut bond = AffinityBond::new(Uuid::new_v4(), Uuid::new_v4());
        
        bond.strengthen(0.3);
        assert_eq!(bond.strength, 0.8);
        assert_eq!(bond.success_count, 1);
        
        bond.weaken(0.5);
        assert_eq!(bond.strength, 0.3);
        assert_eq!(bond.failure_count, 1);
    }
}