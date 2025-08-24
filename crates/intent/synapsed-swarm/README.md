# Synapsed Swarm

Unified swarm coordination system integrating Intent, Promise Theory, and Verification for reliable multi-agent collaboration.

## Overview

This crate provides a complete framework for coordinating multiple AI agents (including Claude sub-agents) using:
- **Intent System**: Hierarchical task planning and delegation
- **Promise Theory**: Voluntary cooperation without coercion
- **Verification Framework**: All agent claims are verified against reality
- **Trust Management**: Reputation-based trust scoring
- **Recovery System**: Comprehensive error recovery with multiple strategies

## Features

### 🤝 Swarm Coordination
- Manage multiple autonomous agents
- Dynamic task assignment based on capabilities
- Load balancing and resource management
- Consensus mechanisms for critical decisions

### 📜 Promise-Based Cooperation
- Agents make voluntary promises about behavior
- No command-and-control hierarchy
- Willingness evaluation before commitment
- Promise fulfillment tracking

### ✅ Real Command Execution
- Production-ready command execution with `tokio::process::Command`
- Security: allowlist/blocklist and working directory restrictions  
- Resource limits: timeouts, memory/CPU constraints
- Sandboxing: user/group restrictions when running as root

### ✅ Execution Verification
- Command execution verification
- File system state verification  
- Network operation verification
- Cryptographic proof generation

### 🔐 Trust Management
- Dynamic trust scores based on performance
- **Persistent storage** with SQLite, JSON files, or in-memory backends
- **ACID transactions** for data integrity
- **Schema migrations** for database evolution
- Trust-based task assignment
- Peer feedback integration
- Time decay for stale trust scores
- **Automated backups** and restore capabilities
- **Concurrent access** support for multi-agent environments

### 🔄 Recovery System
- **Multiple recovery strategies**: Exponential backoff, checkpoint recovery, graceful degradation, self-healing
- **Automatic error detection** and strategy selection
- **State reconstruction** from checkpoints
- **Resource monitoring** and adaptive degradation
- **Self-healing rules** with configurable patterns and actions
- **Recovery history** and performance metrics

### 🤖 Claude Integration
- Special wrapper for Claude sub-agents
- Context injection for sub-agents
- Trust boundary enforcement
- Tool restriction based on trust

## Implementation Status

- ✅ Core swarm coordinator
- ✅ Agent protocol (message passing)
- ✅ Trust management system with persistent storage
- ✅ Verification framework
- ✅ Real command execution engine
- ✅ Claude agent wrapper
- ✅ Byzantine Fault Tolerant consensus (PBFT)
- ✅ Fault tolerance with circuit breakers
- ✅ Recovery strategies and self-healing
- ✅ Monitoring & metrics (Prometheus)
- ✅ Full promise integration
- 📋 Distributed coordination across networks

## Architecture

```
┌─────────────────────────────────────┐
│         Swarm Coordinator           │
├─────────────────────────────────────┤
│  Intent  │  Promise  │ Verification │
│  System  │  Theory   │  Framework   │
├─────────────────────────────────────┤
│         Agent Protocol              │
├─────────────────────────────────────┤
│   Agent₁    Agent₂    Agent₃        │
└─────────────────────────────────────┘
```

## Usage

### Basic Swarm Setup

```rust
use synapsed_swarm::prelude::*;
use synapsed_intent::IntentBuilder;
use synapsed_promise::AutonomousAgent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create swarm coordinator
    let config = SwarmConfig {
        max_agents: 10,
        min_trust_score: 0.5,
        require_verification: true,
        ..Default::default()
    };
    let coordinator = SwarmCoordinator::new(config);
    coordinator.initialize().await?;
    
    // Add agents to swarm
    let agent1 = create_code_agent();
    let agent2 = create_review_agent();
    let agent3 = create_test_agent();
    
    coordinator.add_agent(agent1, AgentRole::Worker).await?;
    coordinator.add_agent(agent2, AgentRole::Worker).await?;
    coordinator.add_agent(agent3, AgentRole::Verifier).await?;
    
    // Create and delegate intent
    let intent = IntentBuilder::new("Implement feature")
        .add_step(/* ... */)
        .build()?;
    
    let task_id = coordinator.delegate_intent(intent, context).await?;
    
    // Wait for completion
    let result = coordinator.get_task_result(task_id).await;
    
    Ok(())
}
```

### Claude Agent Integration

```rust
use synapsed_swarm::claude_agent::{ClaudeAgent, ClaudeAgentConfig, ClaudeContext};

// Configure Claude agent
let config = ClaudeAgentConfig {
    name: "claude_coder".to_string(),
    role: AgentRole::Worker,
    capabilities: vec!["code_generation".to_string()],
    tools: vec!["read_file", "write_file", "execute_command"],
    require_verification: true,
    ..Default::default()
};

// Create Claude agent
let claude = ClaudeAgent::new(config);
claude.initialize().await?;

// Inject context for sub-agent
let context = ClaudeContext {
    swarm_id,
    task_id,
    intent: intent.clone(),
    verification_required: true,
    trust_boundaries: TrustBoundaries {
        filesystem_access: FilesystemAccess::Workspace,
        max_memory_mb: 512,
        can_delegate: false,
        ..Default::default()
    },
    ..Default::default()
};

claude.inject_context(context).await?;

// Accept and execute task
let promise = claude.accept_task(assignment).await?;
let result = claude.execute_task(task_id).await?;
```

