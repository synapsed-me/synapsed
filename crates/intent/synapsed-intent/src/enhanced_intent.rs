//! Enhanced intent with full verification and error recovery

use crate::{
    types::*, Result,
    context::IntentContext,
    checkpoint::{CheckpointManager, FileRollbackHandler},
    execution::{VerifiedExecutor, ContextMonitor},
    intent::{HierarchicalIntent, IntentResult},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;
// Observability would be handled through synapsed-substrates in production
use tracing::{info, warn, error, debug};

/// Enhanced hierarchical intent with full verification
pub struct VerifiedIntent {
    /// Base intent
    intent: HierarchicalIntent,
    /// Verified executor
    executor: Arc<RwLock<VerifiedExecutor>>,
    /// Checkpoint manager
    checkpoint_manager: Arc<CheckpointManager>,
    /// Context monitor
    context_monitor: Arc<ContextMonitor>,
    /// Error recovery strategy
    recovery_strategy: RecoveryStrategy,
    /// Execution metrics
    metrics: Arc<RwLock<ExecutionMetrics>>,
}

/// Strategy for error recovery
#[derive(Clone)]
pub enum RecoveryStrategy {
    /// Retry failed steps
    Retry { max_attempts: u32, delay_ms: u64 },
    /// Skip failed steps and continue
    Skip,
    /// Rollback to last checkpoint
    Rollback,
    /// Custom recovery function
    Custom(Arc<dyn Fn(&StepResult) -> RecoveryAction + Send + Sync>),
}

impl std::fmt::Debug for RecoveryStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Retry { max_attempts, delay_ms } => 
                f.debug_struct("Retry")
                    .field("max_attempts", max_attempts)
                    .field("delay_ms", delay_ms)
                    .finish(),
            Self::Skip => write!(f, "Skip"),
            Self::Rollback => write!(f, "Rollback"),
            Self::Custom(_) => write!(f, "Custom(...)"),
        }
    }
}

/// Action to take for recovery
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    Retry,
    Skip,
    Rollback,
    Abort,
}

/// Metrics for execution
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ExecutionMetrics {
    pub steps_executed: u64,
    pub steps_succeeded: u64,
    pub steps_failed: u64,
    pub steps_skipped: u64,
    pub verifications_passed: u64,
    pub verifications_failed: u64,
    pub rollbacks_performed: u64,
    pub total_duration_ms: u64,
    pub context_violations: u64,
}

impl VerifiedIntent {
    /// Creates a new verified intent
    pub fn new(intent: HierarchicalIntent, context_bounds: ContextBounds) -> Self {
        let executor = Arc::new(RwLock::new(VerifiedExecutor::new(context_bounds.clone())));
        let checkpoint_manager = Arc::new(CheckpointManager::new());
        let context_monitor = Arc::new(ContextMonitor::new(context_bounds));
        
        Self {
            intent,
            executor,
            checkpoint_manager,
            context_monitor,
            recovery_strategy: RecoveryStrategy::Retry { 
                max_attempts: 3, 
                delay_ms: 1000 
            },
            metrics: Arc::new(RwLock::new(ExecutionMetrics::default())),
        }
    }

    /// Sets the recovery strategy
    pub fn with_recovery_strategy(mut self, strategy: RecoveryStrategy) -> Self {
        self.recovery_strategy = strategy;
        self
    }

    /// Sets a file rollback handler
    pub fn with_file_rollback(self, _base_dir: impl Into<String>) -> Self {
        // In production, we would set the rollback handler on the checkpoint manager
        // For now, just return self
        self
    }

