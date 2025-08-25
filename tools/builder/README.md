# Synapsed Builder System

A powerful no-code module composition framework that allows Claude (and developers) to build applications by assembling pre-tested Synapsed modules without writing implementation code.

## Overview

The Synapsed Builder provides:
- **Component Registry**: Discover modules by capability
- **Recipe System**: Reusable application templates
- **Builder DSL**: Fluent API for composition
- **Smart Composer**: Automatic dependency resolution
- **Code Generation**: Generate Rust/TypeScript from compositions
- **Validation**: Ensure compositions are valid before building

## Quick Start

### Using the Builder in Rust

```rust
use synapsed_builder::prelude::*;

// Build an application
let app = SynapsedBuilder::new("my-app")
    .description("My distributed application")
    .add_intent_verification()
    .add_storage(StorageBackend::Postgres)
    .add_observability(ObservabilityLevel::Full)
    .add_consensus()
    .connect(
        "synapsed-intent", "intent_declared",
        "synapsed-consensus", "validate"
    )
    .build()?;

// Save to directory
app.save("./my-app").await?;
```

### Using Pre-built Templates

```rust
use synapsed_builder::templates::Templates;

// Use a template
let app = Templates::verified_ai_agent()
    .configure("synapsed-storage", json!({
        "path": "./agent.db"
    }))
    .env("RUST_LOG", "debug")
    .build()?;
```

### Using Recipes

```yaml
# my-app.yaml
name: my-distributed-app
description: Distributed application with consensus
version: 1.0.0
components:
  - name: synapsed-core
  - name: synapsed-consensus
  - name: synapsed-net
  - name: synapsed-storage
connections:
  - from: synapsed-net
    event: message_received
    to: synapsed-consensus
    handler: process_message
config:
  synapsed-consensus:
    committee_size: 5
    block_time_ms: 1000
```

```rust
use synapsed_builder::recipe::RecipeManager;

let mut manager = RecipeManager::new();
let recipe_name = manager.load_yaml(&yaml_content)?;
let recipe = manager.get(&recipe_name).unwrap();

let app = SynapsedBuilder::from_recipe(recipe)
    .build()?;
```

## Component Registry

### Finding Components by Capability

```rust
let registry = ComponentRegistry::new();
registry.initialize_default_components();

// Find storage components
let storage_components = registry.find_by_capability(Capability::Storage);
// Returns: ["synapsed-storage", "synapsed-crdt"]

// Find verification components  
let verify_components = registry.find_by_capability(Capability::Verification);
// Returns: ["synapsed-verify", "synapsed-intent"]
```

### Available Capabilities

- `Core` - Runtime and memory management
- `Intent` - Intent declaration and planning
- `Verification` - Proof generation and verification
- `Consensus` - Distributed consensus protocols
- `Networking` - P2P and network transport
- `Storage` - Data persistence
- `Observability` - Monitoring and metrics
- `Crypto` - Cryptographic operations
- `Payments` - Financial transactions
- `CRDT` - Conflict-free replicated data types

## Builder DSL

### High-Level Methods

```rust
SynapsedBuilder::new("app-name")
    // Add capabilities
    .add_intent_verification()      // Adds intent + verify modules
    .add_consensus()                 // Adds consensus + networking
    .add_observability(level)        // Adds monitoring at specified level
    .add_storage(backend)            // Adds storage with backend
    .add_network(network_type)       // Adds networking layer
    .add_payments()                  // Adds payment processing
    
    // Add specific components
    .add_component("synapsed-crdt")
    
    // Configure components
    .configure("component-name", json!({...}))
    
    // Set environment variables
    .env("RUST_LOG", "debug")
    
    // Define connections
    .connect("from", "event", "to", "handler")
    
    // Build the application
    .build()?
```

### Storage Backends

- `StorageBackend::Sqlite` - Embedded SQLite
- `StorageBackend::Postgres` - PostgreSQL
- `StorageBackend::Redis` - Redis key-value store

### Observability Levels

- `ObservabilityLevel::Basic` - Essential metrics only
- `ObservabilityLevel::Standard` - Metrics + tracing
- `ObservabilityLevel::Full` - Complete observability suite

### Network Types

- `NetworkType::P2P` - Peer-to-peer networking
- `NetworkType::ClientServer` - Traditional client-server
- `NetworkType::Hybrid` - Both P2P and client-server

## Templates

### Available Templates

1. **verified-ai-agent** - AI agent with intent verification
2. **distributed-consensus** - Distributed system with consensus
3. **secure-payment-system** - Payment processing with security
4. **observable-microservice** - Microservice with monitoring
5. **p2p-network** - P2P networking application
6. **event-driven-system** - Event-based architecture
7. **crdt-collaborative** - Collaborative app with CRDTs
8. **quantum-secure-app** - Post-quantum secure application
9. **ai-swarm-coordinator** - Multi-agent swarm system
10. **full-stack-verified** - Complete verified stack

### Using Templates

```rust
// List all templates
let templates = Templates::list();
for template in templates {
    println!("{}: {}", template.name, template.description);
}

// Use a specific template
let app = Templates::secure_payment_system()
    .configure("synapsed-payments", json!({
        "supported_currencies": ["USD", "EUR"],
        "risk_threshold": 80
    }))
    .build()?;
```