### Trust Management with Persistent Storage

```rust
use synapsed_swarm::{
    trust::{TrustManager, TrustOperation, BackupConfig},
    persistence::{SqliteTrustStore, FileTrustStore, InMemoryTrustStore},
};
use std::sync::Arc;

// Option 1: SQLite storage (production recommended)
let store = Arc::new(SqliteTrustStore::new("trust.db", Some("./backups")).await?);

// Option 2: File-based JSON storage
// let store = Arc::new(FileTrustStore::new("./trust_data", Some("./backups"))?);

// Option 3: In-memory storage (testing only)
// let store = Arc::new(InMemoryTrustStore::new());

// Configure automatic backups
let backup_config = BackupConfig {
    enabled: true,
    interval_secs: 3600, // Backup every hour
    on_significant_change: true,
    significant_change_threshold: 0.1,
};

let trust_manager = TrustManager::with_storage(store)
    .with_backup_config(backup_config);
trust_manager.initialize().await?;

// Initialize agent trust
trust_manager.initialize_agent(agent_id, 0.5).await?;

// Update trust based on performance (automatically persisted)
trust_manager.update_trust(agent_id, success, verified).await?;

// Check trust for operation
let can_perform = trust_manager.check_threshold(
    agent_id,
    TrustOperation::CriticalTask
).await?;

// Get trusted agents for consensus
let trusted = trust_manager.get_trusted_agents(0.7).await?;

// Create manual backup
trust_manager.create_backup("backup_20241224.db").await?;

// Check storage health
let health = trust_manager.get_storage_health().await?;
println!("Trust storage health: {} agents tracked", health.total_agents);

// Cleanup old data (older than 30 days)
let cutoff = chrono::Utc::now() - chrono::Duration::days(30);
let cleaned = trust_manager.cleanup_old_data(cutoff).await?;
println!("Cleaned up {} old records", cleaned);
```

### Real Command Execution

```rust
use synapsed_swarm::{ExecutionEngine, ExecutionConfig};
use std::path::PathBuf;

// Configure execution engine
let mut config = ExecutionConfig::default();
config.allowed_commands = vec![
    "ls".to_string(), "cat".to_string(), "echo".to_string()
];
config.blocked_commands = vec![
    "rm".to_string(), "sudo".to_string()
];
config.max_execution_time_secs = 30;
config.allowed_working_dirs = vec![PathBuf::from("/tmp")];
config.enable_sandboxing = true;

let engine = ExecutionEngine::with_config(config);
engine.initialize().await?;

// Execute commands safely
let result = engine.execute_command("echo", &["Hello", "World"], None).await?;
println!("Output: {}", result.stdout);
println!("Success: {}", result.success);
println!("Duration: {}ms", result.duration_ms);

// Get execution history
let history = engine.execution_history().await;
println!("Executed {} commands", history.len());
```

### Verification

```rust
use synapsed_swarm::verification::{SwarmVerifier, VerificationPolicy};

let policy = VerificationPolicy {
    verify_commands: true,
    verify_filesystem: true,
    generate_proofs: true,
    min_confidence: 0.8,
    ..Default::default()
};

let verifier = SwarmVerifier::with_policy(policy);

// Verify task execution
let report = verifier.verify_execution(
    &intent,
    &result,
    agent_id
).await?;

if report.verified {
    println!("✅ Verification passed with {:.0}% confidence", 
             report.confidence * 100.0);
}
```

## Protocol

Agents communicate using a structured protocol:

```rust
// Send task request
let message = AgentProtocol::create_task_request(
    coordinator_id,
    agent_id,
    assignment
);

// Handle response
match message.message_type {
    MessageType::TaskAccept => {
        // Agent accepted task
    },
    MessageType::TaskReject => {
        // Find another agent
    },
    _ => {}
}
```

## Examples

See the `examples/` directory for:
- `swarm_demo.rs` - Basic swarm coordination demo
- `execution_demo.rs` - Real command execution demonstration
- `trust_persistence_demo.rs` - Trust storage system demonstration
- `recovery_demo.rs` - Recovery system demonstration
- `simple_recovery_integration.rs` - Simple recovery integration example

## Testing

```bash
cargo test -p synapsed-swarm
cargo run --example swarm_demo
cargo run --example execution_demo
cargo run --example trust_persistence_demo
cargo run --example recovery_demo
cargo run --example simple_recovery_integration
```

## Storage Backends

### SQLite (Production Recommended)
- ACID transactions for data integrity
- Schema versioning and migrations
- Automatic periodic backups
- Concurrent read/write access
- Query optimization with indexes

### File-based JSON Storage
- Human-readable format
- Simple deployment
- Atomic file operations
- No external dependencies

### In-Memory Storage
- Ultra-fast operations
- Perfect for testing
- No persistence (by design)
- Full feature compatibility

For detailed information, see `docs/TRUST_PERSISTENCE.md`.

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT license