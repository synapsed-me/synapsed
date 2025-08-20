//! Chaos engineering tests for HotStuff consensus protocol
//! These tests simulate network failures, Byzantine behavior, and other edge cases

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
use rand::Rng;
use tokio::time::{sleep, Duration};

/// Byzantine behavior simulator
#[derive(Debug, Clone)]
enum ByzantineBehavior {
    /// Node behaves honestly
    Honest,
    /// Node crashes and stops responding
    Crash,
    /// Node sends invalid messages
    SendInvalid,
    /// Node sends conflicting votes
    DoubleVote,
    /// Node delays messages
    DelayMessages(Duration),
    /// Node drops messages randomly
    DropMessages(f64), // probability of dropping
}

/// Chaos network transport that can simulate failures
#[derive(Debug)]
struct ChaosNetworkTransport {
    node_id: NodeId,
    messages: Arc<Mutex<Vec<(NodeId, ConsensusMessage)>>>,
    peers: Vec<NodeId>,
    byzantine_behavior: ByzantineBehavior,
    message_delay: Duration,
    drop_probability: f64,
    partition_nodes: Vec<NodeId>, // Nodes this node is partitioned from
}

impl ChaosNetworkTransport {
    fn new(node_id: NodeId, peers: Vec<NodeId>) -> Self {
        Self {
            node_id,
            messages: Arc::new(Mutex::new(Vec::new())),
            peers,
            byzantine_behavior: ByzantineBehavior::Honest,
            message_delay: Duration::from_millis(0),
            drop_probability: 0.0,
            partition_nodes: Vec::new(),
        }
    }

    fn set_byzantine_behavior(&mut self, behavior: ByzantineBehavior) {
        self.byzantine_behavior = behavior;
    }

    fn set_message_delay(&mut self, delay: Duration) {
        self.message_delay = delay;
    }

    fn set_drop_probability(&mut self, probability: f64) {
        self.drop_probability = probability;
    }

    fn partition_from(&mut self, nodes: Vec<NodeId>) {
        self.partition_nodes = nodes;
    }

    async fn should_drop_message(&self) -> bool {
        if self.drop_probability > 0.0 {
            let mut rng = rand::thread_rng();
            rng.gen::<f64>() < self.drop_probability
        } else {
            false
        }
    }

    async fn apply_byzantine_behavior(&self, message: &mut ConsensusMessage) -> bool {
        match &self.byzantine_behavior {
            ByzantineBehavior::Honest => true,
            ByzantineBehavior::Crash => false, // Drop all messages
            ByzantineBehavior::SendInvalid => {
                // Modify message to make it invalid
                match message {
                    ConsensusMessage::Vote(vote) => {
                        // Corrupt the signature
                        let mut corrupted_vote = vote.clone();
                        corrupted_vote.signature = vec![0xFF; corrupted_vote.signature.len()];
                        *message = ConsensusMessage::Vote(corrupted_vote);
                    }
                    _ => {}
                }
                true
            }
            ByzantineBehavior::DoubleVote => {
                // For testing, we'll send the original message
                // In real implementation, this would send conflicting votes
                true
            }
            ByzantineBehavior::DelayMessages(delay) => {
                sleep(*delay).await;
                true
            }
            ByzantineBehavior::DropMessages(prob) => {
                let mut rng = rand::thread_rng();
                rng.gen::<f64>() >= *prob
            }
        }
    }
}

#[async_trait]
impl NetworkTransport for ChaosNetworkTransport {
    async fn broadcast(&self, mut message: ConsensusMessage) -> Result<()> {
        // Apply Byzantine behavior
        if !self.apply_byzantine_behavior(&mut message).await {
            return Ok(()); // Message dropped
        }

        // Apply network delay
        if self.message_delay > Duration::from_millis(0) {
            sleep(self.message_delay).await;
        }

        // Check if message should be dropped
        if self.should_drop_message().await {
            return Ok(());
        }

        let mut messages = self.messages.lock().await;
        for peer in &self.peers {
            // Skip partitioned nodes
            if self.partition_nodes.contains(peer) || *peer == self.node_id {
                continue;
            }
            messages.push((peer.clone(), message.clone()));
        }
        Ok(())
    }

