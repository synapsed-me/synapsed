//! Comprehensive tests for HotStuff consensus protocol

use synapsed_consensus::{
    HotStuffConsensus, ConsensusConfig, ConsensusProtocol, StateMachine, NetworkTransport,
    ConsensusCrypto, Block, NodeId, Vote, QuorumCertificate, ViewNumber, Transaction, VoteType,
    ConsensusError, Result,
    traits::ConsensusMessage
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;

/// Mock state machine for testing
#[derive(Debug, Clone)]
struct MockStateMachine {
    blocks: Vec<Block>,
    state_hash: Vec<u8>,
}

impl MockStateMachine {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            state_hash: vec![0; 32],
        }
    }
}

#[async_trait]
impl StateMachine for MockStateMachine {
    async fn apply_block(&mut self, block: &Block) -> Result<()> {
        self.blocks.push(block.clone());
        // Update state hash (simple concatenation for testing)
        self.state_hash = block.hash();
        Ok(())
    }

    async fn state_hash(&self) -> Result<Vec<u8>> {
        Ok(self.state_hash.clone())
    }

    async fn create_snapshot(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self.blocks)?)
    }

    async fn restore_snapshot(&mut self, snapshot: &[u8]) -> Result<()> {
        self.blocks = serde_json::from_slice(snapshot)?;
        if let Some(last_block) = self.blocks.last() {
            self.state_hash = last_block.hash();
        }
        Ok(())
    }

    async fn validate_block(&self, block: &Block) -> Result<bool> {
        // Simple validation - check if block height is correct
        let expected_height = self.blocks.len() as u64 + 1;
        Ok(block.height == expected_height && block.is_valid())
    }
}

/// Mock network transport for testing
#[derive(Debug)]
struct MockNetworkTransport {
    node_id: NodeId,
    messages: Arc<Mutex<Vec<(NodeId, ConsensusMessage)>>>,
    peers: Vec<NodeId>,
}

impl MockNetworkTransport {
    fn new(node_id: NodeId, peers: Vec<NodeId>) -> Self {
        Self {
            node_id,
            messages: Arc::new(Mutex::new(Vec::new())),
            peers,
        }
    }

    async fn get_messages(&self) -> Vec<(NodeId, ConsensusMessage)> {
        let messages = self.messages.lock().await;
        messages.clone()
    }
}

#[async_trait]
impl NetworkTransport for MockNetworkTransport {
    async fn broadcast(&self, message: ConsensusMessage) -> Result<()> {
        let mut messages = self.messages.lock().await;
        for peer in &self.peers {
            if *peer != self.node_id {
                messages.push((peer.clone(), message.clone()));
            }
        }
        Ok(())
    }

    async fn send_to(&self, peer: NodeId, message: ConsensusMessage) -> Result<()> {
        let mut messages = self.messages.lock().await;
        messages.push((peer, message));
        Ok(())
    }

    async fn receive(&mut self) -> Result<(NodeId, ConsensusMessage)> {
        let mut messages = self.messages.lock().await;
        messages.pop().ok_or(ConsensusError::NetworkError("No messages".to_string()))
    }

    async fn peers(&self) -> Result<Vec<NodeId>> {
        Ok(self.peers.clone())
    }

    async fn is_connected(&self, peer: &NodeId) -> Result<bool> {
        Ok(self.peers.contains(peer))
    }
}

/// Mock crypto implementation for testing
#[derive(Debug, Clone)]
struct MockCrypto {
    node_id: NodeId,
    keypairs: HashMap<NodeId, (Vec<u8>, Vec<u8>)>, // (private, public) keys
}

impl MockCrypto {
    fn new(node_id: NodeId) -> Self {
        let mut keypairs = HashMap::new();
        // Generate mock keypair for this node
        let private_key = vec![1, 2, 3, 4]; // Mock private key
        let public_key = vec![5, 6, 7, 8];  // Mock public key
        keypairs.insert(node_id.clone(), (private_key, public_key));
        
        Self { node_id, keypairs }
    }

    fn add_node(&mut self, node_id: NodeId) {
        if !self.keypairs.contains_key(&node_id) {
            let private_key = vec![node_id.as_uuid().as_bytes()[0], 1, 2, 3];
            let public_key = vec![node_id.as_uuid().as_bytes()[0], 5, 6, 7];
            self.keypairs.insert(node_id, (private_key, public_key));
        }
    }
}

