//! Test fixtures and sample data for synapsed-core tests

use std::collections::HashMap;
use synapsed_core::{
    config::ConfigValue,
    network::{NetworkAddress, NetworkMessage, NetworkEvent},
    traits::{HealthCheck, HealthLevel, HealthStatus, ObservableState, ObservableStatus},
};
use uuid::Uuid;

/// Configuration fixtures
pub mod config {
    use super::*;

    /// Sample database configuration
    pub fn database_config() -> ConfigValue {
        let mut config = HashMap::new();
        config.insert("host".to_string(), ConfigValue::String("localhost".to_string()));
        config.insert("port".to_string(), ConfigValue::Integer(5432));
        config.insert("username".to_string(), ConfigValue::String("test_user".to_string()));
        config.insert("password".to_string(), ConfigValue::String("secret".to_string()));
        config.insert("ssl".to_string(), ConfigValue::Boolean(true));
        config.insert("pool_size".to_string(), ConfigValue::Integer(10));
        ConfigValue::Object(config)
    }

    /// Sample network configuration
    pub fn network_config() -> ConfigValue {
        let mut config = HashMap::new();
        config.insert("listen_address".to_string(), ConfigValue::String("127.0.0.1:8080".to_string()));
        config.insert("max_connections".to_string(), ConfigValue::Integer(100));
        config.insert("timeout_seconds".to_string(), ConfigValue::Integer(30));
        config.insert("enable_tls".to_string(), ConfigValue::Boolean(false));
        config.insert("buffer_size".to_string(), ConfigValue::Integer(8192));
        ConfigValue::Object(config)
    }

    /// Sample logging configuration
    pub fn logging_config() -> ConfigValue {
        let mut config = HashMap::new();
        config.insert("level".to_string(), ConfigValue::String("info".to_string()));
        config.insert("format".to_string(), ConfigValue::String("json".to_string()));
        config.insert("enable_tracing".to_string(), ConfigValue::Boolean(true));
        config.insert("output".to_string(), ConfigValue::String("stdout".to_string()));
        ConfigValue::Object(config)
    }

    /// Complete application configuration
    pub fn complete_app_config() -> ConfigValue {
        let mut config = HashMap::new();
        config.insert("database".to_string(), database_config());
        config.insert("network".to_string(), network_config());
        config.insert("logging".to_string(), logging_config());
        ConfigValue::Object(config)
    }

    /// Invalid configuration with wrong types
    pub fn invalid_config() -> ConfigValue {
        let mut config = HashMap::new();
        config.insert("port".to_string(), ConfigValue::String("not-a-number".to_string()));
        config.insert("ssl".to_string(), ConfigValue::String("not-a-boolean".to_string()));
        ConfigValue::Object(config)
    }

    /// Minimal configuration
    pub fn minimal_config() -> ConfigValue {
        let mut config = HashMap::new();
        config.insert("host".to_string(), ConfigValue::String("localhost".to_string()));
        ConfigValue::Object(config)
    }

    /// Configuration with arrays
    pub fn config_with_arrays() -> ConfigValue {
        let mut config = HashMap::new();
        let servers = vec![
            ConfigValue::String("server1.example.com".to_string()),
            ConfigValue::String("server2.example.com".to_string()),
            ConfigValue::String("server3.example.com".to_string()),
        ];
        config.insert("servers".to_string(), ConfigValue::Array(servers));
        
        let ports = vec![
            ConfigValue::Integer(8080),
            ConfigValue::Integer(8081),
            ConfigValue::Integer(8082),
        ];
        config.insert("ports".to_string(), ConfigValue::Array(ports));
        ConfigValue::Object(config)
    }
}

/// Network fixtures
pub mod network {
    use super::*;

    /// Sample socket addresses
    pub fn socket_addresses() -> Vec<NetworkAddress> {
        vec![
            NetworkAddress::Socket("127.0.0.1:8080".parse().unwrap()),
            NetworkAddress::Socket("0.0.0.0:8080".parse().unwrap()),
            NetworkAddress::Socket("[::1]:8080".parse().unwrap()),
            NetworkAddress::Socket("[::]:8080".parse().unwrap()),
        ]
    }

    /// Sample peer IDs
    pub fn peer_addresses() -> Vec<NetworkAddress> {
        vec![
            NetworkAddress::PeerId("12D3KooWBhV9dP6FDvFzRK8f2nbDXDDZ7NZ8xJ3Q4k5v7W8X9Y2A".to_string()),
            NetworkAddress::PeerId("12D3KooWEMh5gKz2vH4F8X7q3Y9pR5nL6kJ4D3m8X2Q1v9W7Z8A".to_string()),
            NetworkAddress::PeerId("12D3KooWFx3Y9q2pL8K4vR7nX5J6mD3Q1kA8v2W9Z7Y3X4F5G6".to_string()),
        ]
    }

