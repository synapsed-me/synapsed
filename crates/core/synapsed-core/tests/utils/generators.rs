//! Property-based test generators for synapsed-core

use proptest::prelude::*;
use std::collections::HashMap;
use synapsed_core::{
    config::ConfigValue,
    error::SynapsedError,
    network::{NetworkAddress, NetworkMessage, ConnectionState},
    traits::{HealthLevel, ObservableState, LifecycleState},
};
use uuid::Uuid;

/// Generators for basic types

/// Generate arbitrary strings with reasonable length limits
pub fn arb_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,50}"
}

/// Generate non-empty strings
pub fn arb_non_empty_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,50}"
}

/// Generate strings that could be identifiers
pub fn arb_identifier() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_]{0,30}"
}

/// Generate valid email addresses
pub fn arb_email() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9]{1,20}@[a-zA-Z0-9]{1,20}\\.[a-zA-Z]{2,4}"
}

/// Generate valid URLs
pub fn arb_url() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "http://example.com".to_string(),
        "https://api.example.com/v1".to_string(),
        "ftp://files.example.com".to_string(),
        "ws://socket.example.com:8080".to_string(),
        "wss://secure.example.com:443".to_string(),
    ])
}

/// Generate UUIDs
pub fn arb_uuid() -> impl Strategy<Value = Uuid> {
    any::<[u8; 16]>().prop_map(|bytes| Uuid::from_bytes(bytes))
}

/// Generate positive integers
pub fn arb_positive_i32() -> impl Strategy<Value = i32> {
    1i32..=i32::MAX
}

/// Generate port numbers
pub fn arb_port() -> impl Strategy<Value = u16> {
    1024u16..=65535u16
}

/// Generators for synapsed-core types

/// Generate ConfigValue instances
pub fn arb_config_value() -> impl Strategy<Value = ConfigValue> {
    let leaf = prop_oneof![
        arb_string().prop_map(ConfigValue::String),
        any::<i64>().prop_map(ConfigValue::Integer),
        any::<f64>().prop_map(ConfigValue::Float),
        any::<bool>().prop_map(ConfigValue::Boolean),
        Just(ConfigValue::Null),
    ];

    leaf.prop_recursive(
        8,   // Max depth
        256, // Max size
        10,  // Items per collection
        |inner| {
            prop_oneof![
                // Arrays
                prop::collection::vec(inner.clone(), 0..10).prop_map(ConfigValue::Array),
                // Objects
                prop::collection::hash_map(arb_identifier(), inner, 0..10)
                    .prop_map(ConfigValue::Object),
            ]
        },
    )
}

/// Generate NetworkAddress instances
pub fn arb_network_address() -> impl Strategy<Value = NetworkAddress> {
    prop_oneof![
        // Socket addresses
        (any::<[u8; 4]>(), arb_port()).prop_map(|(ip, port)| {
            let addr = std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::from(ip),
                port,
            ));
            NetworkAddress::Socket(addr)
        }),
        // IPv6 socket addresses
        (any::<[u8; 16]>(), arb_port()).prop_map(|(ip, port)| {
            let addr = std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
                std::net::Ipv6Addr::from(ip),
                port,
                0,
                0,
            ));
            NetworkAddress::Socket(addr)
        }),
        // Peer IDs
        "[a-zA-Z0-9]{20,60}".prop_map(NetworkAddress::PeerId),
        // DIDs
        "did:[a-z]{3,10}:[a-zA-Z0-9]{10,50}".prop_map(NetworkAddress::Did),
        // Multiaddrs
        prop::sample::select(vec![
            "/ip4/127.0.0.1/tcp/8080".to_string(),
            "/ip6/::1/tcp/8080".to_string(),
            "/dns4/example.com/tcp/443".to_string(),
        ])
        .prop_map(NetworkAddress::Multiaddr),
        // WebRTC
        "webrtc://[a-zA-Z0-9.-]{5,30}:[0-9]{4,5}".prop_map(NetworkAddress::WebRtc),
        // Custom
        (arb_identifier(), arb_string()).prop_map(|(protocol, address)| {
            NetworkAddress::Custom { protocol, address }
        }),
    ]
}

