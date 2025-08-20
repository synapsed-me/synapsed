//! Unit tests for individual transport implementations.

use synapsed_net::{
    transport::{MemoryTransport, QuicTransport, TcpTransport, UdpTransport, WebRTCTransport, Transport},
    types::{PeerId, PeerInfo, NetworkAddress, PeerMetadata, TransportType, TransportRequirements},
    error::{NetworkError, TransportError},
};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use proptest::prelude::*;

mod common;
use common::*;

// Memory Transport Tests

#[tokio::test]
async fn test_memory_transport_connect_without_listener() {
    init_test_logging();
    
    let transport = Arc::new(MemoryTransport::new());
    let peer = create_test_peer("127.0.0.1:20000".parse().unwrap());
    
    let result = transport.connect(&peer).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        NetworkError::Transport(TransportError::NotAvailable(_)) => {},
        e => panic!("Unexpected error: {:?}", e),
    }
}

#[tokio::test]
async fn test_memory_transport_multiple_listeners_same_port() {
    init_test_logging();
    
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:20001".parse().unwrap();
    
    let _listener1 = transport.listen(addr).await.unwrap();
    let listener2 = transport.listen(addr).await;
    
    // Memory transport currently allows multiple listeners on same port
    // This is a limitation that could be fixed
    assert!(listener2.is_ok());
}

#[tokio::test]
async fn test_memory_transport_stream_properties() {
    init_test_logging();
    
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:20002".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let peer = create_test_peer(addr);
    
    let transport_clone = transport.clone();
    let peer_clone = peer.clone();
    let connect_task = tokio::spawn(async move {
        transport_clone.connect(&peer_clone).await
    });
    
    let (conn, remote_addr) = listener.accept().await.unwrap();
    let _ = connect_task.await.unwrap().unwrap();
    
    // Check connection info
    let info = conn.info();
    assert_eq!(info.transport, TransportType::Memory);
    assert_eq!(remote_addr, addr);
}

// TCP Transport Tests

#[tokio::test]
async fn test_tcp_transport_basic_connectivity() {
    init_test_logging();
    
    let transport = Arc::new(TcpTransport::new());
    let addr = "127.0.0.1:0".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let peer = create_test_peer(local_addr);
    
    let transport_clone = transport.clone();
    let peer_clone = peer.clone();
    let connect_task = tokio::spawn(async move {
        transport_clone.connect(&peer_clone).await
    });
    
    let (server_conn, client_addr) = listener.accept().await.unwrap();
    let client_conn = connect_task.await.unwrap().unwrap();
    
    // Test data transfer
    let test_data = b"Hello TCP!";
    let mut client_stream = client_conn.stream();
    client_stream.write_all(test_data).await.unwrap();
    client_stream.flush().await.unwrap();
    
    let mut server_stream = server_conn.stream();
    let mut received = vec![0u8; test_data.len()];
    server_stream.read_exact(&mut received).await.unwrap();
    
    assert_eq!(&received, test_data);
}

#[tokio::test]
async fn test_tcp_transport_large_transfer() {
    init_test_logging();
    
    let transport = Arc::new(TcpTransport::new());
    let addr = "127.0.0.1:0".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let peer = create_test_peer(local_addr);
    
    let transport_clone = transport.clone();
    let peer_clone = peer.clone();
    let connect_task = tokio::spawn(async move {
        transport_clone.connect(&peer_clone).await
    });
    
    let (server_conn, _) = listener.accept().await.unwrap();
    let client_conn = connect_task.await.unwrap().unwrap();
    
    // Test with 10MB data
    let test_data = generate_test_data(10 * 1024 * 1024);
    
    let data_clone = test_data.clone();
    let mut client_stream = client_conn.stream();
    let send_task = tokio::spawn(async move {
        client_stream.write_all(&data_clone).await?;
        client_stream.flush().await
    });
    
    let mut server_stream = server_conn.stream();
    let mut received = vec![0u8; test_data.len()];
    server_stream.read_exact(&mut received).await.unwrap();
    
    send_task.await.unwrap().unwrap();
    assert_eq!(received, test_data);
}

