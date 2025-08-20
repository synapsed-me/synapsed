//! PN-Counter (Increment/Decrement Counter) CRDT implementation
//!
//! PN-Counter allows both increment and decrement operations on a distributed counter.
//! It maintains separate P (positive) and N (negative) counters internally.

use crate::{
    error::{CrdtError, Result},
    traits::{Crdt, Mergeable, Synchronizable},
    types::{ActorId, Delta, VectorClock},
    clock::ClockManager,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Display},
};
use parking_lot::RwLock;

/// PN-Counter operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PnCounterOperation {
    /// Increment counter by amount
    Increment {
        actor: ActorId,
        amount: u64,
    },
    /// Decrement counter by amount
    Decrement {
        actor: ActorId,
        amount: u64,
    },
}

/// PN-Counter state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PnCounterState {
    /// Positive counters per actor
    pub positive: HashMap<ActorId, u64>,
    /// Negative counters per actor
    pub negative: HashMap<ActorId, u64>,
}

impl PnCounterState {
    /// Create new empty state
    pub fn new() -> Self {
        Self {
            positive: HashMap::new(),
            negative: HashMap::new(),
        }
    }
    
    /// Get current counter value
    pub fn value(&self) -> i64 {
        let positive_sum: u64 = self.positive.values().sum();
        let negative_sum: u64 = self.negative.values().sum();
        positive_sum as i64 - negative_sum as i64
    }
    
    /// Get positive sum
    pub fn positive_sum(&self) -> u64 {
        self.positive.values().sum()
    }
    
    /// Get negative sum
    pub fn negative_sum(&self) -> u64 {
        self.negative.values().sum()
    }
    
    /// Get positive counter for actor
    pub fn get_positive(&self, actor: &ActorId) -> u64 {
        self.positive.get(actor).copied().unwrap_or(0)
    }
    
    /// Get negative counter for actor
    pub fn get_negative(&self, actor: &ActorId) -> u64 {
        self.negative.get(actor).copied().unwrap_or(0)
    }
    
    /// Get all actors
    pub fn actors(&self) -> std::collections::HashSet<ActorId> {
        let mut actors = std::collections::HashSet::new();
        actors.extend(self.positive.keys().cloned());
        actors.extend(self.negative.keys().cloned());
        actors
    }
}

impl Default for PnCounterState {
    fn default() -> Self {
        Self::new()
    }
}

/// PN-Counter CRDT
#[derive(Debug)]
pub struct PnCounter {
    /// Actor ID for this replica
    actor_id: ActorId,
    /// Current state
    state: RwLock<PnCounterState>,
    /// Clock manager
    clock_manager: ClockManager,
}

impl PnCounter {
    /// Create new PN-Counter
    pub fn new(actor_id: ActorId) -> Self {
        Self {
            actor_id: actor_id.clone(),
            state: RwLock::new(PnCounterState::new()),
            clock_manager: ClockManager::new(actor_id),
        }
    }
    
    /// Increment counter
    pub async fn increment(&mut self, amount: u64) -> Result<PnCounterOperation> {
        let operation = PnCounterOperation::Increment {
            actor: self.actor_id.clone(),
            amount,
        };
        
        self.apply_operation(operation.clone()).await?;
        Ok(operation)
    }
    
    /// Increment by 1
    pub async fn inc(&mut self) -> Result<PnCounterOperation> {
        self.increment(1).await
    }
    
    /// Decrement counter
    pub async fn decrement(&mut self, amount: u64) -> Result<PnCounterOperation> {
        let operation = PnCounterOperation::Decrement {
            actor: self.actor_id.clone(),
            amount,
        };
        
        self.apply_operation(operation.clone()).await?;
        Ok(operation)
    }
    
    /// Decrement by 1
    pub async fn dec(&mut self) -> Result<PnCounterOperation> {
        self.decrement(1).await
    }
    
    /// Get current counter value
    pub fn value(&self) -> i64 {
        self.state.read().value()
    }
    
    /// Get positive sum
    pub fn positive_sum(&self) -> u64 {
        self.state.read().positive_sum()
    }
    
    /// Get negative sum
    pub fn negative_sum(&self) -> u64 {
        self.state.read().negative_sum()
    }
    
    /// Get positive counter for an actor
    pub fn get_positive(&self, actor: &ActorId) -> u64 {
        self.state.read().get_positive(actor)
    }
    
    /// Get negative counter for an actor
    pub fn get_negative(&self, actor: &ActorId) -> u64 {
        self.state.read().get_negative(actor)
    }
    
    /// Apply increment operation
    async fn apply_increment(&mut self, actor: ActorId, amount: u64) -> Result<()> {
        let mut state = self.state.write();
        let current = state.positive.get(&actor).copied().unwrap_or(0);
        state.positive.insert(actor, current + amount);
        Ok(())
    }
    
    /// Apply decrement operation
    async fn apply_decrement(&mut self, actor: ActorId, amount: u64) -> Result<()> {
        let mut state = self.state.write();
        let current = state.negative.get(&actor).copied().unwrap_or(0);
        state.negative.insert(actor, current + amount);
        Ok(())
    }
}

