//! P2P transport for anonymous agent communication
//! 
//! This replaces HTTP/TLS with our own protocol stack:
//! - P2P messaging over onion circuits
//! - No HTTP, no client-server model
//! - Pure peer-to-peer with CRDT synchronization

use crate::error::{McpError, Result};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use synapsed_net::{Transport, PeerInfo, Connection};
use synapsed_routing::{OnionRouter, Circuit};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};

/// P2P message types for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum P2PMessage {
    /// Intent declaration (broadcast to all peers)
    IntentDeclare {
        intent_id: String,
        goal: String,
        agent_did: String,
        signature: Vec<u8>,
    },
    
    /// Intent verification (peer-to-peer)
    IntentVerify {
        intent_id: String,
        verifier_did: String,
        verified: bool,
        proof: Vec<u8>,
    },
    
    /// CRDT state sync (gossip protocol)
    StateSync {
        vector_clock: Vec<u8>,
        crdt_delta: Vec<u8>,
    },
    
    /// Agent discovery (DHT announcement)
    AgentAnnounce {
        did: String,
        capabilities: Vec<String>,
        rendezvous_point: String,
    },
    
    /// Trust update (reputation change)
    TrustUpdate {
        agent_did: String,
        trust_delta: i64,
        reason: String,
    },
}

/// P2P transport using our network stack
pub struct P2PTransport {
    /// Our anonymous DID
    our_did: String,
    /// Onion router for anonymous communication
    router: Arc<OnionRouter>,
    /// Active P2P connections (through onion circuits)
    peers: Arc<RwLock<Vec<PeerConnection>>>,
    /// Message handler
    message_handler: Arc<RwLock<Box<dyn MessageHandler>>>,
}

/// A peer connection through onion routing
struct PeerConnection {
    /// Peer's anonymous DID
    peer_did: String,
    /// Onion circuit to this peer
    circuit: Circuit,
    /// Last seen timestamp
    last_seen: std::time::Instant,
}

/// Handler for incoming P2P messages
#[async_trait::async_trait]
pub trait MessageHandler: Send + Sync {
    async fn handle_message(&self, from: &str, message: P2PMessage) -> Result<Option<P2PMessage>>;
}

impl P2PTransport {
    /// Create new P2P transport
    pub async fn new(our_did: String, router: Arc<OnionRouter>) -> Result<Self> {
        info!("Creating P2P transport for {}", our_did);
        
        Ok(Self {
            our_did,
            router,
            peers: Arc::new(RwLock::new(Vec::new())),
            message_handler: Arc::new(RwLock::new(Box::new(DefaultMessageHandler))),
        })
    }
    
