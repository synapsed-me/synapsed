//! Comprehensive tests for the recovery system

use synapsed_swarm::{
    prelude::*,
    recovery::*,
    error::SwarmError,
    types::*,
};
use std::{sync::Arc, time::Duration};
use tokio_test;
use uuid::Uuid;

/// Test exponential backoff strategy
#[tokio::test]
async fn test_exponential_backoff_strategy() {
    let config = BackoffConfig {
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(1000),
        multiplier: 2.0,
        max_retries: 3,
        jitter_factor: 0.0, // No jitter for predictable testing
    };
    
    let strategy = ExponentialBackoffStrategy::new(config);
    
    // Test strategy identification
    assert_eq!(strategy.strategy_id(), "exponential_backoff");
    assert!(!strategy.description().is_empty());
    assert!(!strategy.requires_external_resources().await);
    
    // Test error handling capability
    let comm_error = SwarmError::CommunicationError("test".to_string());
    assert!(strategy.can_handle(&comm_error).await);
    
    let timeout_error = SwarmError::CoordinationTimeout(30);
    assert!(strategy.can_handle(&timeout_error).await);
    
    let unhandled_error = SwarmError::AgentNotFound(Uuid::new_v4());
    assert!(!strategy.can_handle(&unhandled_error).await);
    
    // Test delay calculation
    assert_eq!(strategy.calculate_delay(0), Duration::from_millis(10));
    assert_eq!(strategy.calculate_delay(1), Duration::from_millis(20));
    assert_eq!(strategy.calculate_delay(2), Duration::from_millis(40));
    
    // Test recovery attempt
    let swarm_config = SwarmConfig::default();
    let coordinator = Arc::new(SwarmCoordinator::new(swarm_config));
    
    let context = RecoveryContext {
        coordinator: coordinator.clone(),
        swarm_state: coordinator.state().await,
        failed_task_id: None,
        failed_agent_id: None,
        retry_count: 1,
        error_timestamp: chrono::Utc::now(),
        metadata: std::collections::HashMap::new(),
    };
    
    let result = strategy.recover(&context, &comm_error).await;
    assert!(result.success);
    assert!(result.confidence > 0.5);
    assert!(result.continue_recovery);
}

/// Test checkpoint recovery strategy
#[tokio::test]
async fn test_checkpoint_recovery_strategy() {
    let strategy = CheckpointRecoveryStrategy::new(5);
    
    // Test strategy identification
    assert_eq!(strategy.strategy_id(), "checkpoint_recovery");
    assert!(!strategy.description().is_empty());
    assert!(!strategy.requires_external_resources().await);
    
    // Test error handling capability
    let concurrency_error = SwarmError::ConcurrencyError("test".to_string());
    assert!(strategy.can_handle(&concurrency_error).await);
    
    let transaction_error = SwarmError::TransactionFailed("test".to_string());
    assert!(strategy.can_handle(&transaction_error).await);
    
    // Test checkpoint creation and restoration
    let swarm_config = SwarmConfig::default();
    let coordinator = SwarmCoordinator::new(swarm_config);
    coordinator.initialize().await.unwrap();
    
    let checkpoint_id = strategy.create_checkpoint(&coordinator).await.unwrap();
    assert!(!checkpoint_id.is_nil());
    
    // Test checkpoint restoration
    let restored_checkpoint = strategy
        .restore_from_checkpoint(&coordinator, Some(checkpoint_id))
        .await
        .unwrap();
    
    assert_eq!(restored_checkpoint.checkpoint_id, checkpoint_id);
    assert_eq!(restored_checkpoint.swarm_state.swarm_id, coordinator.state().await.swarm_id);
}

