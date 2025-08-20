//! Common types used throughout the networking layer.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::SocketAddr;
use std::time::{Duration, Instant, SystemTime};
use uuid::Uuid;

/// Unique identifier for a peer in the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(Uuid);

impl PeerId {
    /// Creates a new random peer ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    
    /// Creates a peer ID from raw bytes.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }
    
    /// Returns the peer ID as raw bytes.
    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
    
    /// Creates an anonymized version of this peer ID for logging.
    pub fn anonymized(&self) -> String {
        let bytes = self.0.as_bytes();
        format!("peer_{:x}{:x}****", bytes[0], bytes[1])
    }
}

impl Default for PeerId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Only show first 8 characters for privacy
        write!(f, "{}", &self.0.to_string()[..8])
    }
}

/// Information about a peer in the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Unique identifier for the peer
    pub id: PeerId,
    
    /// Known addresses for the peer
    pub addresses: Vec<NetworkAddress>,
    
    /// Primary address for connection (convenience field)
    pub address: String,
    
    /// Supported protocols
    pub protocols: Vec<Protocol>,
    
    /// Peer capabilities
    pub capabilities: Vec<String>,
    
    /// Public key for encryption
    pub public_key: Option<PublicKey>,
    
    /// Metadata about the peer
    pub metadata: PeerMetadata,
}

impl PeerInfo {
    /// Creates a new peer info with the given ID.
    pub fn new(id: PeerId) -> Self {
        Self {
            id,
            addresses: Vec::new(),
            address: String::new(),
            protocols: Vec::new(),
            capabilities: Vec::new(),
            public_key: None,
            metadata: PeerMetadata::default(),
        }
    }
    
    /// Adds an address to this peer.
    pub fn add_address(&mut self, address: NetworkAddress) {
        if !self.addresses.contains(&address) {
            self.addresses.push(address);
        }
    }
    
    /// Checks if this peer supports post-quantum cryptography.
    pub fn supports_post_quantum(&self) -> bool {
        self.protocols.iter().any(|p| matches!(p, Protocol::PostQuantum))
    }
    
    /// Creates an anonymized version for observability.
    pub fn anonymized(&self) -> AnonymizedPeerInfo {
        AnonymizedPeerInfo {
            id_hash: blake3::hash(self.id.as_bytes()).to_hex().to_string(),
            protocol_count: self.protocols.len(),
            has_public_key: self.public_key.is_some(),
        }
    }
}

/// Anonymized peer information for observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizedPeerInfo {
    /// Hashed peer ID
    pub id_hash: String,
    
    /// Number of supported protocols
    pub protocol_count: usize,
    
    /// Whether the peer has a public key
    pub has_public_key: bool,
}

impl fmt::Display for AnonymizedPeerInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "peer_{}_{}p", &self.id_hash[..8], self.protocol_count)
    }
}

/// Network address for a peer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkAddress {
    /// Direct TCP/UDP address
    Socket(SocketAddr),
    
    /// QUIC address
    Quic(SocketAddr),
    
    /// WebRTC address (signaling server)
    WebRtc(String),
    
    /// libp2p multiaddr
    Libp2p(String),
    
    /// Tor hidden service
    Tor(String),
    
    /// I2P address
    I2p(String),
}

/// Supported protocols.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    /// QUIC transport
    Quic,
    
    /// WebRTC transport
    WebRtc,
    
    /// libp2p transport
    Libp2p,
    
    /// Noise protocol
    Noise(String),
    
    /// Post-quantum cryptography
    PostQuantum,
    
    /// Custom protocol
    Custom(String),
}

/// Public key types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PublicKey {
    /// Ed25519 public key
    Ed25519(Vec<u8>),
    
    /// X25519 public key
    X25519(Vec<u8>),
    
    /// Post-quantum public key
    PostQuantum(PostQuantumPublicKey),
}

/// Post-quantum public key types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PostQuantumPublicKey {
    /// Kyber public key
    Kyber(Vec<u8>),
    
    /// Dilithium public key
    Dilithium(Vec<u8>),
}

/// Metadata about a peer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PeerMetadata {
    /// When the peer was first seen
    pub first_seen: Option<SystemTime>,
    
    /// When the peer was last seen
    pub last_seen: Option<SystemTime>,
    
    /// Connection quality score (0.0 to 1.0)
    pub quality_score: Option<f64>,
    
    /// Custom metadata
    pub custom: serde_json::Value,
}

