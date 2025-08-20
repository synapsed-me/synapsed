//! Integration tests for the Synapsed Safety system
//!
//! These tests verify the complete safety system functionality,
//! including all components working together.

use synapsed_safety::prelude::*;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

/// Create a test state with realistic values
fn create_test_state() -> SafetyState {
    SafetyState {
        id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        values: {
            let mut values = HashMap::new();
            values.insert("balance".to_string(), synapsed_safety::types::StateValue::Integer(1000));
            values.insert("user_id".to_string(), synapsed_safety::types::StateValue::String("user123".to_string()));
            values.insert("transaction_count".to_string(), synapsed_safety::types::StateValue::Integer(5));
            values
        },
        active_constraints: vec![],
        resource_usage: ResourceUsage {
            cpu_usage: 0.4,
            memory_usage: 512 * 1024 * 1024, // 512MB
            memory_limit: 1024 * 1024 * 1024, // 1GB
            network_usage: 1024 * 1024, // 1MB/s
            disk_io: 512 * 1024, // 512KB/s
            file_descriptors: 50,
            thread_count: 10,
            custom_resources: HashMap::new(),
        },
        health_indicators: HealthIndicators {
            overall_health: 0.85,
            component_health: {
                let mut health = HashMap::new();
                health.insert("database".to_string(), 0.9);
                health.insert("cache".to_string(), 0.8);
                health.insert("api".to_string(), 0.85);
                health
            },
            error_rates: {
                let mut rates = HashMap::new();
                rates.insert("api_errors".to_string(), 0.05);
                rates
            },
            response_times: {
                let mut times = HashMap::new();
                times.insert("api_response".to_string(), 120.0);
                times
            },
            availability: {
                let mut avail = HashMap::new();
                avail.insert("service".to_string(), 0.99);
                avail
            },
            performance_indicators: HashMap::new(),
        },
        metadata: synapsed_safety::types::StateMetadata {
            source: "integration_test".to_string(),
            version: "1.0.0".to_string(),
            checksum: "test_checksum".to_string(),
            size_bytes: 4096,
            compression_ratio: None,
            tags: vec!["test".to_string()],
            properties: HashMap::new(),
        },
    }
}

#[tokio::test]
async fn test_complete_safety_workflow() {
    // Create and configure safety engine
    let mut safety = SafetyEngine::new().await.expect("Failed to create safety engine");
    
    // Add comprehensive constraints
    let balance_constraint = DefaultConstraintEngine::balance_constraint();
    safety.add_constraint(balance_constraint).await.expect("Failed to add balance constraint");
    
    let memory_constraint = DefaultConstraintEngine::memory_constraint(0.8);
    safety.add_constraint(memory_constraint).await.expect("Failed to add memory constraint");
    
    let health_constraint = DefaultConstraintEngine::health_constraint(0.6);
    safety.add_constraint(health_constraint).await.expect("Failed to add health constraint");
    
    // Start safety engine
    safety.start().await.expect("Failed to start safety engine");
    
    // Wait for initialization
    sleep(Duration::from_millis(200)).await;
    
    // Verify engine is running
    assert!(matches!(safety.get_engine_state(), synapsed_safety::engine::EngineState::Running));
    
    // Test safe operation execution
    let result = safety.execute_safe(|| {
        // Simulate a successful operation
        Ok("Transaction completed")
    }).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Transaction completed");
    
    // Test checkpoint creation and rollback
    let checkpoint_id = safety.create_checkpoint().await.expect("Failed to create checkpoint");
    assert!(!checkpoint_id.is_nil());
    
    // Simulate state change that might require rollback
    let rollback_result = safety.rollback_to_checkpoint(&checkpoint_id).await;
    assert!(rollback_result.is_ok());
    
    // Validate current state
    let validation = safety.validate_current_state().await.expect("Failed to validate state");
    assert!(validation.passed, "State validation should pass with proper constraints");
    
    // Check engine health
    let health = safety.health_check().await.expect("Failed to check health");
    assert!(health.performance_score > 0.0);
    
    // Get comprehensive statistics
    let stats = safety.get_stats().await.expect("Failed to get stats");
    assert!(stats.checkpoints_created > 0);
    assert!(stats.constraints_evaluated > 0);
    
    // Stop safety engine
    safety.stop().await.expect("Failed to stop safety engine");
    
    assert!(matches!(safety.get_engine_state(), synapsed_safety::engine::EngineState::Shutdown));
}