/// Test graceful degradation strategy
#[tokio::test]
async fn test_graceful_degradation_strategy() {
    let resource_monitor = Arc::new(ResourceMonitor::new());
    let strategy = GracefulDegradationStrategy::new(resource_monitor.clone());
    
    // Test strategy identification
    assert_eq!(strategy.strategy_id(), "graceful_degradation");
    assert!(!strategy.description().is_empty());
    assert!(!strategy.requires_external_resources().await);
    
    // Test error handling capability
    let size_limit_error = SwarmError::SwarmSizeLimitExceeded { current: 10, max: 5 };
    assert!(strategy.can_handle(&size_limit_error).await);
    
    let resource_conflict_error = SwarmError::ResourceConflict { 
        resource: "cpu".to_string() 
    };
    assert!(strategy.can_handle(&resource_conflict_error).await);
    
    // Test recovery attempt
    let swarm_config = SwarmConfig::default();
    let coordinator = Arc::new(SwarmCoordinator::new(swarm_config));
    
    let context = RecoveryContext {
        coordinator: coordinator.clone(),
        swarm_state: coordinator.state().await,
        failed_task_id: None,
        failed_agent_id: None,
        retry_count: 0,
        error_timestamp: chrono::Utc::now(),
        metadata: std::collections::HashMap::new(),
    };
    
    let result = strategy.recover(&context, &size_limit_error).await;
    assert!(result.success);
    assert!(!result.continue_recovery); // Graceful degradation typically doesn't continue
    
    // Test resource monitor updates
    resource_monitor.update_cpu_usage(0.8);
    resource_monitor.update_memory_usage(0.6);
    resource_monitor.increment_connections();
    resource_monitor.increment_tasks();
    
    let usage = resource_monitor.get_usage().await;
    assert_eq!(usage.cpu_percent, 0.8);
    assert_eq!(usage.memory_percent, 0.6);
    assert_eq!(usage.active_connections, 1);
    assert_eq!(usage.active_tasks, 1);
}

/// Test self-healing strategy
#[tokio::test]
async fn test_self_healing_strategy() {
    let strategy = SelfHealingStrategy::new();
    
    // Test strategy identification
    assert_eq!(strategy.strategy_id(), "self_healing");
    assert!(!strategy.description().is_empty());
    assert!(strategy.requires_external_resources().await);
    
    // Test error handling capability with default rules
    let comm_error = SwarmError::CommunicationError("test failure".to_string());
    assert!(strategy.can_handle(&comm_error).await);
    
    let timeout_error = SwarmError::CoordinationTimeout(60);
    assert!(strategy.can_handle(&timeout_error).await);
    
    // Test adding custom healing rule
    let custom_rule = HealingRule {
        rule_id: "custom_rule".to_string(),
        error_pattern: "CustomError".to_string(),
        healing_action: HealingAction::TriggerAlert("Custom error detected".to_string()),
        cooldown_duration: Duration::from_secs(120),
        last_applied: None,
    };
    
    strategy.add_healing_rule(custom_rule).await;
    
    // Test recovery attempt
    let swarm_config = SwarmConfig::default();
    let coordinator = Arc::new(SwarmCoordinator::new(swarm_config));
    
    let context = RecoveryContext {
        coordinator: coordinator.clone(),
        swarm_state: coordinator.state().await,
        failed_task_id: None,
        failed_agent_id: None,
        retry_count: 0,
        error_timestamp: chrono::Utc::now(),
        metadata: std::collections::HashMap::new(),
    };
    
    let result = strategy.recover(&context, &comm_error).await;
    assert!(result.success);
    assert!(!result.continue_recovery); // Self-healing typically resolves the issue
    assert!(result.confidence > 0.7);
}

/// Test resource monitor functionality
#[tokio::test]
async fn test_resource_monitor() {
    let monitor = ResourceMonitor::new();
    
    // Test initial state
    let initial_usage = monitor.get_usage().await;
    assert_eq!(initial_usage.cpu_percent, 0.0);
    assert_eq!(initial_usage.memory_percent, 0.0);
    assert_eq!(initial_usage.active_connections, 0);
    assert_eq!(initial_usage.active_tasks, 0);
    
    // Test updates
    monitor.update_cpu_usage(0.45);
    monitor.update_memory_usage(0.32);
    monitor.increment_connections();
    monitor.increment_connections();
    monitor.increment_tasks();
    
    let updated_usage = monitor.get_usage().await;
    assert_eq!(updated_usage.cpu_percent, 0.45);
    assert_eq!(updated_usage.memory_percent, 0.32);
    assert_eq!(updated_usage.active_connections, 2);
    assert_eq!(updated_usage.active_tasks, 1);
    
    // Test decrements
    monitor.decrement_connections();
    monitor.decrement_tasks();
    
    let final_usage = monitor.get_usage().await;
    assert_eq!(final_usage.active_connections, 1);
    assert_eq!(final_usage.active_tasks, 0);
}