## Code Generation

### Generate Rust Code

```rust
let app = /* ... build your app ... */;

// Generate Cargo.toml
let cargo_toml = app.generate_cargo_toml();

// Generate main.rs
let main_rs = app.generate_main_rs();

// Save to directory
app.save("./output").await?;
```

### Generated Structure

```
output/
├── Cargo.toml           # Dependencies and metadata
├── src/
│   └── main.rs         # Application entry point
├── config.json         # Component configuration
└── .env               # Environment variables
```

## Validation

The builder automatically validates:
- Component dependencies are satisfied
- Connections reference valid components
- Required configurations are provided
- No circular dependencies exist
- Component compatibility

```rust
let validator = Validator::new();
let result = validator.validate(&app);

match result {
    Ok(_) => println!("✅ Valid composition"),
    Err(e) => println!("❌ Invalid: {}", e),
}
```

## Advanced Features

### Custom Component Registration

```rust
let mut registry = ComponentRegistry::new();

registry.register(Component {
    name: "my-custom-component".to_string(),
    version: "1.0.0".to_string(),
    capabilities: vec![Capability::Custom("my-capability")],
    description: "My custom component".to_string(),
    dependencies: vec!["synapsed-core".to_string()],
});
```

### Dependency Resolution

```rust
let composer = Composer::new();
let resolved = composer.resolve_dependencies(&app)?;
// Returns components in correct initialization order
```

### Recipe Management

```rust
let mut manager = RecipeManager::new();

// Load from YAML
manager.load_yaml(&yaml_content)?;

// Load from JSON
manager.load_json(&json_content)?;

// Save recipe
let recipe = Recipe {
    name: "my-recipe".to_string(),
    description: "My recipe".to_string(),
    version: "1.0.0".to_string(),
    components: vec![/* ... */],
    connections: vec![/* ... */],
    config: HashMap::new(),
};
manager.save_recipe(&recipe)?;
```

## Example: Building a Verified Payment System

```rust
use synapsed_builder::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Build the application
    let app = SynapsedBuilder::new("verified-payments")
        .description("Payment system with verification and consensus")
        
        // Add core capabilities
        .add_payments()
        .add_intent_verification()
        .add_consensus()
        .add_observability(ObservabilityLevel::Full)
        .add_storage(StorageBackend::Postgres)
        
        // Wire components together
        .connect(
            "synapsed-payments", "transaction_initiated",
            "synapsed-intent", "declare_intent"
        )
        .connect(
            "synapsed-intent", "intent_declared",
            "synapsed-verify", "verify_intent"
        )
        .connect(
            "synapsed-verify", "verification_complete",
            "synapsed-consensus", "propose_transaction"
        )
        .connect(
            "synapsed-consensus", "consensus_reached",
            "synapsed-payments", "finalize_transaction"
        )
        
        // Configure components
        .configure("synapsed-payments", json!({
            "supported_currencies": ["USD", "EUR", "BTC"],
            "max_transaction_amount": 100000,
            "risk_assessment": true
        }))
        .configure("synapsed-consensus", json!({
            "consensus_type": "hotstuff",
            "committee_size": 7,
            "block_time_ms": 500
        }))
        .configure("synapsed-storage", json!({
            "connection_string": "postgres://localhost/payments",
            "pool_size": 20
        }))
        
        // Set environment
        .env("RUST_LOG", "info")
        .env("PAYMENT_ENV", "production")
        
        // Build and validate
        .build()?;
    
    // Save to directory
    app.save("./verified-payments").await?;
    
    println!("✅ Payment system built successfully!");
    println!("📁 Output saved to ./verified-payments/");
    
    Ok(())
}
```

## Testing Your Compositions

The builder includes comprehensive testing:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_composition_valid() {
        let app = SynapsedBuilder::new("test-app")
            .add_intent_verification()
            .build()
            .unwrap();
        
        let validator = Validator::new();
        assert!(validator.validate(&app).is_ok());
    }
    
    #[tokio::test]
    async fn test_code_generation() {
        let app = /* build app */;
        
        let cargo_toml = app.generate_cargo_toml();
        assert!(cargo_toml.contains("[dependencies]"));
        assert!(cargo_toml.contains("synapsed-core"));
    }
}
```

## Best Practices

1. **Start with templates** for common patterns
2. **Use capability discovery** to find the right components
3. **Validate early** to catch configuration errors
4. **Test compositions** before deploying
5. **Save recipes** for reusable patterns
6. **Document connections** clearly
7. **Use environment variables** for configuration
8. **Version your recipes** for reproducibility

## Troubleshooting

### Common Issues

**Missing Dependencies**
```
Error: synapsed-consensus requires synapsed-net
```
Solution: The builder should auto-resolve dependencies, but you can manually add missing components.

**Invalid Connections**
```
Error: Connection from unknown component: my-component
```
Solution: Ensure all components in connections are added to the application.

**Configuration Errors**
```
Error: Required configuration missing for synapsed-storage
```
Solution: Check component documentation for required configuration fields.

## API Reference

See the [API Documentation](https://docs.rs/synapsed-builder) for complete details.

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines on contributing to the builder system.