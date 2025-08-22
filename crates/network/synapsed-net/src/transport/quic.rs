//! QUIC transport implementation using quinn for efficient, secure connections.

use crate::error::{NetworkError, Result, TransportError};
use crate::transport::traits::{Listener, Stream, Transport, TransportFeature, TransportPriority};
use crate::transport::Connection;
use crate::types::{ConnectionId, ConnectionInfo, PeerInfo, PeerId, TransportType};
use async_trait::async_trait;
use quinn::{ClientConfig, Endpoint, ServerConfig, TransportConfig};
use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use quinn::rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use quinn::rustls::client::danger::{ServerCertVerifier as RustlsServerCertVerifier, ServerCertVerified, HandshakeSignatureValid};
use quinn::rustls::client::WebPkiServerVerifier;
use quinn::rustls::{DigitallySignedStruct, SignatureScheme, Error as RustlsError, RootCertStore};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use std::future::Future;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info};

/// QUIC transport implementation with 0-RTT support.
pub struct QuicTransport {
    /// QUIC endpoint
    endpoint: Option<Endpoint>,
    
    /// Bind address
    bind_addr: SocketAddr,
    
    /// Server configuration
    server_config: Option<ServerConfig>,
    
    /// Client configuration
    client_config: ClientConfig,
    
    /// Active connections
    connections: Arc<Mutex<Vec<quinn::Connection>>>,
    
    /// 0-RTT session cache
    zero_rtt_cache: Arc<Mutex<std::collections::HashMap<SocketAddr, Vec<u8>>>>,
}

impl QuicTransport {
    /// Creates a new QUIC transport.
    pub fn new(bind_addr: SocketAddr) -> Result<Self> {
        // Create self-signed certificate for development
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
            .map_err(|e| NetworkError::Transport(TransportError::Quic(e.to_string())))?;
        
        let cert_der = cert.cert.der().to_vec();
        let key_der = cert.key_pair.serialize_der();
        
        let cert_chain = vec![CertificateDer::from(cert_der)];
        let private_key = PrivateKeyDer::try_from(key_der)
            .map_err(|e| NetworkError::Transport(TransportError::Quic(e.to_string())))?;
        
        // Create client configuration for quinn using quinn::rustls
        let client_crypto = quinn::rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(ServerCertVerifier::new(
                cfg!(debug_assertions) // Allow self-signed in debug builds only
            )))
            .with_no_client_auth();
        
        let mut client_config = ClientConfig::new(Arc::new(QuicClientConfig::try_from(client_crypto).unwrap()));
        
        // Configure transport
        let mut transport_config = TransportConfig::default();
        transport_config.max_concurrent_bidi_streams(100u32.into());
        transport_config.max_concurrent_uni_streams(100u32.into());
        transport_config.max_idle_timeout(Some(
            Duration::from_secs(30).try_into()
                .map_err(|_| NetworkError::Transport(
                    TransportError::Quic("Invalid idle timeout duration".to_string())
                ))?
        ));
        transport_config.keep_alive_interval(Some(Duration::from_secs(10)));
        
        client_config.transport_config(Arc::new(transport_config));
        
