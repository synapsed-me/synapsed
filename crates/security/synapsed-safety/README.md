# Synapsed Safety

Self-aware safety mechanisms, constraint engines, and automatic rollback systems for the Synapsed ecosystem.

## Overview

This crate provides comprehensive safety guarantees for distributed systems through formal constraint specification, real-time monitoring, and automatic recovery mechanisms. 

## Core Features

### Constraint Engines
- **Formal Specification**: Express safety requirements mathematically
- **Real-time Validation**: Continuous constraint checking
- **Automatic Enforcement**: Block unsafe operations before execution
- **Custom Predicates**: User-defined safety conditions

### Rollback Mechanisms
- **Instant Recovery**: Automatic state restoration on violations
- **Checkpoint Management**: Efficient state snapshot storage
- **Memory Compression**: Optimized history management
- **Selective Rollback**: Partial state restoration

### Self-Aware Systems
- **Dynamic Boundaries**: Adaptive safety threshold detection
- **Learning**: Pattern recognition for potential violations
- **Predictive Safety**: Prevent violations before they occur
- **Self-Healing**: Automatic adaptation to prevent future issues

## Quick Start

```rust
use synapsed_safety::{
    SafetyEngine, Constraint, Severity, 
    RollbackPoint, CheckpointId
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create safety engine
    let mut engine = SafetyEngine::new().await?;
    
    // Define safety constraints
    let balance_constraint = Constraint::new("positive_balance")
        .rule(|state: &AccountState| state.balance >= 0)
        .severity(Severity::Critical)
        .message("Account balance cannot be negative");
    
    let rate_limit_constraint = Constraint::new("api_rate_limit")
        .rule(|state: &ApiState| state.requests_per_minute <= 1000)
        .severity(Severity::Warning)
        .message("API rate limit exceeded");
    
    engine.add_constraint(balance_constraint).await?;
    engine.add_constraint(rate_limit_constraint).await?;
    
    // Create checkpoint before risky operation
    let checkpoint = engine.create_checkpoint().await?;
    
    // Execute operation with safety monitoring
    match engine.execute_safe(|| {
        // Potentially unsafe operation
        transfer_funds(account_id, 1000)
    }).await {
        Ok(result) => {
            println!("Operation succeeded: {:?}", result);
            engine.commit_checkpoint(&checkpoint).await?;
        },
        Err(violation) => {
            println!("Safety violation: {}", violation);
            engine.rollback_to_checkpoint(&checkpoint).await?;
        }
    }
    
    Ok(())
}
```

## Advanced Usage

### Custom Constraint Types

```rust
use synapsed_safety::{Constraint, ConstraintType};

// Temporal constraint
let temporal_constraint = Constraint::temporal("deadline")
    .rule(|state: &TaskState| state.deadline > Utc::now())
    .window(Duration::minutes(5))
    .severity(Severity::High);

// Resource constraint
let memory_constraint = Constraint::resource("memory_usage")
    .rule(|state: &SystemState| state.memory_usage < 0.8)
    .threshold(0.9)  // Warning threshold
    .action(ConstraintAction::FreeMemory);

// Dependency constraint
let dependency_constraint = Constraint::dependency("service_health")
    .rule(|state: &ServiceState| state.all_dependencies_healthy())
    .retry_count(3)
    .backoff(Duration::seconds(2));
```

### Formal Verification

```rust
use synapsed_safety::formal::{Theorem, Proof, Z3Solver};

// Define safety theorem
let safety_theorem = Theorem::new("account_safety")
    .precondition("balance >= 0")
    .operation("transfer(amount)")
    .postcondition("balance >= 0")
    .invariant("balance == old_balance - amount");

// Verify with Z3 solver
let solver = Z3Solver::new();
let proof = solver.verify(&safety_theorem).await?;

if proof.is_valid() {
    println!("Safety property formally verified!");
} else {
    println!("Counterexample found: {:?}", proof.counterexample());
}
```

### Self-Healing Systems

```rust
use synapsed_safety::healing::{HealingStrategy, AdaptationRule};

// Define healing strategies
let healing_strategy = HealingStrategy::new()
    .detect_pattern("memory_leak")
    .action(HealingAction::RestartComponent)
    .learning_enabled(true);

let adaptation_rule = AdaptationRule::new()
    .trigger("repeated_violations")
    .adaptation(|constraint| {
        constraint.tighten_threshold(0.9); // Make more restrictive
    });

engine.add_healing_strategy(healing_strategy).await?;
engine.add_adaptation_rule(adaptation_rule).await?;
```

## Architecture

### SafetyEngine
Core orchestrator for all safety mechanisms:
```rust
pub struct SafetyEngine {
    constraints: Vec<Constraint>,
    monitor: SafetyMonitor,
    rollback_manager: RollbackManager,
    healing_system: HealingSystem,
}
```

