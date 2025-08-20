//! Runtime configuration for the WASM runtime

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::types::CompilationTarget;

/// Complete runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Compilation settings
    pub compilation: CompilationConfig,
    /// Feature flags
    pub features: FeatureConfig,
    /// Resource limits
    pub limits: LimitsConfig,
    /// Security settings
    pub security: SecurityConfig,
    /// Memory management
    pub memory: MemoryConfig,
    /// Optimization settings
    pub optimization: OptimizationConfig,
    /// Debug settings
    pub debug: DebugConfig,
    /// Networking configuration
    pub network: NetworkConfig,
    /// P2P platform configuration
    pub p2p: P2pConfig,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            compilation: CompilationConfig::default(),
            features: FeatureConfig::default(),
            limits: LimitsConfig::default(),
            security: SecurityConfig::default(),
            memory: MemoryConfig::default(),
            optimization: OptimizationConfig::default(),
            debug: DebugConfig::default(),
            network: NetworkConfig::default(),
            p2p: P2pConfig::default(),
        }
    }
}

impl RuntimeConfig {
    /// Create a new configuration with safe defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Create configuration optimized for development
    pub fn development() -> Self {
        Self {
            debug: DebugConfig {
                enable_debug_info: true,
                enable_logging: true,
                log_level: LogLevel::Debug,
                enable_profiling: true,
            },
            limits: LimitsConfig {
                default_timeout: Duration::from_secs(300), // 5 minutes for development
                ..Default::default()
            },
            optimization: OptimizationConfig {
                enable_optimizations: false, // Disable for faster compilation
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Create configuration optimized for production
    pub fn production() -> Self {
        Self {
            security: SecurityConfig {
                enable_sandboxing: true,
                strict_validation: true,
                enable_stack_protection: true,
                ..Default::default()
            },
            optimization: OptimizationConfig {
                enable_optimizations: true,
                enable_lto: true,
                optimization_level: OptimizationLevel::Speed,
                ..Default::default()
            },
            limits: LimitsConfig {
                default_timeout: Duration::from_secs(30),
                max_memory_per_module: 64 * 1024 * 1024, // 64MB
                enable_fuel: true,
                ..Default::default()
            },
            debug: DebugConfig {
                enable_debug_info: false,
                enable_logging: true,
                log_level: LogLevel::Warn,
                enable_profiling: false,
            },
            ..Default::default()
        }
    }

    /// Create configuration for P2P communication platform
    pub fn p2p_platform() -> Self {
        Self {
            compilation: CompilationConfig {
                target: CompilationTarget::Web,
                enable_wasi: false, // Not needed for P2P browser apps
            },
            features: FeatureConfig {
                threads: false, // Limited browser support
                simd: true, // Enable for crypto optimization
                multi_value: true,
                bulk_memory: true,
                reference_types: true,
                component_model: false,
                multi_memory: false,
            },
            limits: LimitsConfig {
                default_timeout: Duration::from_secs(10), // Responsive P2P operations
                max_memory_per_module: 32 * 1024 * 1024, // 32MB for browser safety
                max_stack_size: 1024 * 1024, // 1MB stack
                enable_fuel: false, // Not needed for P2P
                max_modules: 50, // Reasonable limit for P2P apps
                max_call_depth: 500,
                ..Default::default()
            },
            security: SecurityConfig {
                enable_sandboxing: true,
                strict_validation: true,
                enable_stack_protection: true,
                enable_deterministic_execution: false, // P2P allows non-deterministic operations
                disable_unsafe_host_functions: false, // Need WebRTC/crypto functions
                max_imports: 500,
                max_exports: 200,
            },
            memory: MemoryConfig {
                memory_pool_size: 64 * 1024 * 1024, // 64MB pool for browser
                enable_memory_sharing: false, // Not supported in browsers
                enable_gc: true,
                gc_threshold: 32 * 1024 * 1024, // 32MB threshold
                enable_memory_protection: true,
                page_size: 64 * 1024,
            },
            optimization: OptimizationConfig {
                enable_optimizations: true,
                optimization_level: OptimizationLevel::Size, // Optimize for bundle size
                enable_lto: true,
                enable_simd_optimizations: true, // Important for crypto
                enable_parallel_compilation: true,
            },
            network: NetworkConfig {
                enable_network_access: true, // P2P needs network access
                allowed_origins: vec!["*".to_string()], // Will be configured per deployment
                network_timeout: Duration::from_secs(30), // P2P operations may take longer
                max_connections: 50, // Reasonable P2P peer limit
            },
            p2p: P2pConfig {
                enable_webrtc: true,
                enable_crdt: true,
                enable_sync: true,
                enable_zkp: true,
                enable_did: true,
                enable_pwa: true,
                max_peers: 20,
                webrtc_config: WebRtcConfig::default(),
                crdt_config: CrdtConfig::default(),
                sync_config: SyncConfig::default(),
            },
            debug: DebugConfig {
                enable_debug_info: false,
                enable_logging: true,
                log_level: LogLevel::Info,
                enable_profiling: false,
            },
            ..Default::default()
        }
    }

    /// Create configuration for blockchain/smart contract execution (deprecated for P2P platform)
    #[deprecated(note = "Use p2p_platform() for P2P communication focus")]
    pub fn blockchain() -> Self {
        // Keep for backward compatibility but mark as deprecated
        Self::p2p_platform()
    }

    /// Create configuration for web deployment
    pub fn web() -> Self {
        Self {
            compilation: CompilationConfig {
                target: CompilationTarget::Web,
                enable_wasi: false,
            },
            memory: MemoryConfig {
                enable_memory_sharing: false, // Not supported in web
                memory_pool_size: 32 * 1024 * 1024, // 32MB pool
                ..Default::default()
            },
            limits: LimitsConfig {
                max_memory_per_module: 32 * 1024 * 1024, // 32MB per module
                default_timeout: Duration::from_secs(10), // Shorter timeout for web
                ..Default::default()
            },
            network: NetworkConfig {
                enable_network_access: false, // Restrict network access
                allowed_origins: vec![], // Will be configured per deployment
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Check memory limits
        if self.memory.memory_pool_size < self.limits.max_memory_per_module {
            return Err("Memory pool size must be larger than max memory per module".to_string());
        }

        // Check timeout values
        if self.limits.default_timeout.is_zero() {
            return Err("Default timeout must be greater than zero".to_string());
        }

        // Validate fuel settings
        if self.limits.enable_fuel && self.limits.default_fuel == 0 {
            return Err("Default fuel must be greater than zero when fuel is enabled".to_string());
        }

        // Check compilation target consistency
        match self.compilation.target {
            CompilationTarget::Web => {
                if self.features.threads {
                    return Err("Threads are not supported in web target".to_string());
                }
                if self.memory.enable_memory_sharing {
                    return Err("Memory sharing is not supported in web target".to_string());
                }
            }
            CompilationTarget::Substrate => {
                if !self.security.enable_deterministic_execution {
                    return Err("Deterministic execution must be enabled for substrate target".to_string());
                }
            }
            _ => {}
        }

        // Validate P2P configuration
        if self.p2p.max_peers == 0 {
            return Err("Maximum peers must be greater than zero".to_string());
        }
        
        if self.p2p.enable_webrtc && !self.network.enable_network_access {
            return Err("Network access must be enabled for WebRTC".to_string());
        }

        Ok(())
    }
}

/// Compilation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationConfig {
    /// Target compilation platform
    pub target: CompilationTarget,
    /// Enable WASI support
    pub enable_wasi: bool,
}

impl Default for CompilationConfig {
    fn default() -> Self {
        Self {
            target: CompilationTarget::Native,
            enable_wasi: true,
        }
    }
}

/// WASM feature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    /// Enable WASM threads
    pub threads: bool,
    /// Enable SIMD instructions
    pub simd: bool,
    /// Enable multi-value returns
    pub multi_value: bool,
    /// Enable multiple memories
    pub multi_memory: bool,
    /// Enable bulk memory operations
    pub bulk_memory: bool,
    /// Enable reference types
    pub reference_types: bool,
    /// Enable component model
    pub component_model: bool,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            threads: true,
            simd: true,
            multi_value: true,
            multi_memory: false,
            bulk_memory: true,
            reference_types: true,
            component_model: false,
        }
    }
}

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    /// Default execution timeout
    pub default_timeout: Duration,
    /// Maximum memory per module (bytes)
    pub max_memory_per_module: usize,
    /// Maximum stack size (bytes)
    pub max_stack_size: usize,
    /// Enable fuel-based execution limiting
    pub enable_fuel: bool,
    /// Default fuel amount
    pub default_fuel: u64,
    /// Enable epoch-based interruption
    pub enable_epoch_interruption: bool,
    /// Maximum number of loaded modules
    pub max_modules: usize,
    /// Maximum function call depth
    pub max_call_depth: usize,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            max_memory_per_module: 128 * 1024 * 1024, // 128MB
            max_stack_size: 2 * 1024 * 1024, // 2MB
            enable_fuel: false,
            default_fuel: 1_000_000,
            enable_epoch_interruption: true,
            max_modules: 100,
            max_call_depth: 1000,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable module sandboxing
    pub enable_sandboxing: bool,
    /// Strict bytecode validation
    pub strict_validation: bool,
    /// Enable stack overflow protection
    pub enable_stack_protection: bool,
    /// Enable deterministic execution (for blockchain)
    pub enable_deterministic_execution: bool,
    /// Disable unsafe host functions
    pub disable_unsafe_host_functions: bool,
    /// Maximum allowed imports per module
    pub max_imports: usize,
    /// Maximum allowed exports per module
    pub max_exports: usize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_sandboxing: true,
            strict_validation: true,
            enable_stack_protection: true,
            enable_deterministic_execution: false,
            disable_unsafe_host_functions: false,
            max_imports: 1000,
            max_exports: 1000,
        }
    }
}