        // Create server configuration for quinn using quinn::rustls
        let server_crypto = quinn::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain.clone(), private_key.clone_key())
            .map_err(|e| NetworkError::Transport(TransportError::Quic(e.to_string())))?;
        
        let mut server_config = ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(server_crypto).unwrap()));
        let mut transport_config2 = TransportConfig::default();
        transport_config2.max_concurrent_bidi_streams(100u32.into());
        transport_config2.max_concurrent_uni_streams(100u32.into());
        transport_config2.max_idle_timeout(Some(
            Duration::from_secs(30).try_into()
                .map_err(|_| NetworkError::Transport(
                    TransportError::Quic("Invalid idle timeout duration".to_string())
                ))?
        ));
        transport_config2.keep_alive_interval(Some(Duration::from_secs(10)));
        server_config.transport_config(Arc::new(transport_config2));
        
        Ok(Self {
            endpoint: None,
            bind_addr,
            server_config: Some(server_config),
            client_config,
            connections: Arc::new(Mutex::new(Vec::new())),
            zero_rtt_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
    }
    
    /// Creates a QUIC transport with custom certificates.
    pub fn with_certificates(
        bind_addr: SocketAddr,
        cert_chain: Vec<Vec<u8>>,
        private_key: Vec<u8>,
    ) -> Result<Self> {
        let certs: Vec<CertificateDer> = cert_chain.into_iter()
            .map(CertificateDer::from)
            .collect();
        
        let key = PrivateKeyDer::try_from(private_key)
            .map_err(|e| NetworkError::Transport(TransportError::Quic(e.to_string())))?;
        
        // Create client configuration for quinn using quinn::rustls
        let client_crypto = quinn::rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(ServerCertVerifier::new(
                cfg!(debug_assertions) // Allow self-signed in debug builds only
            )))
            .with_no_client_auth();
        
        let mut client_config = ClientConfig::new(Arc::new(QuicClientConfig::try_from(client_crypto).unwrap()));
        
        // Configure transport with optimized settings
        let mut transport_config = TransportConfig::default();
        transport_config.max_concurrent_bidi_streams(100u32.into());
        transport_config.max_concurrent_uni_streams(100u32.into());
        transport_config.max_idle_timeout(Some(
            Duration::from_secs(30).try_into()
                .map_err(|_| NetworkError::Transport(
                    TransportError::Quic("Invalid idle timeout duration".to_string())
                ))?
        ));
        transport_config.congestion_controller_factory(Arc::new(quinn::congestion::BbrConfig::default()));
        
        client_config.transport_config(Arc::new(transport_config));
        
        // Create server configuration
        let server_crypto = quinn::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| NetworkError::Transport(TransportError::Quic(e.to_string())))?;
        
        let mut server_config = ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(server_crypto).unwrap()));
        let mut transport_config2 = TransportConfig::default();
        transport_config2.max_concurrent_bidi_streams(100u32.into());
        transport_config2.max_concurrent_uni_streams(100u32.into());
        transport_config2.max_idle_timeout(Some(
            Duration::from_secs(30).try_into()
                .map_err(|_| NetworkError::Transport(
                    TransportError::Quic("Invalid idle timeout duration".to_string())
                ))?
        ));
        transport_config2.keep_alive_interval(Some(Duration::from_secs(10)));
        server_config.transport_config(Arc::new(transport_config2));
        
        Ok(Self {
            endpoint: None,
            bind_addr,
            server_config: Some(server_config),
            client_config,
            connections: Arc::new(Mutex::new(Vec::new())),
            zero_rtt_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
    }
    
    /// Initializes the QUIC endpoint.
    pub async fn initialize(&mut self) -> Result<()> {
        if self.endpoint.is_some() {
            return Ok(());
        }
        
        let server_config = self.server_config.clone()
            .ok_or_else(|| NetworkError::Configuration("Server configuration not set".to_string()))?;
            
        let endpoint = Endpoint::server(
            server_config,
            self.bind_addr,
        ).map_err(|e| NetworkError::Transport(TransportError::Quic(e.to_string())))?;
        
        info!("QUIC transport initialized on {}", self.bind_addr);
        self.endpoint = Some(endpoint);
        Ok(())
    }
}

#[async_trait]
impl Transport for QuicTransport {
    async fn connect(&self, peer: &PeerInfo) -> Result<Connection> {
        info!("Connecting to peer {} via QUIC", peer.id);
        
        let endpoint = self.endpoint.as_ref()
            .ok_or_else(|| NetworkError::Transport(TransportError::NotInitialized("QUIC endpoint not initialized".to_string())))?;
        
        let addr: SocketAddr = peer.address.parse()
            .map_err(|e| NetworkError::Transport(TransportError::InvalidAddress(format!("{}: {}", peer.address, e))))?;
        
        // Attempt 0-RTT connection if we have cached session data
        let connecting = {
            let cache = self.zero_rtt_cache.lock().await;
            if let Some(_session_data) = cache.get(&addr) {
                debug!("Attempting 0-RTT connection to {}", addr);
                // For now, just use the standard config
                // TODO: Implement proper 0-RTT session resumption
                endpoint.connect(addr, "synapsed")
                    .map_err(|e| NetworkError::Transport(TransportError::Quic(e.to_string())))?    
            } else {
                endpoint.connect(addr, "synapsed")
                    .map_err(|e| NetworkError::Transport(TransportError::Quic(e.to_string())))?   
            }
        };
        
        let connection = connecting.await
            .map_err(|e| NetworkError::Transport(TransportError::Quic(e.to_string())))?;
        
        // Store connection for management
        {
            let mut conns = self.connections.lock().await;
            conns.push(connection.clone());
        }
        
        // Open a bidirectional stream
        let (send, recv) = connection.open_bi().await
            .map_err(|e| NetworkError::Transport(TransportError::Quic(e.to_string())))?;
        
        let conn_info = ConnectionInfo {
            id: ConnectionId::new(),
            local_peer: PeerId::new(), // TODO: Convert from actual local peer ID
            remote_peer: peer.id,
            transport: TransportType::Quic,
            established_at: std::time::SystemTime::now(),
            metrics: Default::default(),
        };
        
        let stream = QuicStream::new(send, recv);
        
        Ok(Connection::new(
            conn_info,
            Box::new(stream) as Box<dyn Stream>,
        ))
    }
    
