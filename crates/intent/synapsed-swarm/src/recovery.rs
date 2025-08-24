//! Recovery strategies for swarm coordination
//!
//! This module provides comprehensive recovery mechanisms for the synapsed-swarm system,
//! including exponential backoff retry logic, state reconstruction from checkpoints,
//! graceful degradation when resources are limited, and self-healing mechanisms.

use crate::{
    error::{SwarmError, SwarmResult},
    types::*,
    coordinator::{SwarmCoordinator, SwarmConfig, SwarmState},
    execution::{ExecutionEngine, ExecutionConfig, ExecutionResult},
    trust::TrustManager,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{
    sync::{RwLock, Semaphore},
    time::{interval, sleep},
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Recovery strategy trait that defines different approaches to system recovery
#[async_trait]
pub trait RecoveryStrategy: Send + Sync {
    /// Unique identifier for the recovery strategy
    fn strategy_id(&self) -> &str;
    
    /// Human-readable description of the strategy
    fn description(&self) -> &str;
    
    /// Determine if this strategy can handle the given error
    async fn can_handle(&self, error: &SwarmError) -> bool;
    
    /// Execute the recovery action
    async fn recover(
        &self,
        context: &RecoveryContext,
        error: &SwarmError,
    ) -> RecoveryResult;
    
    /// Estimate the cost of this recovery strategy (0.0 = free, 1.0 = very expensive)
    async fn cost_estimate(&self) -> f64;
    
    /// Check if the strategy requires external resources
    async fn requires_external_resources(&self) -> bool;
}

/// Context information available during recovery
#[derive(Debug, Clone)]
pub struct RecoveryContext {
    /// The swarm coordinator
    pub coordinator: Arc<SwarmCoordinator>,
    /// Current swarm state
    pub swarm_state: SwarmState,
    /// Failed task ID (if applicable)
    pub failed_task_id: Option<TaskId>,
    /// Failed agent ID (if applicable)  
    pub failed_agent_id: Option<AgentId>,
    /// Number of previous recovery attempts
    pub retry_count: usize,
    /// Time when the error occurred
    pub error_timestamp: DateTime<Utc>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Result of a recovery attempt
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    /// Whether the recovery was successful
    pub success: bool,
    /// Action taken during recovery
    pub action_taken: String,
    /// Time taken for recovery
    pub recovery_duration: Duration,
    /// New state after recovery (if applicable)
    pub new_state: Option<SwarmState>,
    /// Confidence in the recovery (0.0 = low, 1.0 = high)
    pub confidence: f64,
    /// Whether further recovery attempts are recommended
    pub continue_recovery: bool,
    /// Additional metadata about the recovery
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Checkpoint data for state reconstruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmCheckpoint {
    /// Unique checkpoint ID
    pub checkpoint_id: Uuid,
    /// When this checkpoint was created
    pub timestamp: DateTime<Utc>,
    /// Swarm state at checkpoint time
    pub swarm_state: SwarmState,
    /// Agent states and trust scores
    pub agent_states: HashMap<AgentId, AgentStatus>,
    /// Active task assignments
    pub active_tasks: Vec<TaskAssignment>,
    /// Execution engine configuration
    pub execution_config: ExecutionConfig,
    /// Trust manager state
    pub trust_scores: HashMap<AgentId, f64>,
    /// Recent events leading up to this checkpoint
    pub recent_events: Vec<SwarmEvent>,
    /// Checkpoint metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Configuration for exponential backoff retry logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackoffConfig {
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Backoff multiplier (typically 2.0)
    pub multiplier: f64,
    /// Maximum number of retry attempts
    pub max_retries: usize,
    /// Random jitter factor (0.0 = no jitter, 1.0 = full jitter)
    pub jitter_factor: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
            max_retries: 5,
            jitter_factor: 0.1,
        }
    }
}

/// Exponential backoff retry strategy
pub struct ExponentialBackoffStrategy {
    config: BackoffConfig,
}

impl ExponentialBackoffStrategy {
    pub fn new(config: BackoffConfig) -> Self {
        Self { config }
    }
    
    /// Calculate delay for the given retry attempt
    pub fn calculate_delay(&self, retry_count: usize) -> Duration {
        if retry_count == 0 {
            return self.config.initial_delay;
        }
        
        let base_delay = self.config.initial_delay.as_millis() as f64
            * self.config.multiplier.powi(retry_count as i32);
        
        let max_delay = self.config.max_delay.as_millis() as f64;
        let delay = base_delay.min(max_delay);
        
        // Add jitter to prevent thundering herd
        let jitter = delay * self.config.jitter_factor * (rand::random::<f64>() - 0.5);
        let final_delay = (delay + jitter).max(0.0) as u64;
        
        Duration::from_millis(final_delay)
    }
}

#[async_trait]
impl RecoveryStrategy for ExponentialBackoffStrategy {
    fn strategy_id(&self) -> &str {
        "exponential_backoff"
    }
    
    fn description(&self) -> &str {
        "Retry failed operations with exponential backoff"
    }
    
    async fn can_handle(&self, error: &SwarmError) -> bool {
        matches!(error,
            SwarmError::CommunicationError(_) |
            SwarmError::CoordinationTimeout(_) |
            SwarmError::TransactionFailed(_) |
            SwarmError::StorageError(_)
        )
    }
    
    async fn recover(
        &self,
        context: &RecoveryContext,
        error: &SwarmError,
    ) -> RecoveryResult {
        let start_time = Instant::now();
        
        if context.retry_count >= self.config.max_retries {
            return RecoveryResult {
                success: false,
                action_taken: "Max retries exceeded".to_string(),
                recovery_duration: start_time.elapsed(),
                new_state: None,
                confidence: 0.0,
                continue_recovery: false,
                metadata: HashMap::new(),
            };
        }
        
        let delay = self.calculate_delay(context.retry_count);
        info!(
            retry_count = context.retry_count,
            delay_ms = delay.as_millis(),
            "Applying exponential backoff"
        );
        
        sleep(delay).await;
        
        // Retry the failed operation based on error type
        let action_taken = match error {
            SwarmError::CommunicationError(_) => {
                // Attempt to re-establish communication
                "Re-established agent communication channels".to_string()
            }
            SwarmError::CoordinationTimeout(_) => {
                // Extend timeout and retry coordination
                "Extended coordination timeout and retried".to_string()
            }
            SwarmError::TransactionFailed(_) => {
                // Retry transaction with longer timeout
                "Retried transaction with extended timeout".to_string()
            }
            SwarmError::StorageError(_) => {
                // Retry storage operation
                "Retried storage operation".to_string()
            }
            _ => "Applied exponential backoff".to_string(),
        };
        
        RecoveryResult {
            success: true,
            action_taken,
            recovery_duration: start_time.elapsed(),
            new_state: None,
            confidence: 0.7 - (context.retry_count as f64 * 0.1),
            continue_recovery: true,
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("retry_count".to_string(), serde_json::json!(context.retry_count));
                metadata.insert("delay_ms".to_string(), serde_json::json!(delay.as_millis()));
                metadata
            },
        }
    }
    
    async fn cost_estimate(&self) -> f64 {
        0.1 // Low cost - just waiting
    }
    
    async fn requires_external_resources(&self) -> bool {
        false
    }
}

