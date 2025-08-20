//! WebRTC signaling server and client implementation.

use crate::error::{NetworkError, Result, TransportError};
use crate::types::{PeerId, SignalingMessage};
use futures_util::{SinkExt, StreamExt};
// Removed unused serde imports
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use webrtc::peer_connection::RTCPeerConnection;

/// WebRTC signaling client for peer discovery and connection establishment.
#[derive(Clone)]
pub struct SignalingClient {
    /// Signaling server URL
    server_url: String,
    
    /// Local peer ID
    local_peer_id: PeerId,
    
    /// Message handlers
    handlers: Arc<RwLock<SignalingHandlers>>,
    
    /// Connection to signaling server
    connection: Arc<Mutex<Option<SignalingConnection>>>,
}

/// Signaling message handlers.
struct SignalingHandlers {
    offer_handler: Option<Box<dyn Fn(PeerId, String) + Send + Sync>>,
    answer_handler: Option<Box<dyn Fn(PeerId, String) + Send + Sync>>,
    ice_handler: Option<Box<dyn Fn(PeerId, String) + Send + Sync>>,
}

/// Active signaling connection.
struct SignalingConnection {
    tx: mpsc::Sender<SignalingMessage>,
    task_handle: tokio::task::JoinHandle<()>,
}

impl SignalingClient {
    /// Creates a new signaling client.
    pub fn new(server_url: String, local_peer_id: PeerId) -> Self {
        Self {
            server_url,
            local_peer_id,
            handlers: Arc::new(RwLock::new(SignalingHandlers {
                offer_handler: None,
                answer_handler: None,
                ice_handler: None,
            })),
            connection: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Connects to the signaling server.
    pub async fn connect(&self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.server_url).await
            .map_err(|e| NetworkError::Transport(TransportError::SignalingFailed(e.to_string())))?;
        
        let (tx, mut rx) = mpsc::channel::<SignalingMessage>(32);
        
        // Split WebSocket
        let (mut ws_sink, mut ws_stream_rx) = ws_stream.split();
        
        // Send registration message
        let register_msg = SignalingMessage::Register {
            peer_id: self.local_peer_id.clone(),
        };
        
        let msg_json = serde_json::to_string(&register_msg)
            .map_err(|e| NetworkError::Transport(TransportError::SignalingFailed(e.to_string())))?;
        
        ws_sink.send(Message::Text(msg_json)).await
            .map_err(|e| NetworkError::Transport(TransportError::SignalingFailed(e.to_string())))?;
        
        let handlers = self.handlers.clone();
        let _local_peer_id = self.local_peer_id.clone();
        
        // Spawn message handling task
        let task_handle = tokio::spawn(async move {
            // Handle outgoing messages
            let send_task = tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    let msg_json = match serde_json::to_string(&msg) {
                        Ok(json) => json,
                        Err(e) => {
                            error!("Failed to serialize signaling message: {}", e);
                            continue;
                        }
                    };
                    
                    if let Err(e) = ws_sink.send(Message::Text(msg_json)).await {
                        error!("Failed to send signaling message: {}", e);
                        break;
                    }
                }
            });
            
            // Handle incoming messages
            while let Some(result) = ws_stream_rx.next().await {
                match result {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<SignalingMessage>(&text) {
                            Ok(msg) => {
                                let handlers = handlers.read().await;
                                match msg {
                                    SignalingMessage::Offer { from, to: _, sdp } => {
                                        if let Some(handler) = &handlers.offer_handler {
                                            handler(from, sdp);
                                        }
                                    }
                                    SignalingMessage::Answer { from, to: _, sdp } => {
                                        if let Some(handler) = &handlers.answer_handler {
                                            handler(from, sdp);
                                        }
                                    }
                                    SignalingMessage::IceCandidate { from, to: _, candidate } => {
                                        if let Some(handler) = &handlers.ice_handler {
                                            handler(from, candidate);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse signaling message: {}", e);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("Signaling server closed connection");
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
            
            send_task.abort();
        });
        
        // Store the connection state
        *self.connection.lock().await = Some(SignalingConnection {
            tx,
            task_handle,
        });
        
        info!("Connected to signaling server at {}", self.server_url);
        Ok(())
    }
    
    /// Sends an offer to a peer.
    pub async fn send_offer(&self, to: PeerId, sdp: String) -> Result<()> {
        let msg = SignalingMessage::Offer {
            from: self.local_peer_id.clone(),
            to,
            sdp,
        };
        self.send_message(msg).await
    }
    
    /// Sends an answer to a peer.
    pub async fn send_answer(&self, to: PeerId, sdp: String) -> Result<()> {
        let msg = SignalingMessage::Answer {
            from: self.local_peer_id.clone(),
            to,
            sdp,
        };
        self.send_message(msg).await
    }
    
    /// Sends an ICE candidate to a peer.
    pub async fn send_ice_candidate(&self, to: PeerId, candidate: String) -> Result<()> {
        let msg = SignalingMessage::IceCandidate {
            from: self.local_peer_id.clone(),
            to,
            candidate,
        };
        self.send_message(msg).await
    }
    
    /// Sets the offer handler.
    pub async fn on_offer<F>(&self, handler: F)
    where
        F: Fn(PeerId, String) + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().await;
        handlers.offer_handler = Some(Box::new(handler));
    }
    
    /// Sets the answer handler.
    pub async fn on_answer<F>(&self, handler: F)
    where
        F: Fn(PeerId, String) + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().await;
        handlers.answer_handler = Some(Box::new(handler));
    }
    
    /// Sets the ICE candidate handler.
    pub async fn on_ice_candidate<F>(&self, handler: F)
    where
        F: Fn(PeerId, String) + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().await;
        handlers.ice_handler = Some(Box::new(handler));
    }
    
    /// Sends a message through the signaling connection.
    async fn send_message(&self, msg: SignalingMessage) -> Result<()> {
        let conn = self.connection.lock().await;
        if let Some(conn) = conn.as_ref() {
            conn.tx.send(msg).await
                .map_err(|_| NetworkError::Transport(TransportError::SignalingFailed("Channel closed".to_string())))?;
            Ok(())
        } else {
            Err(NetworkError::Transport(TransportError::NotConnected("Not connected to signaling server".to_string())))
        }
    }
    
    /// Disconnects from the signaling server.
    pub async fn disconnect(&self) -> Result<()> {
        if let Some(conn) = self.connection.lock().await.take() {
            conn.task_handle.abort();
        }
        Ok(())
    }
}

/// WebRTC connection pool for managing peer connections.
pub struct WebRTCConnectionPool {
    /// Active peer connections
    connections: Arc<RwLock<HashMap<PeerId, PooledWebRTCConnection>>>,
    
    /// Maximum connections
    max_connections: usize,
    
    /// Connection timeout
    connection_timeout: Duration,
}

/// Pooled WebRTC connection.
struct PooledWebRTCConnection {
    connection: Arc<RTCPeerConnection>,
    established_at: Instant,
    last_activity: Instant,
}

impl WebRTCConnectionPool {
    /// Creates a new connection pool.
    pub fn new(max_connections: usize, connection_timeout: Duration) -> Self {
        let pool = Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            max_connections,
            connection_timeout,
        };
        
        // Start cleanup task
        let connections = pool.connections.clone();
        let timeout = pool.connection_timeout;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                
                let mut conns = connections.write().await;
                let now = Instant::now();
                
                conns.retain(|peer_id, pooled| {
                    let inactive = now.duration_since(pooled.last_activity) > timeout;
                    if inactive {
                        debug!("Removing inactive WebRTC connection to {}", peer_id);
                    }
                    !inactive
                });
            }
        });
        
        pool
    }
    
    /// Adds a connection to the pool.
    pub async fn add(&self, peer_id: PeerId, connection: Arc<RTCPeerConnection>) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        // Check capacity
        if connections.len() >= self.max_connections {
            // Remove oldest connection
            if let Some((oldest_id, _)) = connections.iter()
                .min_by_key(|(_, pooled)| pooled.established_at)
                .map(|(id, pooled)| (id.clone(), pooled.established_at))
            {
                debug!("Evicting oldest connection to {} to make room", oldest_id);
                connections.remove(&oldest_id);
            }
        }
        
        let now = Instant::now();
        connections.insert(peer_id, PooledWebRTCConnection {
            connection,
            established_at: now,
            last_activity: now,
        });
        
        Ok(())
    }
    
    /// Gets a connection from the pool.
    pub async fn get(&self, peer_id: &PeerId) -> Option<Arc<RTCPeerConnection>> {
        let mut connections = self.connections.write().await;
        if let Some(pooled) = connections.get_mut(peer_id) {
            pooled.last_activity = Instant::now();
            Some(pooled.connection.clone())
        } else {
            None
        }
    }
    
    /// Removes a connection from the pool.
    pub async fn remove(&self, peer_id: &PeerId) -> Option<Arc<RTCPeerConnection>> {
        let mut connections = self.connections.write().await;
        connections.remove(peer_id).map(|pooled| pooled.connection)
    }
    
    /// Gets the number of active connections.
    pub async fn len(&self) -> usize {
        self.connections.read().await.len()
    }
}

