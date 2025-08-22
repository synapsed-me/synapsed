# Promise Theory Cooperation Example

This example demonstrates how autonomous AI agents cooperate using Promise Theory principles, ensuring voluntary cooperation without coercion.

## Overview

Promise Theory, developed by Mark Burgess, provides a framework for autonomous agents to cooperate voluntarily. Key principles:

1. **Autonomy**: Agents cannot be coerced, only make voluntary promises
2. **Promises**: Agents declare what they will do (not what others must do)
3. **Impositions**: Requests from others that agents can accept or reject
4. **Trust**: Built through verified promise fulfillment

## Running the Example

```bash
cd examples/promise-cooperation
cargo run
```

## Examples Included

### 1. Basic Promise Exchange
Shows the fundamental promise lifecycle:
- Promise creation by promiser
- Acceptance by promisee
- Fulfillment or violation
- State tracking

### 2. Trust Model and Reputation
Demonstrates dynamic trust scoring:
- Initial trust assignments
- Trust updates based on behavior
- Trust-based decision making
- Reputation thresholds

### 3. Cooperation Protocol
Shows formal cooperation protocols:
- Protocol rule definition
- Agent capability declaration
- Protocol-compliant promises
- Verification requirements

### 4. Handling Impositions
Demonstrates how agents handle requests:
- Capability evaluation
- Acceptance/rejection logic
- Converting impositions to promises
- Respecting agent boundaries

### 5. Multi-Agent Scenario
Complex data pipeline with 4 agents:
- Promise chains
- Trust-based execution
- Observability integration
- Failure handling

## Key Concepts

### Autonomous Agents
```rust
let agent = AutonomousAgent::new("agent_id", capabilities);
```
Agents are independent entities with:
- Unique identity
- Declared capabilities
- Voluntary participation

### Promises
```rust
let promise = Promise::new(body, promiser, promisee);
let accepted = promise.accept();
let fulfilled = accepted.fulfill();
```
Three-stage lifecycle:
1. **Proposed**: Initial promise
2. **Accepted**: Promisee agrees
3. **Fulfilled/Violated**: Final state

### Trust Model
```rust
let mut trust = TrustModel::new();
trust.add_agent("agent", 0.7);
trust.update_trust("agent", fulfilled);
```
Dynamic trust scoring based on:
- Promise fulfillment history
- Time decay
- Violation penalties

### Impositions
```rust
let imposition = Imposition::new(request, requester, target);
```
Requests that agents can:
- Evaluate against capabilities
- Accept (convert to promise)
- Reject (beyond capabilities)

## Architecture

```
┌────────────────┐     Imposition    ┌────────────────┐
│   Agent A      │ ◄──────────────── │   Agent B      │
│                │                    │                │
│  Capabilities  │     Evaluate       │  Capabilities  │
│  - Can do X    │ ──────────────►   │  - Can do Y    │
│  - Max size N  │                    │  - Max time T  │
│                │     Promise        │                │
│                │ ──────────────►   │                │
│                │                    │                │
│                │     Accept         │                │
│                │ ◄──────────────── │                │
│                │                    │                │
│                │     Fulfill        │                │
│                │ ──────────────►   │                │
└────────────────┘                    └────────────────┘
        │                                      │
        └──────────────┬───────────────────────┘
                       ▼
                ┌─────────────┐
                │ Trust Model │
                │  Updates    │
                └─────────────┘
```

## Benefits

1. **No Coercion**: Agents maintain autonomy
2. **Clear Contracts**: Explicit promise definitions
3. **Trust Building**: Reputation-based cooperation
4. **Failure Tolerance**: Graceful handling of violations
5. **Scalability**: Decentralized cooperation model

## Related Examples

- `intent-verification`: Verifies promise fulfillment
- `substrates-observability`: Monitors promise events
- `mcp-server`: Exposes promise metrics