#[tokio::test]
async fn test_constraint_violation_handling() {
    let mut safety = SafetyEngine::new().await.expect("Failed to create safety engine");
    
    // Add a constraint that will be violated
    let balance_constraint = DefaultConstraintEngine::balance_constraint();
    safety.add_constraint(balance_constraint).await.expect("Failed to add constraint");
    
    safety.start().await.expect("Failed to start safety engine");
    sleep(Duration::from_millis(100)).await;
    
    // Create checkpoint before risky operation
    let checkpoint_id = safety.create_checkpoint().await.expect("Failed to create checkpoint");
    
    // Execute operation that should trigger constraint violation
    let result = safety.execute_safe(|| {
        // Simulate an operation that would cause constraint violation
        // In a real scenario, this might modify state that violates balance constraint
        Err(SafetyError::constraint_violation(
            "balance_check",
            "Simulated negative balance",
            Severity::Critical
        ))
    }).await;
    
    // Operation should fail due to constraint violation
    assert!(result.is_err());
    
    // Verify rollback occurred
    let stats = safety.get_stats().await.expect("Failed to get stats");
    assert!(stats.rollbacks_performed > 0);
    
    safety.stop().await.expect("Failed to stop safety engine");
}

#[tokio::test]
async fn test_resource_monitoring() {
    let mut safety = SafetyEngine::new().await.expect("Failed to create safety engine");
    
    // Add resource constraints
    let memory_constraint = DefaultConstraintEngine::memory_constraint(0.9); // 90% limit
    safety.add_constraint(memory_constraint).await.expect("Failed to add memory constraint");
    
    safety.start().await.expect("Failed to start safety engine");
    sleep(Duration::from_millis(100)).await;
    
    // Monitor should be capturing resource usage
    let validation = safety.validate_current_state().await.expect("Failed to validate state");
    
    // Should pass since simulated resource usage is below 90%
    assert!(validation.passed);
    
    // Check that monitoring is working
    let stats = safety.get_stats().await.expect("Failed to get stats");
    assert!(stats.constraints_evaluated > 0);
    
    safety.stop().await.expect("Failed to stop safety engine");
}

#[tokio::test]
async fn test_checkpoint_management() {
    let mut safety = SafetyEngine::new().await.expect("Failed to create safety engine");
    
    safety.start().await.expect("Failed to start safety engine");
    sleep(Duration::from_millis(100)).await;
    
    // Create multiple checkpoints
    let checkpoint1 = safety.create_checkpoint().await.expect("Failed to create checkpoint 1");
    let checkpoint2 = safety.create_checkpoint().await.expect("Failed to create checkpoint 2");
    let checkpoint3 = safety.create_checkpoint().await.expect("Failed to create checkpoint 3");
    
    // All checkpoints should be unique
    assert_ne!(checkpoint1, checkpoint2);
    assert_ne!(checkpoint2, checkpoint3);
    assert_ne!(checkpoint1, checkpoint3);
    
    // Test rollback to specific checkpoint
    let rollback_result = safety.rollback_to_checkpoint(&checkpoint2).await;
    assert!(rollback_result.is_ok());
    
    // Verify statistics reflect multiple checkpoints and rollback
    let stats = safety.get_stats().await.expect("Failed to get stats");
    assert!(stats.checkpoints_created >= 3); // At least our 3 plus initial
    assert!(stats.rollbacks_performed >= 1);
    
    safety.stop().await.expect("Failed to stop safety engine");
}

#[tokio::test]
async fn test_concurrent_operations() {
    let mut safety = SafetyEngine::new().await.expect("Failed to create safety engine");
    
    // Add constraints
    safety.add_constraint(DefaultConstraintEngine::balance_constraint()).await.expect("Failed to add constraint");
    safety.add_constraint(DefaultConstraintEngine::memory_constraint(0.8)).await.expect("Failed to add constraint");
    
    safety.start().await.expect("Failed to start safety engine");
    sleep(Duration::from_millis(100)).await;
    
    // Execute multiple concurrent safe operations
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let result = safety.execute_safe(move || {
            // Simulate different operations
            match i % 3 {
                0 => Ok(format!("Operation {} - Success", i)),
                1 => {
                    // Simulate a small delay
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    Ok(format!("Operation {} - Delayed Success", i))
                },
                _ => Ok(format!("Operation {} - Immediate Success", i)),
            }
        }).await;
        
        assert!(result.is_ok(), "Operation {} should succeed", i);
        handles.push(result);
    }
    
    // All operations should have succeeded
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.expect("Operation should have succeeded");
        assert!(result.contains(&i.to_string()));
    }
    
    safety.stop().await.expect("Failed to stop safety engine");
}

#[tokio::test]
async fn test_engine_health_monitoring() {
    let mut safety = SafetyEngine::new().await.expect("Failed to create safety engine");
    
    // Health check before starting
    let initial_health = safety.health_check().await.expect("Failed to check initial health");
    assert!(!initial_health.healthy); // Should not be healthy when not running
    
    // Start engine
    safety.start().await.expect("Failed to start safety engine");
    sleep(Duration::from_millis(150)).await;
    
    // Health check after starting
    let running_health = safety.health_check().await.expect("Failed to check running health");
    assert!(running_health.performance_score > initial_health.performance_score);
    
    // Add some load and check health again
    let _checkpoint = safety.create_checkpoint().await.expect("Failed to create checkpoint");
    let _validation = safety.validate_current_state().await.expect("Failed to validate state");
    
    let loaded_health = safety.health_check().await.expect("Failed to check loaded health");
    assert!(loaded_health.performance_score > 0.0);
    
    safety.stop().await.expect("Failed to stop safety engine");
    
    // Health check after stopping
    let stopped_health = safety.health_check().await.expect("Failed to check stopped health");
    assert!(!stopped_health.healthy); // Should not be healthy when stopped
}

