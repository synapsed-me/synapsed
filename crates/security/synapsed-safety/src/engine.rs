//! Main safety engine implementation
//!
//! This module provides the central orchestrator for all safety mechanisms,
//! integrating monitoring, constraint checking, and rollback capabilities.

use crate::constraint::DefaultConstraintEngine;
use crate::error::{Result, SafetyError};
use crate::monitor::DefaultSafetyMonitor;
use crate::rollback::DefaultRollbackManager;
use crate::traits::{ConstraintEngine, RollbackManager, SafetyMonitor, StateChangeCallback};
use crate::types::*;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn, error};
use uuid::Uuid;

/// Main safety engine that orchestrates all safety mechanisms
#[derive(Debug)]
pub struct SafetyEngine {
    /// Constraint engine for rule evaluation
    constraint_engine: Arc<RwLock<DefaultConstraintEngine>>,
    /// Safety monitor for state tracking
    safety_monitor: Arc<RwLock<DefaultSafetyMonitor>>,
    /// Rollback manager for state recovery
    rollback_manager: Arc<RwLock<DefaultRollbackManager>>,
    /// Engine configuration
    config: SafetyConfig,
    /// Engine statistics
    stats: Arc<RwLock<SafetyStats>>,
    /// Violation handler task
    violation_handler: Arc<RwLock<Option<JoinHandle<()>>>>,
    /// Violation notification channel
    violation_tx: Arc<RwLock<Option<mpsc::UnboundedSender<ConstraintViolation>>>>,
    /// Engine state
    engine_state: Arc<RwLock<EngineState>>,
    /// Last successful checkpoint
    last_checkpoint: Arc<RwLock<Option<CheckpointId>>>,
}

/// Internal engine state
#[derive(Debug, Clone)]
enum EngineState {
    /// Engine is initializing
    Initializing,
    /// Engine is running normally
    Running,
    /// Engine is in safe mode (violations detected)
    SafeMode,
    /// Engine is performing rollback
    RollingBack,
    /// Engine is shut down
    Shutdown,
    /// Engine encountered critical error
    Error(String),
}

/// Safety operation context
#[derive(Debug)]
struct SafetyOperation {
    pub id: Uuid,
    pub name: String,
    pub start_time: Instant,
    pub checkpoint_id: Option<CheckpointId>,
    pub metadata: HashMap<String, String>,
}

/// Callback for handling safety violations
struct ViolationHandler {
    engine: Arc<RwLock<SafetyEngine>>,
}

#[async_trait]
impl StateChangeCallback for ViolationHandler {
    async fn on_state_change(&mut self, _old_state: &SafetyState, new_state: &SafetyState) -> Result<()> {
        debug!("State change detected to: {}", new_state.id);
        
        // Note: In a real implementation, we would need to handle this differently
        // to avoid holding locks across await points. For now, we just log the change.
        // The rollback manager would be updated through a different mechanism.
        info!("Would update rollback manager with new state: {}", new_state.id);
        
        Ok(())
    }

    async fn on_violation(&mut self, violation: &ConstraintViolation) -> Result<()> {
        warn!("Constraint violation detected: {} ({})", 
              violation.constraint_id, violation.severity);
        
        // Send violation to handler
        let engine = self.engine.read();
        if let Some(tx) = engine.violation_tx.read().as_ref() {
            if let Err(e) = tx.send(violation.clone()) {
                error!("Failed to send violation notification: {}", e);
            }
        }
        
        Ok(())
    }

    async fn on_checkpoint_created(&mut self, checkpoint_id: &CheckpointId) -> Result<()> {
        info!("Checkpoint created: {}", checkpoint_id);
        
        let engine = self.engine.read();
        *engine.last_checkpoint.write() = Some(*checkpoint_id);
        
        Ok(())
    }

    async fn on_rollback(&mut self, checkpoint_id: &CheckpointId) -> Result<()> {
        info!("Rollback performed to checkpoint: {}", checkpoint_id);
        
        let engine = self.engine.read();
        *engine.engine_state.write() = EngineState::Running;
        
        Ok(())
    }
}

impl SafetyEngine {
    /// Create a new safety engine with default configuration
    pub async fn new() -> Result<Self> {
        Self::with_config(SafetyConfig::default()).await
    }