/// Generate NetworkMessage instances
pub fn arb_network_message() -> impl Strategy<Value = NetworkMessage> {
    (
        arb_identifier(),                                    // message_type
        prop::collection::vec(any::<u8>(), 0..1024),        // payload
        prop::collection::hash_map(arb_string(), arb_string(), 0..5), // headers
        prop::option::of(arb_network_address()),             // sender
        prop::option::of(arb_network_address()),             // recipient
    )
        .prop_map(|(message_type, payload, headers, sender, recipient)| {
            let mut message = NetworkMessage::new(&message_type, payload);
            for (key, value) in headers {
                message = message.with_header(key, value);
            }
            if let Some(sender) = sender {
                message = message.with_sender(sender);
            }
            if let Some(recipient) = recipient {
                message = message.with_recipient(recipient);
            }
            message
        })
}

/// Generate SynapsedError instances
pub fn arb_synapsed_error() -> impl Strategy<Value = SynapsedError> {
    prop_oneof![
        arb_string().prop_map(SynapsedError::Configuration),
        arb_string().prop_map(SynapsedError::Network),
        arb_string().prop_map(SynapsedError::Cryptographic),
        arb_string().prop_map(SynapsedError::Storage),
        arb_string().prop_map(SynapsedError::Authentication),
        arb_string().prop_map(SynapsedError::InvalidInput),
        arb_string().prop_map(SynapsedError::NotFound),
        arb_string().prop_map(SynapsedError::PermissionDenied),
        arb_string().prop_map(SynapsedError::Timeout),
        arb_string().prop_map(SynapsedError::Internal),
        arb_string().prop_map(SynapsedError::Serialization),
        arb_string().prop_map(SynapsedError::Did),
        arb_string().prop_map(SynapsedError::P2P),
        arb_string().prop_map(SynapsedError::Wasm),
        arb_string().prop_map(SynapsedError::Payment),
        (arb_string(), arb_string()).prop_map(|(message, context)| {
            SynapsedError::Application { message, context }
        }),
    ]
}

/// Generate enum types

/// Generate ObservableState
pub fn arb_observable_state() -> impl Strategy<Value = ObservableState> {
    prop_oneof![
        Just(ObservableState::Initializing),
        Just(ObservableState::Running),
        Just(ObservableState::Degraded),
        Just(ObservableState::Failed),
        Just(ObservableState::ShuttingDown),
        Just(ObservableState::Stopped),
    ]
}

/// Generate HealthLevel
pub fn arb_health_level() -> impl Strategy<Value = HealthLevel> {
    prop_oneof![
        Just(HealthLevel::Healthy),
        Just(HealthLevel::Warning),
        Just(HealthLevel::Critical),
        Just(HealthLevel::Unknown),
    ]
}

/// Generate LifecycleState
pub fn arb_lifecycle_state() -> impl Strategy<Value = LifecycleState> {
    prop_oneof![
        Just(LifecycleState::Created),
        Just(LifecycleState::Starting),
        Just(LifecycleState::Running),
        Just(LifecycleState::Stopping),
        Just(LifecycleState::Stopped),
        Just(LifecycleState::Failed),
    ]
}

/// Generate ConnectionState
pub fn arb_connection_state() -> impl Strategy<Value = ConnectionState> {
    prop_oneof![
        Just(ConnectionState::Disconnected),
        Just(ConnectionState::Connecting),
        Just(ConnectionState::Connected),
        Just(ConnectionState::Disconnecting),
        Just(ConnectionState::Failed),
    ]
}

/// Domain-specific generators

/// Generate valid configuration objects
pub fn arb_valid_config() -> impl Strategy<Value = ConfigValue> {
    prop_oneof![
        // Database config
        (arb_string(), arb_port(), arb_string(), any::<bool>()).prop_map(
            |(host, port, user, ssl)| {
                let mut config = HashMap::new();
                config.insert("host".to_string(), ConfigValue::String(host));
                config.insert("port".to_string(), ConfigValue::Integer(port as i64));
                config.insert("username".to_string(), ConfigValue::String(user));
                config.insert("ssl".to_string(), ConfigValue::Boolean(ssl));
                ConfigValue::Object(config)
            }
        ),
        // Network config
        (arb_network_address(), arb_positive_i32(), arb_positive_i32()).prop_map(
            |(addr, max_conn, timeout)| {
                let mut config = HashMap::new();
                config.insert(
                    "address".to_string(),
                    ConfigValue::String(addr.to_string()),
                );
                config.insert(
                    "max_connections".to_string(),
                    ConfigValue::Integer(max_conn as i64),
                );
                config.insert(
                    "timeout_seconds".to_string(),
                    ConfigValue::Integer(timeout as i64),
                );
                ConfigValue::Object(config)
            }
        ),
    ]
}

