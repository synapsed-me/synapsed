//! Configuration types for the networking layer.

use crate::types::TransportType;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main configuration for the network stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Transport layer configuration
    pub transport: TransportConfig,
    
    /// Security configuration
    pub security: SecurityConfig,
    
    /// Privacy configuration
    pub privacy: PrivacyConfig,
    
    /// Observability configuration
    pub observability: ObservabilityConfig,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            transport: TransportConfig::default(),
            security: SecurityConfig::default(),
            privacy: PrivacyConfig::default(),
            observability: ObservabilityConfig::default(),
        }
    }
}

/// Transport layer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Default transport type to use
    pub default_transport: TransportType,
    /// Enable QUIC transport
    pub enable_quic: bool,
    
    /// Enable WebRTC transport
    pub enable_webrtc: bool,
    
    /// Enable libp2p transport
    pub enable_libp2p: bool,
    
    /// QUIC-specific configuration
    pub quic: QuicConfig,
    
    /// WebRTC-specific configuration
    pub webrtc: WebRtcConfig,
    
    /// libp2p-specific configuration
    pub libp2p: Libp2pConfig,
    
    /// Connection timeout
    pub connection_timeout: Duration,
    
    /// Maximum concurrent connections
    pub max_connections: usize,
    
    /// Transport selection strategy
    pub selection_strategy: SelectionStrategy,
    
    /// Whether to prefer anonymous transports
    pub prefer_anonymity: bool,
    
    /// Whether to require post-quantum security
    pub require_post_quantum: bool,
}

/// Transport selection strategy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SelectionStrategy {
    /// Always select the best matching transport
    BestMatch,
    
    /// Randomly select from suitable transports
    Random,
    
    /// Round-robin between suitable transports
    RoundRobin,
}

impl Default for SelectionStrategy {
    fn default() -> Self {
        Self::BestMatch
    }
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            default_transport: TransportType::Quic,
            enable_quic: true,
            enable_webrtc: true,
            enable_libp2p: true,
            quic: QuicConfig::default(),
            webrtc: WebRtcConfig::default(),
            libp2p: Libp2pConfig::default(),
            connection_timeout: Duration::from_secs(30),
            max_connections: 1000,
            selection_strategy: SelectionStrategy::BestMatch,
            prefer_anonymity: false,
            require_post_quantum: false,
        }
    }
}

/// QUIC transport configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuicConfig {
    /// Server name for TLS
    pub server_name: Option<String>,
    
    /// Keep-alive interval
    pub keep_alive_interval: Duration,
    
    /// Maximum idle timeout
    pub max_idle_timeout: Duration,
    
    /// Enable 0-RTT
    pub enable_0rtt: bool,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            server_name: None,
            keep_alive_interval: Duration::from_secs(30),
            max_idle_timeout: Duration::from_secs(300),
            enable_0rtt: true,
        }
    }
}

/// WebRTC transport configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRtcConfig {
    /// STUN servers
    pub stun_servers: Vec<String>,
    
    /// TURN servers
    pub turn_servers: Vec<TurnServer>,
    
    /// Enable unreliable channels
    pub enable_unreliable: bool,
}

impl Default for WebRtcConfig {
    fn default() -> Self {
        Self {
            stun_servers: vec!["stun:stun.l.google.com:19302".to_string()],
            turn_servers: vec![],
            enable_unreliable: true,
        }
    }
}

/// TURN server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnServer {
    /// Server URL
    pub url: String,
    
    /// Username
    pub username: Option<String>,
    
    /// Credential
    pub credential: Option<String>,
}

/// libp2p transport configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Libp2pConfig {
    /// Enable Kademlia DHT
    pub enable_kad: bool,
    
    /// Enable mDNS discovery
    pub enable_mdns: bool,
    
    /// Bootstrap nodes
    pub bootstrap_nodes: Vec<String>,
}

impl Default for Libp2pConfig {
    fn default() -> Self {
        Self {
            enable_kad: true,
            enable_mdns: true,
            bootstrap_nodes: vec![],
        }
    }
}

/// Security configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Noise protocol patterns to support
    pub noise_patterns: Vec<String>,
    
    /// Enable post-quantum cryptography
    pub enable_post_quantum: bool,
    
    /// Certificate path for TLS
    pub cert_path: Option<String>,
    
    /// Key path for TLS
    pub key_path: Option<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            noise_patterns: vec!["Noise_XX_25519_ChaChaPoly_BLAKE2s".to_string()],
            enable_post_quantum: true,
            cert_path: None,
            key_path: None,
        }
    }
}

/// Privacy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Enable Tor integration
    pub enable_tor: bool,
    
    /// Enable mix network
    pub enable_mixnet: bool,
    
    /// Anonymization level
    pub anonymization_level: AnonymizationLevel,
    
    /// Minimum k-anonymity value
    pub k_anonymity: usize,
    
    /// Differential privacy epsilon
    pub differential_privacy_epsilon: f64,
    
    /// Data retention period
    pub data_retention: Duration,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            enable_tor: false,
            enable_mixnet: false,
            anonymization_level: AnonymizationLevel::Standard,
            k_anonymity: 5,
            differential_privacy_epsilon: 1.0,
            data_retention: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
        }
    }
}

/// Anonymization levels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AnonymizationLevel {
    /// No anonymization
    None,
    /// Standard anonymization
    Standard,
    /// High anonymization
    High,
    /// Maximum anonymization
    Maximum,
}

/// Observability configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Substrates configuration
    pub substrates: SubstratesConfig,
    
    /// Serventis configuration
    pub serventis: ServentisConfig,
    
    /// Performance configuration
    pub performance: PerformanceConfig,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            substrates: SubstratesConfig::default(),
            serventis: ServentisConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

/// Substrates configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstratesConfig {
    /// Buffer size for event channels
    pub buffer_size: usize,
    
    /// Event flush interval
    pub flush_interval: Duration,
    
    /// Sampling rate for high-frequency events
    pub sampling_rate: f64,
}

impl Default for SubstratesConfig {
    fn default() -> Self {
        Self {
            buffer_size: 10000,
            flush_interval: Duration::from_millis(100),
            sampling_rate: 0.1,
        }
    }
}

/// Serventis configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServentisConfig {
    /// Assessment interval for monitors
    pub assessment_interval: Duration,
    
    /// Confidence threshold for state changes
    pub confidence_threshold: f64,
    
    /// Enable detailed reporting
    pub enable_detailed_reporting: bool,
}

impl Default for ServentisConfig {
    fn default() -> Self {
        Self {
            assessment_interval: Duration::from_secs(10),
            confidence_threshold: 0.8,
            enable_detailed_reporting: false,
        }
    }
}

/// Performance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Maximum CPU overhead percentage
    pub max_cpu_overhead: f64,
    
    /// Maximum memory overhead in MB
    pub max_memory_overhead: usize,
    
    /// Maximum added latency in milliseconds
    pub max_latency_overhead: u64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_cpu_overhead: 5.0,
            max_memory_overhead: 50,
            max_latency_overhead: 1,
        }
    }
}