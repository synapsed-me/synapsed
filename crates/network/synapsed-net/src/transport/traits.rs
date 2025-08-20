//! Transport traits and abstractions.

use crate::error::Result;
use crate::types::{ConnectionInfo, PeerInfo, TransportType};
use async_trait::async_trait;
use std::net::SocketAddr;
// Removed unused Pin, Context, Poll imports
use tokio::io::{AsyncRead, AsyncWrite};

/// Core transport trait that all transport implementations must satisfy.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Connects to a peer.
    async fn connect(&self, peer: &PeerInfo) -> Result<crate::transport::Connection>;
    
    /// Listens for incoming connections.
    async fn listen(&self, addr: SocketAddr) -> Result<Box<dyn Listener>>;
    
    /// Returns the priority of this transport.
    fn priority(&self) -> TransportPriority;
    
    /// Returns the type of this transport.
    fn transport_type(&self) -> TransportType;
    
    /// Checks if this transport supports a specific feature.
    fn supports_feature(&self, feature: TransportFeature) -> bool;
}

/// Listener trait for accepting incoming connections.
#[async_trait]
pub trait Listener: Send + Sync {
    /// Accepts a new connection.
    async fn accept(&mut self) -> Result<(crate::transport::Connection, SocketAddr)>;
    
    /// Returns the local address this listener is bound to.
    fn local_addr(&self) -> Result<SocketAddr>;
    
    /// Closes the listener.
    async fn close(&mut self) -> Result<()>;
}

/// Stream trait for bidirectional communication.
pub trait Stream: AsyncRead + AsyncWrite + Send + Sync + Unpin {
    /// Returns information about this stream.
    fn info(&self) -> ConnectionInfo;
    
    /// Closes the stream gracefully.
    fn close(&mut self) -> Result<()>;
}

/// Transport features that can be queried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportFeature {
    /// Zero Round Trip Time (0-RTT) connection establishment
    ZeroRTT,
    
    /// Support for multiple streams over a single connection
    Multistream,
    
    /// Support for unreliable (datagram) channels
    UnreliableChannel,
    
    /// Built-in NAT traversal capabilities
    NATTraversal,
    
    /// Anonymous connections
    Anonymity,
    
    /// Post-quantum secure
    PostQuantum,
    
    /// Connection migration (e.g., WiFi to cellular)
    ConnectionMigration,
    
    /// Bandwidth estimation
    BandwidthEstimation,
}

/// Transport priority for selection algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransportPriority {
    /// Lowest priority (fallback transports)
    Fallback = 0,
    
    /// Low priority
    Low = 1,
    
    /// Medium priority
    Medium = 2,
    
    /// High priority
    High = 3,
    
    /// Preferred transports
    Preferred = 4,
    
    /// Required transport (must use)
    Required = 5,
}

impl Default for TransportPriority {
    fn default() -> Self {
        Self::Medium
    }
}

/// Requirements for transport selection.
#[derive(Debug, Clone, Default)]
pub struct TransportRequirements {
    /// Features that are required
    pub required_features: Vec<TransportFeature>,
    
    /// Features that are preferred but not required
    pub preferred_features: Vec<TransportFeature>,
    
    /// Maximum acceptable latency in milliseconds
    pub max_latency_ms: Option<u64>,
    
    /// Minimum required bandwidth in Mbps
    pub min_bandwidth_mbps: Option<f64>,
    
    /// Requires anonymity/privacy features
    pub require_anonymity: bool,
    
    /// Requires post-quantum cryptography
    pub require_post_quantum: bool,
}

impl TransportRequirements {
    /// Creates requirements for ultra-low latency applications (e.g., drones).
    pub fn ultra_low_latency() -> Self {
        Self {
            max_latency_ms: Some(10),
            min_bandwidth_mbps: Some(10.0),
            preferred_features: vec![
                TransportFeature::ZeroRTT,
                TransportFeature::ConnectionMigration,
            ],
            ..Default::default()
        }
    }
    
    /// Creates requirements for high privacy applications.
    pub fn high_privacy() -> Self {
        Self {
            require_anonymity: true,
            require_post_quantum: true,
            required_features: vec![TransportFeature::Anonymity],
            ..Default::default()
        }
    }
    
    /// Creates requirements for compliance (e.g., healthcare).
    pub fn compliance() -> Self {
        Self {
            require_post_quantum: true,
            required_features: vec![TransportFeature::PostQuantum],
            ..Default::default()
        }
    }
}

/// Mock stream for testing
#[cfg(test)]
pub struct MockStream {
    pub read_data: Vec<u8>,
    pub write_data: Vec<u8>,
    pub info: ConnectionInfo,
}

#[cfg(test)]
impl AsyncRead for MockStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let len = std::cmp::min(buf.remaining(), self.read_data.len());
        buf.put_slice(&self.read_data[..len]);
        self.read_data.drain(..len);
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
impl AsyncWrite for MockStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.write_data.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }
    
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
impl Stream for MockStream {
    fn info(&self) -> ConnectionInfo {
        self.info.clone()
    }
    
    fn close(&mut self) -> Result<()> {
        Ok(())
    }
}