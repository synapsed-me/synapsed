# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Synapsed is a Rust framework for building distributed, observable, and verifiable systems with a focus on preventing AI agent context escaping and ensuring verifiable execution. The project specifically addresses the problem where Claude sub-agents can lose context, make false claims, and lack verification mechanisms.

## Core Architecture

The system is organized into layered crates:

1. **Observability Layer** (`crates/observability/`)
   - `synapsed-substrates`: Event circuits based on Humainary's Substrates API
   - `synapsed-serventis`: Service-level monitoring and probes

2. **Intent Verification Layer** (`crates/intent/`)
   - `synapsed-intent`: Hierarchical intent trees for agent planning
   - `synapsed-promise`: Promise Theory implementation for autonomous agent cooperation
   - `synapsed-verify`: Multi-strategy verification for agent claims
   - `synapsed-enforce`: Context boundary enforcement (planned)

3. **Core Infrastructure** (`crates/core/`)
   - `synapsed-core`: Base traits, memory management, runtime
   - `synapsed-crypto`: Post-quantum cryptography (Kyber, Dilithium)
   - `synapsed-gpu`: GPU acceleration for compute operations

4. **Network Layer** (`crates/network/`)
   - `synapsed-net`: P2P, WebRTC, QUIC with privacy layers
   - `synapsed-consensus`: HotStuff consensus implementation
   - `synapsed-routing`: Advanced routing algorithms

## Build and Development Commands

```bash
# Build all crates
cargo build --all

# Run all tests
cargo test --all --all-features

# Run tests for a specific crate
cargo test -p synapsed-intent

# Run a single test
cargo test -p synapsed-promise test_trust_model -- --exact

# Format code
cargo fmt --all

# Lint with clippy
cargo clippy --all-targets --all-features -- -D warnings

# Check compilation for all crates
cargo check --all

# Check a specific crate (faster iteration)
cargo check -p synapsed-verify

# Build documentation
cargo doc --all --all-features --open

# Run benchmarks
cargo bench --all

# Security audit
cargo audit
```

## Intent Verification System

The project implements a hierarchical intent verification system to prevent Claude sub-agents from escaping context:

1. **Intent Declaration**: Agents must declare intentions before acting
2. **Promise Contracts**: Based on Promise Theory - agents make voluntary promises about behavior
3. **Verification Strategies**: 
   - Command execution verification (sandboxed)
   - File system state verification
   - Network/API response verification
   - Cryptographic proof generation
4. **Context Boundaries**: Enforced through the cooperation protocol

Key principle: Every agent claim must be verifiable against external reality, not self-assessment.

## Working with Promise Theory

When implementing agent cooperation:
- Agents are autonomous and cannot be coerced
- Promises are voluntary declarations about own behavior
- Impositions from other agents must be explicitly accepted
- Trust is built through verified promise fulfillment
- Use `synapsed-promise::AutonomousAgent` for agent implementation

## Trait Hierarchy

All major components implement these core traits from `synapsed-core`:
- `Identifiable`: Unique identification
- `Observable`: Status, health, metrics
- `Validatable`: Self-validation capability
- `Configurable`: Runtime configuration

## Testing Patterns

```rust
// Use tokio::test for async tests
#[tokio::test]
async fn test_agent_cooperation() {
    let agent = AutonomousAgent::new(config);
    agent.initialize().await.unwrap();
    // Test implementation
}

// Integration tests go in tests/ directory of each crate
// Unit tests go in src/ files with #[cfg(test)] modules
```

## Critical Implementation Notes

1. **Context Injection for Sub-Agents**: When spawning sub-agents, always include:
   - Parent context serialization
   - Verification requirements
   - Trust boundaries
   - Allowed operations scope

2. **Verification First**: Before claiming success, verify:
   - Commands actually executed
   - Files actually exist/were modified
   - Network calls actually succeeded
   - State matches expectations

3. **Observable by Design**: All components emit events through Substrates circuits - use the substrate integration for monitoring.

4. **Post-Quantum Ready**: Use `synapsed-crypto` for any cryptographic operations to ensure quantum resistance.

## Common Pitfalls to Avoid

- Don't trust agent self-reporting - always verify externally
- Don't skip precondition/postcondition checks in intents
- Don't bypass the trust model when evaluating impositions
- Don't create promises that exceed agent capabilities

## MCP Integration

The `synapsed-mcp` crate provides Model Context Protocol server for Claude integration with tools:
- `intent_declare`: Declare intentions before acting
- `intent_verify`: Verify execution against declaration
- `context_inject`: Pass context to sub-agents
- `trust_check`: Query agent trust levels