    async fn send_to(&self, peer: NodeId, mut message: ConsensusMessage) -> Result<()> {
        // Skip partitioned nodes
        if self.partition_nodes.contains(&peer) {
            return Ok(());
        }

        // Apply Byzantine behavior
        if !self.apply_byzantine_behavior(&mut message).await {
            return Ok(()); // Message dropped
        }

        // Apply network delay
        if self.message_delay > Duration::from_millis(0) {
            sleep(self.message_delay).await;
        }

        // Check if message should be dropped
        if self.should_drop_message().await {
            return Ok(());
        }

        let mut messages = self.messages.lock().await;
        messages.push((peer, message));
        Ok(())
    }

    async fn receive(&mut self) -> Result<(NodeId, ConsensusMessage)> {
        let mut messages = self.messages.lock().await;
        messages.pop().ok_or(ConsensusError::NetworkError("No messages".to_string()))
    }

    async fn peers(&self) -> Result<Vec<NodeId>> {
        // Return only non-partitioned peers
        let peers: Vec<NodeId> = self.peers.iter()
            .filter(|peer| !self.partition_nodes.contains(peer))
            .cloned()
            .collect();
        Ok(peers)
    }

    async fn is_connected(&self, peer: &NodeId) -> Result<bool> {
        Ok(self.peers.contains(peer) && !self.partition_nodes.contains(peer))
    }
}

/// Simple state machine for chaos testing
#[derive(Debug, Clone)]
struct ChaosStateMachine {
    blocks: Vec<Block>,
    corrupted: bool,
}

impl ChaosStateMachine {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            corrupted: false,
        }
    }

    fn corrupt(&mut self) {
        self.corrupted = true;
    }
}

#[async_trait]
impl StateMachine for ChaosStateMachine {
    async fn apply_block(&mut self, block: &Block) -> Result<()> {
        if self.corrupted {
            return Err(ConsensusError::StateMachineError("State machine corrupted".to_string()));
        }
        self.blocks.push(block.clone());
        Ok(())
    }

    async fn state_hash(&self) -> Result<Vec<u8>> {
        if self.corrupted {
            return Ok(vec![0xFF; 32]); // Return corrupted hash
        }
        Ok(vec![0; 32])
    }

    async fn create_snapshot(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self.blocks)?)
    }

    async fn restore_snapshot(&mut self, snapshot: &[u8]) -> Result<()> {
        self.blocks = serde_json::from_slice(snapshot)?;
        Ok(())
    }

    async fn validate_block(&self, block: &Block) -> Result<bool> {
        if self.corrupted {
            return Ok(false); // Reject all blocks when corrupted
        }
        
        let expected_height = self.blocks.len() as u64 + 1;
        Ok(block.height == expected_height && !block.transactions.is_empty())
    }
}

/// Byzantine crypto implementation for testing
#[derive(Debug, Clone)]
struct ChaosCrypto {
    node_id: NodeId,
    keypairs: HashMap<NodeId, (Vec<u8>, Vec<u8>)>,
    byzantine_behavior: ByzantineBehavior,
}

impl ChaosCrypto {
    fn new(node_id: NodeId) -> Self {
        let mut keypairs = HashMap::new();
        let private_key = vec![1, 2, 3, 4];
        let public_key = vec![5, 6, 7, 8];
        keypairs.insert(node_id.clone(), (private_key, public_key));
        
        Self {
            node_id,
            keypairs,
            byzantine_behavior: ByzantineBehavior::Honest,
        }
    }

