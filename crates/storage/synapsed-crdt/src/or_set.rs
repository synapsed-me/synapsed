//! Observed-Remove Set (OR-Set) CRDT implementation
//!
//! OR-Set provides conflict-free set operations where elements can be added and removed.
//! Removes are only effective if they have observed the corresponding add.

use crate::{
    error::{CrdtError, Result},
    traits::{Crdt, Mergeable, Synchronizable},
    types::{ActorId, Delta, Timestamp, VectorClock},
    clock::ClockManager,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Display},
    hash::Hash,
};
use parking_lot::RwLock;

/// Unique tag for OR-Set elements
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ElementTag {
    /// Actor that added the element
    pub actor: ActorId,
    /// Timestamp when added
    pub timestamp: Timestamp,
    /// Sequence number for uniqueness
    pub sequence: u64,
}

impl ElementTag {
    pub fn new(actor: ActorId, timestamp: Timestamp, sequence: u64) -> Self {
        Self {
            actor,
            timestamp,
            sequence,
        }
    }
}

impl PartialOrd for ElementTag {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ElementTag {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp
            .cmp(&other.timestamp)
            .then_with(|| self.actor.cmp(&other.actor))
            .then_with(|| self.sequence.cmp(&other.sequence))
    }
}

/// OR-Set operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrSetOperation<T> {
    /// Add element with unique tag
    Add {
        element: T,
        tag: ElementTag,
    },
    /// Remove element with observed tags
    Remove {
        element: T,
        observed_tags: HashSet<ElementTag>,
    },
}

/// OR-Set state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "T: Clone + Eq + Hash + Serialize + for<'de> Deserialize<'de>")]
pub struct OrSetState<T> {
    /// Elements with their tags (added elements)
    added: HashMap<T, HashSet<ElementTag>>,
    /// Removed tags for each element
    removed: HashMap<T, HashSet<ElementTag>>,
}

impl<T> OrSetState<T>
where
    T: Clone + Eq + Hash + Serialize + for<'de> Deserialize<'de>,
{
    /// Create new empty state
    pub fn new() -> Self {
        Self {
            added: HashMap::new(),
            removed: HashMap::new(),
        }
    }
    
    /// Get all elements currently in the set
    pub fn elements(&self) -> HashSet<T> {
        let mut result = HashSet::new();
        
        for (element, added_tags) in &self.added {
            let removed_tags = self.removed.get(element).cloned().unwrap_or_default();
            
            // Element is present if it has tags that haven't been removed
            if added_tags.difference(&removed_tags).next().is_some() {
                result.insert(element.clone());
            }
        }
        
        result
    }
    
    /// Check if element is in the set
    pub fn contains(&self, element: &T) -> bool {
        if let Some(added_tags) = self.added.get(element) {
            let removed_tags = self.removed.get(element).cloned().unwrap_or_default();
            added_tags.difference(&removed_tags).next().is_some()
        } else {
            false
        }
    }
    
    /// Get size of the set
    pub fn len(&self) -> usize {
        self.elements().len()
    }
    
    /// Check if set is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Get all tags for an element
    pub fn get_tags(&self, element: &T) -> HashSet<ElementTag> {
        self.added.get(element).cloned().unwrap_or_default()
    }
}

impl<T> Default for OrSetState<T>
where
    T: Clone + Eq + Hash + Serialize + for<'de> Deserialize<'de>,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Observed-Remove Set CRDT
#[derive(Debug)]
pub struct OrSet<T> 
where
    T: Clone + Eq + Hash + Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    /// Actor ID for this replica
    actor_id: ActorId,
    /// Current state
    state: RwLock<OrSetState<T>>,
    /// Clock manager
    clock_manager: ClockManager,
    /// Sequence counter for unique tags
    sequence_counter: RwLock<u64>,
}

