//! Core traits for CRDT implementations

use crate::{ActorId, Delta, Result, VectorClock};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Core CRDT trait that all CRDTs must implement
#[async_trait]
pub trait Crdt: Clone + Send + Sync {
    /// The type of operations this CRDT supports
    type Operation: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>;
    
    /// The type of the CRDT's state
    type State: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>;
    
    /// Apply a local operation to this CRDT
    async fn apply_operation(&mut self, operation: Self::Operation) -> Result<()>;
    
    /// Apply a remote operation to this CRDT
    async fn apply_remote_operation(&mut self, operation: Self::Operation) -> Result<()>;
    
    /// Get the current state of the CRDT
    fn state(&self) -> &Self::State;
    
    /// Get the actor ID for this replica
    fn actor_id(&self) -> &ActorId;
    
    /// Get the current vector clock
    fn vector_clock(&self) -> &VectorClock;
    
    /// Validate that an operation is applicable to current state
    fn validate_operation(&self, operation: &Self::Operation) -> Result<()>;
}

/// Trait for CRDTs that support merging with other replicas
#[async_trait]
pub trait Mergeable: Crdt {
    /// Merge this CRDT with another replica
    async fn merge(&mut self, other: &Self) -> Result<()>;
    
    /// Check if this CRDT can be merged with another
    fn can_merge(&self, other: &Self) -> bool;
    
    /// Get differences between this CRDT and another
    fn diff(&self, other: &Self) -> Vec<Self::Operation>;
}

/// Trait for CRDTs that support incremental synchronization
#[async_trait]
pub trait Synchronizable: Crdt {
    /// Get delta since a given vector clock
    fn delta_since(&self, clock: &VectorClock) -> Result<Delta<Self::State>>;
    
    /// Apply a delta to this CRDT
    async fn apply_delta(&mut self, delta: Delta<Self::State>) -> Result<()>;
    
    /// Get all operations since a given vector clock
    fn operations_since(&self, clock: &VectorClock) -> Vec<Self::Operation>;
    
    /// Get the size in bytes of this CRDT
    fn size_bytes(&self) -> usize;
}

/// Trait for CRDTs that support conflict resolution
pub trait ConflictResolvable: Crdt {
    /// The type of conflicts this CRDT can resolve
    type Conflict: Clone + Send + Sync;
    
    /// Detect conflicts between operations
    fn detect_conflicts(&self, operations: &[Self::Operation]) -> Vec<Self::Conflict>;
    
    /// Resolve a conflict automatically if possible
    fn resolve_conflict(&self, conflict: Self::Conflict) -> Result<Vec<Self::Operation>>;
    
    /// Check if an operation would cause a conflict
    fn would_conflict(&self, operation: &Self::Operation) -> bool;
}

/// Trait for CRDTs that support garbage collection
#[async_trait]
pub trait GarbageCollectable: Crdt {
    /// Perform garbage collection to remove unnecessary data
    async fn garbage_collect(&mut self) -> Result<usize>;
    
    /// Check if garbage collection is needed
    fn needs_gc(&self) -> bool;
    
    /// Get the amount of garbage data
    fn garbage_size(&self) -> usize;
}

/// Trait for CRDTs that support serialization for network transport
pub trait NetworkSerializable: Crdt {
    /// Serialize for network transmission
    fn serialize_for_network(&self) -> Result<Vec<u8>>;
    
    /// Deserialize from network data
    fn deserialize_from_network(data: &[u8]) -> Result<Self>;
    
    /// Get network protocol version
    fn protocol_version(&self) -> u32;
}

/// Trait for observing CRDT operations
pub trait Observable: Crdt {
    /// The type of events this CRDT emits
    type Event: Clone + Send + Sync;
    
    /// Register an observer for CRDT events
    fn observe<F>(&mut self, observer: F) 
    where 
        F: Fn(Self::Event) + Send + Sync + 'static;
    
    /// Emit an event to all observers
    fn emit_event(&self, event: Self::Event);
}

/// Trait for CRDTs with causal delivery requirements
pub trait CausallyOrdered: Crdt {
    /// Check if an operation can be delivered now
    fn can_deliver(&self, operation: &Self::Operation) -> bool;
    
    /// Get operations that are ready for delivery
    fn ready_operations(&self) -> Vec<Self::Operation>;
    
    /// Buffer an operation for later delivery
    fn buffer_operation(&mut self, operation: Self::Operation) -> Result<()>;
}

/// Trait for persistent storage of CRDT state
#[async_trait]
pub trait Persistent: Crdt {
    /// Save CRDT state to persistent storage
    async fn save(&self) -> Result<()>;
    
    /// Load CRDT state from persistent storage
    async fn load(actor_id: ActorId) -> Result<Self>;
    
    /// Get storage key for this CRDT
    fn storage_key(&self) -> String;
}