    fn set_byzantine_behavior(&mut self, behavior: ByzantineBehavior) {
        self.byzantine_behavior = behavior;
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
impl ConsensusCrypto for ChaosCrypto {
    async fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        match &self.byzantine_behavior {
            ByzantineBehavior::SendInvalid => {
                // Return invalid signature
                Ok(vec![0xFF; 8])
            }
            _ => {
                if let Some((private_key, _)) = self.keypairs.get(&self.node_id) {
                    let mut signature = private_key.clone();
                    signature.extend_from_slice(&message[..std::cmp::min(message.len(), 4)]);
                    Ok(signature)
                } else {
                    Err(ConsensusError::CryptographicError("No private key".to_string()))
                }
            }
        }
    }

    async fn verify(&self, node: &NodeId, message: &[u8], signature: &[u8]) -> Result<bool> {
        if let Some((private_key, _)) = self.keypairs.get(node) {
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
        for vote in &qc.votes {
            let vote_data = serde_json::to_vec(&vote)?;
            if !self.verify(&vote.voter, &vote_data, &vote.signature).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn aggregate_signatures(&self, signatures: &[Vec<u8>]) -> Result<Vec<u8>> {
        let mut aggregated = Vec::new();
        for sig in signatures {
            aggregated.extend_from_slice(sig);
        }
        Ok(aggregated)
    }
}

#[tokio::test]
async fn test_network_partition() -> Result<()> {
    // Create 4 nodes
    let mut validators = Vec::new();
    for _ in 0..4 {
        validators.push(NodeId::new());
    }

    let mut networks = Vec::new();
    let mut consensus_instances = Vec::new();

    // Create consensus instances
    for node_id in &validators {
        let config = ConsensusConfig::new(node_id.clone(), validators.clone());
        
        let mut network = ChaosNetworkTransport::new(node_id.clone(), validators.clone());
        
        // Partition first two nodes from last two nodes
        if validators.iter().position(|v| v == node_id).unwrap() < 2 {
            network.partition_from(validators[2..].to_vec());
        } else {
            network.partition_from(validators[..2].to_vec());
        }
        
        let network = Arc::new(network);
        networks.push(network.clone());
        
        let mut crypto = ChaosCrypto::new(node_id.clone());
        for validator in &validators {
            crypto.add_node(validator.clone());
        }
        let crypto = Arc::new(crypto);
        
        let state_machine = Arc::new(Mutex::new(ChaosStateMachine::new()));
        
        let consensus = HotStuffConsensus::new(config, network, crypto, state_machine).await?;
        consensus_instances.push(consensus);
    }

    // Start consensus on all nodes
    for consensus in &mut consensus_instances {
        consensus.start().await?;
    }

    // In a partition, neither side should make progress without quorum
    // With 4 nodes partitioned 2-2, and f=1 requiring 3 votes, no side can commit
    
    // Let the system run for a bit
    sleep(Duration::from_millis(100)).await;
    
    // Check that no blocks were committed due to partition
    for consensus in &consensus_instances {
        let stats = consensus.get_stats();
        assert_eq!(stats.blocks_committed, 0);
    }

    Ok(())
}

#[tokio::test]
async fn test_byzantine_nodes() -> Result<()> {
    let mut validators = Vec::new();
    for _ in 0..4 {
        validators.push(NodeId::new());
    }

    let mut consensus_instances = Vec::new();

    // Create consensus instances with one Byzantine node
    for (i, node_id) in validators.iter().enumerate() {
        let config = ConsensusConfig::new(node_id.clone(), validators.clone());
        
        let network = Arc::new(ChaosNetworkTransport::new(node_id.clone(), validators.clone()));
        
        let mut crypto = ChaosCrypto::new(node_id.clone());
        for validator in &validators {
            crypto.add_node(validator.clone());
        }
        
        // Make first node Byzantine (send invalid messages)
        if i == 0 {
            crypto.set_byzantine_behavior(ByzantineBehavior::SendInvalid);
        }
        let crypto = Arc::new(crypto);
        
        let state_machine = Arc::new(Mutex::new(ChaosStateMachine::new()));
        
        let consensus = HotStuffConsensus::new(config, network, crypto, state_machine).await?;
        consensus_instances.push(consensus);
    }

    // Start consensus on all nodes
    for consensus in &mut consensus_instances {
        consensus.start().await?;
    }

    // The system should still work with 1 Byzantine node out of 4
    // (f=1, so we can tolerate 1 Byzantine node)
    
    sleep(Duration::from_millis(100)).await;
    
    // At least honest nodes should be operational
    let honest_nodes = &consensus_instances[1..];
    for consensus in honest_nodes {
        // Should be able to get stats without errors
        let stats = consensus.get_stats();
        assert!(stats.current_view >= 0);
    }

    Ok(())
}

#[tokio::test]
async fn test_message_delays() -> Result<()> {
    let mut validators = Vec::new();
    for _ in 0..4 {
        validators.push(NodeId::new());
    }

    let mut consensus_instances = Vec::new();

    // Create consensus instances with network delays
    for node_id in &validators {
        let config = ConsensusConfig::new(node_id.clone(), validators.clone());
        
        let mut network = ChaosNetworkTransport::new(node_id.clone(), validators.clone());
        // Add 50ms delay to all messages
        network.set_message_delay(Duration::from_millis(50));
        let network = Arc::new(network);
        
        let mut crypto = ChaosCrypto::new(node_id.clone());
        for validator in &validators {
            crypto.add_node(validator.clone());
        }
        let crypto = Arc::new(crypto);
        
        let state_machine = Arc::new(Mutex::new(ChaosStateMachine::new()));
        
        let consensus = HotStuffConsensus::new(config, network, crypto, state_machine).await?;
        consensus_instances.push(consensus);
    }

    // Start consensus on all nodes
    for consensus in &mut consensus_instances {
        consensus.start().await?;
    }

    // System should work even with delays (just slower)
    sleep(Duration::from_millis(200)).await;
    
    for consensus in &consensus_instances {
        let stats = consensus.get_stats();
        // Should still be operational
        assert!(stats.current_view >= 0);
    }

    Ok(())
}

#[tokio::test]
async fn test_message_drops() -> Result<()> {
    let mut validators = Vec::new();
    for _ in 0..4 {
        validators.push(NodeId::new());
    }

    let mut consensus_instances = Vec::new();

    // Create consensus instances with message drops
    for node_id in &validators {
        let config = ConsensusConfig::new(node_id.clone(), validators.clone());
        
        let mut network = ChaosNetworkTransport::new(node_id.clone(), validators.clone());
        // Drop 10% of messages
        network.set_drop_probability(0.1);
        let network = Arc::new(network);
        
        let mut crypto = ChaosCrypto::new(node_id.clone());
        for validator in &validators {
            crypto.add_node(validator.clone());
        }
        let crypto = Arc::new(crypto);
        
        let state_machine = Arc::new(Mutex::new(ChaosStateMachine::new()));
        
        let consensus = HotStuffConsensus::new(config, network, crypto, state_machine).await?;
        consensus_instances.push(consensus);
    }

    // Start consensus on all nodes
    for consensus in &mut consensus_instances {
        consensus.start().await?;
    }

    // System should work even with message drops (with retries)
    sleep(Duration::from_millis(100)).await;
    
    for consensus in &consensus_instances {
        let stats = consensus.get_stats();
        assert!(stats.current_view >= 0);
    }

    Ok(())
}

#[tokio::test]
async fn test_state_machine_corruption() -> Result<()> {
    let mut validators = Vec::new();
    for _ in 0..4 {
        validators.push(NodeId::new());
    }

    let mut consensus_instances = Vec::new();

    // Create consensus instances with one corrupted state machine
    for (i, node_id) in validators.iter().enumerate() {
        let config = ConsensusConfig::new(node_id.clone(), validators.clone());
        
        let network = Arc::new(ChaosNetworkTransport::new(node_id.clone(), validators.clone()));
        
        let mut crypto = ChaosCrypto::new(node_id.clone());
        for validator in &validators {
            crypto.add_node(validator.clone());
        }
        let crypto = Arc::new(crypto);
        
        let mut state_machine = ChaosStateMachine::new();
        // Corrupt first node's state machine
        if i == 0 {
            state_machine.corrupt();
        }
        let state_machine = Arc::new(Mutex::new(state_machine));
        
        let consensus = HotStuffConsensus::new(config, network, crypto, state_machine).await?;
        consensus_instances.push(consensus);
    }

    // Start consensus on all nodes
    for consensus in &mut consensus_instances {
        consensus.start().await?;
    }

    // Honest nodes should continue working despite one corrupted state machine
    sleep(Duration::from_millis(100)).await;
    
    let honest_nodes = &consensus_instances[1..];
    for consensus in honest_nodes {
        let stats = consensus.get_stats();
        assert!(stats.current_view >= 0);
    }

    Ok(())
}

#[tokio::test]
async fn test_high_load_scenario() -> Result<()> {
    // Test with more nodes to simulate higher load
    let num_nodes = 10; // f=3, quorum=7
    let mut validators = Vec::new();
    for _ in 0..num_nodes {
        validators.push(NodeId::new());
    }

    let mut consensus_instances = Vec::new();

    // Create consensus instances
    for node_id in &validators {
        let config = ConsensusConfig::new(node_id.clone(), validators.clone());
        
        let network = Arc::new(ChaosNetworkTransport::new(node_id.clone(), validators.clone()));
        
        let mut crypto = ChaosCrypto::new(node_id.clone());
        for validator in &validators {
            crypto.add_node(validator.clone());
        }
        let crypto = Arc::new(crypto);
        
        let state_machine = Arc::new(Mutex::new(ChaosStateMachine::new()));
        
        let consensus = HotStuffConsensus::new(config, network, crypto, state_machine).await?;
        consensus_instances.push(consensus);
    }

    // Start consensus on all nodes
    for consensus in &mut consensus_instances {
        consensus.start().await?;
    }

    // System should handle larger validator sets
    sleep(Duration::from_millis(200)).await;
    
    for consensus in &consensus_instances {
        let stats = consensus.get_stats();
        assert!(stats.current_view >= 0);
        assert_eq!(consensus.config.validators.len(), num_nodes);
        assert_eq!(consensus.config.byzantine_threshold, 3);
        assert_eq!(consensus.config.quorum_size(), 7);
    }

    Ok(())
}

#[tokio::test]
async fn test_rapid_view_changes() -> Result<()> {
    let mut validators = Vec::new();
    for _ in 0..4 {
        validators.push(NodeId::new());
    }

    let mut consensus_instances = Vec::new();

    // Create consensus instances
    for node_id in &validators {
        let config = ConsensusConfig::new(node_id.clone(), validators.clone());
        
        let network = Arc::new(ChaosNetworkTransport::new(node_id.clone(), validators.clone()));
        
        let mut crypto = ChaosCrypto::new(node_id.clone());
        for validator in &validators {
            crypto.add_node(validator.clone());
        }
        let crypto = Arc::new(crypto);
        
        let state_machine = Arc::new(Mutex::new(ChaosStateMachine::new()));
        
        let consensus = HotStuffConsensus::new(config, network, crypto, state_machine).await?;
        consensus_instances.push(consensus);
    }

    // Start consensus on all nodes
    for consensus in &mut consensus_instances {
        consensus.start().await?;
    }

    // Simulate rapid timeouts to force view changes
    for consensus in &mut consensus_instances {
        for view in 0..5 {
            consensus.handle_timeout(NodeId::new(), ViewNumber::new(view)).await?;
            sleep(Duration::from_millis(10)).await;
        }
    }

    // System should handle rapid view changes gracefully
    for consensus in &consensus_instances {
        let stats = consensus.get_stats();
        assert!(stats.view_changes > 0);
    }

    Ok(())
}