//! WebSocket transport implementation for browser compatibility.

use crate::error::{NetworkError, Result, TransportError};
use crate::transport::traits::{Listener, Stream, Transport, TransportFeature, TransportPriority};
use crate::transport::Connection;
use crate::types::{ConnectionId, ConnectionInfo, PeerInfo, PeerId, TransportType};
use async_trait::async_trait;
use futures_util::{Sink, stream::Stream as FuturesStream};
use std::net::SocketAddr;
use std::pin::Pin;
use pin_project_lite::pin_project;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;
use tokio_tungstenite::{
    accept_async, connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info};

/// WebSocket transport for browser-compatible connections.
pub struct WebSocketTransport {
    /// WebSocket configuration
    config: WebSocketConfig,
    
    /// Active connections
    connections: Arc<Mutex<Vec<WebSocketConnection>>>,
}

/// WebSocket configuration.
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// Connection timeout
    pub connection_timeout: Duration,
    
    /// Maximum frame size
    pub max_frame_size: usize,
    
    /// Enable compression
    pub enable_compression: bool,
    
    /// Subprotocols
    pub subprotocols: Vec<String>,
    
    /// Custom headers
    pub headers: Vec<(String, String)>,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            connection_timeout: Duration::from_secs(10),
            max_frame_size: 64 * 1024, // 64KB
            enable_compression: true,
            subprotocols: vec!["synapsed".to_string()],
            headers: vec![],
        }
    }
}

