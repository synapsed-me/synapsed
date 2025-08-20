//! Performance benchmarks for synapsed-net.

use synapsed_net::{
    transport::{MemoryTransport, Transport},
    types::{PeerId, PeerInfo, NetworkAddress, PeerMetadata},
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn bench_connection_establishment() {
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:10000".parse().unwrap();
    
    // Warmup
    let mut listener = transport.listen(addr).await.unwrap();
    let peer = create_test_peer(addr);
    
    let measurements = 100;
    let mut durations = Vec::with_capacity(measurements);
    
    for i in 0..measurements {
        let current_addr = format!("127.0.0.1:{}", 10001 + i).parse().unwrap();
        let mut listener = transport.listen(current_addr).await.unwrap();
        let peer = create_test_peer(current_addr);
        
        let transport_clone = transport.clone();
        let peer_clone = peer.clone();
        
        let start = Instant::now();
        
        let connect_task = tokio::spawn(async move {
            transport_clone.connect(&peer_clone).await
        });
        
        let _ = listener.accept().await.unwrap();
        let _ = connect_task.await.unwrap().unwrap();
        
        durations.push(start.elapsed());
    }
    
    // Calculate statistics
    let stats = calculate_stats(&durations);
    
    println!("Connection Establishment Benchmarks:");
    println!("  Min:  {:?}", stats.min);
    println!("  Max:  {:?}", stats.max);
    println!("  Mean: {:?}", stats.mean);
    println!("  P50:  {:?}", stats.p50);
    println!("  P95:  {:?}", stats.p95);
    println!("  P99:  {:?}", stats.p99);
    
    // Assert reasonable performance
    assert!(stats.mean < Duration::from_millis(10));
    assert!(stats.p99 < Duration::from_millis(50));
}

#[tokio::test]
async fn bench_throughput() {
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:11000".parse().unwrap();
    
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
    
    // Test different data sizes
    let test_sizes = vec![
        (1024, "1KB"),
        (1024 * 10, "10KB"),
        (1024 * 100, "100KB"),
        (1024 * 1024, "1MB"),
        (1024 * 1024 * 10, "10MB"),
    ];
    
    println!("\nThroughput Benchmarks:");
    
    for (size, label) in test_sizes {
        let data = vec![0xAA; size];
        let start = Instant::now();
        
        // Send data
        let data_clone = data.clone();
        let mut client_stream = client_conn.stream();
        let send_task = tokio::spawn(async move {
            client_stream.write_all(&data_clone).await?;
            client_stream.flush().await
        });
        
        // Receive data
        let mut server_stream = server_conn.stream();
        let mut received = vec![0u8; size];
        server_stream.read_exact(&mut received).await.unwrap();
        
        send_task.await.unwrap().unwrap();
        
        let duration = start.elapsed();
        let throughput_mbps = (size as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000.0);
        
        println!("  {}: {:.2} Mbps ({:?} for {} bytes)", label, throughput_mbps, duration, size);
        
        // Memory transport should be very fast
        assert!(throughput_mbps > 100.0); // At least 100 Mbps
    }
}

#[tokio::test]
async fn bench_latency() {
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:12000".parse().unwrap();
    
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
    
    // Measure round-trip latency
    let iterations = 1000;
    let mut latencies = Vec::with_capacity(iterations);
    
    for _ in 0..iterations {
        let ping = b"ping";
        let start = Instant::now();
        
        // Send ping
        let mut client_stream = client_conn.stream();
        client_stream.write_all(ping).await.unwrap();
        client_stream.flush().await.unwrap();
        
        // Receive ping and send pong
        let mut server_stream = server_conn.stream();
        let mut buf = [0u8; 4];
        server_stream.read_exact(&mut buf).await.unwrap();
        server_stream.write_all(b"pong").await.unwrap();
        server_stream.flush().await.unwrap();
        
        // Receive pong
        client_stream.read_exact(&mut buf).await.unwrap();
        
        latencies.push(start.elapsed());
    }
    
    let stats = calculate_stats(&latencies);
    
    println!("\nRound-trip Latency Benchmarks:");
    println!("  Min:  {:?}", stats.min);
    println!("  Max:  {:?}", stats.max);
    println!("  Mean: {:?}", stats.mean);
    println!("  P50:  {:?}", stats.p50);
    println!("  P95:  {:?}", stats.p95);
    println!("  P99:  {:?}", stats.p99);
    
    // Memory transport should have very low latency
    assert!(stats.mean < Duration::from_micros(100));
    assert!(stats.p99 < Duration::from_micros(500));
}

#[tokio::test]
async fn bench_concurrent_connections() {
    let transport = Arc::new(MemoryTransport::new());
    let base_port = 13000;
    
    let connection_counts = vec![10, 50, 100, 200];
    
    println!("\nConcurrent Connection Benchmarks:");
    
    for count in connection_counts {
        let addr = format!("127.0.0.1:{}", base_port).parse().unwrap();
        let start = Instant::now();
        
        let durations = test_concurrent_connections(transport.clone(), addr, count)
            .await
            .unwrap();
        
        let total_time = start.elapsed();
        let stats = calculate_stats(&durations);
        
        println!("  {} connections:", count);
        println!("    Total time:     {:?}", total_time);
        println!("    Mean conn time: {:?}", stats.mean);
        println!("    P99 conn time:  {:?}", stats.p99);
        
        // Should scale well
        assert!(total_time < Duration::from_secs(1));
    }
}

#[tokio::test]
async fn bench_memory_usage() {
    // This test would require memory profiling tools
    // For now, we'll test that connections don't leak memory
    
    let transport = Arc::new(MemoryTransport::new());
    let addr = "127.0.0.1:14000".parse().unwrap();
    
    // Create and destroy many connections
    for i in 0..100 {
        let current_addr = format!("127.0.0.1:{}", 14001 + i).parse().unwrap();
        let mut listener = transport.listen(current_addr).await.unwrap();
        let peer = create_test_peer(current_addr);
        
        let transport_clone = transport.clone();
        let peer_clone = peer.clone();
        
        let connect_task = tokio::spawn(async move {
            transport_clone.connect(&peer_clone).await
        });
        
        let (server_conn, _) = listener.accept().await.unwrap();
        let client_conn = connect_task.await.unwrap().unwrap();
        
        // Send some data
        let mut client_stream = client_conn.stream();
        client_stream.write_all(b"test").await.unwrap();
        
        // Explicitly drop connections
        drop(server_conn);
        drop(client_conn);
        drop(listener);
    }
    
    // In a real benchmark, we would measure memory usage here
    // and ensure it returns to baseline
}

#[tokio::test]
async fn bench_security_handshake() {
    use synapsed_net::security::SecurityLayer;
    
    let layer = SecurityLayer::new(false);
    let mut durations = Vec::with_capacity(100);
    
    for _ in 0..100 {
        let peer = PeerInfo {
            id: PeerId::new(),
            address: "127.0.0.1:8080".to_string(),
            addresses: vec![],
            protocols: vec![],
            capabilities: vec!["ChaCha20Poly1305X25519".to_string()],
            public_key: None,
            metadata: PeerMetadata::default(),
        };
        
        let start = Instant::now();
        
        let handshake = layer.initiate_handshake(&peer).await.unwrap();
        let _ = layer.complete_handshake(handshake, &peer).await.unwrap();
        
        durations.push(start.elapsed());
    }
    
    let stats = calculate_stats(&durations);
    
    println!("\nSecurity Handshake Benchmarks:");
    println!("  Min:  {:?}", stats.min);
    println!("  Max:  {:?}", stats.max);
    println!("  Mean: {:?}", stats.mean);
    println!("  P50:  {:?}", stats.p50);
    println!("  P95:  {:?}", stats.p95);
    println!("  P99:  {:?}", stats.p99);
    
    // Handshake should be fast
    assert!(stats.mean < Duration::from_millis(10));
}

// Helper functions
fn create_test_peer(addr: std::net::SocketAddr) -> PeerInfo {
    PeerInfo {
        id: PeerId::new(),
        address: addr.to_string(),
        addresses: vec![NetworkAddress::Socket(addr)],
        protocols: vec![],
        capabilities: vec![],
        public_key: None,
        metadata: PeerMetadata::default(),
    }
}

async fn test_concurrent_connections<T: Transport>(
    transport: Arc<T>,
    listener_addr: std::net::SocketAddr,
    num_connections: usize,
) -> Result<Vec<Duration>, synapsed_net::error::NetworkError> {
    use tokio::sync::Barrier;
    
    let mut listener = transport.listen(listener_addr).await?;
    let barrier = Arc::new(Barrier::new(num_connections + 1));
    
    // Spawn acceptor task
    let accept_barrier = barrier.clone();
    let accept_task = tokio::spawn(async move {
        accept_barrier.wait().await;
        let mut connections = vec![];
        for _ in 0..num_connections {
            let (conn, _) = listener.accept().await?;
            connections.push(conn);
        }
        Ok::<_, synapsed_net::error::NetworkError>(connections)
    });
    
    // Spawn connector tasks
    let mut connect_tasks = vec![];
    for _ in 0..num_connections {
        let transport = transport.clone();
        let barrier = barrier.clone();
        let peer = create_test_peer(listener_addr);
        
        let task = tokio::spawn(async move {
            barrier.wait().await;
            let start = Instant::now();
            let _ = transport.connect(&peer).await?;
            Ok::<_, synapsed_net::error::NetworkError>(start.elapsed())
        });
        
        connect_tasks.push(task);
    }
    
    // Wait for all connections
    let mut durations = vec![];
    for task in connect_tasks {
        durations.push(task.await.unwrap()?);
    }
    
    let _ = accept_task.await.unwrap()?;
    
    Ok(durations)
}

struct PerfStats {
    min: Duration,
    max: Duration,
    mean: Duration,
    p50: Duration,
    p95: Duration,
    p99: Duration,
}

fn calculate_stats(durations: &[Duration]) -> PerfStats {
    let mut sorted = durations.to_vec();
    sorted.sort();
    
    let len = sorted.len();
    let sum: Duration = sorted.iter().sum();
    let mean = sum / len as u32;
    
    PerfStats {
        min: sorted[0],
        max: sorted[len - 1],
        mean,
        p50: sorted[len / 2],
        p95: sorted[(len * 95) / 100],
        p99: sorted[(len * 99) / 100],
    }
}