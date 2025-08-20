//! Replicated Growable Array (RGA) for collaborative text editing
//!
//! RGA provides conflict-free collaborative text editing with operation-based semantics.
//! It maintains a linear sequence of characters with unique identifiers for each position.

use crate::{
    error::{CrdtError, Result},
    traits::{Crdt, Mergeable, Synchronizable, GarbageCollectable},
    types::{ActorId, Delta, GlobalId, HybridLogicalClock, VectorClock},
    clock::ClockManager,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    fmt::{self, Display},
};
use parking_lot::RwLock;

/// RGA node representing a single character with metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgaNode {
    /// Unique identifier for this node
    pub id: GlobalId,
    /// Character content
    pub content: char,
    /// Whether this node is visible (not deleted)
    pub visible: bool,
    /// Hybrid logical clock timestamp
    pub timestamp: HybridLogicalClock,
    /// Author of this node
    pub author: ActorId,
}

impl RgaNode {
    /// Create a new RGA node
    pub fn new(
        id: GlobalId,
        content: char,
        timestamp: HybridLogicalClock,
        author: ActorId,
    ) -> Self {
        Self {
            id,
            content,
            visible: true,
            timestamp,
            author,
        }
    }
    
    /// Mark node as deleted (tombstone)
    pub fn delete(&mut self, timestamp: HybridLogicalClock) {
        self.visible = false;
        self.timestamp = timestamp;
    }
}

/// RGA operation types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RgaOperation {
    /// Insert character at position
    Insert {
        id: GlobalId,
        content: char,
        position: Option<GlobalId>, // None means insert at beginning
        timestamp: HybridLogicalClock,
        author: ActorId,
    },
    /// Delete character at position
    Delete {
        target_id: GlobalId,
        timestamp: HybridLogicalClock,
        author: ActorId,
    },
}

/// RGA document state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgaState {
    /// Ordered sequence of nodes
    nodes: Vec<RgaNode>,
    /// Fast lookup by ID
    node_index: HashMap<GlobalId, usize>,
    /// Current text length (visible characters only)
    text_length: usize,
}

impl RgaState {
    /// Create new empty RGA state
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            node_index: HashMap::new(),
            text_length: 0,
        }
    }
    
    /// Get visible text
    pub fn text(&self) -> String {
        self.nodes
            .iter()
            .filter(|node| node.visible)
            .map(|node| node.content)
            .collect()
    }
    
    /// Get text length (visible characters only)
    pub fn len(&self) -> usize {
        self.text_length
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.text_length == 0
    }
    
    /// Find node by ID
    pub fn find_node(&self, id: &GlobalId) -> Option<&RgaNode> {
        self.node_index.get(id).and_then(|&index| self.nodes.get(index))
    }
    
    /// Find node by position offset
    pub fn find_node_at_offset(&self, offset: usize) -> Option<&RgaNode> {
        let mut current_offset = 0;
        for node in &self.nodes {
            if node.visible {
                if current_offset == offset {
                    return Some(node);
                }
                current_offset += 1;
            }
        }
        None
    }
    
    /// Get position of node as offset in visible text
    pub fn get_offset(&self, id: &GlobalId) -> Option<usize> {
        let mut offset = 0;
        for node in &self.nodes {
            if node.id == *id {
                return if node.visible { Some(offset) } else { None };
            }
            if node.visible {
                offset += 1;
            }
        }
        None
    }
}

impl Default for RgaState {
    fn default() -> Self {
        Self::new()
    }
}

/// Replicated Growable Array CRDT for text editing
#[derive(Debug)]
pub struct Rga {
    /// Actor ID for this replica
    actor_id: ActorId,
    /// Current state
    state: RwLock<RgaState>,
    /// Clock manager
    clock_manager: ClockManager,
    /// Operation counter for unique IDs
    counter: RwLock<u64>,
    /// Buffered operations for causal delivery
    operation_buffer: RwLock<VecDeque<RgaOperation>>,
}