/// Connection information.
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Local peer ID
    pub local_peer: PeerId,
    
    /// Remote peer ID
    pub remote_peer: PeerId,
    
    /// Connection ID
    pub id: ConnectionId,
    
    /// Transport used
    pub transport: TransportType,
    
    /// When the connection was established
    pub established_at: SystemTime,
    
    /// Connection metrics
    pub metrics: ConnectionMetrics,
}

/// Unique identifier for a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(pub Uuid);

impl ConnectionId {
    /// Creates a new connection ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Transport types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportType {
    /// QUIC transport
    Quic,
    
    /// WebRTC transport
    WebRtc,
    
    /// TCP transport
    Tcp,
    
    /// UDP transport
    Udp,
    
    /// WebSocket transport
    WebSocket,
    
    /// Memory transport (for testing)
    Memory,
}

impl fmt::Display for TransportType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportType::Quic => write!(f, "QUIC"),
            TransportType::WebRtc => write!(f, "WebRTC"),
            TransportType::Tcp => write!(f, "TCP"),
            TransportType::Udp => write!(f, "UDP"),
            TransportType::WebSocket => write!(f, "WebSocket"),
            TransportType::Memory => write!(f, "Memory"),
        }
    }
}

/// Connection metrics.
#[derive(Debug, Clone, Default)]
pub struct ConnectionMetrics {
    /// Bytes sent
    pub bytes_sent: u64,
    
    /// Bytes received
    pub bytes_received: u64,
    
    /// Messages sent
    pub messages_sent: u64,
    
    /// Messages received
    pub messages_received: u64,
    
    /// Average round-trip time
    pub avg_rtt: Option<Duration>,
    
    /// Packet loss rate (0.0 to 1.0)
    pub packet_loss_rate: Option<f64>,
}

/// Message types for the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message ID
    pub id: MessageId,
    
    /// Message payload
    pub payload: Vec<u8>,
    
    /// Message metadata
    pub metadata: MessageMetadata,
}

/// Unique identifier for a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(Uuid);

impl MessageId {
    /// Creates a new message ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

/// Message metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// When the message was created
    pub timestamp: SystemTime,
    
    /// Message priority
    pub priority: MessagePriority,
    
    /// Whether this message requires acknowledgment
    pub requires_ack: bool,
    
    /// Substrate context for Humanitary.io observability
    pub substrate_context: Option<SubstrateContext>,
}

/// Message priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessagePriority {
    /// Low priority
    Low,
    
    /// Normal priority
    Normal,
    
    /// High priority
    High,
    
    /// Critical priority
    Critical,
}

/// Substrate context for Humanitary.io observability patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateContext {
    /// Circuit path this operation flows through
    pub circuit_path: String,
    
    /// Channel within the circuit
    pub channel: String,
    
    /// Event sequence number for ordering
    pub sequence: u64,
    
    /// Associated subject identifiers
    pub subjects: Vec<String>,
    
    /// Context metadata for Substrates
    pub metadata: serde_json::Value,
}

/// WebRTC signaling messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalingMessage {
    /// Register a peer with the signaling server
    Register {
        peer_id: PeerId,
    },
    
    /// Send an offer to a peer
    Offer {
        from: PeerId,
        to: PeerId,
        sdp: String,
    },
    
    /// Send an answer to a peer
    Answer {
        from: PeerId,
        to: PeerId,
        sdp: String,
    },
    
    /// Send an ICE candidate
    IceCandidate {
        from: PeerId,
        to: PeerId,
        candidate: String,
    },
    
    /// Peer disconnected
    Disconnect {
        peer_id: PeerId,
    },
}

/// Transport metrics for performance monitoring.
#[derive(Debug, Clone)]
pub struct TransportMetrics {
    /// Total connections attempted
    pub connection_attempts: u64,
    
    /// Successful connections
    pub successful_connections: u64,
    
    /// Failed connections
    pub failed_connections: u64,
    
    /// Average connection time
    pub avg_connection_time: Duration,
    
    /// Total bytes sent
    pub bytes_sent: u64,
    
    /// Total bytes received
    pub bytes_received: u64,
    
    /// Last activity timestamp
    pub last_used: Instant,
}

impl Default for TransportMetrics {
    fn default() -> Self {
        Self {
            connection_attempts: 0,
            successful_connections: 0,
            failed_connections: 0,
            avg_connection_time: Duration::from_secs(0),
            bytes_sent: 0,
            bytes_received: 0,
            last_used: Instant::now(),
        }
    }
}