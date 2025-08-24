//! Byzantine Fault Tolerant Consensus for Swarm Coordination
//! 
//! This module provides consensus mechanisms for critical swarm decisions,
//! implementing PBFT (Practical Byzantine Fault Tolerance) with three-phase commit
//! to ensure agreement among agents even when some are Byzantine faulty.

use crate::{
    error::{SwarmError, SwarmResult},
    types::*,
    protocol::{AgentMessage, MessageType, MessagePayload},
};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{RwLock, watch},
    time::{sleep, timeout, Instant},
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Consensus protocol trait for different algorithms
#[async_trait]
pub trait ConsensusProtocol: Send + Sync {
    /// Start the consensus protocol
    async fn start(&mut self) -> SwarmResult<()>;
    
    /// Stop the consensus protocol gracefully
    async fn stop(&mut self) -> SwarmResult<()>;
    
    /// Initiate consensus on a proposal
    async fn propose(&self, proposal: ConsensusProposal) -> SwarmResult<ProposalId>;
    
    /// Handle incoming consensus message
    async fn handle_message(&self, message: ConsensusMessage) -> SwarmResult<()>;
    
    /// Get the result of a consensus round
    async fn get_result(&self, proposal_id: ProposalId) -> SwarmResult<Option<ConsensusResult>>;
    
    /// Check if enough agents are available for consensus
    fn can_achieve_consensus(&self) -> bool;
    
    /// Get current consensus statistics
    fn get_stats(&self) -> ConsensusStats;
}

/// PBFT Consensus implementation with three-phase commit
pub struct PBFTConsensus {
    /// Swarm ID this consensus instance belongs to
    swarm_id: SwarmId,
    /// This agent's ID
    agent_id: AgentId,
    /// List of participating agents
    agents: Arc<RwLock<Vec<AgentId>>>,
    /// Byzantine fault tolerance threshold (f in 3f+1 formula)
    byzantine_threshold: usize,
    /// Active voting rounds
    voting_rounds: Arc<DashMap<ProposalId, VotingRound>>,
    /// Completed consensus results
    results: Arc<DashMap<ProposalId, ConsensusResult>>,
    /// Message sender for communication
    message_sender: Arc<dyn ConsensusCommunication>,
    /// Configuration
    config: ConsensusConfig,
    /// Consensus statistics
    stats: Arc<RwLock<ConsensusStats>>,
    /// Running state
    running: Arc<RwLock<bool>>,
}

/// Voting round state for PBFT three-phase commit
#[derive(Debug, Clone)]
pub struct VotingRound {
    /// Unique proposal ID
    pub proposal_id: ProposalId,
    /// The proposal being voted on
    pub proposal: ConsensusProposal,
    /// Current phase of the consensus
    pub phase: ConsensusPhase,
    /// Proposer (primary) for this round
    pub proposer: AgentId,
    /// View number (for view changes)
    pub view: u64,
    /// Start time of the round
    pub started_at: Instant,
    /// Timeout for this round
    pub timeout: Duration,
    /// Pre-prepare messages received
    pub pre_prepare: Option<PrePrepareMessage>,
    /// Prepare votes received
    pub prepare_votes: HashMap<AgentId, PrepareMessage>,
    /// Commit votes received
    pub commit_votes: HashMap<AgentId, CommitMessage>,
    /// Whether we've sent our prepare vote
    pub prepare_sent: bool,
    /// Whether we've sent our commit vote
    pub commit_sent: bool,
    /// Final result
    pub result: Option<ConsensusResult>,
}

/// Quorum Certificate for Byzantine fault tolerance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumCertificate {
    /// Proposal this certificate is for
    pub proposal_id: ProposalId,
    /// Phase this certificate covers
    pub phase: ConsensusPhase,
    /// View number
    pub view: u64,
    /// Signatures from agents
    pub signatures: Vec<ConsensusSignature>,
    /// Timestamp when certificate was created
    pub created_at: DateTime<Utc>,
}

