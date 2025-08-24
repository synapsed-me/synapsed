//! # Synapsed Safety
//!
//! Self-aware safety mechanisms, constraint engines, and automatic rollback systems.
//! 
//! This crate provides comprehensive safety guarantees for distributed systems through:
//! - **Constraint Engines**: Formal constraint specification and checking
//! - **Rollback Mechanisms**: Automatic state recovery on safety violations
//! - **Self-Aware Systems**: Dynamic safety boundary detection
//! - **Formal Verification**: Mathematical proofs of safety properties
//!
//! ## Features
//!
//! - **Constraint Specification**: Express safety requirements formally
//! - **Real-time Monitoring**: Continuous constraint validation
//! - **Automatic Rollback**: Instant recovery from unsafe states
//! - **Memory Compression**: Efficient state history management
//! - **Self-Healing**: Automatic adaptation to prevent future violations
//!
//! ## Example
//!
//! ```rust,no_run
//! use synapsed_safety::{SafetyEngine, constraint::DefaultConstraintEngine};
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut engine = SafetyEngine::new().await?;
//!     
//!     // Define safety constraints
//!     let constraint = DefaultConstraintEngine::balance_constraint();
//!     engine.add_constraint(constraint).await?;
//!     
//!     // Start safety monitoring
//!     engine.start().await?;
//!     
//!     // Create rollback point
//!     let checkpoint = engine.create_checkpoint().await?;
//!     
//!     // Perform operations with safety monitoring
//!     match engine.execute_safe(|| {
//!         // Some potentially unsafe operation
//!         Ok("operation_result")
//!     }).await {
//!         Ok(result) => println!("Operation succeeded: {:?}", result),
//!         Err(_) => {
//!             // Automatic rollback on constraint violation
//!             println!("Operation rolled back due to safety violation");
//!         }
//!     }
//!     
//!     engine.stop().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! The safety system is built around several key components:
//!
//! ### SafetyEngine
//! 
//! The central orchestrator that coordinates all safety mechanisms:
//! - Integrates monitoring, constraint checking, and rollback
//! - Provides high-level safety operations
//! - Manages the overall safety lifecycle
//!
//! ### ConstraintEngine
//!
//! Evaluates safety rules against system state:
//! - Supports custom constraint expressions
//! - Provides caching for performance
//! - Enables/disables constraints dynamically
//!
//! ### SafetyMonitor
//!
//! Real-time system state monitoring:
//! - Captures resource usage metrics
//! - Tracks health indicators
//! - Detects significant state changes
//!
//! ### RollbackManager
//!
//! Checkpoint and recovery operations:
//! - Creates state snapshots
//! - Manages checkpoint retention
//! - Performs state recovery
//!
//! ## Safety Patterns
//!
//! ### Critical Section Protection
//!
//! ```rust,no_run
//! use synapsed_safety::SafetyEngine;
//!
//! async fn transfer_funds(engine: &SafetyEngine, amount: i64) -> Result<(), Box<dyn std::error::Error>> {
//!     engine.execute_safe(|| {
//!         // Database transaction with automatic rollback on violation
//!         // update_balance(-amount)?;
//!         // validate_balance_constraints()?;
//!         Ok(())
//!     }).await
//! }
//! ```
//!
//! ### Resource Management
//!
//! ```rust,no_run
//! use synapsed_safety::constraint::DefaultConstraintEngine;
//!
//! // Create memory usage constraint
//! let memory_constraint = DefaultConstraintEngine::memory_constraint(0.8); // 80% limit
//! engine.add_constraint(memory_constraint).await?;
//! ```
//!
//! ### Health Monitoring
//!
//! ```rust,no_run
//! use synapsed_safety::constraint::DefaultConstraintEngine;
//!
//! // Create health check constraint
//! let health_constraint = DefaultConstraintEngine::health_constraint(0.7); // 70% minimum
//! engine.add_constraint(health_constraint).await?;
//! ```
//!
//! ## Integration with Synapsed Core
//!
//! The safety system integrates with the broader Synapsed ecosystem:
//!
//! - **Observability**: Hooks into synapsed-core's observability system
//! - **Storage**: Uses synapsed-storage for checkpoint persistence
//! - **Network**: Monitors network-related safety constraints
//! - **Identity**: Enforces identity and access safety rules
//!
//! ## Performance Characteristics
//!
//! | Operation | Typical Latency | Memory Overhead | Use Case |
//! |-----------|----------------|-----------------|----------|
//! | Constraint Check | < 1ms | Minimal | Always-on monitoring |
//! | Checkpoint Creation | 5-50ms | O(state size) | Transactional safety |
//! | Rollback Operation | 10-100ms | Minimal | Error recovery |
//! | State Monitoring | 1-10ms | Low | Real-time tracking |
//!
//! ## Error Handling
//!
//! The safety system uses a comprehensive error hierarchy:
//!
//! - **SafetyError**: All safety-related errors
//! - **ConstraintViolation**: Rule violations with severity levels
//! - **RollbackFailed**: Recovery operation failures
//! - **MonitorError**: Monitoring system issues
//! - **Critical**: System-threatening conditions
//!
//! ## Testing
//!
//! ```bash
//! # Run all safety tests
//! cargo test --package synapsed-safety
//!
//! # Run with specific features
//! cargo test --package synapsed-safety --features "formal-verification"
//!
//! # Run integration tests
//! cargo test --test integration_tests
//!
//! # Run benchmarks
//! cargo bench --package synapsed-safety
//! ```