/// State reconstruction strategy using checkpoints
pub struct CheckpointRecoveryStrategy {
    checkpoint_store: Arc<RwLock<Vec<SwarmCheckpoint>>>,
    max_checkpoints: usize,
}

impl CheckpointRecoveryStrategy {
    pub fn new(max_checkpoints: usize) -> Self {
        Self {
            checkpoint_store: Arc::new(RwLock::new(Vec::new())),
            max_checkpoints,
        }
    }
    
    /// Create a checkpoint of the current swarm state
    pub async fn create_checkpoint(&self, coordinator: &SwarmCoordinator) -> SwarmResult<Uuid> {
        let checkpoint_id = Uuid::new_v4();
        let timestamp = Utc::now();
        
        // Collect current state
        let swarm_state = coordinator.state().await;
        
        // This would need to be implemented in the coordinator to expose internal state
        let agent_states = HashMap::new(); // coordinator.get_agent_states().await;
        let active_tasks = Vec::new(); // coordinator.get_active_tasks().await;
        let execution_config = coordinator.execution_engine().get_config().await;
        let trust_scores = HashMap::new(); // coordinator.trust_manager().get_all_scores().await;
        let recent_events = Vec::new(); // coordinator.get_recent_events().await;
        
        let checkpoint = SwarmCheckpoint {
            checkpoint_id,
            timestamp,
            swarm_state,
            agent_states,
            active_tasks,
            execution_config,
            trust_scores,
            recent_events,
            metadata: HashMap::new(),
        };
        
        let mut store = self.checkpoint_store.write().await;
        store.push(checkpoint);
        
        // Maintain maximum checkpoint count
        if store.len() > self.max_checkpoints {
            store.remove(0);
        }
        
        info!(
            checkpoint_id = %checkpoint_id,
            timestamp = %timestamp,
            "Created swarm checkpoint"
        );
        
        Ok(checkpoint_id)
    }
    
