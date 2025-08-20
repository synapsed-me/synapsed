//! # Synapsed-Net
//!
//! Core networking layer for the Synapsed ecosystem with built-in observability.
//!
//! This crate provides a unified, secure, and observable networking stack that supports
//! multiple transport protocols, privacy layers, and comprehensive monitoring through
//! Substrates and Serventis integration.
//!
//! ## Architecture
//!
//! The networking stack is organized in layers:
//! - **Transport Layer**: QUIC, WebRTC, P2P protocols
//! - **Security Layer**: Noise protocol, post-quantum cryptography
//! - **Privacy Layer**: Tor integration, mix networks, anonymous routing
//! - **Observability Layer**: Substrates for events, Serventis for service monitoring
//!
//! ## Features
//!
//! - Multiple transport protocols with automatic selection
//! - Built-in observability from the ground up
//! - Privacy-preserving metrics and monitoring
//! - Post-quantum secure communications
//! - Extensible plugin architecture

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod config;
pub mod compression;
pub mod crypto;
pub mod error;
pub mod observability;
pub mod privacy;
pub mod security;
pub mod transport;
pub mod types;

// Re-export commonly used types
pub use config::{NetworkConfig, TransportConfig};
pub use compression::{CompressionEngine, CompressionConfig, Algorithm, AdaptiveSelector};
pub use crypto::{
    EnhancedSecurityManager, EnhancedSecurityConfig, SecureCipherSuite,
    CertificateValidator, CertificatePinner, SessionManager,
};
pub use error::{NetworkError, Result};
pub use observability::{ObservabilityContext, UnifiedObservability};
pub use privacy::{PrivacyLevel, PrivacyConfig, PrivacyProvider};
pub use security::SecurityLayer;
pub use transport::{Connection, Transport, TransportManager};
pub use types::{PeerId, PeerInfo};

// Re-export core types for better integration
pub use synapsed_core::{SynapsedError, SynapsedResult};
pub use synapsed_core::traits::{Observable, Configurable, Identifiable, Validatable};

// Map NetworkError to SynapsedError
impl From<NetworkError> for SynapsedError {
    fn from(err: NetworkError) -> Self {
        match err {
            NetworkError::ConnectionFailed(msg) => SynapsedError::Network(format!("Connection failed: {}", msg)),
            NetworkError::TransportError(msg) => SynapsedError::Network(format!("Transport error: {}", msg)),
            NetworkError::SecurityError(msg) => SynapsedError::Cryptographic(format!("Security error: {}", msg)),
            NetworkError::ConfigurationError(msg) => SynapsedError::Configuration(msg),
            NetworkError::P2PError(msg) => SynapsedError::P2P(msg),
            NetworkError::Timeout(msg) => SynapsedError::Timeout(msg),
            NetworkError::InvalidPeer(msg) => SynapsedError::InvalidInput(format!("Invalid peer: {}", msg)),
            NetworkError::ProtocolError(msg) => SynapsedError::Network(format!("Protocol error: {}", msg)),
            NetworkError::InvalidAddress(msg) => SynapsedError::InvalidInput(format!("Invalid address: {}", msg)),
            NetworkError::InvalidMessage(msg) => SynapsedError::InvalidInput(format!("Invalid message: {}", msg)),
        }
    }
}

// Re-export observability frameworks
pub use synapsed_serventis as serventis;
pub use synapsed_substrates as substrates;

use std::sync::Arc;
use tokio::sync::RwLock;

/// The main entry point for the Synapsed networking stack.
#[derive(Clone)]
pub struct NetworkStack {
    config: Arc<NetworkConfig>,
    transport_manager: Arc<TransportManager>,
    observability: Arc<UnifiedObservability>,
    state: Arc<RwLock<NetworkState>>,
}

#[derive(Default)]
struct NetworkState {
    is_initialized: bool,
    active_connections: usize,
}

// Implement core traits for NetworkStack
impl Identifiable for NetworkStack {
    fn id(&self) -> uuid::Uuid {
        // Generate a consistent ID based on configuration
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        format!("{:?}", self.config).hash(&mut hasher);
        let hash = hasher.finish();
        
        // Convert hash to UUID (deterministic)
        let bytes = hash.to_be_bytes();
        let mut uuid_bytes = [0u8; 16];
        for (i, &byte) in bytes.iter().cycle().take(16).enumerate() {
            uuid_bytes[i] = byte;
        }
        uuid::Uuid::from_bytes(uuid_bytes)
    }

    fn name(&self) -> &str {
        "NetworkStack"
    }

    fn type_name(&self) -> &'static str {
        "NetworkStack"
    }
}