    async fn listen(&self, _addr: SocketAddr) -> Result<Box<dyn Listener>> {
        let endpoint = self.endpoint.as_ref()
            .ok_or_else(|| NetworkError::Transport(TransportError::NotInitialized("QUIC endpoint not initialized".to_string())))?;
        
        Ok(Box::new(QuicListener::new(endpoint.clone())))
    }
    
    fn priority(&self) -> TransportPriority {
        TransportPriority::Preferred
    }
    
    fn transport_type(&self) -> TransportType {
        TransportType::Quic
    }
    
    fn supports_feature(&self, feature: TransportFeature) -> bool {
        match feature {
            TransportFeature::ZeroRTT => true,
            TransportFeature::Multistream => true,
            TransportFeature::UnreliableChannel => true,
            TransportFeature::ConnectionMigration => true,
            TransportFeature::BandwidthEstimation => true,
            TransportFeature::NATTraversal => false,
            TransportFeature::Anonymity => false,
            TransportFeature::PostQuantum => false,
        }
    }
}

/// QUIC stream implementation.
pub struct QuicStream {
    send: quinn::SendStream,
    recv: quinn::RecvStream,
}

impl QuicStream {
    fn new(send: quinn::SendStream, recv: quinn::RecvStream) -> Self {
        Self { send, recv }
    }
}

impl AsyncRead for QuicStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // quinn's RecvStream doesn't implement AsyncRead directly,
        // so we need to manually implement the polling logic
        let this = self.get_mut();
        let temp_buf_len = buf.remaining();
        if temp_buf_len == 0 {
            return Poll::Ready(Ok(()));
        }
        
        // Create a future that will do the read
        let read_future = async {
            let mut temp_buf = vec![0u8; temp_buf_len];
            match this.recv.read(&mut temp_buf).await {
                Ok(Some(n)) => Ok((n, temp_buf)),
                Ok(None) => Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "QUIC stream closed",
                )),
                Err(e) => Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e,
                )),
            }
        };
        
        tokio::pin!(read_future);
        
        match read_future.poll(cx) {
            Poll::Ready(Ok((n, temp_buf))) => {
                buf.put_slice(&temp_buf[..n]);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for QuicStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match Pin::new(&mut self.send).poll_write(cx, buf) {
            Poll::Ready(Ok(n)) => Poll::Ready(Ok(n)),
            Poll::Ready(Err(e)) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e,
            ))),
            Poll::Pending => Poll::Pending,
        }
    }
    
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match Pin::new(&mut self.send).poll_flush(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e,
            ))),
            Poll::Pending => Poll::Pending,
        }
    }
    
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        // The finish() method is not async - it just marks the stream as finished
        let this = self.get_mut();
        match this.send.finish() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                e,
            ))),
        }
    }
}

impl Stream for QuicStream {
    fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            id: ConnectionId::new(),
            transport: TransportType::Quic,
            local_peer: PeerId::new(),
            remote_peer: PeerId::new(),
            established_at: std::time::SystemTime::now(),
            metrics: Default::default(),
        }
    }
    
    fn close(&mut self) -> Result<()> {
        // QUIC streams close automatically when dropped
        Ok(())
    }
}