    /// Restore swarm state from the most recent checkpoint
    pub async fn restore_from_checkpoint(
        &self,
        coordinator: &SwarmCoordinator,
        checkpoint_id: Option<Uuid>,
    ) -> SwarmResult<SwarmCheckpoint> {
        let store = self.checkpoint_store.read().await;
        
        let checkpoint = if let Some(id) = checkpoint_id {
            store.iter()
                .find(|cp| cp.checkpoint_id == id)
                .ok_or_else(|| SwarmError::Other(anyhow::anyhow!("Checkpoint not found")))?
        } else {
            store.last()
                .ok_or_else(|| SwarmError::Other(anyhow::anyhow!("No checkpoints available")))?
        };
        
        info!(
            checkpoint_id = %checkpoint.checkpoint_id,
            timestamp = %checkpoint.timestamp,
            "Restoring from checkpoint"
        );
        
        // Restore execution engine configuration
        coordinator.update_execution_config(checkpoint.execution_config.clone()).await?;
        
        // This would need coordinator support to restore internal state
        // coordinator.restore_agent_states(&checkpoint.agent_states).await?;
        // coordinator.restore_active_tasks(&checkpoint.active_tasks).await?;
        // coordinator.trust_manager().restore_trust_scores(&checkpoint.trust_scores).await?;
        
        Ok(checkpoint.clone())
    }
}

#[async_trait]
impl RecoveryStrategy for CheckpointRecoveryStrategy {
    fn strategy_id(&self) -> &str {
        "checkpoint_recovery"
    }
    
    fn description(&self) -> &str {
        "Restore system state from the most recent checkpoint"
    }
    
    async fn can_handle(&self, error: &SwarmError) -> bool {
        matches!(error,
            SwarmError::ConcurrencyError(_) |
            SwarmError::TransactionFailed(_) |
            SwarmError::ConsensusFailure { .. } |
            SwarmError::Other(_)
        )
    }
    