impl WebSocketTransport {
    /// Creates a new WebSocket transport.
    pub fn new(config: WebSocketConfig) -> Self {
        Self {
            config,
            connections: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Creates a WebSocket transport with TLS support.
    pub fn with_tls(config: WebSocketConfig, _tls_config: rustls::ClientConfig) -> Self {
        // TODO: Store TLS config for secure connections
        Self::new(config)
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn connect(&self, peer: &PeerInfo) -> Result<Connection> {
        info!("Connecting to peer {} via WebSocket", peer.id);
        
        // Parse URL - support both ws:// and wss://
        let url = if peer.address.starts_with("ws://") || peer.address.starts_with("wss://") {
            peer.address.clone()
        } else {
            format!("ws://{}", peer.address)
        };
        
        // Create request with custom headers using builder
        let mut request = tokio_tungstenite::tungstenite::handshake::client::Request::builder()
            .uri(&url)
            .header("User-Agent", "synapsed-net/0.1.0")
            .body(())
            .map_err(|e| NetworkError::Transport(TransportError::WebSocket(e.to_string())))?;
        
        // Add custom headers
        for (key, value) in &self.config.headers {
            use tokio_tungstenite::tungstenite::http::HeaderName;
            request.headers_mut().insert(
                key.parse::<HeaderName>().map_err(|e| 
                    NetworkError::Transport(TransportError::WebSocket(e.to_string())))?,
                value.parse::<tokio_tungstenite::tungstenite::http::HeaderValue>().map_err(|e| 
                    NetworkError::Transport(TransportError::WebSocket(e.to_string())))?,
            );
        }
        
        // Add subprotocols
        if !self.config.subprotocols.is_empty() {
            request.headers_mut().insert(
                "Sec-WebSocket-Protocol",
                self.config.subprotocols.join(", ").parse()
                    .map_err(|e: tokio_tungstenite::tungstenite::http::header::InvalidHeaderValue| 
                        NetworkError::Transport(TransportError::WebSocket(e.to_string())))?,
            );
        }
        
        // Connect with timeout
        let (ws_stream, _) = timeout(
            self.config.connection_timeout,
            connect_async(request),
        )
        .await
        .map_err(|_| NetworkError::Transport(TransportError::TimeoutWithMsg("WebSocket connection timeout".to_string())))?
        .map_err(|e| NetworkError::Transport(TransportError::WebSocket(e.to_string())))?;
        
        let conn_info = ConnectionInfo {
            id: ConnectionId::new(),
            local_peer: PeerId::new(),
            remote_peer: peer.id.clone(),
            transport: TransportType::WebSocket,
            established_at: std::time::SystemTime::now(),
            metrics: Default::default(),
        };
        
        let stream = WebSocketStreamWrapper::new(ws_stream);
        
        // Store connection
        {
            let mut conns = self.connections.lock().await;
            conns.push(WebSocketConnection {
                peer_id: peer.id.clone(),
                established_at: std::time::SystemTime::now(),
            });
        }
        
        Ok(Connection::new(
            conn_info,
            Box::new(stream) as Box<dyn Stream>,
        ))
    }
    
    async fn listen(&self, addr: SocketAddr) -> Result<Box<dyn Listener>> {
        let listener = TcpListener::bind(addr).await
            .map_err(|e| NetworkError::Transport(TransportError::WebSocket(e.to_string())))?;
        
        let local_addr = listener.local_addr()
            .map_err(|e| NetworkError::Transport(TransportError::WebSocket(e.to_string())))?;
        
        info!("WebSocket transport listening on {}", local_addr);
        
        Ok(Box::new(WebSocketListener::new(
            listener,
            self.config.clone(),
        )))
    }
    
    fn priority(&self) -> TransportPriority {
        TransportPriority::Medium // Good for browser compatibility
    }
    
    fn transport_type(&self) -> TransportType {
        TransportType::WebSocket
    }
    
    fn supports_feature(&self, feature: TransportFeature) -> bool {
        matches!(
            feature,
            TransportFeature::Multistream | TransportFeature::BandwidthEstimation
        )
    }
}

/// Active WebSocket connection metadata.
struct WebSocketConnection {
    peer_id: PeerId,
    established_at: std::time::SystemTime,
}

// WebSocket stream wrapper that can handle both TLS and non-TLS streams.
pin_project! {
    #[project = WebSocketStreamWrapperProj]
    pub enum WebSocketStreamWrapper {
        Tls {
            #[pin]
            ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
            read_buffer: Vec<u8>,
            write_buffer: Vec<u8>,
        },
        Plain {
            #[pin]
            ws: WebSocketStream<TcpStream>,
            read_buffer: Vec<u8>,
            write_buffer: Vec<u8>,
        },
    }
}

impl WebSocketStreamWrapper {
    fn new_tls(ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        Self::Tls {
            ws,
            read_buffer: Vec::new(),
            write_buffer: Vec::new(),
        }
    }
    
    fn new_plain(ws: WebSocketStream<TcpStream>) -> Self {
        Self::Plain {
            ws,
            read_buffer: Vec::new(),
            write_buffer: Vec::new(),
        }
    }
    
    // For backward compatibility
    fn new(ws: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        Self::new_tls(ws)
    }
}

impl AsyncRead for WebSocketStreamWrapper {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.project() {
            WebSocketStreamWrapperProj::Tls { mut ws, read_buffer, .. } => {
                // If we have buffered data, return it first
                if !read_buffer.is_empty() {
                    let len = std::cmp::min(buf.remaining(), read_buffer.len());
                    buf.put_slice(&read_buffer[..len]);
                    read_buffer.drain(..len);
                    return Poll::Ready(Ok(()));
                }
                
                // Try to read a new message
                match ws.as_mut().poll_next(cx) {
                    Poll::Ready(Some(Ok(Message::Binary(data)))) => {
                        let len = std::cmp::min(buf.remaining(), data.len());
                        buf.put_slice(&data[..len]);
                        
                        // Buffer remaining data
                        if len < data.len() {
                            read_buffer.extend_from_slice(&data[len..]);
                        }
                        
                        Poll::Ready(Ok(()))
                    }
                    Poll::Ready(Some(Ok(Message::Text(text)))) => {
                        let data = text.as_bytes();
                        let len = std::cmp::min(buf.remaining(), data.len());
                        buf.put_slice(&data[..len]);
                        
                        // Buffer remaining data
                        if len < data.len() {
                            read_buffer.extend_from_slice(&data[len..]);
                        }
                        
                        Poll::Ready(Ok(()))
                    }
                    Poll::Ready(Some(Ok(Message::Close(_)))) => {
                        Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "WebSocket closed",
                        )))
                    }
                    Poll::Ready(Some(Ok(_))) => {
                        // Ignore ping/pong messages
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                    Poll::Ready(Some(Err(e))) => {
                        Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        )))
                    }
                    Poll::Ready(None) => {
                        Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "WebSocket stream ended",
                        )))
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
            WebSocketStreamWrapperProj::Plain { mut ws, read_buffer, .. } => {
                // If we have buffered data, return it first
                if !read_buffer.is_empty() {
                    let len = std::cmp::min(buf.remaining(), read_buffer.len());
                    buf.put_slice(&read_buffer[..len]);
                    read_buffer.drain(..len);
                    return Poll::Ready(Ok(()));
                }
                
                // Try to read a new message
                match ws.as_mut().poll_next(cx) {
                    Poll::Ready(Some(Ok(Message::Binary(data)))) => {
                        let len = std::cmp::min(buf.remaining(), data.len());
                        buf.put_slice(&data[..len]);
                        
                        // Buffer remaining data
                        if len < data.len() {
                            read_buffer.extend_from_slice(&data[len..]);
                        }
                        
                        Poll::Ready(Ok(()))
                    }
                    Poll::Ready(Some(Ok(Message::Text(text)))) => {
                        let data = text.as_bytes();
                        let len = std::cmp::min(buf.remaining(), data.len());
                        buf.put_slice(&data[..len]);
                        
                        // Buffer remaining data
                        if len < data.len() {
                            read_buffer.extend_from_slice(&data[len..]);
                        }
                        
                        Poll::Ready(Ok(()))
                    }
                    Poll::Ready(Some(Ok(Message::Close(_)))) => {
                        Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "WebSocket closed",
                        )))
                    }
                    Poll::Ready(Some(Ok(_))) => {
                        // Ignore ping/pong messages
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                    Poll::Ready(Some(Err(e))) => {
                        Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e,
                        )))
                    }
                    Poll::Ready(None) => {
                        Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "WebSocket stream ended",
                        )))
                    }
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}

