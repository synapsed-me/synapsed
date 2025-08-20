//! Synchronization utilities for CRDT coordination
//!
//! This module provides utilities for efficient CRDT synchronization,
//! including delta sync, merkle proofs, and conflict resolution.

use crate::{
    error::{CrdtError, Result},
    types::{ActorId, Delta, Hash, VectorClock, VectorClockComparison},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

/// Synchronization metadata for tracking sync state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    /// Last sync timestamp
    pub last_sync: SystemTime,
    /// Vector clock at last sync
    pub last_sync_clock: VectorClock,
    /// Sync session ID for tracking
    pub session_id: String,
    /// Peer we're syncing with
    pub peer_id: ActorId,
}

/// Sync request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    /// Requesting actor ID
    pub from: ActorId,
    /// Target actor ID
    pub to: ActorId,
    /// Current vector clock
    pub clock: VectorClock,
    /// Hash of current state
    pub state_hash: Hash,
    /// Session ID for tracking
    pub session_id: String,
}

/// Sync response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse<T> {
    /// Responding actor ID
    pub from: ActorId,
    /// Target actor ID
    pub to: ActorId,
    /// Delta since requested clock
    pub delta: Option<Delta<T>>,
    /// Operations to apply
    pub operations: Vec<Vec<u8>>,
    /// Updated vector clock
    pub clock: VectorClock,
    /// Session ID for tracking
    pub session_id: String,
}

/// Sync status for monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// Not currently syncing
    Idle,
    /// Sync request sent, waiting for response
    Pending,
    /// Receiving data from peer
    Receiving,
    /// Sending data to peer
    Sending,
    /// Sync completed successfully
    Completed,
    /// Sync failed with error
    Failed,
}

/// Sync coordinator for managing CRDT synchronization
#[derive(Debug)]
pub struct SyncCoordinator {
    /// This actor's ID
    actor_id: ActorId,
    /// Active sync sessions
    active_sessions: HashMap<String, SyncSession>,
    /// Peer sync metadata
    peer_metadata: HashMap<ActorId, SyncMetadata>,
    /// Sync statistics
    stats: SyncStatistics,
}

/// Individual sync session state
#[derive(Debug, Clone)]
pub struct SyncSession {
    /// Session ID
    pub id: String,
    /// Peer actor ID
    pub peer_id: ActorId,
    /// Current status
    pub status: SyncStatus,
    /// Start time
    pub started_at: SystemTime,
    /// Last activity time
    pub last_activity: SystemTime,
    /// Vector clock at start
    pub initial_clock: VectorClock,
    /// Current vector clock
    pub current_clock: VectorClock,
    /// Bytes sent
    pub bytes_sent: usize,
    /// Bytes received
    pub bytes_received: usize,
}

/// Synchronization statistics
#[derive(Debug, Clone, Default)]
pub struct SyncStatistics {
    /// Total sync sessions
    pub total_sessions: u64,
    /// Successful syncs
    pub successful_syncs: u64,
    /// Failed syncs
    pub failed_syncs: u64,
    /// Total bytes sent
    pub total_bytes_sent: u64,
    /// Total bytes received
    pub total_bytes_received: u64,
    /// Average sync duration
    pub avg_sync_duration: Duration,
}

impl SyncCoordinator {
    /// Create new sync coordinator
    pub fn new(actor_id: ActorId) -> Self {
        Self {
            actor_id,
            active_sessions: HashMap::new(),
            peer_metadata: HashMap::new(),
            stats: SyncStatistics::default(),
        }
    }
    
    /// Start sync with peer
    pub fn start_sync(&mut self, peer_id: ActorId, current_clock: VectorClock) -> Result<SyncRequest> {
        let session_id = format!("sync-{}-{}", self.actor_id.as_str(), uuid::Uuid::new_v4());
        
        let session = SyncSession {
            id: session_id.clone(),
            peer_id: peer_id.clone(),
            status: SyncStatus::Pending,
            started_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            initial_clock: current_clock.clone(),
            current_clock: current_clock.clone(),
            bytes_sent: 0,
            bytes_received: 0,
        };
        
        self.active_sessions.insert(session_id.clone(), session);
        self.stats.total_sessions += 1;
        
        Ok(SyncRequest {
            from: self.actor_id.clone(),
            to: peer_id,
            clock: current_clock,
            state_hash: Hash::zero(), // Would compute actual hash
            session_id,
        })
    }
    