/// Test recovery manager coordination
#[tokio::test]
async fn test_recovery_manager() {
    let manager = RecoveryManager::new();
    
    // Test that default strategies are loaded
    let strategies = manager.strategies.read().await;
    assert!(strategies.len() >= 4); // Should have at least the 4 default strategies
    drop(strategies);
    
    // Test custom strategy addition
    let custom_strategy = Arc::new(TestRecoveryStrategy::new());
    manager.add_strategy(custom_strategy.clone()).await;
    
    let strategies = manager.strategies.read().await;
    assert!(strategies.len() >= 5);
    drop(strategies);
    
    // Test recovery attempt
    let swarm_config = SwarmConfig::default();
    let coordinator = Arc::new(SwarmCoordinator::new(swarm_config));
    coordinator.initialize().await.unwrap();
    
    let error = SwarmError::CommunicationError("test error".to_string());
    let result = manager
        .recover(coordinator.clone(), error, None, None)
        .await
        .unwrap();
    
    assert!(result.success);
    
    // Test recovery history
    let history = manager.get_recovery_history().await;
    assert!(!history.is_empty());
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].strategy_used, "exponential_backoff");
    
    // Test resource monitor access
    let resource_monitor = manager.resource_monitor();
    resource_monitor.update_cpu_usage(0.5);
    let usage = resource_monitor.get_usage().await;
    assert_eq!(usage.cpu_percent, 0.5);
}

/// Test checkpoint integration with coordinator
#[tokio::test]
async fn test_coordinator_checkpoint_integration() {
    let swarm_config = SwarmConfig::default();
    let coordinator = SwarmCoordinator::new(swarm_config);
    coordinator.initialize().await.unwrap();
    
    // Test checkpoint creation through coordinator
    let checkpoint_id = coordinator.create_checkpoint().await.unwrap();
    assert!(!checkpoint_id.is_nil());
    
    // Test recovery manager access
    let recovery_manager = coordinator.recovery_manager();
    assert!(recovery_manager.resource_monitor().get_usage().await.cpu_percent >= 0.0);
}

/// Test comprehensive recovery scenario
#[tokio::test]
async fn test_comprehensive_recovery_scenario() {
    let mut swarm_config = SwarmConfig::default();
    swarm_config.max_agents = 2;
    
    let coordinator = Arc::new(SwarmCoordinator::new(swarm_config));
    coordinator.initialize().await.unwrap();
    
    // Start monitoring
    coordinator.recovery_manager().start_monitoring().await;
    
    // Create initial checkpoint
    let checkpoint_id = coordinator.create_checkpoint().await.unwrap();
    assert!(!checkpoint_id.is_nil());
    
    // Simulate various error scenarios and recovery
    let errors = vec![
        SwarmError::CommunicationError("network timeout".to_string()),
        SwarmError::CoordinationTimeout(30),
        SwarmError::SwarmSizeLimitExceeded { current: 3, max: 2 },
        SwarmError::ConcurrencyError("race condition".to_string()),
    ];
    
    for error in errors {
        let result = coordinator
            .recover_from_error(error.clone(), None, None)
            .await
            .unwrap();
        
        assert!(result.success, "Recovery failed for error: {:?}", error);
        assert!(result.confidence > 0.0);
    }
    
    // Check recovery history
    let history = coordinator.recovery_manager().get_recovery_history().await;
    assert!(history.len() >= 4);
    
    // Verify all recoveries were recorded
    let successful_recoveries = history.iter().filter(|attempt| attempt.result.success).count();
    assert_eq!(successful_recoveries, 4);
}

