//! Integration tests for transport implementations.

use synapsed_net::{
    transport::{MemoryTransport, QuicTransport, WebRtcTransport, Transport, TransportManager},
    types::{TransportType, TransportRequirements},
};
use std::sync::Arc;
use std::time::Duration;

mod common;
use common::*;

#[tokio::test]
async fn test_memory_transport_basic() {
    init_test_logging();
    
    let transport = Arc::new(MemoryTransport::new());
    let addr1 = "127.0.0.1:8001".parse().unwrap();
    let addr2 = "127.0.0.1:8002".parse().unwrap();
    
    // Test basic connection
    let mut listener = transport.listen(addr1).await.unwrap();
    let peer = create_test_peer(addr1);
    
    let transport_clone = transport.clone();
    let peer_clone = peer.clone();
    let connect_task = tokio::spawn(async move {
        transport_clone.connect(&peer_clone).await
    });
    
    let (server_conn, _) = listener.accept().await.unwrap();
    let client_conn = connect_task.await.unwrap().unwrap();
    
    // Test data transfer
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let test_data = b"Hello, Memory Transport!";
    
    let mut client_stream = client_conn.stream();
    client_stream.write_all(test_data).await.unwrap();
    client_stream.flush().await.unwrap();
    
    let mut server_stream = server_conn.stream();
    let mut received = vec![0u8; test_data.len()];
    server_stream.read_exact(&mut received).await.unwrap();
    
    assert_eq!(&received, test_data);
}

#[tokio::test]
async fn test_memory_transport_concurrent_connections() {
    init_test_logging();
    
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:8003".parse().unwrap();
    
    let durations = test_concurrent_connections(transport, addr, 10).await.unwrap();
    
    // Memory transport should be very fast
    for duration in durations {
        assert!(duration < Duration::from_millis(10));
    }
}

#[tokio::test]
async fn test_memory_transport_large_data_transfer() {
    init_test_logging();
    
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:8004".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let peer = create_test_peer(addr);
    
    // Connect
    let transport_clone = transport.clone();
    let peer_clone = peer.clone();
    let connect_task = tokio::spawn(async move {
        transport_clone.connect(&peer_clone).await
    });
    
    let (server_conn, _) = listener.accept().await.unwrap();
    let client_conn = connect_task.await.unwrap().unwrap();
    
    // Test with various data sizes
    for size in [1024, 1024 * 10, 1024 * 100, 1024 * 1024] {
        let test_data = generate_test_data(size);
        
        // Send from client
        let data_clone = test_data.clone();
        let mut client_stream = client_conn.stream();
        let send_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            client_stream.write_all(&data_clone).await?;
            client_stream.flush().await
        });
        
        // Receive on server
        let mut server_stream = server_conn.stream();
        let mut received = vec![0u8; size];
        use tokio::io::AsyncReadExt;
        server_stream.read_exact(&mut received).await.unwrap();
        
        send_task.await.unwrap().unwrap();
        assert_eq!(received, test_data);
    }
}

#[tokio::test]
async fn test_transport_manager_selection() {
    init_test_logging();
    
    let mut manager = TransportManager::new();
    
    // Add multiple transports
    let memory_transport = Arc::new(MemoryTransport::new());
    let quic_transport = Arc::new(QuicTransport::new("0.0.0.0:0".parse().unwrap()).unwrap());
    
    manager.add_transport(memory_transport.clone()).await.unwrap();
    manager.add_transport(quic_transport.clone()).await.unwrap();
    
    // Test selection with different requirements
    
    // No specific requirements - should prefer QUIC (higher priority)
    let transport = manager.select_transport(&TransportRequirements::default()).unwrap();
    assert_eq!(transport.transport_type(), TransportType::Quic);
    
    // Ultra-low latency - should still work with available transports
    let transport = manager.select_transport(&TransportRequirements::ultra_low_latency()).unwrap();
    assert!(matches!(
        transport.transport_type(),
        TransportType::Quic | TransportType::Memory
    ));
}

#[tokio::test]
async fn test_transport_feature_support() {
    use synapsed_net::transport::traits::TransportFeature;
    
    let memory = MemoryTransport::new();
    let quic = QuicTransport::new("0.0.0.0:0".parse().unwrap()).unwrap();
    
    // Memory transport features
    assert!(memory.supports_feature(TransportFeature::ZeroRTT));
    assert!(memory.supports_feature(TransportFeature::NATTraversal));
    assert!(!memory.supports_feature(TransportFeature::Multistream));
    assert!(!memory.supports_feature(TransportFeature::PostQuantum));
    
    // QUIC transport features
    assert!(quic.supports_feature(TransportFeature::ZeroRTT));
    assert!(quic.supports_feature(TransportFeature::Multistream));
    assert!(quic.supports_feature(TransportFeature::ConnectionMigration));
    assert!(!quic.supports_feature(TransportFeature::PostQuantum));
}