impl AsyncWrite for WebSocketStreamWrapper {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.project() {
            WebSocketStreamWrapperProj::Tls { mut ws, write_buffer, .. } => {
                // Buffer the data
                write_buffer.extend_from_slice(buf);
                
                // Try to send if we have enough data or on explicit flush
                if write_buffer.len() >= 1024 {
                    let data = std::mem::take(write_buffer);
                    let message = Message::Binary(data.clone());
                    
                    match Pin::new(&mut ws).poll_ready(cx) {
                        Poll::Ready(Ok(())) => {
                            match Pin::new(&mut ws).start_send(message) {
                                Ok(()) => {
                                    // Also flush after send
                                    match Pin::new(&mut ws).poll_flush(cx) {
                                        Poll::Ready(Ok(())) => Poll::Ready(Ok(buf.len())),
                                        Poll::Ready(Err(e)) => Poll::Ready(Err(std::io::Error::new(
                                            std::io::ErrorKind::Other,
                                            e,
                                        ))),
                                        Poll::Pending => Poll::Pending,
                                    }
                                }
                                Err(e) => Poll::Ready(Err(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    e,
                                ))),
                            }
                        }
                        Poll::Ready(Err(e)) => {
                            Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e,
                            )))
                        }
                        Poll::Pending => {
                            // Restore buffer
                            *write_buffer = data;
                            Poll::Pending
                        }
                    }
                } else {
                    Poll::Ready(Ok(buf.len()))
                }
            }
            WebSocketStreamWrapperProj::Plain { mut ws, write_buffer, .. } => {
                // Buffer the data
                write_buffer.extend_from_slice(buf);
                
                // Try to send if we have enough data or on explicit flush
                if write_buffer.len() >= 1024 {
                    let data = std::mem::take(write_buffer);
                    let message = Message::Binary(data.clone());
                    
                    match Pin::new(&mut ws).poll_ready(cx) {
                        Poll::Ready(Ok(())) => {
                            match Pin::new(&mut ws).start_send(message) {
                                Ok(()) => {
                                    // Also flush after send
                                    match Pin::new(&mut ws).poll_flush(cx) {
                                        Poll::Ready(Ok(())) => Poll::Ready(Ok(buf.len())),
                                        Poll::Ready(Err(e)) => Poll::Ready(Err(std::io::Error::new(
                                            std::io::ErrorKind::Other,
                                            e,
                                        ))),
                                        Poll::Pending => Poll::Pending,
                                    }
                                }
                                Err(e) => Poll::Ready(Err(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    e,
                                ))),
                            }
                        }
                        Poll::Ready(Err(e)) => {
                            Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e,
                            )))
                        }
                        Poll::Pending => {
                            // Restore buffer
                            *write_buffer = data;
                            Poll::Pending
                        }
                    }
                } else {
                    Poll::Ready(Ok(buf.len()))
                }
            }
        }
    }
    
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.project() {
            WebSocketStreamWrapperProj::Tls { mut ws, write_buffer, .. } => {
                // Send any buffered data
                if !write_buffer.is_empty() {
                    let data = std::mem::take(write_buffer);
                    let message = Message::Binary(data.clone());
                    
                    match Pin::new(&mut ws).poll_ready(cx) {
                        Poll::Ready(Ok(())) => {
                            if let Err(e) = Pin::new(&mut ws).start_send(message) {
                                return Poll::Ready(Err(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    e,
                                )));
                            }
                        }
                        Poll::Ready(Err(e)) => {
                            return Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e,
                            )));
                        }
                        Poll::Pending => {
                            // Restore buffer
                            *write_buffer = data;
                            return Poll::Pending;
                        }
                    }
                }
                
                // Flush WebSocket
                match Pin::new(&mut ws).poll_flush(cx) {
                    Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                    Poll::Ready(Err(e)) => Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e,
                    ))),
                    Poll::Pending => Poll::Pending,
                }
            }
            WebSocketStreamWrapperProj::Plain { mut ws, write_buffer, .. } => {
                // Send any buffered data
                if !write_buffer.is_empty() {
                    let data = std::mem::take(write_buffer);
                    let message = Message::Binary(data.clone());
                    
                    match Pin::new(&mut ws).poll_ready(cx) {
                        Poll::Ready(Ok(())) => {
                            if let Err(e) = Pin::new(&mut ws).start_send(message) {
                                return Poll::Ready(Err(std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    e,
                                )));
                            }
                        }
                        Poll::Ready(Err(e)) => {
                            return Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e,
                            )));
                        }
                        Poll::Pending => {
                            // Restore buffer
                            *write_buffer = data;
                            return Poll::Pending;
                        }
                    }
                }
                
                // Flush WebSocket
                match Pin::new(&mut ws).poll_flush(cx) {
                    Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                    Poll::Ready(Err(e)) => Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e,
                    ))),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
    
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.project() {
            WebSocketStreamWrapperProj::Tls { mut ws, .. } => {
                match Pin::new(&mut ws).poll_close(cx) {
                    Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                    Poll::Ready(Err(e)) => Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e,
                    ))),
                    Poll::Pending => Poll::Pending,
                }
            }
            WebSocketStreamWrapperProj::Plain { mut ws, .. } => {
                match Pin::new(&mut ws).poll_close(cx) {
                    Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                    Poll::Ready(Err(e)) => Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e,
                    ))),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}