/// Test strategy cost ordering
#[tokio::test]
async fn test_strategy_cost_ordering() {
    let manager = RecoveryManager::new();
    let strategies = manager.strategies.read().await;
    
    let mut costs = Vec::new();
    for strategy in strategies.iter() {
        costs.push(strategy.cost_estimate().await);
    }
    
    // Test that we have a range of costs (not all the same)
    assert!(costs.iter().any(|&cost| cost < 0.3));
    assert!(costs.iter().any(|&cost| cost > 0.3));
}

/// Mock recovery strategy for testing
struct TestRecoveryStrategy {
    id: String,
}

impl TestRecoveryStrategy {
    fn new() -> Self {
        Self {
            id: "test_strategy".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl RecoveryStrategy for TestRecoveryStrategy {
    fn strategy_id(&self) -> &str {
        &self.id
    }
    
    fn description(&self) -> &str {
        "Test recovery strategy for unit tests"
    }
    
    async fn can_handle(&self, error: &SwarmError) -> bool {
        matches!(error, SwarmError::Other(_))
    }
    
    async fn recover(
        &self,
        _context: &RecoveryContext,
        _error: &SwarmError,
    ) -> RecoveryResult {
        RecoveryResult {
            success: true,
            action_taken: "Test recovery action".to_string(),
            recovery_duration: Duration::from_millis(1),
            new_state: None,
            confidence: 0.95,
            continue_recovery: false,
            metadata: std::collections::HashMap::new(),
        }
    }
    
    async fn cost_estimate(&self) -> f64 {
        0.05 // Very low cost for testing
    }
    
    async fn requires_external_resources(&self) -> bool {
        false
    }
}

/// Test error recovery with different retry patterns
#[tokio::test]
async fn test_error_recovery_patterns() {
    let config = BackoffConfig {
        initial_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(100),
        multiplier: 2.0,
        max_retries: 3,
        jitter_factor: 0.0,
    };
    
    let strategy = ExponentialBackoffStrategy::new(config);
    
    // Test different retry counts
    for retry_count in 0..5 {
        let delay = strategy.calculate_delay(retry_count);
        
        if retry_count <= 3 {
            assert!(delay.as_millis() > 0);
            assert!(delay.as_millis() <= 100);
        }
        
        // Each delay should be roughly double the previous (without jitter)
        if retry_count > 0 && retry_count <= 3 {
            let prev_delay = strategy.calculate_delay(retry_count - 1);
            assert!(delay >= prev_delay, 
                "Delay should increase: retry {}, delay {:?} vs prev {:?}", 
                retry_count, delay, prev_delay);
        }
    }
}

/// Test healing rule cooldown mechanism
#[tokio::test]
async fn test_healing_rule_cooldown() {
    let strategy = SelfHealingStrategy::new();
    
    // Add a rule with short cooldown for testing
    let rule = HealingRule {
        rule_id: "cooldown_test".to_string(),
        error_pattern: "CooldownTest".to_string(),
        healing_action: HealingAction::TriggerAlert("Test alert".to_string()),
        cooldown_duration: Duration::from_millis(100),
        last_applied: None,
    };
    
    strategy.add_healing_rule(rule).await;
    
    let test_error = SwarmError::Other(anyhow::anyhow!("CooldownTest error"));
    
    // First application should work
    assert!(strategy.can_handle(&test_error).await);
    
    // Wait for cooldown to expire
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    // Should work again after cooldown
    assert!(strategy.can_handle(&test_error).await);
}

/// Integration test with swarm coordinator recovery methods
#[tokio::test]
async fn test_coordinator_recovery_integration() {
    let swarm_config = SwarmConfig::default();
    let coordinator = SwarmCoordinator::new(swarm_config);
    coordinator.initialize().await.unwrap();
    
    // Test recovery manager access
    let recovery_manager = coordinator.recovery_manager();
    assert!(recovery_manager.resource_monitor().get_usage().await.cpu_percent >= 0.0);
    
    // Test checkpoint creation
    let checkpoint_id = coordinator.create_checkpoint().await.unwrap();
    assert!(!checkpoint_id.is_nil());
    
    // Test error recovery
    let error = SwarmError::CommunicationError("test".to_string());
    let result = coordinator
        .recover_from_error(error, None, None)
        .await
        .unwrap();
    
    assert!(result.success);
    assert!(!result.action_taken.is_empty());
}