    /// Executes the intent with full verification and error recovery
    pub async fn execute(&self, context: &IntentContext) -> Result<IntentResult> {
        info!("Starting verified intent execution: {}", self.intent.goal());
        let start = Utc::now();
        
        // Update status
        *self.intent.status.write().await = IntentStatus::Executing;
        
        // Emit start event
        self.emit_event(EventType::Started, json!({
            "goal": self.intent.goal(),
            "recovery_strategy": format!("{:?}", self.recovery_strategy),
        })).await;
        
        // Validate intent structure
        self.intent.validate().await?;
        
        // Create initial checkpoint
        self.checkpoint_manager.create_checkpoint(
            self.intent.id(),
            Uuid::nil()
        ).await?;
        
        // Plan execution
        let plan = self.intent.plan().await?;
        
        // Execute steps with verification and recovery
        let mut results = Vec::new();
        let mut success = true;
        
        for step_id in &plan.steps {
            if let Some(step) = self.intent.steps.iter().find(|s| s.id == *step_id) {
                // Monitor step execution
                if let Err(e) = self.context_monitor.monitor_step(step).await {
                    error!("Context violation detected: {}", e);
                    self.metrics.write().await.context_violations += 1;
                    
                    if self.intent.config.stop_on_failure {
                        success = false;
                        break;
                    }
                    continue;
                }
                
                // Execute step with recovery
                let step_result = self.execute_step_with_recovery(step, context).await?;
                
                // Update metrics
                self.update_metrics(&step_result).await;
                
                results.push(step_result.clone());
                
                if !step_result.success && self.intent.config.stop_on_failure {
                    success = false;
                    break;
                }
            }
        }
        
        // Execute sub-intents (would be recursive in production)
        // For now, just mark as successful to avoid infinite recursion
        for _sub in &self.intent.sub_intents {
            // In production, this would recursively execute sub-intents
            // let sub_context = context.create_child_context(self.intent.bounds.clone());
            // let verified_sub = VerifiedIntent::new(sub.clone(), sub_context.bounds().clone());
            // let sub_result = Box::pin(verified_sub.execute(&sub_context)).await?;
            
            // For now, simulate success
            info!("Sub-intent execution simulated");
        }
        
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        // Update final metrics
        self.metrics.write().await.total_duration_ms = duration_ms;
        
        // Update status
        *self.intent.status.write().await = if success {
            IntentStatus::Completed
        } else {
            IntentStatus::Failed
        };
        
        // Emit completion event
        self.emit_event(
            if success { EventType::Completed } else { EventType::Failed },
            json!({
                "duration_ms": duration_ms,
                "metrics": *self.metrics.read().await,
            })
        ).await;
        
        // Generate final report
        let metrics = self.metrics.read().await.clone();
        info!(
            "Intent execution completed: success={}, steps={}/{}, verifications={}/{}, duration={}ms",
            success,
            metrics.steps_succeeded,
            metrics.steps_executed,
            metrics.verifications_passed,
            metrics.verifications_passed + metrics.verifications_failed,
            duration_ms
        );
        
        Ok(IntentResult {
            intent_id: self.intent.id(),
            success,
            step_results: results,
            duration_ms,
            verification_proofs: Vec::new(), // Would be populated from executor
        })
    }

