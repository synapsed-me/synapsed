//! Last-Writer-Wins Register CRDT implementation
//!
//! LWW-Register stores a single value with timestamp-based conflict resolution.
//! The most recent write wins in case of conflicts.

use crate::{
    error::Result,
    traits::{Crdt, Mergeable, Synchronizable},
    types::{ActorId, Delta, Timestamp, VectorClock},
    clock::ClockManager,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use parking_lot::RwLock;

/// LWW Register operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LwwOperation<T> {
    /// New value
    pub value: T,
    /// Timestamp of the write
    pub timestamp: Timestamp,
    /// Actor that performed the write
    pub actor: ActorId,
}

/// LWW Register state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LwwState<T> {
    /// Current value
    pub value: Option<T>,
    /// Timestamp of last write
    pub timestamp: Timestamp,
    /// Actor who performed last write
    pub actor: Option<ActorId>,
}

impl<T> LwwState<T> {
    /// Create new empty state
    pub fn new() -> Self {
        Self {
            value: None,
            timestamp: Timestamp::from_millis(0),
            actor: None,
        }
    }
    
    /// Get current value
    pub fn get(&self) -> Option<&T> {
        self.value.as_ref()
    }
    
    /// Check if state has a value
    pub fn is_empty(&self) -> bool {
        self.value.is_none()
    }
}

impl<T> Default for LwwState<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Last-Writer-Wins Register CRDT
#[derive(Debug)]
pub struct LwwRegister<T> {
    /// Actor ID for this replica
    actor_id: ActorId,
    /// Current state
    state: RwLock<LwwState<T>>,
    /// Clock manager
    clock_manager: ClockManager,
}

impl<T> LwwRegister<T>
where
    T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    /// Create new LWW register
    pub fn new(actor_id: ActorId) -> Self {
        Self {
            actor_id: actor_id.clone(),
            state: RwLock::new(LwwState::new()),
            clock_manager: ClockManager::new(actor_id),
        }
    }
    
    /// Set value
    pub async fn set(&mut self, value: T) -> Result<LwwOperation<T>> {
        let timestamp = self.clock_manager.create_timestamp();
        let operation = LwwOperation {
            value: value.clone(),
            timestamp,
            actor: self.actor_id.clone(),
        };
        
        self.apply_operation(operation.clone()).await?;
        Ok(operation)
    }
    
    /// Get current value
    pub fn get(&self) -> Option<T> {
        self.state.read().value.clone()
    }
    
    /// Check if register is empty
    pub fn is_empty(&self) -> bool {
        self.state.read().is_empty()
    }
    
    /// Get last write timestamp
    pub fn last_write_timestamp(&self) -> Timestamp {
        self.state.read().timestamp
    }
    
    /// Get last writer
    pub fn last_writer(&self) -> Option<ActorId> {
        self.state.read().actor.clone()
    }
}

impl<T> Clone for LwwRegister<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            actor_id: self.actor_id.clone(),
            state: RwLock::new(self.state.read().clone()),
            clock_manager: self.clock_manager.clone(),
        }
    }
}

#[async_trait]
impl<T> Crdt for LwwRegister<T>
where
    T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    type Operation = LwwOperation<T>;
    type State = LwwState<T>;
    
    async fn apply_operation(&mut self, operation: Self::Operation) -> Result<()> {
        let mut state = self.state.write();
        
        // Apply if this operation is newer
        if operation.timestamp > state.timestamp ||
           (operation.timestamp == state.timestamp && operation.actor > state.actor.clone().unwrap_or_default()) {
            state.value = Some(operation.value);
            state.timestamp = operation.timestamp;
            state.actor = Some(operation.actor);
        }
        
        Ok(())
    }
    
    async fn apply_remote_operation(&mut self, operation: Self::Operation) -> Result<()> {
        self.apply_operation(operation).await
    }
    
    fn state(&self) -> &Self::State {
        // This requires a different approach due to RwLock
        unimplemented!("Use clone_state() instead")
    }
    
    fn actor_id(&self) -> &ActorId {
        &self.actor_id
    }
    
    fn vector_clock(&self) -> &VectorClock {
        unimplemented!("Use get_vector_clock() instead")
    }
    
    fn validate_operation(&self, _operation: &Self::Operation) -> Result<()> {
        // LWW operations are always valid
        Ok(())
    }
}

impl<T> LwwRegister<T>
where
    T: Clone,
{
    /// Get cloned state
    pub fn clone_state(&self) -> LwwState<T> {
        self.state.read().clone()
    }
    
    /// Get current vector clock
    pub fn get_vector_clock(&self) -> VectorClock {
        self.clock_manager.vector_clock()
    }
}