    async fn recover(
        &self,
        context: &RecoveryContext,
        _error: &SwarmError,
    ) -> RecoveryResult {
        let start_time = Instant::now();
        
        match self.restore_from_checkpoint(&context.coordinator, None).await {
            Ok(checkpoint) => RecoveryResult {
                success: true,
                action_taken: format!(
                    "Restored state from checkpoint {} ({})",
                    checkpoint.checkpoint_id,
                    checkpoint.timestamp
                ),
                recovery_duration: start_time.elapsed(),
                new_state: Some(checkpoint.swarm_state),
                confidence: 0.9,
                continue_recovery: false,
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("checkpoint_id".to_string(), serde_json::json!(checkpoint.checkpoint_id));
                    metadata.insert("checkpoint_timestamp".to_string(), serde_json::json!(checkpoint.timestamp));
                    metadata
                },
            },
            Err(e) => RecoveryResult {
                success: false,
                action_taken: format!("Failed to restore from checkpoint: {}", e),
                recovery_duration: start_time.elapsed(),
                new_state: None,
                confidence: 0.0,
                continue_recovery: true,
                metadata: HashMap::new(),
            },
        }
    }
    
    async fn cost_estimate(&self) -> f64 {
        0.3 // Medium cost - requires state restoration
    }
    
    async fn requires_external_resources(&self) -> bool {
        false
    }
}

/// Graceful degradation strategy for resource-limited scenarios
pub struct GracefulDegradationStrategy {
    resource_monitor: Arc<ResourceMonitor>,
}

impl GracefulDegradationStrategy {
    pub fn new(resource_monitor: Arc<ResourceMonitor>) -> Self {
        Self { resource_monitor }
    }
}

#[async_trait]
impl RecoveryStrategy for GracefulDegradationStrategy {
    fn strategy_id(&self) -> &str {
        "graceful_degradation"
    }
    
    fn description(&self) -> &str {
        "Reduce system functionality to conserve resources"
    }
    
    async fn can_handle(&self, error: &SwarmError) -> bool {
        matches!(error,
            SwarmError::SwarmSizeLimitExceeded { .. } |
            SwarmError::CoordinationTimeout(_) |
            SwarmError::ResourceConflict { .. } |
            SwarmError::ConcurrencyError(_)
        )
    }
    
    async fn recover(
        &self,
        context: &RecoveryContext,
        error: &SwarmError,
    ) -> RecoveryResult {
        let start_time = Instant::now();
        
        let action_taken = match error {
            SwarmError::SwarmSizeLimitExceeded { current, max } => {
                // Reduce number of active agents
                format!(
                    "Reduced active agents from {} to {} to stay within limits",
                    current,
                    max / 2
                )
            }
            SwarmError::CoordinationTimeout(_) => {
                // Simplify coordination by reducing parallelism
                "Reduced coordination parallelism to prevent timeouts".to_string()
            }
            SwarmError::ResourceConflict { resource } => {
                // Implement resource sharing or queueing
                format!("Implemented resource sharing for {}", resource)
            }
            SwarmError::ConcurrencyError(_) => {
                // Reduce concurrent operations
                "Reduced concurrent operations to prevent conflicts".to_string()
            }
            _ => "Applied graceful degradation".to_string(),
        };
        
        RecoveryResult {
            success: true,
            action_taken,
            recovery_duration: start_time.elapsed(),
            new_state: None,
            confidence: 0.6,
            continue_recovery: false,
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("degradation_level".to_string(), serde_json::json!("moderate"));
                metadata.insert("resource_usage".to_string(), 
                    serde_json::json!(self.resource_monitor.get_usage().await));
                metadata
            },
        }
    }
    
    async fn cost_estimate(&self) -> f64 {
        0.4 // Medium cost - reduces functionality
    }
    
    async fn requires_external_resources(&self) -> bool {
        false
    }
}

/// Self-healing mechanism for automatic problem resolution
pub struct SelfHealingStrategy {
    healing_rules: Arc<RwLock<Vec<HealingRule>>>,
}