impl Rga {
    /// Create new RGA
    pub fn new(actor_id: ActorId) -> Self {
        let clock_manager = ClockManager::new(actor_id.clone());
        
        Self {
            actor_id,
            state: RwLock::new(RgaState::new()),
            clock_manager,
            counter: RwLock::new(0),
            operation_buffer: RwLock::new(VecDeque::new()),
        }
    }
    
    /// Insert character at specified offset
    pub async fn insert_at_offset(&mut self, offset: usize, character: char) -> Result<RgaOperation> {
        let counter_value = {
            let mut counter = self.counter.write();
            *counter += 1;
            *counter
        };
        
        let hlc = self.clock_manager.advance_hlc();
        let id = GlobalId::new(counter_value, self.actor_id, hlc.logical_time);
        
        // Find position to insert after
        let position = if offset == 0 {
            None // Insert at beginning
        } else {
            let state = self.state.read();
            state.find_node_at_offset(offset - 1).map(|node| node.id.clone())
        };
        
        let operation = RgaOperation::Insert {
            id,
            content: character,
            position,
            timestamp: hlc,
            author: self.actor_id,
        };
        
        self.apply_operation(operation.clone()).await?;
        Ok(operation)
    }
    
    /// Delete character at specified offset
    pub async fn delete_at_offset(&mut self, offset: usize) -> Result<RgaOperation> {
        let state = self.state.read();
        let node = state.find_node_at_offset(offset)
            .ok_or_else(|| CrdtError::InvalidOperation("Offset out of bounds".to_string()))?;
        
        let target_id = node.id.clone();
        drop(state);
        
        let hlc = self.clock_manager.advance_hlc();
        let operation = RgaOperation::Delete {
            target_id,
            timestamp: hlc,
            author: self.actor_id,
        };
        
        self.apply_operation(operation.clone()).await?;
        Ok(operation)
    }
    
    /// Get current text
    pub fn text(&self) -> String {
        self.state.read().text()
    }
    
    /// Get text length
    pub fn len(&self) -> usize {
        self.state.read().len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.state.read().is_empty()
    }
    
    /// Apply insert operation
    async fn apply_insert(
        &mut self,
        id: GlobalId,
        content: char,
        position: Option<GlobalId>,
        timestamp: HybridLogicalClock,
        author: ActorId,
    ) -> Result<()> {
        let mut state = self.state.write();
        
        // Check if already applied (idempotency)
        if state.node_index.contains_key(&id) {
            return Ok(());
        }
        
        let new_node = RgaNode::new(id.clone(), content, timestamp, author);
        
        // Find insertion position
        let insert_index = if let Some(prev_id) = position {
            // Insert after the specified node
            if let Some(&prev_index) = state.node_index.get(&prev_id) {
                let mut insert_idx = prev_index + 1;
                
                // Maintain total ordering by ID
                while insert_idx < state.nodes.len() && id > state.nodes[insert_idx].id {
                    insert_idx += 1;
                }
                insert_idx
            } else {
                // Previous node not found, insert at end
                state.nodes.len()
            }
        } else {
            // Insert at beginning, but maintain ordering
            let mut insert_idx = 0;
            while insert_idx < state.nodes.len() && id > state.nodes[insert_idx].id {
                insert_idx += 1;
            }
            insert_idx
        };
        
        // Insert node
        state.nodes.insert(insert_index, new_node);
        
        // Update index map for all nodes after insertion point
        let nodes_to_update: Vec<_> = state.nodes.iter()
            .enumerate()
            .skip(insert_index)
            .map(|(i, node)| (node.id.clone(), i))
            .collect();
        
        for (node_id, index) in nodes_to_update {
            state.node_index.insert(node_id, index);
        }
        
        // Update text length
        state.text_length += 1;
        
        Ok(())
    }
    
