//! Transport layer abstractions and implementations.
//!
//! This module provides a unified interface for multiple transport protocols
//! with built-in observability and automatic protocol selection.

pub mod connection;
pub mod libp2p_simple;
pub mod manager;
pub mod memory;
pub mod quic;
pub mod signaling;
pub mod tcp;
pub mod traits;
pub mod websocket;
pub mod webrtc;

pub use connection::{Connection, ConnectionImpl};
pub use libp2p_simple::{Libp2pTransport, Libp2pConfig};
pub use manager::TransportManager;
pub use memory::MemoryTransport;
pub use quic::QuicTransport;
pub use signaling::{SignalingClient, WebRTCConnectionPool};
pub use tcp::TcpTransport;
pub use traits::{Transport, TransportFeature, TransportPriority, TransportRequirements};
pub use websocket::{WebSocketTransport, WebSocketConfig};
pub use webrtc::WebRTCTransport;

use crate::error::Result;
use crate::observability::{SubstrateEvent, TransportEvent};
use crate::types::{PeerInfo, TransportType};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

/// Wrapper that adds observability to any transport.
pub struct ObservableTransport {
    inner: Arc<dyn Transport + Send + Sync>,
    observability: Arc<crate::observability::UnifiedObservability>,
    transport_type: TransportType,
}

impl ObservableTransport {
    /// Creates a new observable transport.
    pub fn new(
        inner: Arc<dyn Transport + Send + Sync>,
        observability: Arc<crate::observability::UnifiedObservability>,
        transport_type: TransportType,
    ) -> Self {
        Self {
            inner,
            observability,
            transport_type,
        }
    }
}

#[async_trait]
impl Transport for ObservableTransport {
    async fn connect(&self, peer: &PeerInfo) -> Result<Connection> {
        let start = Instant::now();
        
        // Emit connection attempt event
        let handle = self.observability.create_handle();
        handle.emit_event(SubstrateEvent::Transport(TransportEvent::ConnectionAttempt {
            peer: peer.anonymized(),
            timestamp: start,
            transport_type: format!("{:?}", self.transport_type),
        }));
        
        // Attempt connection
        match self.inner.connect(peer).await {
            Ok(mut conn) => {
                let duration = start.elapsed();
                
                // Emit success event
                handle.emit_event(SubstrateEvent::Transport(TransportEvent::ConnectionEstablished {
                    duration,
                    protocol_version: conn.protocol_version(),
                }));
                
                // Update metrics - placeholder for circuit integration
                // TODO: Implement proper circuit event emission when API is finalized
                
                // Wrap connection with observability
                conn.set_observability(self.observability.clone());
                
                Ok(conn)
            }
            Err(e) => {
                let duration = start.elapsed();
                
                // Emit failure event
                handle.emit_event(SubstrateEvent::Transport(TransportEvent::ConnectionFailed {
                    duration,
                    error_type: e.classify(),
                }));
                
                // Update metrics - placeholder for circuit integration
                // TODO: Implement proper circuit event emission when API is finalized
                
                Err(e)
            }
        }
    }
    
    async fn listen(&self, addr: std::net::SocketAddr) -> Result<Box<dyn crate::transport::traits::Listener>> {
        self.inner.listen(addr).await
    }
    
    fn priority(&self) -> TransportPriority {
        self.inner.priority()
    }
    
    fn transport_type(&self) -> TransportType {
        self.transport_type
    }
    
    fn supports_feature(&self, feature: TransportFeature) -> bool {
        self.inner.supports_feature(feature)
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_observable_transport_wrapper() {
        // This test would require a mock transport implementation
        // Left as a placeholder for when we have concrete transports
    }
}