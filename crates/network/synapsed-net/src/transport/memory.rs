//! In-memory transport for testing and local communication.

use crate::error::Result;
use crate::transport::traits::{Listener, Stream, Transport, TransportFeature, TransportPriority};
use crate::transport::Connection;
use crate::types::{ConnectionId, ConnectionInfo, ConnectionMetrics, PeerInfo, TransportType};
use async_trait::async_trait;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::SystemTime;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::{mpsc, Mutex};
use tracing::info;

/// In-memory transport implementation for testing.
pub struct MemoryTransport {
    /// Registry of listeners
    listeners: Arc<Mutex<HashMap<SocketAddr, mpsc::Sender<MemoryStream>>>>,
}

impl MemoryTransport {
    /// Creates a new memory transport.
    pub fn new() -> Self {
        Self {
            listeners: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for MemoryTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for MemoryTransport {
    async fn connect(&self, peer: &PeerInfo) -> Result<Connection> {
        let addr: SocketAddr = peer.address.parse()
            .map_err(|e| crate::error::NetworkError::Configuration(format!("Invalid address: {}", e)))?;
        
        let listeners = self.listeners.lock().await;
        let sender = listeners.get(&addr)
            .ok_or_else(|| crate::error::NetworkError::Connection("No listener at address".to_string()))?;
        
        // Create bidirectional channels
        let (tx1, rx1) = mpsc::channel::<Vec<u8>>(1024);
        let (tx2, rx2) = mpsc::channel::<Vec<u8>>(1024);
        
        let client_stream = MemoryStream {
            read_rx: rx1,
            write_tx: tx2,
            info: ConnectionInfo {
                local_peer: peer.id,
                remote_peer: peer.id,
                id: ConnectionId::new(),
                transport: TransportType::Memory,
                established_at: SystemTime::now(),
                metrics: ConnectionMetrics::default(),
            },
        };
        
        let server_stream = MemoryStream {
            read_rx: rx2,
            write_tx: tx1,
            info: client_stream.info.clone(),
        };
        
        // Send the server stream to the listener
        let _ = sender.send(server_stream).await;
        
        let conn = Connection::new(client_stream.info.clone(), Box::new(client_stream));
        
        info!("Memory transport connected to {}", peer.anonymized());
        
        Ok(conn)
    }
    
    async fn listen(&self, addr: SocketAddr) -> Result<Box<dyn Listener>> {
        let (tx, rx) = mpsc::channel(1024);
        
        let mut listeners = self.listeners.lock().await;
        listeners.insert(addr, tx);
        
        info!("Memory transport listening on {}", addr);
        
        Ok(Box::new(MemoryListener {
            receiver: rx,
            local_addr: addr,
        }))
    }
    
    fn priority(&self) -> TransportPriority {
        TransportPriority::Low
    }
    
    fn transport_type(&self) -> TransportType {
        TransportType::Memory
    }
    
    fn supports_feature(&self, feature: TransportFeature) -> bool {
        match feature {
            TransportFeature::ZeroRTT => true,
            TransportFeature::Multistream => false,
            TransportFeature::UnreliableChannel => false,
            TransportFeature::ConnectionMigration => false,
            TransportFeature::BandwidthEstimation => false,
            TransportFeature::NATTraversal => true,
            TransportFeature::Anonymity => false,
            TransportFeature::PostQuantum => false,
        }
    }
}

/// Memory transport listener.
pub struct MemoryListener {
    receiver: mpsc::Receiver<MemoryStream>,
    local_addr: SocketAddr,
}

#[async_trait]
impl Listener for MemoryListener {
    async fn accept(&mut self) -> Result<(Connection, SocketAddr)> {
        let stream = self.receiver.recv().await
            .ok_or_else(|| crate::error::NetworkError::Connection("Listener closed".to_string()))?;
        
        let info = stream.info.clone();
        let conn = Connection::new(info, Box::new(stream));
        
        Ok((conn, self.local_addr))
    }
    
    fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.local_addr)
    }
    
    async fn close(&mut self) -> Result<()> {
        self.receiver.close();
        Ok(())
    }
}

/// Memory transport stream.
pub struct MemoryStream {
    read_rx: mpsc::Receiver<Vec<u8>>,
    write_tx: mpsc::Sender<Vec<u8>>,
    info: ConnectionInfo,
}

impl Stream for MemoryStream {
    fn info(&self) -> ConnectionInfo {
        self.info.clone()
    }
    
    fn close(&mut self) -> Result<()> {
        self.read_rx.close();
        Ok(())
    }
}

impl AsyncRead for MemoryStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let me = self.get_mut();
        match me.read_rx.poll_recv(cx) {
            Poll::Ready(Some(data)) => {
                let len = std::cmp::min(buf.remaining(), data.len());
                buf.put_slice(&data[..len]);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for MemoryStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.write_tx.try_send(buf.to_vec()) {
            Ok(()) => Poll::Ready(Ok(buf.len())),
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => Poll::Pending,
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "Channel closed",
                )))
            }
        }
    }
    
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PeerId;
    
    #[tokio::test]
    async fn test_memory_transport() {
        let transport = MemoryTransport::new();
        let addr = "127.0.0.1:8080".parse().unwrap();
        
        // Start listener
        let mut listener = transport.listen(addr).await.unwrap();
        
        // Connect
        let peer = PeerInfo {
            id: PeerId::new(),
            address: addr.to_string(),
            addresses: vec![NetworkAddress::Socket(addr)],
            protocols: vec![],
            capabilities: vec![],
            public_key: None,
            metadata: PeerMetadata::default(),
        };
        
        // Spawn accept task
        let accept_task = tokio::spawn(async move {
            listener.accept().await
        });
        
        // Connect
        let _conn = transport.connect(&peer).await.unwrap();
        
        // Wait for accept
        let (_server_conn, _) = accept_task.await.unwrap().unwrap();
        
        // Both connections established
        assert_eq!(transport.transport_type(), TransportType::Memory);
        assert_eq!(transport.priority(), TransportPriority::Low);
    }
}