/// Memory management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Memory pool size for allocation
    pub memory_pool_size: usize,
    /// Enable memory sharing between modules
    pub enable_memory_sharing: bool,
    /// Enable garbage collection
    pub enable_gc: bool,
    /// GC threshold (bytes)
    pub gc_threshold: usize,
    /// Enable memory protection
    pub enable_memory_protection: bool,
    /// Page size for memory allocation
    pub page_size: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            memory_pool_size: 256 * 1024 * 1024, // 256MB
            enable_memory_sharing: true,
            enable_gc: true,
            gc_threshold: 64 * 1024 * 1024, // 64MB
            enable_memory_protection: true,
            page_size: 64 * 1024, // 64KB
        }
    }
}

/// Optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    /// Enable compiler optimizations
    pub enable_optimizations: bool,
    /// Optimization level
    pub optimization_level: OptimizationLevel,
    /// Enable link-time optimization
    pub enable_lto: bool,
    /// Enable SIMD optimizations
    pub enable_simd_optimizations: bool,
    /// Enable parallel compilation
    pub enable_parallel_compilation: bool,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            enable_optimizations: true,
            optimization_level: OptimizationLevel::Balanced,
            enable_lto: false,
            enable_simd_optimizations: true,
            enable_parallel_compilation: true,
        }
    }
}

