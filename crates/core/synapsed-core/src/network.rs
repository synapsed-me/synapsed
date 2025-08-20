//! Network abstractions and utilities for the Synapsed ecosystem.
//!
//! This module provides common networking traits and utilities that can be
//! used across all network-related Synapsed components.

use crate::{SynapsedError, SynapsedResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use uuid::Uuid;

/// Network address abstraction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NetworkAddress {
    /// IPv4/IPv6 socket address
    Socket(SocketAddr),
    /// Peer ID for P2P networks
    PeerId(String),
    /// DID (Decentralized Identifier)
    Did(String),
    /// Multiaddr for libp2p
    Multiaddr(String),
    /// WebRTC connection string
    WebRtc(String),
    /// Custom address format
    Custom { 
        /// Protocol name
        protocol: String, 
        /// Address string
        address: String 
    },
}

impl NetworkAddress {
    /// Get the protocol name
    #[must_use] pub fn protocol(&self) -> &str {
        match self {
            NetworkAddress::Socket(_) => "tcp",
            NetworkAddress::PeerId(_) => "p2p",
            NetworkAddress::Did(_) => "did",
            NetworkAddress::Multiaddr(_) => "multiaddr",
            NetworkAddress::WebRtc(_) => "webrtc",
            NetworkAddress::Custom { protocol, .. } => protocol,
        }
    }

    /// Get the address as a string
    #[must_use] pub fn address_string(&self) -> String {
        match self {
            NetworkAddress::Socket(addr) => addr.to_string(),
            NetworkAddress::PeerId(id) => id.clone(),
            NetworkAddress::Did(did) => did.clone(),
            NetworkAddress::Multiaddr(addr) => addr.clone(),
            NetworkAddress::WebRtc(conn) => conn.clone(),
            NetworkAddress::Custom { address, .. } => address.clone(),
        }
    }
}

impl std::fmt::Display for NetworkAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}://{}", self.protocol(), self.address_string())
    }
}

impl std::str::FromStr for NetworkAddress {
    type Err = SynapsedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(socket_addr) = s.parse::<SocketAddr>() {
            return Ok(NetworkAddress::Socket(socket_addr));
        }

        if let Some((protocol, address)) = s.split_once("://") {
            match protocol {
                "tcp" | "udp" => {
                    let socket_addr = address.parse::<SocketAddr>()
                        .map_err(|_| SynapsedError::network(format!("Invalid socket address: {address}")))?;
                    Ok(NetworkAddress::Socket(socket_addr))
                }
                "p2p" => Ok(NetworkAddress::PeerId(address.to_string())),
                "did" => Ok(NetworkAddress::Did(address.to_string())),
                "multiaddr" => Ok(NetworkAddress::Multiaddr(address.to_string())),
                "webrtc" => Ok(NetworkAddress::WebRtc(address.to_string())),
                _ => Ok(NetworkAddress::Custom {
                    protocol: protocol.to_string(),
                    address: address.to_string(),
                }),
            }
        } else {
            Err(SynapsedError::network(format!("Invalid network address format: {s}")))
        }
    }
}

/// Connection state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectionState {
    /// Disconnected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected
    Connected,
    /// Disconnecting
    Disconnecting,
    /// Failed connection
    Failed,
}

/// Connection metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetadata {
    /// Connection ID
    pub id: Uuid,
    /// Local address
    pub local_address: NetworkAddress,
    /// Remote address
    pub remote_address: NetworkAddress,
    /// Connection state
    pub state: ConnectionState,
    /// Connection start time
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last activity time
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// Protocol version
    pub protocol_version: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Trait for network connections
#[async_trait]
pub trait NetworkConnection: Send + Sync {
    /// Get connection metadata
    fn metadata(&self) -> &ConnectionMetadata;

    /// Send data
    async fn send(&mut self, data: &[u8]) -> SynapsedResult<usize>;

    /// Receive data
    async fn receive(&mut self, buffer: &mut [u8]) -> SynapsedResult<usize>;

    /// Close the connection
    async fn close(&mut self) -> SynapsedResult<()>;

    /// Check if connection is active
    fn is_active(&self) -> bool {
        matches!(self.metadata().state, ConnectionState::Connected)
    }

    /// Get local address
    fn local_address(&self) -> &NetworkAddress {
        &self.metadata().local_address
    }

    /// Get remote address
    fn remote_address(&self) -> &NetworkAddress {
        &self.metadata().remote_address
    }
}

/// Trait for network listeners
#[async_trait]
pub trait NetworkListener: Send + Sync {
    /// Connection type
    type Connection: NetworkConnection;

    /// Start listening
    async fn start(&mut self) -> SynapsedResult<()>;

    /// Stop listening
    async fn stop(&mut self) -> SynapsedResult<()>;