    /// Apply delete operation
    async fn apply_delete(
        &mut self,
        target_id: GlobalId,
        timestamp: HybridLogicalClock,
        _author: ActorId,
    ) -> Result<()> {
        let mut state = self.state.write();
        
        if let Some(&index) = state.node_index.get(&target_id) {
            if let Some(node) = state.nodes.get_mut(index) {
                if node.visible {
                    node.delete(timestamp);
                    state.text_length -= 1;
                }
            }
        }
        // Note: We don't return error if node not found to handle concurrent deletes
        
        Ok(())
    }
}

impl Clone for Rga {
    fn clone(&self) -> Self {
        Self {
            actor_id: self.actor_id.clone(),
            state: RwLock::new(self.state.read().clone()),
            clock_manager: self.clock_manager.clone(),
            counter: RwLock::new(*self.counter.read()),
            operation_buffer: RwLock::new(self.operation_buffer.read().clone()),
        }
    }
}

#[async_trait]
impl Crdt for Rga {
    type Operation = RgaOperation;
    type State = RgaState;
    
    async fn apply_operation(&mut self, operation: Self::Operation) -> Result<()> {
        match operation {
            RgaOperation::Insert { id, content, position, timestamp, author } => {
                self.clock_manager.advance_hlc_remote(&timestamp);
                self.apply_insert(id, content, position, timestamp, author).await
            }
            RgaOperation::Delete { target_id, timestamp, author } => {
                self.clock_manager.advance_hlc_remote(&timestamp);
                self.apply_delete(target_id, timestamp, author).await
            }
        }
    }
    
    async fn apply_remote_operation(&mut self, operation: Self::Operation) -> Result<()> {
        self.apply_operation(operation).await
    }
    
    fn state(&self) -> &Self::State {
        // This is a bit tricky with RwLock, so we'll need a different approach
        // For now, we'll provide a method to get a cloned state
        unimplemented!("Use clone_state() instead")
    }
    
    fn actor_id(&self) -> &ActorId {
        &self.actor_id
    }
    
    fn vector_clock(&self) -> &VectorClock {
        // This would need to be stored separately
        unimplemented!("Vector clock not directly stored in RGA")
    }
    
    fn validate_operation(&self, operation: &Self::Operation) -> Result<()> {
        match operation {
            RgaOperation::Insert { content, .. } => {
                if content.is_control() {
                    return Err(CrdtError::InvalidOperation(
                        "Control characters not allowed".to_string()
                    ));
                }
                Ok(())
            }
            RgaOperation::Delete { .. } => Ok(()),
        }
    }
}

impl Rga {
    /// Get a cloned state
    pub fn clone_state(&self) -> RgaState {
        self.state.read().clone()
    }
    
    /// Get current vector clock
    pub fn get_vector_clock(&self) -> VectorClock {
        self.clock_manager.vector_clock()
    }
}