/// Debug configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    /// Enable debug information in compiled code
    pub enable_debug_info: bool,
    /// Enable runtime logging
    pub enable_logging: bool,
    /// Logging level
    pub log_level: LogLevel,
    /// Enable performance profiling
    pub enable_profiling: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            enable_debug_info: false,
            enable_logging: true,
            log_level: LogLevel::Info,
            enable_profiling: false,
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable network access from WASM modules
    pub enable_network_access: bool,
    /// Allowed origins for web deployment
    pub allowed_origins: Vec<String>,
    /// Network timeout for outbound requests
    pub network_timeout: Duration,
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Enable CORS headers for browser P2P apps
    pub enable_cors: bool,
    /// WebSocket configuration for signaling
    pub websocket_config: WebSocketConfig,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enable_network_access: false,
            allowed_origins: vec!["*".to_string()],
            network_timeout: Duration::from_secs(10),
            max_connections: 100,
            enable_cors: true,
            websocket_config: WebSocketConfig::default(),
        }
    }
}

/// Optimization level
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OptimizationLevel {
    /// No optimizations (fastest compilation)
    None,
    /// Size optimizations
    Size,
    /// Speed optimizations
    Speed,
    /// Balanced optimizations
    Balanced,
}

/// Logging level
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LogLevel {
    /// Error level
    Error,
    /// Warning level
    Warn,
    /// Info level
    Info,
    /// Debug level
    Debug,
    /// Trace level
    Trace,
}