#[tokio::test]
async fn test_tcp_transport_concurrent_streams() {
    init_test_logging();
    
    let transport = Arc::new(TcpTransport::new());
    let addr = "127.0.0.1:0".parse().unwrap();
    
    let durations = test_concurrent_connections(transport, addr, 20).await.unwrap();
    
    // TCP should handle concurrent connections well
    for duration in &durations {
        assert!(duration < &std::time::Duration::from_secs(1));
    }
    
    let stats = PerfStats::from_durations(durations);
    println!("TCP concurrent connection stats: {:?}", stats);
}

// UDP Transport Tests

#[tokio::test]
async fn test_udp_transport_basic_connectivity() {
    init_test_logging();
    
    let transport = Arc::new(UdpTransport::new());
    let addr = "127.0.0.1:0".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let peer = create_test_peer(local_addr);
    
    let transport_clone = transport.clone();
    let peer_clone = peer.clone();
    let connect_task = tokio::spawn(async move {
        transport_clone.connect(&peer_clone).await
    });
    
    let client_conn = connect_task.await.unwrap().unwrap();
    
    // Send initial packet to establish "connection"
    let test_data = b"Hello UDP!";
    let mut client_stream = client_conn.stream();
    client_stream.write_all(test_data).await.unwrap();
    client_stream.flush().await.unwrap();
    
    // Accept should detect the new "connection"
    let (server_conn, _) = listener.accept().await.unwrap();
    
    // Verify connection info
    let info = server_conn.info();
    assert_eq!(info.transport, TransportType::Udp);
}

#[tokio::test]
async fn test_udp_transport_packet_boundaries() {
    init_test_logging();
    
    let transport = Arc::new(UdpTransport::new());
    let addr = "127.0.0.1:0".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let peer = create_test_peer(local_addr);
    
    let client_conn = transport.connect(&peer).await.unwrap();
    
    // Send multiple packets
    let packets = vec![
        b"Packet 1".to_vec(),
        b"Packet 2 is longer".to_vec(),
        b"P3".to_vec(),
    ];
    
    let mut client_stream = client_conn.stream();
    for packet in &packets {
        client_stream.write_all(packet).await.unwrap();
        client_stream.flush().await.unwrap(); // Each flush sends a UDP packet
    }
    
    // Note: UDP packet boundaries are not preserved in our stream abstraction
    // This is a limitation of mapping datagram semantics to stream semantics
}

#[tokio::test]
async fn test_udp_transport_max_packet_size() {
    init_test_logging();
    
    let transport = Arc::new(UdpTransport::new());
    let addr = "127.0.0.1:0".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let peer = create_test_peer(local_addr);
    
    let client_conn = transport.connect(&peer).await.unwrap();
    
    // Try to send packet larger than UDP max (65507 bytes)
    let large_data = vec![0u8; 65508];
    let mut client_stream = client_conn.stream();
    let result = client_stream.write_all(&large_data).await;
    
    // Should fail or be fragmented
    if result.is_ok() {
        let flush_result = client_stream.flush().await;
        assert!(flush_result.is_err());
    }
}

// QUIC Transport Tests

#[tokio::test]
async fn test_quic_transport_initialization() {
    init_test_logging();
    
    let addr = "127.0.0.1:0".parse().unwrap();
    let mut transport = QuicTransport::new(addr).unwrap();
    
    assert!(transport.initialize().await.is_ok());
    assert_eq!(transport.transport_type(), TransportType::Quic);
    assert_eq!(transport.priority(), synapsed_net::transport::traits::TransportPriority::Preferred);
}