pub mod error;
pub mod types;
pub mod traits;

// Core safety components
pub mod constraint;
pub mod engine;
pub mod rollback;
pub mod monitor;

// Verification systems
#[cfg(feature = "verification")]
pub mod verification;

#[cfg(feature = "formal-verification")]
pub mod formal;

// Self-healing mechanisms
#[cfg(feature = "self-healing")]
pub mod healing;

// Re-exports for convenience
pub use error::{SafetyError, Result};
pub use types::{Constraint, SafetyState, Severity, CheckpointId, SafetyConfig};
pub use traits::{SafetyMonitor, ConstraintEngine, RollbackManager};
pub use engine::SafetyEngine;

// Re-export main implementations
pub use constraint::DefaultConstraintEngine;
pub use monitor::DefaultSafetyMonitor;
pub use rollback::DefaultRollbackManager;

// Common constraint builders
pub mod prelude {
    //! Common imports for safety operations
    
    pub use crate::{
        SafetyEngine,
        SafetyError,
        Result,
        Constraint,
        SafetyState,
        Severity,
        CheckpointId,
    };
    
    pub use crate::constraint::DefaultConstraintEngine;
    pub use crate::monitor::DefaultSafetyMonitor;
    pub use crate::rollback::DefaultRollbackManager;
    
    pub use crate::traits::{
        SafetyMonitor,
        ConstraintEngine,
        RollbackManager,
        StateChangeCallback,
    };
    
    pub use crate::types::{
        SafetyConfig,
        ValidationResult,
        ConstraintViolation,
        ResourceUsage,
        HealthIndicators,
    };
}

/// Convenience function to create a basic safety engine
/// 
/// This is equivalent to `SafetyEngine::new().await` but provides
/// a more ergonomic API for simple use cases.
/// 
/// # Example
/// 
/// ```rust,no_run
/// use synapsed_safety::create_safety_engine;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut safety = create_safety_engine().await?;
///     safety.start().await?;
///     
///     // Use the safety engine..
///     
///     safety.stop().await?;
///     Ok(())
/// }
/// ```
pub async fn create_safety_engine() -> Result<SafetyEngine> {
    SafetyEngine::new().await
}