    /// Handle incoming sync request
    pub fn handle_sync_request<T>(
        &mut self,
        request: SyncRequest,
        delta_provider: impl Fn(&VectorClock) -> Result<Delta<T>>,
    ) -> Result<SyncResponse<T>> {
        let delta = delta_provider(&request.clock)?;
        
        // Update peer metadata
        let metadata = SyncMetadata {
            last_sync: SystemTime::now(),
            last_sync_clock: request.clock.clone(),
            session_id: request.session_id.clone(),
            peer_id: request.from.clone(),
        };
        self.peer_metadata.insert(request.from.clone(), metadata);
        
        Ok(SyncResponse {
            from: self.actor_id.clone(),
            to: request.from,
            delta: Some(delta),
            operations: Vec::new(), // Would include relevant operations
            clock: request.clock, // Would update with current clock
            session_id: request.session_id,
        })
    }
    
    /// Handle sync response
    pub fn handle_sync_response<T>(&mut self, response: SyncResponse<T>) -> Result<()> {
        let response_size = self.estimate_response_size(&response);
        
        if let Some(session) = self.active_sessions.get_mut(&response.session_id) {
            session.status = SyncStatus::Receiving;
            session.last_activity = SystemTime::now();
            session.bytes_received += response_size;
            session.current_clock.merge(&response.clock);
        }
        
        Ok(())
    }
    
    /// Complete sync session
    pub fn complete_sync(&mut self, session_id: &str) -> Result<()> {
        if let Some(mut session) = self.active_sessions.remove(session_id) {
            session.status = SyncStatus::Completed;
            
            let duration = session.started_at.elapsed().unwrap_or_default();
            self.update_stats(&session, duration, true);
        }
        
        Ok(())
    }
    
    /// Fail sync session
    pub fn fail_sync(&mut self, session_id: &str, _error: CrdtError) -> Result<()> {
        if let Some(mut session) = self.active_sessions.remove(session_id) {
            session.status = SyncStatus::Failed;
            
            let duration = session.started_at.elapsed().unwrap_or_default();
            self.update_stats(&session, duration, false);
        }
        
        Ok(())
    }
    
    /// Get active sessions
    pub fn active_sessions(&self) -> &HashMap<String, SyncSession> {
        &self.active_sessions
    }
    
    /// Get sync statistics
    pub fn statistics(&self) -> &SyncStatistics {
        &self.stats
    }
    
    /// Clean up expired sessions
    pub fn cleanup_expired_sessions(&mut self, timeout: Duration) {
        let now = SystemTime::now();
        let mut expired_sessions = Vec::new();
        
        for (session_id, session) in &self.active_sessions {
            if let Ok(elapsed) = now.duration_since(session.last_activity) {
                if elapsed > timeout {
                    expired_sessions.push(session_id.clone());
                }
            }
        }
        
        for session_id in expired_sessions {
            self.active_sessions.remove(&session_id);
            self.stats.failed_syncs += 1;
        }
    }
    
    /// Get peer metadata
    pub fn peer_metadata(&self, peer_id: &ActorId) -> Option<&SyncMetadata> {
        self.peer_metadata.get(peer_id)
    }
    
    /// Check if sync is needed with peer
    pub fn needs_sync(&self, peer_id: &ActorId, current_clock: &VectorClock) -> bool {
        if let Some(metadata) = self.peer_metadata.get(peer_id) {
            // Check if we have new operations since last sync
            !current_clock.happens_before(&metadata.last_sync_clock) &&
            current_clock != &metadata.last_sync_clock
        } else {
            // Never synced with this peer
            true
        }
    }
    
    /// Estimate response size (simplified)
    fn estimate_response_size<T>(&self, response: &SyncResponse<T>) -> usize {
        // Rough estimate based on operations count
        response.operations.len() * 64 + 256 // Base overhead
    }
    
    /// Update sync statistics
    fn update_stats(&mut self, session: &SyncSession, duration: Duration, success: bool) {
        if success {
            self.stats.successful_syncs += 1;
        } else {
            self.stats.failed_syncs += 1;
        }
        
        self.stats.total_bytes_sent += session.bytes_sent as u64;
        self.stats.total_bytes_received += session.bytes_received as u64;
        
        // Update average duration (simple moving average)
        let total_sessions = self.stats.successful_syncs + self.stats.failed_syncs;
        if total_sessions > 0 {
            let current_avg = self.stats.avg_sync_duration.as_millis() as u64;
            let new_duration = duration.as_millis() as u64;
            let new_avg = (current_avg * (total_sessions - 1) + new_duration) / total_sessions;
            self.stats.avg_sync_duration = Duration::from_millis(new_avg);
        }
    }
}

