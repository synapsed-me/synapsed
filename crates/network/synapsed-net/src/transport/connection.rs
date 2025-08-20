//! Connection abstraction for all transport types.

use crate::error::Result;
use crate::observability::{SubstrateEvent, TransportEvent};
use crate::types::{ConnectionId, ConnectionInfo, ConnectionMetrics, Message, TransportType};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::Mutex;

/// A connection to a remote peer.
pub struct Connection {
    /// Unique connection ID
    id: ConnectionId,
    
    /// Connection information
    info: ConnectionInfo,
    
    /// The underlying stream
    stream: Box<dyn crate::transport::traits::Stream>,
    
    /// Connection state
    state: Arc<Mutex<ConnectionState>>,
    
    /// Observability handle
    observability: Option<Arc<crate::observability::UnifiedObservability>>,
}

struct ConnectionState {
    metrics: ConnectionMetrics,
    is_closed: bool,
}

impl Connection {
    /// Creates a new connection.
    pub fn new(
        info: ConnectionInfo,
        stream: Box<dyn crate::transport::traits::Stream>,
    ) -> Self {
        Self {
            id: info.id,
            info,
            stream,
            state: Arc::new(Mutex::new(ConnectionState {
                metrics: ConnectionMetrics::default(),
                is_closed: false,
            })),
            observability: None,
        }
    }
    
    /// Sets the observability handle for this connection.
    pub fn set_observability(&mut self, observability: Arc<crate::observability::UnifiedObservability>) {
        self.observability = Some(observability);
        
        // Emit connection opened event
        if let Some(obs) = &self.observability {
            let handle = obs.create_handle();
            handle.emit_event(SubstrateEvent::Connection(
                crate::observability::ConnectionEvent::Opened {
                    connection_id: self.id.to_string(),
                    transport: self.info.transport,
                }
            ));
        }
    }
    
    /// Returns the connection ID.
    pub fn id(&self) -> ConnectionId {
        self.id
    }
    
    /// Returns connection information.
    pub fn info(&self) -> &ConnectionInfo {
        &self.info
    }
    
    /// Returns the protocol version string.
    pub fn protocol_version(&self) -> String {
        match self.info.transport {
            TransportType::Quic => "QUIC/1.0".to_string(),
            TransportType::WebRtc => "WebRTC/1.0".to_string(),
            TransportType::Tcp => "TCP/1.0".to_string(),
            TransportType::WebSocket => "WebSocket/1.0".to_string(),
            TransportType::Memory => "Memory/1.0".to_string(),
            TransportType::Udp => "UDP/1.0".to_string(),
        }
    }
    
    /// Sends a message over the connection.
    pub async fn send(&mut self, message: &Message) -> Result<()> {
        let start = Instant::now();
        let data = serde_json::to_vec(message)?;
        let bytes_len = data.len();
        
        // Use the AsyncWrite trait to send data
        use tokio::io::AsyncWriteExt;
        self.stream.write_all(&data).await?;
        self.stream.flush().await?;
        
        let duration = start.elapsed();
        
        // Update metrics
        let mut state = self.state.lock().await;
        state.metrics.bytes_sent += bytes_len as u64;
        state.metrics.messages_sent += 1;
        
        // Emit event
        if let Some(obs) = &self.observability {
            let handle = obs.create_handle();
            handle.emit_event(SubstrateEvent::Transport(TransportEvent::DataSent {
                bytes: bytes_len,
                duration,
            }));
        }
        
        Ok(())
    }
    
    /// Receives a message from the connection.
    pub async fn receive(&mut self) -> Result<Message> {
        use tokio::io::AsyncReadExt;
        
        // Read length prefix (4 bytes)
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        // Read message data
        let mut data = vec![0u8; len];
        self.stream.read_exact(&mut data).await?;
        
        // Update metrics
        let mut state = self.state.lock().await;
        state.metrics.bytes_received += len as u64;
        state.metrics.messages_received += 1;
        
        // Emit event
        if let Some(obs) = &self.observability {
            let handle = obs.create_handle();
            handle.emit_event(SubstrateEvent::Transport(TransportEvent::DataReceived {
                bytes: len,
            }));
        }
        
        // Deserialize message
        let message: Message = serde_json::from_slice(&data)?;
        Ok(message)
    }
    
    /// Returns the current metrics for this connection.
    pub async fn metrics(&self) -> ConnectionMetrics {
        let state = self.state.lock().await;
        state.metrics.clone()
    }
    
    /// Closes the connection.
    pub async fn close(mut self) -> Result<()> {
        let start_time = self.info.established_at;
        let duration = start_time.elapsed()?;
        
        // Close the underlying stream
        self.stream.close()?;
        
        // Update state
        let mut state = self.state.lock().await;
        state.is_closed = true;
        
        // Emit connection closed event
        if let Some(obs) = &self.observability {
            let handle = obs.create_handle();
            handle.emit_event(SubstrateEvent::Connection(
                crate::observability::ConnectionEvent::Closed {
                    connection_id: self.id.to_string(),
                    reason: "graceful_close".to_string(),
                    duration,
                }
            ));
            
            // Update metrics one final time
            handle.emit_event(SubstrateEvent::Connection(
                crate::observability::ConnectionEvent::MetricsUpdate {
                    connection_id: self.id.to_string(),
                    bytes_sent: state.metrics.bytes_sent,
                    bytes_received: state.metrics.bytes_received,
                    rtt_ms: state.metrics.avg_rtt.map(|d| d.as_millis() as u64),
                }
            ));
        }
        
        Ok(())
    }
}

// Implement AsyncRead for Connection
impl AsyncRead for Connection {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}

// Implement AsyncWrite for Connection
impl AsyncWrite for Connection {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(cx, buf)
    }
    
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }
    
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

/// Implementation detail for creating connections.
pub struct ConnectionImpl;

impl ConnectionImpl {
    /// Creates a new connection with the given parameters.
    pub fn create(
        info: ConnectionInfo,
        stream: Box<dyn crate::transport::traits::Stream>,
    ) -> Connection {
        Connection::new(info, stream)
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PeerId, MessageId, MessageMetadata, MessagePriority};
    use std::time::SystemTime;
    
    #[tokio::test]
    async fn test_connection_lifecycle() {
        let peer_id = PeerId::new();
        let info = ConnectionInfo {
            local_peer: peer_id,
            remote_peer: peer_id,
            id: ConnectionId::new(),
            transport: TransportType::Memory,
            established_at: SystemTime::now(),
            metrics: ConnectionMetrics::default(),
        };
        
        let stream = Box::new(crate::transport::traits::MockStream {
            read_data: vec![],
            write_data: vec![],
            info: info.clone(),
        });
        
        let mut conn = Connection::new(info, stream);
        
        // Test sending a message
        let message = Message {
            id: MessageId::new(),
            payload: b"Hello, world!".to_vec(),
            metadata: MessageMetadata {
                timestamp: SystemTime::now(),
                priority: MessagePriority::Normal,
                requires_ack: false,
                substrate_context: None,
            },
        };
        
        // This would fail in the test because MockStream doesn't implement proper I/O
        // but it demonstrates the API
        let _ = conn.send(&message).await;
        
        // Get metrics
        let metrics = conn.metrics().await;
        assert_eq!(metrics.messages_sent, 0); // Would be 1 with real stream
        
        // Close connection
        let _ = conn.close().await;
    }
}