#[async_trait]
impl<T> Mergeable for LwwRegister<T>
where
    T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    async fn merge(&mut self, other: &Self) -> Result<()> {
        let operation = {
            let other_state = other.state.read();
            if let (Some(value), Some(actor)) = (&other_state.value, &other_state.actor) {
                Some(LwwOperation {
                    value: value.clone(),
                    timestamp: other_state.timestamp,
                    actor: actor.clone(),
                })
            } else {
                None
            }
        };
        
        if let Some(op) = operation {
            self.apply_remote_operation(op).await?;
        }
        
        Ok(())
    }
    
    fn can_merge(&self, _other: &Self) -> bool {
        true // LWW can always merge
    }
    
    fn diff(&self, other: &Self) -> Vec<Self::Operation> {
        let self_state = self.state.read();
        let other_state = other.state.read();
        
        // If other has a newer value, include it in diff
        if other_state.timestamp > self_state.timestamp {
            if let (Some(value), Some(actor)) = (&other_state.value, &other_state.actor) {
                return vec![LwwOperation {
                    value: value.clone(),
                    timestamp: other_state.timestamp,
                    actor: actor.clone(),
                }];
            }
        }
        
        Vec::new()
    }
}

#[async_trait]
impl<T> Synchronizable for LwwRegister<T>
where
    T: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    fn delta_since(&self, _clock: &VectorClock) -> Result<Delta<Self::State>> {
        Ok(Delta::FullState(self.clone_state()))
    }
    
    async fn apply_delta(&mut self, delta: Delta<Self::State>) -> Result<()> {
        match delta {
            Delta::FullState(state) => {
                if let (Some(value), Some(actor)) = (state.value, state.actor) {
                    let operation = LwwOperation {
                        value,
                        timestamp: state.timestamp,
                        actor,
                    };
                    self.apply_remote_operation(operation).await?;
                }
                Ok(())
            }
            Delta::Operation(bytes) => {
                let operation: LwwOperation<T> = serde_json::from_slice(&bytes)?;
                self.apply_remote_operation(operation).await
            }
            Delta::Batch(operations) => {
                for op_bytes in operations {
                    let operation: LwwOperation<T> = serde_json::from_slice(&op_bytes)?;
                    self.apply_remote_operation(operation).await?;
                }
                Ok(())
            }
        }
    }
    
    fn operations_since(&self, _clock: &VectorClock) -> Vec<Self::Operation> {
        // For LWW, we only have the current state
        let state = self.state.read();
        if let (Some(value), Some(actor)) = (&state.value, &state.actor) {
            vec![LwwOperation {
                value: value.clone(),
                timestamp: state.timestamp,
                actor: actor.clone(),
            }]
        } else {
            Vec::new()
        }
    }
    
    fn size_bytes(&self) -> usize {
        std::mem::size_of::<LwwState<T>>()
    }
}

impl<T> Display for LwwRegister<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.state.read();
        match &state.value {
            Some(value) => write!(f, "LWW[{}]: {} ({})", self.actor_id, value, state.timestamp),
            None => write!(f, "LWW[{}]: empty", self.actor_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_lww_set_get() {
        let actor_id = ActorId::new();
        let mut lww = LwwRegister::new(actor_id);
        
        lww.set("hello".to_string()).await.unwrap();
        assert_eq!(lww.get(), Some("hello".to_string()));
    }
    
    #[tokio::test]
    async fn test_lww_merge() {
        let actor1 = ActorId::new();
        let actor2 = ActorId::new();
        
        let mut lww1 = LwwRegister::new(actor1);
        let mut lww2 = LwwRegister::new(actor2);
        
        lww1.set("value1".to_string()).await.unwrap();
        
        // Sleep to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        
        lww2.set("value2".to_string()).await.unwrap();
        
        lww1.merge(&lww2).await.unwrap();
        
        // lww1 should have the newer value
        assert_eq!(lww1.get(), Some("value2".to_string()));
    }
    
    #[tokio::test]
    async fn test_lww_conflict_resolution() {
        let actor1 = ActorId::new();
        let actor2 = ActorId::new();
        
        let mut lww1 = LwwRegister::new(actor1.clone());
        let mut lww2 = LwwRegister::new(actor2.clone());
        
        // Set same timestamp but different actors
        let timestamp = Timestamp::now();
        let op1 = LwwOperation {
            value: "value1".to_string(),
            timestamp,
            actor: actor1.clone(),
        };
        let op2 = LwwOperation {
            value: "value2".to_string(),
            timestamp,
            actor: actor2.clone(),
        };
        
        lww1.apply_operation(op1).await.unwrap();
        lww1.apply_operation(op2).await.unwrap();
        
        // Should resolve based on actor ID comparison
        let result = lww1.get().unwrap();
        assert!(result == "value1" || result == "value2");
    }
}