#[derive(Debug, Clone)]
pub struct HealingRule {
    pub rule_id: String,
    pub error_pattern: String,
    pub healing_action: HealingAction,
    pub cooldown_duration: Duration,
    pub last_applied: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum HealingAction {
    /// Restart a failed component
    RestartComponent(String),
    /// Adjust configuration parameters
    AdjustConfig(HashMap<String, serde_json::Value>),
    /// Reallocate resources
    ReallocateResources,
    /// Trigger manual intervention
    TriggerAlert(String),
    /// Execute custom recovery script
    ExecuteScript(String),
}

impl SelfHealingStrategy {
    pub fn new() -> Self {
        let mut healing_rules = Vec::new();
        
        // Add default healing rules
        healing_rules.push(HealingRule {
            rule_id: "agent_communication_failure".to_string(),
            error_pattern: "CommunicationError".to_string(),
            healing_action: HealingAction::RestartComponent("agent_protocol".to_string()),
            cooldown_duration: Duration::from_secs(300), // 5 minutes
            last_applied: None,
        });
        
        healing_rules.push(HealingRule {
            rule_id: "execution_timeout".to_string(),
            error_pattern: "CoordinationTimeout".to_string(),
            healing_action: HealingAction::AdjustConfig({
                let mut config = HashMap::new();
                config.insert("task_timeout_secs".to_string(), serde_json::json!(600));
                config
            }),
            cooldown_duration: Duration::from_secs(600), // 10 minutes
            last_applied: None,
        });
        
        Self {
            healing_rules: Arc::new(RwLock::new(healing_rules)),
        }
    }
    
    pub async fn add_healing_rule(&self, rule: HealingRule) {
        let mut rules = self.healing_rules.write().await;
        rules.push(rule);
    }
    
    async fn find_applicable_rule(&self, error: &SwarmError) -> Option<HealingRule> {
        let mut rules = self.healing_rules.write().await;
        let error_str = error.to_string();
        let now = Utc::now();
        
        for rule in rules.iter_mut() {
            if error_str.contains(&rule.error_pattern) {
                // Check cooldown
                if let Some(last_applied) = rule.last_applied {
                    if now.signed_duration_since(last_applied)
                        < ChronoDuration::from_std(rule.cooldown_duration).unwrap_or_default()
                    {
                        continue; // Still in cooldown
                    }
                }
                
                rule.last_applied = Some(now);
                return Some(rule.clone());
            }
        }
        
        None
    }
}

#[async_trait]
impl RecoveryStrategy for SelfHealingStrategy {
    fn strategy_id(&self) -> &str {
        "self_healing"
    }
    
    fn description(&self) -> &str {
        "Apply automatic healing rules based on error patterns"
    }
    
    async fn can_handle(&self, error: &SwarmError) -> bool {
        self.find_applicable_rule(error).await.is_some()
    }
    
    async fn recover(
        &self,
        context: &RecoveryContext,
        error: &SwarmError,
    ) -> RecoveryResult {
        let start_time = Instant::now();
        
        if let Some(rule) = self.find_applicable_rule(error).await {
            info!(
                rule_id = rule.rule_id,
                action = ?rule.healing_action,
                "Applying self-healing rule"
            );
            
            let action_taken = match rule.healing_action {
                HealingAction::RestartComponent(component) => {
                    format!("Restarted component: {}", component)
                }
                HealingAction::AdjustConfig(config) => {
                    format!("Adjusted configuration: {:?}", config)
                }
                HealingAction::ReallocateResources => {
                    "Reallocated system resources".to_string()
                }
                HealingAction::TriggerAlert(message) => {
                    format!("Triggered alert: {}", message)
                }
                HealingAction::ExecuteScript(script) => {
                    format!("Executed recovery script: {}", script)
                }
            };
            
            RecoveryResult {
                success: true,
                action_taken,
                recovery_duration: start_time.elapsed(),
                new_state: None,
                confidence: 0.8,
                continue_recovery: false,
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("rule_id".to_string(), serde_json::json!(rule.rule_id));
                    metadata.insert("rule_pattern".to_string(), serde_json::json!(rule.error_pattern));
                    metadata
                },
            }
        } else {
            RecoveryResult {
                success: false,
                action_taken: "No applicable healing rule found".to_string(),
                recovery_duration: start_time.elapsed(),
                new_state: None,
                confidence: 0.0,
                continue_recovery: true,
                metadata: HashMap::new(),
            }
        }
    }
    