    /// Accept a new connection
    async fn accept(&mut self) -> SynapsedResult<Self::Connection>;

    /// Get local address
    fn local_address(&self) -> &NetworkAddress;

    /// Check if listener is active
    fn is_listening(&self) -> bool;
}

/// Trait for network clients
#[async_trait]
pub trait NetworkClient: Send + Sync {
    /// Connection type
    type Connection: NetworkConnection;

    /// Connect to a remote address
    async fn connect(&mut self, address: &NetworkAddress) -> SynapsedResult<Self::Connection>;

    /// Get supported protocols
    fn supported_protocols(&self) -> Vec<String>;
}

/// Message envelope for network communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    /// Message ID
    pub id: Uuid,
    /// Message type
    pub message_type: String,
    /// Payload
    pub payload: Vec<u8>,
    /// Headers
    pub headers: HashMap<String, String>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Sender address
    pub sender: Option<NetworkAddress>,
    /// Recipient address
    pub recipient: Option<NetworkAddress>,
}

impl NetworkMessage {
    /// Create a new message
    #[must_use] pub fn new(message_type: &str, payload: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type: message_type.to_string(),
            payload,
            headers: HashMap::new(),
            timestamp: chrono::Utc::now(),
            sender: None,
            recipient: None,
        }
    }

    /// Add a header
    pub fn with_header<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set sender
    #[must_use] pub fn with_sender(mut self, sender: NetworkAddress) -> Self {
        self.sender = Some(sender);
        self
    }

    /// Set recipient
    #[must_use] pub fn with_recipient(mut self, recipient: NetworkAddress) -> Self {
        self.recipient = Some(recipient);
        self
    }

    /// Get header value
    #[must_use] pub fn get_header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(std::string::String::as_str)
    }

    /// Get payload size
    #[must_use] pub fn payload_size(&self) -> usize {
        self.payload.len()
    }
}

/// Network statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkStats {
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Messages sent
    pub messages_sent: u64,
    /// Messages received
    pub messages_received: u64,
    /// Connection count
    pub connection_count: u32,
    /// Error count
    pub error_count: u64,
    /// Last error
    pub last_error: Option<String>,
    /// Uptime
    pub uptime_seconds: u64,
}

impl NetworkStats {
    /// Create new network stats
    #[must_use] pub fn new() -> Self {
        Self::default()
    }

    /// Record bytes sent
    pub fn record_bytes_sent(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
    }

    /// Record bytes received
    pub fn record_bytes_received(&mut self, bytes: u64) {
        self.bytes_received += bytes;
    }

    /// Record message sent
    pub fn record_message_sent(&mut self) {
        self.messages_sent += 1;
    }

    /// Record message received
    pub fn record_message_received(&mut self) {
        self.messages_received += 1;
    }

    /// Record connection
    pub fn record_connection(&mut self) {
        self.connection_count += 1;
    }

    /// Record disconnection
    pub fn record_disconnection(&mut self) {
        if self.connection_count > 0 {
            self.connection_count -= 1;
        }
    }

    /// Record error
    pub fn record_error(&mut self, error: &str) {
        self.error_count += 1;
        self.last_error = Some(error.to_string());
    }

    /// Update uptime
    pub fn update_uptime(&mut self, seconds: u64) {
        self.uptime_seconds = seconds;
    }

    /// Get throughput (bytes per second)
    #[must_use] pub fn throughput(&self) -> f64 {
        if self.uptime_seconds > 0 {
            (self.bytes_sent + self.bytes_received) as f64 / self.uptime_seconds as f64
        } else {
            0.0
        }
    }

    /// Get message rate (messages per second)
    #[must_use] pub fn message_rate(&self) -> f64 {
        if self.uptime_seconds > 0 {
            (self.messages_sent + self.messages_received) as f64 / self.uptime_seconds as f64
        } else {
            0.0
        }
    }

    /// Get error rate (errors per second)
    #[must_use] pub fn error_rate(&self) -> f64 {
        if self.uptime_seconds > 0 {
            self.error_count as f64 / self.uptime_seconds as f64
        } else {
            0.0
        }
    }
}

/// Trait for network components with statistics
pub trait NetworkStatistics {
    /// Get network statistics
    fn stats(&self) -> &NetworkStats;
    
    /// Get mutable network statistics
    fn stats_mut(&mut self) -> &mut NetworkStats;
    
    /// Reset statistics
    fn reset_stats(&mut self) {
        *self.stats_mut() = NetworkStats::new();
    }
}

