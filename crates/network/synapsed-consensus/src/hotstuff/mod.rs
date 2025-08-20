//! HotStuff consensus protocol implementation
//!
//! HotStuff is a leader-based Byzantine fault-tolerant replication protocol
//! that provides safety and liveness in a partially synchronous network with
//! up to f < n/3 Byzantine faults.
//!
//! ## Key Features
//! - **Linear message complexity**: O(n) messages per view
//! - **Pipelined consensus**: Overlapping phases for high throughput
//! - **Responsive**: Commits in 2 round trips when network is stable
//! - **View synchronization**: Efficient leader rotation mechanism

pub mod types;
pub mod leader;
pub mod voting;

use crate::{
    Block, NodeId, Vote, QuorumCertificate, ViewNumber, Transaction, VoteType,
    ConsensusError, Result,
    traits::{ConsensusProtocol, ConsensusStats, NetworkTransport, ConsensusCrypto, StateMachine, 
             ConsensusConfig, LeaderElection}
};
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info, warn};

pub use self::types::*;
pub use self::leader::*;
pub use self::voting::*;

/// HotStuff consensus protocol implementation
pub struct HotStuffConsensus<N, C, S> 
where
    N: NetworkTransport,
    C: ConsensusCrypto,
    S: StateMachine,
{
    /// Node identifier
    node_id: NodeId,
    /// Configuration
    config: ConsensusConfig,
    /// Current consensus state
    state: Arc<RwLock<HotStuffState>>,
    /// Network transport layer
    network: Arc<N>,
    /// Cryptographic operations
    crypto: Arc<C>,
    /// State machine
    state_machine: Arc<Mutex<S>>,
    /// Leader election strategy
    leader_election: Arc<dyn LeaderElection>,
    /// Vote aggregator
    vote_aggregator: Arc<Mutex<voting::VoteCollector>>,
    /// Message handler
    message_sender: mpsc::UnboundedSender<HotStuffMessage>,
    /// Running state
    is_running: Arc<RwLock<bool>>,
    /// Statistics
    stats: Arc<RwLock<ConsensusStats>>,
}