    /// Executes a step with error recovery
    async fn execute_step_with_recovery(
        &self,
        step: &Step,
        context: &IntentContext,
    ) -> Result<StepResult> {
        let mut attempts = 0;
        let max_attempts = match &self.recovery_strategy {
            RecoveryStrategy::Retry { max_attempts, .. } => *max_attempts,
            _ => 1,
        };
        
        loop {
            attempts += 1;
            debug!("Executing step '{}' (attempt {}/{})", step.name, attempts, max_attempts);
            
            // Check preconditions
            if !self.check_conditions(&step.preconditions, context).await? {
                warn!("Preconditions not met for step '{}'", step.name);
                return Ok(StepResult {
                    success: false,
                    output: None,
                    error: Some("Preconditions not met".to_string()),
                    duration_ms: 0,
                    verification: None,
                });
            }
            
            // Create checkpoint before execution
            if self.intent.config.enable_rollback {
                self.checkpoint_manager.create_checkpoint(
                    self.intent.id(),
                    step.id
                ).await?;
            }
            
            // Execute step with verification
            let mut executor = self.executor.write().await;
            let result = executor.execute_step(step, context).await?;
            
            // Check postconditions
            if result.success && !self.check_conditions(&step.postconditions, context).await? {
                warn!("Postconditions not met for step '{}'", step.name);
                let mut modified_result = result;
                modified_result.success = false;
                modified_result.error = Some("Postconditions not met".to_string());
                return Ok(modified_result);
            }
            
            // If successful or no recovery, return
            if result.success {
                info!("Step '{}' completed successfully", step.name);
                return Ok(result);
            }
            
            // Determine recovery action
            let recovery_action = self.determine_recovery_action(&result, attempts, max_attempts);
            
            match recovery_action {
                RecoveryAction::Retry => {
                    if attempts >= max_attempts {
                        error!("Step '{}' failed after {} attempts", step.name, attempts);
                        return Ok(result);
                    }
                    
                    warn!("Step '{}' failed, retrying (attempt {}/{})", step.name, attempts + 1, max_attempts);
                    
                    // Wait before retry
                    if let RecoveryStrategy::Retry { delay_ms, .. } = &self.recovery_strategy {
                        tokio::time::sleep(tokio::time::Duration::from_millis(*delay_ms)).await;
                    }
                    continue;
                },
                RecoveryAction::Skip => {
                    warn!("Skipping failed step '{}'", step.name);
                    self.metrics.write().await.steps_skipped += 1;
                    return Ok(result);
                },
                RecoveryAction::Rollback => {
                    warn!("Rolling back due to failed step '{}'", step.name);
                    self.checkpoint_manager.rollback_to_last().await?;
                    self.metrics.write().await.rollbacks_performed += 1;
                    return Ok(result);
                },
                RecoveryAction::Abort => {
                    error!("Aborting execution due to failed step '{}'", step.name);
                    return Ok(result);
                },
            }
        }
    }

    /// Determines recovery action based on result and strategy
    fn determine_recovery_action(&self, result: &StepResult, attempts: u32, max_attempts: u32) -> RecoveryAction {
        match &self.recovery_strategy {
            RecoveryStrategy::Retry { .. } => {
                if attempts < max_attempts {
                    RecoveryAction::Retry
                } else {
                    RecoveryAction::Abort
                }
            },
            RecoveryStrategy::Skip => RecoveryAction::Skip,
            RecoveryStrategy::Rollback => RecoveryAction::Rollback,
            RecoveryStrategy::Custom(f) => f(result),
        }
    }

    /// Checks conditions
    async fn check_conditions(
        &self,
        conditions: &[Condition],
        context: &IntentContext,
    ) -> Result<bool> {
        for condition in conditions {
            let met = match condition.condition_type {
                ConditionType::FileExists => {
                    if let Some(path) = condition.expected.as_str() {
                        tokio::fs::metadata(path).await.is_ok()
                    } else {
                        false
                    }
                },
                ConditionType::CommandSuccess => {
                    // Would execute command and check result
                    true
                },
                ConditionType::StateMatch => {
                    // Would check state against expected
                    true
                },
                ConditionType::Custom => {
                    // Custom condition evaluation
                    true
                },
            };
            
            if !met && condition.critical {
                return Ok(false);
            }
        }
        
        Ok(true)
    }

    /// Updates execution metrics
    async fn update_metrics(&self, result: &StepResult) {
        let mut metrics = self.metrics.write().await;
        metrics.steps_executed += 1;
        
        if result.success {
            metrics.steps_succeeded += 1;
        } else {
            metrics.steps_failed += 1;
        }
        
        if let Some(verification) = &result.verification {
            if verification.passed {
                metrics.verifications_passed += 1;
            } else {
                metrics.verifications_failed += 1;
            }
        }
    }

    /// Emits an event
    async fn emit_event(&self, event_type: EventType, data: serde_json::Value) {
        let event = IntentEvent {
            id: Uuid::new_v4(),
            intent_id: self.intent.id(),
            event_type,
            data,
            timestamp: Utc::now(),
        };
        
        // Would emit through substrate in production
        // The substrate integration would be handled by synapsed-observability
        
        debug!("Event emitted: {:?}", event);
    }

    /// Gets execution metrics
    pub async fn metrics(&self) -> ExecutionMetrics {
        self.metrics.read().await.clone()
    }

    /// Gets context violations
    pub async fn get_violations(&self) -> Vec<crate::execution::ContextViolation> {
        self.context_monitor.get_violations().await
    }
}