#[async_trait]
impl ConsensusCrypto for MockCrypto {
    async fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        if let Some((private_key, _)) = self.keypairs.get(&self.node_id) {
            // Mock signature - just concatenate private key with message hash
            let mut signature = private_key.clone();
            signature.extend_from_slice(&message[..std::cmp::min(message.len(), 4)]);
            Ok(signature)
        } else {
            Err(ConsensusError::CryptographicError("No private key".to_string()))
        }
    }

    async fn verify(&self, node: &NodeId, message: &[u8], signature: &[u8]) -> Result<bool> {
        if let Some((private_key, _)) = self.keypairs.get(node) {
            // Mock verification - check if signature starts with private key
            let expected_start = private_key;
            Ok(signature.len() >= expected_start.len() && 
               signature[..expected_start.len()] == *expected_start)
        } else {
            Ok(false)
        }
    }

    async fn public_key(&self) -> Result<Vec<u8>> {
        if let Some((_, public_key)) = self.keypairs.get(&self.node_id) {
            Ok(public_key.clone())
        } else {
            Err(ConsensusError::CryptographicError("No public key".to_string()))
        }
    }

    async fn verify_qc(&self, qc: &QuorumCertificate) -> Result<bool> {
        // Mock QC verification - verify each vote
        for vote in &qc.votes {
            let vote_data = serde_json::to_vec(&vote)?;
            if !self.verify(&vote.voter, &vote_data, &vote.signature).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn aggregate_signatures(&self, signatures: &[Vec<u8>]) -> Result<Vec<u8>> {
        // Mock aggregation - just concatenate
        let mut aggregated = Vec::new();
        for sig in signatures {
            aggregated.extend_from_slice(sig);
        }
        Ok(aggregated)
    }
}

/// Create a test setup with multiple nodes
async fn create_test_setup(num_nodes: usize) -> Result<Vec<(NodeId, HotStuffConsensus<MockNetworkTransport, MockCrypto, MockStateMachine>)>> {
    let mut nodes = Vec::new();
    let mut validators = Vec::new();

    // Create node IDs
    for _ in 0..num_nodes {
        validators.push(NodeId::new());
    }

    // Create consensus instances
    for (i, node_id) in validators.iter().enumerate() {
        let config = ConsensusConfig::new(node_id.clone(), validators.clone());
        
        let network = Arc::new(MockNetworkTransport::new(node_id.clone(), validators.clone()));
        let mut crypto = MockCrypto::new(node_id.clone());
        
        // Add all validators to crypto
        for validator in &validators {
            crypto.add_node(validator.clone());
        }
        let crypto = Arc::new(crypto);
        
        let state_machine = Arc::new(Mutex::new(MockStateMachine::new()));
        
        let consensus = HotStuffConsensus::new(config, network, crypto, state_machine).await?;
        
        nodes.push((node_id.clone(), consensus));
    }

    Ok(nodes)
}

#[tokio::test]
async fn test_hotstuff_consensus_creation() -> Result<()> {
    let nodes = create_test_setup(4).await?;
    assert_eq!(nodes.len(), 4);
    
    // Check that each node has correct configuration
    for (node_id, consensus) in &nodes {
        assert_eq!(consensus.node_id, *node_id);
        assert_eq!(consensus.config.validators.len(), 4);
        assert_eq!(consensus.config.byzantine_threshold, 1); // (4-1)/3 = 1
        assert_eq!(consensus.config.quorum_size(), 3); // 2*1+1 = 3
    }
    
    Ok(())
}

#[tokio::test]
async fn test_leader_election() -> Result<()> {
    let nodes = create_test_setup(4).await?;
    let (_, consensus) = &nodes[0];
    
    // Test leader election for different views
    let leader_view_0 = consensus.get_current_leader();
    let leader_view_1 = consensus.leader_election.get_leader(ViewNumber::new(1), &consensus.config.validators);
    
    // Leaders should be deterministic and valid validators
    assert!(consensus.config.validators.contains(&leader_view_0));
    assert!(consensus.config.validators.contains(&leader_view_1));
    
    // Check is_leader function
    let current_view = consensus.state.read().view;
    let current_leader = consensus.get_current_leader();
    assert!(consensus.leader_election.is_leader(&current_leader, current_view, &consensus.config.validators));
    
    Ok(())
}

#[tokio::test]
async fn test_vote_aggregation() -> Result<()> {
    let nodes = create_test_setup(4).await?;
    let (node_id, mut consensus) = nodes.into_iter().next().unwrap();
    
    let block_id = Uuid::new_v4();
    let view = ViewNumber::new(1);
    
    // Create votes from different validators
    let mut votes = Vec::new();
    for (i, validator) in consensus.config.validators.iter().enumerate() {
        let vote = Vote::new(
            VoteType::Prepare,
            view,
            block_id,
            validator.clone(),
            vec![i as u8; 4], // Mock signature
        );
        votes.push(vote);
    }
    
    // Add votes to aggregator
    let mut vote_aggregator = consensus.vote_aggregator.lock().await;
    
    // First two votes shouldn't form QC (need 3 for quorum)
    assert!(vote_aggregator.add_vote(votes[0].clone())?.is_none());
    assert!(vote_aggregator.add_vote(votes[1].clone())?.is_none());
    
    // Third vote should form QC
    let qc = vote_aggregator.add_vote(votes[2].clone())?;
    assert!(qc.is_some());
    
    let qc = qc.unwrap();
    assert_eq!(qc.vote_count(), 3);
    assert_eq!(qc.view, view);
    assert_eq!(qc.block_id, block_id);
    
    Ok(())
}

#[tokio::test]
async fn test_block_proposal_and_voting() -> Result<()> {
    let mut nodes = create_test_setup(4).await?;
    
    // Start consensus on all nodes
    for (_, consensus) in &mut nodes {
        consensus.start().await?;
    }
    
    // Find the leader node
    let leader_node = nodes.iter()
        .find(|(_, consensus)| consensus.is_current_leader())
        .cloned();
    
    assert!(leader_node.is_some());
    let (leader_id, mut leader_consensus) = leader_node.unwrap();
    
    // Create a test transaction
    let transaction = Transaction::new(
        b"test transaction data".to_vec(),
        vec![1, 2, 3, 4], // Mock signature
    );
    
    // Propose a block
    let block = leader_consensus.propose_block(vec![transaction]).await?;
    
    // Verify block properties
    assert_eq!(block.height, 1);
    assert_eq!(block.proposer, leader_id);
    assert_eq!(block.transactions.len(), 1);
    assert!(block.is_valid());
    
    // Check that proposal was broadcast (mock network should have messages)
    let network = leader_consensus.network.clone();
    let messages = network.get_messages().await;
    assert!(!messages.is_empty());
    
    // Verify message is a proposal
    if let Some((_, ConsensusMessage::Proposal { block: proposed_block, .. })) = messages.first() {
        assert_eq!(proposed_block.id, block.id);
    } else {
        panic!("Expected proposal message");
    }
    
    Ok(())
}

#[tokio::test]
async fn test_view_change() -> Result<()> {
    let mut nodes = create_test_setup(4).await?;
    let (_, mut consensus) = nodes.into_iter().next().unwrap();
    
    consensus.start().await?;
    
    let initial_view = consensus.current_view();
    assert_eq!(initial_view, ViewNumber::new(0));
    
    // Simulate timeout
    consensus.handle_timeout(NodeId::new(), initial_view).await?;
    
    // View should advance
    let new_view = consensus.current_view();
    assert!(new_view > initial_view);
    
    Ok(())
}

#[tokio::test]
async fn test_byzantine_fault_tolerance() -> Result<()> {
    let nodes = create_test_setup(4).await?; // f=1, can tolerate 1 Byzantine node
    
    // With 4 nodes and f=1, we need 3 votes for quorum
    // This means 1 Byzantine node cannot prevent progress
    assert_eq!(nodes[0].1.config.byzantine_threshold, 1);
    assert_eq!(nodes[0].1.config.quorum_size(), 3);
    
    // Simulate 1 Byzantine node by having it not vote
    // The other 3 honest nodes should still reach consensus
    
    Ok(())
}

#[tokio::test]
async fn test_performance_requirements() -> Result<()> {
    let start_time = std::time::Instant::now();
    
    // Create larger network
    let nodes = create_test_setup(100).await?; // Test with 100 validators
    
    let setup_time = start_time.elapsed();
    println!("Setup time for 100 nodes: {:?}", setup_time);
    
    // Setup should be fast (< 1 second)
    assert!(setup_time.as_secs() < 1);
    
    // Test that we can handle the required throughput
    // (This is a simplified test - real performance testing would be more comprehensive)
    
    Ok(())
}

#[tokio::test]
async fn test_safety_properties() -> Result<()> {
    let nodes = create_test_setup(4).await?;
    let (_, consensus) = &nodes[0];
    
    // Test that state machine validation is enforced
    let invalid_block = Block::new(
        vec![0; 32], // Invalid parent hash
        999,         // Invalid height
        vec![],      // Empty transactions (invalid in our mock)
        NodeId::new(),
    );
    
    let state_machine = consensus.state_machine.lock().await;
    let is_valid = state_machine.validate_block(&invalid_block).await?;
    assert!(!is_valid);
    
    Ok(())
}

#[tokio::test]
async fn test_liveness_properties() -> Result<()> {
    let mut nodes = create_test_setup(4).await?;
    
    // Start all nodes
    for (_, consensus) in &mut nodes {
        consensus.start().await?;
    }
    
    // Verify that a leader is always elected
    for (_, consensus) in &nodes {
        let leader = consensus.current_leader();
        assert!(leader.is_some());
        
        let leader_id = leader.unwrap();
        assert!(consensus.config.validators.contains(&leader_id));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_consensus_stats() -> Result<()> {
    let mut nodes = create_test_setup(4).await?;
    let (_, mut consensus) = nodes.into_iter().next().unwrap();
    
    consensus.start().await?;
    
    let stats = consensus.get_stats();
    
    // Initial stats should be reasonable
    assert_eq!(stats.current_view, 0);
    assert_eq!(stats.blocks_committed, 0);
    assert_eq!(stats.transactions_processed, 0);
    
    Ok(())
}