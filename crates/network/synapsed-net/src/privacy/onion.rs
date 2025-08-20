//! Onion routing implementation for anonymous communications.

use crate::error::{NetworkError, PrivacyError, Result};
use crate::types::{PeerId, PeerInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;
use rand::seq::SliceRandom;

/// Onion routing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnionConfig {
    /// Number of hops in the circuit
    pub circuit_length: usize,
    /// Circuit creation timeout
    pub circuit_timeout: Duration,
    /// Circuit refresh interval
    pub refresh_interval: Duration,
    /// Maximum circuits to maintain
    pub max_circuits: usize,
}

impl Default for OnionConfig {
    fn default() -> Self {
        Self {
            circuit_length: 3,
            circuit_timeout: Duration::from_secs(30),
            refresh_interval: Duration::from_secs(600), // 10 minutes
            max_circuits: 10,
        }
    }
}

/// Onion circuit for anonymous routing.
#[derive(Debug, Clone)]
pub struct OnionCircuit {
    /// Circuit identifier
    pub id: Uuid,
    /// Ordered list of nodes in the circuit
    pub nodes: Vec<OnionNode>,
    /// Circuit state
    pub state: CircuitState,
    /// When the circuit was created
    pub created_at: SystemTime,
    /// When the circuit was last used
    pub last_used: SystemTime,
    /// Encryption keys for each hop
    pub encryption_keys: Vec<Vec<u8>>,
}

/// Node in an onion circuit.
#[derive(Debug, Clone)]
pub struct OnionNode {
    /// Node peer ID
    pub peer_id: PeerId,
    /// Node information
    pub peer_info: PeerInfo,
    /// Public key for this node
    pub public_key: Vec<u8>,
    /// Position in the circuit (0 = entry, last = exit)
    pub position: usize,
}

/// Circuit state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is being built
    Building,
    /// Circuit is ready for use
    Ready,
    /// Circuit is being torn down
    TearingDown,
    /// Circuit has failed
    Failed,
}

/// Onion packet for routing through circuits.
#[derive(Debug, Clone)]
pub struct OnionPacket {
    /// Packet identifier
    pub id: Uuid,
    /// Circuit this packet belongs to
    pub circuit_id: Uuid,
    /// Current hop in the circuit
    pub current_hop: usize,
    /// Encrypted payload
    pub payload: Vec<u8>,
    /// Next hop destination
    pub next_hop: Option<PeerId>,
}

/// Onion routing manager.
pub struct OnionRouter {
    /// Configuration
    config: OnionConfig,
    /// Active circuits
    circuits: HashMap<Uuid, OnionCircuit>,
    /// Available nodes for circuit construction
    nodes: HashMap<PeerId, OnionNode>,
    /// Circuit statistics
    stats: OnionStats,
}

impl OnionRouter {
    /// Creates a new onion router.
    pub fn new(config: OnionConfig) -> Self {
        Self {
            config,
            circuits: HashMap::new(),
            nodes: HashMap::new(),
            stats: OnionStats::default(),
        }
    }
    
    /// Registers a node for circuit construction.
    pub fn register_node(&mut self, peer_id: PeerId, peer_info: PeerInfo, public_key: Vec<u8>) -> Result<()> {
        let node = OnionNode {
            peer_id,
            peer_info,
            public_key,
            position: 0, // Will be set when used in circuit
        };
        
        self.nodes.insert(peer_id, node);
        Ok(())
    }
    
    /// Unregisters a node.
    pub fn unregister_node(&mut self, peer_id: &PeerId) -> Result<()> {
        self.nodes.remove(peer_id);
        
        // Remove circuits using this node
        let circuits_to_remove: Vec<_> = self.circuits
            .iter()
            .filter(|(_, circuit)| circuit.nodes.iter().any(|n| &n.peer_id == peer_id))
            .map(|(id, _)| *id)
            .collect();
        
        for circuit_id in circuits_to_remove {
            self.tear_down_circuit(&circuit_id)?;
        }
        
        Ok(())
    }
    