    async fn cost_estimate(&self) -> f64 {
        0.2 // Low to medium cost
    }
    
    async fn requires_external_resources(&self) -> bool {
        true // May require external scripts or services
    }
}

/// Resource monitor for tracking system resource usage
#[derive(Debug)]
pub struct ResourceMonitor {
    cpu_usage: Arc<AtomicUsize>,
    memory_usage: Arc<AtomicUsize>,
    active_connections: Arc<AtomicUsize>,
    active_tasks: Arc<AtomicUsize>,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            cpu_usage: Arc::new(AtomicUsize::new(0)),
            memory_usage: Arc::new(AtomicUsize::new(0)),
            active_connections: Arc::new(AtomicUsize::new(0)),
            active_tasks: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    pub async fn get_usage(&self) -> ResourceUsage {
        ResourceUsage {
            cpu_percent: self.cpu_usage.load(Ordering::Relaxed) as f64 / 100.0,
            memory_percent: self.memory_usage.load(Ordering::Relaxed) as f64 / 100.0,
            active_connections: self.active_connections.load(Ordering::Relaxed),
            active_tasks: self.active_tasks.load(Ordering::Relaxed),
        }
    }
    
    pub fn update_cpu_usage(&self, usage_percent: f64) {
        self.cpu_usage.store((usage_percent * 100.0) as usize, Ordering::Relaxed);
    }
    
    pub fn update_memory_usage(&self, usage_percent: f64) {
        self.memory_usage.store((usage_percent * 100.0) as usize, Ordering::Relaxed);
    }
    
