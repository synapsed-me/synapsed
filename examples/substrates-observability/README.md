# Substrates Observability Example

This example demonstrates the Synapsed implementation of Humainary's Substrates observability framework, showing the correct event flow patterns and architectural principles.

## Overview

The Substrates pattern provides a sophisticated observability framework based on William Louth's vision at Humainary. The key insight is that emissions flow through channels and pipes, NOT directly from subjects.

## Correct Pattern

```
Subject → Channel → Pipe → Emission
```

**NOT**: ~~Subject → Emission~~ (This is incorrect!)

## Running the Example

```bash
cd examples/substrates-observability
cargo run
```

## Examples Included

### 1. Basic Emission Flow
Demonstrates the correct pattern where subjects create channels, channels create pipes, and emissions flow through pipes.

### 2. Subscription Model
Shows how managed sources handle subscriptions with multiple channels registered to a subscription.

### 3. Queue and Script Execution
Demonstrates priority-based queue processing with script execution.

### 4. Sink Patterns
Shows different sink types:
- **BasicSink**: Collects all emissions
- **FilteredSink**: Only collects matching emissions
- **BatchingSink**: Groups emissions into time/size-based batches

### 5. Percepts with Composers
Demonstrates how Composers create type-safe percepts (wrappers) around channels.

### 6. Complex Circuit
Shows a complete monitoring circuit with multiple channels and circuit-wide operations.

## Key Concepts

### Subjects
Observable entities that are monitored. They don't emit directly!
```rust
let subject = Subject::new("context", "name");
```

### Channels
Created from subjects, they manage the flow of typed emissions.
```rust
let channel: Arc<dyn Channel<T>> = Arc::new(BasicChannel::new(subject));
```

### Pipes
Created from channels, pipes are the actual emission points.
```rust
let pipe = channel.create_pipe("pipe_name");
pipe.emit(Emission::new(data, subject));
```

### Circuits
Computational networks that manage channels and conduits.
```rust
let circuit = Arc::new(BasicCircuit::new("circuit_name"));
circuit.add_channel(channel);
```

### Percepts
Type-safe wrappers around channels created by Composers.
```rust
let composer = ValueComposer::new(|ch| MyPercept { channel: ch });
let percept = composer.compose(channel);
```

## Architecture

```
┌──────────────┐
│   Circuit    │ (Computational Network)
└──────┬───────┘
       │ manages
       ▼
┌──────────────┐
│   Channels   │ (Typed Event Streams)
└──────┬───────┘
       │ create
       ▼
┌──────────────┐
│    Pipes     │ (Emission Points)
└──────┬───────┘
       │ emit
       ▼
┌──────────────┐
│  Emissions   │ (Actual Events)
└──────────────┘
```

## Benefits

1. **Correct Separation**: Subjects don't emit directly, maintaining proper abstraction
2. **Type Safety**: Strongly typed channels and emissions
3. **Composability**: Percepts and Composers for building higher-level abstractions
4. **Performance**: Efficient event routing through circuits
5. **Flexibility**: Multiple sink patterns for different use cases

## Related Examples

- `intent-verification`: Uses Substrates for intent observability
- `promise-cooperation`: Monitors agent cooperation through Substrates
- `mcp-server`: Exposes Substrates metrics via Model Context Protocol