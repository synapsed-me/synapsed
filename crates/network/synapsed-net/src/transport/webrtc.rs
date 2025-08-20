//! WebRTC transport implementation for browser compatibility and NAT traversal.

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
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info};
use webrtc::api::APIBuilder;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::RTCPeerConnection;

/// WebRTC transport for browser-compatible P2P connections.
pub struct WebRTCTransport {
    /// Active peer connections
    connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
    
    /// ICE servers for NAT traversal
    ice_servers: Vec<RTCIceServer>,
    
    /// WebRTC API instance
    api: Arc<webrtc::api::API>,
    
    /// Signaling server address
    signaling_server: Option<String>,
}

impl WebRTCTransport {
    /// Creates a new WebRTC transport.
    pub fn new(signaling_server: Option<String>) -> Result<Self> {
        // Default STUN servers
        let ice_servers = vec![
            RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            },
            RTCIceServer {
                urls: vec!["stun:stun1.l.google.com:19302".to_owned()],
                ..Default::default()
            },
        ];
        
        // Build WebRTC API
        let api = APIBuilder::new().build();
        
        Ok(Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            ice_servers,
            api: Arc::new(api),
            signaling_server,
        })
    }
    
    /// Creates a new peer connection.
    async fn create_peer_connection(&self) -> Result<Arc<RTCPeerConnection>> {
        let config = RTCConfiguration {
            ice_servers: self.ice_servers.clone(),
            ..Default::default()
        };
        
        self.api
            .new_peer_connection(config)
            .await
            .map(Arc::new)
            .map_err(|e| NetworkError::Transport(TransportError::WebRtc(e.to_string())))
    }
}

#[async_trait]
impl Transport for WebRTCTransport {
    async fn connect(&self, peer: &PeerInfo) -> Result<Connection> {
        info!("Connecting to peer {} via WebRTC", peer.id);
        
        // Create peer connection
        let pc = self.create_peer_connection().await?;
        
        // Set up connection state handler
        let peer_id = peer.id.clone();
        let _connections = self.connections.clone();
        pc.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
            debug!("Peer {} connection state: {:?}", peer_id, s);
            Box::pin(async {})
        }));
        
        // Create data channel for communication
        let data_channel = pc
            .create_data_channel("data", None)
            .await
            .map_err(|e| NetworkError::Transport(TransportError::WebRtc(e.to_string())))?;
        
        // Store the connection
        {
            let mut conns = self.connections.lock().await;
            conns.insert(peer.id.to_string(), pc.clone());
        }
        
        // Create offer and handle signaling
        let offer = pc
            .create_offer(None)
            .await
            .map_err(|e| NetworkError::Transport(TransportError::WebRtc(e.to_string())))?;
        
        pc.set_local_description(offer.clone())
            .await
            .map_err(|e| NetworkError::Transport(TransportError::WebRtc(e.to_string())))?;
        
        // TODO: Send offer through signaling server and wait for answer
        // For now, return a placeholder connection
        
        let conn_info = ConnectionInfo {
            id: ConnectionId::new(),
            local_peer: PeerId::new(),  // TODO: Use actual local peer ID
            remote_peer: peer.id,
            transport: TransportType::WebRtc,
            established_at: std::time::SystemTime::now(),
            metrics: Default::default(),
        };
        
        let stream = WebRTCStream::new(data_channel);
        
        Ok(Connection::new(
            conn_info,
            Box::new(stream) as Box<dyn Stream>,
        ))
    }
    
    async fn listen(&self, _addr: SocketAddr) -> Result<Box<dyn Listener>> {
        // WebRTC doesn't use traditional socket listening
        // Instead, it uses signaling servers for connection establishment
        Ok(Box::new(WebRTCListener::new(
            self.api.clone(),
            self.ice_servers.clone(),
        )))
    }
    
    fn priority(&self) -> TransportPriority {
        TransportPriority::High // Good for NAT traversal
    }
    
    fn transport_type(&self) -> TransportType {
        TransportType::WebRtc
    }
    
    fn supports_feature(&self, feature: TransportFeature) -> bool {
        matches!(
            feature,
            TransportFeature::NATTraversal
                | TransportFeature::UnreliableChannel
                | TransportFeature::Multistream
                | TransportFeature::BandwidthEstimation
        )
    }
}