impl<N, C, S> HotStuffConsensus<N, C, S>
where
    N: NetworkTransport + 'static,
    C: ConsensusCrypto + 'static,
    S: StateMachine + 'static,
{
    /// Create a new HotStuff consensus instance
    pub async fn new(
        config: ConsensusConfig,
        network: Arc<N>,
        crypto: Arc<C>,
        state_machine: Arc<Mutex<S>>,
    ) -> Result<Self> {
        let node_id = config.node_id.clone();
        let leader_election = Arc::new(leader::RoundRobinLeaderElection::new());
        let state = Arc::new(RwLock::new(types::HotStuffState::new(ViewNumber::new(0))));
        let vote_aggregator = Arc::new(Mutex::new(voting::VoteCollector::new(config.quorum_size())));
        let (message_sender, _) = mpsc::unbounded_channel();
        
        Ok(Self {
            node_id,
            config,
            state,
            network,
            crypto,
            state_machine,
            leader_election,
            vote_aggregator,
            message_sender,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(ConsensusStats::default())),
        })
    }

    /// Check if this node is the current leader
    pub fn is_current_leader(&self) -> bool {
        let state = self.state.read();
        self.leader_election.is_leader(&self.node_id, state.view, &self.config.validators)
    }

    /// Get current leader for the view
    pub fn get_current_leader(&self) -> NodeId {
        let state = self.state.read();
        self.leader_election.get_leader(state.view, &self.config.validators)
    }

    /// Handle incoming HotStuff message
    pub async fn handle_message(&self, from: NodeId, message: HotStuffMessage) -> Result<()> {
        match message {
            HotStuffMessage::Proposal { block, view, justify } => {
                self.handle_proposal(from, block, view, justify).await
            }
            HotStuffMessage::Vote(vote) => {
                self.handle_vote_message(from, vote).await
            }
            HotStuffMessage::NewView { view, high_qc } => {
                self.handle_new_view(from, view, high_qc).await
            }
            HotStuffMessage::TimeoutNotification { view } => {
                self.handle_timeout(from, view).await
            }
        }
    }

    /// Handle a block proposal
    async fn handle_proposal(
        &self,
        from: NodeId,
        block: Block,
        view: ViewNumber,
        justify: Option<QuorumCertificate>,
    ) -> Result<()> {
        debug!("Handling proposal from {} for view {}", from, view);

        // Verify sender is the expected leader
        let expected_leader = self.leader_election.get_leader(view, &self.config.validators);
        if from != expected_leader {
            warn!("Received proposal from {} but expected leader is {}", from, expected_leader);
            return Err(ConsensusError::InvalidLeader);
        }

        // Update state
        {
            let mut state = self.state.write();
            if view < state.view {
                debug!("Ignoring proposal for old view {}", view);
                return Ok(());
            }
            
            if view > state.view {
                state.advance_to_view(view);
            }
            
            state.pending_block = Some(block.clone());
            
            if let Some(qc) = justify {
                state.update_high_qc(qc);
            }
        }

        // Validate block through state machine
        let is_valid = {
            let state_machine = self.state_machine.lock().await;
            state_machine.validate_block(&block).await?
        };

        if !is_valid {
            warn!("Block validation failed for proposal from {}", from);
            return Err(ConsensusError::InvalidBlock);
        }

        // Create and send vote
        let vote = self.create_vote(crate::types::VoteType::Prepare, view, block.id).await?;
        self.broadcast_vote(vote).await?;

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.messages_received += 1;
        }

        Ok(())
    }

    /// Handle a vote message
    async fn handle_vote_message(&self, from: NodeId, vote: Vote) -> Result<()> {
        debug!("Handling vote from {} for view {}", from, vote.view);

        // Verify vote signature
        let vote_data = self.serialize_vote_data(&vote)?;
        if !self.crypto.verify(&from, &vote_data, &vote.signature).await? {
            warn!("Invalid vote signature from {}", from);
            return Err(ConsensusError::InvalidSignature);
        }

        // Add vote to aggregator
        let qc_opt = {
            let mut aggregator = self.vote_aggregator.lock().await;
            aggregator.add_vote(vote.clone())?
        };

        // Check if we formed a quorum certificate
        if let Some(qc) = qc_opt {
            self.handle_quorum_certificate(qc).await?;
        }

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.messages_received += 1;
        }

        Ok(())
    }

    /// Handle formation of a quorum certificate
    async fn handle_quorum_certificate(&self, qc: QuorumCertificate) -> Result<()> {
        info!("Formed quorum certificate for view {}", qc.view);

        let committed_block_opt = {
            let mut state = self.state.write();
            state.update_high_qc(qc.clone());
            state.check_commit_condition()
        };
        
        if let Some(committed_block) = committed_block_opt {
            info!("Committing block at height {}", committed_block.height);
            
            // Apply block to state machine
            {
                let mut state_machine = self.state_machine.lock().await;
                state_machine.apply_block(&committed_block).await?;
            }
            
            // Update state after state machine application
            {
                let mut state = self.state.write();
                state.last_committed_block = Some(committed_block.clone());
            }
            
            // Update statistics
            {
                let mut stats = self.stats.write();
                stats.blocks_committed += 1;
                stats.transactions_processed += committed_block.transactions.len() as u64;
            }
        }

        // If we're the next leader, propose next block
        let next_view = qc.view.next();
        if self.leader_election.is_leader(&self.node_id, next_view, &self.config.validators) {
            self.propose_next_block(next_view, qc).await?;
        }

        Ok(())
    }

    /// Propose the next block as a leader
    async fn propose_next_block(&self, view: ViewNumber, justify: QuorumCertificate) -> Result<()> {
        info!("Proposing block for view {} as leader", view);

        // Get transactions from mempool (simulated for now)
        let transactions = self.get_transactions_for_block().await?;
        
        // Create new block
        let parent_hash = {
            let state = self.state.read();
            state.last_committed_block
                .as_ref()
                .map(|b| b.hash())
                .unwrap_or_default()
        };
        
        let height = {
            let state = self.state.read();
            state.last_committed_block
                .as_ref()
                .map(|b| b.height + 1)
                .unwrap_or(1)
        };

        let block = Block::new(parent_hash, height, transactions, self.node_id.clone());

        // Broadcast proposal
        let message = HotStuffMessage::Proposal {
            block: block.clone(),
            view,
            justify: Some(justify),
        };

        self.broadcast_message(message).await?;

        // Update state
        {
            let mut state = self.state.write();
            state.advance_to_view(view);
            state.pending_block = Some(block);
        }

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.messages_sent += self.config.validators.len() as u64;
        }

        Ok(())
    }

    /// Create a vote for the given parameters
    async fn create_vote(
        &self,
        vote_type: VoteType,
        view: ViewNumber,
        block_id: uuid::Uuid,
    ) -> Result<Vote> {
        let vote_data = self.serialize_vote_data(&Vote {
            vote_type: vote_type.clone(),
            view,
            block_id,
            voter: self.node_id.clone(),
            signature: vec![], // Will be filled after signing
            timestamp: Utc::now(),
        })?;

        let signature = self.crypto.sign(&vote_data).await?;

        Ok(Vote::new(vote_type, view, block_id, self.node_id.clone(), signature))
    }

    /// Serialize vote data for signing
    fn serialize_vote_data(&self, vote: &Vote) -> Result<Vec<u8>> {
        let vote_for_signing = VoteForSigning {
            vote_type: vote.vote_type.clone(),
            view: vote.view,
            block_id: vote.block_id,
            voter: vote.voter.clone(),
        };
        
        serde_json::to_vec(&vote_for_signing)
            .map_err(|e| ConsensusError::SerializationError(e.to_string()))
    }

    /// Broadcast a vote to all validators
    async fn broadcast_vote(&self, vote: Vote) -> Result<()> {
        let message = HotStuffMessage::Vote(vote);
        self.broadcast_message(message).await
    }

    /// Broadcast a message to all validators
    async fn broadcast_message(&self, message: HotStuffMessage) -> Result<()> {
        // Convert to consensus message
        let consensus_msg = message.into_consensus_message();
        
        // Broadcast through network layer
        self.network.broadcast(consensus_msg).await?;
        
        Ok(())
    }

    /// Get transactions for the next block (placeholder)
    async fn get_transactions_for_block(&self) -> Result<Vec<Transaction>> {
        // This would typically come from a mempool
        // For now, return empty transactions
        Ok(vec![])
    }

    /// Handle new view message
    async fn handle_new_view(
        &self,
        _from: NodeId,
        view: ViewNumber,
        _high_qc: QuorumCertificate,
    ) -> Result<()> {
        debug!("Handling new view message for view {}", view);
        
        {
            let mut state = self.state.write();
            if view > state.view {
                state.advance_to_view(view);
            }
        }
        
        Ok(())
    }

    /// Handle timeout notification
    async fn handle_timeout(&self, _from: NodeId, view: ViewNumber) -> Result<()> {
        debug!("Handling timeout for view {}", view);
        
        // Initiate view change
        let new_view = view.next();
        {
            let mut state = self.state.write();
            if new_view > state.view {
                state.advance_to_view(new_view);
            }
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.view_changes += 1;
        }
        
        Ok(())
    }
}

