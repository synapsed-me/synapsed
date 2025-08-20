//! Common test utilities and helpers.

use synapsed_net::{
    error::Result,
    transport::{Connection, Transport},
    types::{NetworkAddress, PeerId, PeerInfo, PeerMetadata},
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Barrier;
use tokio::time::timeout;
use tracing_subscriber::EnvFilter;

/// Initialize test logging
pub fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("synapsed_net=debug".parse().unwrap()))
        .with_test_writer()
        .try_init();
}

/// Create a test peer with random ID
pub fn create_test_peer(addr: SocketAddr) -> PeerInfo {
    PeerInfo {
        id: PeerId::new(),
        address: addr.to_string(),
        addresses: vec![NetworkAddress::Socket(addr)],
        protocols: vec!["test/1.0".to_string()],
        capabilities: vec![],
        public_key: None,
        metadata: PeerMetadata::default(),
    }
}

/// Create a pair of test peers
pub fn create_test_peer_pair() -> (PeerInfo, PeerInfo) {
    let peer1 = create_test_peer("127.0.0.1:9001".parse().unwrap());
    let peer2 = create_test_peer("127.0.0.1:9002".parse().unwrap());
    (peer1, peer2)
}

/// Test data generator for various sizes
pub fn generate_test_data(size: usize) -> Vec<u8> {
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut data = vec![0u8; size];
    rng.fill_bytes(&mut data);
    data
}

/// Pattern-based test data for property testing
pub fn generate_pattern_data(pattern: DataPattern, size: usize) -> Vec<u8> {
    match pattern {
        DataPattern::Zeros => vec![0u8; size],
        DataPattern::Ones => vec![0xFFu8; size],
        DataPattern::Alternating => (0..size).map(|i| if i % 2 == 0 { 0x55 } else { 0xAA }).collect(),
        DataPattern::Random => generate_test_data(size),
        DataPattern::Sequential => (0..size).map(|i| (i % 256) as u8).collect(),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DataPattern {
    Zeros,
    Ones,
    Alternating,
    Random,
    Sequential,
}

/// Connection test harness for bidirectional communication
pub struct ConnectionTestHarness {
    pub conn1: Connection,
    pub conn2: Connection,
    pub barrier: Arc<Barrier>,
}

impl ConnectionTestHarness {
    /// Create a new test harness with two connected endpoints
    pub async fn new<T: Transport>(transport: Arc<T>, addr1: SocketAddr, addr2: SocketAddr) -> Result<Self> {
        let barrier = Arc::new(Barrier::new(2));
        
        // Start listeners
        let mut listener1 = transport.listen(addr1).await?;
        let mut listener2 = transport.listen(addr2).await?;
        
        let peer1 = create_test_peer(addr1);
        let peer2 = create_test_peer(addr2);
        
        // Connect in both directions
        let connect_barrier = barrier.clone();
        let transport_clone = transport.clone();
        let peer2_clone = peer2.clone();
        
        let connect_task = tokio::spawn(async move {
            connect_barrier.wait().await;
            transport_clone.connect(&peer2_clone).await
        });
        
        let accept_barrier = barrier.clone();
        let accept_task = tokio::spawn(async move {
            accept_barrier.wait().await;
            listener2.accept().await.map(|(conn, _)| conn)
        });
        
        let conn1 = connect_task.await??;
        let conn2 = accept_task.await??;
        
        Ok(Self {
            conn1,
            conn2,
            barrier,
        })
    }
    
    /// Test bidirectional data transfer
    pub async fn test_bidirectional_transfer(&mut self, data1: &[u8], data2: &[u8]) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        
        let data1_clone = data1.to_vec();
        let data2_clone = data2.to_vec();
        
        // Send data1 from conn1 to conn2
        let send_task = tokio::spawn(async move {
            let mut stream = self.conn1.stream();
            stream.write_all(&data1_clone).await?;
            stream.flush().await
        });
        
        // Send data2 from conn2 to conn1
        let recv_task = tokio::spawn(async move {
            let mut stream = self.conn2.stream();
            let mut received = vec![0u8; data1.len()];
            stream.read_exact(&mut received).await?;
            assert_eq!(&received, data1);
            
            stream.write_all(&data2_clone).await?;
            stream.flush().await
        });
        
        send_task.await??;
        recv_task.await??;
        
        Ok(())
    }
}

/// Measure connection establishment time
pub async fn measure_connection_time<T: Transport>(
    transport: Arc<T>,
    peer: &PeerInfo,
) -> Result<Duration> {
    let start = std::time::Instant::now();
    let _ = transport.connect(peer).await?;
    Ok(start.elapsed())
}

/// Test concurrent connections
pub async fn test_concurrent_connections<T: Transport>(
    transport: Arc<T>,
    listener_addr: SocketAddr,
    num_connections: usize,
) -> Result<Vec<Duration>> {
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
    for i in 0..num_connections {
        let transport = transport.clone();
        let barrier = barrier.clone();
        let peer = create_test_peer(listener_addr);
        
        let task = tokio::spawn(async move {
            barrier.wait().await;
            let start = std::time::Instant::now();
            let _ = transport.connect(&peer).await?;
            Ok::<_, synapsed_net::error::NetworkError>(start.elapsed())
        });
        
        connect_tasks.push(task);
    }
    
    // Wait for all connections
    let mut durations = vec![];
    for task in connect_tasks {
        durations.push(task.await??);
    }
    
    let _ = accept_task.await??;
    
    Ok(durations)
}

/// Network condition simulator for chaos testing
#[derive(Debug, Clone)]
pub struct NetworkCondition {
    /// Packet loss percentage (0-100)
    pub packet_loss: u8,
    /// Additional latency in milliseconds
    pub latency_ms: u64,
    /// Bandwidth limit in Mbps
    pub bandwidth_mbps: Option<f64>,
    /// Jitter in milliseconds
    pub jitter_ms: u64,
    /// Whether to simulate connection drops
    pub random_disconnects: bool,
}

impl Default for NetworkCondition {
    fn default() -> Self {
        Self {
            packet_loss: 0,
            latency_ms: 0,
            bandwidth_mbps: None,
            jitter_ms: 0,
            random_disconnects: false,
        }
    }
}

impl NetworkCondition {
    /// Create a lossy network condition
    pub fn lossy(packet_loss: u8) -> Self {
        Self {
            packet_loss,
            ..Default::default()
        }
    }
    
    /// Create a high-latency network condition
    pub fn high_latency(latency_ms: u64) -> Self {
        Self {
            latency_ms,
            ..Default::default()
        }
    }
    
    /// Create a bandwidth-limited network condition
    pub fn limited_bandwidth(bandwidth_mbps: f64) -> Self {
        Self {
            bandwidth_mbps: Some(bandwidth_mbps),
            ..Default::default()
        }
    }
    
    /// Create an unstable network condition
    pub fn unstable() -> Self {
        Self {
            packet_loss: 10,
            latency_ms: 100,
            jitter_ms: 50,
            random_disconnects: true,
            ..Default::default()
        }
    }
}

/// Test timeout helper
pub async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    timeout(duration, future)
        .await
        .map_err(|_| synapsed_net::error::NetworkError::Timeout)?
}

