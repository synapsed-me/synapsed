//! Core types for consensus protocols

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a node in the consensus network
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(Uuid);

impl NodeId {
    /// Create a new random node ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    
    /// Create a node ID from a UUID
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }
    
    /// Get the inner UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Node({})", self.0)
    }
}

/// View number for consensus rounds
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ViewNumber(pub u64);

impl ViewNumber {
    pub fn new(view: u64) -> Self {
        Self(view)
    }
    
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
    
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for ViewNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "View({})", self.0)
    }
}

/// A block of transactions in the consensus protocol
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// Unique block identifier
    pub id: Uuid,
    /// Hash of the previous block
    pub parent_hash: Vec<u8>,
    /// Block height in the chain
    pub height: u64,
    /// Timestamp when block was created
    pub timestamp: DateTime<Utc>,
    /// Transactions in the block
    pub transactions: Vec<Transaction>,
    /// Node that proposed this block
    pub proposer: NodeId,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl Block {
    /// Create a new block
    pub fn new(
        parent_hash: Vec<u8>,
        height: u64,
        transactions: Vec<Transaction>,
        proposer: NodeId,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent_hash,
            height,
            timestamp: Utc::now(),
            transactions,
            proposer,
            metadata: HashMap::new(),
        }
    }
    
    /// Calculate the hash of this block
    pub fn hash(&self) -> Vec<u8> {
        use ring::digest::{Context, SHA256};
        
        let serialized = serde_json::to_vec(self).unwrap_or_default();
        let mut context = Context::new(&SHA256);
        context.update(&serialized);
        context.finish().as_ref().to_vec()
    }
    
    /// Check if this block is valid
    pub fn is_valid(&self) -> bool {
        // Basic validation rules
        !self.transactions.is_empty() && 
        self.height > 0 &&
        self.timestamp <= Utc::now()
    }
}

/// A transaction within a block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction ID
    pub id: Uuid,
    /// Transaction payload
    pub data: Vec<u8>,
    /// Transaction timestamp
    pub timestamp: DateTime<Utc>,
    /// Digital signature
    pub signature: Vec<u8>,
}

impl Transaction {
    pub fn new(data: Vec<u8>, signature: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4(),
            data,
            timestamp: Utc::now(),
            signature,
        }
    }
}

/// Vote types for consensus protocols
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoteType {
    /// Prepare phase vote (PBFT, HotStuff)
    Prepare,
    /// Pre-commit phase vote
    PreCommit,
    /// Commit phase vote
    Commit,
    /// View change vote
    ViewChange,
}

/// A vote cast by a validator
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vote {
    /// Vote type
    pub vote_type: VoteType,
    /// View number this vote applies to
    pub view: ViewNumber,
    /// Block being voted on
    pub block_id: Uuid,
    /// Node casting the vote
    pub voter: NodeId,
    /// Digital signature of the vote
    pub signature: Vec<u8>,
    /// Timestamp of the vote
    pub timestamp: DateTime<Utc>,
}

impl Vote {
    pub fn new(
        vote_type: VoteType,
        view: ViewNumber,
        block_id: Uuid,
        voter: NodeId,
        signature: Vec<u8>,
    ) -> Self {
        Self {
            vote_type,
            view,
            block_id,
            voter,
            signature,
            timestamp: Utc::now(),
        }
    }
}

/// Quorum certificate aggregating multiple votes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuorumCertificate {
    /// View number
    pub view: ViewNumber,
    /// Block this QC applies to
    pub block_id: Uuid,
    /// Votes comprising this QC
    pub votes: Vec<Vote>,
    /// Aggregated signature (future optimization)
    pub aggregate_signature: Option<Vec<u8>>,
}

impl QuorumCertificate {
    pub fn new(votes: Vec<Vote>) -> Self {
        let view = votes.first().map(|v| v.view).unwrap_or(ViewNumber(0));
        let block_id = votes.first().map(|v| v.block_id).unwrap_or_default();
        
        Self {
            view,
            block_id,
            votes,
            aggregate_signature: None,
        }
    }
    
    /// Check if this QC has enough votes for the given threshold
    pub fn has_quorum(&self, threshold: usize) -> bool {
        self.votes.len() >= threshold
    }
    
    /// Get the number of votes in this QC
    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }
}

/// Consensus protocol phases
#[derive(Debug, Clone, PartialEq)]
pub enum ConsensusPhase {
    /// Preparing to propose
    Prepare,
    /// Pre-commit phase
    PreCommit,
    /// Commit phase
    Commit,
    /// View change in progress
    ViewChange,
    /// Consensus decided
    Decided,
}

/// Consensus protocol state
#[derive(Debug, Clone)]
pub struct ConsensusState {
    /// Current view number
    pub view: ViewNumber,
    /// Current consensus phase
    pub phase: ConsensusPhase,
    /// Current leader (for leader-based protocols)
    pub leader: Option<NodeId>,
    /// Last committed block
    pub last_committed_block: Option<Block>,
    /// Pending block being decided
    pub pending_block: Option<Block>,
    /// Collected votes for current view
    pub votes: HashMap<VoteType, Vec<Vote>>,
    /// Quorum certificates
    pub qcs: HashMap<ViewNumber, QuorumCertificate>,
}

impl ConsensusState {
    pub fn new(initial_view: ViewNumber) -> Self {
        Self {
            view: initial_view,
            phase: ConsensusPhase::Prepare,
            leader: None,
            last_committed_block: None,
            pending_block: None,
            votes: HashMap::new(),
            qcs: HashMap::new(),
        }
    }
    
    /// Advance to the next view
    pub fn advance_view(&mut self) {
        self.view = self.view.next();
        self.phase = ConsensusPhase::Prepare;
        self.votes.clear();
        self.pending_block = None;
    }
    
    /// Add a vote to the current state
    pub fn add_vote(&mut self, vote: Vote) {
        self.votes
            .entry(vote.vote_type.clone())
            .or_insert_with(Vec::new)
            .push(vote);
    }
    
    /// Check if we have enough votes for a quorum
    pub fn has_quorum(&self, vote_type: &VoteType, threshold: usize) -> bool {
        self.votes
            .get(vote_type)
            .map(|votes| votes.len() >= threshold)
            .unwrap_or(false)
    }
}