//! HotStuff-specific types and data structures

use crate::{Block, NodeId, Vote, QuorumCertificate, ViewNumber, VoteType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// HotStuff consensus protocol phases
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HotStuffPhase {
    /// Prepare phase - leader proposes block
    Prepare,
    /// Pre-commit phase - validators vote on prepare
    PreCommit,
    /// Commit phase - validators vote on pre-commit
    Commit,
    /// Decide phase - finalize commitment
    Decide,
    /// View change in progress
    ViewChange,
}

/// HotStuff consensus state
#[derive(Debug, Clone)]
pub struct HotStuffState {
    /// Current view number
    pub view: ViewNumber,
    /// Current consensus phase
    pub phase: HotStuffPhase,
    /// Highest QC we've seen
    pub high_qc: Option<QuorumCertificate>,
    /// Locked QC (for safety)
    pub locked_qc: Option<QuorumCertificate>,
    /// Last committed block
    pub last_committed_block: Option<Block>,
    /// Current pending block
    pub pending_block: Option<Block>,
    /// Generic QC for current view
    pub generic_qc: Option<QuorumCertificate>,
    /// Block tree for safety checks
    pub block_tree: BlockTree,
    /// Timeout state
    pub timeout_state: TimeoutState,
}

impl HotStuffState {
    pub fn new(initial_view: ViewNumber) -> Self {
        Self {
            view: initial_view,
            phase: HotStuffPhase::Prepare,
            high_qc: None,
            locked_qc: None,
            last_committed_block: None,
            pending_block: None,
            generic_qc: None,
            block_tree: BlockTree::new(),
            timeout_state: TimeoutState::new(),
        }
    }

    /// Advance to a new view
    pub fn advance_to_view(&mut self, new_view: ViewNumber) {
        if new_view > self.view {
            self.view = new_view;
            self.phase = HotStuffPhase::Prepare;
            self.pending_block = None;
            self.timeout_state.reset();
        }
    }

    /// Update the highest QC we've seen
    pub fn update_high_qc(&mut self, qc: QuorumCertificate) {
        if self.high_qc.as_ref().map_or(true, |current| qc.view > current.view) {
            self.high_qc = Some(qc);
        }
    }

    /// Check if we can commit a block (HotStuff 3-chain rule)
    pub fn check_commit_condition(&self) -> Option<Block> {
        // Implementation of 3-chain commit rule
        if let Some(ref high_qc) = self.high_qc {
            if let Some(block) = self.block_tree.get_block(&high_qc.block_id) {
                if self.block_tree.has_three_chain(&block.id) {
                    return Some(block.clone());
                }
            }
        }
        None
    }

    /// Check safety conditions before voting
    pub fn is_safe_to_vote(&self, _block: &Block, qc: Option<&QuorumCertificate>) -> bool {
        // Implement HotStuff safety rules
        if let Some(ref locked_qc) = self.locked_qc {
            if let Some(qc) = qc {
                // Can vote if QC extends locked block or is newer view
                return qc.view > locked_qc.view || 
                       self.block_tree.extends(&qc.block_id, &locked_qc.block_id);
            }
            return false;
        }
        true // Safe to vote if no lock
    }
}

/// Block tree for tracking block relationships and safety
#[derive(Debug, Clone)]
pub struct BlockTree {
    /// Blocks indexed by ID
    blocks: HashMap<Uuid, Block>,
    /// Parent-child relationships
    children: HashMap<Uuid, Vec<Uuid>>,
    /// Genesis block ID
    genesis_id: Option<Uuid>,
}

impl BlockTree {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            children: HashMap::new(),
            genesis_id: None,
        }
    }

    /// Add a block to the tree
    pub fn add_block(&mut self, block: Block) {
        let parent_id = if block.parent_hash.is_empty() {
            self.genesis_id = Some(block.id);
            None
        } else {
            // Find parent by hash (simplified)
            self.blocks.values()
                .find(|b| b.hash() == block.parent_hash)
                .map(|b| b.id)
        };

        if let Some(parent_id) = parent_id {
            self.children.entry(parent_id)
                .or_insert_with(Vec::new)
                .push(block.id);
        }

        self.blocks.insert(block.id, block);
    }

    /// Get a block by ID
    pub fn get_block(&self, id: &Uuid) -> Option<&Block> {
        self.blocks.get(id)
    }

    /// Check if block A extends block B
    pub fn extends(&self, block_a: &Uuid, block_b: &Uuid) -> bool {
        if block_a == block_b {
            return true;
        }

        if let Some(block_a) = self.blocks.get(block_a) {
            let parent_hash = &block_a.parent_hash;
            if let Some(parent) = self.blocks.values().find(|b| b.hash() == *parent_hash) {
                return self.extends(&parent.id, block_b);
            }
        }
        false
    }

    /// Check if a block has a 3-chain (for commit rule)
    pub fn has_three_chain(&self, block_id: &Uuid) -> bool {
        let mut current_id = *block_id;
        let mut chain_length = 0;

        while let Some(block) = self.blocks.get(&current_id) {
            chain_length += 1;
            if chain_length >= 3 {
                return true;
            }

            // Find parent
            if let Some(parent) = self.blocks.values().find(|b| b.hash() == block.parent_hash) {
                current_id = parent.id;
            } else {
                break;
            }
        }

        false
    }
}

