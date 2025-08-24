# Recovery System for Synapsed Swarm

## Overview

The recovery system provides comprehensive error recovery capabilities for the synapsed-swarm coordination system. It implements multiple recovery strategies with exponential backoff retry logic, state reconstruction from checkpoints, graceful degradation when resources are limited, and self-healing mechanisms.

## Architecture

```text
┌─────────────────────────────────────┐
│         Recovery Manager            │
├─────────────────────────────────────┤
│ ┌─────────────┐ ┌─────────────────┐ │
│ │  Strategy   │ │   Resource      │ │
│ │  Selection  │ │   Monitor       │ │
│ └─────────────┘ └─────────────────┘ │
├─────────────────────────────────────┤
│          Recovery Strategies        │
│ ┌──────────────┬─────────────────┐  │
│ │ Exponential  │ Checkpoint      │  │
│ │ Backoff      │ Recovery        │  │
│ ├──────────────┼─────────────────┤  │
│ │ Graceful     │ Self-Healing    │  │
│ │ Degradation  │ Rules           │  │
│ └──────────────┴─────────────────┘  │
├─────────────────────────────────────┤
│        Coordinator Integration      │
└─────────────────────────────────────┘
```

## Core Components

### RecoveryStrategy Trait

All recovery strategies implement the `RecoveryStrategy` trait:

```rust
#[async_trait]
pub trait RecoveryStrategy: Send + Sync {
    fn strategy_id(&self) -> &str;
    fn description(&self) -> &str;
    async fn can_handle(&self, error: &SwarmError) -> bool;
    async fn recover(&self, context: &RecoveryContext, error: &SwarmError) -> RecoveryResult;
    async fn cost_estimate(&self) -> f64;
    async fn requires_external_resources(&self) -> bool;
}
```

### Recovery Manager

The `RecoveryManager` coordinates multiple recovery strategies:

- **Strategy Selection**: Chooses the most appropriate strategy based on error type and cost
- **Resource Monitoring**: Tracks system resource usage
- **Recovery History**: Maintains a log of recovery attempts
- **Concurrent Recovery**: Limits simultaneous recovery operations

## Recovery Strategies

### 1. Exponential Backoff Strategy

**Purpose**: Retry failed operations with increasing delays to avoid overwhelming the system.

**Use Cases**:
- Network communication failures
- Temporary resource unavailability
- Transaction conflicts

**Configuration**:
```rust
let config = BackoffConfig {
    initial_delay: Duration::from_millis(100),
    max_delay: Duration::from_secs(60),
    multiplier: 2.0,
    max_retries: 5,
    jitter_factor: 0.1,
};
```

**Features**:
- Exponential delay progression: 100ms → 200ms → 400ms → 800ms...
- Configurable maximum delay and retry limits
- Jitter to prevent thundering herd problems
- Low cost and no external resource requirements

### 2. Checkpoint Recovery Strategy

**Purpose**: Restore system state from previously saved checkpoints.

**Use Cases**:
- State corruption
- Concurrency conflicts
- System consistency issues

**Features**:
- Automatic checkpoint creation at regular intervals
- State serialization including agent states, tasks, and configurations
- Configurable checkpoint retention (default: 10 checkpoints)
- Fast state restoration with high confidence

**Usage**:
```rust
let strategy = CheckpointRecoveryStrategy::new(max_checkpoints: 10);
let checkpoint_id = strategy.create_checkpoint(&coordinator).await?;
let restored = strategy.restore_from_checkpoint(&coordinator, Some(checkpoint_id)).await?;
```

### 3. Graceful Degradation Strategy

**Purpose**: Reduce system functionality to conserve resources when under stress.

**Use Cases**:
- Resource exhaustion (memory, CPU, connections)
- Swarm size limits exceeded
- Performance degradation

**Features**:
- Resource usage monitoring (CPU, memory, connections, tasks)
- Automatic load shedding
- Functionality reduction rather than complete failure
- Maintains core services while reducing non-essential features

**Resource Monitoring**:
```rust
let monitor = ResourceMonitor::new();
monitor.update_cpu_usage(0.85);
monitor.update_memory_usage(0.90);
let usage = monitor.get_usage().await;
```

### 4. Self-Healing Strategy

**Purpose**: Apply automatic healing rules based on error patterns.

**Use Cases**:
- Known error patterns with established solutions
- Component restarts
- Configuration adjustments
- Alert generation

**Healing Actions**:
- **RestartComponent**: Restart a failed service or component
- **AdjustConfig**: Modify configuration parameters
- **ReallocateResources**: Redistribute system resources
- **TriggerAlert**: Notify administrators
- **ExecuteScript**: Run custom recovery scripts