    /// Create a new safety engine with custom configuration
    pub async fn with_config(config: SafetyConfig) -> Result<Self> {
        info!("Initializing SafetyEngine with config: {:?}", config);
        
        let constraint_engine = Arc::new(RwLock::new(DefaultConstraintEngine::new()));
        let safety_monitor = Arc::new(RwLock::new(DefaultSafetyMonitor::new()));
        let rollback_manager = Arc::new(RwLock::new(DefaultRollbackManager::new()));
        
        let engine = Self {
            constraint_engine,
            safety_monitor,
            rollback_manager,
            config: config.clone(),
            stats: Arc::new(RwLock::new(SafetyStats {
                constraints_evaluated: 0,
                violations_detected: 0,
                rollbacks_performed: 0,
                checkpoints_created: 0,
                avg_evaluation_time_ms: 0.0,
                uptime_ms: 0,
                memory_stats: MemoryStats {
                    current_usage_bytes: 0,
                    peak_usage_bytes: 0,
                    avg_usage_bytes: 0,
                    checkpoint_memory_bytes: 0,
                    constraint_memory_bytes: 0,
                },
                performance_metrics: HashMap::new(),
            })),
            violation_handler: Arc::new(RwLock::new(None)),
            violation_tx: Arc::new(RwLock::new(None)),
            engine_state: Arc::new(RwLock::new(EngineState::Initializing)),
            last_checkpoint: Arc::new(RwLock::new(None)),
        };
        
        info!("SafetyEngine initialized successfully");
        Ok(engine)
    }

    /// Start the safety engine
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting SafetyEngine");
        
        *self.engine_state.write() = EngineState::Initializing;
        
        // Start violation handler
        self.start_violation_handler().await?;
        
        // Start safety monitor
        {
            let mut monitor = self.safety_monitor.write();
            monitor.start_monitoring().await?;
            
            // Set up state change callback
            let violation_handler = ViolationHandler { 
                engine: Arc::new(RwLock::new(SafetyEngine {
                    constraint_engine: Arc::clone(&self.constraint_engine),
                    safety_monitor: Arc::clone(&self.safety_monitor),
                    rollback_manager: Arc::clone(&self.rollback_manager),
                    config: self.config.clone(),
                    stats: Arc::clone(&self.stats),
                    violation_handler: Arc::clone(&self.violation_handler),
                    violation_tx: Arc::clone(&self.violation_tx),
                    engine_state: Arc::clone(&self.engine_state),
                    last_checkpoint: Arc::clone(&self.last_checkpoint),
                })),
            };
            
            monitor.subscribe_to_changes(Box::new(violation_handler)).await?;
        }
        
        // Create initial checkpoint
        let initial_state = self.safety_monitor.read().get_current_state().await?;
        {
            let mut rollback_manager = self.rollback_manager.write();
            rollback_manager.set_current_state(initial_state).await?;
            let checkpoint_id = rollback_manager
                .create_tagged_checkpoint(
                    Some("Initial engine state".to_string()),
                    vec!["initial".to_string(), "startup".to_string()],
                )
                .await?;
            
            *self.last_checkpoint.write() = Some(checkpoint_id);
            
            let mut stats = self.stats.write();
            stats.checkpoints_created += 1;
        }
        
        *self.engine_state.write() = EngineState::Running;
        