/// Timeout state management
#[derive(Debug, Clone)]
pub struct TimeoutState {
    /// Last timeout timestamp
    pub last_timeout: Option<DateTime<Utc>>,
    /// Current timeout duration
    pub current_timeout_ms: u64,
    /// Number of consecutive timeouts
    pub timeout_count: u32,
}

impl TimeoutState {
    pub fn new() -> Self {
        Self {
            last_timeout: None,
            current_timeout_ms: 1000, // 1 second default
            timeout_count: 0,
        }
    }

    pub fn reset(&mut self) {
        self.last_timeout = None;
        self.current_timeout_ms = 1000;
        self.timeout_count = 0;
    }

    pub fn record_timeout(&mut self) {
        self.last_timeout = Some(Utc::now());
        self.timeout_count += 1;
        // Exponential backoff
        self.current_timeout_ms = (self.current_timeout_ms as f64 * 1.5) as u64;
    }

    pub fn should_timeout(&self, timeout_ms: u64) -> bool {
        if let Some(last_timeout) = self.last_timeout {
            let elapsed = Utc::now().signed_duration_since(last_timeout);
            elapsed.num_milliseconds() as u64 > timeout_ms
        } else {
            false
        }
    }
}

/// HotStuff-specific message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HotStuffMessage {
    /// Block proposal with justification
    Proposal {
        block: Block,
        view: ViewNumber,
        justify: Option<QuorumCertificate>,
    },
    /// Vote message
    Vote(Vote),
    /// New view message from leader
    NewView {
        view: ViewNumber,
        high_qc: QuorumCertificate,
    },
    /// Timeout notification
    TimeoutNotification {
        view: ViewNumber,
    },
}

impl HotStuffMessage {
    /// Convert to generic consensus message
    pub fn into_consensus_message(self) -> crate::traits::ConsensusMessage {
        match self {
            HotStuffMessage::Proposal { block, view, justify } => {
                crate::traits::ConsensusMessage::Proposal {
                    block,
                    view,
                    qc: justify,
                }
            }
            HotStuffMessage::Vote(vote) => {
                crate::traits::ConsensusMessage::Vote(vote)
            }
            HotStuffMessage::NewView { view, high_qc } => {
                crate::traits::ConsensusMessage::NewView {
                    view,
                    view_change_qc: high_qc,
                }
            }
            HotStuffMessage::TimeoutNotification { view } => {
                crate::traits::ConsensusMessage::ViewChange {
                    new_view: view.next(),
                    from: NodeId::new(), // Will be filled by network layer
                    prepare_qc: None,
                }
            }
        }
    }
}

/// Vote aggregation and quorum certificate formation
#[derive(Debug)]
pub struct VoteAggregator {
    /// Votes by view and vote type
    votes: HashMap<(ViewNumber, VoteType), HashMap<NodeId, Vote>>,
    /// Required quorum size
    quorum_size: usize,
}

impl VoteAggregator {
    pub fn new(quorum_size: usize) -> Self {
        Self {
            votes: HashMap::new(),
            quorum_size,
        }
    }

    /// Add a vote and return QC if quorum reached
    pub fn add_vote(&mut self, vote: Vote) -> Option<QuorumCertificate> {
        let key = (vote.view, vote.vote_type.clone());
        let vote_map = self.votes.entry(key).or_insert_with(HashMap::new);
        
        // Add the vote
        vote_map.insert(vote.voter.clone(), vote);
        
        // Check if we have quorum
        if vote_map.len() >= self.quorum_size {
            let votes: Vec<Vote> = vote_map.values().cloned().collect();
            Some(QuorumCertificate::new(votes))
        } else {
            None
        }
    }

    /// Clear old votes for a view
    pub fn clear_view(&mut self, view: ViewNumber) {
        self.votes.retain(|(v, _), _| *v >= view);
    }
}
