//! Comprehensive benchmarks for all transport implementations.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use synapsed_net::{
    transport::{MemoryTransport, TcpTransport, UdpTransport, Transport},
    types::{PeerId, PeerInfo, NetworkAddress, PeerMetadata},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Runtime;

fn create_test_peer(addr: std::net::SocketAddr) -> PeerInfo {
    PeerInfo {
        id: PeerId::new(),
        address: addr.to_string(),
        addresses: vec![NetworkAddress::Socket(addr)],
        protocols: vec!["bench/1.0".to_string()],
        capabilities: vec![],
        public_key: None,
        metadata: PeerMetadata::default(),
    }
}

fn bench_connection_establishment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("connection_establishment");
    
    // Memory transport
    group.bench_function("memory", |b| {
        b.to_async(&rt).iter(|| async {
            let transport = Arc::new(MemoryTransport::new());
            let addr = "127.0.0.1:40000".parse().unwrap();
            
            let mut listener = transport.listen(addr).await.unwrap();
            let peer = create_test_peer(addr);
            
            let transport_clone = transport.clone();
            let peer_clone = peer.clone();
            
            let connect_task = tokio::spawn(async move {
                transport_clone.connect(&peer_clone).await
            });
            
            let (server_conn, _) = listener.accept().await.unwrap();
            let client_conn = connect_task.await.unwrap().unwrap();
            
            black_box((server_conn, client_conn));
        });
    });
    
    // TCP transport
    group.bench_function("tcp", |b| {
        b.iter_custom(|iters| {
            rt.block_on(async {
                let mut total_duration = Duration::ZERO;
                
                for i in 0..iters {
                    let transport = Arc::new(TcpTransport::new());
                    let addr = format!("127.0.0.1:{}", 41000 + i).parse().unwrap();
                    
                    let start = std::time::Instant::now();
                    
                    let mut listener = transport.listen(addr).await.unwrap();
                    let peer = create_test_peer(addr);
                    
                    let transport_clone = transport.clone();
                    let peer_clone = peer.clone();
                    
                    let connect_task = tokio::spawn(async move {
                        transport_clone.connect(&peer_clone).await
                    });
                    
                    let (server_conn, _) = listener.accept().await.unwrap();
                    let client_conn = connect_task.await.unwrap().unwrap();
                    
                    total_duration += start.elapsed();
                    
                    black_box((server_conn, client_conn));
                }
                
                total_duration
            })
        });
    });
    
    group.finish();
}

fn bench_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("throughput");
    
    let data_sizes = vec![1024, 10240, 102400, 1048576]; // 1KB, 10KB, 100KB, 1MB
    
    for size in data_sizes {
        group.throughput(Throughput::Bytes(size as u64));
        
        // Memory transport throughput
        group.bench_with_input(
            BenchmarkId::new("memory", size),
            &size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let transport = Arc::new(MemoryTransport::new());
                    let addr = "127.0.0.1:42000".parse().unwrap();
                    
                    let mut listener = transport.listen(addr).await.unwrap();
                    let peer = create_test_peer(addr);
                    
                    let transport_clone = transport.clone();
                    let peer_clone = peer.clone();
                    
                    let connect_task = tokio::spawn(async move {
                        transport_clone.connect(&peer_clone).await
                    });
                    
                    let (server_conn, _) = listener.accept().await.unwrap();
                    let client_conn = connect_task.await.unwrap().unwrap();
                    
                    let data = vec![0xAA; size];
                    
                    let data_clone = data.clone();
                    let mut client_stream = client_conn.stream();
                    let send_task = tokio::spawn(async move {
                        client_stream.write_all(&data_clone).await.unwrap();
                        client_stream.flush().await.unwrap();
                    });
                    
                    let mut server_stream = server_conn.stream();
                    let mut received = vec![0u8; size];
                    server_stream.read_exact(&mut received).await.unwrap();
                    
                    send_task.await.unwrap();
                    
                    black_box(received);
                });
            },
        );
        
        // TCP transport throughput
        group.bench_with_input(
            BenchmarkId::new("tcp", size),
            &size,
            |b, &size| {
                b.iter_custom(|iters| {
                    rt.block_on(async {
                        let mut total_duration = Duration::ZERO;
                        
                        for i in 0..iters {
                            let transport = Arc::new(TcpTransport::new());
                            let addr = format!("127.0.0.1:{}", 43000 + i).parse().unwrap();
                            
                            let mut listener = transport.listen(addr).await.unwrap();
                            let peer = create_test_peer(addr);
                            
                            let transport_clone = transport.clone();
                            let peer_clone = peer.clone();
                            
                            let connect_task = tokio::spawn(async move {
                                transport_clone.connect(&peer_clone).await
                            });
                            
                            let (server_conn, _) = listener.accept().await.unwrap();
                            let client_conn = connect_task.await.unwrap().unwrap();
                            
                            let data = vec![0xAA; size];
                            
                            let start = std::time::Instant::now();
                            
                            let data_clone = data.clone();
                            let mut client_stream = client_conn.stream();
                            let send_task = tokio::spawn(async move {
                                client_stream.write_all(&data_clone).await.unwrap();
                                client_stream.flush().await.unwrap();
                            });
                            
                            let mut server_stream = server_conn.stream();
                            let mut received = vec![0u8; size];
                            server_stream.read_exact(&mut received).await.unwrap();
                            
                            send_task.await.unwrap();
                            
                            total_duration += start.elapsed();
                            
                            black_box(received);
                        }
                        
                        total_duration
                    })
                });
            },
        );
    }
    
    group.finish();
}