/// P2P platform configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2pConfig {
    /// Enable WebRTC support
    pub enable_webrtc: bool,
    /// Enable CRDT synchronization
    pub enable_crdt: bool,
    /// Enable rsync-like synchronization
    pub enable_sync: bool,
    /// Enable zero-knowledge proofs
    pub enable_zkp: bool,
    /// Enable DID operations
    pub enable_did: bool,
    /// Enable PWA features
    pub enable_pwa: bool,
    /// Maximum number of peers
    pub max_peers: usize,
    /// WebRTC configuration
    pub webrtc_config: WebRtcConfig,
    /// CRDT configuration
    pub crdt_config: CrdtConfig,
    /// Sync configuration
    pub sync_config: SyncConfig,
}

impl Default for P2pConfig {
    fn default() -> Self {
        Self {
            enable_webrtc: true,
            enable_crdt: true,
            enable_sync: true,
            enable_zkp: false, // Optional, may require additional setup
            enable_did: false, // Optional, may require additional setup
            enable_pwa: true,
            max_peers: 10,
            webrtc_config: WebRtcConfig::default(),
            crdt_config: CrdtConfig::default(),
            sync_config: SyncConfig::default(),
        }
    }
}

/// WebRTC configuration for P2P connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRtcConfig {
    /// STUN servers for NAT traversal
    pub stun_servers: Vec<String>,
    /// TURN servers for relay (optional)
    pub turn_servers: Vec<TurnServer>,
    /// Data channel configuration
    pub data_channel_config: DataChannelConfig,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Keep-alive interval
    pub keep_alive_interval: Duration,
}

impl Default for WebRtcConfig {
    fn default() -> Self {
        Self {
            stun_servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ],
            turn_servers: vec![],
            data_channel_config: DataChannelConfig::default(),
            connection_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(10),
        }
    }
}

/// TURN server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnServer {
    /// TURN server URL
    pub url: String,
    /// Username for authentication
    pub username: String,
    /// Credential for authentication
    pub credential: String,
}

/// Data channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataChannelConfig {
    /// Ordered delivery
    pub ordered: bool,
    /// Maximum retransmits
    pub max_retransmits: Option<u16>,
    /// Maximum packet lifetime
    pub max_packet_lifetime: Option<Duration>,
}

impl Default for DataChannelConfig {
    fn default() -> Self {
        Self {
            ordered: true,
            max_retransmits: Some(3),
            max_packet_lifetime: Some(Duration::from_secs(5)),
        }
    }
}

/// CRDT configuration for real-time collaboration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtConfig {
    /// Synchronization interval
    pub sync_interval: Duration,
    /// Maximum operations per sync
    pub max_ops_per_sync: usize,
    /// Enable operation compression
    pub enable_compression: bool,
    /// Conflict resolution strategy
    pub conflict_resolution: ConflictResolution,
}

impl Default for CrdtConfig {
    fn default() -> Self {
        Self {
            sync_interval: Duration::from_millis(100),
            max_ops_per_sync: 100,
            enable_compression: true,
            conflict_resolution: ConflictResolution::LastWriteWins,
        }
    }
}

/// Sync configuration for rsync-like operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Chunk size for synchronization
    pub chunk_size: usize,
    /// Maximum concurrent transfers
    pub max_concurrent_transfers: usize,
    /// Enable delta compression
    pub enable_delta_compression: bool,
    /// Verification method
    pub verification_method: VerificationMethod,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            chunk_size: 64 * 1024, // 64KB chunks
            max_concurrent_transfers: 4,
            enable_delta_compression: true,
            verification_method: VerificationMethod::Sha256,
        }
    }
}

/// WebSocket configuration for signaling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Maximum message size
    pub max_message_size: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Ping interval
    pub ping_interval: Duration,
    /// Enable compression
    pub enable_compression: bool,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_message_size: 1024 * 1024, // 1MB
            connection_timeout: Duration::from_secs(10),
            ping_interval: Duration::from_secs(30),
            enable_compression: true,
        }
    }
}

