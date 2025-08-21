//! Trust model implementation for Promise Theory

use crate::{types::*, Result, PromiseError};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use chrono::{DateTime, Utc, Duration};

/// Level of trust in an agent
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum TrustLevel {
    /// No trust established
    None = 0,
    /// Very low trust
    VeryLow = 1,
    /// Low trust
    Low = 2,
    /// Medium trust
    Medium = 3,
    /// High trust
    High = 4,
    /// Very high trust
    VeryHigh = 5,
    /// Complete trust
    Complete = 6,
}

impl TrustLevel {
    /// Checks if trust level is sufficient for basic cooperation
    pub fn is_sufficient(&self) -> bool {
        *self >= TrustLevel::Low
    }
    
    /// Checks if trust level is high enough for critical operations
    pub fn is_high(&self) -> bool {
        *self >= TrustLevel::High
    }
    
    /// Converts trust score to trust level
    pub fn from_score(score: f64) -> Self {
        match score {
            s if s >= 0.95 => TrustLevel::Complete,
            s if s >= 0.85 => TrustLevel::VeryHigh,
            s if s >= 0.70 => TrustLevel::High,
            s if s >= 0.50 => TrustLevel::Medium,
            s if s >= 0.30 => TrustLevel::Low,
            s if s > 0.0 => TrustLevel::VeryLow,
            _ => TrustLevel::None,
        }
    }
}

/// Reputation of an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reputation {
    /// Agent being evaluated
    pub agent_id: AgentId,
    /// Total promises made
    pub promises_made: u64,
    /// Promises kept
    pub promises_kept: u64,
    /// Promises broken
    pub promises_broken: u64,
    /// Average quality of kept promises
    pub average_quality: f64,
    /// Trust score (0.0 to 1.0)
    pub trust_score: f64,
    /// Current trust level
    pub trust_level: TrustLevel,
    /// History of interactions
    pub interactions: Vec<Interaction>,
    /// Last updated
    pub last_updated: DateTime<Utc>,
}

impl Reputation {
    /// Creates a new reputation record
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            promises_made: 0,
            promises_kept: 0,
            promises_broken: 0,
            average_quality: 0.0,
            trust_score: 0.5, // Start with neutral trust
            trust_level: TrustLevel::Medium,
            interactions: Vec::new(),
            last_updated: Utc::now(),
        }
    }
    
    /// Updates reputation based on a promise outcome
    pub fn update(&mut self, kept: bool, quality: f64) {
        self.promises_made += 1;
        
        if kept {
            self.promises_kept += 1;
            // Update average quality
            let total_quality = self.average_quality * (self.promises_kept - 1) as f64 + quality;
            self.average_quality = total_quality / self.promises_kept as f64;
        } else {
            self.promises_broken += 1;
        }
        
        // Calculate new trust score
        self.recalculate_trust_score();
        
        // Add interaction record
        self.interactions.push(Interaction {
            timestamp: Utc::now(),
            kept,
            quality,
        });
        
        // Keep only last 100 interactions
        if self.interactions.len() > 100 {
            self.interactions.remove(0);
        }
        
        self.last_updated = Utc::now();
    }
    
    /// Recalculates the trust score
    fn recalculate_trust_score(&mut self) {
        if self.promises_made == 0 {
            self.trust_score = 0.5;
            self.trust_level = TrustLevel::Medium;
            return;
        }
        
        // Base score from kept/broken ratio
        let kept_ratio = self.promises_kept as f64 / self.promises_made as f64;
        
        // Weight by quality
        let quality_weight = self.average_quality;
        
        // Recent interactions have more weight
        let recent_weight = self.calculate_recent_weight();
        
        // Combine factors
        self.trust_score = (kept_ratio * 0.5 + quality_weight * 0.3 + recent_weight * 0.2)
            .max(0.0)
            .min(1.0);
        
        self.trust_level = TrustLevel::from_score(self.trust_score);
    }
    
    /// Calculates weight based on recent interactions
    fn calculate_recent_weight(&self) -> f64 {
        if self.interactions.is_empty() {
            return 0.5;
        }
        
        let recent = self.interactions.iter()
            .rev()
            .take(10)
            .collect::<Vec<_>>();
        
        let kept_count = recent.iter().filter(|i| i.kept).count() as f64;
        let total = recent.len() as f64;
        
        kept_count / total
    }
}

/// Record of an interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    /// When the interaction occurred
    pub timestamp: DateTime<Utc>,
    /// Whether the promise was kept
    pub kept: bool,
    /// Quality of the interaction
    pub quality: f64,
}

/// Trust model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustModelConfig {
    /// Initial trust level for new agents
    pub initial_trust: TrustLevel,
    /// Decay rate for trust over time (per day)
    pub decay_rate: f64,
    /// Minimum interactions before trust is established
    pub min_interactions: u32,
    /// Whether to use transitive trust
    pub use_transitive_trust: bool,
    /// Maximum transitive trust hops
    pub max_trust_hops: u32,
}

impl Default for TrustModelConfig {
    fn default() -> Self {
        Self {
            initial_trust: TrustLevel::Low,
            decay_rate: 0.01,
            min_interactions: 3,
            use_transitive_trust: true,
            max_trust_hops: 2,
        }
    }
}

/// Trust model for managing agent trust relationships
#[derive(Debug, Clone)]
pub struct TrustModel {
    /// Configuration
    config: TrustModelConfig,
    /// Reputation records
    reputations: Arc<DashMap<AgentId, Reputation>>,
    /// Trust relationships (transitive trust)
    trust_graph: Arc<DashMap<(AgentId, AgentId), f64>>,
}

