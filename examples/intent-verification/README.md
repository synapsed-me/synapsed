# Intent Verification Example

This example demonstrates how to use the Synapsed framework's intent verification system to ensure AI agents execute their declared intentions correctly.

## Overview

The intent verification system addresses a critical problem in AI agent systems: ensuring that agents actually do what they claim to do. This example shows how to:

1. Create hierarchical intents with clear goals and steps
2. Add verification requirements to each step
3. Use multiple verification strategies (command, filesystem, API)
4. Integrate with the Substrates observability framework
5. Generate cryptographic proofs of execution

## Running the Example

```bash
cd examples/intent-verification
cargo run
```

## Examples Included

### 1. Simple Command Execution Intent
Shows how to create an intent that executes shell commands and verifies their output.

### 2. File Operation Intent
Demonstrates file creation and verification using the FileSystemVerifier.

### 3. Hierarchical Intent
Complex example with parent and sub-intents for a deployment workflow.

### 4. Observable Intent
Integration with Synapsed Substrates for full observability of intent execution.

## Key Concepts

### Intent Declaration
```rust
let intent = IntentBuilder::new("Process data")
    .description("Transform and validate data")
    .priority(Priority::High)
    .step("fetch", StepAction::Command { /* ... */ })
    .verified_step("validate", action, verification_requirement)
    .build();
```

### Verification Strategies
- **CommandVerifier**: Verifies command execution and output
- **FileSystemVerifier**: Verifies file existence and content
- **APIVerifier**: Verifies API responses match expectations
- **CompositeVerifier**: Combines multiple verification strategies

### Observability Integration
The example shows how to emit events through Substrates channels for monitoring:
```rust
let pipe = channel.create_pipe("intent_events");
pipe.emit(Emission::new("Intent started", subject));
```

## Architecture

```
┌─────────────────┐
│   Intent Tree   │
│   (Hierarchical)│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Execution Plan │
│  (Topological)  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Verification  │
│   (Multi-strategy)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Observability  │
│   (Substrates)  │
└─────────────────┘
```

## Benefits

1. **Trustless Verification**: Every agent claim is verified against external reality
2. **Hierarchical Planning**: Complex tasks broken down into verifiable steps
3. **Multiple Verification Methods**: Choose the right verification for each action
4. **Full Observability**: Complete visibility into agent behavior
5. **Cryptographic Proofs**: Generate proofs of correct execution

## Related Examples

- `substrates-observability`: Deep dive into the observability framework
- `promise-cooperation`: Agent cooperation using Promise Theory
- `mcp-server`: Model Context Protocol integration for Claude