    /// Sample DID addresses
    pub fn did_addresses() -> Vec<NetworkAddress> {
        vec![
            NetworkAddress::Did("did:example:alice".to_string()),
            NetworkAddress::Did("did:key:z6MkpTHR8VNsBxYAAWHut2Geadd9jSwuBV8xRoAnwWsdvktH".to_string()),
            NetworkAddress::Did("did:web:example.com:user:alice".to_string()),
        ]
    }

    /// Sample multiaddresses
    pub fn multiaddresses() -> Vec<NetworkAddress> {
        vec![
            NetworkAddress::Multiaddr("/ip4/127.0.0.1/tcp/8080".to_string()),
            NetworkAddress::Multiaddr("/ip6/::1/tcp/8080".to_string()),
            NetworkAddress::Multiaddr("/dns4/example.com/tcp/8080".to_string()),
            NetworkAddress::Multiaddr("/ip4/192.168.1.1/tcp/8080/p2p/12D3KooW...".to_string()),
        ]
    }

    /// Sample WebRTC addresses
    pub fn webrtc_addresses() -> Vec<NetworkAddress> {
        vec![
            NetworkAddress::WebRtc("webrtc://stun:stun.l.google.com:19302".to_string()),
            NetworkAddress::WebRtc("webrtc://turn:turn.example.com:3478".to_string()),
        ]
    }

    /// Sample custom addresses
    pub fn custom_addresses() -> Vec<NetworkAddress> {
        vec![
            NetworkAddress::Custom {
                protocol: "ipc".to_string(),
                address: "/tmp/synapsed.sock".to_string(),
            },
            NetworkAddress::Custom {
                protocol: "memory".to_string(),
                address: "test-channel".to_string(),
            },
            NetworkAddress::Custom {
                protocol: "mqtt".to_string(),
                address: "broker.example.com:1883/topic".to_string(),
            },
        ]
    }

    /// All network address types
    pub fn all_address_types() -> Vec<NetworkAddress> {
        let mut addresses = Vec::new();
        addresses.extend(socket_addresses());
        addresses.extend(peer_addresses());
        addresses.extend(did_addresses());
        addresses.extend(multiaddresses());
        addresses.extend(webrtc_addresses());
        addresses.extend(custom_addresses());
        addresses
    }

    /// Sample network messages
    pub fn sample_messages() -> Vec<NetworkMessage> {
        vec![
            NetworkMessage::new("ping", b"ping".to_vec()),
            NetworkMessage::new("pong", b"pong".to_vec()),
            NetworkMessage::new("handshake", b"hello".to_vec())
                .with_header("version", "1.0")
                .with_header("client", "synapsed-test"),
            NetworkMessage::new("data", generate_sample_payload(1024))
                .with_header("content-type", "application/octet-stream")
                .with_header("compression", "gzip"),
            NetworkMessage::new("heartbeat", Vec::new())
                .with_header("timestamp", &chrono::Utc::now().to_rfc3339()),
        ]
    }

    /// Generate sample payload data
    pub fn generate_sample_payload(size: usize) -> Vec<u8> {
        (0..size).map(|i| (i % 256) as u8).collect()
    }

    /// Sample network events
    pub fn sample_events() -> Vec<NetworkEvent> {
        let connection_id = Uuid::new_v4();
        let message = NetworkMessage::new("test", b"test data".to_vec());
        
        vec![
            NetworkEvent::ConnectionEstablished {
                connection_id,
                remote_address: NetworkAddress::Socket("192.168.1.100:8080".parse().unwrap()),
            },
            NetworkEvent::MessageReceived {
                connection_id,
                message: message.clone(),
            },
            NetworkEvent::MessageSent {
                connection_id,
                message_id: message.id,
            },
            NetworkEvent::ConnectionLost {
                connection_id,
                reason: "Timeout".to_string(),
            },
            NetworkEvent::NetworkError {
                error: "Connection refused".to_string(),
                context: {
                    let mut ctx = HashMap::new();
                    ctx.insert("address".to_string(), "192.168.1.100:8080".to_string());
                    ctx.insert("attempt".to_string(), "3".to_string());
                    ctx
                },
            },
        ]
    }
}

/// Health and observability fixtures
pub mod health {
    use super::*;

    /// Healthy status
    pub fn healthy_status() -> ObservableStatus {
        ObservableStatus {
            state: ObservableState::Running,
            last_updated: chrono::Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("uptime".to_string(), "3600".to_string());
                meta.insert("version".to_string(), "1.0.0".to_string());
                meta
            },
        }
    }

    /// Degraded status
    pub fn degraded_status() -> ObservableStatus {
        ObservableStatus {
            state: ObservableState::Degraded,
            last_updated: chrono::Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("reason".to_string(), "High memory usage".to_string());
                meta.insert("memory_usage".to_string(), "85%".to_string());
                meta
            },
        }
    }

    /// Failed status
    pub fn failed_status() -> ObservableStatus {
        ObservableStatus {
            state: ObservableState::Failed,
            last_updated: chrono::Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("error".to_string(), "Database connection failed".to_string());
                meta.insert("retry_count".to_string(), "3".to_string());
                meta
            },
        }
    }

    /// Comprehensive health status with multiple checks
    pub fn comprehensive_health_status() -> HealthStatus {
        let mut checks = HashMap::new();
        
        checks.insert("database".to_string(), HealthCheck {
            level: HealthLevel::Healthy,
            message: "Database connection active".to_string(),
            timestamp: chrono::Utc::now(),
        });
        
        checks.insert("memory".to_string(), HealthCheck {
            level: HealthLevel::Warning,
            message: "Memory usage at 75%".to_string(),
            timestamp: chrono::Utc::now(),
        });
        
        checks.insert("disk".to_string(), HealthCheck {
            level: HealthLevel::Healthy,
            message: "Disk usage at 45%".to_string(),
            timestamp: chrono::Utc::now(),
        });
        
        checks.insert("network".to_string(), HealthCheck {
            level: HealthLevel::Critical,
            message: "High network latency detected".to_string(),
            timestamp: chrono::Utc::now(),
        });

        HealthStatus {
            overall: HealthLevel::Warning,
            checks,
            last_check: chrono::Utc::now(),
        }
    }

    /// Sample metrics data
    pub fn sample_metrics() -> HashMap<String, f64> {
        let mut metrics = HashMap::new();
        metrics.insert("cpu_usage_percent".to_string(), 45.5);
        metrics.insert("memory_usage_bytes".to_string(), 1_073_741_824.0); // 1GB
        metrics.insert("network_bytes_sent".to_string(), 104_857_600.0); // 100MB
        metrics.insert("network_bytes_received".to_string(), 209_715_200.0); // 200MB
        metrics.insert("connections_active".to_string(), 25.0);
        metrics.insert("requests_per_second".to_string(), 150.0);
        metrics.insert("response_time_ms".to_string(), 50.0);
        metrics.insert("error_rate_percent".to_string(), 0.1);
        metrics.insert("uptime_seconds".to_string(), 86400.0); // 1 day
        metrics
    }
}

/// Error fixtures
pub mod errors {
    use super::*;
    use synapsed_core::error::SynapsedError;

    /// Common configuration errors
    pub fn config_errors() -> Vec<SynapsedError> {
        vec![
            SynapsedError::config("Missing required field 'host'"),
            SynapsedError::config("Invalid port number: not-a-number"),
            SynapsedError::config("Configuration file not found"),
            SynapsedError::config("Invalid TOML syntax on line 5"),
            SynapsedError::config("Environment variable 'DATABASE_URL' not set"),
        ]
    }

    /// Common network errors
    pub fn network_errors() -> Vec<SynapsedError> {
        vec![
            SynapsedError::network("Connection refused"),
            SynapsedError::network("DNS resolution failed"),
            SynapsedError::network("SSL handshake failed"),
            SynapsedError::network("Connection timeout"),
            SynapsedError::network("Network unreachable"),
        ]
    }

    /// Common validation errors
    pub fn validation_errors() -> Vec<SynapsedError> {
        vec![
            SynapsedError::invalid_input("Field 'name' cannot be empty"),
            SynapsedError::invalid_input("Invalid email format"),
            SynapsedError::invalid_input("Password must be at least 8 characters"),
            SynapsedError::invalid_input("Invalid UUID format"),
            SynapsedError::invalid_input("Value must be between 1 and 100"),
        ]
    }

    /// Authentication and authorization errors
    pub fn auth_errors() -> Vec<SynapsedError> {
        vec![
            SynapsedError::auth("Invalid credentials"),
            SynapsedError::auth("Token expired"),
            SynapsedError::auth("Insufficient permissions"),
            SynapsedError::auth("Account locked"),
            SynapsedError::auth("Two-factor authentication required"),
        ]
    }

    /// Internal system errors
    pub fn internal_errors() -> Vec<SynapsedError> {
        vec![
            SynapsedError::internal("Database connection pool exhausted"),
            SynapsedError::internal("Out of memory"),
            SynapsedError::internal("Disk full"),
            SynapsedError::internal("Service unavailable"),
            SynapsedError::internal("Unexpected panic in worker thread"),
        ]
    }

    /// All error types
    pub fn all_errors() -> Vec<SynapsedError> {
        let mut errors = Vec::new();
        errors.extend(config_errors());
        errors.extend(network_errors());
        errors.extend(validation_errors());
        errors.extend(auth_errors());
        errors.extend(internal_errors());
        errors
    }
}

