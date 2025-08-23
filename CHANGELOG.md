# Changelog

All notable changes to the Synapsed framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added 
- **Promise Theory Implementation**
  - Voluntary cooperation protocol with willingness evaluation
  - Causal independence verification for true agent autonomy
  - Semantic spacetime contexts for promises
  - Promise chemistry for interaction modeling
  - FIPA ACL performatives for semantic agent communication
  - Conversation state management for multi-turn dialogues

- **Hybrid Memory Architecture**
  - Vector memory with 768-dimensional embeddings and cosine similarity
  - Episodic memory for sequential experience storage
  - Semantic memory with knowledge graphs and relationship inference
  - Working memory with attention-based management
  - Automatic memory consolidation between types

- **Adaptive Permission System**
  - Trust scoring that evolves based on agent behavior
  - Learning engine for discovering successful patterns
  - Hierarchical delegation chain for permission escalation
  - Context-aware decision making with resource monitoring

- **Safety Integration**
  - SafeVerifiedExecutor framework design
  - Circuit breaker patterns for failure prevention
  - Resource guards for automatic cleanup
  - Critical sections for atomic operations
  - Integration points for synapsed-safety crate

- **Claude Code Integration**
  - Hooks configuration for intent capture and verification
  - MCP server tools for agent management
  - Session lifecycle management

### Added (Earlier)
- Complete Substrates observability framework implementation aligned with Humainary's vision
- Intent verification system with hierarchical intent trees
- Promise Theory implementation for autonomous agent cooperation
- MCP (Model Context Protocol) server for AI agent integration
- WASM bindings for browser deployment
- Comprehensive example applications
- GitHub Actions CI/CD workflows
- Complete documentation for all crates

### Changed
- Refactored Substrates to use correct emission flow pattern (Channel → Pipe → Emission)
- Updated intent system to integrate with Substrates observability
- Improved trust model with dynamic scoring
- Enhanced documentation to reflect current agent-focused architecture

### Fixed
- Fixed Arc mutability issues in Substrates tests
- Resolved compilation errors in intent and promise modules
- Fixed WASM package naming conflicts
- Fixed memory module warnings (unused parentheses, unused variables)

## [0.1.0] - 

### Added
- Initial framework structure with layered architecture
- Core infrastructure crates (synapsed-core, synapsed-crypto, synapsed-gpu)
- Observability layer (synapsed-substrates, synapsed-serventis)
- Intent verification layer (synapsed-intent, synapsed-promise, synapsed-verify)
- Network layer (synapsed-net, synapsed-consensus, synapsed-routing)
- Storage layer (synapsed-storage, synapsed-crdt)
- Security layer (synapsed-identity, synapsed-safety)
- Compute layer (synapsed-wasm, synapsed-neural-core)
- Application layer (synapsed-payments, synapsed-mcp)

### Core Features

#### Observability (Substrates)
- Event circuits based on Humainary's Substrates API
- Proper separation of concerns: Subjects, Channels, Pipes, Emissions
- Percepts with Composers for type-safe wrappers
- Queue and Script execution with priorities
- Multiple sink patterns (Basic, Filtered, Batching)
- Subscription model with managed sources

#### Intent Verification
- Hierarchical intent trees for complex agent planning
- Multiple verification strategies (Command, FileSystem, API, Composite)
- Integration with observability for full tracking
- Cryptographic proof generation
- Context boundary enforcement

#### Promise Theory
- Autonomous agent implementation
- Voluntary promise lifecycle (Proposed → Accepted → Fulfilled/Violated)
- Trust model with reputation scoring
- Cooperation protocols
- Imposition handling

#### Network & Security
- Multi-transport support (TCP, QUIC, WebSocket, WebRTC)
- Post-quantum cryptography ready (Kyber, Dilithium)
- Privacy layers (Onion routing, Mix networks)
- HotStuff consensus implementation
- Advanced routing algorithms

#### Storage & Data
- Multi-backend support (RocksDB, SQLite, IndexedDB)
- CRDT implementations for conflict-free replicated data
- Encryption at rest
- Distributed storage capabilities

#### WASM Support
- Browser-compatible bindings
- Full framework functionality in WASM
- Support for web, bundler, and Node.js targets
- Optimized for size and performance

### Development
- Comprehensive test coverage
- Benchmark suite for performance testing
- Example applications demonstrating all features
- Full API documentation
- CI/CD pipeline with GitHub Actions

### Security
- All agent claims must be verifiable
- Context boundaries strictly enforced
- Cryptographic proofs for all verifications
- Post-quantum ready algorithms
- Privacy-preserving observability

## Migration Guide

### From IntentProof
The Synapsed framework incorporates and extends the IntentProof concepts:
- Intent verification is now in `synapsed-intent`
- Promise Theory is in `synapsed-promise`
- Verification strategies are in `synapsed-verify`
- All integrated with Substrates observability

### Breaking Changes
- Substrates emission pattern changed: emissions now flow through pipes, not directly from subjects
- Intent IDs now use UUID wrapper type
- Promise states are immutable transitions
- Trust scores use 0.0-1.0 range

## Future Roadmap

### v0.2.0 (Planned)
- [ ] Complete synapsed-enforce for context boundary enforcement
- [ ] Add synapsed-cli for command-line interface
- [ ] Implement advanced CRDT algorithms
- [ ] Add more verification strategies
- [ ] Enhanced MCP tool catalog

### v0.3.0 (Planned)
- [ ] Multi-agent orchestration
- [ ] Distributed intent execution
- [ ] Cross-platform mobile support
- [ ] Advanced privacy features
- [ ] Performance optimizations

## Contributors

- Synapsed Team
- IntentProof contributors
- Community contributors

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

- William Louth and Humainary for the Substrates observability vision
- Mark Burgess for Promise Theory
- The Rust community for excellent tooling and libraries