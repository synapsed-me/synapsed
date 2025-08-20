# Synapsed Routing

Anonymous onion routing and P2P communication protocols for privacy-preserving applications in the Synapsed ecosystem.

## Overview

This crate provides production-ready anonymous routing protocols designed for maximum privacy and traffic analysis resistance. All communications are cryptographically protected with multiple layers of encryption.

## Supported Protocols

### Onion Routing (Default)
- **Privacy**: Multi-hop routing with layered encryption
- **Features**: Circuit construction, path diversity, traffic padding
- **Use Case**: Anonymous communication with strong privacy guarantees

### Kademlia DHT
- **Performance**: O(log n) routing complexity
- **Features**: Distributed peer discovery, self-organizing network
- **Use Case**: Decentralized peer discovery and content routing

### Mix Networks
- **Privacy**: Traffic analysis resistance through mixing
- **Features**: Uniform packet sizes, batch processing, cover traffic
- **Use Case**: Maximum anonymity for high-security applications

## Quick Start

```rust
use synapsed_routing::{OnionRouter, RouterConfig, Circuit};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure 3-hop onion routing
    let config = RouterConfig::new()
        .with_hop_count(3)
        .with_circuit_lifetime(600)  // 10 minutes
        .with_padding_enabled(true);
    
    // Create router
    let mut router = OnionRouter::new(config).await?;
    
    // Establish anonymous circuit
    let circuit = router.create_circuit().await?;
    
    // Send anonymous message
    let response = router.send_message(
        &circuit,
        b"Hello anonymous world!"
    ).await?;
    
    println!("Response: {:?}", response);
    Ok(())
}
```

## Architecture

### Router Trait
Core interface for all routing protocols:
```rust
#[async_trait]
pub trait Router {
    async fn create_circuit(&mut self) -> Result<Circuit>;
    async fn send_message(&self, circuit: &Circuit, data: &[u8]) -> Result<Vec<u8>>;
    async fn close_circuit(&mut self, circuit: &Circuit) -> Result<()>;
}
```

### Directory Service
Decentralized node discovery:
```rust
#[async_trait]
pub trait DirectoryService {
    async fn discover_nodes(&self, count: usize) -> Result<Vec<NodeInfo>>;
    async fn get_node_info(&self, node_id: &NodeId) -> Result<NodeInfo>;
    async fn publish_node(&self, info: NodeInfo) -> Result<()>;
}
```

## Privacy Features

### Onion Encryption
- **Layered Encryption**: Each hop adds/removes one encryption layer
- **Forward Secrecy**: Ephemeral keys for each circuit
- **Padding**: Uniform packet sizes to prevent traffic analysis

### Circuit Management
- **Path Selection**: Cryptographically secure random path selection
- **Circuit Rotation**: Automatic circuit renewal for long-lived connections
- **Load Balancing**: Distribute traffic across multiple circuits

### Traffic Analysis Resistance
- **Cover Traffic**: Dummy traffic to mask communication patterns
- **Timing Obfuscation**: Random delays to prevent timing attacks
- **Batch Processing**: Process messages in batches to hide patterns

## Configuration

### Basic Onion Routing
```rust
let config = RouterConfig::new()
    .with_hop_count(3)                    // 3-hop circuits
    .with_circuit_lifetime(600)           // 10 minute lifetime
    .with_padding_enabled(true)           // Enable traffic padding
    .with_directory_cache_ttl(3600);      // 1 hour directory cache
```

### Mix Network Configuration
```rust
let mix_config = MixNetConfig::new()
    .with_batch_size(100)                 // Mix 100 messages at once
    .with_batch_timeout(5000)             // 5 second batching window
    .with_cover_traffic_rate(0.1)         // 10% cover traffic
    .with_uniform_packet_size(1024);      // 1KB uniform packets
```

## Performance Characteristics

| Protocol | Latency | Anonymity | Scalability | Use Case |
|----------|---------|-----------|-------------|----------|
| Onion | 300-500ms | High | Good | General anonymity |
| Kademlia | 50-100ms | Low | Excellent | Peer discovery |
| MixNet | 5-10s | Maximum | Fair | High-security |

## Security Properties

### Anonymity Guarantees
- **Sender Anonymity**: Entry node cannot identify sender
- **Receiver Anonymity**: Exit node cannot identify final destination
- **Unlinkability**: Cannot link senders to receivers

### Traffic Analysis Resistance
- **Timing Attacks**: Protected by random delays and batching
- **Size Attacks**: Protected by uniform packet sizes
- **Volume Attacks**: Protected by cover traffic

### Cryptographic Security
- **Forward Secrecy**: Past communications remain secure if keys compromised
- **End-to-End Encryption**: Additional layer beyond onion encryption
- **Authentication**: Each hop authenticates the next

## Testing

```bash
# Unit tests
cargo test

# Integration tests with real network
cargo test --test integration_tests

# Performance benchmarks
cargo bench

# Privacy analysis tests
cargo test --features privacy-analysis
```

## Dependencies

### Core Dependencies
- `x25519-dalek`: Elliptic curve Diffie-Hellman
- `chacha20poly1305`: Authenticated encryption
- `ed25519-dalek`: Digital signatures

### Internal Dependencies
- `synapsed-core`: Shared utilities
- `synapsed-crypto`: Cryptographic operations

## Features

- `default`: Enables onion, kademlia, and mixnet protocols
- `onion`: Onion routing implementation
- `kademlia`: Kademlia DHT implementation  
- `mixnet`: Mix network implementation
- `tor-compatible`: Tor network compatibility layer

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT License

## Security Notice

This crate provides strong privacy guarantees but should be audited before use in production systems. Network-level attacks and timing analysis may still be possible depending on your threat model.