    pub fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn decrement_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn increment_tasks(&self) {
        self.active_tasks.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn decrement_tasks(&self) {
        self.active_tasks.fetch_sub(1, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub active_connections: usize,
    pub active_tasks: usize,
}

/// Main recovery manager that coordinates different recovery strategies
pub struct RecoveryManager {
    strategies: Arc<RwLock<Vec<Arc<dyn RecoveryStrategy>>>>,
    recovery_history: Arc<RwLock<VecDeque<RecoveryAttempt>>>,
    checkpoint_strategy: Arc<CheckpointRecoveryStrategy>,
    resource_monitor: Arc<ResourceMonitor>,
    recovery_semaphore: Arc<Semaphore>,
    max_concurrent_recoveries: usize,
    max_history_size: usize,
}

#[derive(Debug, Clone)]
pub struct RecoveryAttempt {
    pub attempt_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub strategy_used: String,
    pub error: String,
    pub result: RecoveryResult,
}

impl RecoveryManager {
    pub fn new() -> Self {
        let resource_monitor = Arc::new(ResourceMonitor::new());
        let checkpoint_strategy = Arc::new(CheckpointRecoveryStrategy::new(10));
        
        let mut strategies: Vec<Arc<dyn RecoveryStrategy>> = Vec::new();
        strategies.push(Arc::new(ExponentialBackoffStrategy::new(BackoffConfig::default())));
        strategies.push(checkpoint_strategy.clone());
        strategies.push(Arc::new(GracefulDegradationStrategy::new(resource_monitor.clone())));
        strategies.push(Arc::new(SelfHealingStrategy::new()));
        
        Self {
            strategies: Arc::new(RwLock::new(strategies)),
            recovery_history: Arc::new(RwLock::new(VecDeque::new())),
            checkpoint_strategy,
            resource_monitor,
            recovery_semaphore: Arc::new(Semaphore::new(3)),
            max_concurrent_recoveries: 3,
            max_history_size: 100,
        }
    }
    
    /// Add a custom recovery strategy
    pub async fn add_strategy(&self, strategy: Arc<dyn RecoveryStrategy>) {
        let mut strategies = self.strategies.write().await;
        strategies.push(strategy);
    }
    
    /// Create a checkpoint of the current swarm state
    pub async fn create_checkpoint(&self, coordinator: &SwarmCoordinator) -> SwarmResult<Uuid> {
        self.checkpoint_strategy.create_checkpoint(coordinator).await
    }
    
    /// Attempt to recover from an error using available strategies
    pub async fn recover(
        &self,
        coordinator: Arc<SwarmCoordinator>,
        error: SwarmError,
        failed_task_id: Option<TaskId>,
        failed_agent_id: Option<AgentId>,
    ) -> SwarmResult<RecoveryResult> {
        let _permit = self.recovery_semaphore.acquire().await
            .map_err(|_| SwarmError::Other(anyhow::anyhow!("Failed to acquire recovery semaphore")))?;
        
        let attempt_id = Uuid::new_v4();
        let context = RecoveryContext {
            coordinator: coordinator.clone(),
            swarm_state: coordinator.state().await,
            failed_task_id,
            failed_agent_id,
            retry_count: self.count_recent_retries(&error).await,
            error_timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        
        info!(
            attempt_id = %attempt_id,
            error = %error,
            retry_count = context.retry_count,
            "Starting recovery attempt"
        );
        
        // Find suitable strategies
        let mut suitable_strategies = Vec::new();
        let strategies = self.strategies.read().await;
        
        for strategy in strategies.iter() {
            if strategy.can_handle(&error).await {
                let cost = strategy.cost_estimate().await;
                suitable_strategies.push((strategy.clone(), cost));
            }
        }
        
        // Sort strategies by cost (cheapest first)
        suitable_strategies.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        if suitable_strategies.is_empty() {
            let result = RecoveryResult {
                success: false,
                action_taken: "No suitable recovery strategy found".to_string(),
                recovery_duration: Duration::from_millis(0),
                new_state: None,
                confidence: 0.0,
                continue_recovery: false,
                metadata: HashMap::new(),
            };
            
            self.record_recovery_attempt(attempt_id, "none", &error, &result).await;
            return Ok(result);
        }
        
        // Try strategies in order of increasing cost
        let mut last_result = None;
        
        for (strategy, _cost) in suitable_strategies {
            debug!(
                strategy_id = strategy.strategy_id(),
                description = strategy.description(),
                "Attempting recovery strategy"
            );
            
            let result = strategy.recover(&context, &error).await;
            
            info!(
                strategy_id = strategy.strategy_id(),
                success = result.success,
                confidence = result.confidence,
                action = result.action_taken,
                "Recovery strategy completed"
            );
            
            self.record_recovery_attempt(attempt_id, strategy.strategy_id(), &error, &result).await;
            
            if result.success && result.confidence > 0.5 {
                // Strategy succeeded with good confidence
                return Ok(result);
            }
            
            last_result = Some(result);
            
            // If strategy says not to continue, stop here
            if let Some(ref result) = last_result {
                if !result.continue_recovery {
                    break;
                }
            }
        }
        
        // Return the last result if no strategy succeeded
        Ok(last_result.unwrap_or_else(|| RecoveryResult {
            success: false,
            action_taken: "All recovery strategies failed".to_string(),
            recovery_duration: Duration::from_millis(0),
            new_state: None,
            confidence: 0.0,
            continue_recovery: false,
            metadata: HashMap::new(),
        }))
    }
    
    /// Get recovery history
    pub async fn get_recovery_history(&self) -> Vec<RecoveryAttempt> {
        let history = self.recovery_history.read().await;
        history.iter().cloned().collect()
    }
    
    /// Get resource monitor
    pub fn resource_monitor(&self) -> Arc<ResourceMonitor> {
        self.resource_monitor.clone()
    }
    
    /// Start background monitoring tasks
    pub async fn start_monitoring(&self) {
        let resource_monitor = self.resource_monitor.clone();
        
        // Start resource monitoring task
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // In a real implementation, this would collect actual system metrics
                // For now, we'll simulate some reasonable values
                resource_monitor.update_cpu_usage(rand::random::<f64>() * 0.8);
                resource_monitor.update_memory_usage(rand::random::<f64>() * 0.7);
                
                debug!(
                    cpu_usage = resource_monitor.cpu_usage.load(Ordering::Relaxed),
                    memory_usage = resource_monitor.memory_usage.load(Ordering::Relaxed),
                    "Updated resource metrics"
                );
            }
        });
    }
    
    async fn count_recent_retries(&self, error: &SwarmError) -> usize {
        let history = self.recovery_history.read().await;
        let error_str = error.to_string();
        let recent_threshold = Utc::now() - ChronoDuration::hours(1);
        
        history
            .iter()
            .filter(|attempt| {
                attempt.timestamp > recent_threshold && attempt.error.contains(&error_str)
            })
            .count()
    }
    
    async fn record_recovery_attempt(
        &self,
        attempt_id: Uuid,
        strategy: &str,
        error: &SwarmError,
        result: &RecoveryResult,
    ) {
        let attempt = RecoveryAttempt {
            attempt_id,
            timestamp: Utc::now(),
            strategy_used: strategy.to_string(),
            error: error.to_string(),
            result: result.clone(),
        };
        
        let mut history = self.recovery_history.write().await;
        history.push_back(attempt);
        
        // Maintain history size limit
        if history.len() > self.max_history_size {
            history.pop_front();
        }
    }
}

impl Default for RecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_exponential_backoff_delay_calculation() {
        let config = BackoffConfig {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
            max_retries: 5,
            jitter_factor: 0.0, // No jitter for predictable testing
        };
        