#[tokio::test]
async fn test_quic_transport_features() {
    use synapsed_net::transport::traits::TransportFeature;
    
    let addr = "127.0.0.1:0".parse().unwrap();
    let transport = QuicTransport::new(addr).unwrap();
    
    // QUIC should support advanced features
    assert!(transport.supports_feature(TransportFeature::ZeroRTT));
    assert!(transport.supports_feature(TransportFeature::Multistream));
    assert!(transport.supports_feature(TransportFeature::ConnectionMigration));
    assert!(transport.supports_feature(TransportFeature::BandwidthEstimation));
    assert!(!transport.supports_feature(TransportFeature::PostQuantum));
}

#[tokio::test]
async fn test_quic_transport_with_custom_certs() {
    init_test_logging();
    
    let addr = "127.0.0.1:0".parse().unwrap();
    let (cert_chain, private_key) = generate_test_cert();
    
    let transport = QuicTransport::with_certificates(addr, cert_chain, private_key);
    assert!(transport.is_ok());
}

// WebRTC Transport Tests

#[tokio::test]
async fn test_webrtc_transport_creation() {
    init_test_logging();
    
    let transport = WebRTCTransport::new(None).unwrap();
    
    assert_eq!(transport.transport_type(), TransportType::WebRtc);
    assert_eq!(transport.priority(), synapsed_net::transport::traits::TransportPriority::High);
}

#[tokio::test]
async fn test_webrtc_transport_features() {
    use synapsed_net::transport::traits::TransportFeature;
    
    let transport = WebRTCTransport::new(None).unwrap();
    
    // WebRTC excels at NAT traversal
    assert!(transport.supports_feature(TransportFeature::NATTraversal));
    assert!(transport.supports_feature(TransportFeature::UnreliableChannel));
    assert!(transport.supports_feature(TransportFeature::Multistream));
    assert!(transport.supports_feature(TransportFeature::BandwidthEstimation));
    assert!(!transport.supports_feature(TransportFeature::PostQuantum));
}

// Property-based tests

proptest! {
    #[test]
    fn prop_memory_transport_data_integrity(data: Vec<u8>) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let transport = Arc::new(MemoryTransport::new());
            let addr = "127.0.0.1:30000".parse().unwrap();
            
            let mut listener = transport.listen(addr).await.unwrap();
            let peer = create_test_peer(addr);
            
            let transport_clone = transport.clone();
            let peer_clone = peer.clone();
            let connect_task = tokio::spawn(async move {
                transport_clone.connect(&peer_clone).await
            });
            
            let (server_conn, _) = listener.accept().await.unwrap();
            let client_conn = connect_task.await.unwrap().unwrap();
            
            // Send data
            let data_clone = data.clone();
            let mut client_stream = client_conn.stream();
            let send_task = tokio::spawn(async move {
                client_stream.write_all(&data_clone).await?;
                client_stream.flush().await
            });
            
            // Receive and verify
            let mut server_stream = server_conn.stream();
            let mut received = vec![0u8; data.len()];
            if !data.is_empty() {
                server_stream.read_exact(&mut received).await.unwrap();
                assert_eq!(received, data);
            }
            
            send_task.await.unwrap().unwrap();
        });
    }
    
    #[test]
    fn prop_transport_requirements_matching(
        low_latency: bool,
        high_throughput: bool,
        reliability: bool,
        nat_traversal: bool,
    ) {
        let mut reqs = TransportRequirements::default();
        
        if low_latency {
            reqs = reqs.with_low_latency();
        }
        if high_throughput {
            reqs = reqs.with_high_throughput();
        }
        if reliability {
            reqs = reqs.with_reliability();
        }
        if nat_traversal {
            reqs = reqs.with_nat_traversal();
        }
        
        // Requirements should be internally consistent
        if reqs.max_latency_ms.is_some() && reqs.min_throughput_mbps.is_some() {
            prop_assert!(reqs.max_latency_ms.unwrap() > 0);
            prop_assert!(reqs.min_throughput_mbps.unwrap() > 0.0);
        }
    }
}