#[async_trait]
impl<N, C, S> ConsensusProtocol for HotStuffConsensus<N, C, S>
where
    N: NetworkTransport + 'static,
    C: ConsensusCrypto + 'static,
    S: StateMachine + 'static,
{
    async fn start(&mut self) -> Result<()> {
        info!("Starting HotStuff consensus for node {}", self.node_id);
        
        {
            let mut is_running = self.is_running.write();
            if *is_running {
                return Err(ConsensusError::AlreadyRunning);
            }
            *is_running = true;
        }

        // Initialize consensus state
        {
            let mut state = self.state.write();
            state.phase = types::HotStuffPhase::Prepare;
        }

        info!("HotStuff consensus started successfully");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        info!("Stopping HotStuff consensus");
        
        {
            let mut is_running = self.is_running.write();
            *is_running = false;
        }

        info!("HotStuff consensus stopped");
        Ok(())
    }

    async fn propose_block(&mut self, transactions: Vec<Transaction>) -> Result<Block> {
        if !self.is_current_leader() {
            return Err(ConsensusError::NotLeader);
        }

        let parent_hash = {
            let state = self.state.read();
            state.last_committed_block
                .as_ref()
                .map(|b| b.hash())
                .unwrap_or_default()
        };
        
        let height = {
            let state = self.state.read();
            state.last_committed_block
                .as_ref()
                .map(|b| b.height + 1)
                .unwrap_or(1)
        };

        let block = Block::new(parent_hash, height, transactions, self.node_id.clone());
        
        // Broadcast proposal
        let message = HotStuffMessage::Proposal {
            block: block.clone(),
            view: self.state.read().view,
            justify: self.state.read().high_qc.clone(),
        };

        self.broadcast_message(message).await?;

        Ok(block)
    }

    async fn handle_vote(&mut self, vote: Vote) -> Result<()> {
        self.handle_vote_message(vote.voter.clone(), vote).await
    }

    async fn handle_proposal(&mut self, block: Block) -> Result<()> {
        let message = HotStuffMessage::Proposal {
            view: self.state.read().view,
            block,
            justify: None,
        };
        
        self.handle_message(self.node_id.clone(), message).await
    }

    fn current_view(&self) -> ViewNumber {
        self.state.read().view
    }

    fn is_leader(&self) -> bool {
        self.is_current_leader()
    }

    fn current_leader(&self) -> Option<NodeId> {
        Some(self.get_current_leader())
    }

    fn get_stats(&self) -> ConsensusStats {
        let stats = self.stats.read();
        let state = self.state.read();
        
        ConsensusStats {
            current_view: state.view.as_u64(),
            blocks_committed: stats.blocks_committed,
            transactions_processed: stats.transactions_processed,
            avg_block_time_ms: stats.avg_block_time_ms,
            current_tps: stats.current_tps,
            view_changes: stats.view_changes,
            messages_sent: stats.messages_sent,
            messages_received: stats.messages_received,
            avg_consensus_latency_ms: stats.avg_consensus_latency_ms,
            p95_consensus_latency_ms: stats.p95_consensus_latency_ms,
            p99_consensus_latency_ms: stats.p99_consensus_latency_ms,
        }
    }
}

/// Voting data structure for signing (without signature field)
#[derive(serde::Serialize)]
struct VoteForSigning {
    vote_type: VoteType,
    view: ViewNumber,
    block_id: uuid::Uuid,
    voter: NodeId,
}