//! HTTP/TLS transport implementation for MCP client

use crate::{
    error::{McpError, Result},
    client::ClientConfig,
    protocol::{JsonRpcRequest, JsonRpcResponse},
};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, Request, Response};
use hyper_util::client::legacy::{Client, connect::HttpConnector};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use rustls::pki_types::ServerName;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// HTTP transport for MCP client with TLS support
pub struct HttpTransport {
    client: Client<HttpsConnector<HttpConnector>, Full<Bytes>>,
    server_url: String,
    request_tx: mpsc::UnboundedSender<JsonRpcRequest>,
    response_rx: Arc<RwLock<mpsc::UnboundedReceiver<JsonRpcResponse>>>,
    shutdown_tx: mpsc::Sender<()>,
}

impl HttpTransport {
    /// Create a new HTTP transport
    pub async fn new(config: &ClientConfig) -> Result<Self> {
        info!("Creating HTTP transport for {}", config.server_url);
        
        // Create HTTPS connector with TLS configuration
        let https = if config.use_tls {
            let mut roots = rustls::RootCertStore::empty();
            
            // Load system certificates
            for cert in rustls_native_certs::load_native_certs()
                .map_err(|e| McpError::Transport(format!("Failed to load native certs: {}", e)))? 
            {
                roots.add(cert).map_err(|e| 
                    McpError::Transport(format!("Failed to add certificate: {}", e)))?;
            }
            
            let tls_config = if config.allow_self_signed {
                // Development mode: accept self-signed certificates
                warn!("Allowing self-signed certificates - DO NOT USE IN PRODUCTION");
                rustls::ClientConfig::builder()
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(DangerousAcceptAll))
                    .with_no_client_auth()
            } else {
                // Production mode: verify certificates
                rustls::ClientConfig::builder()
                    .with_root_certificates(roots)
                    .with_no_client_auth()
            };
            
            HttpsConnectorBuilder::new()
                .with_tls_config(tls_config)
                .https_or_http()
                .enable_all_versions()
                .build()
        } else {
            // HTTP only (not recommended)
            warn!("Using unencrypted HTTP - DO NOT USE IN PRODUCTION");
            HttpsConnectorBuilder::new()
                .with_tls_config(
                    rustls::ClientConfig::builder()
                        .dangerous()
                        .with_custom_certificate_verifier(Arc::new(DangerousAcceptAll))
                        .with_no_client_auth()
                )
                .https_or_http()
                .enable_all_versions()
                .build()
        };
        
        // Create HTTP client with connection pooling
        let client = Client::builder(hyper_util::rt::TokioExecutor::new())
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(config.pool_size)
            .http2_only(true) // Use HTTP/2 for streaming
            .build(https);
        
        // Create channels for request/response handling
        let (request_tx, mut request_rx) = mpsc::unbounded_channel::<JsonRpcRequest>();
        let (response_tx, response_rx) = mpsc::unbounded_channel::<JsonRpcResponse>();
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        
        let transport = Self {
            client,
            server_url: config.server_url.clone(),
            request_tx,
            response_rx: Arc::new(RwLock::new(response_rx)),
            shutdown_tx,
        };
        