impl<T> OrSet<T>
where
    T: Clone + Eq + Hash + Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    /// Create new OR-Set
    pub fn new(actor_id: ActorId) -> Self {
        Self {
            actor_id: actor_id.clone(),
            state: RwLock::new(OrSetState::new()),
            clock_manager: ClockManager::new(actor_id),
            sequence_counter: RwLock::new(0),
        }
    }
    
    /// Add element to the set
    pub async fn add(&mut self, element: T) -> Result<OrSetOperation<T>> {
        let mut seq_counter = self.sequence_counter.write();
        *seq_counter += 1;
        let sequence = *seq_counter;
        drop(seq_counter);
        
        let timestamp = self.clock_manager.create_timestamp();
        let tag = ElementTag::new(self.actor_id.clone(), timestamp, sequence);
        
        let operation = OrSetOperation::Add {
            element: element.clone(),
            tag: tag.clone(),
        };
        
        self.apply_operation(operation.clone()).await?;
        Ok(operation)
    }
    
    /// Remove element from the set
    pub async fn remove(&mut self, element: &T) -> Result<OrSetOperation<T>> {
        let state = self.state.read();
        let observed_tags = state.get_tags(element);
        drop(state);
        
        if observed_tags.is_empty() {
            return Err(CrdtError::InvalidOperation(
                "Cannot remove element that was never added".to_string(),
            ));
        }
        
        let operation = OrSetOperation::Remove {
            element: element.clone(),
            observed_tags,
        };
        
        self.apply_operation(operation.clone()).await?;
        Ok(operation)
    }
    
    /// Check if element is in the set
    pub fn contains(&self, element: &T) -> bool {
        self.state.read().contains(element)
    }
    
    /// Get all elements in the set
    pub fn elements(&self) -> HashSet<T> {
        self.state.read().elements()
    }
    
    /// Get size of the set
    pub fn len(&self) -> usize {
        self.state.read().len()
    }
    
    /// Check if set is empty
    pub fn is_empty(&self) -> bool {
        self.state.read().is_empty()
    }
    
    /// Apply add operation
    async fn apply_add(&mut self, element: T, tag: ElementTag) -> Result<()> {
        let mut state = self.state.write();
        state.added.entry(element).or_default().insert(tag);
        Ok(())
    }
    
    /// Apply remove operation
    async fn apply_remove(&mut self, element: T, observed_tags: HashSet<ElementTag>) -> Result<()> {
        let mut state = self.state.write();
        
        // Only remove tags that we actually have
        if let Some(added_tags) = state.added.get(&element) {
            let valid_removes: HashSet<_> = observed_tags
                .intersection(added_tags)
                .cloned()
                .collect();
            
            if !valid_removes.is_empty() {
                state.removed.entry(element).or_default().extend(valid_removes);
            }
        }
        
        Ok(())
    }
}

impl<T> Clone for OrSet<T>
where
    T: Clone + Eq + Hash + Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    fn clone(&self) -> Self {
        Self {
            actor_id: self.actor_id.clone(),
            state: RwLock::new(self.state.read().clone()),
            clock_manager: self.clock_manager.clone(),
            sequence_counter: RwLock::new(*self.sequence_counter.read()),
        }
    }
}