/// Network event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkEvent {
    /// Connection established
    ConnectionEstablished {
        /// Connection ID
        connection_id: Uuid,
        /// Remote address
        remote_address: NetworkAddress,
    },
    /// Connection lost
    ConnectionLost {
        /// Connection ID
        connection_id: Uuid,
        /// Reason
        reason: String,
    },
    /// Message received
    MessageReceived {
        /// Connection ID
        connection_id: Uuid,
        /// Message
        message: NetworkMessage,
    },
    /// Message sent
    MessageSent {
        /// Connection ID
        connection_id: Uuid,
        /// Message ID
        message_id: Uuid,
    },
    /// Network error
    NetworkError {
        /// Error message
        error: String,
        /// Context
        context: HashMap<String, String>,
    },
}

/// Trait for network event handling
#[async_trait]
pub trait NetworkEventHandler: Send + Sync {
    /// Handle a network event
    async fn handle_event(&mut self, event: NetworkEvent) -> SynapsedResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_network_address() {
        let socket_addr = NetworkAddress::Socket("127.0.0.1:8080".parse().unwrap());
        assert_eq!(socket_addr.protocol(), "tcp");
        assert_eq!(socket_addr.address_string(), "127.0.0.1:8080");
        assert_eq!(socket_addr.to_string(), "tcp://127.0.0.1:8080");

        let peer_id = NetworkAddress::PeerId("12D3KooW...".to_string());
        assert_eq!(peer_id.protocol(), "p2p");
        assert_eq!(peer_id.to_string(), "p2p://12D3KooW...");

        let did = NetworkAddress::Did("did:example:123".to_string());
        assert_eq!(did.protocol(), "did");
        assert_eq!(did.to_string(), "did://did:example:123");
    }

    #[test]
    fn test_network_address_parsing() {
        let addr = NetworkAddress::from_str("127.0.0.1:8080").unwrap();
        assert!(matches!(addr, NetworkAddress::Socket(_)));

        let addr = NetworkAddress::from_str("tcp://127.0.0.1:8080").unwrap();
        assert!(matches!(addr, NetworkAddress::Socket(_)));

        let addr = NetworkAddress::from_str("p2p://12D3KooW...").unwrap();
        assert!(matches!(addr, NetworkAddress::PeerId(_)));

        let addr = NetworkAddress::from_str("did://did:example:123").unwrap();
        assert!(matches!(addr, NetworkAddress::Did(_)));

        let addr = NetworkAddress::from_str("custom://some-address").unwrap();
        assert!(matches!(addr, NetworkAddress::Custom { .. }));

        assert!(NetworkAddress::from_str("invalid-format").is_err());
    }

    #[test]
    fn test_network_message() {
        let payload = b"hello world".to_vec();
        let msg = NetworkMessage::new("test.message", payload.clone())
            .with_header("version", "1.0")
            .with_sender(NetworkAddress::PeerId("sender".to_string()))
            .with_recipient(NetworkAddress::PeerId("recipient".to_string()));

        assert_eq!(msg.message_type, "test.message");
        assert_eq!(msg.payload, payload);
        assert_eq!(msg.get_header("version"), Some("1.0"));
        assert_eq!(msg.payload_size(), payload.len());
        assert!(msg.sender.is_some());
        assert!(msg.recipient.is_some());
    }

    #[test]
    fn test_network_stats() {
        let mut stats = NetworkStats::new();
        
        stats.record_bytes_sent(100);
        stats.record_bytes_received(200);
        stats.record_message_sent();
        stats.record_message_received();
        stats.record_connection();
        stats.record_error("test error");
        stats.update_uptime(10);

        assert_eq!(stats.bytes_sent, 100);
        assert_eq!(stats.bytes_received, 200);
        assert_eq!(stats.messages_sent, 1);
        assert_eq!(stats.messages_received, 1);
        assert_eq!(stats.connection_count, 1);
        assert_eq!(stats.error_count, 1);
        assert_eq!(stats.uptime_seconds, 10);
        assert_eq!(stats.last_error, Some("test error".to_string()));

        assert_eq!(stats.throughput(), 30.0); // (100 + 200) / 10
        assert_eq!(stats.message_rate(), 0.2); // (1 + 1) / 10
        assert_eq!(stats.error_rate(), 0.1); // 1 / 10

        stats.record_disconnection();
        assert_eq!(stats.connection_count, 0);
    }

    #[test]
    fn test_connection_state() {
        let state = ConnectionState::Connected;
        assert_eq!(state, ConnectionState::Connected);
        assert_ne!(state, ConnectionState::Disconnected);
    }

    #[test]
    fn test_network_event() {
        let event = NetworkEvent::ConnectionEstablished {
            connection_id: Uuid::new_v4(),
            remote_address: NetworkAddress::PeerId("peer123".to_string()),
        };

        match event {
            NetworkEvent::ConnectionEstablished { remote_address, .. } => {
                assert_eq!(remote_address.protocol(), "p2p");
            }
            _ => panic!("Unexpected event type"),
        }
    }
}