        // Spawn request handler task
        let client_clone = transport.client.clone();
        let server_url = config.server_url.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(request) = request_rx.recv() => {
                        let client = client_clone.clone();
                        let url = server_url.clone();
                        let tx = response_tx.clone();
                        
                        // Send request in background
                        tokio::spawn(async move {
                            match Self::send_request_internal(client, url, request).await {
                                Ok(response) => {
                                    if let Err(e) = tx.send(response) {
                                        error!("Failed to send response: {}", e);
                                    }
                                }
                                Err(e) => {
                                    error!("Request failed: {}", e);
                                }
                            }
                        });
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Shutting down HTTP transport");
                        break;
                    }
                }
            }
        });
        
        Ok(transport)
    }
    
    /// Send a JSON-RPC request
    pub async fn send_request(&self, request: JsonRpcRequest) -> Result<()> {
        self.request_tx.send(request)
            .map_err(|e| McpError::Transport(format!("Failed to queue request: {}", e)))?;
        Ok(())
    }
    
    /// Internal request sending
    async fn send_request_internal(
        client: Client<HttpsConnector<HttpConnector>, Full<Bytes>>,
        server_url: String,
        request: JsonRpcRequest,
    ) -> Result<JsonRpcResponse> {
        debug!("Sending request to {}: {:?}", server_url, request.method);
        
        // Serialize request
        let body = serde_json::to_vec(&request)
            .map_err(|e| McpError::SerializationError(e.to_string()))?;
        
        // Build HTTP request
        let req = Request::builder()
            .method("POST")
            .uri(&server_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(Full::new(Bytes::from(body)))
            .map_err(|e| McpError::Transport(format!("Failed to build request: {}", e)))?;
        
        // Send request
        let response = client.request(req).await
            .map_err(|e| McpError::Transport(format!("HTTP request failed: {}", e)))?;
        
        // Check status
        if !response.status().is_success() {
            return Err(McpError::Transport(format!(
                "Server returned error status: {}",
                response.status()
            )));
        }
        
        // Read response body
        let body = response.into_body()
            .collect()
            .await
            .map_err(|e| McpError::Transport(format!("Failed to read response: {}", e)))?
            .to_bytes();
        
        // Parse JSON-RPC response
        let json_response: JsonRpcResponse = serde_json::from_slice(&body)
            .map_err(|e| McpError::SerializationError(format!("Invalid JSON response: {}", e)))?;
        
        Ok(json_response)
    }
    
    /// Receive a response
    pub async fn receive_response(&self) -> Option<JsonRpcResponse> {
        let mut rx = self.response_rx.write().await;
        rx.recv().await
    }
    
    /// Close the transport
    pub async fn close(&self) -> Result<()> {
        let _ = self.shutdown_tx.send(()).await;
        Ok(())
    }
}

/// Dangerous certificate verifier that accepts all certificates
/// ONLY FOR DEVELOPMENT - DO NOT USE IN PRODUCTION
#[derive(Debug)]
struct DangerousAcceptAll;

impl rustls::client::danger::ServerCertVerifier for DangerousAcceptAll {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
        ]
    }
}

/// HTTP/2 streaming transport for bidirectional communication
pub struct StreamingTransport {
    client: Client<HttpsConnector<HttpConnector>, Full<Bytes>>,
    server_url: String,
    stream_tx: mpsc::UnboundedSender<JsonRpcRequest>,
    stream_rx: Arc<RwLock<mpsc::UnboundedReceiver<JsonRpcResponse>>>,
}

impl StreamingTransport {
    /// Create a new streaming transport with HTTP/2
    pub async fn new(config: &ClientConfig) -> Result<Self> {
        info!("Creating HTTP/2 streaming transport");
        
        // Reuse HTTPS configuration from HttpTransport
        let https = HttpsConnectorBuilder::new()
            .with_tls_config(
                rustls::ClientConfig::builder()
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(DangerousAcceptAll))
                    .with_no_client_auth()
            )
            .https_or_http()
            .enable_all_versions()
            .build();
        
        let client = Client::builder(hyper_util::rt::TokioExecutor::new())
            .http2_only(true)
            .build(https);
        
        let (stream_tx, stream_rx) = mpsc::unbounded_channel();
        let (response_tx, response_rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            client,
            server_url: config.server_url.clone(),
            stream_tx,
            stream_rx: Arc::new(RwLock::new(response_rx)),
        })
    }
    
    /// Establish a streaming connection
    pub async fn connect_stream(&self) -> Result<()> {
        info!("Establishing HTTP/2 stream to {}", self.server_url);
        
        // This would establish a persistent HTTP/2 stream
        // Implementation depends on server's streaming endpoint
        
        Ok(())
    }
}