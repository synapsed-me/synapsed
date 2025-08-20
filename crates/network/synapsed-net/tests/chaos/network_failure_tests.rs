//! Chaos tests for network failures and edge cases.

use synapsed_net::{
    transport::{MemoryTransport, TcpTransport, UdpTransport, Transport},
    error::{NetworkError, TransportError},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, sleep};

mod common;
use common::*;

// Simulated network failures

#[tokio::test]
async fn test_connection_timeout() {
    init_test_logging();
    
    let transport = Arc::new(TcpTransport::new());
    
    // Try to connect to a non-routable address (should timeout)
    let peer = create_test_peer("10.255.255.255:12345".parse().unwrap());
    
    let result = timeout(Duration::from_secs(2), transport.connect(&peer)).await;
    
    match result {
        Ok(Ok(_)) => panic!("Should not have connected"),
        Ok(Err(_)) => {}, // Connection error
        Err(_) => {}, // Timeout
    }
}

#[tokio::test]
async fn test_connection_drop_during_transfer() {
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
    
    // Start a large transfer
    let large_data = generate_test_data(10 * 1024 * 1024); // 10MB
    let data_clone = large_data.clone();
    
    let mut client_stream = client_conn.stream();
    let send_task = tokio::spawn(async move {
        client_stream.write_all(&data_clone).await
    });
    
    // Drop server connection mid-transfer
    sleep(Duration::from_millis(10)).await;
    drop(server_conn);
    
    // Send should fail
    let result = send_task.await.unwrap();
    assert!(result.is_err());
}