/// Conflict resolution strategies for CRDT
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Last write wins (timestamp-based)
    LastWriteWins,
    /// First write wins
    FirstWriteWins,
    /// Merge conflicts with custom strategy
    Merge,
}

/// Verification methods for sync operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VerificationMethod {
    /// SHA-256 hash verification
    Sha256,
    /// CRC32 checksum
    Crc32,
    /// No verification (fastest)
    None,
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RuntimeConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_development_config() {
        let config = RuntimeConfig::development();
        assert!(config.validate().is_ok());
        assert!(config.debug.enable_debug_info);
        assert_eq!(config.debug.log_level, LogLevel::Debug);
    }

    #[test]
    fn test_production_config() {
        let config = RuntimeConfig::production();
        assert!(config.validate().is_ok());
        assert!(config.security.enable_sandboxing);
        assert!(config.optimization.enable_optimizations);
        assert!(!config.debug.enable_debug_info);
    }

    #[test]
    fn test_p2p_platform_config() {
        let config = RuntimeConfig::p2p_platform();
        assert!(config.validate().is_ok());
        assert_eq!(config.compilation.target, CompilationTarget::Web);
        assert!(config.p2p.enable_webrtc);
        assert!(config.p2p.enable_crdt);
        assert!(config.features.simd); // Important for crypto
        assert!(!config.features.threads); // Not supported in browsers
        assert!(config.network.enable_network_access);
        assert_eq!(config.optimization.optimization_level, OptimizationLevel::Size);
    }

    #[test]
    fn test_blockchain_config_deprecated() {
        let config = RuntimeConfig::blockchain();
        assert!(config.validate().is_ok());
        // Should now return P2P config
        assert_eq!(config.compilation.target, CompilationTarget::Web);
        assert!(config.p2p.enable_webrtc);
    }

    #[test]
    fn test_web_config() {
        let config = RuntimeConfig::web();
        assert!(config.validate().is_ok());
        assert_eq!(config.compilation.target, CompilationTarget::Web);
        assert!(!config.memory.enable_memory_sharing);
        assert!(!config.network.enable_network_access);
    }

    #[test]
    fn test_config_validation() {
        let mut config = RuntimeConfig::default();
        
        // Test invalid memory configuration
        config.memory.memory_pool_size = 10;
        config.limits.max_memory_per_module = 100;
        assert!(config.validate().is_err());
        
        // Test invalid fuel configuration
        config = RuntimeConfig::default();
        config.limits.enable_fuel = true;
        config.limits.default_fuel = 0;
        assert!(config.validate().is_err());
        
        // Test invalid timeout
        config = RuntimeConfig::default();
        config.limits.default_timeout = Duration::ZERO;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_optimization_levels() {
        let config = RuntimeConfig {
            optimization: OptimizationConfig {
                optimization_level: OptimizationLevel::Speed,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_log_level_conversion() {
        assert_eq!(tracing::Level::from(LogLevel::Error), tracing::Level::ERROR);
        assert_eq!(tracing::Level::from(LogLevel::Debug), tracing::Level::DEBUG);
    }

    #[test]
    fn test_p2p_config_validation() {
        let mut config = RuntimeConfig::p2p_platform();
        
        // Test invalid peer count
        config.p2p.max_peers = 0;
        assert!(config.validate().is_err());
        
        // Test WebRTC without network access
        config = RuntimeConfig::p2p_platform();
        config.p2p.enable_webrtc = true;
        config.network.enable_network_access = false;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_webrtc_config() {
        let config = WebRtcConfig::default();
        assert!(!config.stun_servers.is_empty());
        assert!(config.data_channel_config.ordered);
        assert_eq!(config.connection_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_crdt_config() {
        let config = CrdtConfig::default();
        assert_eq!(config.sync_interval, Duration::from_millis(100));
        assert_eq!(config.max_ops_per_sync, 100);
        assert!(config.enable_compression);
    }

    #[test]
    fn test_sync_config() {
        let config = SyncConfig::default();
        assert_eq!(config.chunk_size, 64 * 1024);
        assert_eq!(config.max_concurrent_transfers, 4);
        assert!(config.enable_delta_compression);
    }
}