### Constraint System
```rust
#[async_trait]
pub trait ConstraintEngine {
    async fn validate(&self, state: &SafetyState) -> Result<ValidationResult>;
    async fn add_constraint(&mut self, constraint: Constraint) -> Result<()>;
    async fn remove_constraint(&mut self, id: &str) -> Result<()>;
}
```

### Rollback Management
```rust
#[async_trait]
pub trait RollbackManager {
    async fn create_checkpoint(&mut self) -> Result<CheckpointId>;
    async fn rollback_to(&mut self, checkpoint: &CheckpointId) -> Result<()>;
    async fn compress_history(&mut self, before: DateTime<Utc>) -> Result<()>;
}
```

## Safety Patterns

### Critical Section Protection
```rust
use synapsed_safety::patterns::CriticalSection;

let critical_section = CriticalSection::new("database_transaction")
    .max_duration(Duration::seconds(30))
    .timeout_action(TimeoutAction::Rollback)
    .isolation_level(IsolationLevel::Serializable);

critical_section.execute(|| async {
    // Database operations with automatic rollback on timeout
    database.begin_transaction().await?;
    database.update_account(account_id, new_balance).await?;
    database.commit_transaction().await?;
    Ok(())
}).await?;
```

### Resource Management
```rust
use synapsed_safety::patterns::ResourceGuard;

let resource_guard = ResourceGuard::new()
    .track_memory(true)
    .track_file_handles(true)
    .track_network_connections(true)
    .leak_detection(true);

resource_guard.execute(|| async {
    // Resource-intensive operation with automatic cleanup
    let data = load_large_dataset().await?;
    let result = process_data(data).await?;
    Ok(result)
}).await?; // Resources automatically cleaned up
```

### Circuit Breaker Pattern
```rust
use synapsed_safety::patterns::CircuitBreaker;

let circuit_breaker = CircuitBreaker::new("external_service")
    .failure_threshold(5)          // Open after 5 failures
    .timeout(Duration::seconds(10)) // Try again after 10 seconds
    .success_threshold(3);         // Close after 3 successes

match circuit_breaker.execute(|| external_service_call()).await {
    Ok(result) => println!("Success: {:?}", result),
    Err(CircuitBreakerError::Open) => println!("Circuit breaker is open"),
    Err(other) => println!("Other error: {:?}", other),
}
```

## Performance Characteristics

| Feature | Overhead | Memory Usage | Use Case |
|---------|----------|--------------|----------|
| Basic Constraints | < 1% | Minimal | Always-on safety |
| Checkpoints | 2-5% | O(state size) | Transactional operations |
| Formal Verification | High | High | Critical system verification |
| Self-Healing | 1-3% | Low | Adaptive systems |

## Configuration

### Engine Configuration
```rust
let config = SafetyConfig::new()
    .max_checkpoints(100)
    .compression_interval(Duration::hours(1))
    .constraint_check_interval(Duration::milliseconds(100))
    .healing_enabled(true)
    .formal_verification_enabled(false); // Resource intensive

let engine = SafetyEngine::with_config(config).await?;
```

### Memory Management
```rust
let memory_config = MemoryConfig::new()
    .checkpoint_compression(CompressionAlgorithm::Zstd)
    .max_memory_usage(100 * 1024 * 1024) // 100MB
    .garbage_collection_interval(Duration::minutes(5));
```

## Testing

```bash
# Unit tests
cargo test

# Safety property tests
cargo test --test safety_properties

# Rollback mechanism tests
cargo test --test rollback_tests

# Self-healing tests
cargo test --test healing_tests

# Formal verification tests (requires Z3)
cargo test --features formal-verification

# Performance benchmarks
cargo bench
```

## Features

- `default`: Basic constraints, rollback, and verification
- `constraints`: Constraint engine system
- `rollback`: State checkpoint and rollback mechanisms
- `verification`: Runtime verification systems
- `formal-verification`: Z3-based formal verification (requires Z3)
- `self-healing`: Adaptive self-healing mechanisms

## Dependencies

### Core Dependencies
- `petgraph`: Graph algorithms for dependency analysis
- `chrono`: Time handling for temporal constraints

### Optional Dependencies
- `z3`: SMT solver for formal verification (feature gated)

### Internal Dependencies
- `synapsed-core`: Shared utilities
- `synapsed-crypto`: Secure hashing for checksums

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT License

## Security Notice

This crate provides safety mechanisms but cannot guarantee complete system safety. Proper testing, auditing, and verification are still required for critical systems.

## Research References

1. Hoare, C.A.R. "An Axiomatic Basis for Computer Programming" (1969)
2. Lamport, L. "The Temporal Logic of Actions" (1994)
3. Clarke, E.M., et al. "Model Checking" (1999)
4. Meyer, B. "Applying Design by Contract" (1992)