#[async_trait]
impl Mergeable for Rga {
    async fn merge(&mut self, other: &Self) -> Result<()> {
        let operations = {
            let other_state = other.state.read();
            let self_state = self.state.read();
            let mut ops = Vec::new();
            
            // Collect operations from other replica
            for node in &other_state.nodes {
                // Apply if not already present
                if !self_state.node_index.contains_key(&node.id) {
                    let operation = if node.visible {
                        RgaOperation::Insert {
                            id: node.id.clone(),
                            content: node.content,
                            position: None, // We'll figure this out during insertion
                            timestamp: node.timestamp,
                            author: node.author,
                        }
                    } else {
                        RgaOperation::Delete {
                            target_id: node.id.clone(),
                            timestamp: node.timestamp,
                            author: node.author,
                        }
                    };
                    ops.push(operation);
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
        true // RGA can always merge
    }
    
    fn diff(&self, other: &Self) -> Vec<Self::Operation> {
        let mut operations = Vec::new();
        let self_state = self.state.read();
        let other_state = other.state.read();
        
        // Find nodes in other that we don't have
        for node in &other_state.nodes {
            if !self_state.node_index.contains_key(&node.id) {
                let operation = if node.visible {
                    RgaOperation::Insert {
                        id: node.id.clone(),
                        content: node.content,
                        position: None,
                        timestamp: node.timestamp,
                        author: node.author,
                    }
                } else {
                    RgaOperation::Delete {
                        target_id: node.id.clone(),
                        timestamp: node.timestamp,
                        author: node.author,
                    }
                };
                operations.push(operation);
            }
        }
        
        operations
    }
}

#[async_trait]
impl Synchronizable for Rga {
    fn delta_since(&self, _clock: &VectorClock) -> Result<Delta<Self::State>> {
        // For RGA, we typically send the full state or specific operations
        Ok(Delta::FullState(self.clone_state()))
    }
    
    async fn apply_delta(&mut self, delta: Delta<Self::State>) -> Result<()> {
        match delta {
            Delta::FullState(state) => {
                // Replace current state (this is a simplified approach)
                *self.state.write() = state;
                Ok(())
            }
            Delta::Operation(bytes) => {
                let operation: RgaOperation = serde_json::from_slice(&bytes)?;
                self.apply_remote_operation(operation).await
            }
            Delta::Batch(operations) => {
                for op_bytes in operations {
                    let operation: RgaOperation = serde_json::from_slice(&op_bytes)?;
                    self.apply_remote_operation(operation).await?;
                }
                Ok(())
            }
        }
    }
    
    fn operations_since(&self, _clock: &VectorClock) -> Vec<Self::Operation> {
        // This would need to track operation history
        Vec::new()
    }
    
    fn size_bytes(&self) -> usize {
        // Rough estimate
        let state = self.state.read();
        state.nodes.len() * std::mem::size_of::<RgaNode>()
    }
}

#[async_trait]
impl GarbageCollectable for Rga {
    async fn garbage_collect(&mut self) -> Result<usize> {
        let mut state = self.state.write();
        let _initial_size = state.nodes.len();
        
        // Remove tombstones (deleted nodes) that are old enough
        // For now, we'll keep all tombstones for conflict resolution
        // In a real implementation, we'd have a more sophisticated GC policy
        
        Ok(0) // No nodes removed for now
    }
    
    fn needs_gc(&self) -> bool {
        let state = self.state.read();
        let deleted_nodes = state.nodes.iter().filter(|n| !n.visible).count();
        let total_nodes = state.nodes.len();
        
        total_nodes > 0 && (deleted_nodes as f64 / total_nodes as f64) > 0.5
    }
    
    fn garbage_size(&self) -> usize {
        let state = self.state.read();
        state.nodes.iter().filter(|n| !n.visible).count() * std::mem::size_of::<RgaNode>()
    }
}

impl Display for Rga {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RGA[{}]: \"{}\"", self.actor_id, self.text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_rga_insert() {
        let actor_id = ActorId::new();
        let mut rga = Rga::new(actor_id);
        
        rga.insert_at_offset(0, 'H').await.unwrap();
        rga.insert_at_offset(1, 'i').await.unwrap();
        
        assert_eq!(rga.text(), "Hi");
        assert_eq!(rga.len(), 2);
    }
    
    #[tokio::test]
    async fn test_rga_delete() {
        let actor_id = ActorId::new();
        let mut rga = Rga::new(actor_id);
        
        rga.insert_at_offset(0, 'H').await.unwrap();
        rga.insert_at_offset(1, 'i').await.unwrap();
        rga.delete_at_offset(0).await.unwrap();
        
        assert_eq!(rga.text(), "i");
        assert_eq!(rga.len(), 1);
    }
    
    #[tokio::test]
    async fn test_rga_merge() {
        let actor1 = ActorId::new();
        let actor2 = ActorId::new();
        
        let mut rga1 = Rga::new(actor1);
        let mut rga2 = Rga::new(actor2);
        
        rga1.insert_at_offset(0, 'A').await.unwrap();
        rga2.insert_at_offset(0, 'B').await.unwrap();
        
        rga1.merge(&rga2).await.unwrap();
        rga2.merge(&rga1).await.unwrap();
        
        // Both should converge to the same state
        assert_eq!(rga1.text(), rga2.text());
    }
}