// Error handling tests

#[tokio::test]
async fn test_transport_invalid_address_handling() {
    init_test_logging();
    
    let transport = Arc::new(TcpTransport::new());
    
    let peer = PeerInfo {
        id: PeerId::new(),
        address: "not-a-valid-address".to_string(),
        addresses: vec![],
        protocols: vec![],
        capabilities: vec![],
        public_key: None,
        metadata: PeerMetadata::default(),
    };
    
    let result = transport.connect(&peer).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        NetworkError::Transport(TransportError::InvalidAddress(_)) => {},
        e => panic!("Unexpected error: {:?}", e),
    }
}

#[tokio::test]
async fn test_transport_connection_refused() {
    init_test_logging();
    
    let transport = Arc::new(TcpTransport::new());
    
    // Try to connect to a port that's not listening
    let peer = create_test_peer("127.0.0.1:40000".parse().unwrap());
    
    let result = transport.connect(&peer).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_transport_listener_cleanup() {
    init_test_logging();
    
    let transport = Arc::new(TcpTransport::new());
    let addr = "127.0.0.1:0".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    
    // Close the listener
    listener.close().await.unwrap();
    
    // Try to connect - should fail
    let peer = create_test_peer(local_addr);
    let result = transport.connect(&peer).await;
    assert!(result.is_err());
}

// Stress tests

#[tokio::test]
#[ignore] // Run with --ignored for stress tests
async fn stress_test_rapid_connections() {
    init_test_logging();
    
    let transport = Arc::new(TcpTransport::new());
    
    for i in 0..1000 {
        let addr = format!("127.0.0.1:{}", 50000 + i).parse().unwrap();
        let mut listener = transport.listen(addr).await.unwrap();
        let peer = create_test_peer(addr);
        
        let transport_clone = transport.clone();
        let peer_clone = peer.clone();
        
        let connect_task = tokio::spawn(async move {
            transport_clone.connect(&peer_clone).await
        });
        
        let (server_conn, _) = listener.accept().await.unwrap();
        let client_conn = connect_task.await.unwrap().unwrap();
        
        // Quick data exchange
        let mut client_stream = client_conn.stream();
        client_stream.write_all(b"test").await.unwrap();
        
        drop(server_conn);
        drop(client_conn);
        drop(listener);
    }
}

#[tokio::test]
#[ignore] // Run with --ignored for stress tests
async fn stress_test_concurrent_data_streams() {
    init_test_logging();
    
    let transport = Arc::new(TcpTransport::new());
    let addr = "127.0.0.1:0".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    
    let num_connections = 50;
    let mut tasks = vec![];
    
    // Spawn acceptor
    let accept_task = tokio::spawn(async move {
        let mut connections = vec![];
        for _ in 0..num_connections {
            let (conn, _) = listener.accept().await.unwrap();
            connections.push(conn);
        }
        connections
    });
    
    // Spawn connectors with data transfer
    for i in 0..num_connections {
        let transport = transport.clone();
        let peer = create_test_peer(local_addr);
        
        let task = tokio::spawn(async move {
            let conn = transport.connect(&peer).await.unwrap();
            let data = generate_test_data(1024 * 1024); // 1MB per connection
            
            let mut stream = conn.stream();
            stream.write_all(&data).await.unwrap();
            stream.flush().await.unwrap();
            
            (i, data)
        });
        
        tasks.push(task);
    }
    
    // Wait for all connections and verify data
    let server_conns = accept_task.await.unwrap();
    
    for (task, server_conn) in tasks.into_iter().zip(server_conns.into_iter()) {
        let (idx, sent_data) = task.await.unwrap();
        
        let mut stream = server_conn.stream();
        let mut received = vec![0u8; sent_data.len()];
        stream.read_exact(&mut received).await.unwrap();
        
        assert_eq!(received, sent_data, "Data mismatch for connection {}", idx);
    }
}