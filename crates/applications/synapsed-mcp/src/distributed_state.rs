//! Distributed state management using CRDTs for agent coordination

use crate::error::{McpError, Result};
use synapsed_crdt::{
    OrSet, PnCounter, LwwRegister, Crdt, Mergeable,
    ActorId, VectorClock, Timestamp,
};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Agent information stored in distributed state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentInfo {
    /// Agent's anonymous ID (DID)
    pub agent_id: String,
    /// Agent capabilities
    pub capabilities: Vec<String>,
    /// Trust score (0.0 - 1.0)
    pub trust_score: f64,
    /// Number of completed intents
    pub completed_intents: u64,
    /// Number of failed intents
    pub failed_intents: u64,
    /// Last seen timestamp
    pub last_seen: DateTime<Utc>,
    /// Agent's public key (for encryption)
    pub public_key: Option<Vec<u8>>,
}

/// Intent record in distributed state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DistributedIntent {
    /// Intent ID
    pub id: String,
    /// Agent that declared the intent
    pub agent_id: String,
    /// Intent goal
    pub goal: String,
    /// Intent status
    pub status: IntentStatus,
    /// Verification proofs
    pub proofs: Vec<VerificationProof>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Intent status in distributed system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IntentStatus {
    Declared,
    InProgress,
    Completed,
    Failed,
    Verified,
}

/// Verification proof attached to intents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct VerificationProof {
    /// Verifier agent ID
    pub verifier: String,
    /// Verification result
    pub verified: bool,
    /// Cryptographic proof (signature)
    pub proof: Vec<u8>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Distributed state manager using CRDTs
pub struct DistributedState {
    /// Our actor ID for CRDT operations
    actor_id: ActorId,
    /// OR-Set of active agents
    agents: Arc<RwLock<OrSet<AgentInfo>>>,
    /// OR-Set of intents
    intents: Arc<RwLock<OrSet<DistributedIntent>>>,
    /// PN-Counter for global reputation
    global_reputation: Arc<RwLock<PnCounter>>,
    /// LWW-Register for network configuration
    network_config: Arc<RwLock<LwwRegister<NetworkConfig>>>,
    /// Agent-specific reputation counters
    agent_reputation: Arc<RwLock<HashMap<String, PnCounter>>>,
    /// Vector clock for causality tracking
    vector_clock: Arc<RwLock<VectorClock>>,
}

/// Network configuration stored in CRDT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Minimum trust score for participation
    pub min_trust_score: f64,
    /// Maximum agents in network
    pub max_agents: usize,
    /// Gossip interval in seconds
    pub gossip_interval_secs: u64,
    /// Rendezvous points for discovery
    pub rendezvous_points: Vec<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            min_trust_score: 0.1,
            max_agents: 1000,
            gossip_interval_secs: 30,
            rendezvous_points: vec![],
        }
    }
}

impl DistributedState {
    /// Create new distributed state manager
    pub fn new() -> Self {
        let actor_id = ActorId::new();
        info!("Created distributed state with actor ID: {:?}", actor_id);
        
        Self {
            actor_id: actor_id.clone(),
            agents: Arc::new(RwLock::new(OrSet::new(actor_id.clone()))),
            intents: Arc::new(RwLock::new(OrSet::new(actor_id.clone()))),
            global_reputation: Arc::new(RwLock::new(PnCounter::new(actor_id.clone()))),
            network_config: Arc::new(RwLock::new(LwwRegister::new(actor_id.clone()))),
            agent_reputation: Arc::new(RwLock::new(HashMap::new())),
            vector_clock: Arc::new(RwLock::new(VectorClock::new())),
        }
    }
    