    /// Creates a new circuit.
    pub async fn create_circuit(&mut self) -> Result<Uuid> {
        if self.circuits.len() >= self.config.max_circuits {
            return Err(NetworkError::Privacy(PrivacyError::AnonymizationFailed(
                "Maximum number of circuits reached".to_string()
            )));
        }
        
        // Select nodes for the circuit
        let selected_nodes = self.select_circuit_nodes()?;
        
        let circuit_id = Uuid::new_v4();
        let now = SystemTime::now();
        
        // Generate encryption keys for each hop
        let mut encryption_keys = Vec::new();
        for _ in 0..selected_nodes.len() {
            encryption_keys.push(self.generate_encryption_key()?);
        }
        
        let circuit = OnionCircuit {
            id: circuit_id,
            nodes: selected_nodes,
            state: CircuitState::Building,
            created_at: now,
            last_used: now,
            encryption_keys,
        };
        
        self.circuits.insert(circuit_id, circuit);
        self.stats.circuits_created += 1;
        
        // TODO: Actually build the circuit by contacting nodes
        // For now, we'll mark it as ready
        if let Some(circuit) = self.circuits.get_mut(&circuit_id) {
            circuit.state = CircuitState::Ready;
        }
        
        Ok(circuit_id)
    }
    
    /// Tears down a circuit.
    pub fn tear_down_circuit(&mut self, circuit_id: &Uuid) -> Result<()> {
        if let Some(mut circuit) = self.circuits.remove(circuit_id) {
            circuit.state = CircuitState::TearingDown;
            
            // TODO: Send teardown messages to circuit nodes
            
            self.stats.circuits_torn_down += 1;
        }
        
        Ok(())
    }
    
    /// Sends data through a circuit.
    pub fn send_through_circuit(&mut self, circuit_id: &Uuid, data: &[u8]) -> Result<OnionPacket> {
        // First, get necessary data without mutable borrow
        let (circuit_state, encryption_keys, first_peer_id) = {
            let circuit = self.circuits.get(circuit_id)
                .ok_or_else(|| NetworkError::Privacy(PrivacyError::AnonymizationFailed(
                    "Circuit not found".to_string()
                )))?;
            
            let first_peer_id = circuit.nodes.first().map(|n| n.peer_id.clone());
            (circuit.state.clone(), circuit.encryption_keys.clone(), first_peer_id)
        };
        
        if circuit_state != CircuitState::Ready {
            return Err(NetworkError::Privacy(PrivacyError::AnonymizationFailed(
                "Circuit not ready".to_string()
            )));
        }
        
        // Encrypt the data for each hop (in reverse order)
        let mut encrypted_payload = data.to_vec();
        for (_i, key) in encryption_keys.iter().enumerate().rev() {
            encrypted_payload = self.encrypt_layer(&encrypted_payload, key)?;
        }
        
        let packet = OnionPacket {
            id: Uuid::new_v4(),
            circuit_id: *circuit_id,
            current_hop: 0,
            payload: encrypted_payload,
            next_hop: first_peer_id,
        };
        
        // Update circuit state with separate mutable borrow
        if let Some(circuit) = self.circuits.get_mut(circuit_id) {
            circuit.last_used = SystemTime::now();
        }
        self.stats.packets_sent += 1;
        
        Ok(packet)
    }
    
    /// Processes an incoming onion packet.
    pub fn process_packet(&mut self, mut packet: OnionPacket) -> Result<ProcessedPacket> {
        // Get necessary data without holding mutable borrow
        let (nodes_len, encryption_key) = {
            let circuit = self.circuits.get(&packet.circuit_id)
                .ok_or_else(|| NetworkError::Privacy(PrivacyError::AnonymizationFailed(
                    "Circuit not found for packet".to_string()
                )))?;
            
            if packet.current_hop >= circuit.nodes.len() {
                return Err(NetworkError::Privacy(PrivacyError::AnonymizationFailed(
                    "Packet hop count exceeded circuit length".to_string()
                )));
            }
            
            (circuit.nodes.len(), circuit.encryption_keys[packet.current_hop].clone())
        };
        
        // Decrypt one layer
        packet.payload = self.decrypt_layer(&packet.payload, &encryption_key)?;
        
        packet.current_hop += 1;
        
        // Determine next action
        if packet.current_hop >= nodes_len {
            // Packet has reached the end of the circuit
            Ok(ProcessedPacket::Final(packet.payload))
        } else {
            // Get next hop info with separate borrow
            let next_hop = {
                let circuit = self.circuits.get(&packet.circuit_id)
                    .ok_or_else(|| NetworkError::Privacy(PrivacyError::AnonymizationFailed(
                        "Circuit not found for next hop".to_string()
                    )))?;
                circuit.nodes.get(packet.current_hop).map(|n| n.peer_id.clone())
            };
            packet.next_hop = next_hop;
            Ok(ProcessedPacket::Forward(packet))
        }
    }
    