    /// Broadcast message to all peers (through onion circuits)
    pub async fn broadcast(&self, message: P2PMessage) -> Result<()> {
        debug!("Broadcasting P2P message: {:?}", message);
        
        let serialized = bincode::serialize(&message)
            .map_err(|e| McpError::SerializationError(e.to_string()))?;
        
        let peers = self.peers.read().await;
        for peer in peers.iter() {
            // Send through onion circuit - completely anonymous
            self.router.send_anonymous(&peer.circuit, &serialized).await
                .map_err(|e| McpError::Transport(format!("Failed to send to peer: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Send message to specific peer (through their rendezvous point)
    pub async fn send_to_peer(&self, peer_did: &str, message: P2PMessage) -> Result<()> {
        let peers = self.peers.read().await;
        
        if let Some(peer) = peers.iter().find(|p| p.peer_did == peer_did) {
            let serialized = bincode::serialize(&message)
                .map_err(|e| McpError::SerializationError(e.to_string()))?;
            
            self.router.send_anonymous(&peer.circuit, &serialized).await
                .map_err(|e| McpError::Transport(format!("Failed to send to peer {}: {}", peer_did, e)))?;
        } else {
            // Peer not directly connected - use DHT routing
            self.route_through_dht(peer_did, message).await?;
        }
        
        Ok(())
    }
    
    /// Route message through DHT (Kademlia over onion)
    async fn route_through_dht(&self, target_did: &str, message: P2PMessage) -> Result<()> {
        debug!("Routing message to {} through DHT", target_did);
        
        // This would use Kademlia routing over onion circuits
        // No direct connection needed - fully anonymous
        
        Ok(())
    }
    
    /// Join the P2P network at a rendezvous point
    pub async fn join_network(&self, rendezvous_point: &str) -> Result<()> {
        info!("Joining P2P network at rendezvous {}", rendezvous_point);
        
        // Create onion circuit to rendezvous point
        let circuit = self.router.create_circuit().await
            .map_err(|e| McpError::Transport(format!("Failed to create circuit: {}", e)))?;
        
        // Announce ourselves (anonymously)
        let announce = P2PMessage::AgentAnnounce {
            did: self.our_did.clone(),
            capabilities: vec!["intent".to_string(), "verify".to_string()],
            rendezvous_point: format!("onion:{}", circuit.id()),
        };
        
        let serialized = bincode::serialize(&announce)
            .map_err(|e| McpError::SerializationError(e.to_string()))?;
        
        self.router.send_anonymous(&circuit, &serialized).await
            .map_err(|e| McpError::Transport(format!("Failed to announce: {}", e)))?;
        
        Ok(())
    }
    
    /// Handle incoming P2P message (received through onion circuit)
    pub async fn handle_incoming(&self, from_circuit: &str, data: &[u8]) -> Result<()> {
        let message: P2PMessage = bincode::deserialize(data)
            .map_err(|e| McpError::SerializationError(e.to_string()))?;
        
        debug!("Received P2P message from circuit {}: {:?}", from_circuit, message);
        
        // Extract peer DID from message if available
        let peer_did = match &message {
            P2PMessage::AgentAnnounce { did, .. } => did.clone(),
            P2PMessage::IntentDeclare { agent_did, .. } => agent_did.clone(),
            P2PMessage::IntentVerify { verifier_did, .. } => verifier_did.clone(),
            _ => format!("anonymous_{}", from_circuit),
        };
        
        // Handle the message
        let handler = self.message_handler.read().await;
        if let Some(response) = handler.handle_message(&peer_did, message).await? {
            // Send response back through same circuit
            self.send_response(from_circuit, response).await?;
        }
        
        Ok(())
    }
    
    /// Send response back through circuit
    async fn send_response(&self, circuit_id: &str, message: P2PMessage) -> Result<()> {
        // This would send back through the same onion circuit
        // maintaining anonymity in both directions
        Ok(())
    }
}

/// Default message handler implementation
struct DefaultMessageHandler;

#[async_trait::async_trait]
impl MessageHandler for DefaultMessageHandler {
    async fn handle_message(&self, from: &str, message: P2PMessage) -> Result<Option<P2PMessage>> {
        match message {
            P2PMessage::IntentDeclare { intent_id, goal, .. } => {
                info!("Received intent {} from {}: {}", intent_id, from, goal);
                // Verify and potentially respond
                Ok(None)
            }
            P2PMessage::StateSync { .. } => {
                debug!("Received state sync from {}", from);
                // Merge CRDT state
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}

/// Convert MCP protocol to P2P messages
pub fn mcp_to_p2p(request: &JsonRpcRequest) -> Result<P2PMessage> {
    match request.method.as_str() {
        "intent/declare" => {
            let params = request.params.as_ref()
                .ok_or_else(|| McpError::InvalidParams("No params".to_string()))?;
            
            Ok(P2PMessage::IntentDeclare {
                intent_id: uuid::Uuid::new_v4().to_string(),
                goal: params["goal"].as_str().unwrap_or("").to_string(),
                agent_did: "self".to_string(), // Would be our DID
                signature: vec![],  // Would be signed
            })
        }
        _ => Err(McpError::InvalidMethod(format!("Unknown method: {}", request.method))),
    }
}