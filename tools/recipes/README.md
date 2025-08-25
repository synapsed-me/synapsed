# Synapsed Recipe System

Pre-built, tested application compositions that can be instantly deployed. Recipes are declarative YAML/JSON files that define complete application architectures.

## What are Recipes?

Recipes are reusable blueprints for Synapsed applications that specify:
- Required components
- Component connections
- Configuration
- Environment settings
- Deployment instructions

## Available Recipes

### Core Recipes

#### verified-ai-agent.yaml
AI agent with intent verification and monitoring.
```yaml
name: verified-ai-agent
components:
  - synapsed-intent
  - synapsed-verify
  - synapsed-substrates
use_case: AI agents that need verification
```

#### distributed-consensus.yaml
Distributed system with Byzantine fault tolerance.
```yaml
name: distributed-consensus  
components:
  - synapsed-consensus
  - synapsed-net
  - synapsed-crdt
use_case: Distributed agreement systems
```

#### payment-system.yaml
Secure payment processing with consensus.
```yaml
name: payment-system
components:
  - synapsed-payments
  - synapsed-consensus
  - synapsed-crypto
use_case: Financial transaction processing
```

#### observable-microservice.yaml
Microservice with complete observability.
```yaml
name: observable-microservice
components:
  - synapsed-substrates
  - synapsed-monitor
  - synapsed-storage
use_case: Production microservices
```

#### p2p-network.yaml
Peer-to-peer networking application.
```yaml
name: p2p-network
components:
  - synapsed-net
  - synapsed-routing
  - synapsed-crypto
use_case: Decentralized applications
```

## Using Recipes

### From Command Line

```bash
# Using the builder SDK
npx @synapsed/builder-sdk use-recipe verified-ai-agent --name my-agent

# With configuration
npx @synapsed/builder-sdk use-recipe payment-system \
  --name my-payments \
  --config '{"synapsed-payments": {"currencies": ["USD", "EUR"]}}'
```

### From Rust Code

```rust
use synapsed_builder::recipe::RecipeManager;

// Load and use a recipe
let mut manager = RecipeManager::new();
manager.load_from_file("recipes/verified-ai-agent.yaml")?;

let recipe = manager.get("verified-ai-agent").unwrap();
let app = SynapsedBuilder::from_recipe(recipe.clone())
    .configure("synapsed-storage", json!({
        "path": "./data.db"
    }))
    .build()?;
```

### From Claude Code

```javascript
// Using builder SDK tools
await call('use_template', {
  template: 'verified-ai-agent',
  name: 'my-agent',
  config: {
    'synapsed-storage': {
      path: './agent.db'
    }
  }
});
```

## Recipe Format

### YAML Format

```yaml
# Recipe metadata
name: recipe-name
description: Recipe description
version: 1.0.0
author: Your Name
tags: [tag1, tag2]

# Required components
components:
  - name: synapsed-core
    version: "*"  # or specific version
  - name: synapsed-intent
    version: ">=0.1.0"

# Component connections
connections:
  - from: synapsed-intent
    event: intent_declared
    to: synapsed-verify
    handler: verify_intent
  - from: synapsed-verify
    event: verification_complete
    to: synapsed-monitor
    handler: log_verification

# Default configuration
config:
  synapsed-intent:
    max_depth: 5
    timeout_ms: 30000
  synapsed-storage:
    backend: sqlite
    path: ./data.db

# Environment variables
env:
  RUST_LOG: info
  NODE_ENV: production

# Deployment hints
deployment:
  min_memory: 512MB
  recommended_memory: 2GB
  ports:
    - 8080  # HTTP API
    - 9090  # Metrics
  volumes:
    - ./data:/app/data
```

### JSON Format

```json
{
  "name": "recipe-name",
  "description": "Recipe description",
  "version": "1.0.0",
  "components": [
    {
      "name": "synapsed-core",
      "version": "*"
    },
    {
      "name": "synapsed-intent",
      "version": ">=0.1.0"
    }
  ],
  "connections": [
    {
      "from": "synapsed-intent",
      "event": "intent_declared",
      "to": "synapsed-verify",
      "handler": "verify_intent"
    }
  ],
  "config": {
    "synapsed-intent": {
      "max_depth": 5
    }
  }
}
```

## Creating Custom Recipes

### Step 1: Define Requirements

Identify what your application needs:
- Core functionality (payments, consensus, storage)
- Non-functional requirements (observability, security)
- Integration points (APIs, databases)

### Step 2: Select Components

Use the component registry to find modules:
```rust
let registry = ComponentRegistry::new();
let storage_components = registry.find_by_capability(Capability::Storage);
```

### Step 3: Design Connections

Map out how components communicate:
```yaml
connections:
  # Input flow
  - from: api-gateway
    event: request_received
    to: request-handler
    handler: process_request
  
  # Processing flow
  - from: request-handler
    event: data_needed
    to: database
    handler: fetch_data
  
  # Output flow
  - from: request-handler
    event: response_ready
    to: api-gateway
    handler: send_response
```