/// Consensus phases in PBFT
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusPhase {
    /// Pre-prepare phase (primary sends proposal)
    PrePrepare,
    /// Prepare phase (agents vote to prepare)
    Prepare,
    /// Commit phase (agents vote to commit)
    Commit,
    /// Committed (consensus reached)
    Committed,
    /// Failed (consensus failed)
    Failed,
}

/// Types of proposals that can be voted on
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusProposal {
    /// Agent join decision
    AgentJoin {
        agent_id: AgentId,
        role: AgentRole,
        trust_score: f64,
    },
    /// Agent removal decision
    AgentRemoval {
        agent_id: AgentId,
        reason: String,
    },
    /// Critical task assignment
    CriticalTask {
        task_id: TaskId,
        intent: serde_json::Value, // Serialized HierarchicalIntent
        requirements: TaskRequirements,
    },
    /// Trust score adjustment
    TrustAdjustment {
        agent_id: AgentId,
        adjustment: f64,
        reason: String,
    },
    /// Configuration change
    ConfigurationChange {
        parameter: String,
        new_value: serde_json::Value,
    },
    /// Emergency action
    EmergencyAction {
        action: String,
        reason: String,
        affected_agents: Vec<AgentId>,
    },
}

/// Requirements for critical tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequirements {
    /// Minimum trust score required
    pub min_trust_score: f64,
    /// Required capabilities
    pub required_capabilities: Vec<String>,
    /// Verification level required
    pub verification_level: VerificationLevel,
    /// Maximum execution time
    pub max_execution_time: Duration,
}

/// Verification levels for tasks
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VerificationLevel {
    /// Basic verification
    Basic,
    /// Enhanced verification with proofs
    Enhanced,
    /// Critical verification with multiple validators
    Critical,
}

/// Consensus message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMessage {
    /// Pre-prepare message from primary
    PrePrepare(PrePrepareMessage),
    /// Prepare vote from backup
    Prepare(PrepareMessage),
    /// Commit vote from backup
    Commit(CommitMessage),
    /// View change request
    ViewChange(ViewChangeMessage),
    /// New view announcement
    NewView(NewViewMessage),
    /// Checkpoint message
    Checkpoint(CheckpointMessage),
}

/// Pre-prepare message (phase 1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrePrepareMessage {
    pub proposal_id: ProposalId,
    pub view: u64,
    pub sequence: u64,
    pub proposal: ConsensusProposal,
    pub proposer: AgentId,
    pub timestamp: DateTime<Utc>,
    pub signature: ConsensusSignature,
}

/// Prepare message (phase 2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareMessage {
    pub proposal_id: ProposalId,
    pub view: u64,
    pub sequence: u64,
    pub agent_id: AgentId,
    pub digest: Vec<u8>, // Hash of the proposal
    pub timestamp: DateTime<Utc>,
    pub signature: ConsensusSignature,
}

/// Commit message (phase 3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitMessage {
    pub proposal_id: ProposalId,
    pub view: u64,
    pub sequence: u64,
    pub agent_id: AgentId,
    pub digest: Vec<u8>,
    pub timestamp: DateTime<Utc>,
    pub signature: ConsensusSignature,
}

/// View change message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewChangeMessage {
    pub new_view: u64,
    pub agent_id: AgentId,
    pub last_stable_checkpoint: u64,
    pub checkpoint_proof: Vec<ConsensusSignature>,
    pub prepared_certificates: Vec<PreparedCertificate>,
    pub timestamp: DateTime<Utc>,
    pub signature: ConsensusSignature,
}

/// New view message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewViewMessage {
    pub view: u64,
    pub view_change_messages: Vec<ViewChangeMessage>,
    pub pre_prepare_messages: Vec<PrePrepareMessage>,
    pub primary: AgentId,
    pub timestamp: DateTime<Utc>,
    pub signature: ConsensusSignature,
}