/// Conflict detection and resolution utilities
pub mod conflict_resolution {
    use super::*;
    
    /// Conflict detection result
    #[derive(Debug, Clone)]
    pub enum ConflictType {
        /// No conflict detected
        None,
        /// Concurrent modifications to same data
        ConcurrentModification,
        /// Causal dependency violation
        CausalViolation,
        /// Incompatible operations
        IncompatibleOperations,
    }
    
    /// Detect conflicts between vector clocks
    pub fn detect_clock_conflicts(local: &VectorClock, remote: &VectorClock) -> ConflictType {
        match local.compare(remote) {
            VectorClockComparison::Concurrent => ConflictType::ConcurrentModification,
            _ => ConflictType::None,
        }
    }
    
    /// Resolve conflicts using a strategy
    pub fn resolve_conflicts<T>(
        local_ops: Vec<T>,
        remote_ops: Vec<T>,
        strategy: ConflictResolutionStrategy,
    ) -> Vec<T>
    where
        T: Clone,
    {
        match strategy {
            ConflictResolutionStrategy::LocalWins => local_ops,
            ConflictResolutionStrategy::RemoteWins => remote_ops,
            ConflictResolutionStrategy::Merge => {
                let mut result = local_ops;
                result.extend(remote_ops);
                result
            }
        }
    }
    
    /// Conflict resolution strategy
    #[derive(Debug, Clone, Copy)]
    pub enum ConflictResolutionStrategy {
        /// Local operations take precedence
        LocalWins,
        /// Remote operations take precedence
        RemoteWins,
        /// Merge both sets of operations
        Merge,
    }
}

/// Delta compression utilities
pub mod delta_compression {
    use super::*;
    use std::collections::VecDeque;
    
    /// Delta history for efficient synchronization
    #[derive(Debug, Clone)]
    pub struct DeltaHistory<T> {
        /// Historical deltas
        deltas: VecDeque<(VectorClock, Delta<T>)>,
        /// Maximum history size
        max_size: usize,
    }
    