/// Simple signaling server for development and testing.
pub struct SignalingServer {
    /// Listening address
    addr: SocketAddr,
    
    /// Connected peers
    peers: Arc<RwLock<HashMap<PeerId, mpsc::Sender<SignalingMessage>>>>,
}

impl SignalingServer {
    /// Creates a new signaling server.
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Starts the signaling server.
    pub async fn start(self) -> Result<()> {
        let listener = TcpListener::bind(self.addr).await
            .map_err(|e| NetworkError::Transport(TransportError::SignalingFailed(e.to_string())))?;
        
        info!("Signaling server listening on {}", self.addr);
        
        loop {
            let (stream, addr) = listener.accept().await
                .map_err(|e| NetworkError::Transport(TransportError::SignalingFailed(e.to_string())))?;
            
            let peers = self.peers.clone();
            
            tokio::spawn(async move {
                if let Err(e) = handle_peer_connection(stream, addr, peers).await {
                    error!("Error handling peer connection from {}: {}", addr, e);
                }
            });
        }
    }
}

/// Handles a peer connection to the signaling server.
async fn handle_peer_connection(
    stream: TcpStream,
    addr: SocketAddr,
    peers: Arc<RwLock<HashMap<PeerId, mpsc::Sender<SignalingMessage>>>>,
) -> Result<()> {
    let ws_stream = accept_async(stream).await
        .map_err(|e| NetworkError::Transport(TransportError::SignalingFailed(e.to_string())))?;
    
    let (mut ws_sink, mut ws_stream_rx) = ws_stream.split();
    let (tx, mut rx) = mpsc::channel::<SignalingMessage>(32);
    
    let mut peer_id: Option<PeerId> = None;
    
    // Handle outgoing messages
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let msg_json = match serde_json::to_string(&msg) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                    continue;
                }
            };
            
            if let Err(e) = ws_sink.send(Message::Text(msg_json)).await {
                error!("Failed to send message: {}", e);
                break;
            }
        }
    });
    
    // Handle incoming messages
    while let Some(result) = ws_stream_rx.next().await {
        match result {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<SignalingMessage>(&text) {
                    Ok(msg) => {
                        match msg {
                            SignalingMessage::Register { peer_id: pid } => {
                                info!("Peer {} registered from {}", pid, addr);
                                peer_id = Some(pid.clone());
                                peers.write().await.insert(pid, tx.clone());
                            }
                            SignalingMessage::Offer { to, .. } |
                            SignalingMessage::Answer { to, .. } |
                            SignalingMessage::IceCandidate { to, .. } => {
                                // Forward message to target peer
                                let peers_read = peers.read().await;
                                if let Some(target_tx) = peers_read.get(&to) {
                                    let _ = target_tx.send(msg).await;
                                } else {
                                    warn!("Target peer {} not found", to);
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse message: {}", e);
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("Peer from {} disconnected", addr);
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
    
    // Clean up
    send_task.abort();
    if let Some(pid) = peer_id {
        peers.write().await.remove(&pid);
        info!("Removed peer {} from registry", pid);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_signaling_client_creation() {
        let client = SignalingClient::new(
            "ws://localhost:8080".to_string(),
            PeerId::new(),
        );
        
        // Set handlers
        client.on_offer(|from, sdp| {
            println!("Received offer from {:?}: {}", from, sdp);
        }).await;
        
        client.on_answer(|from, sdp| {
            println!("Received answer from {:?}: {}", from, sdp);
        }).await;
        
        client.on_ice_candidate(|from, candidate| {
            println!("Received ICE candidate from {:?}: {}", from, candidate);
        }).await;
    }
    
    #[tokio::test]
    async fn test_connection_pool() {
        let pool = WebRTCConnectionPool::new(10, Duration::from_secs(300));
        assert_eq!(pool.len().await, 0);
        
        // Pool operations would require actual WebRTC connections
        // This is just testing the structure
    }
}