impl Validatable for NetworkStack {
    fn validate(&self) -> SynapsedResult<()> {
        // Validate basic configuration
        if self.config.transport.enabled_transports.is_empty() {
            return Err(SynapsedError::Configuration("No transports enabled".to_string()));
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Observable for NetworkStack {
    async fn status(&self) -> SynapsedResult<synapsed_core::traits::ObservableStatus> {
        use synapsed_core::traits::*;
        use std::collections::HashMap;
        
        let state = self.state.read().await;
        let obs_state = if state.is_initialized {
            ObservableState::Running
        } else {
            ObservableState::Stopped
        };
        
        let mut metadata = HashMap::new();
        metadata.insert("active_connections".to_string(), state.active_connections.to_string());
        metadata.insert("enabled_transports".to_string(), self.config.transport.enabled_transports.len().to_string());
        
        Ok(ObservableStatus {
            state: obs_state,
            last_updated: chrono::Utc::now(),
            metadata,
        })
    }

    async fn health(&self) -> SynapsedResult<synapsed_core::traits::HealthStatus> {
        use synapsed_core::traits::*;
        use std::collections::HashMap;
        
        let mut checks = HashMap::new();
        
        // Check initialization status
        let state = self.state.read().await;
        let init_check = if state.is_initialized {
            HealthCheck {
                level: HealthLevel::Healthy,
                message: "Network stack is initialized".to_string(),
                timestamp: chrono::Utc::now(),
            }
        } else {
            HealthCheck {
                level: HealthLevel::Critical,
                message: "Network stack not initialized".to_string(),
                timestamp: chrono::Utc::now(),
            }
        };
        checks.insert("initialization".to_string(), init_check);
        
        // Check connection health
        let connection_check = if state.active_connections > 0 {
            HealthCheck {
                level: HealthLevel::Healthy,
                message: format!("Active connections: {}", state.active_connections),
                timestamp: chrono::Utc::now(),
            }
        } else {
            HealthCheck {
                level: HealthLevel::Warning,
                message: "No active connections".to_string(),
                timestamp: chrono::Utc::now(),
            }
        };
        checks.insert("connections".to_string(), connection_check);
        
        let overall = if checks.values().any(|c| c.level == HealthLevel::Critical) {
            HealthLevel::Critical
        } else if checks.values().any(|c| c.level == HealthLevel::Warning) {
            HealthLevel::Warning
        } else {
            HealthLevel::Healthy
        };
        
        Ok(HealthStatus {
            overall,
            checks,
            last_check: chrono::Utc::now(),
        })
    }

    async fn metrics(&self) -> SynapsedResult<HashMap<String, f64>> {
        let mut metrics = HashMap::new();
        
        let state = self.state.read().await;
        metrics.insert("active_connections".to_string(), state.active_connections as f64);
        metrics.insert("enabled_transports".to_string(), self.config.transport.enabled_transports.len() as f64);
        metrics.insert("is_initialized".to_string(), if state.is_initialized { 1.0 } else { 0.0 });
        
        Ok(metrics)
    }

    fn describe(&self) -> String {
        format!(
            "NetworkStack: {} transports enabled, {} active connections",
            self.config.transport.enabled_transports.len(),
            // We can't await here, so we'll use a default
            0 // state.active_connections - would need async
        )
    }
}

impl NetworkStack {
    /// Creates a new network stack with the given configuration.
    pub async fn new(config: NetworkConfig) -> Result<Self> {
        // Initialize observability first
        let observability = UnifiedObservability::new(&config.observability).await?;
        
        // Create transport manager with observability
        let transport_manager = TransportManager::with_observability(
            config.transport.default_transport,
            observability.clone()
        );
        
        Ok(Self {
            config: Arc::new(config),
            transport_manager: Arc::new(transport_manager),
            observability,
            state: Arc::new(RwLock::new(NetworkState::default())),
        })
    }
    
    /// Initializes the network stack and starts all services.
    pub async fn initialize(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if state.is_initialized {
            return Ok(());
        }
        
        // Start observability services
        self.observability.start().await?;
        
        // Initialize transports
        self.transport_manager.initialize().await?;
        
        state.is_initialized = true;
        Ok(())
    }
    
    /// Connects to a peer using the best available transport.
    pub async fn connect(&self, peer: &PeerInfo) -> Result<Connection> {
        let connection = self.transport_manager.connect(peer).await?;
        
        let mut state = self.state.write().await;
        state.active_connections += 1;
        
        Ok(connection)
    }
    
    /// Shuts down the network stack gracefully.
    pub async fn shutdown(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if !state.is_initialized {
            return Ok(());
        }
        
        // Shutdown transports
        self.transport_manager.shutdown().await?;
        
        // Stop observability services
        self.observability.stop().await?;
        
        state.is_initialized = false;
        state.active_connections = 0;
        
        Ok(())
    }
    
    /// Returns the current observability context.
    pub fn observability(&self) -> &Arc<UnifiedObservability> {
        &self.observability
    }
    
    /// Returns the transport manager.
    pub fn transport_manager(&self) -> &Arc<TransportManager> {
        &self.transport_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_network_stack_lifecycle() {
        let config = NetworkConfig::default();
        let stack = NetworkStack::new(config).await.unwrap();
        
        // Test initialization
        stack.initialize().await.unwrap();
        
        // Test double initialization is safe
        stack.initialize().await.unwrap();
        
        // Test shutdown
        stack.shutdown().await.unwrap();
        
        // Test double shutdown is safe
        stack.shutdown().await.unwrap();
    }
}