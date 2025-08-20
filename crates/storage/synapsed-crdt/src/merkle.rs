//! Merkle tree implementation for efficient sync
//!
//! Provides a Merkle tree structure for efficient synchronization between replicas.

use crate::types::{Hash, VectorClock};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Merkle tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    /// Hash of this node
    pub hash: Hash,
    /// Vector clock at this point
    pub clock: VectorClock,
    /// Child nodes
    pub children: HashMap<String, MerkleNode>,
}

impl MerkleNode {
    /// Create new merkle node
    pub fn new(hash: Hash, clock: VectorClock) -> Self {
        Self {
            hash,
            clock,
            children: HashMap::new(),
        }
    }
}

/// Merkle tree for sync optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    /// Root node
    pub root: MerkleNode,
}

impl MerkleTree {
    /// Create new merkle tree
    pub fn new() -> Self {
        Self {
            root: MerkleNode::new(Hash::zero(), VectorClock::new()),
        }
    }
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}