/// Convenience function to create a safety engine with custom configuration
/// 
/// # Example
/// 
/// ```rust,no_run
/// use synapsed_safety::{create_safety_engine_with_config, types::SafetyConfig};
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = SafetyConfig {
///         max_checkpoints: 50,
///         compression_enabled: true,
///         formal_verification_enabled: false,
///         ..Default::default()
///     };
///     
///     let mut safety = create_safety_engine_with_config(config).await?;
///     safety.start().await?;
///     
///     // Use the safety engine...
///     
///     safety.stop().await?;
///     Ok(())
/// }
/// ```
pub async fn create_safety_engine_with_config(config: SafetyConfig) -> Result<SafetyEngine> {
    SafetyEngine::with_config(config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_create_safety_engine() {
        let engine = create_safety_engine().await;
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_create_safety_engine_with_config() {
        let config = SafetyConfig {
            max_checkpoints: 25,
            ..Default::default()
        };
        
        let engine = create_safety_engine_with_config(config).await;
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_basic_safety_workflow() {
        let mut engine = create_safety_engine().await.unwrap();
        
        // Add a constraint
        let constraint = DefaultConstraintEngine::balance_constraint();
        engine.add_constraint(constraint).await.unwrap();
        
        // Start engine
        engine.start().await.unwrap();
        
        // Wait for initialization
        sleep(Duration::from_millis(100)).await;
        
        // Execute safe operation
        let result = engine.execute_safe(|| {
            Ok("test_result")
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_result");
        
        // Stop engine
        engine.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_safety_with_multiple_constraints() {
        let mut engine = create_safety_engine().await.unwrap();
        
        // Add multiple constraints
        engine.add_constraint(DefaultConstraintEngine::balance_constraint()).await.unwrap();
        engine.add_constraint(DefaultConstraintEngine::memory_constraint(0.8)).await.unwrap();
        engine.add_constraint(DefaultConstraintEngine::health_constraint(0.7)).await.unwrap();
        
        // Start engine
        engine.start().await.unwrap();
        
        // Wait for initialization
        sleep(Duration::from_millis(100)).await;
        
        // Validate current state
        let validation = engine.validate_current_state().await.unwrap();
        // Should pass with default test state
        assert!(validation.passed);
        
        // Check health
        let health = engine.health_check().await.unwrap();
        assert!(health.performance_score > 0.0);
        
        // Stop engine
        engine.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_checkpoint_and_rollback() {
        let mut engine = create_safety_engine().await.unwrap();
        engine.start().await.unwrap();
        
        // Wait for initialization
        sleep(Duration::from_millis(100)).await;
        
        // Create checkpoint
        let checkpoint_id = engine.create_checkpoint().await.unwrap();
        assert!(!checkpoint_id.is_nil());
        
        // Rollback to checkpoint
        engine.rollback_to_checkpoint(&checkpoint_id).await.unwrap();
        
        // Get stats to verify operations
        let stats = engine.get_stats().await.unwrap();
        assert!(stats.checkpoints_created > 0);
        assert!(stats.rollbacks_performed > 0);
        
        engine.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_prelude_imports() {
        use crate::prelude::*;
        
        // Test that prelude imports work
        let engine = SafetyEngine::new().await;
        assert!(engine.is_ok());
        
        let constraint = DefaultConstraintEngine::balance_constraint();
        assert_eq!(constraint.severity, Severity::Critical);
        
        let config = SafetyConfig::default();
        assert_eq!(config.max_checkpoints, 100);
    }

    #[tokio::test]
    async fn test_error_handling() {
        let mut engine = create_safety_engine().await.unwrap();
        engine.start().await.unwrap();
        
        // Wait for initialization
        sleep(Duration::from_millis(100)).await;
        
        // Execute operation that fails
        let result = engine.execute_safe(|| {
            Err(SafetyError::critical("Test error"))
        }).await;
        
        assert!(result.is_err());
        if let Err(error) = result {
            assert!(error.is_critical());
        }
        
        engine.stop().await.unwrap();
    }
}