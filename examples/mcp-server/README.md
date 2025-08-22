# MCP Server Example

This example demonstrates the Synapsed MCP (Model Context Protocol) server that provides intent verification tools to Claude and other AI agents.

## Overview

The MCP server addresses the critical problem of AI agents losing context and making unverifiable claims. It provides tools for:

1. **Intent Declaration**: Agents declare intentions before acting
2. **Verification**: Verify that actions match declarations
3. **Promise Management**: Track and fulfill promises between agents
4. **Context Injection**: Pass context to sub-agents safely
5. **Trust Building**: Build reputation through verified actions

## Running the Example

```bash
cd examples/mcp-server
cargo run
```

The server will start on `http://localhost:3000` by default.

## Available MCP Tools

### Intent Management
- `intent_declare`: Declare an intent before executing actions
- `intent_status`: Check the current status of an intent
- `intent_complete`: Mark an intent as successfully completed

### Verification
- `verify_command`: Verify command execution and output
- `verify_file`: Verify file creation/modification
- `verify_api`: Verify API call responses

### Promise Theory
- `promise_make`: Make a voluntary promise to another agent
- `promise_accept`: Accept a promise from another agent
- `promise_fulfill`: Mark a promise as fulfilled
- `trust_check`: Query trust score for an agent

### Context Management
- `context_inject`: Inject context for sub-agents
- `context_get`: Retrieve current context
- `context_validate`: Validate context boundaries

## Usage with Claude

### Configuration
Add to Claude's MCP settings:
```json
{
  "mcpServers": {
    "synapsed": {
      "command": "cargo",
      "args": ["run", "--bin", "mcp-server-example"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

### Example Workflow

1. **Declare Intent**
```javascript
await use_mcp_tool("synapsed", "intent_declare", {
  goal: "Process and analyze user data",
  steps: ["fetch", "transform", "analyze"],
  verification_required: true
});
```

2. **Execute Actions**
```javascript
// Perform the actual work
const result = await processData();
```

3. **Verify Execution**
```javascript
await use_mcp_tool("synapsed", "verify_command", {
  command: "python analyze.py",
  output: result.output,
  intent_id: "intent_123"
});
```

4. **Complete Intent**
```javascript
await use_mcp_tool("synapsed", "intent_complete", {
  intent_id: "intent_123",
  results: result
});
```

## Architecture

```
┌─────────────┐     MCP Protocol    ┌──────────────┐
│   Claude    │ ◄─────────────────► │  MCP Server  │
│   (Client)  │                      │  (Synapsed)  │
└─────────────┘                      └──────┬───────┘
      │                                      │
      │ Uses Tools                           │ Manages
      ▼                                      ▼
┌─────────────┐                      ┌──────────────┐
│   Intent    │                      │   Intents    │
│   Declare   │                      │  Promises    │
│   Verify    │                      │   Context    │
│   Promise   │                      │    Trust     │
└─────────────┘                      └──────────────┘
```

## Benefits

1. **Verifiable Actions**: Every agent action can be verified
2. **Context Preservation**: Sub-agents maintain parent context
3. **Trust Building**: Reputation system based on promise fulfillment
4. **Failure Detection**: Catch when agents deviate from intentions
5. **Audit Trail**: Complete log of intentions and verifications

## Configuration Options

```rust
ServerConfig {
    name: String,              // Server identifier
    host: String,              // Bind address
    port: u16,                 // Port number
    enable_stdio: bool,        // Use stdio transport
    enable_verification: bool, // Enable verification tools
    enable_promises: bool,     // Enable promise tools
    enable_context_injection: bool, // Enable context tools
    max_context_size: usize,   // Max context size in bytes
    trust_threshold: f64,      // Minimum trust for operations
}
```

## Security Considerations

- Context boundaries are enforced
- Sub-agents cannot exceed parent permissions
- All promises are cryptographically signed
- Verification proofs are tamper-proof
- Trust scores decay over time without activity

## Related Examples

- `intent-verification`: Core verification functionality
- `promise-cooperation`: Promise Theory implementation
- `substrates-observability`: Monitoring MCP operations