/// Checkpoint message for garbage collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMessage {
    pub sequence: u64,
    pub digest: Vec<u8>,
    pub agent_id: AgentId,
    pub timestamp: DateTime<Utc>,
    pub signature: ConsensusSignature,
}

/// Certificate for prepared requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedCertificate {
    pub proposal_id: ProposalId,
    pub view: u64,
    pub sequence: u64,
    pub digest: Vec<u8>,
    pub prepare_signatures: Vec<ConsensusSignature>,
}

/// Digital signature for consensus messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusSignature {
    pub signer: AgentId,
    pub signature: Vec<u8>,
    pub algorithm: String,
}

/// Result of consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    pub proposal_id: ProposalId,
    pub proposal: ConsensusProposal,
    pub decision: ConsensusDecision,
    pub view: u64,
    pub participating_agents: Vec<AgentId>,
    pub quorum_certificate: QuorumCertificate,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u64,
}

/// Final decision of consensus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusDecision {
    /// Proposal accepted
    Accepted,
    /// Proposal rejected
    Rejected,
    /// Consensus failed (timeout, insufficient participants, etc.)
    Failed,
}

/// Configuration for consensus
#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    /// Timeout for each consensus round
    pub round_timeout: Duration,
    /// Timeout for view changes
    pub view_change_timeout: Duration,
    /// Maximum number of view changes before giving up
    pub max_view_changes: u32,
    /// Checkpoint interval (in sequence numbers)
    pub checkpoint_interval: u64,
    /// Enable fast path optimizations
    pub enable_fast_path: bool,
    /// Signature algorithm to use
    pub signature_algorithm: String,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            round_timeout: Duration::from_secs(30),
            view_change_timeout: Duration::from_secs(60),
            max_view_changes: 3,
            checkpoint_interval: 100,
            enable_fast_path: true,
            signature_algorithm: "ed25519".to_string(),
        }
    }
}

/// Statistics for consensus performance
#[derive(Debug, Clone, Default)]
pub struct ConsensusStats {
    /// Total proposals initiated
    pub proposals_initiated: u64,
    /// Total proposals completed
    pub proposals_completed: u64,
    /// Total proposals failed
    pub proposals_failed: u64,
    /// Average consensus duration
    pub avg_consensus_duration_ms: f64,
    /// View changes count
    pub view_changes: u64,
    /// Current view number
    pub current_view: u64,
    /// Messages sent
    pub messages_sent: u64,
    /// Messages received
    pub messages_received: u64,
}

/// Communication interface for sending consensus messages
#[async_trait]
pub trait ConsensusCommunication: Send + Sync {
    /// Send a message to a specific agent
    async fn send_to_agent(&self, agent_id: AgentId, message: ConsensusMessage) -> SwarmResult<()>;
    
    /// Broadcast a message to all agents
    async fn broadcast(&self, message: ConsensusMessage) -> SwarmResult<()>;
    
    /// Get list of active agents
    async fn get_active_agents(&self) -> SwarmResult<Vec<AgentId>>;
}

/// Type aliases for clarity
pub type ProposalId = Uuid;
pub type SequenceNumber = u64;

