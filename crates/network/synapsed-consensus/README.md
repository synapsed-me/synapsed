# Synapsed Consensus

Byzantine fault tolerant consensus algorithms for distributed systems in the Synapsed ecosystem.

## Overview

This crate provides production-ready implementations of multiple consensus protocols designed for secure, distributed applications. All protocols support Byzantine fault tolerance up to f < n/3 faulty nodes.

## Supported Algorithms

### HotStuff (Default)
- **Performance**: 1000+ TPS, sub-second finality
- **Features**: Linear message complexity, leader rotation, pipelining
- **Use Case**: High-throughput applications requiring fast finality

### Practical Byzantine Fault Tolerance (PBFT)
- **Performance**: 500+ TPS, 3-5 second finality  
- **Features**: Three-phase protocol, view changes, message authentication
- **Use Case**: Critical systems requiring proven Byzantine fault tolerance

### Tendermint
- **Performance**: 800+ TPS, immediate finality
- **Features**: Instant finality, fork accountability, evidence handling
- **Use Case**: Applications requiring immediate transaction finality

### Avalanche (Experimental)
- **Performance**: 2000+ TPS, probabilistic finality
- **Features**: DAG-based, high parallelism, leaderless
- **Use Case**: High-throughput systems with network partition tolerance

## Quick Start

```rust
use synapsed_consensus::{HotStuffConsensus, ConsensusConfig, NodeId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create node identities
    let node1 = NodeId::new();
    let node2 = NodeId::new();
    let node3 = NodeId::new();
    let node4 = NodeId::new();
    
    // Configure consensus for 4 nodes (f=1)
    let config = ConsensusConfig::new(
        node1.clone(), 
        vec![node1, node2, node3, node4]
    );
    
    // Start HotStuff consensus
    let mut consensus = HotStuffConsensus::new(config).await?;
    consensus.start().await?;
    
    Ok(())
}
```

## Architecture

The consensus system is built on several key abstractions:

### ConsensusProtocol Trait
Core interface implemented by all consensus algorithms:
```rust
#[async_trait]
pub trait ConsensusProtocol {
    async fn start(&mut self) -> Result<()>;
    async fn propose_block(&mut self, transactions: Vec<Transaction>) -> Result<Block>;
    async fn handle_vote(&mut self, vote: Vote) -> Result<()>;
    // ... more methods
}
```

### StateMachine Trait
Application state management:
```rust
#[async_trait]
pub trait StateMachine {
    async fn apply_block(&mut self, block: &Block) -> Result<()>;
    async fn validate_block(&self, block: &Block) -> Result<bool>;
    // ... more methods
}
```

### NetworkTransport Trait
Network communication abstraction:
```rust
#[async_trait]
pub trait NetworkTransport {
    async fn broadcast(&self, message: ConsensusMessage) -> Result<()>;
    async fn send_to(&self, peer: NodeId, message: ConsensusMessage) -> Result<()>;
    // ... more methods
}
```

## Security Features

### Cryptographic Security
- **Signatures**: Ed25519 digital signatures for all votes and proposals
- **Hashing**: SHA-256 for block and message integrity
- **Randomness**: Cryptographically secure random number generation

### Byzantine Fault Tolerance
- **Safety**: Never commit conflicting blocks
- **Liveness**: Always make progress with ≥ 2f+1 honest nodes
- **Finality**: Committed blocks are immutable

### Network Security
- **Message Authentication**: All messages cryptographically signed
- **Replay Protection**: Sequence numbers and timestamps
- **View Synchronization**: Handles network partitions and timing attacks

## Performance Characteristics

| Algorithm | TPS | Finality | Message Complexity | Memory Usage |
|-----------|-----|----------|-------------------|--------------|
| HotStuff  | 1000+ | < 1s | O(n) | O(n) |
| PBFT      | 500+ | 3-5s | O(n²) | O(n²) |
| Tendermint| 800+ | Immediate | O(n²) | O(n) |
| Avalanche | 2000+ | Probabilistic | O(k log n) | O(n) |

## Configuration

### Basic Configuration
```rust
let config = ConsensusConfig::new(node_id, validators)
    .with_byzantine_threshold(1)  // f=1, supports 4 nodes
    .with_max_transactions(1000)  // Max transactions per block
    .with_fast_path(true);        // Enable optimizations
```

### Timeout Configuration
```rust
let timeouts = TimeoutConfig {
    proposal_timeout_ms: 1000,
    vote_timeout_ms: 500,
    view_change_timeout_ms: 2000,
    base_timeout_ms: 1000,
    timeout_multiplier: 1.5,
};

let config = ConsensusConfig::new(node_id, validators)
    .with_timeouts(timeouts);
```

## Testing

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
cargo test --test integration_tests
```

### Chaos Testing
```bash
cargo test --test chaos_tests --features chaos
```

### Performance Benchmarks
```bash
cargo bench
```

## Dependencies

### Core Dependencies
- `tokio`: Async runtime
- `serde`: Serialization
- `ed25519-dalek`: Digital signatures
- `ring`: Cryptographic primitives

### Internal Dependencies
- `synapsed-core`: Shared utilities
- `synapsed-crypto`: Cryptographic operations

## Features

- `default`: Enables HotStuff and PBFT
- `hotstuff`: HotStuff consensus algorithm
- `pbft`: PBFT consensus algorithm  
- `tendermint`: Tendermint consensus algorithm
- `avalanche`: Avalanche consensus algorithm (experimental)

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT License

at your option.

## Contributing

Contributions welcome! Please see the main Synapsed repository for contribution guidelines.

## Security

Found a security issue? Please report it privately to the Synapsed security team.