### Step 4: Configure Components

Provide sensible defaults:
```yaml
config:
  database:
    pool_size: 10
    timeout: 30
  api-gateway:
    port: 8080
    rate_limit: 100
```

### Step 5: Test Recipe

```rust
#[test]
fn test_recipe_valid() {
    let manager = RecipeManager::new();
    let result = manager.load_yaml(&recipe_yaml);
    assert!(result.is_ok());
    
    let recipe = manager.get("my-recipe").unwrap();
    let app = SynapsedBuilder::from_recipe(recipe)
        .build();
    assert!(app.is_ok());
}
```

## Recipe Categories

### By Use Case

**AI/ML Applications**
- verified-ai-agent
- ml-pipeline
- llm-orchestrator

**Distributed Systems**
- distributed-consensus
- distributed-storage
- distributed-compute

**Financial Systems**
- payment-system
- trading-platform
- settlement-engine

**Infrastructure**
- observable-microservice
- api-gateway
- event-processor

**Networking**
- p2p-network
- mesh-network
- overlay-network

### By Complexity

**Simple** (1-3 components)
- basic-storage
- simple-api
- event-logger

**Intermediate** (4-6 components)
- verified-ai-agent
- observable-microservice
- payment-processor

**Advanced** (7+ components)
- full-stack-verified
- distributed-consensus-system
- multi-region-deployment

## Recipe Composition

Recipes can be composed together:

```rust
// Combine multiple recipes
let ai_recipe = manager.get("verified-ai-agent")?;
let payment_recipe = manager.get("payment-system")?;

let combined = RecipeComposer::new()
    .add_recipe(ai_recipe)
    .add_recipe(payment_recipe)
    .resolve_conflicts(ConflictStrategy::Merge)
    .build()?;
```

## Validating Recipes

### Schema Validation

```rust
use synapsed_builder::recipe::RecipeValidator;

let validator = RecipeValidator::new();
let validation_result = validator.validate_yaml(&yaml_content)?;

if !validation_result.is_valid() {
    for error in validation_result.errors {
        println!("Error: {}", error);
    }
}
```

### Dependency Validation

```rust
// Check all component dependencies are satisfied
let validator = Validator::new();
let app = SynapsedBuilder::from_recipe(recipe).build()?;
validator.validate(&app)?;
```

## Recipe Management

### Organizing Recipes

```
recipes/
├── core/              # Essential recipes
│   ├── verified-ai-agent.yaml
│   └── distributed-consensus.yaml
├── domain/            # Domain-specific
│   ├── payments/
│   ├── gaming/
│   └── iot/
├── examples/          # Example compositions
└── custom/           # User recipes
```

### Versioning Recipes

```yaml
name: my-recipe
version: 2.0.0  # Semantic versioning
changelog:
  - version: 2.0.0
    changes:
      - Added synapsed-crdt component
      - Updated consensus configuration
  - version: 1.0.0
    changes:
      - Initial release
```

### Sharing Recipes

```bash
# Export recipe with dependencies
synapsed-builder export-recipe my-recipe --output my-recipe-bundle.tar.gz

# Import recipe bundle
synapsed-builder import-recipe my-recipe-bundle.tar.gz

# Publish to registry (future)
synapsed-builder publish my-recipe --registry https://recipes.synapsed.ai
```

## Recipe Testing

### Unit Tests

```rust
#[test]
fn test_recipe_components() {
    let recipe = load_recipe("verified-ai-agent.yaml");
    assert!(recipe.components.contains("synapsed-intent"));
    assert!(recipe.components.contains("synapsed-verify"));
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_recipe_deployment() {
    let recipe = load_recipe("payment-system.yaml");
    let app = SynapsedBuilder::from_recipe(recipe).build()?;
    
    // Test code generation
    let code = app.generate_main_rs();
    assert!(code.contains("synapsed_payments"));
    
    // Test deployment
    app.save("./test-output").await?;
    assert!(Path::new("./test-output/Cargo.toml").exists());
}
```

## Best Practices

1. **Keep recipes focused** - One primary use case per recipe
2. **Provide sensible defaults** - Users shouldn't need to configure everything
3. **Document connections** - Explain why components are connected
4. **Version recipes** - Use semantic versioning
5. **Test thoroughly** - Validate recipes work as expected
6. **Use tags** - Help users discover relevant recipes
7. **Include examples** - Show how to customize the recipe
8. **Consider resources** - Document memory/CPU requirements

## Troubleshooting

### Recipe Won't Load
- Check YAML/JSON syntax
- Validate against schema
- Ensure all components exist

### Missing Dependencies
- Recipe should declare all dependencies
- Use dependency resolution in builder

### Configuration Conflicts
- Later configurations override earlier ones
- Use explicit merge strategies

### Performance Issues
- Check resource requirements
- Optimize component connections
- Review configuration values

## Contributing Recipes

We welcome recipe contributions! Please:

1. Test your recipe thoroughly
2. Document use cases clearly
3. Provide configuration examples
4. Include deployment guidance
5. Submit via pull request

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for details.