        info!("SafetyEngine started successfully");
        Ok(())
    }

    /// Stop the safety engine
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping SafetyEngine");
        
        *self.engine_state.write() = EngineState::Shutdown;
        
        // Stop violation handler
        if let Some(handle) = self.violation_handler.write().take() {
            handle.abort();
        }
        
        // Stop safety monitor
        {
            let mut monitor = self.safety_monitor.write();
            monitor.stop_monitoring().await?;
        }
        
        // Clear violation channel
        *self.violation_tx.write() = None;
        
        info!("SafetyEngine stopped successfully");
        Ok(())
    }

    /// Add a safety constraint
    pub async fn add_constraint(&mut self, constraint: Constraint) -> Result<()> {
        info!("Adding safety constraint: {} ({})", constraint.name, constraint.id);
        
        let mut constraint_engine = self.constraint_engine.write();
        constraint_engine.add_constraint(constraint).await?;
        
        Ok(())
    }

    /// Execute an operation with safety monitoring
    pub async fn execute_safe<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let operation_id = Uuid::new_v4();
        let start_time = Instant::now();
        
        info!("Starting safe operation: {}", operation_id);
        
        // Create checkpoint before operation
        let checkpoint_id = self.create_checkpoint().await?;
        
        let _safety_op = SafetyOperation {
            id: operation_id,
            name: "safe_operation".to_string(),
            start_time,
            checkpoint_id: Some(checkpoint_id),
            metadata: HashMap::new(),
        };
        
        // Execute operation in a separate task to handle panics
        let result = tokio::task::spawn_blocking(move || operation()).await;
        
        match result {
            Ok(Ok(value)) => {
                // Operation succeeded, validate final state
                match self.validate_current_state().await {
                    Ok(validation) if validation.passed => {
                        info!(
                            "Safe operation completed successfully: {} ({}ms)",
                            operation_id,
                            start_time.elapsed().as_millis()
                        );
                        
                        // Commit checkpoint
                        self.commit_checkpoint(&checkpoint_id).await?;
                        Ok(value)
                    }
                    Ok(validation) => {
                        // Validation failed, rollback
                        warn!(
                            "State validation failed after operation {}: {} violations",
                            operation_id, validation.violations.len()
                        );
                        
                        self.handle_violations(validation.violations).await?;
                        
                        Err(SafetyError::ConstraintViolation {
                            constraint_id: "post_operation_validation".to_string(),
                            message: "Operation resulted in constraint violations".to_string(),
                            severity: Severity::High,
                        })
                    }
                    Err(e) => {
                        error!(
                            "Failed to validate state after operation {}: {}",
                            operation_id, e
                        );
                        
                        // Rollback on validation error
                        self.rollback_to_checkpoint(&checkpoint_id).await?;
                        Err(e)
                    }
                }
            }
            Ok(Err(e)) => {
                // Operation failed, rollback
                warn!("Operation {} failed: {}", operation_id, e);
                self.rollback_to_checkpoint(&checkpoint_id).await?;
                Err(e)
            }
            Err(panic_err) => {
                // Operation panicked, rollback
                error!("Operation {} panicked: {}", operation_id, panic_err);
                self.rollback_to_checkpoint(&checkpoint_id).await?;
                Err(SafetyError::Critical {
                    message: format!("Operation panicked: {}", panic_err),
                })
            }
        }
    }

    /// Create a checkpoint of current state
    pub async fn create_checkpoint(&self) -> Result<CheckpointId> {
        debug!("Creating checkpoint");
        
        let current_state = self.safety_monitor.read().get_current_state().await?;
        
        let mut rollback_manager = self.rollback_manager.write();
        rollback_manager.set_current_state(current_state).await?;
        
        let checkpoint_id = rollback_manager
            .create_checkpoint(Some("Manual checkpoint".to_string()))
            .await?;
        
        *self.last_checkpoint.write() = Some(checkpoint_id);
        
        let mut stats = self.stats.write();
        stats.checkpoints_created += 1;
        
        info!("Checkpoint created: {}", checkpoint_id);
        Ok(checkpoint_id)
    }

    /// Commit a checkpoint (mark as permanent)
    pub async fn commit_checkpoint(&self, checkpoint_id: &CheckpointId) -> Result<()> {
        debug!("Committing checkpoint: {}", checkpoint_id);
        
        // In this implementation, checkpoints are automatically committed
        // In a more complex system, you might have temporary vs permanent checkpoints
        
        info!("Checkpoint committed: {}", checkpoint_id);
        Ok(())
    }

    /// Rollback to a specific checkpoint
    pub async fn rollback_to_checkpoint(&self, checkpoint_id: &CheckpointId) -> Result<()> {
        info!("Rolling back to checkpoint: {}", checkpoint_id);
        
        *self.engine_state.write() = EngineState::RollingBack;
        
        let mut rollback_manager = self.rollback_manager.write();
        rollback_manager.rollback_to_checkpoint(checkpoint_id).await?;
        
        let mut stats = self.stats.write();
        stats.rollbacks_performed += 1;
        
        *self.engine_state.write() = EngineState::Running;
        
        info!("Rollback completed to checkpoint: {}", checkpoint_id);
        Ok(())
    }

    /// Validate current system state
    pub async fn validate_current_state(&self) -> Result<ValidationResult> {
        let start_time = Instant::now();
        
        let current_state = self.safety_monitor.read().get_current_state().await?;
        let constraint_engine = self.constraint_engine.read();
        let result = constraint_engine.validate_state(&current_state).await?;
        
        let validation_time = start_time.elapsed();
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.constraints_evaluated += result.metadata.constraints_evaluated as u64;
            stats.violations_detected += result.violations.len() as u64;
            
            let new_time_ms = validation_time.as_millis() as f64;
            if stats.constraints_evaluated == result.metadata.constraints_evaluated as u64 {
                stats.avg_evaluation_time_ms = new_time_ms;
            } else {
                stats.avg_evaluation_time_ms = 
                    (stats.avg_evaluation_time_ms * (stats.constraints_evaluated - result.metadata.constraints_evaluated as u64) as f64 + new_time_ms)
                    / stats.constraints_evaluated as f64;
            }
        }
        
        debug!(
            "State validation completed: {} violations in {}ms",
            result.violations.len(),
            validation_time.as_millis()
        );
        
        Ok(result)
    }

    /// Handle constraint violations
    async fn handle_violations(&self, violations: Vec<ConstraintViolation>) -> Result<()> {
        if violations.is_empty() {
            return Ok(());
        }
        
        info!("Handling {} constraint violations", violations.len());
        
        // Categorize violations by severity
        let mut critical_violations = Vec::new();
        let mut high_violations = Vec::new();
        let mut other_violations = Vec::new();
        
        for violation in violations {
            match violation.severity {
                Severity::Critical => critical_violations.push(violation),
                Severity::High => high_violations.push(violation),
                _ => other_violations.push(violation),
            }
        }
        
        // Handle critical violations first
        if !critical_violations.is_empty() {
            error!("Critical violations detected: {}", critical_violations.len());
            *self.engine_state.write() = EngineState::SafeMode;
            
            // Automatic rollback for critical violations
            if let Some(checkpoint_id) = *self.last_checkpoint.read() {
                warn!("Performing automatic rollback due to critical violations");
                self.rollback_to_checkpoint(&checkpoint_id).await?;
            } else {
                error!("No checkpoint available for rollback");
                return Err(SafetyError::EmergencyShutdown {
                    reason: "Critical violations with no rollback checkpoint".to_string(),
                });
            }
        }
        
        // Handle high severity violations
        if !high_violations.is_empty() {
            warn!("High severity violations detected: {}", high_violations.len());
            
            // Execute constraint actions
            for violation in &high_violations {
                self.execute_constraint_actions(&violation.constraint_id).await?;
            }
        }
        
        // Log other violations
        if !other_violations.is_empty() {
            info!("Other violations detected: {}", other_violations.len());
            for violation in &other_violations {
                debug!("Violation: {} - {}", violation.constraint_id, violation.message);
            }
        }
        
        Ok(())
    }

    /// Execute actions for a violated constraint
    async fn execute_constraint_actions(&self, constraint_id: &ConstraintId) -> Result<()> {
        let constraint_engine = self.constraint_engine.read();
        let constraint = constraint_engine.get_constraint(constraint_id).await?;
        
        if let Some(constraint) = constraint {
            info!(
                "Executing {} actions for constraint: {}",
                constraint.actions.len(),
                constraint_id
            );
            
            for action in &constraint.actions {
                match action {
                    ConstraintAction::Log { level, message } => {
                        match level.as_str() {
                            "error" => error!("Constraint action: {}", message),
                            "warn" => warn!("Constraint action: {}", message),
                            "info" => info!("Constraint action: {}", message),
                            _ => debug!("Constraint action: {}", message),
                        }
                    }
                    ConstraintAction::Alert { channel, message, urgency } => {
                        info!(
                            "Alert [{}] ({}): {}",
                            channel, urgency, message
                        );
                        // In a real implementation, this would send actual alerts
                    }
                    ConstraintAction::Rollback { checkpoint_id, automatic } => {
                        if *automatic {
                            let target_checkpoint = checkpoint_id
                                .or(*self.last_checkpoint.read())
                                .ok_or_else(|| SafetyError::RollbackFailed {
                                    checkpoint_id: Uuid::nil(),
                                    reason: "No checkpoint available for automatic rollback".to_string(),
                                })?;
                            
                            warn!("Executing automatic rollback to: {}", target_checkpoint);
                            self.rollback_to_checkpoint(&target_checkpoint).await?;
                        }
                    }
                    ConstraintAction::Execute { command, parameters } => {
                        info!(
                            "Executing command: {} with parameters: {:?}",
                            command, parameters
                        );
                        // In a real implementation, this would execute actual commands
                    }
                    ConstraintAction::Throttle { rate_limit, duration_ms } => {
                        info!(
                            "Throttling operations: rate_limit={}, duration={}ms",
                            rate_limit, duration_ms
                        );
                        // In a real implementation, this would implement actual throttling
                    }
                    ConstraintAction::Shutdown { component, graceful } => {
                        warn!(
                            "Shutting down component: {} (graceful: {})",
                            component, graceful
                        );
                        // In a real implementation, this would shut down components
                    }
                    ConstraintAction::EmergencyStop => {
                        error!("Emergency stop triggered by constraint: {}", constraint_id);
                        return Err(SafetyError::EmergencyShutdown {
                            reason: format!("Emergency stop action from constraint: {}", constraint_id),
                        });
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Start violation handler task
    async fn start_violation_handler(&self) -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<ConstraintViolation>();
        *self.violation_tx.write() = Some(tx);
        
        let engine_weak = Arc::downgrade(&Arc::new(RwLock::new(SafetyEngine {
            constraint_engine: Arc::clone(&self.constraint_engine),
            safety_monitor: Arc::clone(&self.safety_monitor),
            rollback_manager: Arc::clone(&self.rollback_manager),
            config: self.config.clone(),
            stats: Arc::clone(&self.stats),
            violation_handler: Arc::clone(&self.violation_handler),
            violation_tx: Arc::clone(&self.violation_tx),
            engine_state: Arc::clone(&self.engine_state),
            last_checkpoint: Arc::clone(&self.last_checkpoint),
        })));
        
        let handle = tokio::spawn(async move {
            while let Some(violation) = rx.recv().await {
                if let Some(engine_arc) = engine_weak.upgrade() {
                    // Clone what we need before the await
                    let violations_to_handle = vec![violation];
                    drop(engine_arc); // Drop the Arc to allow async operation
                    
                    // Note: In a real implementation, we'd need a different approach
                    // to handle violations without holding the lock
                    error!("Violation received but async handling not fully implemented: {:?}", violations_to_handle);
                } else {
                    break; // Engine has been dropped
                }
            }
        });
        
        *self.violation_handler.write() = Some(handle);
        
        info!("Violation handler started");
        Ok(())
    }

    /// Get engine statistics
    pub async fn get_stats(&self) -> Result<SafetyStats> {
        let mut stats = self.stats.read().clone();
        
        // Update uptime
        // Note: In a real implementation, you'd track the actual start time
        stats.uptime_ms = 0; // Placeholder
        
        // Get memory stats from components
        let _constraint_stats = self.constraint_engine.read().get_stats().await?;
        let _monitor_stats = self.safety_monitor.read().get_stats().await?;
        let rollback_stats = self.rollback_manager.read().get_stats().await?;
        
        stats.memory_stats.constraint_memory_bytes = 1024 * 1024; // Placeholder
        stats.memory_stats.checkpoint_memory_bytes = rollback_stats.avg_checkpoint_size_bytes * rollback_stats.checkpoints_created as u64;
        
        Ok(stats)
    }

    /// Get current engine state
    pub fn get_engine_state(&self) -> EngineState {
        self.engine_state.read().clone()
    }

    /// Perform health check
    pub async fn health_check(&self) -> Result<EngineHealthStatus> {
        let mut issues = Vec::new();
        let mut performance_score: f64 = 1.0;
        
        // Check engine state
        match *self.engine_state.read() {
            EngineState::Running => {},
            EngineState::SafeMode => {
                issues.push("Engine is in safe mode".to_string());
                performance_score -= 0.3;
            }
            EngineState::Error(ref e) => {
                issues.push(format!("Engine error: {}", e));
                performance_score -= 0.8;
            }
            _ => {
                issues.push("Engine is not running".to_string());
                performance_score -= 0.5;
            }
        }
        
        // Check component health
        let monitor_health = self.safety_monitor.read().health_check().await?;
        if !monitor_health.healthy {
            issues.extend(monitor_health.issues);
            performance_score -= 0.2;
        }
        
        // Check recent violations
        let stats = self.stats.read();
        if stats.violations_detected > 0 {
            let violation_rate = stats.violations_detected as f64 / stats.constraints_evaluated.max(1) as f64;
            if violation_rate > 0.1 {
                issues.push(format!("High violation rate: {:.2}%", violation_rate * 100.0));
                performance_score -= 0.1;
            }
        }
        
        Ok(EngineHealthStatus {
            healthy: issues.is_empty(),
            issues,
            performance_score: performance_score.max(0.0),
            last_check: chrono::Utc::now(),
            engine_state: self.get_engine_state(),
        })
    }
}

/// Engine health status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EngineHealthStatus {
    pub healthy: bool,
    pub issues: Vec<String>,
    pub performance_score: f64,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub engine_state: EngineState,
}

// Make EngineState serializable
impl serde::Serialize for EngineState {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            EngineState::Initializing => serializer.serialize_str("Initializing"),
            EngineState::Running => serializer.serialize_str("Running"),
            EngineState::SafeMode => serializer.serialize_str("SafeMode"),
            EngineState::RollingBack => serializer.serialize_str("RollingBack"),
            EngineState::Shutdown => serializer.serialize_str("Shutdown"),
            EngineState::Error(e) => serializer.serialize_str(&format!("Error: {}", e)),
        }
    }
}