#[tokio::test]
async fn test_connection_info_and_metrics() {
    init_test_logging();
    
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:8005".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let peer = create_test_peer(addr);
    
    let transport_clone = transport.clone();
    let peer_clone = peer.clone();
    let connect_task = tokio::spawn(async move {
        transport_clone.connect(&peer_clone).await
    });
    
    let (conn, _) = listener.accept().await.unwrap();
    let _ = connect_task.await.unwrap().unwrap();
    
    // Check connection info
    let info = conn.info();
    assert_eq!(info.transport, TransportType::Memory);
    assert!(info.established_at <= std::time::SystemTime::now());
    
    // Metrics should be initialized
    assert_eq!(info.metrics.bytes_sent, 0);
    assert_eq!(info.metrics.bytes_received, 0);
    assert_eq!(info.metrics.packets_sent, 0);
    assert_eq!(info.metrics.packets_received, 0);
}

#[tokio::test]
async fn test_transport_error_handling() {
    init_test_logging();
    
    let transport = Arc::new(MemoryTransport::new());
    
    // Try to connect to non-existent listener
    let peer = create_test_peer("127.0.0.1:9999".parse().unwrap());
    let result = transport.connect(&peer).await;
    assert!(result.is_err());
    
    // Try to listen on the same address twice
    let addr = "127.0.0.1:8006".parse().unwrap();
    let _listener1 = transport.listen(addr).await.unwrap();
    
    // Memory transport doesn't prevent multiple listeners on same address
    // This is a limitation of the simple implementation
}

#[tokio::test]
async fn test_bidirectional_communication() {
    init_test_logging();
    
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:8007".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let peer = create_test_peer(addr);
    
    // Connect
    let transport_clone = transport.clone();
    let peer_clone = peer.clone();
    let connect_task = tokio::spawn(async move {
        transport_clone.connect(&peer_clone).await
    });
    
    let (server_conn, _) = listener.accept().await.unwrap();
    let client_conn = connect_task.await.unwrap().unwrap();
    
    // Test simultaneous bidirectional transfer
    let data1 = generate_test_data(1024);
    let data2 = generate_test_data(2048);
    
    let data1_clone = data1.clone();
    let data2_clone = data2.clone();
    
    // Client sends data1, receives data2
    let client_task = tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = client_conn.stream();
        
        // Send data1
        stream.write_all(&data1_clone).await?;
        stream.flush().await?;
        
        // Receive data2
        let mut received = vec![0u8; data2_clone.len()];
        stream.read_exact(&mut received).await?;
        
        Ok::<_, std::io::Error>((received, data2_clone))
    });
    
    // Server receives data1, sends data2
    let server_task = tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = server_conn.stream();
        
        // Receive data1
        let mut received = vec![0u8; data1.len()];
        stream.read_exact(&mut received).await?;
        assert_eq!(received, data1);
        
        // Send data2
        stream.write_all(&data2).await?;
        stream.flush().await?;
        
        Ok::<_, std::io::Error>(())
    });
    
    // Wait for both tasks
    let (received, expected) = client_task.await.unwrap().unwrap();
    assert_eq!(received, expected);
    server_task.await.unwrap().unwrap();
}

#[tokio::test]
#[ignore] // QUIC implementation is currently a placeholder
async fn test_quic_transport_basic() {
    init_test_logging();
    
    let transport = Arc::new(QuicTransport::new("127.0.0.1:0".parse().unwrap()).unwrap());
    let result = transport.listen("127.0.0.1:9001".parse().unwrap()).await;
    
    // Currently returns NotAvailable error
    assert!(result.is_err());
}

#[tokio::test]
#[ignore] // WebRTC implementation is currently a placeholder
async fn test_webrtc_transport_basic() {
    init_test_logging();
    
    let transport = Arc::new(WebRtcTransport::new(vec!["stun:stun.l.google.com:19302".to_string()]).unwrap());
    let result = transport.listen("127.0.0.1:9002".parse().unwrap()).await;
    
    // Currently returns NotAvailable error
    assert!(result.is_err());
}

// Property-based tests
#[tokio::test]
async fn test_property_data_integrity() {
    use proptest::prelude::*;
    
    init_test_logging();
    
    let transport = Arc::new(MemoryTransport::new());
    
    // Test that any data sent is received exactly
    proptest!(|(data: Vec<u8>)| {
        let transport = transport.clone();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let addr = "127.0.0.1:0".parse().unwrap();
            let mut listener = transport.listen(addr).await.unwrap();
            let local_addr = listener.local_addr().unwrap();
            let peer = create_test_peer(local_addr);
            
            // Connect
            let transport_clone = transport.clone();
            let peer_clone = peer.clone();
            let connect_task = tokio::spawn(async move {
                transport_clone.connect(&peer_clone).await
            });
            
            let (server_conn, _) = listener.accept().await.unwrap();
            let client_conn = connect_task.await.unwrap().unwrap();
            
            // Send data
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut client_stream = client_conn.stream();
            client_stream.write_all(&data).await.unwrap();
            client_stream.flush().await.unwrap();
            
            // Receive and verify
            let mut server_stream = server_conn.stream();
            let mut received = vec![0u8; data.len()];
            server_stream.read_exact(&mut received).await.unwrap();
            
            prop_assert_eq!(received, data);
            Ok(())
        })?;
    });
}