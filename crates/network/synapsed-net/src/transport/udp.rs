//! UDP transport implementation for unreliable datagram connections.

use crate::error::{NetworkError, Result, TransportError};
use crate::transport::traits::{Listener, Stream, Transport, TransportFeature, TransportPriority};
use crate::transport::Connection;
use crate::types::{ConnectionId, ConnectionInfo, PeerInfo, PeerId, TransportType};
use async_trait::async_trait;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info};
use bytes::{Buf, Bytes, BytesMut};

/// Maximum UDP packet size
const MAX_PACKET_SIZE: usize = 65507;

/// UDP transport implementation providing unreliable datagram connections.
pub struct UdpTransport {
    /// Active connections mapped by peer address
    connections: Arc<Mutex<HashMap<SocketAddr, Arc<UdpSocket>>>>,
}

impl UdpTransport {
    /// Creates a new UDP transport.
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for UdpTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for UdpTransport {
    async fn connect(&self, peer: &PeerInfo) -> Result<Connection> {
        info!("Connecting to peer {} via UDP", peer.id);
        
        let addr: SocketAddr = peer.address.parse()
            .map_err(|e| NetworkError::Transport(TransportError::InvalidAddress(format!("{}: {}", peer.address, e))))?;
        
        // Bind to any available port
        let socket = UdpSocket::bind("0.0.0.0:0").await
            .map_err(|e| NetworkError::Transport(TransportError::Udp(e.to_string())))?;
        
        // Connect to the peer (sets default destination)
        socket.connect(addr).await
            .map_err(|e| NetworkError::Transport(TransportError::Udp(e.to_string())))?;
        
        let socket = Arc::new(socket);
        
        {
            let mut conns = self.connections.lock().await;
            conns.insert(addr, socket.clone());
        }
        
        let conn_info = ConnectionInfo {
            id: ConnectionId::new(),
            local_peer: PeerId::new(), // TODO: Use actual local peer ID
            remote_peer: peer.id,
            transport: TransportType::Udp,
            established_at: std::time::SystemTime::now(),
            metrics: Default::default(),
        };
        
        let stream = UdpStream::new(socket, addr);
        
        Ok(Connection::new(
            conn_info,
            Box::new(stream) as Box<dyn Stream>,
        ))
    }
    
    async fn listen(&self, addr: SocketAddr) -> Result<Box<dyn Listener>> {
        let socket = UdpSocket::bind(addr).await
            .map_err(|e| NetworkError::Transport(TransportError::Udp(e.to_string())))?;
        
        info!("UDP transport listening on {}", addr);
        
        Ok(Box::new(UdpListener::new(Arc::new(socket))))
    }
    
    fn priority(&self) -> TransportPriority {
        TransportPriority::Low // UDP is generally lower priority due to unreliability
    }
    
    fn transport_type(&self) -> TransportType {
        TransportType::Udp
    }
    
    fn supports_feature(&self, feature: TransportFeature) -> bool {
        match feature {
            TransportFeature::ZeroRTT => true, // No handshake needed
            TransportFeature::Multistream => false,
            TransportFeature::UnreliableChannel => true, // UDP is inherently unreliable
            TransportFeature::ConnectionMigration => true, // Can handle IP changes
            TransportFeature::BandwidthEstimation => false,
            TransportFeature::NATTraversal => false, // Basic UDP doesn't handle NAT well
            TransportFeature::Anonymity => false,
            TransportFeature::PostQuantum => false,
        }
    }
}

/// UDP stream implementation that provides AsyncRead/AsyncWrite over datagrams.
pub struct UdpStream {
    socket: Arc<UdpSocket>,
    peer_addr: SocketAddr,
    read_buffer: BytesMut,
    write_buffer: BytesMut,
    read_rx: mpsc::Receiver<Bytes>,
    write_tx: mpsc::Sender<Bytes>,
}

impl UdpStream {
    fn new(socket: Arc<UdpSocket>, peer_addr: SocketAddr) -> Self {
        let (write_tx, mut write_rx) = mpsc::channel::<Bytes>(32);
        let (read_tx, read_rx) = mpsc::channel::<Bytes>(32);
        
        // Spawn read task
        let socket_clone = socket.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; MAX_PACKET_SIZE];
            loop {
                match socket_clone.recv(&mut buf).await {
                    Ok(n) => {
                        let data = Bytes::copy_from_slice(&buf[..n]);
                        if read_tx.send(data).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("UDP receive error: {}", e);
                        break;
                    }
                }
            }
        });
        
        // Spawn write task
        let socket_clone = socket.clone();
        tokio::spawn(async move {
            while let Some(data) = write_rx.recv().await {
                if let Err(e) = socket_clone.send(&data).await {
                    error!("UDP send error: {}", e);
                }
            }
        });
        
        Self {
            socket,
            peer_addr,
            read_buffer: BytesMut::with_capacity(MAX_PACKET_SIZE),
            write_buffer: BytesMut::with_capacity(MAX_PACKET_SIZE),
            read_rx,
            write_tx,
        }
    }
}

