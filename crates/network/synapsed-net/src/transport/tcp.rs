//! TCP transport implementation for reliable stream connections.

use crate::error::{NetworkError, Result, TransportError};
use crate::transport::traits::{Listener, Stream, Transport, TransportFeature, TransportPriority};
use crate::transport::Connection;
use crate::types::{ConnectionId, ConnectionInfo, PeerInfo, PeerId, TransportType};
use async_trait::async_trait;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{error, info};

/// TCP transport implementation providing reliable stream connections.
pub struct TcpTransport;

impl TcpTransport {
    /// Creates a new TCP transport.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TcpTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn connect(&self, peer: &PeerInfo) -> Result<Connection> {
        info!("Connecting to peer {} via TCP", peer.id);
        
        let addr: SocketAddr = peer.address.parse()
            .map_err(|e| NetworkError::Transport(TransportError::InvalidAddress(format!("{}: {}", peer.address, e))))?;
        
        let stream = TcpStream::connect(addr).await
            .map_err(|e| NetworkError::Transport(TransportError::Tcp(e.to_string())))?;
        
        // Enable TCP keepalive
        let sock_ref = socket2::SockRef::from(&stream);
        let keepalive = socket2::TcpKeepalive::new()
            .with_time(std::time::Duration::from_secs(30))
            .with_interval(std::time::Duration::from_secs(10));
        sock_ref.set_tcp_keepalive(&keepalive)
            .map_err(|e| NetworkError::Transport(TransportError::Tcp(format!("Failed to set keepalive: {}", e))))?;
        
        // Set TCP_NODELAY to reduce latency
        stream.set_nodelay(true)
            .map_err(|e| NetworkError::Transport(TransportError::Tcp(format!("Failed to set nodelay: {}", e))))?;
        
        let conn_info = ConnectionInfo {
            id: ConnectionId::new(),
            local_peer: PeerId::new(), // TODO: Use actual local peer ID
            remote_peer: peer.id,
            transport: TransportType::Tcp,
            established_at: std::time::SystemTime::now(),
            metrics: Default::default(),
        };
        
        Ok(Connection::new(
            conn_info,
            Box::new(TcpStreamWrapper(stream)) as Box<dyn Stream>,
        ))
    }
    
    async fn listen(&self, addr: SocketAddr) -> Result<Box<dyn Listener>> {
        let listener = TcpListener::bind(addr).await
            .map_err(|e| NetworkError::Transport(TransportError::Tcp(e.to_string())))?;
        
        info!("TCP transport listening on {}", addr);
        
        Ok(Box::new(TcpListenerWrapper::new(listener)))
    }
    
    fn priority(&self) -> TransportPriority {
        TransportPriority::Medium
    }
    
    fn transport_type(&self) -> TransportType {
        TransportType::Tcp
    }
    
    fn supports_feature(&self, feature: TransportFeature) -> bool {
        match feature {
            TransportFeature::ZeroRTT => false,
            TransportFeature::Multistream => false,
            TransportFeature::UnreliableChannel => false,
            TransportFeature::ConnectionMigration => false,
            TransportFeature::BandwidthEstimation => false,
            TransportFeature::NATTraversal => false,
            TransportFeature::Anonymity => false,
            TransportFeature::PostQuantum => false,
        }
    }
}

/// Wrapper for TcpStream to implement the Stream trait.
pub struct TcpStreamWrapper(TcpStream);

impl AsyncRead for TcpStreamWrapper {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl AsyncWrite for TcpStreamWrapper {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }
    
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }
    
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

impl Stream for TcpStreamWrapper {
    fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            id: ConnectionId::new(),
            transport: TransportType::Tcp,
            local_peer: PeerId::new(),
            remote_peer: PeerId::new(),
            established_at: std::time::SystemTime::now(),
            metrics: Default::default(),
        }
    }
    
    fn close(&mut self) -> Result<()> {
        // TCP streams close when dropped
        Ok(())
    }
}

/// Wrapper for TcpListener to implement the Listener trait.
pub struct TcpListenerWrapper {
    listener: Arc<TcpListener>,
    incoming_rx: mpsc::Receiver<(Connection, SocketAddr)>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl TcpListenerWrapper {
    fn new(listener: TcpListener) -> Self {
        let (tx, rx) = mpsc::channel(32);
        let listener = Arc::new(listener);
        let listener_clone = Arc::clone(&listener);
        
        let task_handle = tokio::spawn(async move {
            loop {
                match listener_clone.accept().await {
                    Ok((stream, addr)) => {
                        // Set TCP options
                        let _ = stream.set_nodelay(true);
                        
                        let conn_info = ConnectionInfo {
                            id: ConnectionId::new(),
                            local_peer: PeerId::new(),
                            remote_peer: PeerId::new(),
                            transport: TransportType::Tcp,
                            established_at: std::time::SystemTime::now(),
                            metrics: Default::default(),
                        };
                        
                        let conn = Connection::new(
                            conn_info,
                            Box::new(TcpStreamWrapper(stream)) as Box<dyn Stream>,
                        );
                        
                        if tx.send((conn, addr)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error accepting TCP connection: {}", e);
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }
        });
        
        Self {
            listener,
            incoming_rx: rx,
            task_handle: Some(task_handle),
        }
    }
}

#[async_trait]
impl Listener for TcpListenerWrapper {
    async fn accept(&mut self) -> Result<(Connection, SocketAddr)> {
        self.incoming_rx
            .recv()
            .await
            .ok_or_else(|| NetworkError::Transport(TransportError::NotAvailable("TCP listener closed".to_string())))
    }
    
    fn local_addr(&self) -> Result<SocketAddr> {
        self.listener.local_addr()
            .map_err(|e| NetworkError::Transport(TransportError::Tcp(e.to_string())))
    }
    
    async fn close(&mut self) -> Result<()> {
        self.incoming_rx.close();
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tcp_transport_creation() {
        let transport = TcpTransport::new();
        assert_eq!(transport.priority(), TransportPriority::Medium);
        assert_eq!(transport.transport_type(), TransportType::Tcp);
    }
    
    #[test]
    fn test_tcp_transport_features() {
        let transport = TcpTransport::new();
        
        assert!(!transport.supports_feature(TransportFeature::ZeroRTT));
        assert!(!transport.supports_feature(TransportFeature::Multistream));
        assert!(!transport.supports_feature(TransportFeature::NATTraversal));
        assert!(!transport.supports_feature(TransportFeature::PostQuantum));
    }
}