# Synapsed Modular SDK Architecture

## Design Principles

### 1. Single Responsibility
Each SDK focuses on one capability domain:
- Intent SDK → Intent verification only
- Builder SDK → Composition only  
- Consensus SDK → Consensus only

### 2. Loose Coupling
SDKs communicate through well-defined interfaces:
- Standard MCP protocol for Claude Code
- Event bus for inter-SDK communication
- Shared configuration schema

### 3. Progressive Enhancement
Start with minimal SDKs, add more as needed:
```
Basic → intent-sdk
Advanced → intent-sdk + builder-sdk + observability-sdk
Full → All SDKs
```

## SDK Structure

```
tools/
├── sdk-core/                    # Shared SDK utilities
│   ├── lib/
│   │   ├── mcp-base.js         # Base MCP server class
│   │   ├── message-bus.js      # Inter-SDK communication
│   │   ├── config-manager.js   # Configuration handling
│   │   └── health-check.js     # Health monitoring
│   └── package.json
│
├── intent-sdk/                  # Intent verification SDK
│   ├── package.json
│   ├── lib/
│   │   ├── mcp-server.js      # MCP implementation
│   │   ├── tools.js            # Intent tools
│   │   └── storage.js          # Intent storage
│   └── bin/
│       └── init.js             # CLI initializer
│
├── builder-sdk/                 # No-code builder SDK
│   ├── package.json
│   ├── lib/
│   │   ├── mcp-server.js
│   │   ├── tools.js            # Builder tools
│   │   ├── registry.js         # Component registry
│   │   └── composer.js         # Composition engine
│   └── bin/
│       └── init.js
│
├── observability-sdk/           # Monitoring SDK
│   ├── package.json
│   ├── lib/
│   │   ├── mcp-server.js
│   │   ├── tools.js            # Observability tools
│   │   ├── circuits.js         # Event circuits
│   │   └── metrics.js          # Metrics collection
│   └── bin/
│       └── init.js
│
└── sdk-generator/               # SDK creation tool
    ├── templates/
    └── bin/
        └── create.js
```

## Package Naming Convention

All SDKs follow the pattern: `@synapsed/{capability}-sdk`

```json
{
  "@synapsed/intent-sdk": "Intent verification and planning",
  "@synapsed/builder-sdk": "No-code application composition",
  "@synapsed/observability-sdk": "Monitoring and metrics",
  "@synapsed/consensus-sdk": "Distributed consensus",
  "@synapsed/network-sdk": "P2P networking",
  "@synapsed/crypto-sdk": "Cryptographic operations",
  "@synapsed/storage-sdk": "Distributed storage",
  "@synapsed/swarm-sdk": "Agent coordination",
  "@synapsed/payment-sdk": "Payment processing",
  "@synapsed/verification-sdk": "Proof verification"
}
```

## SDK Communication Protocol

### 1. Event Bus Pattern
```javascript
// SDK A publishes
eventBus.publish('intent.declared', {
  id: 'intent-123',
  timestamp: Date.now(),
  data: {...}
});

// SDK B subscribes
eventBus.subscribe('intent.declared', (event) => {
  // React to intent
});
```

### 2. Direct RPC
```javascript
// SDK A calls SDK B
const result = await rpc.call('builder-sdk', 'findRecipe', {
  capabilities: ['payment', 'consensus']
});
```

### 3. Shared State
```javascript
// Write to shared state
await sharedState.set('current.intent', intentData);

// Read from shared state
const intent = await sharedState.get('current.intent');
```

## Installation Flow

### Individual SDK Installation
```bash
npx @synapsed/intent-sdk init
# 1. Downloads SDK
# 2. Creates ~/.synapsed/intent/
# 3. Generates config.json
# 4. Registers with Claude Code
# 5. Starts MCP server
# 6. Shows connection info
```

### Bundle Installation
```javascript
// bundles.json
{
  "verified-ai": [
    "@synapsed/intent-sdk",
    "@synapsed/verification-sdk",
    "@synapsed/observability-sdk"
  ],
  "distributed": [
    "@synapsed/consensus-sdk",
    "@synapsed/network-sdk",
    "@synapsed/storage-sdk"
  ],
  "full": ["*"]
}
```

## SDK Registry

Central registry for SDK discovery:

```javascript
// ~/.synapsed/registry.json
{
  "installed": [
    {
      "name": "@synapsed/intent-sdk",
      "version": "1.0.0",
      "port": 3001,
      "status": "running",
      "tools": ["intent_declare", "intent_verify", "intent_status"],
      "capabilities": ["intent", "verification", "planning"]
    },
    {
      "name": "@synapsed/builder-sdk",
      "version": "1.0.0",
      "port": 3002,
      "status": "running",
      "tools": ["compose_app", "find_recipe", "validate_composition"],
      "capabilities": ["composition", "codegen", "validation"]
    }
  ],
  "available": [
    {
      "name": "@synapsed/consensus-sdk",
      "description": "Add distributed consensus",
      "size": "2.3MB"
    }
  ]
}
```

## Claude Code Integration Points

### 1. Tool Registration
Each SDK registers its tools with Claude Code:

```javascript
// intent-sdk/lib/tools.js
export const tools = [
  {
    name: 'intent_declare',
    description: 'Declare an intent',
    inputSchema: {...}
  },
  {
    name: 'intent_verify',
    description: 'Verify intent execution',
    inputSchema: {...}
  }
];
```

### 2. Cross-SDK Tool Calls
Claude can orchestrate multiple SDKs:

```typescript
// In Claude's workflow
const intent = await call('intent_declare', {goal: "..."});
const recipe = await call('find_recipe', {intent: intent.id});
const app = await call('compose_app', {recipe: recipe});
const proof = await call('verify_execution', {app: app});
```

### 3. SDK Coordination
SDKs can coordinate without Claude's involvement:

```javascript
// Auto-coordination example
intentSDK.on('intent.declared', async (intent) => {
  // Builder SDK automatically finds matching recipe
  const recipe = await builderSDK.findRecipe(intent);
  
  // Verification SDK prepares monitors
  await verificationSDK.prepareMonitors(intent);
  
  // Observability SDK creates circuit
  await observabilitySDK.createCircuit(intent);
});
```

## Benefits for Claude Code Users

1. **Selective Installation**: Only install SDKs for needed capabilities
2. **Reduced Overhead**: Each SDK is lightweight and focused
3. **Independent Updates**: Update only the SDKs you use
4. **Clear Boundaries**: Each SDK has well-defined responsibilities
5. **Better Debugging**: Issues isolated to specific SDKs
6. **Flexible Workflows**: Combine SDKs in different ways

## Implementation Priority

Phase 1 (Core):
1. `@synapsed/intent-sdk` - Most requested feature
2. `@synapsed/builder-sdk` - Enables no-code composition
3. `@synapsed/observability-sdk` - Critical for monitoring

Phase 2 (Extended):
4. `@synapsed/verification-sdk` - Enhanced verification
5. `@synapsed/consensus-sdk` - Distributed systems
6. `@synapsed/network-sdk` - P2P capabilities

Phase 3 (Specialized):
7. `@synapsed/crypto-sdk` - Cryptographic operations
8. `@synapsed/storage-sdk` - Distributed storage
9. `@synapsed/swarm-sdk` - Agent coordination
10. `@synapsed/payment-sdk` - Financial operations