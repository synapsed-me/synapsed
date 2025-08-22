# Synapsed 🧠

> **Next-Generation Distributed Systems Framework** - Observable, Verifiable, Secure by Design

[![Crates.io](https://img.shields.io/crates/v/synapsed-core.svg)](https://crates.io/crates/synapsed-core)
[![Documentation](https://docs.rs/synapsed-core/badge.svg)](https://docs.rs/synapsed-core)
[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](LICENSE)
[![Build Status](https://github.com/synapsed-me/synapsed/workflows/CI/badge.svg)](https://github.com/synapsed-me/synapsed/actions)

## Overview

Synapsed is a comprehensive Rust framework for building distributed, observable, and verifiable systems. It combines cutting-edge technologies including:

- **Observable-First Architecture**: Built on Humainary-inspired Substrates and Serventis
- **Intent Verification**: Hierarchical intent trees with Promise Theory
- **Post-Quantum Cryptography**: Kyber and Dilithium implementations with GPU acceleration
- **Distributed Primitives**: Consensus, CRDTs, P2P networking
- **Privacy & Security**: DIDs, ZKP, multi-layer privacy
- **High Performance**: SIMD optimizations, GPU acceleration, zero-copy patterns

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│                     Synapsed Ecosystem                     │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  ┌──────────────────┐  ┌──────────────────┐                │
│  │  Observability   │  │ Intent Framework │                │
│  │  ┌────────────┐  │  │  ┌────────────┐  │                │
│  │  │ Substrates │  │  │  │   Intent   │  │                │
│  │  └────────────┘  │  │  └────────────┘  │                │
│  │  ┌────────────┐  │  │  ┌────────────┐  │                │
│  │  │ Serventis  │  │  │  │  Promise   │  │                │
│  │  └────────────┘  │  │  └────────────┘  │                │
│  └──────────────────┘  │  ┌────────────┐  │                │
│                        │  │   Verify   │  │                │
│  ┌──────────────────┐  │  └────────────┘  │                │
│  │  Core Platform   │  │  ┌────────────┐  │                │
│  │  ┌────────────┐  │  │  │  Enforce   │  │                │
│  │  │    Core    │  │  │  └────────────┘  │                │
│  │  └────────────┘  │  └──────────────────┘                │
│  │  ┌────────────┐  │                                      │
│  │  │   Crypto   │  │  ┌──────────────────┐                │
│  │  └────────────┘  │  │    Networking    │                │
│  │  ┌────────────┐  │  │  ┌────────────┐  │                │
│  │  │    GPU     │  │  │  │    Net     │  │                │
│  │  └────────────┘  │  │  └────────────┘  │                │
│  └──────────────────┘  │  ┌────────────┐  │                │
│                        │  │ Consensus  │  │                │
│  ┌──────────────────┐  │  └────────────┘  │                │
│  │  Storage & Data  │  └──────────────────┘                │
│  │  ┌────────────┐  │                                      │
│  │  │  Storage   │  │  ┌──────────────────┐                │
│  │  └────────────┘  │  │  Security & ID   │                │
│  │  ┌────────────┐  │  │  ┌────────────┐  │                │
│  │  │    CRDT    │  │  │  │  Identity  │  │                │
│  │  └────────────┘  │  │  └────────────┘  │                │
│  └──────────────────┘  │  ┌────────────┐  │                │
│                        │  │   Safety   │  │                │
│                        │  └────────────┘  │                │
│                        └──────────────────┘                │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

## Crates

### Observability Foundation
- **[synapsed-substrates](crates/observability/synapsed-substrates)** - Event circuits and observability fabric
- **[synapsed-serventis](crates/observability/synapsed-serventis)** - Service-level monitoring and probes

### Core Infrastructure
- **[synapsed-core](crates/core/synapsed-core)** - Base traits, memory management, runtime
- **[synapsed-crypto](crates/core/synapsed-crypto)** - Post-quantum cryptography (Kyber, Dilithium)
- **[synapsed-gpu](crates/core/synapsed-gpu)** - GPU acceleration for compute-intensive operations

### Storage & Data
- **[synapsed-storage](crates/storage/synapsed-storage)** - Multi-backend observable storage
- **[synapsed-crdt](crates/storage/synapsed-crdt)** - Conflict-free replicated data types

### Networking
- **[synapsed-net](crates/network/synapsed-net)** - P2P, WebRTC, QUIC with privacy layers
- **[synapsed-consensus](crates/network/synapsed-consensus)** - HotStuff consensus implementation
- **[synapsed-routing](crates/network/synapsed-routing)** - Advanced routing algorithms

### Security & Identity
- **[synapsed-identity](crates/security/synapsed-identity)** - DIDs, auth, MFA with ZKP
- **[synapsed-safety](crates/security/synapsed-safety)** - Runtime safety and rollback

### Intent Framework
- **[synapsed-intent](crates/intent/synapsed-intent)** - Hierarchical intent trees
- **[synapsed-promise](crates/intent/synapsed-promise)** - Promise Theory implementation
- **[synapsed-verify](crates/intent/synapsed-verify)** - Multi-strategy verification
- **[synapsed-enforce](crates/intent/synapsed-enforce)** - Context enforcement

### Compute & Runtime
- **[synapsed-wasm](crates/compute/synapsed-wasm)** - WASM runtime and PWA support
- **[synapsed-neural-core](crates/compute/synapsed-neural-core)** - Neural network primitives

### Applications
- **[synapsed-payments](crates/applications/synapsed-payments)** - Anonymous payment processing
- **[synapsed-mcp](crates/applications/synapsed-mcp)** - Model Context Protocol server
- **[synapsed-cli](crates/applications/synapsed-cli)** - Command-line interface

## Quick Start

### Installation

```bash
# Install the CLI
cargo install synapsed-cli

# Or add to your Cargo.toml
[dependencies]
synapsed-core = "0.1"
synapsed-intent = "0.1"
synapsed-substrates = "0.1"
```

### Basic Usage

```rust
use synapsed_intent::{HierarchicalIntent, IntentTree};
use synapsed_promise::{Promise, TrustModel};
use synapsed_substrates::{Circuit, Subject};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an observable intent
    let intent = HierarchicalIntent::new("Deploy service")
        .requires("All tests pass")
        .step("Build container", "docker build -t app .")
        .step("Push to registry", "docker push app")
        .ensures("Service healthy", "curl health.check");
    
    // Make it observable
    let circuit = Circuit::new()
        .subject(Subject::from("deployment"))
        .observe(intent.clone());
    
    // Execute with verification
    let result = intent.execute().await?;
    assert!(result.verified);
    
    Ok(())
}
```

### Observable Services

```rust
use synapsed_serventis::{Service, Monitor, Signal};
use synapsed_substrates::{Substrate, Pipe};

// Create an observable service
let service = Service::new("api-gateway")
    .monitor(Monitor::health("/health"))
    .monitor(Monitor::latency(Duration::from_millis(100)))
    .pipe(Pipe::to("metrics-collector"));

// Start with automatic observability
service.start().await?;
```

## Features

### 🔍 Observable by Design
Every component emits structured observability data through Substrates circuits, enabling deep system understanding without performance overhead.

### 🎯 Intent Verification
Hierarchical intent trees ensure AI agents and distributed systems do what they claim, with cryptographic proof of execution.

### 🔐 Post-Quantum Security
Ready for quantum computing threats with Kyber (KEM) and Dilithium (signatures), GPU-accelerated for performance.

### 🌐 Privacy-First Networking
Multiple privacy layers including Tor integration, mix networks, and onion routing built into the networking stack.

### ⚡ High Performance
- SIMD optimizations for crypto operations
- GPU acceleration via CUDA/OpenCL
- Zero-copy patterns throughout
- Lock-free data structures

### 🔄 Distributed Primitives
- HotStuff consensus for agreement
- CRDTs for eventual consistency
- P2P mesh networking with libp2p
- WebRTC for browser compatibility

## Documentation

- [Architecture Guide](docs/architecture.md)
- [API Documentation](https://docs.rs/synapsed-core)
- [Examples](examples/)
- [Performance Tuning](docs/performance.md)
- [Security Considerations](docs/security.md)

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# Clone the repository
git clone https://github.com/synapsed-me/synapsed
cd synapsed

# Build all crates
cargo build --all

# Run tests
cargo test --all

# Run benchmarks
cargo bench --all
```

## Benchmarks

| Operation | Time | Throughput |
|-----------|------|------------|
| Kyber768 Keygen | 35μs | 28k ops/sec |
| Intent Verification | 125μs | 8k ops/sec |
| CRDT Merge | 18μs | 55k ops/sec |
| Observable Event | 250ns | 4M ops/sec |

## License

This project is dual-licensed under MIT and Apache 2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details.

## Acknowledgments

- William Louth and the Humainary project for observability concepts
- Promise Theory by Mark Burgess
- The Rust community for excellent libraries

## Contact

- GitHub: [@synapsed-me](https://github.com/synapsed-me)
- Email: team@synapsed.me
- Discord: [Join our server](https://discord.gg/synapsed)

---

Built with ❤️ by the Synapsed Team