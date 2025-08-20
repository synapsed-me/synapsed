//! Mix network implementation for anonymous communications.

use crate::error::{NetworkError, PrivacyError, Result};
use crate::types::{PeerId, PeerInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Mix network configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixNetworkConfig {
    /// Number of mix nodes in the network
    pub num_nodes: usize,
    /// Mixing delay in milliseconds
    pub mix_delay_ms: u64,
    /// Batch size for mixing
    pub batch_size: usize,
    /// Cover traffic rate (packets per second)
    pub cover_traffic_rate: f64,
    /// Maximum message age before dropping
    pub max_message_age_ms: u64,
}

impl Default for MixNetworkConfig {
    fn default() -> Self {
        Self {
            num_nodes: 3,
            mix_delay_ms: 100,
            batch_size: 10,
            cover_traffic_rate: 1.0,
            max_message_age_ms: 30000, // 30 seconds
        }
    }
}

/// Mix node in the network.
#[derive(Debug, Clone)]
pub struct MixNode {
    /// Node identifier
    pub id: PeerId,
    /// Node information
    pub info: PeerInfo,
    /// Public key for encryption
    pub public_key: Vec<u8>,
    /// Node capacity (messages per second)
    pub capacity: f64,
    /// Current load (0.0 to 1.0)
    pub load: f64,
    /// Last seen timestamp
    pub last_seen: SystemTime,
}

impl MixNode {
    /// Creates a new mix node.
    pub fn new(id: PeerId, info: PeerInfo, public_key: Vec<u8>) -> Self {
        Self {
            id,
            info,
            public_key,
            capacity: 100.0, // Default capacity
            load: 0.0,
            last_seen: SystemTime::now(),
        }
    }
    
    /// Checks if the node is available for mixing.
    pub fn is_available(&self) -> bool {
        self.load < 0.8 && // Not overloaded
        self.last_seen.elapsed().unwrap_or(Duration::MAX) < Duration::from_secs(60) // Seen recently
    }
    
    /// Updates the node's load.
    pub fn update_load(&mut self, new_load: f64) {
        self.load = new_load.clamp(0.0, 1.0);
        self.last_seen = SystemTime::now();
    }
}

/// Mix packet that travels through the network.
#[derive(Debug, Clone)]
pub struct MixPacket {
    /// Packet identifier
    pub id: Uuid,
    /// Encrypted payload
    pub payload: Vec<u8>,
    /// Route through mix nodes
    pub route: Vec<PeerId>,
    /// Current hop index
    pub current_hop: usize,
    /// Timestamp when packet was created
    pub created_at: SystemTime,
    /// Delay until next hop
    pub delay_until: SystemTime,
}

impl MixPacket {
    /// Creates a new mix packet.
    pub fn new(payload: Vec<u8>, route: Vec<PeerId>) -> Self {
        let now = SystemTime::now();
        Self {
            id: Uuid::new_v4(),
            payload,
            route,
            current_hop: 0,
            created_at: now,
            delay_until: now,
        }
    }
    
    /// Checks if the packet has expired.
    pub fn is_expired(&self, max_age: Duration) -> bool {
        self.created_at.elapsed().unwrap_or(Duration::MAX) > max_age
    }
    
    /// Checks if the packet is ready for the next hop.
    pub fn is_ready(&self) -> bool {
        SystemTime::now() >= self.delay_until
    }
    
    /// Advances to the next hop.
    pub fn advance_hop(&mut self, delay: Duration) -> Result<()> {
        if self.current_hop >= self.route.len() {
            return Err(NetworkError::Privacy(PrivacyError::MixNetwork(
                "Packet has reached end of route".to_string()
            )));
        }
        
        self.current_hop += 1;
        self.delay_until = SystemTime::now() + delay;
        
        Ok(())
    }
    
    /// Gets the next hop destination.
    pub fn next_destination(&self) -> Option<PeerId> {
        self.route.get(self.current_hop).copied()
    }
}

/// Mix network manager.
pub struct MixNetwork {
    /// Configuration
    config: MixNetworkConfig,
    /// Known mix nodes
    nodes: RwLock<HashMap<PeerId, MixNode>>,
    /// Pending packets
    pending_packets: RwLock<Vec<MixPacket>>,
    /// Cover traffic packets
    cover_packets: RwLock<Vec<MixPacket>>,
}

impl MixNetwork {
    /// Creates a new mix network.
    pub fn new(config: MixNetworkConfig) -> Self {
        Self {
            config,
            nodes: RwLock::new(HashMap::new()),
            pending_packets: RwLock::new(Vec::new()),
            cover_packets: RwLock::new(Vec::new()),
        }
    }
    
    /// Registers a mix node.
    pub async fn register_node(&self, node: MixNode) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        nodes.insert(node.id, node);
        Ok(())
    }
    
    /// Unregisters a mix node.
    pub async fn unregister_node(&self, node_id: PeerId) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        nodes.remove(&node_id);
        Ok(())
    }
    
    /// Gets available mix nodes.
    pub async fn get_available_nodes(&self) -> Vec<MixNode> {
        let nodes = self.nodes.read().await;
        nodes.values()
            .filter(|node| node.is_available())
            .cloned()
            .collect()
    }
    
    /// Creates a random route through the mix network.
    pub async fn create_route(&self, length: usize) -> Result<Vec<PeerId>> {
        let available_nodes = self.get_available_nodes().await;
        
        if available_nodes.len() < length {
            return Err(NetworkError::Privacy(PrivacyError::MixNetwork(
                format!("Not enough available nodes: {} < {}", available_nodes.len(), length)
            )));
        }
        
        // Simple random selection (in production, use more sophisticated selection)
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        let selected: Vec<_> = available_nodes
            .choose_multiple(&mut rng, length)
            .map(|node| node.id)
            .collect();
        
        Ok(selected)
    }
    
    /// Sends a message through the mix network.
    pub async fn send_message(&self, payload: Vec<u8>, destination: PeerId) -> Result<()> {
        // Create route
        let mut route = self.create_route(self.config.num_nodes).await?;
        route.push(destination); // Add final destination
        
        // Create mix packet
        let packet = MixPacket::new(payload, route);
        
        // Add to pending packets
        let mut pending = self.pending_packets.write().await;
        pending.push(packet);
        
        Ok(())
    }
    
    /// Processes pending packets (mixing).
    pub async fn process_packets(&self) -> Result<Vec<MixPacket>> {
        let mut pending = self.pending_packets.write().await;
        let mut ready_packets = Vec::new();
        
        // Remove expired packets
        let max_age = Duration::from_millis(self.config.max_message_age_ms);
        pending.retain(|packet| !packet.is_expired(max_age));
        
        // Find ready packets
        let mut i = 0;
        while i < pending.len() {
            if pending[i].is_ready() {
                let mut packet = pending.remove(i);
                
                if let Err(e) = packet.advance_hop(Duration::from_millis(self.config.mix_delay_ms)) {
                    tracing::warn!("Failed to advance packet hop: {}", e);
                    continue;
                }
                
                ready_packets.push(packet);
            } else {
                i += 1;
            }
        }
        
        Ok(ready_packets)
    }
    
    /// Generates cover traffic.
    pub async fn generate_cover_traffic(&self) -> Result<Vec<MixPacket>> {
        let num_packets = (self.config.cover_traffic_rate * 0.1) as usize; // 100ms worth
        let mut cover_packets = Vec::new();
        
        for _ in 0..num_packets {
            if let Ok(route) = self.create_route(self.config.num_nodes).await {
                // Create dummy payload
                let payload = vec![0u8; 1024];
                let packet = MixPacket::new(payload, route);
                cover_packets.push(packet);
            }
        }
        
        Ok(cover_packets)
    }
    
    /// Updates node statistics.
    pub async fn update_node_stats(&self, node_id: PeerId, load: f64) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(&node_id) {
            node.update_load(load);
        }
        Ok(())
    }
    
    /// Gets network statistics.
    pub async fn get_stats(&self) -> MixNetworkStats {
        let nodes = self.nodes.read().await;
        let pending = self.pending_packets.read().await;
        
        let total_nodes = nodes.len();
        let available_nodes = nodes.values().filter(|n| n.is_available()).count();
        let pending_packets = pending.len();
        let average_load = if total_nodes > 0 {
            nodes.values().map(|n| n.load).sum::<f64>() / total_nodes as f64
        } else {
            0.0
        };
        
        MixNetworkStats {
            total_nodes,
            available_nodes,
            pending_packets,
            average_load,
        }
    }
}

/// Mix network statistics.
#[derive(Debug, Clone, Serialize)]
pub struct MixNetworkStats {
    /// Total number of nodes
    pub total_nodes: usize,
    /// Number of available nodes
    pub available_nodes: usize,
    /// Number of pending packets
    pub pending_packets: usize,
    /// Average node load
    pub average_load: f64,
}

impl Default for MixNetwork {
    fn default() -> Self {
        Self::new(MixNetworkConfig::default())
    }
}