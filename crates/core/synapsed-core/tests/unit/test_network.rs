//! Unit tests for network module

use std::collections::HashMap;
use std::str::FromStr;
use synapsed_core::{
    error::SynapsedError,
    network::*,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_address_socket() {
        let addr_v4 = NetworkAddress::Socket("127.0.0.1:8080".parse().unwrap());
        assert_eq!(addr_v4.protocol(), "tcp");
        assert_eq!(addr_v4.address_string(), "127.0.0.1:8080");
        assert_eq!(addr_v4.to_string(), "tcp://127.0.0.1:8080");

        let addr_v6 = NetworkAddress::Socket("[::1]:8080".parse().unwrap());
        assert_eq!(addr_v6.protocol(), "tcp");
        assert_eq!(addr_v6.address_string(), "[::1]:8080");
        assert_eq!(addr_v6.to_string(), "tcp://[::1]:8080");
    }

    #[test]
    fn test_network_address_peer_id() {
        let peer_id = "12D3KooWBhV9dP6FDvFzRK8f2nbDXDDZ7NZ8xJ3Q4k5v7W8X9Y2A";
        let addr = NetworkAddress::PeerId(peer_id.to_string());
        assert_eq!(addr.protocol(), "p2p");
        assert_eq!(addr.address_string(), peer_id);
        assert_eq!(addr.to_string(), format!("p2p://{}", peer_id));
    }

    #[test]
    fn test_network_address_did() {
        let did = "did:example:alice";
        let addr = NetworkAddress::Did(did.to_string());
        assert_eq!(addr.protocol(), "did");
        assert_eq!(addr.address_string(), did);
        assert_eq!(addr.to_string(), format!("did://{}", did));
    }

    #[test]
    fn test_network_address_multiaddr() {
        let multiaddr = "/ip4/127.0.0.1/tcp/8080";
        let addr = NetworkAddress::Multiaddr(multiaddr.to_string());
        assert_eq!(addr.protocol(), "multiaddr");
        assert_eq!(addr.address_string(), multiaddr);
        assert_eq!(addr.to_string(), format!("multiaddr://{}", multiaddr));
    }

    #[test]
    fn test_network_address_webrtc() {
        let webrtc = "stun:stun.l.google.com:19302";
        let addr = NetworkAddress::WebRtc(webrtc.to_string());
        assert_eq!(addr.protocol(), "webrtc");
        assert_eq!(addr.address_string(), webrtc);
        assert_eq!(addr.to_string(), format!("webrtc://{}", webrtc));
    }

    #[test]
    fn test_network_address_custom() {
        let addr = NetworkAddress::Custom {
            protocol: "ipc".to_string(),
            address: "/tmp/socket".to_string(),
        };
        assert_eq!(addr.protocol(), "ipc");
        assert_eq!(addr.address_string(), "/tmp/socket");
        assert_eq!(addr.to_string(), "ipc:///tmp/socket");
    }

    #[test]
    fn test_network_address_parsing() {
        // Test socket address parsing
        let addr = NetworkAddress::from_str("127.0.0.1:8080").unwrap();
        assert!(matches!(addr, NetworkAddress::Socket(_)));

        let addr = NetworkAddress::from_str("tcp://192.168.1.1:9090").unwrap();
        assert!(matches!(addr, NetworkAddress::Socket(_)));

        let addr = NetworkAddress::from_str("udp://0.0.0.0:8080").unwrap();
        assert!(matches!(addr, NetworkAddress::Socket(_)));

        // Test P2P address parsing
        let addr = NetworkAddress::from_str("p2p://12D3KooW123").unwrap();
        assert!(matches!(addr, NetworkAddress::PeerId(_)));

        // Test DID parsing
        let addr = NetworkAddress::from_str("did://did:example:alice").unwrap();
        assert!(matches!(addr, NetworkAddress::Did(_)));

        // Test multiaddr parsing
        let addr = NetworkAddress::from_str("multiaddr:///ip4/127.0.0.1/tcp/8080").unwrap();
        assert!(matches!(addr, NetworkAddress::Multiaddr(_)));

        // Test WebRTC parsing
        let addr = NetworkAddress::from_str("webrtc://stun:example.com:3478").unwrap();
        assert!(matches!(addr, NetworkAddress::WebRtc(_)));

        // Test custom protocol parsing
        let addr = NetworkAddress::from_str("custom://some-address").unwrap();
        assert!(matches!(addr, NetworkAddress::Custom { .. }));

        // Test invalid formats
        assert!(NetworkAddress::from_str("invalid-format").is_err());
        assert!(NetworkAddress::from_str("tcp://invalid-port").is_err());
    }

    #[test]
    fn test_connection_state() {
        let states = vec![
            ConnectionState::Disconnected,
            ConnectionState::Connecting,
            ConnectionState::Connected,
            ConnectionState::Disconnecting,
            ConnectionState::Failed,
        ];

        for state in states {
            // Test serialization
            let serialized = serde_json::to_string(&state).unwrap();
            let deserialized: ConnectionState = serde_json::from_str(&serialized).unwrap();
            assert_eq!(state, deserialized);

            // Test cloning and equality
            assert_eq!(state, state.clone());
        }
    }

    #[test]
    fn test_connection_metadata() {
        let local_addr = NetworkAddress::Socket("127.0.0.1:8080".parse().unwrap());
        let remote_addr = NetworkAddress::Socket("192.168.1.100:9090".parse().unwrap());
        
        let metadata = ConnectionMetadata {
            id: uuid::Uuid::new_v4(),
            local_address: local_addr.clone(),
            remote_address: remote_addr.clone(),
            state: ConnectionState::Connected,
            connected_at: Some(chrono::Utc::now()),
            last_activity: chrono::Utc::now(),
            protocol_version: "1.0".to_string(),
            metadata: {
                let mut map = HashMap::new();
                map.insert("connection_type".to_string(), "tcp".to_string());
                map
            },
        };

        assert!(!metadata.id.is_nil());
        assert_eq!(metadata.state, ConnectionState::Connected);
        assert!(metadata.connected_at.is_some());
        assert_eq!(metadata.protocol_version, "1.0");
        assert!(metadata.metadata.contains_key("connection_type"));

        // Test serialization
        let serialized = serde_json::to_string(&metadata).unwrap();
        let deserialized: ConnectionMetadata = serde_json::from_str(&serialized).unwrap();
        assert_eq!(metadata.id, deserialized.id);
        assert_eq!(metadata.state, deserialized.state);
    }

    #[test]
    fn test_network_message() {
        let payload = b"Hello, World!".to_vec();
        let message = NetworkMessage::new("greeting", payload.clone());

        assert!(!message.id.is_nil());
        assert_eq!(message.message_type, "greeting");
        assert_eq!(message.payload, payload);
        assert_eq!(message.payload_size(), payload.len());
        assert!(message.headers.is_empty());
        assert!(message.sender.is_none());
        assert!(message.recipient.is_none());

        // Test builder pattern
        let sender = NetworkAddress::PeerId("sender123".to_string());
        let recipient = NetworkAddress::PeerId("recipient456".to_string());
        
        let enhanced_message = NetworkMessage::new("test", vec![1, 2, 3])
            .with_header("content-type", "application/octet-stream")
            .with_header("version", "1.0")
            .with_sender(sender.clone())
            .with_recipient(recipient.clone());

        assert_eq!(enhanced_message.get_header("content-type"), Some("application/octet-stream"));
        assert_eq!(enhanced_message.get_header("version"), Some("1.0"));
        assert_eq!(enhanced_message.get_header("nonexistent"), None);
        assert_eq!(enhanced_message.sender, Some(sender));
        assert_eq!(enhanced_message.recipient, Some(recipient));
    }

    #[test]
    fn test_network_message_serialization() {
        let message = NetworkMessage::new("test.message", vec![1, 2, 3, 4])
            .with_header("test-header", "test-value")
            .with_sender(NetworkAddress::PeerId("sender".to_string()));

        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: NetworkMessage = serde_json::from_str(&serialized).unwrap();

        assert_eq!(message.id, deserialized.id);
        assert_eq!(message.message_type, deserialized.message_type);
        assert_eq!(message.payload, deserialized.payload);
        assert_eq!(message.headers, deserialized.headers);
        assert_eq!(message.sender, deserialized.sender);
    }

    #[test]
    fn test_network_stats() {
        let mut stats = NetworkStats::new();

        // Test initial state
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert_eq!(stats.connection_count, 0);
        assert_eq!(stats.error_count, 0);
        assert!(stats.last_error.is_none());
        assert_eq!(stats.uptime_seconds, 0);

        // Test recording operations
        stats.record_bytes_sent(1024);
        stats.record_bytes_received(2048);
        stats.record_message_sent();
        stats.record_message_received();
        stats.record_connection();
        stats.record_error("test error");
        stats.update_uptime(60);

        assert_eq!(stats.bytes_sent, 1024);
        assert_eq!(stats.bytes_received, 2048);
        assert_eq!(stats.messages_sent, 1);
        assert_eq!(stats.messages_received, 1);
        assert_eq!(stats.connection_count, 1);
        assert_eq!(stats.error_count, 1);
        assert_eq!(stats.last_error, Some("test error".to_string()));
        assert_eq!(stats.uptime_seconds, 60);

        // Test rate calculations
        assert_eq!(stats.throughput(), (1024.0 + 2048.0) / 60.0);
        assert_eq!(stats.message_rate(), (1.0 + 1.0) / 60.0);
        assert_eq!(stats.error_rate(), 1.0 / 60.0);

        // Test disconnection
        stats.record_disconnection();
        assert_eq!(stats.connection_count, 0);

        // Test multiple disconnections don't go negative
        stats.record_disconnection();
        assert_eq!(stats.connection_count, 0);
    }

    #[test]
    fn test_network_stats_edge_cases() {
        let mut stats = NetworkStats::new();

        // Test rate calculations with zero uptime
        assert_eq!(stats.throughput(), 0.0);
        assert_eq!(stats.message_rate(), 0.0);
        assert_eq!(stats.error_rate(), 0.0);

        // Test with some data but zero uptime
        stats.record_bytes_sent(1000);
        stats.record_message_sent();
        stats.record_error("error");
        
        assert_eq!(stats.throughput(), 0.0);
        assert_eq!(stats.message_rate(), 0.0);
        assert_eq!(stats.error_rate(), 0.0);
    }

    #[test]
    fn test_network_event() {
        let connection_id = uuid::Uuid::new_v4();
        let remote_address = NetworkAddress::Socket("192.168.1.100:8080".parse().unwrap());
        let message = NetworkMessage::new("test", vec![1, 2, 3]);
        let message_id = message.id;

        // Test ConnectionEstablished event
        let event = NetworkEvent::ConnectionEstablished {
            connection_id,
            remote_address: remote_address.clone(),
        };

        match event {
            NetworkEvent::ConnectionEstablished { connection_id: id, remote_address: addr } => {
                assert_eq!(id, connection_id);
                assert_eq!(addr, remote_address);
            }
            _ => panic!("Expected ConnectionEstablished event"),
        }

        // Test MessageReceived event
        let event = NetworkEvent::MessageReceived {
            connection_id,
            message: message.clone(),
        };

        match event {
            NetworkEvent::MessageReceived { connection_id: id, message: msg } => {
                assert_eq!(id, connection_id);
                assert_eq!(msg.id, message.id);
            }
            _ => panic!("Expected MessageReceived event"),
        }

        // Test MessageSent event
        let event = NetworkEvent::MessageSent {
            connection_id,
            message_id,
        };

        match event {
            NetworkEvent::MessageSent { connection_id: id, message_id: msg_id } => {
                assert_eq!(id, connection_id);
                assert_eq!(msg_id, message_id);
            }
            _ => panic!("Expected MessageSent event"),
        }

        // Test ConnectionLost event
        let event = NetworkEvent::ConnectionLost {
            connection_id,
            reason: "Timeout".to_string(),
        };

        match event {
            NetworkEvent::ConnectionLost { connection_id: id, reason } => {
                assert_eq!(id, connection_id);
                assert_eq!(reason, "Timeout");
            }
            _ => panic!("Expected ConnectionLost event"),
        }

        // Test NetworkError event
        let mut context = HashMap::new();
        context.insert("address".to_string(), "192.168.1.100:8080".to_string());
        context.insert("attempt".to_string(), "3".to_string());

        let event = NetworkEvent::NetworkError {
            error: "Connection refused".to_string(),
            context: context.clone(),
        };

        match event {
            NetworkEvent::NetworkError { error, context: ctx } => {
                assert_eq!(error, "Connection refused");
                assert_eq!(ctx, context);
            }
            _ => panic!("Expected NetworkError event"),
        }
    }

    #[test]
    fn test_network_event_serialization() {
        let event = NetworkEvent::ConnectionEstablished {
            connection_id: uuid::Uuid::new_v4(),
            remote_address: NetworkAddress::PeerId("test-peer".to_string()),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: NetworkEvent = serde_json::from_str(&serialized).unwrap();

        match (event, deserialized) {
            (
                NetworkEvent::ConnectionEstablished { connection_id: id1, remote_address: addr1 },
                NetworkEvent::ConnectionEstablished { connection_id: id2, remote_address: addr2 }
            ) => {
                assert_eq!(id1, id2);
                assert_eq!(addr1, addr2);
            }
            _ => panic!("Serialization/deserialization mismatch"),
        }
    }

    #[test]
    fn test_network_address_equality_and_hashing() {
        use std::collections::HashSet;

        let addr1 = NetworkAddress::Socket("127.0.0.1:8080".parse().unwrap());
        let addr2 = NetworkAddress::Socket("127.0.0.1:8080".parse().unwrap());
        let addr3 = NetworkAddress::Socket("127.0.0.1:9090".parse().unwrap());

        // Test equality
        assert_eq!(addr1, addr2);
        assert_ne!(addr1, addr3);

        // Test hashing (can be used in HashSet/HashMap)
        let mut set = HashSet::new();
        set.insert(addr1.clone());
        set.insert(addr2.clone());
        set.insert(addr3.clone());

        assert_eq!(set.len(), 2); // addr1 and addr2 are equal, so only 2 unique items
        assert!(set.contains(&addr1));
        assert!(set.contains(&addr3));
    }

    #[test]
    fn test_network_types_debug_format() {
        let addr = NetworkAddress::PeerId("test-peer".to_string());
        let debug_str = format!("{:?}", addr);
        assert!(debug_str.contains("PeerId"));
        assert!(debug_str.contains("test-peer"));

        let state = ConnectionState::Connected;
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("Connected"));

        let message = NetworkMessage::new("debug-test", vec![1, 2, 3]);
        let debug_str = format!("{:?}", message);
        assert!(debug_str.contains("debug-test"));
        assert!(debug_str.contains("payload"));
    }

    #[test]
    fn test_network_types_clone() {
        let addr = NetworkAddress::Custom {
            protocol: "test".to_string(),
            address: "test-address".to_string(),
        };
        let cloned_addr = addr.clone();
        assert_eq!(addr, cloned_addr);

        let message = NetworkMessage::new("test", vec![1, 2, 3])
            .with_header("test", "value");
        let cloned_message = message.clone();
        assert_eq!(message.id, cloned_message.id);
        assert_eq!(message.headers, cloned_message.headers);

        let stats = NetworkStats::new();
        let cloned_stats = stats.clone();
        assert_eq!(stats.bytes_sent, cloned_stats.bytes_sent);
    }

    #[test]
    fn test_network_address_comprehensive_parsing() {
        // Test all supported formats
        let test_cases = vec![
            ("127.0.0.1:8080", NetworkAddress::Socket("127.0.0.1:8080".parse().unwrap())),
            ("tcp://192.168.1.1:9090", NetworkAddress::Socket("192.168.1.1:9090".parse().unwrap())),
            ("udp://0.0.0.0:8080", NetworkAddress::Socket("0.0.0.0:8080".parse().unwrap())),
            ("p2p://peer123", NetworkAddress::PeerId("peer123".to_string())),
            ("did://did:example:alice", NetworkAddress::Did("did:example:alice".to_string())),
            ("multiaddr:///ip4/127.0.0.1/tcp/8080", NetworkAddress::Multiaddr("/ip4/127.0.0.1/tcp/8080".to_string())),
            ("webrtc://stun:example.com:3478", NetworkAddress::WebRtc("stun:example.com:3478".to_string())),
            ("custom://address", NetworkAddress::Custom { 
                protocol: "custom".to_string(), 
                address: "address".to_string() 
            }),
        ];

        for (input, expected) in test_cases {
            let parsed = NetworkAddress::from_str(input).unwrap();
            assert_eq!(parsed, expected, "Failed to parse: {}", input);
        }

        // Test error cases
        let error_cases = vec![
            "invalid-format",
            "tcp://invalid-port",
            "://missing-protocol",
            "",
        ];

        for input in error_cases {
            assert!(NetworkAddress::from_str(input).is_err(), "Should fail to parse: {}", input);
        }
    }
}