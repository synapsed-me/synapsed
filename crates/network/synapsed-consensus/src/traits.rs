//! Core traits for consensus protocols

use crate::{Block, NodeId, Vote, QuorumCertificate, ViewNumber};
use crate::types::Transaction;
use crate::error::Result;
use async_trait::async_trait;
// use std::collections::HashMap; // Will be used when implementing algorithms

/// Main trait for consensus protocol implementations
#[async_trait]
pub trait ConsensusProtocol: Send + Sync {
    /// Start the consensus protocol
    async fn start(&mut self) -> Result<()>;
    
    /// Stop the consensus protocol gracefully
    async fn stop(&mut self) -> Result<()>;
    
    /// Propose a new block
    async fn propose_block(&mut self, transactions: Vec<Transaction>) -> Result<Block>;
    
    /// Handle an incoming vote
    async fn handle_vote(&mut self, vote: Vote) -> Result<()>;
    
    /// Handle an incoming block proposal
    async fn handle_proposal(&mut self, block: Block) -> Result<()>;
    
    /// Get the current view number
    fn current_view(&self) -> ViewNumber;
    
    /// Check if this node is the current leader
    fn is_leader(&self) -> bool;
    
    /// Get the current leader for this view
    fn current_leader(&self) -> Option<NodeId>;
    
    /// Get consensus statistics
    fn get_stats(&self) -> ConsensusStats;
}

/// State machine interface for consensus protocols
#[async_trait]
pub trait StateMachine: Send + Sync {
    /// Apply a committed block to the state machine
    async fn apply_block(&mut self, block: &Block) -> Result<()>;
    
    /// Get current state hash
    async fn state_hash(&self) -> Result<Vec<u8>>;
    
    /// Create a snapshot of current state
    async fn create_snapshot(&self) -> Result<Vec<u8>>;
    
    /// Restore state from snapshot
    async fn restore_snapshot(&mut self, snapshot: &[u8]) -> Result<()>;
    
    /// Validate a block before consensus
    async fn validate_block(&self, block: &Block) -> Result<bool>;
}

/// Network transport abstraction for consensus messages
#[async_trait]
pub trait NetworkTransport: Send + Sync {
    /// Broadcast a message to all peers
    async fn broadcast(&self, message: ConsensusMessage) -> Result<()>;
    
    /// Send a message to a specific peer
    async fn send_to(&self, peer: NodeId, message: ConsensusMessage) -> Result<()>;
    
    /// Receive the next message
    async fn receive(&mut self) -> Result<(NodeId, ConsensusMessage)>;
    
    /// Get list of connected peers
    async fn peers(&self) -> Result<Vec<NodeId>>;
    
    /// Check if connected to a specific peer
    async fn is_connected(&self, peer: &NodeId) -> Result<bool>;
}

/// Cryptographic operations for consensus
#[async_trait]
pub trait ConsensusCrypto: Send + Sync {
    /// Sign a message with this node's private key
    async fn sign(&self, message: &[u8]) -> Result<Vec<u8>>;
    
    /// Verify a signature from another node
    async fn verify(&self, node: &NodeId, message: &[u8], signature: &[u8]) -> Result<bool>;
    
    /// Get this node's public key
    async fn public_key(&self) -> Result<Vec<u8>>;
    
    /// Verify a quorum certificate
    async fn verify_qc(&self, qc: &QuorumCertificate) -> Result<bool>;
    
    /// Create an aggregated signature (if supported)
    async fn aggregate_signatures(&self, signatures: &[Vec<u8>]) -> Result<Vec<u8>>;
}

/// Leader election strategy
pub trait LeaderElection: Send + Sync {
    /// Determine the leader for a given view
    fn get_leader(&self, view: ViewNumber, validators: &[NodeId]) -> NodeId;
    
    /// Check if a node should be the leader for a view
    fn is_leader(&self, node: &NodeId, view: ViewNumber, validators: &[NodeId]) -> bool;
}

/// View synchronization for handling view changes
#[async_trait]
pub trait ViewSynchronizer: Send + Sync {
    /// Start a view change to the next view
    async fn start_view_change(&mut self, new_view: ViewNumber) -> Result<()>;
    