    /// Selects nodes for a new circuit.
    fn select_circuit_nodes(&self) -> Result<Vec<OnionNode>> {
        if self.nodes.len() < self.config.circuit_length {
            return Err(NetworkError::Privacy(PrivacyError::AnonymizationFailed(
                format!("Not enough nodes for circuit: {} < {}", 
                    self.nodes.len(), self.config.circuit_length)
            )));
        }
        
        let mut available_nodes: Vec<_> = self.nodes.values().collect();
        let mut rng = rand::thread_rng();
        available_nodes.shuffle(&mut rng);
        
        let mut selected = Vec::new();
        for (position, node) in available_nodes.iter().take(self.config.circuit_length).enumerate() {
            let mut selected_node = (*node).clone();
            selected_node.position = position;
            selected.push(selected_node);
        }
        
        Ok(selected)
    }
    
    /// Generates an encryption key for a circuit hop.
    fn generate_encryption_key(&self) -> Result<Vec<u8>> {
        use rand::RngCore;
        let mut key = vec![0u8; 32]; // 256-bit key
        rand::thread_rng().fill_bytes(&mut key);
        Ok(key)
    }
    
    /// Encrypts a layer of the onion.
    fn encrypt_layer(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        // Simple XOR encryption for demo (use proper encryption in production)
        let mut encrypted = data.to_vec();
        for (i, byte) in encrypted.iter_mut().enumerate() {
            *byte ^= key[i % key.len()];
        }
        Ok(encrypted)
    }
    
    /// Decrypts a layer of the onion.
    fn decrypt_layer(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        // XOR is symmetric, so decryption is the same as encryption
        self.encrypt_layer(data, key)
    }
    
    /// Performs circuit maintenance.
    pub fn maintenance(&mut self) -> Result<()> {
        let now = SystemTime::now();
        let refresh_interval = self.config.refresh_interval;
        
        // Find circuits that need refreshing
        let circuits_to_refresh: Vec<_> = self.circuits
            .iter()
            .filter(|(_, circuit)| {
                now.duration_since(circuit.created_at)
                    .unwrap_or(Duration::ZERO) > refresh_interval
            })
            .map(|(id, _)| *id)
            .collect();
        
        // Refresh old circuits
        for circuit_id in circuits_to_refresh {
            self.tear_down_circuit(&circuit_id)?;
            // New circuits will be created on demand
        }
        
        Ok(())
    }
    
    /// Gets the circuit with the best characteristics for sending data.
    pub fn select_best_circuit(&self) -> Option<&OnionCircuit> {
        self.circuits
            .values()
            .filter(|c| c.state == CircuitState::Ready)
            .min_by_key(|c| c.last_used) // Use least recently used
    }
    
    /// Gets onion routing statistics.
    pub fn get_stats(&self) -> &OnionStats {
        &self.stats
    }
    
    /// Gets the number of active circuits.
    pub fn active_circuit_count(&self) -> usize {
        self.circuits.len()
    }
    
    /// Gets the number of available nodes.
    pub fn available_node_count(&self) -> usize {
        self.nodes.len()
    }
}

/// Result of processing an onion packet.
#[derive(Debug)]
pub enum ProcessedPacket {
    /// Packet should be forwarded to the next hop
    Forward(OnionPacket),
    /// Packet has reached its final destination
    Final(Vec<u8>),
}

/// Onion routing statistics.
#[derive(Debug, Default, Clone)]
pub struct OnionStats {
    /// Number of circuits created
    pub circuits_created: u64,
    /// Number of circuits torn down
    pub circuits_torn_down: u64,
    /// Number of packets sent
    pub packets_sent: u64,
    /// Number of packets received
    pub packets_received: u64,
    /// Number of failed circuit attempts
    pub failed_circuits: u64,
}

impl Default for OnionRouter {
    fn default() -> Self {
        Self::new(OnionConfig::default())
    }
}