impl Stream for WebSocketStreamWrapper {
    fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            id: ConnectionId::new(),
            transport: TransportType::WebSocket,
            local_peer: PeerId::new(),
            remote_peer: PeerId::new(),
            established_at: std::time::SystemTime::now(),
            metrics: Default::default(),
        }
    }
    
    fn close(&mut self) -> Result<()> {
        // WebSocket will close on drop
        Ok(())
    }
}

impl Sink<Message> for WebSocketStreamWrapper {
    type Error = tokio_tungstenite::tungstenite::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        match self.project() {
            WebSocketStreamWrapperProj::Tls { mut ws, .. } => ws.as_mut().poll_ready(cx),
            WebSocketStreamWrapperProj::Plain { mut ws, .. } => ws.as_mut().poll_ready(cx),
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Message) -> std::result::Result<(), Self::Error> {
        match self.project() {
            WebSocketStreamWrapperProj::Tls { mut ws, .. } => ws.as_mut().start_send(item),
            WebSocketStreamWrapperProj::Plain { mut ws, .. } => ws.as_mut().start_send(item),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        match self.project() {
            WebSocketStreamWrapperProj::Tls { mut ws, .. } => ws.as_mut().poll_flush(cx),
            WebSocketStreamWrapperProj::Plain { mut ws, .. } => ws.as_mut().poll_flush(cx),
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        match self.project() {
            WebSocketStreamWrapperProj::Tls { mut ws, .. } => ws.as_mut().poll_close(cx),
            WebSocketStreamWrapperProj::Plain { mut ws, .. } => ws.as_mut().poll_close(cx),
        }
    }
}

/// WebSocket listener.
pub struct WebSocketListener {
    local_addr: Option<SocketAddr>,
    config: WebSocketConfig,
    accept_rx: mpsc::Receiver<(Connection, SocketAddr)>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl WebSocketListener {
    fn new(tcp_listener: TcpListener, config: WebSocketConfig) -> Self {
        let (tx, rx) = mpsc::channel(32);
        let config_clone = config.clone();
        
        // Store the local address before moving the listener
        let local_addr = tcp_listener.local_addr().ok();
        
        // Spawn accept task with the tcp_listener moved in
        let task_handle = tokio::spawn(async move {
            loop {
                match tcp_listener.accept().await {
                    Ok((stream, addr)) => {
                        debug!("Accepted TCP connection for WebSocket from {}", addr);
                        
                        let tx = tx.clone();
                        let _config = config_clone.clone();
                        
                        // Handle WebSocket handshake in separate task
                        tokio::spawn(async move {
                            match accept_async(stream).await {
                                Ok(ws_stream) => {
                                    let conn_info = ConnectionInfo {
                                        id: ConnectionId::new(),
                                        local_peer: PeerId::new(),
                                        remote_peer: PeerId::new(),
                                        transport: TransportType::WebSocket,
                                        established_at: std::time::SystemTime::now(),
                                        metrics: Default::default(),
                                    };
                                    
                                    // For WebSocket over TLS, we need to properly handle stream types
                                    // Use the websocket stream directly - tokio-tungstenite handles the typing
                                    
                                    let stream = WebSocketStreamWrapper::new_plain(ws_stream);
                                    let connection = Connection::new(
                                        conn_info,
                                        Box::new(stream) as Box<dyn Stream>,
                                    );
                                    
                                    let _ = tx.send((connection, addr)).await;
                                }
                                Err(e) => {
                                    error!("WebSocket handshake failed: {}", e);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!("TCP accept error for WebSocket: {}", e);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        });
        
        Self {
            local_addr,
            config,
            accept_rx: rx,
            task_handle: Some(task_handle),
        }
    }
}

#[async_trait]
impl Listener for WebSocketListener {
    async fn accept(&mut self) -> Result<(Connection, SocketAddr)> {
        self.accept_rx.recv().await
            .ok_or_else(|| NetworkError::Transport(TransportError::NotAvailable("WebSocket listener closed".to_string())))
    }
    
    fn local_addr(&self) -> Result<SocketAddr> {
        self.local_addr
            .ok_or_else(|| NetworkError::Transport(TransportError::WebSocket("Local address not available".to_string())))
    }
    
    async fn close(&mut self) -> Result<()> {
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
    async fn test_websocket_transport_creation() {
        let config = WebSocketConfig::default();
        let transport = WebSocketTransport::new(config);
        assert_eq!(transport.priority(), TransportPriority::Medium);
        assert!(transport.supports_feature(TransportFeature::Multistream));
    }
    
    #[tokio::test]
    async fn test_websocket_config() {
        let config = WebSocketConfig {
            connection_timeout: Duration::from_secs(5),
            max_frame_size: 1024,
            enable_compression: false,
            subprotocols: vec!["test".to_string()],
            headers: vec![("X-Test".to_string(), "value".to_string())],
        };
        
        assert_eq!(config.connection_timeout, Duration::from_secs(5));
        assert_eq!(config.max_frame_size, 1024);
        assert!(!config.enable_compression);
        assert_eq!(config.subprotocols, vec!["test"]);
        assert_eq!(config.headers.len(), 1);
    }
}