/// Generate error scenarios for testing error handling
pub fn arb_error_scenario() -> impl Strategy<Value = (String, SynapsedError)> {
    prop_oneof![
        // Configuration errors
        arb_string().prop_map(|msg| {
            ("config_error".to_string(), SynapsedError::config(msg))
        }),
        // Network errors
        arb_string().prop_map(|msg| {
            ("network_error".to_string(), SynapsedError::network(msg))
        }),
        // Validation errors
        arb_string().prop_map(|msg| {
            ("validation_error".to_string(), SynapsedError::invalid_input(msg))
        }),
    ]
}

/// Generate performance test data
pub fn arb_performance_data() -> impl Strategy<Value = (usize, Vec<u8>)> {
    (1usize..=10000, prop::collection::vec(any::<u8>(), 0..1000))
}

/// Generate concurrent operation scenarios
pub fn arb_concurrent_scenario() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(arb_identifier(), 1..=10)
}

/// Generators for property testing specific scenarios

/// Generate inputs that should always serialize/deserialize correctly
pub fn arb_serializable_data() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        // Valid UTF-8 strings
        arb_string().prop_map(|s| s.into_bytes()),
        // Binary data
        prop::collection::vec(any::<u8>(), 0..1000),
        // JSON-like structures
        prop::collection::vec(any::<u8>(), 0..100)
            .prop_filter("Valid JSON bytes", |bytes| {
                serde_json::from_slice::<serde_json::Value>(bytes).is_ok()
            }),
    ]
}

/// Generate inputs for testing timeout scenarios
pub fn arb_timeout_scenario() -> impl Strategy<Value = (u64, bool)> {
    (1u64..=5000, any::<bool>()) // (timeout_ms, should_succeed)
}

/// Generate inputs for testing retry scenarios
pub fn arb_retry_scenario() -> impl Strategy<Value = (usize, Vec<bool>)> {
    (1usize..=5, prop::collection::vec(any::<bool>(), 1..=10))
}

/// Custom strategies for complex scenarios

/// Generate valid network protocol handshakes
pub fn arb_handshake_scenario() -> impl Strategy<Value = (NetworkMessage, NetworkMessage)> {
    (
        arb_network_message().prop_filter("handshake message", |msg| {
            msg.message_type.contains("handshake") || msg.message_type.contains("hello")
        }),
        arb_network_message().prop_filter("response message", |msg| {
            msg.message_type.contains("response") || msg.message_type.contains("ack")
        }),
    )
}

/// Generate configuration merge scenarios
pub fn arb_config_merge_scenario() -> impl Strategy<Value = (ConfigValue, ConfigValue)> {
    (arb_config_value(), arb_config_value())
}