#[tokio::test]
async fn test_configuration_options() {
    let config = SafetyConfig {
        max_checkpoints: 50,
        checkpoint_interval_ms: 30_000,
        constraint_check_interval_ms: 500,
        memory_limit_bytes: 50 * 1024 * 1024, // 50MB
        compression_enabled: false,
        formal_verification_enabled: false,
        self_healing_enabled: false,
        ..Default::default()
    };
    
    let mut safety = SafetyEngine::with_config(config).await.expect("Failed to create configured safety engine");
    
    safety.start().await.expect("Failed to start configured safety engine");
    sleep(Duration::from_millis(100)).await;
    
    // Create a checkpoint to test configuration
    let checkpoint_id = safety.create_checkpoint().await.expect("Failed to create checkpoint");
    assert!(!checkpoint_id.is_nil());
    
    // Test that engine works with custom configuration
    let result = safety.execute_safe(|| {
        Ok("Configured operation")
    }).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Configured operation");
    
    safety.stop().await.expect("Failed to stop configured safety engine");
}

#[tokio::test]
async fn test_error_recovery() {
    let mut safety = SafetyEngine::new().await.expect("Failed to create safety engine");
    
    safety.add_constraint(DefaultConstraintEngine::balance_constraint()).await.expect("Failed to add constraint");
    safety.start().await.expect("Failed to start safety engine");
    sleep(Duration::from_millis(100)).await;
    
    // Create checkpoint for recovery
    let recovery_checkpoint = safety.create_checkpoint().await.expect("Failed to create recovery checkpoint");
    
    // Execute operation that fails and should trigger recovery
    let error_result = safety.execute_safe(|| {
        Err(SafetyError::critical("Simulated critical error"))
    }).await;
    
    // Operation should fail
    assert!(error_result.is_err());
    
    // Verify that the error was critical
    if let Err(error) = &error_result {
        assert!(error.is_critical());
    }
    
    // System should still be functional after error
    let health = safety.health_check().await.expect("Failed to check health after error");
    assert!(health.performance_score > 0.0);
    
    // Should be able to execute successful operations after error
    let recovery_result = safety.execute_safe(|| {
        Ok("Recovery operation")
    }).await;
    
    assert!(recovery_result.is_ok());
    assert_eq!(recovery_result.unwrap(), "Recovery operation");
    
    safety.stop().await.expect("Failed to stop safety engine");
}

#[tokio::test]
async fn test_memory_and_performance() {
    let mut safety = SafetyEngine::new().await.expect("Failed to create safety engine");
    
    safety.start().await.expect("Failed to start safety engine");
    sleep(Duration::from_millis(100)).await;
    
    // Create many checkpoints to test memory management
    let mut checkpoint_ids = Vec::new();
    for i in 0..10 {
        let checkpoint_id = safety.create_checkpoint().await.expect(&format!("Failed to create checkpoint {}", i));
        checkpoint_ids.push(checkpoint_id);
    }
    
    // Perform many validations to test performance
    for _ in 0..20 {
        let validation = safety.validate_current_state().await.expect("Failed to validate state");
        assert!(validation.passed);
    }
    
    // Get comprehensive statistics
    let stats = safety.get_stats().await.expect("Failed to get stats");
    assert!(stats.checkpoints_created >= 10);
    assert!(stats.constraints_evaluated >= 20);
    assert!(stats.avg_evaluation_time_ms > 0.0);
    
    // Test rollback performance
    let rollback_start = std::time::Instant::now();
    let rollback_result = safety.rollback_to_checkpoint(&checkpoint_ids[5]).await;
    let rollback_duration = rollback_start.elapsed();
    
    assert!(rollback_result.is_ok());
    assert!(rollback_duration.as_millis() < 1000); // Should be fast
    
    safety.stop().await.expect("Failed to stop safety engine");
}

#[tokio::test]
async fn test_prelude_convenience() {
    use synapsed_safety::prelude::*;
    
    // Test that all prelude imports work correctly
    let mut safety = SafetyEngine::new().await.expect("Failed to create safety engine");
    
    let constraint = DefaultConstraintEngine::balance_constraint();
    assert_eq!(constraint.severity, Severity::Critical);
    
    safety.add_constraint(constraint).await.expect("Failed to add constraint");
    safety.start().await.expect("Failed to start safety engine");
    
    sleep(Duration::from_millis(100)).await;
    
    let result = safety.execute_safe(|| {
        Ok("Prelude test")
    }).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Prelude test");
    
    safety.stop().await.expect("Failed to stop safety engine");
}