impl AsyncRead for UdpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // If we have buffered data, return it
        if !self.read_buffer.is_empty() {
            let len = std::cmp::min(buf.remaining(), self.read_buffer.len());
            buf.put_slice(&self.read_buffer[..len]);
            self.read_buffer.advance(len);
            return Poll::Ready(Ok(()));
        }
        
        // Try to receive new data
        match self.read_rx.poll_recv(cx) {
            Poll::Ready(Some(data)) => {
                let len = std::cmp::min(buf.remaining(), data.len());
                buf.put_slice(&data[..len]);
                
                // Buffer any remaining data
                if len < data.len() {
                    self.read_buffer.extend_from_slice(&data[len..]);
                }
                
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "UDP stream closed",
            ))),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for UdpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        // UDP is message-based, so we need to buffer until flush
        if self.write_buffer.len() + buf.len() > MAX_PACKET_SIZE {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "UDP packet too large",
            )));
        }
        
        self.write_buffer.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }
    
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if self.write_buffer.is_empty() {
            return Poll::Ready(Ok(()));
        }
        
        let data = self.write_buffer.split().freeze();
        match self.write_tx.try_send(data) {
            Ok(_) => Poll::Ready(Ok(())),
            Err(mpsc::error::TrySendError::Full(_)) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "UDP stream closed",
                )))
            }
        }
    }
    
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.poll_flush(cx)
    }
}

impl Stream for UdpStream {
    fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            id: ConnectionId::new(),
            transport: TransportType::Udp,
            local_peer: PeerId::new(),
            remote_peer: PeerId::new(),
            established_at: std::time::SystemTime::now(),
            metrics: Default::default(),
        }
    }
    
    fn close(&mut self) -> Result<()> {
        // UDP doesn't have a close handshake
        Ok(())
    }
}

/// UDP listener implementation.
pub struct UdpListener {
    socket: Arc<UdpSocket>,
    incoming_rx: mpsc::Receiver<(Connection, SocketAddr)>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl UdpListener {
    fn new(socket: Arc<UdpSocket>) -> Self {
        let (tx, rx) = mpsc::channel(32);
        let socket_clone = socket.clone();
        
        // For UDP, we need to track "connections" by source address
        let connections: Arc<Mutex<HashMap<SocketAddr, Arc<UdpSocket>>>> = 
            Arc::new(Mutex::new(HashMap::new()));
        
        let task_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; MAX_PACKET_SIZE];
            
            loop {
                match socket_clone.recv_from(&mut buf).await {
                    Ok((n, addr)) => {
                        let mut conns = connections.lock().await;
                        
                        // Create a new "connection" for each unique source
                        if !conns.contains_key(&addr) {
                            // Create a connected socket for this peer
                            if let Ok(peer_socket) = UdpSocket::bind("0.0.0.0:0").await {
                                if peer_socket.connect(addr).await.is_ok() {
                                    let peer_socket = Arc::new(peer_socket);
                                    conns.insert(addr, peer_socket.clone());
                                    
                                    let conn_info = ConnectionInfo {
                                        id: ConnectionId::new(),
                                        local_peer: PeerId::new(),
                                        remote_peer: PeerId::new(),
                                        transport: TransportType::Udp,
                                        established_at: std::time::SystemTime::now(),
                                        metrics: Default::default(),
                                    };
                                    
                                    let stream = UdpStream::new(peer_socket, addr);
                                    let conn = Connection::new(
                                        conn_info,
                                        Box::new(stream) as Box<dyn Stream>,
                                    );
                                    
                                    let _ = tx.send((conn, addr)).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error receiving UDP packet: {}", e);
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }
        });
        
        Self {
            socket,
            incoming_rx: rx,
            task_handle: Some(task_handle),
        }
    }
}

#[async_trait]
impl Listener for UdpListener {
    async fn accept(&mut self) -> Result<(Connection, SocketAddr)> {
        self.incoming_rx
            .recv()
            .await
            .ok_or_else(|| NetworkError::Transport(TransportError::NotAvailable("UDP listener closed".to_string())))
    }
    
    fn local_addr(&self) -> Result<SocketAddr> {
        self.socket.local_addr()
            .map_err(|e| NetworkError::Transport(TransportError::Udp(e.to_string())))
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
    async fn test_udp_transport_creation() {
        let transport = UdpTransport::new();
        assert_eq!(transport.priority(), TransportPriority::Low);
        assert_eq!(transport.transport_type(), TransportType::Udp);
    }
    
    #[test]
    fn test_udp_transport_features() {
        let transport = UdpTransport::new();
        
        assert!(transport.supports_feature(TransportFeature::ZeroRTT));
        assert!(transport.supports_feature(TransportFeature::UnreliableChannel));
        assert!(transport.supports_feature(TransportFeature::ConnectionMigration));
        assert!(!transport.supports_feature(TransportFeature::Multistream));
        assert!(!transport.supports_feature(TransportFeature::PostQuantum));
    }
}