    /// Handle an incoming view change message
    async fn handle_view_change(&mut self, from: NodeId, new_view: ViewNumber) -> Result<()>;
    
    /// Check if we should advance to a new view
    async fn should_advance_view(&self) -> Result<Option<ViewNumber>>;
    
    /// Synchronize with other nodes on the current view
    async fn sync_view(&mut self) -> Result<ViewNumber>;
}

/// Types of consensus messages
#[derive(Debug, Clone)]
pub enum ConsensusMessage {
    /// Block proposal
    Proposal {
        block: Block,
        view: ViewNumber,
        qc: Option<QuorumCertificate>,
    },
    /// Vote message
    Vote(Vote),
    /// View change message
    ViewChange {
        new_view: ViewNumber,
        from: NodeId,
        prepare_qc: Option<QuorumCertificate>,
    },
    /// New view message (from leader)
    NewView {
        view: ViewNumber,
        view_change_qc: QuorumCertificate,
    },
    /// Request for missing blocks
    SyncRequest {
        from_height: u64,
        to_height: u64,
    },
    /// Response with requested blocks
    SyncResponse {
        blocks: Vec<Block>,
    },
}

/// Consensus protocol statistics
#[derive(Debug, Clone, Default)]
pub struct ConsensusStats {
    /// Current view number
    pub current_view: u64,
    /// Total blocks committed
    pub blocks_committed: u64,
    /// Total transactions processed
    pub transactions_processed: u64,
    /// Average block time (milliseconds)
    pub avg_block_time_ms: u64,
    /// Current throughput (TPS)
    pub current_tps: f64,
    /// View changes count
    pub view_changes: u64,
    /// Network message counts
    pub messages_sent: u64,
    pub messages_received: u64,
    /// Consensus latency metrics
    pub avg_consensus_latency_ms: u64,
    pub p95_consensus_latency_ms: u64,
    pub p99_consensus_latency_ms: u64,
}

/// Configuration for timeout values
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Timeout for receiving proposals
    pub proposal_timeout_ms: u64,
    /// Timeout for vote collection
    pub vote_timeout_ms: u64,
    /// Timeout for view changes
    pub view_change_timeout_ms: u64,
    /// Base timeout that scales with view number
    pub base_timeout_ms: u64,
    /// Timeout multiplier for each view change
    pub timeout_multiplier: f64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            proposal_timeout_ms: 1000,    // 1 second
            vote_timeout_ms: 500,         // 500ms
            view_change_timeout_ms: 2000, // 2 seconds
            base_timeout_ms: 1000,        // 1 second
            timeout_multiplier: 1.5,      // 50% increase per view change
        }
    }
}

/// Consensus protocol configuration
#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    /// This node's identifier
    pub node_id: NodeId,
    /// List of validator nodes
    pub validators: Vec<NodeId>,
    /// Byzantine fault threshold (f in n = 3f + 1)
    pub byzantine_threshold: usize,
    /// Timeout configuration
    pub timeouts: TimeoutConfig,
    /// Maximum transactions per block
    pub max_transactions_per_block: usize,
    /// Block size limit in bytes
    pub max_block_size_bytes: usize,
    /// Enable fast path optimizations
    pub enable_fast_path: bool,
    /// Enable signature aggregation
    pub enable_signature_aggregation: bool,
}

impl ConsensusConfig {
    pub fn new(node_id: NodeId, validators: Vec<NodeId>) -> Self {
        let byzantine_threshold = (validators.len() - 1) / 3;
        
        Self {
            node_id,
            validators,
            byzantine_threshold,
            timeouts: TimeoutConfig::default(),
            max_transactions_per_block: 1000,
            max_block_size_bytes: 1024 * 1024, // 1MB
            enable_fast_path: true,
            enable_signature_aggregation: false,
        }
    }
    
    /// Calculate the minimum number of votes needed for a quorum
    pub fn quorum_size(&self) -> usize {
        2 * self.byzantine_threshold + 1
    }
    
    /// Check if the number of validators supports the Byzantine threshold
    pub fn is_valid(&self) -> bool {
        self.validators.len() >= 3 * self.byzantine_threshold + 1
    }
}