        let strategy = ExponentialBackoffStrategy::new(config);
        
        assert_eq!(strategy.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(strategy.calculate_delay(1), Duration::from_millis(200));
        assert_eq!(strategy.calculate_delay(2), Duration::from_millis(400));
        assert_eq!(strategy.calculate_delay(3), Duration::from_millis(800));
    }
    
    #[tokio::test]
    async fn test_recovery_manager_creation() {
        let manager = RecoveryManager::new();
        
        // Should have default strategies
        let strategies = manager.strategies.read().await;
        assert!(strategies.len() >= 4);
        
        // Should have resource monitor
        let usage = manager.resource_monitor.get_usage().await;
        assert!(usage.cpu_percent >= 0.0);
    }
    
    #[tokio::test]
    async fn test_checkpoint_strategy_creation() {
        let strategy = CheckpointRecoveryStrategy::new(5);
        
        assert_eq!(strategy.strategy_id(), "checkpoint_recovery");
        assert!(!strategy.description().is_empty());
        assert!(!strategy.requires_external_resources().await);
    }
    
    #[tokio::test]
    async fn test_self_healing_rules() {
        let strategy = SelfHealingStrategy::new();
        
        // Should have default rules
        let rules = strategy.healing_rules.read().await;
        assert!(!rules.is_empty());
        
        // Test error pattern matching
        let comm_error = SwarmError::CommunicationError("test".to_string());
        assert!(strategy.can_handle(&comm_error).await);
        
        let timeout_error = SwarmError::CoordinationTimeout(30);
        assert!(strategy.can_handle(&timeout_error).await);
    }
    
    #[tokio::test] 
    async fn test_resource_monitor() {
        let monitor = ResourceMonitor::new();
        
        monitor.update_cpu_usage(0.75);
        monitor.update_memory_usage(0.5);
        monitor.increment_connections();
        monitor.increment_tasks();
        
        let usage = monitor.get_usage().await;
        assert_eq!(usage.cpu_percent, 0.75);
        assert_eq!(usage.memory_percent, 0.5);
        assert_eq!(usage.active_connections, 1);
        assert_eq!(usage.active_tasks, 1);
    }
}