impl Clone for PnCounter {
    fn clone(&self) -> Self {
        Self {
            actor_id: self.actor_id.clone(),
            state: RwLock::new(self.state.read().clone()),
            clock_manager: self.clock_manager.clone(),
        }
    }
}

#[async_trait]
impl Crdt for PnCounter {
    type Operation = PnCounterOperation;
    type State = PnCounterState;
    
    async fn apply_operation(&mut self, operation: Self::Operation) -> Result<()> {
        match operation {
            PnCounterOperation::Increment { actor, amount } => {
                self.apply_increment(actor, amount).await
            }
            PnCounterOperation::Decrement { actor, amount } => {
                self.apply_decrement(actor, amount).await
            }
        }
    }
    
    async fn apply_remote_operation(&mut self, operation: Self::Operation) -> Result<()> {
        self.apply_operation(operation).await
    }
    
    fn state(&self) -> &Self::State {
        unimplemented!("Use clone_state() instead")
    }
    
    fn actor_id(&self) -> &ActorId {
        &self.actor_id
    }
    
    fn vector_clock(&self) -> &VectorClock {
        unimplemented!("Use get_vector_clock() instead")
    }
    
    fn validate_operation(&self, operation: &Self::Operation) -> Result<()> {
        match operation {
            PnCounterOperation::Increment { amount, .. } | 
            PnCounterOperation::Decrement { amount, .. } => {
                if *amount == 0 {
                    Err(CrdtError::InvalidOperation(
                        "Counter operation amount must be greater than 0".to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl PnCounter {
    /// Get cloned state
    pub fn clone_state(&self) -> PnCounterState {
        self.state.read().clone()
    }
    
    /// Get current vector clock
    pub fn get_vector_clock(&self) -> VectorClock {
        self.clock_manager.vector_clock()
    }
}

#[async_trait]
impl Mergeable for PnCounter {
    async fn merge(&mut self, other: &Self) -> Result<()> {
        let operations = {
            let other_state = other.state.read();
            let mut ops = Vec::new();
            
            // Merge positive counters
            for (actor, &other_count) in &other_state.positive {
                let self_count = self.get_positive(actor);
                if other_count > self_count {
                    ops.push(PnCounterOperation::Increment {
                        actor: actor.clone(),
                        amount: other_count - self_count,
                    });
                }
            }
            
            // Merge negative counters
            for (actor, &other_count) in &other_state.negative {
                let self_count = self.get_negative(actor);
                if other_count > self_count {
                    ops.push(PnCounterOperation::Decrement {
                        actor: actor.clone(),
                        amount: other_count - self_count,
                    });
                }
            }
            
            ops
        };
        
        // Apply all operations
        for operation in operations {
            self.apply_remote_operation(operation).await?;
        }
        
        Ok(())
    }
    
    fn can_merge(&self, _other: &Self) -> bool {
        true // PN-Counter can always merge
    }
    
    fn diff(&self, other: &Self) -> Vec<Self::Operation> {
        let mut operations = Vec::new();
        let self_state = self.state.read();
        let other_state = other.state.read();
        
        // Find positive increments in other that we're missing
        for (actor, &other_count) in &other_state.positive {
            let self_count = self_state.get_positive(actor);
            if other_count > self_count {
                operations.push(PnCounterOperation::Increment {
                    actor: actor.clone(),
                    amount: other_count - self_count,
                });
            }
        }
        
        // Find negative increments in other that we're missing
        for (actor, &other_count) in &other_state.negative {
            let self_count = self_state.get_negative(actor);
            if other_count > self_count {
                operations.push(PnCounterOperation::Decrement {
                    actor: actor.clone(),
                    amount: other_count - self_count,
                });
            }
        }
        
        operations
    }
}

#[async_trait]
impl Synchronizable for PnCounter {
    fn delta_since(&self, _clock: &VectorClock) -> Result<Delta<Self::State>> {
        Ok(Delta::FullState(self.clone_state()))
    }
    
    async fn apply_delta(&mut self, delta: Delta<Self::State>) -> Result<()> {
        match delta {
            Delta::FullState(state) => {
                // Merge the state
                let temp_counter = PnCounter {
                    actor_id: ActorId::new(), // Temporary actor ID
                    state: RwLock::new(state),
                    clock_manager: ClockManager::new(ActorId::new()),
                };
                self.merge(&temp_counter).await
            }
            Delta::Operation(bytes) => {
                let operation: PnCounterOperation = serde_json::from_slice(&bytes)?;
                self.apply_remote_operation(operation).await
            }
            Delta::Batch(operations) => {
                for op_bytes in operations {
                    let operation: PnCounterOperation = serde_json::from_slice(&op_bytes)?;
                    self.apply_remote_operation(operation).await?;
                }
                Ok(())
            }
        }
    }
    
    fn operations_since(&self, _clock: &VectorClock) -> Vec<Self::Operation> {
        let mut operations = Vec::new();
        let state = self.state.read();
        
        // Convert current state to operations
        for (actor, &count) in &state.positive {
            if count > 0 {
                operations.push(PnCounterOperation::Increment {
                    actor: actor.clone(),
                    amount: count,
                });
            }
        }
        
        for (actor, &count) in &state.negative {
            if count > 0 {
                operations.push(PnCounterOperation::Decrement {
                    actor: actor.clone(),
                    amount: count,
                });
            }
        }
        
        operations
    }
    
    fn size_bytes(&self) -> usize {
        let state = self.state.read();
        std::mem::size_of::<PnCounterState>() +
        state.positive.len() * (std::mem::size_of::<ActorId>() + std::mem::size_of::<u64>()) +
        state.negative.len() * (std::mem::size_of::<ActorId>() + std::mem::size_of::<u64>())
    }
}

impl Display for PnCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.state.read();
        write!(
            f,
            "PN-Counter[{}]: {} (P:{}, N:{})",
            self.actor_id,
            state.value(),
            state.positive_sum(),
            state.negative_sum()
        )
    }
}

// Additional utility methods
impl PnCounter {
    /// Reset counter to zero (creates new counter)
    pub fn reset(actor_id: ActorId) -> Self {
        Self::new(actor_id)
    }
    
    /// Create counter with initial value
    pub async fn with_initial_value(actor_id: ActorId, initial_value: i64) -> Result<Self> {
        let mut counter = Self::new(actor_id);
        
        if initial_value > 0 {
            counter.increment(initial_value as u64).await?;
        } else if initial_value < 0 {
            counter.decrement((-initial_value) as u64).await?;
        }
        
        Ok(counter)
    }
    
    /// Get contribution of a specific actor to the counter
    pub fn actor_contribution(&self, actor: &ActorId) -> i64 {
        let state = self.state.read();
        let positive = state.get_positive(actor) as i64;
        let negative = state.get_negative(actor) as i64;
        positive - negative
    }
    
    /// Get all actors that have contributed to this counter
    pub fn contributing_actors(&self) -> std::collections::HashSet<ActorId> {
        self.state.read().actors()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_pn_counter_increment() {
        let actor_id = ActorId::new();
        let mut counter = PnCounter::new(actor_id);
        
        counter.increment(5).await.unwrap();
        counter.increment(3).await.unwrap();
        
        assert_eq!(counter.value(), 8);
        assert_eq!(counter.positive_sum(), 8);
        assert_eq!(counter.negative_sum(), 0);
    }
    
    #[tokio::test]
    async fn test_pn_counter_decrement() {
        let actor_id = ActorId::new();
        let mut counter = PnCounter::new(actor_id);
        
        counter.increment(10).await.unwrap();
        counter.decrement(3).await.unwrap();
        
        assert_eq!(counter.value(), 7);
        assert_eq!(counter.positive_sum(), 10);
        assert_eq!(counter.negative_sum(), 3);
    }
    
    #[tokio::test]
    async fn test_pn_counter_merge() {
        let actor1 = ActorId::new();
        let actor2 = ActorId::new();
        
        let mut counter1 = PnCounter::new(actor1);
        let mut counter2 = PnCounter::new(actor2);
        
        counter1.increment(5).await.unwrap();
        counter2.increment(3).await.unwrap();
        counter2.decrement(1).await.unwrap();
        
        counter1.merge(&counter2).await.unwrap();
        counter2.merge(&counter1).await.unwrap();
        
        // Both should converge to same value: 5 + 3 - 1 = 7
        assert_eq!(counter1.value(), 7);
        assert_eq!(counter2.value(), 7);
    }
    
    #[tokio::test]
    async fn test_pn_counter_with_initial_value() {
        let actor_id = ActorId::new();
        
        let counter_pos = PnCounter::with_initial_value(actor_id.clone(), 10).await.unwrap();
        assert_eq!(counter_pos.value(), 10);
        
        let counter_neg = PnCounter::with_initial_value(actor_id.clone(), -5).await.unwrap();
        assert_eq!(counter_neg.value(), -5);
        
        let counter_zero = PnCounter::with_initial_value(actor_id, 0).await.unwrap();
        assert_eq!(counter_zero.value(), 0);
    }
    
    #[tokio::test]
    async fn test_pn_counter_actor_contribution() {
        let actor1 = ActorId::new();
        let actor2 = ActorId::new();
        
        let mut counter = PnCounter::new(actor1.clone());
        
        counter.increment(10).await.unwrap();
        counter.decrement(3).await.unwrap();
        
        // Simulate operations from another actor
        counter.apply_remote_operation(PnCounterOperation::Increment {
            actor: actor2.clone(),
            amount: 5,
        }).await.unwrap();
        
        assert_eq!(counter.actor_contribution(&actor1), 7); // 10 - 3
        assert_eq!(counter.actor_contribution(&actor2), 5); // 5 - 0
        assert_eq!(counter.value(), 12); // 7 + 5
    }
}