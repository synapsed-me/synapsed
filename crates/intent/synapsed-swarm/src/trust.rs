//! Trust management for swarm agents

use crate::{error::SwarmResult, types::AgentId};
use dashmap::DashMap;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Trust score for an agent
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrustScore {
    /// Current trust value (0.0 to 1.0)
    pub value: f64,
    /// Confidence in the trust score (0.0 to 1.0)
    pub confidence: f64,
    /// Number of interactions
    pub interactions: u64,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

impl TrustScore {
    /// Create a new trust score
    pub fn new(value: f64) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            confidence: 0.1, // Low initial confidence
            interactions: 0,
            last_updated: Utc::now(),
        }
    }
    
    /// Update trust score based on outcome
    pub fn update(&mut self, success: bool, verified: bool) {
        self.interactions += 1;
        
        // Calculate trust delta based on outcome
        let delta = if success {
            if verified {
                0.05 // Higher increase for verified success
            } else {
                0.02 // Lower increase for unverified success
            }
        } else {
            -0.1 // Penalty for failure
        };
        
        // Apply delta with decay based on interactions
        let decay_factor = 1.0 / (1.0 + (self.interactions as f64).log10());
        self.value = (self.value + delta * decay_factor).clamp(0.0, 1.0);
        
        // Update confidence based on interactions
        self.confidence = (1.0 - (1.0 / (1.0 + self.interactions as f64 * 0.1))).min(0.95);
        
        self.last_updated = Utc::now();
    }
    
    /// Check if trust meets threshold
    pub fn meets_threshold(&self, threshold: f64) -> bool {
        self.value >= threshold
    }
    
    /// Get effective trust (value * confidence)
    pub fn effective_trust(&self) -> f64 {
        self.value * self.confidence
    }
}

impl Default for TrustScore {
    fn default() -> Self {
        Self::new(crate::DEFAULT_TRUST_SCORE)
    }
}

/// Trust update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustUpdate {
    /// Agent whose trust was updated
    pub agent_id: AgentId,
    /// Previous trust score
    pub previous: TrustScore,
    /// New trust score
    pub current: TrustScore,
    /// Reason for update
    pub reason: TrustUpdateReason,
    /// Timestamp of update
    pub timestamp: DateTime<Utc>,
}

/// Reason for trust update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustUpdateReason {
    /// Task completed successfully
    TaskSuccess,
    /// Task failed
    TaskFailure,
    /// Promise fulfilled
    PromiseFulfilled,
    /// Promise broken
    PromiseBroken,
    /// Verification passed
    VerificationPassed,
    /// Verification failed
    VerificationFailed,
    /// Manual adjustment
    ManualAdjustment(String),
    /// Decay over time
    TimeDecay,
    /// Peer feedback
    PeerFeedback(f64),
}

/// Trust manager for the swarm
pub struct TrustManager {
    /// Trust scores for agents
    scores: Arc<DashMap<AgentId, TrustScore>>,
    /// Trust update history
    history: Arc<DashMap<AgentId, Vec<TrustUpdate>>>,
    /// Trust thresholds for different operations
    thresholds: TrustThresholds,
}

/// Trust thresholds for different operations
#[derive(Debug, Clone)]
pub struct TrustThresholds {
    /// Minimum trust for basic tasks
    pub basic_task: f64,
    /// Minimum trust for critical tasks
    pub critical_task: f64,
    /// Minimum trust for delegation
    pub delegation: f64,
    /// Minimum trust for verification
    pub verification: f64,
    /// Minimum trust for consensus participation
    pub consensus: f64,
}

impl Default for TrustThresholds {
    fn default() -> Self {
        Self {
            basic_task: 0.3,
            critical_task: 0.7,
            delegation: 0.5,
            verification: 0.6,
            consensus: 0.5,
        }
    }
}

impl TrustManager {
    /// Create a new trust manager
    pub fn new() -> Self {
        Self {
            scores: Arc::new(DashMap::new()),
            history: Arc::new(DashMap::new()),
            thresholds: TrustThresholds::default(),
        }
    }
    
    /// Create with custom thresholds
    pub fn with_thresholds(thresholds: TrustThresholds) -> Self {
        Self {
            scores: Arc::new(DashMap::new()),
            history: Arc::new(DashMap::new()),
            thresholds,
        }
    }
    
    /// Initialize trust manager
    pub async fn initialize(&self) -> SwarmResult<()> {
        // Could load historical trust data here
        Ok(())
    }
    
    /// Initialize trust for a new agent
    pub async fn initialize_agent(&self, agent_id: AgentId, initial_trust: f64) -> SwarmResult<()> {
        let score = TrustScore::new(initial_trust);
        self.scores.insert(agent_id, score);
        self.history.insert(agent_id, Vec::new());
        Ok(())
    }
    
    /// Get trust score for an agent
    pub async fn get_trust(&self, agent_id: AgentId) -> SwarmResult<f64> {
        self.scores
            .get(&agent_id)
            .map(|score| score.value)
            .ok_or_else(|| crate::error::SwarmError::AgentNotFound(agent_id))
    }
    
    /// Get full trust score for an agent
    pub async fn get_trust_score(&self, agent_id: AgentId) -> SwarmResult<TrustScore> {
        self.scores
            .get(&agent_id)
            .map(|entry| *entry)
            .ok_or_else(|| crate::error::SwarmError::AgentNotFound(agent_id))
    }
    
    /// Update trust based on task outcome
    pub async fn update_trust(
        &self,
        agent_id: AgentId,
        success: bool,
        verified: bool,
    ) -> SwarmResult<()> {
        let mut score = self.scores
            .get_mut(&agent_id)
            .ok_or_else(|| crate::error::SwarmError::AgentNotFound(agent_id))?;
        
        let previous = *score;
        score.update(success, verified);
        let current = *score;
        
        // Record update
        let reason = if success {
            if verified {
                TrustUpdateReason::VerificationPassed
            } else {
                TrustUpdateReason::TaskSuccess
            }
        } else {
            TrustUpdateReason::TaskFailure
        };
        
        self.record_update(agent_id, previous, current, reason).await;
        
        Ok(())
    }
    
    /// Update trust for promise outcome
    pub async fn update_trust_for_promise(
        &self,
        agent_id: AgentId,
        fulfilled: bool,
    ) -> SwarmResult<()> {
        let mut score = self.scores
            .get_mut(&agent_id)
            .ok_or_else(|| crate::error::SwarmError::AgentNotFound(agent_id))?;
        
        let previous = *score;
        score.update(fulfilled, true); // Promises are always "verified"
        let current = *score;
        
        let reason = if fulfilled {
            TrustUpdateReason::PromiseFulfilled
        } else {
            TrustUpdateReason::PromiseBroken
        };
        
        self.record_update(agent_id, previous, current, reason).await;
        
        Ok(())
    }
    
    /// Apply peer feedback to trust score
    pub async fn apply_peer_feedback(
        &self,
        agent_id: AgentId,
        feedback: f64,
        peer_trust: f64,
    ) -> SwarmResult<()> {
        let mut score = self.scores
            .get_mut(&agent_id)
            .ok_or_else(|| crate::error::SwarmError::AgentNotFound(agent_id))?;
        
        let previous = *score;
        
        // Weight feedback by peer's trust
        let weighted_feedback = feedback * peer_trust;
        let delta = (weighted_feedback - score.value) * 0.1; // Conservative update
        score.value = (score.value + delta).clamp(0.0, 1.0);
        score.last_updated = Utc::now();
        
        let current = *score;
        
        self.record_update(
            agent_id,
            previous,
            current,
            TrustUpdateReason::PeerFeedback(feedback),
        ).await;
        
        Ok(())
    }
    
    /// Check if agent meets trust threshold for operation
    pub async fn check_threshold(
        &self,
        agent_id: AgentId,
        operation: TrustOperation,
    ) -> SwarmResult<bool> {
        let score = self.get_trust(agent_id).await?;
        let threshold = match operation {
            TrustOperation::BasicTask => self.thresholds.basic_task,
            TrustOperation::CriticalTask => self.thresholds.critical_task,
            TrustOperation::Delegation => self.thresholds.delegation,
            TrustOperation::Verification => self.thresholds.verification,
            TrustOperation::Consensus => self.thresholds.consensus,
        };
        
        Ok(score >= threshold)
    }
    
    /// Get agents above trust threshold
    pub async fn get_trusted_agents(&self, threshold: f64) -> Vec<(AgentId, TrustScore)> {
        self.scores
            .iter()
            .filter(|entry| entry.value().value >= threshold)
            .map(|entry| (*entry.key(), *entry.value()))
            .collect()
    }
    
    /// Get trust history for an agent
    pub async fn get_history(&self, agent_id: AgentId) -> SwarmResult<Vec<TrustUpdate>> {
        self.history
            .get(&agent_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| crate::error::SwarmError::AgentNotFound(agent_id))
    }
    
    /// Record trust update
    async fn record_update(
        &self,
        agent_id: AgentId,
        previous: TrustScore,
        current: TrustScore,
        reason: TrustUpdateReason,
    ) {
        let update = TrustUpdate {
            agent_id,
            previous,
            current,
            reason,
            timestamp: Utc::now(),
        };
        
        if let Some(mut history) = self.history.get_mut(&agent_id) {
            history.push(update);
            
            // Limit history size
            if history.len() > 100 {
                history.drain(0..10);
            }
        }
    }
    
    /// Apply time decay to all trust scores
    pub async fn apply_time_decay(&self, decay_rate: f64) -> SwarmResult<()> {
        for mut entry in self.scores.iter_mut() {
            let previous = *entry.value();
            
            // Apply decay based on time since last update
            let hours_since_update = (Utc::now() - previous.last_updated).num_hours() as f64;
            if hours_since_update > 24.0 {
                let decay = decay_rate * (hours_since_update / 24.0).min(1.0);
                entry.value -= entry.value * decay;
                entry.last_updated = Utc::now();
                
                self.record_update(
                    *entry.key(),
                    previous,
                    *entry.value(),
                    TrustUpdateReason::TimeDecay,
                ).await;
            }
        }
        
        Ok(())
    }
}

/// Type of operation for trust checking
#[derive(Debug, Clone, Copy)]
pub enum TrustOperation {
    BasicTask,
    CriticalTask,
    Delegation,
    Verification,
    Consensus,
}