#[async_trait]
impl<T> Crdt for OrSet<T>
where
    T: Clone + Eq + Hash + Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    type Operation = OrSetOperation<T>;
    type State = OrSetState<T>;
    
    async fn apply_operation(&mut self, operation: Self::Operation) -> Result<()> {
        match operation {
            OrSetOperation::Add { element, tag } => {
                self.apply_add(element, tag).await
            }
            OrSetOperation::Remove { element, observed_tags } => {
                self.apply_remove(element, observed_tags).await
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
            OrSetOperation::Add { .. } => Ok(()),
            OrSetOperation::Remove { observed_tags, .. } => {
                if observed_tags.is_empty() {
                    Err(CrdtError::InvalidOperation(
                        "Remove operation must observe at least one tag".to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl<T> OrSet<T>
where
    T: Clone + Eq + Hash + Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    /// Get cloned state
    pub fn clone_state(&self) -> OrSetState<T> {
        self.state.read().clone()
    }
    
    /// Get current vector clock
    pub fn get_vector_clock(&self) -> VectorClock {
        self.clock_manager.vector_clock()
    }
}

#[async_trait]
impl<T> Mergeable for OrSet<T>
where
    T: Clone + Eq + Hash + Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    async fn merge(&mut self, other: &Self) -> Result<()> {
        let operations = {
            let other_state = other.state.read();
            let mut ops = Vec::new();
            
            // Collect add operations
            for (element, tags) in &other_state.added {
                for tag in tags {
                    ops.push(OrSetOperation::Add {
                        element: element.clone(),
                        tag: tag.clone(),
                    });
                }
            }
            
            // Collect remove operations
            for (element, tags) in &other_state.removed {
                if !tags.is_empty() {
                    ops.push(OrSetOperation::Remove {
                        element: element.clone(),
                        observed_tags: tags.clone(),
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
        true // OR-Set can always merge
    }
    
    fn diff(&self, other: &Self) -> Vec<Self::Operation> {
        let mut operations = Vec::new();
        let self_state = self.state.read();
        let other_state = other.state.read();
        
        // Find added elements in other that we don't have
        for (element, other_tags) in &other_state.added {
            let self_tags = self_state.added.get(element).cloned().unwrap_or_default();
            
            for tag in other_tags.difference(&self_tags) {
                operations.push(OrSetOperation::Add {
                    element: element.clone(),
                    tag: tag.clone(),
                });
            }
        }
        
        // Find removed elements in other that we don't have
        for (element, other_removed) in &other_state.removed {
            let self_removed = self_state.removed.get(element).cloned().unwrap_or_default();
            let diff_removed: HashSet<_> = other_removed.difference(&self_removed).cloned().collect();
            
            if !diff_removed.is_empty() {
                operations.push(OrSetOperation::Remove {
                    element: element.clone(),
                    observed_tags: diff_removed,
                });
            }
        }
        
        operations
    }
}

#[async_trait]
impl<T> Synchronizable for OrSet<T>
where
    T: Clone + Eq + Hash + Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    fn delta_since(&self, _clock: &VectorClock) -> Result<Delta<Self::State>> {
        Ok(Delta::FullState(self.clone_state()))
    }
    
    async fn apply_delta(&mut self, delta: Delta<Self::State>) -> Result<()> {
        match delta {
            Delta::FullState(state) => {
                // Create operations from state and apply them
                for (element, tags) in state.added {
                    for tag in tags {
                        let op = OrSetOperation::Add {
                            element: element.clone(),
                            tag,
                        };
                        self.apply_remote_operation(op).await?;
                    }
                }
                
                for (element, tags) in state.removed {
                    if !tags.is_empty() {
                        let op = OrSetOperation::Remove {
                            element,
                            observed_tags: tags,
                        };
                        self.apply_remote_operation(op).await?;
                    }
                }
                Ok(())
            }
            Delta::Operation(bytes) => {
                let operation: OrSetOperation<T> = serde_json::from_slice(&bytes)?;
                self.apply_remote_operation(operation).await
            }
            Delta::Batch(operations) => {
                for op_bytes in operations {
                    let operation: OrSetOperation<T> = serde_json::from_slice(&op_bytes)?;
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
        for (element, tags) in &state.added {
            for tag in tags {
                operations.push(OrSetOperation::Add {
                    element: element.clone(),
                    tag: tag.clone(),
                });
            }
        }
        
        for (element, tags) in &state.removed {
            if !tags.is_empty() {
                operations.push(OrSetOperation::Remove {
                    element: element.clone(),
                    observed_tags: tags.clone(),
                });
            }
        }
        
        operations
    }
    
    fn size_bytes(&self) -> usize {
        let state = self.state.read();
        std::mem::size_of::<OrSetState<T>>() + 
        state.added.len() * std::mem::size_of::<T>() +
        state.removed.len() * std::mem::size_of::<T>()
    }
}

impl<T> Display for OrSet<T>
where
    T: Display + Clone + Eq + Hash,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let elements: Vec<String> = self.elements().iter().map(|e| e.to_string()).collect();
        write!(f, "OR-Set[{}]: {{{}}}", self.actor_id, elements.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_or_set_add() {
        let actor_id = ActorId::new();
        let mut set = OrSet::new(actor_id);
        
        set.add("hello".to_string()).await.unwrap();
        set.add("world".to_string()).await.unwrap();
        
        assert!(set.contains(&"hello".to_string()));
        assert!(set.contains(&"world".to_string()));
        assert_eq!(set.len(), 2);
    }
    
    #[tokio::test]
    async fn test_or_set_remove() {
        let actor_id = ActorId::new();
        let mut set = OrSet::new(actor_id);
        
        set.add("hello".to_string()).await.unwrap();
        set.add("world".to_string()).await.unwrap();
        set.remove(&"hello".to_string()).await.unwrap();
        
        assert!(!set.contains(&"hello".to_string()));
        assert!(set.contains(&"world".to_string()));
        assert_eq!(set.len(), 1);
    }
    
    #[tokio::test]
    async fn test_or_set_merge() {
        let actor1 = ActorId::new();
        let actor2 = ActorId::new();
        
        let mut set1 = OrSet::new(actor1);
        let mut set2 = OrSet::new(actor2);
        
        set1.add("A".to_string()).await.unwrap();
        set2.add("B".to_string()).await.unwrap();
        
        set1.merge(&set2).await.unwrap();
        set2.merge(&set1).await.unwrap();
        
        // Both should converge
        assert_eq!(set1.elements(), set2.elements());
        assert!(set1.contains(&"A".to_string()));
        assert!(set1.contains(&"B".to_string()));
    }
    
    #[tokio::test]
    async fn test_or_set_add_remove_conflict() {
        let actor1 = ActorId::new();
        let actor2 = ActorId::new();
        
        let mut set1 = OrSet::new(actor1);
        let mut set2 = OrSet::new(actor2);
        
        // Both add the same element
        set1.add("X".to_string()).await.unwrap();
        set2.add("X".to_string()).await.unwrap();
        
        // set1 removes it
        set1.remove(&"X".to_string()).await.unwrap();
        
        // Merge
        set1.merge(&set2).await.unwrap();
        set2.merge(&set1).await.unwrap();
        
        // Element should still be present (add wins)
        assert!(set1.contains(&"X".to_string()));
        assert!(set2.contains(&"X".to_string()));
    }
}