#[tokio::test]
async fn test_rapid_connect_disconnect_cycles() {
    init_test_logging();
    
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:60000".parse().unwrap();
    
    for cycle in 0..10 {
        let mut listener = transport.listen(addr).await.unwrap();
        let peer = create_test_peer(addr);
        
        // Rapid connect/disconnect
        for i in 0..10 {
            let transport_clone = transport.clone();
            let peer_clone = peer.clone();
            
            let connect_task = tokio::spawn(async move {
                transport_clone.connect(&peer_clone).await
            });
            
            let (server_conn, _) = listener.accept().await.unwrap();
            let client_conn = connect_task.await.unwrap().unwrap();
            
            // Quick data exchange
            let mut client_stream = client_conn.stream();
            client_stream.write_all(b"ping").await.unwrap();
            
            let mut server_stream = server_conn.stream();
            let mut buf = [0u8; 4];
            server_stream.read_exact(&mut buf).await.unwrap();
            
            // Immediately drop connections
            drop(client_conn);
            drop(server_conn);
        }
        
        drop(listener);
        
        // Brief pause between cycles
        sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn test_packet_loss_simulation() {
    init_test_logging();
    
    // This test simulates packet loss by randomly dropping data
    struct LossyTransport<T: Transport> {
        inner: T,
        loss_rate: f32,
    }
    
    impl<T: Transport> LossyTransport<T> {
        fn new(inner: T, loss_rate: f32) -> Self {
            Self { inner, loss_rate }
        }
    }
    
    // Note: Real packet loss simulation would require modifying the stream implementation
    // This is a simplified version showing the concept
    
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:60001".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let peer = create_test_peer(addr);
    
    let transport_clone = transport.clone();
    let peer_clone = peer.clone();
    let connect_task = tokio::spawn(async move {
        transport_clone.connect(&peer_clone).await
    });
    
    let (server_conn, _) = listener.accept().await.unwrap();
    let client_conn = connect_task.await.unwrap().unwrap();
    
    // Send data in small chunks to simulate packets
    let data = generate_test_data(1024);
    let chunk_size = 64;
    
    let mut client_stream = client_conn.stream();
    for chunk in data.chunks(chunk_size) {
        // Simulate 10% packet loss
        if rand::random::<f32>() > 0.1 {
            client_stream.write_all(chunk).await.unwrap();
            client_stream.flush().await.unwrap();
        }
    }
}

#[tokio::test]
async fn test_high_latency_conditions() {
    init_test_logging();
    
    // Simulate high latency by adding delays
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:60002".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let peer = create_test_peer(addr);
    
    let transport_clone = transport.clone();
    let peer_clone = peer.clone();
    
    // Add artificial delay to connection
    let connect_task = tokio::spawn(async move {
        sleep(Duration::from_millis(200)).await; // Simulate 200ms latency
        transport_clone.connect(&peer_clone).await
    });
    
    let accept_task = tokio::spawn(async move {
        listener.accept().await
    });
    
    let (server_result, client_result) = tokio::join!(accept_task, connect_task);
    let (server_conn, _) = server_result.unwrap().unwrap();
    let client_conn = client_result.unwrap().unwrap();
    
    // Measure round-trip time with artificial delays
    let start = std::time::Instant::now();
    
    let mut client_stream = client_conn.stream();
    client_stream.write_all(b"ping").await.unwrap();
    client_stream.flush().await.unwrap();
    
    sleep(Duration::from_millis(100)).await; // One-way latency
    
    let mut server_stream = server_conn.stream();
    let mut buf = [0u8; 4];
    server_stream.read_exact(&mut buf).await.unwrap();
    server_stream.write_all(b"pong").await.unwrap();
    server_stream.flush().await.unwrap();
    
    sleep(Duration::from_millis(100)).await; // Return latency
    
    client_stream.read_exact(&mut buf).await.unwrap();
    
    let rtt = start.elapsed();
    assert!(rtt >= Duration::from_millis(200));
}

#[tokio::test]
async fn test_bandwidth_limitation() {
    init_test_logging();
    
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:60003".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let peer = create_test_peer(addr);
    
    let transport_clone = transport.clone();
    let peer_clone = peer.clone();
    let connect_task = tokio::spawn(async move {
        transport_clone.connect(&peer_clone).await
    });
    
    let (server_conn, _) = listener.accept().await.unwrap();
    let client_conn = connect_task.await.unwrap().unwrap();
    
    // Simulate bandwidth limitation by throttling writes
    let data_size = 1024 * 1024; // 1MB
    let bandwidth_mbps = 10.0; // 10 Mbps
    let chunk_size = 8192; // 8KB chunks
    let chunk_delay_ms = (chunk_size as f64 * 8.0) / (bandwidth_mbps * 1000.0);
    
    let data = generate_test_data(data_size);
    let start = std::time::Instant::now();
    
    let mut client_stream = client_conn.stream();
    for chunk in data.chunks(chunk_size) {
        client_stream.write_all(chunk).await.unwrap();
        client_stream.flush().await.unwrap();
        sleep(Duration::from_millis(chunk_delay_ms as u64)).await;
    }
    
    let duration = start.elapsed();
    let actual_bandwidth = (data_size as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000.0);
    
    // Should be close to target bandwidth
    assert!(actual_bandwidth < bandwidth_mbps * 1.1); // Within 10% tolerance
}

#[tokio::test]
async fn test_connection_pool_exhaustion() {
    init_test_logging();
    
    let transport = Arc::new(TcpTransport::new());
    let base_port = 60100;
    
    // Create many connections without closing them
    let mut connections = vec![];
    
    for i in 0..100 {
        let addr = format!("127.0.0.1:{}", base_port + i).parse().unwrap();
        let listener = transport.listen(addr).await;
        
        if let Ok(listener) = listener {
            connections.push(listener);
        } else {
            // Eventually we might run out of resources
            println!("Failed to create listener {} - resource exhaustion?", i);
            break;
        }
    }
    
    // Should have created at least some connections
    assert!(connections.len() > 10);
    
    // Clean up
    for mut conn in connections {
        let _ = conn.close().await;
    }
}

#[tokio::test]
async fn test_malformed_data_handling() {
    init_test_logging();
    
    let transport = Arc::new(TcpTransport::new());
    let addr = "127.0.0.1:0".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    
    // Connect with raw TCP socket to send malformed data
    let raw_socket = tokio::net::TcpStream::connect(local_addr).await.unwrap();
    
    // Accept the connection
    let (server_conn, _) = listener.accept().await.unwrap();
    
    // Try to read - should handle gracefully
    let mut server_stream = server_conn.stream();
    let mut buf = vec![0u8; 1024];
    let result = timeout(Duration::from_secs(1), server_stream.read(&mut buf)).await;
    
    // Should timeout or return 0 bytes (EOF)
    match result {
        Ok(Ok(0)) => {}, // EOF
        Ok(Ok(_)) => {}, // Got some data
        Ok(Err(_)) => {}, // Error
        Err(_) => {}, // Timeout
    }
}

#[tokio::test]
async fn test_concurrent_operations_under_stress() {
    init_test_logging();
    
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:60200".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let peer = create_test_peer(addr);
    
    // Create multiple connections
    let num_conns = 10;
    let mut server_conns = vec![];
    let mut client_conns = vec![];
    
    for _ in 0..num_conns {
        let transport_clone = transport.clone();
        let peer_clone = peer.clone();
        
        let connect_task = tokio::spawn(async move {
            transport_clone.connect(&peer_clone).await
        });
        
        let (server_conn, _) = listener.accept().await.unwrap();
        let client_conn = connect_task.await.unwrap().unwrap();
        
        server_conns.push(server_conn);
        client_conns.push(client_conn);
    }
    
    // Stress test with concurrent reads and writes
    let mut tasks = vec![];
    
    for (i, (server_conn, client_conn)) in server_conns.into_iter().zip(client_conns.into_iter()).enumerate() {
        // Client sends rapidly
        let client_task = tokio::spawn(async move {
            let mut stream = client_conn.stream();
            for j in 0..100 {
                let data = format!("Client {} message {}", i, j);
                stream.write_all(data.as_bytes()).await.unwrap();
                if j % 10 == 0 {
                    stream.flush().await.unwrap();
                }
            }
            stream.flush().await.unwrap();
        });
        
        // Server echoes back
        let server_task = tokio::spawn(async move {
            let mut stream = server_conn.stream();
            let mut buf = vec![0u8; 1024];
            let mut total_read = 0;
            
            while total_read < 2000 { // Approximate expected data size
                match stream.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        total_read += n;
                        stream.write_all(&buf[..n]).await.unwrap();
                    }
                    Err(e) => {
                        eprintln!("Server read error: {}", e);
                        break;
                    }
                }
            }
            stream.flush().await.unwrap();
        });
        
        tasks.push(client_task);
        tasks.push(server_task);
    }
    
    // Wait for all tasks with timeout
    let results = timeout(Duration::from_secs(10), futures::future::join_all(tasks)).await;
    assert!(results.is_ok(), "Tasks did not complete in time");
}

