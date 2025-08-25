# RMCP Integration Guide for Synapsed MCP

## Overview

This document describes the integration between Synapsed's MCP implementation and the official Rust MCP SDK (rmcp).

## Current Status

Synapsed uses **rmcp v0.3** for the `#[tool]` procedural macro while maintaining its own specialized protocol handling for intent verification and Promise Theory features.

## Architecture

### 1. Hybrid Approach
- **rmcp**: Used for tool definition macros and future protocol compliance
- **Synapsed Custom**: Intent verification, Promise Theory, context boundary enforcement

### 2. Key Components

#### rmcp_adapter.rs
- Bridges rmcp's `Handler` trait with Synapsed's verification system
- Exposes tools: `intent_declare`, `intent_verify`, `intent_status`, `context_inject`
- Provides resources: active intents, verification results, trust levels

#### main_rmcp.rs (Alternative Entry Point)
- Uses rmcp's `StdioTransport` for protocol handling
- Maintains Synapsed's domain-specific logic
- Compatible with standard MCP clients

### 3. Integration Points

```rust
// Using rmcp's Handler trait
impl Handler for SynapsedMcpAdapter {
    async fn initialize(&self, options: InitializeOptions) -> InitializedNotification {
        // Synapsed-specific initialization
    }
    
    async fn call_tool(&self, request: CallToolRequest) -> CallToolResponse {
        // Routes to Synapsed's verification logic
    }
}
```

## Migration Path

### Phase 1: Enhanced Integration (Current)
✅ Created adapter layer (`rmcp_adapter.rs`)
✅ Maintained domain-specific features
✅ Added rmcp transport option (`main_rmcp.rs`)

### Phase 2: Full Protocol Adoption (Future)
- Upgrade to rmcp 0.6+ when API stabilizes
- Replace custom JSON-RPC with rmcp's protocol handler
- Maintain backward compatibility with existing clients

## Usage

### Running with rmcp Transport
```bash
cargo run --bin synapsed-mcp-server --features rmcp-transport
```

### Running with Custom Transport (Default)
```bash
cargo run --bin synapsed-mcp-server
```

## Key Differences from Standard MCP

1. **Intent Verification**: Every action must be declared before execution
2. **Promise Theory**: Agents make voluntary promises about behavior
3. **Context Boundaries**: Enforced isolation between agent contexts
4. **Verification-First**: All claims must be externally verifiable

## Benefits of This Approach

- **Protocol Compliance**: Can work with standard MCP clients
- **Domain Expertise**: Maintains Synapsed's security features
- **Flexibility**: Can switch between transports as needed
- **Future-Proof**: Ready for rmcp upgrades without losing functionality

## Known Limitations

- rmcp 0.6+ has breaking API changes - staying on 0.3 for stability
- Some advanced rmcp features not yet utilized
- Custom protocol extensions not exposed through rmcp interface

## Testing

Test the rmcp integration:
```bash
# Test with MCP client
npx @modelcontextprotocol/client test synapsed-mcp

# Test with Claude Code
# Add to Claude Code settings:
{
  "mcpServers": {
    "synapsed": {
      "command": "cargo",
      "args": ["run", "-p", "synapsed-mcp", "--bin", "synapsed-mcp-server"]
    }
  }
}
```

## Future Enhancements

1. **Full rmcp 0.6+ Migration**: When API stabilizes
2. **Tool Composition**: Use rmcp's tool composition features
3. **Resource Streaming**: Implement streaming for large verification results
4. **Authentication**: Add OAuth support via rmcp's auth features