impl<'de> serde::Deserialize<'de> for EngineState {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "Initializing" => Ok(EngineState::Initializing),
            "Running" => Ok(EngineState::Running),
            "SafeMode" => Ok(EngineState::SafeMode),
            "RollingBack" => Ok(EngineState::RollingBack),
            "Shutdown" => Ok(EngineState::Shutdown),
            error_str if error_str.starts_with("Error: ") => {
                Ok(EngineState::Error(error_str[7..].to_string()))
            }
            _ => Ok(EngineState::Error(format!("Unknown state: {}", s))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::time::{sleep, Duration};

    async fn create_test_engine() -> SafetyEngine {
        SafetyEngine::new().await.unwrap()
    }

    #[tokio::test]
    async fn test_engine_lifecycle() {
        let mut engine = create_test_engine().await;
        
        // Start engine
        engine.start().await.unwrap();
        assert!(matches!(engine.get_engine_state(), EngineState::Running));
        
        // Wait a bit for initialization
        sleep(Duration::from_millis(100)).await;
        
        // Stop engine
        engine.stop().await.unwrap();
        assert!(matches!(engine.get_engine_state(), EngineState::Shutdown));
    }

    #[tokio::test]
    async fn test_constraint_management() {
        let mut engine = create_test_engine().await;
        
        // Add constraint
        let constraint = crate::constraint::DefaultConstraintEngine::balance_constraint();
        engine.add_constraint(constraint.clone()).await.unwrap();
        
        // Verify constraint was added
        let constraint_engine = engine.constraint_engine.read();
        let retrieved = constraint_engine.get_constraint(&constraint.id).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_safe_operation_success() {
        let mut engine = create_test_engine().await;
        engine.start().await.unwrap();
        
        // Wait for initialization
        sleep(Duration::from_millis(100)).await;
        
        // Execute safe operation that should succeed
        let result = engine.execute_safe(|| {
            Ok(42)
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        
        engine.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_safe_operation_failure() {
        let mut engine = create_test_engine().await;
        engine.start().await.unwrap();
        
        // Wait for initialization
        sleep(Duration::from_millis(100)).await;
        
        // Execute safe operation that should fail
        let result = engine.execute_safe(|| {
            Err(SafetyError::critical("Test failure"))
        }).await;
        
        assert!(result.is_err());
        
        engine.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_checkpoint_operations() {
        let mut engine = create_test_engine().await;
        engine.start().await.unwrap();
        
        // Wait for initialization
        sleep(Duration::from_millis(100)).await;
        
        // Create checkpoint
        let checkpoint_id = engine.create_checkpoint().await.unwrap();
        assert!(!checkpoint_id.is_nil());
        
        // Commit checkpoint
        engine.commit_checkpoint(&checkpoint_id).await.unwrap();
        
        // Rollback to checkpoint
        engine.rollback_to_checkpoint(&checkpoint_id).await.unwrap();
        
        engine.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_state_validation() {
        let mut engine = create_test_engine().await;
        engine.start().await.unwrap();
        
        // Wait for initialization
        sleep(Duration::from_millis(100)).await;
        
        // Validate current state
        let validation = engine.validate_current_state().await.unwrap();
        assert!(validation.passed); // Should pass with no constraints
        
        engine.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_engine_health_check() {
        let mut engine = create_test_engine().await;
        
        // Health check when not started
        let health = engine.health_check().await.unwrap();
        assert!(!health.healthy);
        assert!(health.performance_score < 1.0);
        
        // Start engine
        engine.start().await.unwrap();
        sleep(Duration::from_millis(100)).await;
        
        // Health check when running
        let health = engine.health_check().await.unwrap();
        // Should be healthier when running
        assert!(health.performance_score > 0.0);
        
        engine.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_engine_statistics() {
        let mut engine = create_test_engine().await;
        engine.start().await.unwrap();
        
        // Wait for some activity
        sleep(Duration::from_millis(200)).await;
        
        // Get stats
        let stats = engine.get_stats().await.unwrap();
        assert!(stats.checkpoints_created > 0); // Initial checkpoint
        
        engine.stop().await.unwrap();
    }
}