    /// Add an agent to the network
    pub async fn add_agent(&self, agent: AgentInfo) -> Result<()> {
        let mut agents = self.agents.write().await;
        agents.add(agent.clone());
        
        // Initialize reputation counter for agent
        let mut rep_map = self.agent_reputation.write().await;
        rep_map.entry(agent.agent_id.clone())
            .or_insert_with(|| PnCounter::new(self.actor_id.clone()));
        
        debug!("Added agent {} to distributed state", agent.agent_id);
        Ok(())
    }
    
    /// Remove an agent from the network
    pub async fn remove_agent(&self, agent_id: &str) -> Result<()> {
        let mut agents = self.agents.write().await;
        let agents_list = agents.elements();
        
        if let Some(agent) = agents_list.iter().find(|a| a.agent_id == agent_id) {
            agents.remove(agent.clone());
            debug!("Removed agent {} from distributed state", agent_id);
        }
        
        Ok(())
    }
    
    /// Get all active agents
    pub async fn get_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents.elements()
    }
    
    /// Add an intent to distributed state
    pub async fn add_intent(&self, intent: DistributedIntent) -> Result<()> {
        let mut intents = self.intents.write().await;
        intents.add(intent.clone());
        
        // Update vector clock
        let mut clock = self.vector_clock.write().await;
        clock.increment(&self.actor_id);
        
        debug!("Added intent {} to distributed state", intent.id);
        Ok(())
    }
    
    /// Update intent status
    pub async fn update_intent_status(&self, intent_id: &str, status: IntentStatus) -> Result<()> {
        let mut intents = self.intents.write().await;
        let current_intents = intents.elements();
        
        // Find and update intent (remove old, add updated)
        if let Some(mut intent) = current_intents.iter().find(|i| i.id == intent_id).cloned() {
            intents.remove(intent.clone());
            intent.status = status;
            intents.add(intent);
            
            debug!("Updated intent {} status", intent_id);
        }
        
        Ok(())
    }
    
    /// Add verification proof to intent
    pub async fn add_verification(&self, intent_id: &str, proof: VerificationProof) -> Result<()> {
        let mut intents = self.intents.write().await;
        let current_intents = intents.elements();
        
        if let Some(mut intent) = current_intents.iter().find(|i| i.id == intent_id).cloned() {
            intents.remove(intent.clone());
            intent.proofs.push(proof);
            
            // Update status if enough verifications
            if intent.proofs.iter().filter(|p| p.verified).count() >= 3 {
                intent.status = IntentStatus::Verified;
            }
            
            intents.add(intent);
        }
        
        Ok(())
    }
    
    /// Get all intents
    pub async fn get_intents(&self) -> Vec<DistributedIntent> {
        let intents = self.intents.read().await;
        intents.elements()
    }
    
    /// Update agent reputation
    pub async fn update_reputation(&self, agent_id: &str, delta: i64) -> Result<()> {
        let mut rep_map = self.agent_reputation.write().await;
        let counter = rep_map.entry(agent_id.to_string())
            .or_insert_with(|| PnCounter::new(self.actor_id.clone()));
        
        if delta > 0 {
            counter.increment(delta as u64);
        } else {
            counter.decrement((-delta) as u64);
        }
        
        // Update global reputation
        let mut global = self.global_reputation.write().await;
        if delta > 0 {
            global.increment(1);
        } else {
            global.decrement(1);
        }
        
        debug!("Updated reputation for agent {} by {}", agent_id, delta);
        Ok(())
    }
    
    /// Get agent reputation
    pub async fn get_reputation(&self, agent_id: &str) -> i64 {
        let rep_map = self.agent_reputation.read().await;
        rep_map.get(agent_id)
            .map(|counter| counter.value())
            .unwrap_or(0)
    }
    
    /// Merge state from another node
    pub async fn merge_state(&self, other: &DistributedState) -> Result<()> {
        info!("Merging distributed state from another node");
        
        // Merge agents
        {
            let mut our_agents = self.agents.write().await;
            let their_agents = other.agents.read().await;
            our_agents.merge(&*their_agents);
        }
        
        // Merge intents
        {
            let mut our_intents = self.intents.write().await;
            let their_intents = other.intents.read().await;
            our_intents.merge(&*their_intents);
        }
        
        // Merge global reputation
        {
            let mut our_rep = self.global_reputation.write().await;
            let their_rep = other.global_reputation.read().await;
            our_rep.merge(&*their_rep);
        }
        
        // Merge network config (LWW - last writer wins)
        {
            let mut our_config = self.network_config.write().await;
            let their_config = other.network_config.read().await;
            our_config.merge(&*their_config);
        }
        
        // Merge agent reputations
        {
            let mut our_reps = self.agent_reputation.write().await;
            let their_reps = other.agent_reputation.read().await;
            
            for (agent_id, their_counter) in their_reps.iter() {
                let our_counter = our_reps.entry(agent_id.clone())
                    .or_insert_with(|| PnCounter::new(self.actor_id.clone()));
                our_counter.merge(their_counter);
            }
        }
        
        // Merge vector clocks
        {
            let mut our_clock = self.vector_clock.write().await;
            let their_clock = other.vector_clock.read().await;
            our_clock.merge(&*their_clock);
        }
        
        debug!("State merge completed");
        Ok(())
    }
    
    /// Get a snapshot of the current state for synchronization
    pub async fn snapshot(&self) -> StateSnapshot {
        StateSnapshot {
            agents: self.agents.read().await.clone(),
            intents: self.intents.read().await.clone(),
            global_reputation: self.global_reputation.read().await.clone(),
            network_config: self.network_config.read().await.clone(),
            vector_clock: self.vector_clock.read().await.clone(),
            timestamp: Utc::now(),
        }
    }
    
    /// Apply a state snapshot
    pub async fn apply_snapshot(&self, snapshot: StateSnapshot) -> Result<()> {
        info!("Applying state snapshot from {}", snapshot.timestamp);
        
        // Create temporary state to merge
        let temp_state = DistributedState::new();
        *temp_state.agents.write().await = snapshot.agents;
        *temp_state.intents.write().await = snapshot.intents;
        *temp_state.global_reputation.write().await = snapshot.global_reputation;
        *temp_state.network_config.write().await = snapshot.network_config;
        *temp_state.vector_clock.write().await = snapshot.vector_clock;
        
        // Merge with our state
        self.merge_state(&temp_state).await?;
        
        Ok(())
    }
    
    /// Garbage collect old data
    pub async fn garbage_collect(&self, max_age_secs: u64) -> Result<()> {
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_secs as i64);
        
        // Remove old agents
        let mut agents = self.agents.write().await;
        let old_agents: Vec<_> = agents.elements()
            .into_iter()
            .filter(|a| a.last_seen < cutoff)
            .collect();
        
        for agent in old_agents {
            agents.remove(agent);
        }
        
        // Remove old intents
        let mut intents = self.intents.write().await;
        let old_intents: Vec<_> = intents.elements()
            .into_iter()
            .filter(|i| i.timestamp < cutoff)
            .collect();
        
        for intent in old_intents {
            intents.remove(intent);
        }
        
        debug!("Garbage collection completed");
        Ok(())
    }
}

/// Snapshot of distributed state for synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub agents: OrSet<AgentInfo>,
    pub intents: OrSet<DistributedIntent>,
    pub global_reputation: PnCounter,
    pub network_config: LwwRegister<NetworkConfig>,
    pub vector_clock: VectorClock,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_distributed_state() {
        let state = DistributedState::new();
        
        // Add agent
        let agent = AgentInfo {
            agent_id: "agent1".to_string(),
            capabilities: vec!["intent".to_string()],
            trust_score: 0.8,
            completed_intents: 0,
            failed_intents: 0,
            last_seen: Utc::now(),
            public_key: None,
        };
        
        state.add_agent(agent).await.unwrap();
        
        let agents = state.get_agents().await;
        assert_eq!(agents.len(), 1);
    }
}