fn bench_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("latency");
    
    // Memory transport latency
    group.bench_function("memory_roundtrip", |b| {
        b.to_async(&rt).iter(|| async {
            let transport = Arc::new(MemoryTransport::new());
            let addr = "127.0.0.1:44000".parse().unwrap();
            
            let mut listener = transport.listen(addr).await.unwrap();
            let peer = create_test_peer(addr);
            
            let transport_clone = transport.clone();
            let peer_clone = peer.clone();
            
            let connect_task = tokio::spawn(async move {
                transport_clone.connect(&peer_clone).await
            });
            
            let (server_conn, _) = listener.accept().await.unwrap();
            let client_conn = connect_task.await.unwrap().unwrap();
            
            // Round-trip ping-pong
            let ping = b"ping";
            
            let mut client_stream = client_conn.stream();
            client_stream.write_all(ping).await.unwrap();
            client_stream.flush().await.unwrap();
            
            let mut server_stream = server_conn.stream();
            let mut buf = [0u8; 4];
            server_stream.read_exact(&mut buf).await.unwrap();
            server_stream.write_all(b"pong").await.unwrap();
            server_stream.flush().await.unwrap();
            
            client_stream.read_exact(&mut buf).await.unwrap();
            
            black_box(buf);
        });
    });
    
    group.finish();
}

fn bench_concurrent_connections(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_connections");
    
    let connection_counts = vec![10, 50, 100];
    
    for count in connection_counts {
        // Memory transport concurrent connections
        group.bench_with_input(
            BenchmarkId::new("memory", count),
            &count,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let transport = Arc::new(MemoryTransport::new());
                    let addr = "127.0.0.1:45000".parse().unwrap();
                    
                    let mut listener = transport.listen(addr).await.unwrap();
                    let peer = create_test_peer(addr);
                    
                    let mut tasks = vec![];
                    
                    // Spawn acceptor
                    let accept_task = tokio::spawn(async move {
                        let mut connections = vec![];
                        for _ in 0..count {
                            let (conn, _) = listener.accept().await.unwrap();
                            connections.push(conn);
                        }
                        connections
                    });
                    
                    // Spawn connectors
                    for _ in 0..count {
                        let transport = transport.clone();
                        let peer = peer.clone();
                        
                        let task = tokio::spawn(async move {
                            transport.connect(&peer).await.unwrap()
                        });
                        
                        tasks.push(task);
                    }
                    
                    // Wait for all connections
                    let mut client_conns = vec![];
                    for task in tasks {
                        client_conns.push(task.await.unwrap());
                    }
                    
                    let server_conns = accept_task.await.unwrap();
                    
                    black_box((client_conns, server_conns));
                });
            },
        );
    }
    
    group.finish();
}

fn bench_crypto_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("crypto_operations");
    
    let data_sizes = vec![64, 1024, 16384]; // 64B, 1KB, 16KB
    
    for size in data_sizes {
        group.throughput(Throughput::Bytes(size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("encrypt_decrypt", size),
            &size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    use synapsed_net::security::SecurityLayer;
                    use synapsed_net::types::{PeerId, PeerInfo, PeerMetadata};
                    
                    let mut layer = SecurityLayer::new(false).unwrap();
                    let peer = PeerInfo {
                        id: PeerId::new(),
                        address: "127.0.0.1:8080".to_string(),
                        addresses: vec![],
                        protocols: vec![],
                        capabilities: vec![],
                        public_key: None,
                        metadata: PeerMetadata::default(),
                    };
                    
                    // Establish session
                    let handshake = layer.initiate_handshake(&peer).await.unwrap();
                    let session_id = layer.complete_handshake(handshake, &peer).await.unwrap();
                    
                    let data = vec![0xAA; size];
                    
                    // Benchmark encryption + decryption
                    let encrypted = layer.encrypt(&data, &session_id).unwrap();
                    let decrypted = layer.decrypt(&encrypted, &session_id).unwrap();
                    
                    black_box((encrypted, decrypted));
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_connection_establishment,
    bench_throughput,
    bench_latency,
    bench_concurrent_connections,
    bench_crypto_operations
);
criterion_main!(benches);