impl TrustModel {
    /// Creates a new trust model
    pub fn new(config: TrustModelConfig) -> Self {
        Self {
            config,
            reputations: Arc::new(DashMap::new()),
            trust_graph: Arc::new(DashMap::new()),
        }
    }
    
    /// Creates with default configuration
    pub fn default() -> Self {
        Self::new(TrustModelConfig::default())
    }
    
    /// Initializes the trust model
    pub fn initialize(&mut self) -> Result<()> {
        // Could load persistent trust data here
        Ok(())
    }
    
    /// Gets the trust level for an agent
    pub async fn get_trust_level(&self, agent_id: AgentId) -> Result<TrustLevel> {
        if let Some(reputation) = self.reputations.get(&agent_id) {
            // Apply time decay
            let decayed = self.apply_decay(&reputation);
            Ok(decayed.trust_level)
        } else {
            Ok(self.config.initial_trust)
        }
    }
    
    /// Gets the reputation for an agent
    pub async fn get_reputation(&self, agent_id: AgentId) -> Option<Reputation> {
        self.reputations.get(&agent_id).map(|r| r.clone())
    }
    
    /// Updates trust based on a promise outcome
    pub async fn update_trust(
        &mut self,
        agent_id: AgentId,
        kept: bool,
        quality: f64,
    ) -> Result<()> {
        self.reputations
            .entry(agent_id)
            .or_insert_with(|| Reputation::new(agent_id))
            .update(kept, quality);
        
        Ok(())
    }
    
    /// Establishes transitive trust between agents
    pub async fn establish_transitive_trust(
        &mut self,
        from: AgentId,
        to: AgentId,
        through: AgentId,
    ) -> Result<()> {
        if !self.config.use_transitive_trust {
            return Err(PromiseError::ValidationFailed(
                "Transitive trust is disabled".to_string()
            ));
        }
        
        // Get trust levels
        let trust_to_through = self.get_trust_score(from, through).await?;
        let trust_through_to = self.get_trust_score(through, to).await?;
        
        // Calculate transitive trust (reduced by one hop)
        let transitive_trust = (trust_to_through * trust_through_to * 0.8)
            .max(0.0)
            .min(1.0);
        
        self.trust_graph.insert((from, to), transitive_trust);
        
        Ok(())
    }
    
    /// Gets trust score between two agents
    async fn get_trust_score(&self, from: AgentId, to: AgentId) -> Result<f64> {
        // Direct trust
        if let Some(reputation) = self.reputations.get(&to) {
            return Ok(reputation.trust_score);
        }
        
        // Transitive trust
        if self.config.use_transitive_trust {
            if let Some(score) = self.trust_graph.get(&(from, to)) {
                return Ok(*score);
            }
        }
        
        // Default trust
        Ok(0.5)
    }
    
    /// Applies time-based decay to reputation
    fn apply_decay(&self, reputation: &Reputation) -> Reputation {
        let mut decayed = reputation.clone();
        
        let days_since_update = (Utc::now() - reputation.last_updated).num_days();
        if days_since_update > 0 {
            let decay = self.config.decay_rate * days_since_update as f64;
            decayed.trust_score = (reputation.trust_score - decay).max(0.0);
            decayed.trust_level = TrustLevel::from_score(decayed.trust_score);
        }
        
        decayed
    }
    
    /// Resets trust for an agent
    pub async fn reset_trust(&mut self, agent_id: AgentId) -> Result<()> {
        self.reputations.remove(&agent_id);
        
        // Remove from trust graph
        let keys_to_remove: Vec<_> = self.trust_graph
            .iter()
            .filter(|entry| entry.key().0 == agent_id || entry.key().1 == agent_id)
            .map(|entry| entry.key().clone())
            .collect();
        
        for key in keys_to_remove {
            self.trust_graph.remove(&key);
        }
        
        Ok(())
    }
    
    /// Gets all trusted agents above a threshold
    pub async fn get_trusted_agents(&self, min_level: TrustLevel) -> Vec<AgentId> {
        self.reputations
            .iter()
            .filter(|entry| entry.value().trust_level >= min_level)
            .map(|entry| entry.key().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_trust_model() {
        let mut trust_model = TrustModel::default();
        let agent_id = AgentId::new();
        
        // Initial trust
        let initial = trust_model.get_trust_level(agent_id).await.unwrap();
        assert_eq!(initial, TrustLevel::Low);
        
        // Update with kept promise
        trust_model.update_trust(agent_id, true, 0.9).await.unwrap();
        trust_model.update_trust(agent_id, true, 0.8).await.unwrap();
        trust_model.update_trust(agent_id, true, 0.95).await.unwrap();
        
        // Check improved trust
        let updated = trust_model.get_trust_level(agent_id).await.unwrap();
        assert!(updated > initial);
        
        // Update with broken promise
        trust_model.update_trust(agent_id, false, 0.0).await.unwrap();
        
        // Check reputation
        let reputation = trust_model.get_reputation(agent_id).await.unwrap();
        assert_eq!(reputation.promises_made, 4);
        assert_eq!(reputation.promises_kept, 3);
        assert_eq!(reputation.promises_broken, 1);
    }
    
    #[test]
    fn test_trust_level_conversion() {
        assert_eq!(TrustLevel::from_score(0.0), TrustLevel::None);
        assert_eq!(TrustLevel::from_score(0.1), TrustLevel::VeryLow);
        assert_eq!(TrustLevel::from_score(0.4), TrustLevel::Low);
        assert_eq!(TrustLevel::from_score(0.6), TrustLevel::Medium);
        assert_eq!(TrustLevel::from_score(0.75), TrustLevel::High);
        assert_eq!(TrustLevel::from_score(0.9), TrustLevel::VeryHigh);
        assert_eq!(TrustLevel::from_score(0.98), TrustLevel::Complete);
    }
}