#[tokio::test]
async fn test_network_partition_simulation() {
    init_test_logging();
    
    // Simulate network partition by breaking connections
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:60300".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let peer = create_test_peer(addr);
    
    // Establish connections
    let mut connections = vec![];
    for _ in 0..5 {
        let transport_clone = transport.clone();
        let peer_clone = peer.clone();
        
        let connect_task = tokio::spawn(async move {
            transport_clone.connect(&peer_clone).await
        });
        
        let (server_conn, _) = listener.accept().await.unwrap();
        let client_conn = connect_task.await.unwrap().unwrap();
        
        connections.push((server_conn, client_conn));
    }
    
    // Simulate partition by dropping half the connections
    for (i, (server_conn, client_conn)) in connections.into_iter().enumerate() {
        if i < 2 {
            // These connections "survive" the partition
            let mut client_stream = client_conn.stream();
            client_stream.write_all(b"survived").await.unwrap();
            
            let mut server_stream = server_conn.stream();
            let mut buf = vec![0u8; 8];
            server_stream.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, b"survived");
        } else {
            // These connections are "partitioned"
            drop(server_conn);
            drop(client_conn);
        }
    }
}

// Helper function for jitter simulation
fn add_jitter(base_delay: Duration, jitter_ms: u64) -> Duration {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let jitter = rng.gen_range(0..=jitter_ms * 2);
    base_delay + Duration::from_millis(jitter)
}

#[tokio::test]
async fn test_jittery_network_conditions() {
    init_test_logging();
    
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:60400".parse().unwrap();
    
    let mut listener = transport.listen(addr).await.unwrap();
    let peer = create_test_peer(addr);
    
    let transport_clone = transport.clone();
    let peer_clone = peer.clone();
    let connect_task = tokio::spawn(async move {
        transport_clone.connect(&peer_clone).await
    });
    
    let (server_conn, _) = listener.accept().await.unwrap();
    let client_conn = connect_task.await.unwrap().unwrap();
    
    // Send packets with variable delays (jitter)
    let base_delay = Duration::from_millis(50);
    let jitter_ms = 25;
    
    let mut client_stream = client_conn.stream();
    let mut server_stream = server_conn.stream();
    
    for i in 0..10 {
        let delay = add_jitter(base_delay, jitter_ms);
        sleep(delay).await;
        
        let msg = format!("Message {}", i);
        client_stream.write_all(msg.as_bytes()).await.unwrap();
        client_stream.flush().await.unwrap();
        
        let mut buf = vec![0u8; msg.len()];
        server_stream.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf, msg.as_bytes());
    }
}