/// Assert that an operation completes within a time limit
pub async fn assert_completes_within<F, T>(duration: Duration, future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    timeout(duration, future)
        .await
        .expect("Operation did not complete within time limit")
}

/// Property-based testing helper for connection properties
pub async fn check_connection_property<T, F, P>(
    transport: Arc<T>,
    property_fn: F,
    iterations: usize,
) -> Result<()>
where
    T: Transport,
    F: Fn(&Connection) -> P,
    P: std::future::Future<Output = Result<bool>>,
{
    for _ in 0..iterations {
        let addr = "127.0.0.1:0".parse().unwrap();
        let mut listener = transport.listen(addr).await?;
        let local_addr = listener.local_addr()?;
        let peer = create_test_peer(local_addr);
        
        let transport_clone = transport.clone();
        let peer_clone = peer.clone();
        
        let connect_task = tokio::spawn(async move {
            transport_clone.connect(&peer_clone).await
        });
        
        let (conn, _) = listener.accept().await?;
        let _ = connect_task.await??;
        
        if !property_fn(&conn).await? {
            return Err(synapsed_net::error::NetworkError::Protocol(
                "Property check failed".to_string()
            ));
        }
    }
    
    Ok(())
}

/// Generate test certificate for QUIC/TLS testing
pub fn generate_test_cert() -> (Vec<Vec<u8>>, Vec<u8>) {
    // Simplified test certificate generation
    // In production tests, use rcgen or similar
    let cert = vec![vec![0u8; 256]]; // Mock certificate
    let key = vec![0u8; 32]; // Mock private key
    (cert, key)
}

/// Performance measurement helpers
pub struct PerfStats {
    pub min: Duration,
    pub max: Duration,
    pub mean: Duration,
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
}

impl PerfStats {
    pub fn from_durations(mut durations: Vec<Duration>) -> Self {
        durations.sort();
        let len = durations.len();
        
        let sum: Duration = durations.iter().sum();
        let mean = sum / len as u32;
        
        Self {
            min: durations[0],
            max: durations[len - 1],
            mean,
            p50: durations[len / 2],
            p95: durations[(len * 95) / 100],
            p99: durations[(len * 99) / 100],
        }
    }
}