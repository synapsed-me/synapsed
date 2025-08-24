# Synapsed Swarm

Unified swarm coordination system integrating Intent, Promise Theory, and Verification for reliable multi-agent collaboration.

## Overview

This crate provides a complete framework for coordinating multiple AI agents (including Claude sub-agents) using:
- **Intent System**: Hierarchical task planning and delegation
- **Promise Theory**: Voluntary cooperation without coercion
- **Verification Framework**: All agent claims are verified against reality
- **Trust Management**: Reputation-based trust scoring

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

### ✅ Execution Verification
- Command execution verification
- File system state verification  
- Network operation verification
- Cryptographic proof generation

### 🔐 Trust Management
- Dynamic trust scores based on performance
- Trust-based task assignment
- Peer feedback integration
- Time decay for stale trust scores

### 🤖 Claude Integration
- Special wrapper for Claude sub-agents
- Context injection for sub-agents
- Trust boundary enforcement
- Tool restriction based on trust

## Implementation Status

- ✅ Core swarm coordinator
- ✅ Agent protocol (message passing)
- ✅ Trust management system
- ✅ Verification framework
- ✅ Claude agent wrapper
- 🚧 Full promise integration
- 🚧 Consensus mechanisms
- 📋 Distributed coordination
- 📋 Fault tolerance

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

### Trust Management

```rust
use synapsed_swarm::trust::{TrustManager, TrustOperation};

let trust_manager = TrustManager::new();

// Initialize agent trust
trust_manager.initialize_agent(agent_id, 0.5).await?;

// Update trust based on performance
trust_manager.update_trust(agent_id, success, verified).await?;

// Check trust for operation
let can_perform = trust_manager.check_threshold(
    agent_id,
    TrustOperation::CriticalTask
).await?;

// Get trusted agents for consensus
let trusted = trust_manager.get_trusted_agents(0.7).await;
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
- More examples coming soon

## Testing

```bash
cargo test -p synapsed-swarm
cargo run --example swarm_demo
```

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT license