//! Trust management for swarm agents

use crate::{
    error::SwarmResult, 
    types::AgentId,
    persistence::{TrustStore, InMemoryTrustStore},
};
use dashmap::DashMap;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::{sync::RwLock, time::{interval, Duration}};
use tracing::{debug, info, warn};

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
    /// Persistent storage for trust data
    storage: Arc<dyn TrustStore>,
    /// In-memory cache for fast access
    cache: Arc<DashMap<AgentId, TrustScore>>,
    /// Trust thresholds for different operations
    thresholds: TrustThresholds,
    /// Backup configuration
    backup_config: BackupConfig,
    /// Shutdown signal for background tasks
    shutdown: Arc<RwLock<bool>>,
}

/// Configuration for trust score backups
#[derive(Debug, Clone)]
pub struct BackupConfig {
    /// Enable periodic backups
    pub enabled: bool,
    /// Backup interval in seconds
    pub interval_secs: u64,
    /// Enable backup on significant trust changes
    pub on_significant_change: bool,
    /// Threshold for "significant" trust change
    pub significant_change_threshold: f64,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 3600, // 1 hour
            on_significant_change: true,
            significant_change_threshold: 0.1,
        }
    }
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
    /// Create a new trust manager with in-memory storage
    pub fn new() -> Self {
        Self::with_storage(Arc::new(InMemoryTrustStore::new()))
    }
    
    /// Create with custom storage
    pub fn with_storage(storage: Arc<dyn TrustStore>) -> Self {
        Self {
            storage,
            cache: Arc::new(DashMap::new()),
            thresholds: TrustThresholds::default(),
            backup_config: BackupConfig::default(),
            shutdown: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Create with custom thresholds and storage
    pub fn with_thresholds_and_storage(
        thresholds: TrustThresholds,
        storage: Arc<dyn TrustStore>,
    ) -> Self {
        Self {
            storage,
            cache: Arc::new(DashMap::new()),
            thresholds,
            backup_config: BackupConfig::default(),
            shutdown: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Configure backup settings
    pub fn with_backup_config(mut self, config: BackupConfig) -> Self {
        self.backup_config = config;
        self
    }
    
    /// Initialize trust manager
    pub async fn initialize(&self) -> SwarmResult<()> {
        // Initialize the storage backend
        self.storage.initialize().await?;
        
        // Load existing trust scores into cache
        let scores = self.storage.get_all_trust_scores().await?;
        for (agent_id, score) in scores {
            self.cache.insert(agent_id, score);
        }
        
        info!("Loaded {} trust scores from storage", self.cache.len());
        
        // Start periodic backup task if enabled
        if self.backup_config.enabled {
            self.start_periodic_backup().await;
        }
        
        Ok(())
    }
    
    /// Start periodic backup task
    async fn start_periodic_backup(&self) {
        let storage = Arc::clone(&self.storage);
        let shutdown = Arc::clone(&self.shutdown);
        let interval_secs = self.backup_config.interval_secs;
        
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));
            
            loop {
                ticker.tick().await;
                
                // Check if we should shutdown
                if *shutdown.read().await {
                    debug!("Periodic backup task shutting down");
                    break;
                }
                
                // Create backup
                let backup_path = std::env::temp_dir().join(format!(
                    "synapsed_trust_backup_{}.backup", 
                    chrono::Utc::now().timestamp()
                ));
                
                if let Err(e) = storage.create_backup(&backup_path).await {
                    warn!("Failed to create periodic backup: {}", e);
                } else {
                    debug!("Created periodic backup at {:?}", backup_path);
                }
            }
        });
    }
    
    /// Initialize trust for a new agent
    pub async fn initialize_agent(&self, agent_id: AgentId, initial_trust: f64) -> SwarmResult<()> {
        let score = TrustScore::new(initial_trust);
        
        // Store in persistent storage
        self.storage.store_trust_score(agent_id, score).await?;
        
        // Update cache
        self.cache.insert(agent_id, score);
        
        info!("Initialized trust for agent {} with score {}", agent_id, initial_trust);
        
        Ok(())
    }
    
    /// Get trust score for an agent
    pub async fn get_trust(&self, agent_id: AgentId) -> SwarmResult<f64> {
        // Try cache first
        if let Some(score) = self.cache.get(&agent_id) {
            return Ok(score.value);
        }
        
        // Fall back to storage
        if let Some(score) = self.storage.get_trust_score(agent_id).await? {
            // Update cache
            self.cache.insert(agent_id, score);
            Ok(score.value)
        } else {
            Err(crate::error::SwarmError::AgentNotFound(agent_id))
        }
    }
    
    /// Get full trust score for an agent
    pub async fn get_trust_score(&self, agent_id: AgentId) -> SwarmResult<TrustScore> {
        // Try cache first
        if let Some(score) = self.cache.get(&agent_id) {
            return Ok(*score.value());
        }
        
        // Fall back to storage
        if let Some(score) = self.storage.get_trust_score(agent_id).await? {
            // Update cache
            self.cache.insert(agent_id, score);
            Ok(score)
        } else {
            Err(crate::error::SwarmError::AgentNotFound(agent_id))
        }
    }
    
    /// Update trust based on task outcome
    pub async fn update_trust(
        &self,
        agent_id: AgentId,
        success: bool,
        verified: bool,
    ) -> SwarmResult<()> {
        // Get current score from cache or storage
        let current_score = self.get_trust_score(agent_id).await?;
        let previous = current_score;
        
        // Calculate new score
        let mut new_score = current_score;
        new_score.update(success, verified);
        
        // Determine if this is a significant change
        let is_significant = (new_score.value - previous.value).abs() 
            >= self.backup_config.significant_change_threshold;
        
        // Use transaction for atomic update
        let mut tx = self.storage.begin_transaction().await?;
        tx.store_trust_score(agent_id, new_score).await?;
        
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
        
        let update = TrustUpdate {
            agent_id,
            previous,
            current: new_score,
            reason,
            timestamp: Utc::now(),
        };
        
        tx.store_trust_update(&update).await?;
        tx.commit().await?;
        
        // Update cache
        self.cache.insert(agent_id, new_score);
        
        debug!(
            "Updated trust for agent {} from {:.3} to {:.3} (reason: {:?})",
            agent_id, previous.value, new_score.value, reason
        );
        
        // Create backup on significant change if enabled
        if self.backup_config.on_significant_change && is_significant {
            let backup_path = std::env::temp_dir().join(format!(
                "synapsed_trust_significant_{}.backup", 
                chrono::Utc::now().timestamp()
            ));
            
            if let Err(e) = self.storage.create_backup(&backup_path).await {
                warn!("Failed to create backup on significant change: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Update trust for promise outcome
    pub async fn update_trust_for_promise(
        &self,
        agent_id: AgentId,
        fulfilled: bool,
    ) -> SwarmResult<()> {
        // Get current score
        let current_score = self.get_trust_score(agent_id).await?;
        let previous = current_score;
        
        // Calculate new score
        let mut new_score = current_score;
        new_score.update(fulfilled, true); // Promises are always "verified"
        
        // Use transaction for atomic update
        let mut tx = self.storage.begin_transaction().await?;
        tx.store_trust_score(agent_id, new_score).await?;
        
        let reason = if fulfilled {
            TrustUpdateReason::PromiseFulfilled
        } else {
            TrustUpdateReason::PromiseBroken
        };
        
        let update = TrustUpdate {
            agent_id,
            previous,
            current: new_score,
            reason,
            timestamp: Utc::now(),
        };
        
        tx.store_trust_update(&update).await?;
        tx.commit().await?;
        
        // Update cache
        self.cache.insert(agent_id, new_score);
        
        debug!(
            "Updated trust for agent {} promise (fulfilled: {}) from {:.3} to {:.3}",
            agent_id, fulfilled, previous.value, new_score.value
        );
        
        Ok(())
    }
    
    /// Apply peer feedback to trust score
    pub async fn apply_peer_feedback(
        &self,
        agent_id: AgentId,
        feedback: f64,
        peer_trust: f64,
    ) -> SwarmResult<()> {
        // Get current score
        let current_score = self.get_trust_score(agent_id).await?;
        let previous = current_score;
        
        // Calculate new score with peer feedback
        let mut new_score = current_score;
        
        // Weight feedback by peer's trust
        let weighted_feedback = feedback * peer_trust;
        let delta = (weighted_feedback - new_score.value) * 0.1; // Conservative update
        new_score.value = (new_score.value + delta).clamp(0.0, 1.0);
        new_score.last_updated = Utc::now();
        
        // Use transaction for atomic update
        let mut tx = self.storage.begin_transaction().await?;
        tx.store_trust_score(agent_id, new_score).await?;
        
        let update = TrustUpdate {
            agent_id,
            previous,
            current: new_score,
            reason: TrustUpdateReason::PeerFeedback(feedback),
            timestamp: Utc::now(),
        };
        
        tx.store_trust_update(&update).await?;
        tx.commit().await?;
        
        // Update cache
        self.cache.insert(agent_id, new_score);
        
        debug!(
            "Applied peer feedback for agent {} (feedback: {:.3}, peer_trust: {:.3}) from {:.3} to {:.3}",
            agent_id, feedback, peer_trust, previous.value, new_score.value
        );
        
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
    pub async fn get_trusted_agents(&self, threshold: f64) -> SwarmResult<Vec<(AgentId, TrustScore)>> {
        // Get all scores from storage to ensure we have the latest data
        let all_scores = self.storage.get_all_trust_scores().await?;
        
        Ok(all_scores
            .into_iter()
            .filter(|(_, score)| score.value >= threshold)
            .collect())
    }
    
    /// Get trust history for an agent
    pub async fn get_history(&self, agent_id: AgentId, limit: Option<usize>) -> SwarmResult<Vec<TrustUpdate>> {
        self.storage.get_trust_history(agent_id, limit).await
    }
    
    /// Get trust updates since a specific timestamp
    pub async fn get_updates_since(&self, timestamp: DateTime<Utc>) -> SwarmResult<Vec<TrustUpdate>> {
        self.storage.get_trust_updates_since(timestamp).await
    }
    
    /// Remove an agent and all associated data
    pub async fn remove_agent(&self, agent_id: AgentId) -> SwarmResult<()> {
        // Remove from storage
        self.storage.remove_agent(agent_id).await?;
        
        // Remove from cache
        self.cache.remove(&agent_id);
        
        info!("Removed agent {} from trust management", agent_id);
        Ok(())
    }
    
    /// Create a backup of trust data
    pub async fn create_backup<P: AsRef<std::path::Path>>(&self, path: P) -> SwarmResult<()> {
        self.storage.create_backup(path.as_ref()).await
    }
    
    /// Restore from a backup
    pub async fn restore_backup<P: AsRef<std::path::Path>>(&self, path: P) -> SwarmResult<()> {
        // Restore storage
        self.storage.restore_backup(path.as_ref()).await?;
        
        // Reload cache
        self.cache.clear();
        let scores = self.storage.get_all_trust_scores().await?;
        for (agent_id, score) in scores {
            self.cache.insert(agent_id, score);
        }
        
        info!("Restored trust data from backup and reloaded cache");
        Ok(())
    }
    
    /// Get storage health information
    pub async fn get_storage_health(&self) -> SwarmResult<crate::persistence::StorageHealth> {
        self.storage.health_check().await
    }
    
    /// Cleanup old trust update data
    pub async fn cleanup_old_data(&self, older_than: DateTime<Utc>) -> SwarmResult<usize> {
        self.storage.cleanup_old_data(older_than).await
    }
    
    /// Shutdown the trust manager and cleanup background tasks
    pub async fn shutdown(&self) {
        *self.shutdown.write().await = true;
        info!("Trust manager shutting down");
    }
    
    /// Apply time decay to all trust scores
    pub async fn apply_time_decay(&self, decay_rate: f64) -> SwarmResult<()> {
        let all_scores = self.storage.get_all_trust_scores().await?;
        let mut updates = Vec::new();
        
        for (agent_id, score) in all_scores {
            let previous = score;
            
            // Apply decay based on time since last update
            let hours_since_update = (Utc::now() - previous.last_updated).num_hours() as f64;
            if hours_since_update > 24.0 {
                let decay = decay_rate * (hours_since_update / 24.0).min(1.0);
                let mut new_score = previous;
                new_score.value = new_score.value - (new_score.value * decay);
                new_score.last_updated = Utc::now();
                
                let update = TrustUpdate {
                    agent_id,
                    previous,
                    current: new_score,
                    reason: TrustUpdateReason::TimeDecay,
                    timestamp: Utc::now(),
                };
                
                updates.push((agent_id, new_score, update));
            }
        }
        
        // Apply all updates in a transaction
        if !updates.is_empty() {
            let mut tx = self.storage.begin_transaction().await?;
            
            for (agent_id, new_score, update) in &updates {
                tx.store_trust_score(*agent_id, *new_score).await?;
                tx.store_trust_update(update).await?;
            }
            
            tx.commit().await?;
            
            // Update cache
            for (agent_id, new_score, _) in updates {
                self.cache.insert(agent_id, new_score);
            }
            
            debug!("Applied time decay to {} agents", self.cache.len());
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