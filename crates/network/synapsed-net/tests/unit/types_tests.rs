//! Unit tests for the types module.

use synapsed_net::types::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[cfg(test)]
mod peer_id_tests {
    use super::*;

    #[test]
    fn test_peer_id_creation() {
        let id1 = PeerId::new();
        let id2 = PeerId::new();
        
        // Each ID should be unique
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_peer_id_from_bytes() {
        let bytes = [1u8; 16];
        let id = PeerId::from_bytes(bytes);
        
        assert_eq!(id.as_bytes(), &bytes);
    }

    #[test]
    fn test_peer_id_anonymized() {
        let id = PeerId::from_bytes([0x12, 0x34, 0x56, 0x78, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let anonymized = id.anonymized();
        
        // Should start with peer_ and contain first two bytes in hex
        assert!(anonymized.starts_with("peer_1234"));
        assert!(anonymized.ends_with("****"));
    }

    #[test]
    fn test_peer_id_display() {
        let id = PeerId::new();
        let display = format!("{}", id);
        
        // Should only show first 8 characters
        assert_eq!(display.len(), 8);
    }

    #[test]
    fn test_peer_id_default() {
        let id1 = PeerId::default();
        let id2 = PeerId::default();
        
        // Default should create unique IDs
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_peer_id_serialization() {
        let id = PeerId::new();
        
        // Test JSON serialization
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: PeerId = serde_json::from_str(&json).unwrap();
        
        assert_eq!(id, deserialized);
    }
}

#[cfg(test)]
mod network_address_tests {
    use super::*;

    #[test]
    fn test_network_address_variants() {
        let socket = NetworkAddress::Socket(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080
        ));
        let quic = NetworkAddress::Quic(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8081
        ));
        let webrtc = NetworkAddress::WebRtc("signal.example.com".to_string());
        let libp2p = NetworkAddress::Libp2p("/ip4/127.0.0.1/tcp/4001".to_string());
        let tor = NetworkAddress::Tor("3g2upl4pq6kufc4m.onion".to_string());
        let i2p = NetworkAddress::I2p("example.i2p".to_string());
        
        // Test equality
        assert_eq!(socket, socket.clone());
        assert_ne!(socket, quic);
        
        // Test serialization
        for addr in [socket, quic, webrtc, libp2p, tor, i2p] {
            let json = serde_json::to_string(&addr).unwrap();
            let deserialized: NetworkAddress = serde_json::from_str(&json).unwrap();
            assert_eq!(addr, deserialized);
        }
    }
}

#[cfg(test)]
mod protocol_tests {
    use super::*;

    #[test]
    fn test_protocol_variants() {
        let protocols = vec![
            Protocol::Tcp,
            Protocol::Udp,
            Protocol::Quic,
            Protocol::WebRtc,
            Protocol::WebSocket,
            Protocol::Http,
            Protocol::Https,
            Protocol::Libp2p,
            Protocol::PostQuantum,
            Protocol::Custom("myprotocol".to_string()),
        ];
        
        // Test serialization
        for protocol in &protocols {
            let json = serde_json::to_string(protocol).unwrap();
            let deserialized: Protocol = serde_json::from_str(&json).unwrap();
            assert_eq!(protocol, &deserialized);
        }
    }
}

#[cfg(test)]
mod transport_type_tests {
    use super::*;

    #[test]
    fn test_transport_type_variants() {
        let types = vec![
            TransportType::Tcp,
            TransportType::Quic,
            TransportType::WebSocket,
            TransportType::WebRtc,
            TransportType::Udp,
            TransportType::Memory,
        ];
        
        // Test serialization
        for transport in &types {
            let json = serde_json::to_string(transport).unwrap();
            let deserialized: TransportType = serde_json::from_str(&json).unwrap();
            assert_eq!(transport, &deserialized);
        }
    }

    #[test]
    fn test_transport_type_display() {
        assert_eq!(format!("{}", TransportType::Tcp), "TCP");
        assert_eq!(format!("{}", TransportType::Quic), "QUIC");
        assert_eq!(format!("{}", TransportType::WebSocket), "WebSocket");
        assert_eq!(format!("{}", TransportType::Memory), "Memory");
    }
}

#[cfg(test)]
mod connection_id_tests {
    use super::*;

    #[test]
    fn test_connection_id_creation() {
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        
        // Each ID should be unique
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_connection_id_default() {
        let id1 = ConnectionId::default();
        let id2 = ConnectionId::default();
        
        // Default should create unique IDs
        assert_ne!(id1, id2);
    }
}

#[cfg(test)]
mod connection_metrics_tests {
    use super::*;

    #[test]
    fn test_connection_metrics_default() {
        let metrics = ConnectionMetrics::default();
        
        assert_eq!(metrics.bytes_sent, 0);
        assert_eq!(metrics.bytes_received, 0);
        assert_eq!(metrics.messages_sent, 0);
        assert_eq!(metrics.messages_received, 0);
        assert_eq!(metrics.avg_rtt, None);
        assert_eq!(metrics.packet_loss_rate, None);
    }

    #[test]
    fn test_connection_metrics_fields() {
        let mut metrics = ConnectionMetrics::default();
        
        // Update fields
        metrics.bytes_sent = 100;
        metrics.messages_sent = 1;
        metrics.bytes_received = 200;
        metrics.messages_received = 1;
        metrics.avg_rtt = Some(Duration::from_millis(50));
        metrics.packet_loss_rate = Some(0.01);
        
        assert_eq!(metrics.bytes_sent, 100);
        assert_eq!(metrics.messages_sent, 1);
        assert_eq!(metrics.bytes_received, 200);
        assert_eq!(metrics.messages_received, 1);
        assert_eq!(metrics.avg_rtt, Some(Duration::from_millis(50)));
        assert_eq!(metrics.packet_loss_rate, Some(0.01));
    }
}

#[cfg(test)]
mod message_tests {
    use super::*;

    #[test]
    fn test_message_id() {
        let id1 = MessageId::new();
        let id2 = MessageId::new();
        
        // Each ID should be unique
        assert_ne!(id1, id2);
        
        // Test default
        let id_default = MessageId::default();
        assert_ne!(id_default, id1);
        
        // Test serialization
        let json = serde_json::to_string(&id1).unwrap();
        let deserialized: MessageId = serde_json::from_str(&json).unwrap();
        assert_eq!(id1, deserialized);
    }

    #[test]
    fn test_message_priority() {
        let priorities = vec![
            MessagePriority::Low,
            MessagePriority::Normal,
            MessagePriority::High,
            MessagePriority::Critical,
        ];
        
        // Test serialization
        for priority in &priorities {
            let json = serde_json::to_string(priority).unwrap();
            let deserialized: MessagePriority = serde_json::from_str(&json).unwrap();
            assert_eq!(priority, &deserialized);
        }
    }

    #[test]
    fn test_message_metadata() {
        let metadata = MessageMetadata {
            timestamp: SystemTime::now(),
            priority: MessagePriority::Normal,
            requires_ack: true,
            substrate_context: None,
        };
        
        // Test serialization
        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: MessageMetadata = serde_json::from_str(&json).unwrap();
        
        assert_eq!(metadata.priority, deserialized.priority);
        assert_eq!(metadata.requires_ack, deserialized.requires_ack);
    }
}

#[cfg(test)]
mod public_key_tests {
    use super::*;

    #[test]
    fn test_public_key_variants() {
        let keys = vec![
            PublicKey::Ed25519(vec![1, 2, 3, 4]),
            PublicKey::X25519(vec![5, 6, 7, 8]),
            PublicKey::PostQuantum(PostQuantumPublicKey::Kyber1024(vec![9, 10])),
        ];
        
        // Test serialization
        for key in &keys {
            let json = serde_json::to_string(key).unwrap();
            let deserialized: PublicKey = serde_json::from_str(&json).unwrap();
            match (key, &deserialized) {
                (PublicKey::Ed25519(a), PublicKey::Ed25519(b)) => assert_eq!(a, b),
                (PublicKey::X25519(a), PublicKey::X25519(b)) => assert_eq!(a, b),
                (PublicKey::PostQuantum(a), PublicKey::PostQuantum(b)) => {
                    match (a, b) {
                        (PostQuantumPublicKey::Kyber1024(x), PostQuantumPublicKey::Kyber1024(y)) => assert_eq!(x, y),
                        _ => panic!("Mismatched PostQuantum types"),
                    }
                },
                _ => panic!("Mismatched key types"),
            }
        }
    }
}

#[cfg(test)]
mod signaling_message_tests {
    use super::*;

    #[test]
    fn test_signaling_message_register() {
        let msg = SignalingMessage::Register {
            peer_id: PeerId::new(),
        };
        
        // Test serialization
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: SignalingMessage = serde_json::from_str(&json).unwrap();
        
        match (&msg, &deserialized) {
            (SignalingMessage::Register { peer_id: a }, SignalingMessage::Register { peer_id: b }) => {
                assert_eq!(a, b);
            },
            _ => panic!("Unexpected message type"),
        }
    }
}