/// File content fixtures
pub mod files {
    /// Sample TOML configuration file content
    pub const SAMPLE_TOML: &str = r#"
[database]
host = "localhost"
port = 5432
username = "synapsed"
password = "secret"
ssl = true
max_connections = 20

[network]
listen_address = "0.0.0.0:8080"
max_connections = 1000
timeout_seconds = 30
enable_tls = false
certificate_path = "/etc/ssl/certs/synapsed.crt"
private_key_path = "/etc/ssl/private/synapsed.key"

[logging]
level = "info"
format = "json"
output = "stdout"
enable_tracing = true
trace_sample_rate = 0.1

[metrics]
enable = true
endpoint = "/metrics"
namespace = "synapsed"
push_gateway = "http://prometheus:9091"

[features]
debug_mode = false
experimental_features = false
enable_admin_api = true
"#;

    /// Sample JSON configuration file content
    pub const SAMPLE_JSON: &str = r#"{
  "database": {
    "host": "localhost",
    "port": 5432,
    "username": "synapsed",
    "password": "secret",
    "ssl": true,
    "max_connections": 20
  },
  "network": {
    "listen_address": "0.0.0.0:8080",
    "max_connections": 1000,
    "timeout_seconds": 30,
    "enable_tls": false,
    "certificate_path": "/etc/ssl/certs/synapsed.crt",
    "private_key_path": "/etc/ssl/private/synapsed.key"
  },
  "logging": {
    "level": "info",
    "format": "json",
    "output": "stdout",
    "enable_tracing": true,
    "trace_sample_rate": 0.1
  },
  "metrics": {
    "enable": true,
    "endpoint": "/metrics",
    "namespace": "synapsed",
    "push_gateway": "http://prometheus:9091"
  },
  "features": {
    "debug_mode": false,
    "experimental_features": false,
    "enable_admin_api": true
  }
}"#;

    /// Invalid TOML content
    pub const INVALID_TOML: &str = r#"
[database
host = "localhost"
port = "not-a-number"
ssl = maybe
"#;

    /// Invalid JSON content
    pub const INVALID_JSON: &str = r#"{
  "database": {
    "host": "localhost",
    "port": "not-a-number",
    "ssl": "maybe",
  }
"#;

    /// Minimal configuration
    pub const MINIMAL_CONFIG: &str = r#"
[core]
name = "synapsed-test"
version = "1.0.0"
"#;
}

#[cfg(test)]
mod tests {
    use super::*;
    use synapsed_core::error::SynapsedError;

    #[test]
    fn test_config_fixtures() {
        let db_config = config::database_config();
        match db_config {
            ConfigValue::Object(map) => {
                assert!(map.contains_key("host"));
                assert!(map.contains_key("port"));
            }
            _ => panic!("Expected object"),
        }

        let complete_config = config::complete_app_config();
        match complete_config {
            ConfigValue::Object(map) => {
                assert!(map.contains_key("database"));
                assert!(map.contains_key("network"));
                assert!(map.contains_key("logging"));
            }
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn test_network_fixtures() {
        let addresses = network::all_address_types();
        assert!(!addresses.is_empty());
        
        let messages = network::sample_messages();
        assert!(!messages.is_empty());
        assert!(messages.iter().any(|m| m.message_type == "ping"));
        
        let events = network::sample_events();
        assert!(!events.is_empty());
    }

    #[test]
    fn test_health_fixtures() {
        let healthy = health::healthy_status();
        assert_eq!(healthy.state, ObservableState::Running);
        
        let comprehensive = health::comprehensive_health_status();
        assert!(!comprehensive.checks.is_empty());
        assert!(comprehensive.checks.contains_key("database"));
        
        let metrics = health::sample_metrics();
        assert!(!metrics.is_empty());
        assert!(metrics.contains_key("cpu_usage_percent"));
    }

    #[test]
    fn test_error_fixtures() {
        let config_errs = errors::config_errors();
        assert!(!config_errs.is_empty());
        assert!(config_errs.iter().all(|e| matches!(e, SynapsedError::Configuration(_))));
        
        let network_errs = errors::network_errors();
        assert!(!network_errs.is_empty());
        assert!(network_errs.iter().all(|e| matches!(e, SynapsedError::Network(_))));
        
        let all_errs = errors::all_errors();
        assert!(all_errs.len() > config_errs.len());
    }

    #[test]
    fn test_file_fixtures() {
        assert!(files::SAMPLE_TOML.contains("[database]"));
        assert!(files::SAMPLE_JSON.contains("\"database\""));
        assert!(files::INVALID_TOML.contains("[database"));
        assert!(files::INVALID_JSON.contains("\"port\": \"not-a-number\","));
    }
}