    impl<T> DeltaHistory<T>
    where
        T: Clone,
    {
        /// Create new delta history
        pub fn new(max_size: usize) -> Self {
            Self {
                deltas: VecDeque::new(),
                max_size,
            }
        }
        
        /// Add delta to history
        pub fn add_delta(&mut self, clock: VectorClock, delta: Delta<T>) {
            self.deltas.push_back((clock, delta));
            
            // Maintain size limit
            if self.deltas.len() > self.max_size {
                self.deltas.pop_front();
            }
        }
        
        /// Get delta since a specific vector clock
        pub fn delta_since(&self, since: &VectorClock) -> Option<Delta<T>> {
            // Find all deltas after the given clock
            let relevant_deltas: Vec<_> = self.deltas
                .iter()
                .filter(|(clock, _)| !clock.happens_before(since))
                .map(|(_, delta)| delta)
                .collect();
            
            if relevant_deltas.is_empty() {
                None
            } else if relevant_deltas.len() == 1 {
                Some(relevant_deltas[0].clone())
            } else {
                // Combine multiple deltas
                let operations: Vec<Vec<u8>> = relevant_deltas
                    .into_iter()
                    .filter_map(|delta| match delta {
                        Delta::Operation(bytes) => Some(bytes.clone()),
                        Delta::Batch(ops) => Some(ops.clone()).map(|ops| ops.into_iter().flatten().collect()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                
                Some(Delta::Batch(operations))
            }
        }
        
        /// Get history size
        pub fn len(&self) -> usize {
            self.deltas.len()
        }
        
        /// Check if history is empty
        pub fn is_empty(&self) -> bool {
            self.deltas.is_empty()
        }
        
        /// Clear history
        pub fn clear(&mut self) {
            self.deltas.clear();
        }
    }
}

/// Bandwidth optimization utilities
pub mod bandwidth {
    use super::*;
    
    /// Bandwidth limiter for sync operations
    #[derive(Debug)]
    pub struct BandwidthLimiter {
        /// Maximum bytes per second
        max_bps: u64,
        /// Current usage tracking
        current_usage: u64,
        /// Usage reset time
        last_reset: SystemTime,
    }
    
    impl BandwidthLimiter {
        /// Create new bandwidth limiter
        pub fn new(max_bps: u64) -> Self {
            Self {
                max_bps,
                current_usage: 0,
                last_reset: SystemTime::now(),
            }
        }
        
        /// Check if we can send data of given size
        pub fn can_send(&mut self, bytes: u64) -> bool {
            self.reset_if_needed();
            self.current_usage + bytes <= self.max_bps
        }
        
        /// Record data sent
        pub fn record_sent(&mut self, bytes: u64) {
            self.reset_if_needed();
            self.current_usage += bytes;
        }
        
        /// Reset usage counter if a second has passed
        fn reset_if_needed(&mut self) {
            if let Ok(elapsed) = self.last_reset.elapsed() {
                if elapsed >= Duration::from_secs(1) {
                    self.current_usage = 0;
                    self.last_reset = SystemTime::now();
                }
            }
        }
        
        /// Get remaining bandwidth
        pub fn remaining_bandwidth(&mut self) -> u64 {
            self.reset_if_needed();
            self.max_bps.saturating_sub(self.current_usage)
        }
    }
    
    /// Adaptive sync scheduler
    #[derive(Debug)]
    pub struct AdaptiveSyncScheduler {
        /// Peer priorities
        peer_priorities: HashMap<ActorId, u8>,
        /// Last sync times
        last_sync_times: HashMap<ActorId, SystemTime>,
        /// Sync intervals based on priority
        base_interval: Duration,
    }
    
    impl AdaptiveSyncScheduler {
        /// Create new adaptive scheduler
        pub fn new(base_interval: Duration) -> Self {
            Self {
                peer_priorities: HashMap::new(),
                last_sync_times: HashMap::new(),
                base_interval,
            }
        }
        
        /// Set peer priority (0 = highest, 255 = lowest)
        pub fn set_peer_priority(&mut self, peer_id: ActorId, priority: u8) {
            self.peer_priorities.insert(peer_id, priority);
        }
        
        /// Check if sync is due for peer
        pub fn is_sync_due(&self, peer_id: &ActorId) -> bool {
            let priority = self.peer_priorities.get(peer_id).copied().unwrap_or(128);
            let interval = self.base_interval.mul_f64(1.0 + priority as f64 / 128.0);
            
            if let Some(last_sync) = self.last_sync_times.get(peer_id) {
                last_sync.elapsed().unwrap_or_default() >= interval
            } else {
                true // Never synced
            }
        }
        
        /// Record sync completion
        pub fn record_sync(&mut self, peer_id: ActorId) {
            self.last_sync_times.insert(peer_id, SystemTime::now());
        }
        
        /// Get next peer to sync
        pub fn next_peer_to_sync(&self) -> Option<ActorId> {
            let mut candidates: Vec<_> = self.peer_priorities
                .keys()
                .filter(|peer_id| self.is_sync_due(peer_id))
                .collect();
            
            // Sort by priority (lower number = higher priority)
            candidates.sort_by_key(|peer_id| self.peer_priorities.get(peer_id).unwrap_or(&255));
            
            candidates.first().map(|&peer_id| *peer_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sync_coordinator_creation() {
        let actor_id = ActorId::new();
        let coordinator = SyncCoordinator::new(actor_id.clone());
        
        assert_eq!(coordinator.actor_id, actor_id);
        assert!(coordinator.active_sessions.is_empty());
        assert!(coordinator.peer_metadata.is_empty());
    }
    
    #[test]
    fn test_sync_request_creation() {
        let actor_id = ActorId::new();
        let peer_id = ActorId::new();
        let mut coordinator = SyncCoordinator::new(actor_id);
        
        let clock = VectorClock::new();
        let request = coordinator.start_sync(peer_id.clone(), clock).unwrap();
        
        assert_eq!(request.from, actor_id);
        assert_eq!(request.to, peer_id);
        assert_eq!(coordinator.active_sessions.len(), 1);
    }
    
    #[test]
    fn test_bandwidth_limiter() {
        let mut limiter = bandwidth::BandwidthLimiter::new(1000); // 1000 bytes per second
        
        assert!(limiter.can_send(500));
        limiter.record_sent(500);
        
        assert!(limiter.can_send(400));
        assert!(!limiter.can_send(600));
        
        limiter.record_sent(400);
        assert!(!limiter.can_send(200));
    }
}