**Custom Rules**:
```rust
let rule = HealingRule {
    rule_id: "network_failure".to_string(),
    error_pattern: "NetworkTimeout".to_string(),
    healing_action: HealingAction::RestartComponent("network_manager".to_string()),
    cooldown_duration: Duration::from_secs(300),
    last_applied: None,
};
strategy.add_healing_rule(rule).await;
```

## Integration with SwarmCoordinator

### Recovery Manager Integration

The `SwarmCoordinator` includes a recovery manager for automatic error handling:

```rust
pub struct SwarmCoordinator {
    // ... other fields
    recovery_manager: Arc<RecoveryManager>,
    // ...
}

impl SwarmCoordinator {
    pub fn recovery_manager(&self) -> &Arc<RecoveryManager> {
        &self.recovery_manager
    }
    
    pub async fn create_checkpoint(&self) -> SwarmResult<Uuid> {
        self.recovery_manager.create_checkpoint(self).await
    }
    
    pub async fn recover_from_error(
        &self,
        error: SwarmError,
        failed_task_id: Option<TaskId>,
        failed_agent_id: Option<AgentId>,
    ) -> SwarmResult<RecoveryResult> {
        // Recovery logic...
    }
}
```

### Enhanced Task Execution

Tasks can be executed with automatic recovery:

```rust
// Instead of direct task execution
let result = coordinator.execute_task(task_id).await;

// Use recovery-enhanced execution
let result = coordinator.execute_task_with_recovery(task_id).await;
```

## Usage Examples

### Basic Recovery Manager Setup

```rust
use synapsed_swarm::recovery::*;

let recovery_manager = RecoveryManager::new();

// Start background monitoring
recovery_manager.start_monitoring().await;

// Add custom strategies if needed
let custom_strategy = Arc::new(MyCustomStrategy::new());
recovery_manager.add_strategy(custom_strategy).await;
```

### Manual Recovery Attempt

```rust
let coordinator = Arc::new(SwarmCoordinator::new(config));
let error = SwarmError::CommunicationError("Network timeout".to_string());

let result = recovery_manager.recover(
    coordinator, 
    error, 
    Some(task_id), 
    Some(agent_id)
).await?;

if result.success {
    println!("Recovery successful: {}", result.action_taken);
} else {
    println!("Recovery failed, manual intervention required");
}
```

### Checkpoint Management

```rust
// Create checkpoint before risky operation
let checkpoint_id = coordinator.create_checkpoint().await?;

// Perform risky operation
match risky_operation().await {
    Ok(result) => {
        // Success, continue normally
    }
    Err(error) => {
        // Attempt recovery
        coordinator.recover_from_error(error, None, None).await?;
    }
}
```

## Configuration

### Recovery Manager Configuration

```rust
let recovery_manager = RecoveryManager::with_config(RecoveryConfig {
    max_concurrent_recoveries: 3,
    history_retention: 100,
    checkpoint_interval: Duration::from_secs(300),
    monitoring_interval: Duration::from_secs(30),
});
```

### Strategy-Specific Configuration

Each recovery strategy can be configured individually:

```rust
// Exponential backoff configuration
let backoff_config = BackoffConfig {
    initial_delay: Duration::from_millis(50),
    max_delay: Duration::from_secs(30),
    multiplier: 1.5,
    max_retries: 8,
    jitter_factor: 0.2,
};

// Checkpoint configuration
let checkpoint_strategy = CheckpointRecoveryStrategy::new(15); // Keep 15 checkpoints

// Graceful degradation thresholds
let degradation_config = DegradationConfig {
    cpu_threshold: 0.80,
    memory_threshold: 0.85,
    connection_threshold: 1000,
    task_threshold: 500,
};
```

## Monitoring and Observability

### Recovery Statistics

```rust
let history = recovery_manager.get_recovery_history().await;
let successful_recoveries = history.iter().filter(|a| a.result.success).count();
let success_rate = successful_recoveries as f64 / history.len() as f64;

println!("Recovery success rate: {:.1}%", success_rate * 100.0);
```

### Resource Monitoring

```rust
let resource_monitor = recovery_manager.resource_monitor();
let usage = resource_monitor.get_usage().await;

println!("System load:");
println!("  CPU: {:.1}%", usage.cpu_percent * 100.0);
println!("  Memory: {:.1}%", usage.memory_percent * 100.0);
println!("  Active connections: {}", usage.active_connections);
println!("  Active tasks: {}", usage.active_tasks);
```

### Recovery Events

The recovery system emits structured events that can be observed:

```rust
// Recovery events are logged with structured data
tracing::info!(
    recovery_attempt_id = %attempt_id,
    strategy_used = strategy.strategy_id(),
    error_type = %error,
    success = result.success,
    confidence = result.confidence,
    duration_ms = result.recovery_duration.as_millis(),
    "Recovery attempt completed"
);
```