impl PBFTConsensus {
    /// Create a new PBFT consensus instance
    pub fn new(
        swarm_id: SwarmId,
        agent_id: AgentId,
        message_sender: Arc<dyn ConsensusCommunication>,
        config: ConsensusConfig,
    ) -> Self {
        Self {
            swarm_id,
            agent_id,
            agents: Arc::new(RwLock::new(Vec::new())),
            byzantine_threshold: 0,
            voting_rounds: Arc::new(DashMap::new()),
            results: Arc::new(DashMap::new()),
            message_sender,
            config,
            stats: Arc::new(RwLock::new(ConsensusStats::default())),
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Add an agent to the consensus group
    pub async fn add_agent(&self, agent_id: AgentId) -> SwarmResult<()> {
        let mut agents = self.agents.write().await;
        if !agents.contains(&agent_id) {
            agents.push(agent_id);
            // Recalculate Byzantine threshold: f = (n-1)/3
            let n = agents.len();
            let f = (n.saturating_sub(1)) / 3;
            drop(agents);
            
            // Update byzantine threshold in a thread-safe way
            // Note: In a real implementation, this would need to be properly synchronized
            // For now, we'll store it as a field that gets updated
            info!(
                "Agent {} added to consensus group. Total: {}, Byzantine threshold: {}",
                agent_id, n, f
            );
        }
        Ok(())
    }
    
    /// Remove an agent from the consensus group
    pub async fn remove_agent(&self, agent_id: AgentId) -> SwarmResult<()> {
        let mut agents = self.agents.write().await;
        agents.retain(|&id| id != agent_id);
        let n = agents.len();
        let f = (n.saturating_sub(1)) / 3;
        info!(
            "Agent {} removed from consensus group. Total: {}, Byzantine threshold: {}",
            agent_id, n, f
        );
        Ok(())
    }
    
    /// Check if we have enough agents for Byzantine fault tolerance
    pub async fn has_sufficient_agents(&self) -> bool {
        let agents = self.agents.read().await;
        agents.len() >= 4 // Minimum for 3f+1 with f=1
    }
    
    /// Get the primary (proposer) for a given view
    fn get_primary(&self, view: u64, agents: &[AgentId]) -> Option<AgentId> {
        if agents.is_empty() {
            return None;
        }
        let index = (view as usize) % agents.len();
        agents.get(index).copied()
    }
    
    /// Check if this agent is the primary for the current view
    async fn is_primary(&self, view: u64) -> bool {
        let agents = self.agents.read().await;
        self.get_primary(view, &agents) == Some(self.agent_id)
    }
    
    /// Calculate quorum size (2f + 1)
    fn calculate_quorum_size(&self, total_agents: usize) -> usize {
        let f = (total_agents.saturating_sub(1)) / 3;
        2 * f + 1
    }
    
    /// Start pre-prepare phase
    async fn start_pre_prepare(&self, proposal: ConsensusProposal, view: u64) -> SwarmResult<ProposalId> {
        let proposal_id = Uuid::new_v4();
        let sequence = self.get_next_sequence().await;
        
        let pre_prepare_msg = PrePrepareMessage {
            proposal_id,
            view,
            sequence,
            proposal: proposal.clone(),
            proposer: self.agent_id,
            timestamp: Utc::now(),
            signature: self.sign_message(&proposal).await?,
        };
        
        // Create voting round
        let voting_round = VotingRound {
            proposal_id,
            proposal,
            phase: ConsensusPhase::PrePrepare,
            proposer: self.agent_id,
            view,
            started_at: Instant::now(),
            timeout: self.config.round_timeout,
            pre_prepare: Some(pre_prepare_msg.clone()),
            prepare_votes: HashMap::new(),
            commit_votes: HashMap::new(),
            prepare_sent: false,
            commit_sent: false,
            result: None,
        };
        
        self.voting_rounds.insert(proposal_id, voting_round);
        
        // Broadcast pre-prepare message
        let consensus_msg = ConsensusMessage::PrePrepare(pre_prepare_msg);
        self.message_sender.broadcast(consensus_msg).await?;
        
        info!("Started pre-prepare phase for proposal {}", proposal_id);
        Ok(proposal_id)
    }
    
    /// Handle pre-prepare message
    async fn handle_pre_prepare(&self, msg: PrePrepareMessage) -> SwarmResult<()> {
        debug!("Handling pre-prepare message for proposal {}", msg.proposal_id);
        
        // Verify the message is from the expected primary
        let agents = self.agents.read().await;
        let expected_primary = self.get_primary(msg.view, &agents);
        if expected_primary != Some(msg.proposer) {
            warn!("Pre-prepare from unexpected primary: {}", msg.proposer);
            return Ok(());
        }
        drop(agents);
        
        // Verify signature
        if !self.verify_signature(&msg.signature, &msg.proposal).await? {
            warn!("Invalid signature in pre-prepare message");
            return Ok(());
        }
        
        // Create or update voting round
        let mut voting_round = VotingRound {
            proposal_id: msg.proposal_id,
            proposal: msg.proposal.clone(),
            phase: ConsensusPhase::Prepare,
            proposer: msg.proposer,
            view: msg.view,
            started_at: Instant::now(),
            timeout: self.config.round_timeout,
            pre_prepare: Some(msg),
            prepare_votes: HashMap::new(),
            commit_votes: HashMap::new(),
            prepare_sent: false,
            commit_sent: false,
            result: None,
        };
        
        self.voting_rounds.insert(msg.proposal_id, voting_round.clone());
        
        // Send prepare message
        let prepare_msg = PrepareMessage {
            proposal_id: msg.proposal_id,
            view: msg.view,
            sequence: msg.sequence,
            agent_id: self.agent_id,
            digest: self.hash_proposal(&msg.proposal).await?,
            timestamp: Utc::now(),
            signature: self.sign_message(&msg.proposal).await?,
        };
        
        voting_round.prepare_sent = true;
        self.voting_rounds.insert(msg.proposal_id, voting_round);
        
        let consensus_msg = ConsensusMessage::Prepare(prepare_msg);
        self.message_sender.broadcast(consensus_msg).await?;
        
        Ok(())
    }
    
    /// Handle prepare message
    async fn handle_prepare(&self, msg: PrepareMessage) -> SwarmResult<()> {
        debug!("Handling prepare message for proposal {} from {}", 
               msg.proposal_id, msg.agent_id);
        
        if let Some(mut voting_round) = self.voting_rounds.get_mut(&msg.proposal_id) {
            // Verify signature
            if !self.verify_signature(&msg.signature, &voting_round.proposal).await? {
                warn!("Invalid signature in prepare message from {}", msg.agent_id);
                return Ok(());
            }
            
            // Add prepare vote
            voting_round.prepare_votes.insert(msg.agent_id, msg);
            
            // Check if we have enough prepare votes
            let agents = self.agents.read().await;
            let quorum_size = self.calculate_quorum_size(agents.len());
            
            if voting_round.prepare_votes.len() >= quorum_size && !voting_round.commit_sent {
                // Move to commit phase
                voting_round.phase = ConsensusPhase::Commit;
                voting_round.commit_sent = true;
                
                let commit_msg = CommitMessage {
                    proposal_id: msg.proposal_id,
                    view: msg.view,
                    sequence: msg.sequence,
                    agent_id: self.agent_id,
                    digest: self.hash_proposal(&voting_round.proposal).await?,
                    timestamp: Utc::now(),
                    signature: self.sign_message(&voting_round.proposal).await?,
                };
                
                let consensus_msg = ConsensusMessage::Commit(commit_msg);
                self.message_sender.broadcast(consensus_msg).await?;
                
                info!("Moving to commit phase for proposal {}", msg.proposal_id);
            }
        }
        
        Ok(())
    }
    
    /// Handle commit message
    async fn handle_commit(&self, msg: CommitMessage) -> SwarmResult<()> {
        debug!("Handling commit message for proposal {} from {}", 
               msg.proposal_id, msg.agent_id);
        
        if let Some(mut voting_round) = self.voting_rounds.get_mut(&msg.proposal_id) {
            // Verify signature
            if !self.verify_signature(&msg.signature, &voting_round.proposal).await? {
                warn!("Invalid signature in commit message from {}", msg.agent_id);
                return Ok(());
            }
            
            // Add commit vote
            voting_round.commit_votes.insert(msg.agent_id, msg);
            
            // Check if we have enough commit votes
            let agents = self.agents.read().await;
            let quorum_size = self.calculate_quorum_size(agents.len());
            
            if voting_round.commit_votes.len() >= quorum_size {
                // Consensus reached!
                voting_round.phase = ConsensusPhase::Committed;
                
                let duration_ms = voting_round.started_at.elapsed().as_millis() as u64;
                let participating_agents: Vec<AgentId> = voting_round.commit_votes.keys().copied().collect();
                
                let quorum_certificate = QuorumCertificate {
                    proposal_id: voting_round.proposal_id,
                    phase: ConsensusPhase::Committed,
                    view: voting_round.view,
                    signatures: voting_round.commit_votes.values()
                        .map(|commit| commit.signature.clone())
                        .collect(),
                    created_at: Utc::now(),
                };
                
                let result = ConsensusResult {
                    proposal_id: voting_round.proposal_id,
                    proposal: voting_round.proposal.clone(),
                    decision: ConsensusDecision::Accepted,
                    view: voting_round.view,
                    participating_agents,
                    quorum_certificate,
                    completed_at: Utc::now(),
                    duration_ms,
                };
                
                voting_round.result = Some(result.clone());
                self.results.insert(voting_round.proposal_id, result);
                
                // Update statistics
                let mut stats = self.stats.write().await;
                stats.proposals_completed += 1;
                stats.avg_consensus_duration_ms = 
                    (stats.avg_consensus_duration_ms * (stats.proposals_completed - 1) as f64 + duration_ms as f64) 
                    / stats.proposals_completed as f64;
                
                info!("Consensus reached for proposal {} in {}ms", 
                      voting_round.proposal_id, duration_ms);
            }
        }
        
        Ok(())
    }
    
    /// Get the next sequence number
    async fn get_next_sequence(&self) -> SequenceNumber {
        // In a real implementation, this would be persisted and atomic
        self.stats.read().await.proposals_initiated + 1
    }
    
    /// Sign a message (simplified implementation)
    async fn sign_message(&self, _proposal: &ConsensusProposal) -> SwarmResult<ConsensusSignature> {
        // In a real implementation, this would use actual cryptographic signing
        Ok(ConsensusSignature {
            signer: self.agent_id,
            signature: vec![0u8; 64], // Placeholder signature
            algorithm: self.config.signature_algorithm.clone(),
        })
    }
    
    /// Verify a signature (simplified implementation)
    async fn verify_signature(&self, _signature: &ConsensusSignature, _proposal: &ConsensusProposal) -> SwarmResult<bool> {
        // In a real implementation, this would verify actual cryptographic signatures
        Ok(true)
    }
    
    /// Hash a proposal for digest creation
    async fn hash_proposal(&self, proposal: &ConsensusProposal) -> SwarmResult<Vec<u8>> {
        let serialized = serde_json::to_vec(proposal)
            .map_err(|e| SwarmError::Other(anyhow::anyhow!("Serialization error: {}", e)))?;
        Ok(blake3::hash(&serialized).as_bytes().to_vec())
    }
    
    /// Clean up old voting rounds and results
    async fn cleanup_old_rounds(&self) {
        let cutoff = Instant::now() - Duration::from_secs(3600); // 1 hour ago
        
        self.voting_rounds.retain(|_, round| {
            round.started_at > cutoff || round.result.is_none()
        });
        
        // Keep results longer for auditability
        let result_cutoff = Utc::now() - chrono::Duration::hours(24);
        self.results.retain(|_, result| {
            result.completed_at > result_cutoff
        });
    }
}

#[async_trait]
impl ConsensusProtocol for PBFTConsensus {
    async fn start(&mut self) -> SwarmResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(SwarmError::Other(anyhow::anyhow!("Consensus already running")));
        }
        
        *running = true;
        info!("PBFT consensus started for swarm {}", self.swarm_id);
        
        // Start cleanup task
        let voting_rounds = Arc::clone(&self.voting_rounds);
        let results = Arc::clone(&self.results);
        let running_flag = Arc::clone(&self.running);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            
            loop {
                interval.tick().await;
                
                if !*running_flag.read().await {
                    break;
                }
                
                // Cleanup logic here would go in the main struct
                // This is just a placeholder
            }
        });
        
        Ok(())
    }
    
    async fn stop(&mut self) -> SwarmResult<()> {
        let mut running = self.running.write().await;
        *running = false;
        info!("PBFT consensus stopped for swarm {}", self.swarm_id);
        Ok(())
    }
    
    async fn propose(&self, proposal: ConsensusProposal) -> SwarmResult<ProposalId> {
        if !*self.running.read().await {
            return Err(SwarmError::Other(anyhow::anyhow!("Consensus not running")));
        }
        
        if !self.has_sufficient_agents().await {
            return Err(SwarmError::Other(anyhow::anyhow!("Insufficient agents for consensus")));
        }
        
        let mut stats = self.stats.write().await;
        let current_view = stats.current_view;
        stats.proposals_initiated += 1;
        drop(stats);
        
        if !self.is_primary(current_view).await {
            return Err(SwarmError::Other(anyhow::anyhow!("Not the primary for current view")));
        }
        
        self.start_pre_prepare(proposal, current_view).await
    }
    
    async fn handle_message(&self, message: ConsensusMessage) -> SwarmResult<()> {
        if !*self.running.read().await {
            return Ok(()); // Ignore messages if not running
        }
        
        match message {
            ConsensusMessage::PrePrepare(msg) => self.handle_pre_prepare(msg).await,
            ConsensusMessage::Prepare(msg) => self.handle_prepare(msg).await,
            ConsensusMessage::Commit(msg) => self.handle_commit(msg).await,
            ConsensusMessage::ViewChange(_) => {
                // TODO: Implement view change handling
                warn!("View change not yet implemented");
                Ok(())
            }
            ConsensusMessage::NewView(_) => {
                // TODO: Implement new view handling
                warn!("New view not yet implemented");
                Ok(())
            }
            ConsensusMessage::Checkpoint(_) => {
                // TODO: Implement checkpoint handling
                warn!("Checkpoint not yet implemented");
                Ok(())
            }
        }
    }
    
    async fn get_result(&self, proposal_id: ProposalId) -> SwarmResult<Option<ConsensusResult>> {
        Ok(self.results.get(&proposal_id).map(|r| r.clone()))
    }
    
    fn can_achieve_consensus(&self) -> bool {
        // Check if we have sufficient agents (will need to be async in real implementation)
        let agents_len = self.voting_rounds.len(); // Placeholder
        agents_len >= 4 // Minimum for 3f+1 with f=1
    }
    
    fn get_stats(&self) -> ConsensusStats {
        // This would need to be async in a real implementation
        ConsensusStats::default()
    }
}

/// Integration with the swarm coordinator
impl crate::coordinator::SwarmCoordinator {
    /// Get consensus protocol instance
    pub fn consensus(&self) -> Option<Arc<dyn ConsensusProtocol>> {
        // This would be stored as a field in the coordinator
        None
    }
    
    /// Enable consensus for critical decisions
    pub async fn enable_consensus(&self, _config: ConsensusConfig) -> SwarmResult<()> {
        // TODO: Create and store consensus instance
        info!("Consensus enabled for swarm");
        Ok(())
    }
    
    /// Initiate consensus on a critical decision
    pub async fn consensus_vote(
        &self,
        proposal: ConsensusProposal,
    ) -> SwarmResult<ProposalId> {
        // TODO: Use stored consensus instance
        Err(SwarmError::Other(anyhow::anyhow!("Consensus not yet integrated")))
    }
    
    /// Check consensus result
    pub async fn get_consensus_result(
        &self,
        proposal_id: ProposalId,
    ) -> SwarmResult<Option<ConsensusResult>> {
        // TODO: Use stored consensus instance
        Ok(None)
    }
}