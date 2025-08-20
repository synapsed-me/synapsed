//! Core types for CRDT implementations

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    hash::{Hash as StdHash, Hasher},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

/// Unique actor identifier for CRDT operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ActorId(Uuid);

impl ActorId {
    /// Create a new random actor ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    
    /// Create actor ID from UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
    
    /// Get the inner UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
    
    /// Convert to string representation
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for ActorId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for ActorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ActorId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<&str> for ActorId {
    fn from(s: &str) -> Self {
        Self(Uuid::parse_str(s).unwrap_or_else(|_| Uuid::new_v4()))
    }
}

impl StdHash for ActorId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Logical timestamp for ordering operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Create timestamp from current system time
    pub fn now() -> Self {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self(duration.as_millis() as u64)
    }
    
    /// Create timestamp from value
    pub fn from_millis(millis: u64) -> Self {
        Self(millis)
    }
    
    /// Get timestamp value
    pub fn as_millis(&self) -> u64 {
        self.0
    }
    
    /// Increment timestamp
    pub fn increment(&mut self) {
        self.0 += 1;
    }
    
    /// Get next timestamp
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdHash for Timestamp {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Vector clock for causal ordering
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorClock {
    clocks: HashMap<ActorId, u64>,
}

impl VectorClock {
    /// Create new vector clock
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }
    
    /// Advance clock for an actor
    pub fn advance(&mut self, actor: &ActorId) {
        let counter = self.clocks.entry(actor.clone()).or_insert(0);
        *counter += 1;
    }
    
    /// Get clock value for an actor
    pub fn get(&self, actor: &ActorId) -> u64 {
        self.clocks.get(actor).copied().unwrap_or(0)
    }
    
    /// Set clock value for an actor
    pub fn set(&mut self, actor: ActorId, value: u64) {
        self.clocks.insert(actor, value);
    }
    
    /// Merge with another vector clock (take maximum of each clock)
    pub fn merge(&mut self, other: &VectorClock) {
        for (actor, &timestamp) in &other.clocks {
            let current = self.clocks.entry(actor.clone()).or_insert(0);
            *current = (*current).max(timestamp);
        }
    }
    
    /// Compare with another vector clock
    pub fn compare(&self, other: &VectorClock) -> VectorClockComparison {
        let mut less_than = false;
        let mut greater_than = false;
        
        // Get all actors from both clocks
        let mut all_actors = std::collections::HashSet::new();
        all_actors.extend(self.clocks.keys());
        all_actors.extend(other.clocks.keys());
        
        for actor in all_actors {
            let self_time = self.get(actor);
            let other_time = other.get(actor);
            
            if self_time < other_time {
                less_than = true;
            } else if self_time > other_time {
                greater_than = true;
            }
        }
        
        match (less_than, greater_than) {
            (false, false) => VectorClockComparison::Equal,
            (true, false) => VectorClockComparison::Before,
            (false, true) => VectorClockComparison::After,
            (true, true) => VectorClockComparison::Concurrent,
        }
    }
    
    /// Check if this clock happened before another
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        matches!(self.compare(other), VectorClockComparison::Before)
    }
    
    /// Check if clocks are concurrent
    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        matches!(self.compare(other), VectorClockComparison::Concurrent)
    }
    
    /// Get all actors in this vector clock
    pub fn actors(&self) -> impl Iterator<Item = &ActorId> {
        self.clocks.keys()
    }
}

impl Default for VectorClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Vector clock comparison result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorClockComparison {
    Before,
    After,
    Equal,
    Concurrent,
}

/// Hybrid Logical Clock for message ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HybridLogicalClock {
    /// Logical time component
    pub logical_time: u64,
    /// Physical time component (milliseconds since epoch)
    pub physical_time: u64,
    /// Replica identifier
    pub replica_id: ActorId,
}

impl HybridLogicalClock {
    /// Create new HLC
    pub fn new(replica_id: ActorId) -> Self {
        let physical_time = Timestamp::now().as_millis();
        Self {
            logical_time: physical_time,
            physical_time,
            replica_id,
        }
    }
    
    /// Create a copy of this HLC
    pub fn copy(&self) -> Self {
        *self
    }
    
    /// Advance local clock
    pub fn advance_local(&mut self) -> Self {
        let current_physical = Timestamp::now().as_millis();
        
        if current_physical > self.physical_time {
            self.logical_time = current_physical;
            self.physical_time = current_physical;
        } else {
            self.logical_time += 1;
        }
        
        *self
    }
    
    /// Advance clock based on remote clock
    pub fn advance_remote(&mut self, remote: &HybridLogicalClock) -> Self {
        let current_physical = Timestamp::now().as_millis();
        let max_logical = self.logical_time.max(remote.logical_time);
        
        if current_physical > max_logical {
            self.logical_time = current_physical;
            self.physical_time = current_physical;
        } else {
            self.logical_time = max_logical + 1;
        }
        
        *self
    }
    
    /// Compare HLC values for ordering
    pub fn compare(&self, other: &HybridLogicalClock) -> std::cmp::Ordering {
        self.logical_time
            .cmp(&other.logical_time)
            .then_with(|| self.replica_id.cmp(&other.replica_id))
    }
}

impl StdHash for HybridLogicalClock {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.logical_time.hash(state);
        self.physical_time.hash(state);
        self.replica_id.hash(state);
    }
}

/// Global ID for RGA nodes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalId {
    /// Monotonic counter per replica
    pub counter: u64,
    /// Unique replica identifier
    pub replica_id: ActorId,
    /// Sequence number for this counter
    pub sequence: u64,
}

impl GlobalId {
    /// Create new global ID
    pub fn new(counter: u64, replica_id: ActorId, sequence: u64) -> Self {
        Self {
            counter,
            replica_id,
            sequence,
        }
    }
}

impl PartialOrd for GlobalId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GlobalId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.counter
            .cmp(&other.counter)
            .then_with(|| self.replica_id.cmp(&other.replica_id))
            .then_with(|| self.sequence.cmp(&other.sequence))
    }
}

impl StdHash for GlobalId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.counter.hash(state);
        self.replica_id.hash(state);
        self.sequence.hash(state);
    }
}

/// Delta for incremental updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Delta<T> {
    /// Full state replacement
    FullState(T),
    /// Incremental operation
    Operation(Vec<u8>),
    /// Multiple operations
    Batch(Vec<Vec<u8>>),
}

/// Unique identifier for files in Merkle sync
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileId(pub Uuid);

impl FileId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for FileId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdHash for FileId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Cryptographic hash type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    /// Create zero hash
    pub fn zero() -> Self {
        Self([0u8; 32])
    }
    
    /// Create hash from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    
    /// Get hash bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Simple hex encoding without external dependency
        let hex_chars: Vec<String> = self.0.iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        write!(f, "{}", hex_chars.join(""))
    }
}

impl StdHash for Hash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Thread ID for message organization
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadId(pub Uuid);

impl ThreadId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ThreadId {
    fn default() -> Self {
        Self::new()
    }
}

impl StdHash for ThreadId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Message ID for chat messages
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageId {
    pub hlc: HybridLogicalClock,
    pub author: ActorId,
    pub nonce: u64,
}

impl MessageId {
    pub fn new(hlc: HybridLogicalClock, author: ActorId, nonce: u64) -> Self {
        Self { hlc, author, nonce }
    }
}

impl StdHash for MessageId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hlc.hash(state);
        self.author.hash(state);
        self.nonce.hash(state);
    }
}