/// WebRTC stream implementation.
pub struct WebRTCStream {
    data_channel: Arc<RTCDataChannel>,
    read_rx: mpsc::Receiver<Vec<u8>>,
    write_tx: mpsc::Sender<Vec<u8>>,
}

impl WebRTCStream {
    fn new(data_channel: Arc<RTCDataChannel>) -> Self {
        let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(32);
        let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>(32);
        
        // Set up data channel handlers
        let read_tx_clone = read_tx.clone();
        data_channel.on_message(Box::new(move |msg| {
            let tx = read_tx_clone.clone();
            Box::pin(async move {
                let _ = tx.send(msg.data.to_vec()).await;
            })
        }));
        
        // Handle outgoing messages
        let dc_clone = data_channel.clone();
        tokio::spawn(async move {
            while let Some(data) = write_rx.recv().await {
                let bytes = bytes::Bytes::from(data);
                if let Err(e) = dc_clone.send(&bytes).await {
                    error!("Failed to send WebRTC data: {}", e);
                }
            }
        });
        
        Self {
            data_channel,
            read_rx,
            write_tx,
        }
    }
}

impl AsyncRead for WebRTCStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.read_rx.poll_recv(cx) {
            Poll::Ready(Some(data)) => {
                let len = std::cmp::min(buf.remaining(), data.len());
                buf.put_slice(&data[..len]);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "WebRTC channel closed",
            ))),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for WebRTCStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.write_tx.try_send(buf.to_vec()) {
            Ok(_) => Poll::Ready(Ok(buf.len())),
            Err(mpsc::error::TrySendError::Full(_)) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "WebRTC channel closed",
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

impl Stream for WebRTCStream {
    fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            id: ConnectionId::new(),
            transport: TransportType::WebRtc,
            local_peer: PeerId::new(),
            remote_peer: PeerId::new(),
            established_at: std::time::SystemTime::now(),
            metrics: Default::default(),
        }
    }
    
    fn close(&mut self) -> Result<()> {
        // WebRTC data channels close asynchronously
        // For now, we can't close synchronously
        // The channel will be closed when dropped
        Ok(())
    }
}

/// WebRTC listener implementation.
pub struct WebRTCListener {
    api: Arc<webrtc::api::API>,
    ice_servers: Vec<RTCIceServer>,
    incoming_rx: mpsc::Receiver<(Connection, SocketAddr)>,
}

impl WebRTCListener {
    fn new(api: Arc<webrtc::api::API>, ice_servers: Vec<RTCIceServer>) -> Self {
        let (_tx, rx) = mpsc::channel(32);
        
        // TODO: Set up signaling server listener
        // For now, create a placeholder listener
        
        Self {
            api,
            ice_servers,
            incoming_rx: rx,
        }
    }
}

#[async_trait]
impl Listener for WebRTCListener {
    async fn accept(&mut self) -> Result<(Connection, SocketAddr)> {
        self.incoming_rx
            .recv()
            .await
            .ok_or_else(|| NetworkError::Transport(TransportError::NotAvailable("WebRTC listener closed".to_string())))
    }
    
    fn local_addr(&self) -> Result<SocketAddr> {
        // WebRTC doesn't have traditional socket addresses
        // Return a placeholder
        Ok("0.0.0.0:0".parse().unwrap())
    }
    
    async fn close(&mut self) -> Result<()> {
        // Close the incoming channel
        self.incoming_rx.close();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_webrtc_transport_creation() {
        let transport = WebRTCTransport::new(None).unwrap();
        assert_eq!(transport.priority(), TransportPriority::High);
        assert!(transport.supports_feature(TransportFeature::NATTraversal));
    }
}