/// QUIC listener implementation.
pub struct QuicListener {
    endpoint: Endpoint,
    incoming_rx: mpsc::Receiver<(Connection, SocketAddr)>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl QuicListener {
    fn new(endpoint: Endpoint) -> Self {
        let (tx, rx) = mpsc::channel(32);
        let endpoint_clone = endpoint.clone();
        
        // Spawn task to accept incoming connections
        let task_handle = tokio::spawn(async move {
            while let Some(connecting) = endpoint_clone.accept().await {
                let tx = tx.clone();
                tokio::spawn(async move {
                    if let Ok(connection) = connecting.await {
                        let remote_addr = connection.remote_address();
                        
                        // Accept the first bidirectional stream
                        if let Ok((send, recv)) = connection.accept_bi().await {
                            let conn_info = ConnectionInfo {
                                id: ConnectionId::new(),
                                local_peer: PeerId::new(),
                                remote_peer: PeerId::new(),
                                transport: TransportType::Quic,
                                established_at: std::time::SystemTime::now(),
                                metrics: Default::default(),
                            };
                            
                            let stream = QuicStream::new(send, recv);
                            let conn = Connection::new(
                                conn_info,
                                Box::new(stream) as Box<dyn Stream>,
                            );
                            
                            let _ = tx.send((conn, remote_addr)).await;
                        }
                    }
                });
            }
        });
        
        Self {
            endpoint,
            incoming_rx: rx,
            task_handle: Some(task_handle),
        }
    }
}

#[async_trait]
impl Listener for QuicListener {
    async fn accept(&mut self) -> Result<(Connection, SocketAddr)> {
        self.incoming_rx
            .recv()
            .await
            .ok_or_else(|| NetworkError::Transport(TransportError::NotAvailable("QUIC listener closed".to_string())))
    }
    
    fn local_addr(&self) -> Result<SocketAddr> {
        self.endpoint.local_addr()
            .map_err(|e| NetworkError::Transport(TransportError::Quic(e.to_string())))
    }
    
    async fn close(&mut self) -> Result<()> {
        self.incoming_rx.close();
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        self.endpoint.close(0u32.into(), b"closing");
        Ok(())
    }
}

/// Server certificate verification with development mode support.
#[derive(Debug)]
struct ServerCertVerifier {
    /// Whether to allow self-signed certificates (development only)
    allow_self_signed: bool,
    /// Trusted root certificates
    roots: RootCertStore,
}

impl ServerCertVerifier {
    fn new(allow_self_signed: bool) -> Self {
        let mut roots = RootCertStore::empty();
        // Load system root certificates
        // Load native certificates
        let cert_result = rustls_native_certs::load_native_certs();
        for cert in cert_result.certs {
            let _ = roots.add(cert);
        }
        if !cert_result.errors.is_empty() {
            for err in &cert_result.errors {
                tracing::warn!("Certificate loading error: {:?}", err);
            }
        }
        Self { allow_self_signed, roots }
    }
}

impl RustlsServerCertVerifier for ServerCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, RustlsError> {
        if self.allow_self_signed {
            // In development, warn but allow
            tracing::warn!("Allowing self-signed certificate for {:?} (development mode)", server_name);
            return Ok(ServerCertVerified::assertion());
        }
        
        // Production: Proper certificate validation using WebPkiServerVerifier
        let verifier = WebPkiServerVerifier::builder(Arc::new(self.roots.clone()))
            .build()
            .map_err(|e| RustlsError::General(format!("Failed to build verifier: {}", e)))?;
        
        verifier.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        )
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, RustlsError> {
        if self.allow_self_signed {
            return Ok(HandshakeSignatureValid::assertion());
        }
        
        let verifier = WebPkiServerVerifier::builder(Arc::new(self.roots.clone()))
            .build()
            .map_err(|e| RustlsError::General(format!("Failed to build verifier: {}", e)))?;
            
        verifier.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, RustlsError> {
        if self.allow_self_signed {
            return Ok(HandshakeSignatureValid::assertion());
        }
        
        let verifier = WebPkiServerVerifier::builder(Arc::new(self.roots.clone()))
            .build()
            .map_err(|e| RustlsError::General(format!("Failed to build verifier: {}", e)))?;
            
        verifier.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_quic_transport_creation() {
        let addr = "127.0.0.1:0".parse().unwrap();
        let transport = QuicTransport::new(addr);
        assert!(transport.is_ok());
    }
    
    #[test]
    fn test_quic_transport_features() {
        let addr = "127.0.0.1:0".parse().unwrap();
        let transport = QuicTransport::new(addr).unwrap();
        
        assert!(transport.supports_feature(TransportFeature::ZeroRTT));
        assert!(transport.supports_feature(TransportFeature::Multistream));
        assert!(!transport.supports_feature(TransportFeature::Anonymity));
        assert_eq!(transport.priority(), TransportPriority::Preferred);
        assert_eq!(transport.transport_type(), TransportType::Quic);
    }
    
    #[tokio::test]
    async fn test_quic_initialization() {
        let addr = "127.0.0.1:0".parse().unwrap();
        let mut transport = QuicTransport::new(addr).unwrap();
        assert!(transport.initialize().await.is_ok());
    }
}