## Error Handling

### Recoverable vs Non-Recoverable Errors

The recovery system categorizes errors:

**Recoverable Errors** (handled by strategies):
- `CommunicationError`: Network failures, timeouts
- `CoordinationTimeout`: Task execution timeouts
- `SwarmSizeLimitExceeded`: Resource limits
- `ResourceConflict`: Resource contention
- `ConcurrencyError`: Race conditions
- `TransactionFailed`: Database/storage failures

**Non-Recoverable Errors** (require manual intervention):
- `AgentNotFound`: Missing required components
- `InvalidConfiguration`: Fundamental configuration errors
- `ProtocolMismatch`: Version incompatibilities

### Recovery Result Interpretation

```rust
match recovery_result {
    RecoveryResult { success: true, confidence, .. } if confidence > 0.8 => {
        // High confidence recovery, continue normally
    }
    RecoveryResult { success: true, confidence, continue_recovery: true, .. } => {
        // Partial recovery, may need additional attempts
    }
    RecoveryResult { success: false, continue_recovery: false, .. } => {
        // Recovery failed, manual intervention required
    }
    _ => {
        // Other cases, evaluate based on specific needs
    }
}
```

## Best Practices

### 1. Strategy Ordering

Strategies are executed in order of increasing cost:
- Exponential Backoff (cost: 0.1) - Try first
- Self-Healing (cost: 0.2) - Low-cost automation
- Checkpoint Recovery (cost: 0.3) - Medium cost but effective
- Graceful Degradation (cost: 0.4) - Reduces functionality

### 2. Checkpoint Timing

Create checkpoints at strategic points:
- Before major state changes
- After successful complex operations
- At regular intervals during normal operation
- Before high-risk operations

### 3. Resource Monitoring

Monitor key metrics:
- CPU and memory usage trends
- Connection pool utilization
- Task queue depth
- Error rates and patterns

### 4. Custom Strategy Development

When implementing custom strategies:
- Implement proper error pattern matching
- Provide accurate cost estimates
- Include comprehensive logging
- Test with various error scenarios
- Consider resource requirements

## Testing

### Unit Testing Recovery Strategies

```rust
#[tokio::test]
async fn test_exponential_backoff_strategy() {
    let strategy = ExponentialBackoffStrategy::new(BackoffConfig::default());
    
    let error = SwarmError::CommunicationError("test".to_string());
    assert!(strategy.can_handle(&error).await);
    
    let context = create_test_context();
    let result = strategy.recover(&context, &error).await;
    assert!(result.success);
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_recovery_manager_integration() {
    let manager = RecoveryManager::new();
    let coordinator = Arc::new(SwarmCoordinator::new(config));
    
    let error = SwarmError::CommunicationError("test".to_string());
    let result = manager.recover(coordinator, error, None, None).await.unwrap();
    
    assert!(result.success);
    assert!(!result.action_taken.is_empty());
}
```

### Load Testing Recovery Systems

Test recovery under various load conditions:
- High error rates
- Resource exhaustion
- Concurrent recovery attempts
- Strategy cooldown behavior

## Troubleshooting

### Common Issues

**Recovery strategies not being applied**:
- Check error type matches strategy `can_handle()` logic
- Verify strategy is registered with recovery manager
- Check cooldown periods for self-healing rules

**Poor recovery performance**:
- Review strategy cost estimates
- Monitor resource usage during recovery
- Check for strategy conflicts or overlaps

**Recovery failures**:
- Examine recovery history for patterns
- Verify checkpoint validity and completeness
- Check external resource availability

### Debugging

Enable debug logging for detailed recovery information:

```rust
tracing_subscriber::fmt()
    .with_env_filter("synapsed_swarm::recovery=debug")
    .init();
```

### Metrics and Alerting

Set up monitoring for key recovery metrics:
- Recovery attempt frequency
- Success rates by strategy
- Resource usage trends
- Checkpoint creation/restoration times

## Future Enhancements

Planned improvements to the recovery system:

1. **Machine Learning Integration**: Learn from recovery patterns to improve strategy selection
2. **Distributed Recovery**: Coordinate recovery across multiple swarm instances
3. **Predictive Recovery**: Proactively apply recovery based on system health trends
4. **Custom Recovery Workflows**: Define complex multi-step recovery procedures
5. **Integration with External Monitoring**: Connect with monitoring systems like Prometheus/Grafana

## Conclusion

The recovery system provides a robust foundation for handling errors in the synapsed-swarm coordination system. By implementing multiple complementary strategies and providing comprehensive monitoring and configuration options, it ensures high availability and reliability of swarm operations.

The system is designed to be extensible, allowing for custom recovery strategies tailored to specific deployment environments and error patterns. Regular monitoring and tuning of recovery parameters will help maintain optimal system resilience.