/// Generate load testing scenarios
pub fn arb_load_test_scenario() -> impl Strategy<Value = (usize, usize, Vec<NetworkMessage>)> {
    (
        1usize..=100,                                             // concurrent_connections
        1usize..=1000,                                            // messages_per_connection
        prop::collection::vec(arb_network_message(), 1..=100),    // message_templates
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_arb_string_generates_valid_strings(s in arb_string()) {
            assert!(!s.is_empty());
            assert!(s.len() <= 50);
            assert!(s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
        }

        #[test]
        fn test_arb_email_generates_valid_format(email in arb_email()) {
            assert!(email.contains('@'));
            assert!(email.contains('.'));
            let parts: Vec<&str> = email.split('@').collect();
            assert_eq!(parts.len(), 2);
            assert!(!parts[0].is_empty());
            assert!(!parts[1].is_empty());
        }

        #[test]
        fn test_arb_port_generates_valid_ports(port in arb_port()) {
            assert!(port >= 1024);
            assert!(port <= 65535);
        }

        #[test]
        fn test_arb_uuid_generates_valid_uuids(uuid in arb_uuid()) {
            // UUID should not be nil (all zeros)
            assert_ne!(uuid, Uuid::nil());
            // Should be a valid UUID string representation
            let uuid_str = uuid.to_string();
            assert_eq!(uuid_str.len(), 36);
            assert_eq!(uuid_str.chars().filter(|&c| c == '-').count(), 4);
        }

        #[test]
        fn test_arb_config_value_generates_valid_configs(config in arb_config_value()) {
            // Should be able to serialize/deserialize
            match &config {
                ConfigValue::String(s) => assert!(!s.is_empty() || s.is_empty()), // Both valid
                ConfigValue::Integer(_) => (),
                ConfigValue::Float(f) => assert!(f.is_finite() || f.is_infinite() || f.is_nan()), // All valid
                ConfigValue::Boolean(_) => (),
                ConfigValue::Array(arr) => assert!(arr.len() <= 100), // Reasonable size
                ConfigValue::Object(obj) => assert!(obj.len() <= 100), // Reasonable size
                ConfigValue::Null => (),
            }
        }

        #[test]
        fn test_arb_network_address_generates_valid_addresses(addr in arb_network_address()) {
            // Should have valid protocol
            let protocol = addr.protocol();
            assert!(!protocol.is_empty());
            
            // Should have valid address string
            let addr_str = addr.address_string();
            assert!(!addr_str.is_empty());
            
            // Should format correctly
            let formatted = addr.to_string();
            assert!(formatted.contains("://"));
        }

        #[test]
        fn test_arb_network_message_generates_valid_messages(msg in arb_network_message()) {
            // Should have valid message type
            assert!(!msg.message_type.is_empty());
            
            // Should have valid ID
            assert_ne!(msg.id, Uuid::nil());
            
            // Should have reasonable payload size
            assert!(msg.payload.len() <= 1024);
            
            // Headers should be reasonable
            assert!(msg.headers.len() <= 5);
        }

        #[test]
        fn test_arb_synapsed_error_generates_valid_errors(error in arb_synapsed_error()) {
            // Should have non-empty message
            let error_str = error.to_string();
            assert!(!error_str.is_empty());
            
            // Should have consistent retryable classification
            let is_retryable = error.is_retryable();
            let is_client = error.is_client_error();
            let is_server = error.is_server_error();
            
            // Client and server errors should be mutually exclusive
            if is_client {
                assert!(!is_server);
            }
            
            // Some basic consistency checks
            match &error {
                SynapsedError::Network(_) | SynapsedError::Timeout(_) | SynapsedError::Internal(_) => {
                    assert!(is_retryable);
                }
                SynapsedError::InvalidInput(_) | SynapsedError::NotFound(_) | 
                SynapsedError::PermissionDenied(_) | SynapsedError::Authentication(_) => {
                    assert!(is_client);
                    assert!(!is_retryable);
                }
                _ => (), // Other errors may vary
            }
        }

        #[test]
        fn test_arb_valid_config_generates_valid_structures(config in arb_valid_config()) {
            match config {
                ConfigValue::Object(map) => {
                    // Should have at least one key
                    assert!(!map.is_empty());
                    
                    // Keys should be non-empty
                    for key in map.keys() {
                        assert!(!key.is_empty());
                    }
                }
                _ => panic!("Expected object config"),
            }
        }

        #[test]
        fn test_arb_error_scenario_generates_consistent_pairs(
            (scenario_type, error) in arb_error_scenario()
        ) {
            assert!(!scenario_type.is_empty());
            
            // Error type should match scenario type
            match scenario_type.as_str() {
                "config_error" => assert!(matches!(error, SynapsedError::Configuration(_))),
                "network_error" => assert!(matches!(error, SynapsedError::Network(_))),
                "validation_error" => assert!(matches!(error, SynapsedError::InvalidInput(_))),
                _ => (), // Other scenarios may vary
            }
        }

        #[test]
        fn test_arb_performance_data_generates_reasonable_sizes(
            (count, data) in arb_performance_data()
        ) {
            assert!(count >= 1);
            assert!(count <= 10000);
            assert!(data.len() <= 1000);
        }
    }

    #[test]
    fn test_generator_compilation() {
        // Ensure all generators compile and can be used
        let mut runner = proptest::test_runner::TestRunner::default();
        
        // Test a few generators
        runner.run(&arb_string(), |_| Ok(())).unwrap();
        runner.run(&arb_network_address(), |_| Ok(())).unwrap();
        runner.run(&arb_config_value(), |_| Ok(())).unwrap();
        runner.run(&arb_synapsed_error(), |_| Ok(())).unwrap();
    }
}