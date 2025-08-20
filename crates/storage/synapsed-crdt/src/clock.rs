//! Clock implementations for CRDT ordering

use crate::{
    types::{ActorId, HybridLogicalClock, Timestamp, VectorClock, VectorClockComparison},
};
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Clock manager for coordinating logical clocks across CRDTs
#[derive(Debug)]
pub struct ClockManager {
    actor_id: ActorId,
    vector_clock: Arc<RwLock<VectorClock>>,
    hlc: Arc<RwLock<HybridLogicalClock>>,
}

impl ClockManager {
    /// Create new clock manager
    pub fn new(actor_id: ActorId) -> Self {
        let hlc = HybridLogicalClock::new(actor_id.clone());
        
        Self {
            actor_id: actor_id.clone(),
            vector_clock: Arc::new(RwLock::new(VectorClock::new())),
            hlc: Arc::new(RwLock::new(hlc)),
        }
    }
    
    /// Get current actor ID
    pub fn actor_id(&self) -> &ActorId {
        &self.actor_id
    }
    
    /// Advance local vector clock
    pub fn advance_vector_clock(&self) -> VectorClock {
        let mut clock = self.vector_clock.write();
        clock.advance(&self.actor_id);
        clock.clone()
    }
    
    /// Get current vector clock
    pub fn vector_clock(&self) -> VectorClock {
        self.vector_clock.read().clone()
    }
    
    /// Merge with remote vector clock
    pub fn merge_vector_clock(&self, remote_clock: &VectorClock) {
        let mut clock = self.vector_clock.write();
        clock.merge(remote_clock);
    }
    
    /// Advance local HLC
    pub fn advance_hlc(&self) -> HybridLogicalClock {
        let mut hlc = self.hlc.write();
        hlc.advance_local()
    }
    
    /// Get current HLC
    pub fn hlc(&self) -> HybridLogicalClock {
        self.hlc.read().clone()
    }
    
    /// Advance HLC based on remote clock
    pub fn advance_hlc_remote(&self, remote_hlc: &HybridLogicalClock) -> HybridLogicalClock {
        let mut hlc = self.hlc.write();
        hlc.advance_remote(remote_hlc)
    }
    
    /// Create timestamp for operation
    pub fn create_timestamp(&self) -> Timestamp {
        Timestamp::now()
    }
}

impl Clone for ClockManager {
    fn clone(&self) -> Self {
        Self {
            actor_id: self.actor_id.clone(),
            vector_clock: Arc::clone(&self.vector_clock),
            hlc: Arc::clone(&self.hlc),
        }
    }
}

/// Utility functions for clock operations
pub mod utils {
    use super::*;
    
    /// Compare two vector clocks and return the relationship
    pub fn compare_vector_clocks(a: &VectorClock, b: &VectorClock) -> VectorClockOrdering {
        match a.compare(b) {
            VectorClockComparison::Before => VectorClockOrdering::Before,
            VectorClockComparison::After => VectorClockOrdering::After,
            VectorClockComparison::Equal => VectorClockOrdering::Equal,
            VectorClockComparison::Concurrent => VectorClockOrdering::Concurrent,
        }
    }
    
    /// Check if operation is causally ready for delivery
    pub fn is_causally_ready(
        operation_clock: &VectorClock,
        local_clock: &VectorClock,
        operation_actor: &ActorId,
    ) -> bool {
        // Check if all dependencies are satisfied
        for actor in operation_clock.actors() {
            if actor == operation_actor {
                // For the operation's actor, we need exactly the next logical time
                let expected = local_clock.get(actor) + 1;
                if operation_clock.get(actor) != expected {
                    return false;
                }
            } else {
                // For other actors, we need to have seen at least this time
                if local_clock.get(actor) < operation_clock.get(actor) {
                    return false;
                }
            }
        }
        true
    }
    
    /// Calculate the drift between physical and logical time in HLC
    pub fn calculate_hlc_drift(hlc: &HybridLogicalClock) -> i64 {
        let current_physical = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        hlc.logical_time as i64 - current_physical as i64
    }
    
    /// Check if HLC drift is within acceptable bounds
    pub fn is_hlc_drift_acceptable(hlc: &HybridLogicalClock, max_drift_ms: u64) -> bool {
        let drift = calculate_hlc_drift(hlc).abs() as u64;
        drift <= max_drift_ms
    }
}

/// Vector clock ordering relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorClockOrdering {
    Before,
    After,
    Equal,
    Concurrent,
}

/// Clock synchronization service for distributed systems
#[derive(Debug)]
pub struct ClockSyncService {
    local_manager: ClockManager,
    peer_clocks: Arc<RwLock<HashMap<ActorId, VectorClock>>>,
    peer_hlcs: Arc<RwLock<HashMap<ActorId, HybridLogicalClock>>>,
}

impl ClockSyncService {
    /// Create new clock sync service
    pub fn new(actor_id: ActorId) -> Self {
        Self {
            local_manager: ClockManager::new(actor_id),
            peer_clocks: Arc::new(RwLock::new(HashMap::new())),
            peer_hlcs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Update peer's vector clock
    pub fn update_peer_vector_clock(&self, peer_id: ActorId, clock: VectorClock) {
        let mut peer_clocks = self.peer_clocks.write();
        peer_clocks.insert(peer_id, clock.clone());
        
        // Merge with local clock
        self.local_manager.merge_vector_clock(&clock);
    }
    
    /// Update peer's HLC
    pub fn update_peer_hlc(&self, peer_id: ActorId, hlc: HybridLogicalClock) {
        let mut peer_hlcs = self.peer_hlcs.write();
        peer_hlcs.insert(peer_id, hlc);
        
        // Advance local HLC
        self.local_manager.advance_hlc_remote(&hlc);
    }
    
    /// Get local clock manager
    pub fn local_manager(&self) -> &ClockManager {
        &self.local_manager
    }
    
    /// Get peer vector clocks
    pub fn peer_vector_clocks(&self) -> HashMap<ActorId, VectorClock> {
        self.peer_clocks.read().clone()
    }
    
    /// Get peer HLCs
    pub fn peer_hlcs(&self) -> HashMap<ActorId, HybridLogicalClock> {
        self.peer_hlcs.read().clone()
    }
    
    /// Check if we have recent clock info from a peer
    pub fn has_recent_peer_clock(&self, peer_id: &ActorId, max_age: Duration) -> bool {
        if let Some(peer_hlc) = self.peer_hlcs.read().get(peer_id) {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            
            let age = current_time.saturating_sub(peer_hlc.physical_time);
            age <= max_age.as_millis() as u64
        } else {
            false
        }
    }
}