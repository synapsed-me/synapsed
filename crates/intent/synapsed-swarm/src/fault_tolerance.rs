//! Fault tolerance mechanisms for swarm coordination
//!
//! This module provides comprehensive fault tolerance for agent swarms:
//! - Heartbeat-based agent failure detection
//! - Circuit breaker pattern for failing agents
//! - Automatic agent restart and recovery
//! - Task redistribution when agents fail
//! - Checkpoint and rollback mechanisms

use crate::{
    error::{SwarmError, SwarmResult},
    types::*,
    claude_agent::ClaudeAgent,
    trust::TrustManager,
    execution::ExecutionEngine,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{
    sync::{RwLock, Notify, watch},
    time::{interval, sleep, timeout},
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use synapsed_intent::HierarchicalIntent;
use synapsed_promise::{AutonomousAgent, Promise};

/// Configuration for fault tolerance mechanisms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultToleranceConfig {
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
    /// Agent timeout threshold in milliseconds
    pub agent_timeout_ms: u64,
    /// Maximum number of failures before circuit breaker opens
    pub circuit_breaker_failure_threshold: u32,
    /// Circuit breaker timeout in milliseconds
    pub circuit_breaker_timeout_ms: u64,
    /// Maximum restart attempts per agent
    pub max_restart_attempts: u32,
    /// Delay between restart attempts in milliseconds
    pub restart_delay_ms: u64,
    /// Task redistribution delay in milliseconds
    pub task_redistribution_delay_ms: u64,
    /// Checkpoint interval in milliseconds
    pub checkpoint_interval_ms: u64,
    /// Maximum number of checkpoints to keep
    pub max_checkpoints: usize,
    /// Enable automatic recovery
    pub enable_auto_recovery: bool,
    /// Enable task redistribution
    pub enable_task_redistribution: bool,
    /// Maximum time to wait for recovery confirmation in milliseconds
    pub recovery_confirmation_timeout_ms: u64,
}

impl Default for FaultToleranceConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: 5000,     // 5 seconds
            agent_timeout_ms: 15000,         // 15 seconds
            circuit_breaker_failure_threshold: 5,
            circuit_breaker_timeout_ms: 60000, // 1 minute
            max_restart_attempts: 3,
            restart_delay_ms: 10000,         // 10 seconds
            task_redistribution_delay_ms: 5000, // 5 seconds
            checkpoint_interval_ms: 30000,   // 30 seconds
            max_checkpoints: 10,
            enable_auto_recovery: true,
            enable_task_redistribution: true,
            recovery_confirmation_timeout_ms: 30000, // 30 seconds
        }
    }
}

/// Agent health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentHealthStatus {
    /// Agent is healthy and responsive
    Healthy,
    /// Agent is unresponsive but not yet failed
    Unresponsive,
    /// Agent has failed
    Failed,
    /// Agent is recovering
    Recovering,
    /// Agent is in circuit breaker open state
    CircuitOpen,
}

/// Agent heartbeat information
#[derive(Debug, Clone)]
pub struct AgentHeartbeat {
    /// Agent ID
    pub agent_id: AgentId,
    /// Last heartbeat timestamp
    pub last_heartbeat: Instant,
    /// Health status
    pub health_status: AgentHealthStatus,
    /// Number of consecutive missed heartbeats
    pub missed_heartbeats: u32,
    /// Current task being executed (if any)
    pub current_task: Option<TaskId>,
    /// Agent performance metrics
    pub performance_metrics: AgentPerformanceMetrics,
}

/// Performance metrics for an agent
#[derive(Debug, Clone, Default)]
pub struct AgentPerformanceMetrics {
    /// Number of tasks completed
    pub tasks_completed: u64,
    /// Number of tasks failed
    pub tasks_failed: u64,
    /// Average task execution time in milliseconds
    pub avg_execution_time_ms: f64,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Last performance update
    pub last_updated: Option<Instant>,
}

/// Circuit breaker state for an agent
#[derive(Debug, Clone)]
pub struct CircuitBreakerState {
    /// Current state
    pub state: CircuitBreakerStatus,
    /// Number of failures in current window
    pub failure_count: u32,
    /// Last failure timestamp
    pub last_failure: Option<Instant>,
    /// Last state change timestamp
    pub last_state_change: Instant,
    /// Number of requests since last reset
    pub request_count: u64,
}

/// Circuit breaker status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerStatus {
    /// Circuit is closed (normal operation)
    Closed,
    /// Circuit is open (blocking requests)
    Open,
    /// Circuit is half-open (testing recovery)
    HalfOpen,
}

/// Task checkpoint for recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCheckpoint {
    /// Checkpoint ID
    pub checkpoint_id: Uuid,
    /// Task ID
    pub task_id: TaskId,
    /// Agent ID that was executing
    pub agent_id: AgentId,
    /// Checkpoint timestamp
    pub timestamp: DateTime<Utc>,
    /// Task state at checkpoint
    pub task_state: TaskState,
    /// Execution progress
    pub progress: TaskProgress,
    /// Context snapshot
    pub context_snapshot: HashMap<String, serde_json::Value>,
}

/// Task state for checkpointing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    /// Current step being executed
    pub current_step: usize,
    /// Completed steps
    pub completed_steps: Vec<StepResult>,
    /// Remaining steps
    pub remaining_steps: Vec<IntentStep>,
    /// Task metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Task execution progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    /// Progress percentage (0.0 to 1.0)
    pub percentage: f64,
    /// Number of completed steps
    pub completed_steps: usize,
    /// Total number of steps
    pub total_steps: usize,
    /// Estimated time remaining in milliseconds
    pub estimated_remaining_ms: Option<u64>,
}

/// Step result for checkpointing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step index
    pub step_index: usize,
    /// Whether step succeeded
    pub success: bool,
    /// Step output
    pub output: Option<serde_json::Value>,
    /// Step metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Execution time
    pub duration_ms: u64,
}

/// Intent step for checkpointing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentStep {
    /// Step description
    pub description: String,
    /// Step parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Dependencies
    pub dependencies: Vec<usize>,
}

/// Recovery action to be taken
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Restart the agent
    RestartAgent {
        agent_id: AgentId,
        attempt: u32,
    },
    /// Redistribute task to another agent
    RedistributeTask {
        task_id: TaskId,
        from_agent: AgentId,
        to_agent: Option<AgentId>,
    },
    /// Rollback task to last checkpoint
    RollbackTask {
        task_id: TaskId,
        checkpoint_id: Uuid,
    },
    /// Mark agent as failed
    MarkAgentFailed {
        agent_id: AgentId,
        reason: String,
    },
}

/// Main fault tolerance manager
pub struct FaultToleranceManager {
    /// Configuration
    config: Arc<FaultToleranceConfig>,
    /// Agent heartbeats
    heartbeats: Arc<DashMap<AgentId, AgentHeartbeat>>,
    /// Circuit breaker states
    circuit_breakers: Arc<DashMap<AgentId, CircuitBreakerState>>,
    /// Task checkpoints
    checkpoints: Arc<DashMap<TaskId, VecDeque<TaskCheckpoint>>>,
    /// Recovery actions queue
    recovery_queue: Arc<RwLock<VecDeque<RecoveryAction>>>,
    /// Active agents
    agents: Arc<DashMap<AgentId, Arc<AutonomousAgent>>>,
    /// Trust manager reference
    trust_manager: Arc<TrustManager>,
    /// Execution engine reference
    execution_engine: Arc<ExecutionEngine>,
    /// Running state
    is_running: Arc<AtomicBool>,
    /// Shutdown notification
    shutdown_notify: Arc<Notify>,
    /// Recovery statistics
    recovery_stats: Arc<RwLock<RecoveryStatistics>>,
}

/// Recovery operation statistics
#[derive(Debug, Clone, Default)]
pub struct RecoveryStatistics {
    /// Total recovery attempts
    pub total_recovery_attempts: u64,
    /// Successful recoveries
    pub successful_recoveries: u64,
    /// Failed recoveries
    pub failed_recoveries: u64,
    /// Agent restarts
    pub agent_restarts: u64,
    /// Task redistributions
    pub task_redistributions: u64,
    /// Task rollbacks
    pub task_rollbacks: u64,
    /// Last recovery timestamp
    pub last_recovery: Option<DateTime<Utc>>,
}

impl FaultToleranceManager {
    /// Create a new fault tolerance manager
    pub fn new(
        config: FaultToleranceConfig,
        trust_manager: Arc<TrustManager>,
        execution_engine: Arc<ExecutionEngine>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            heartbeats: Arc::new(DashMap::new()),
            circuit_breakers: Arc::new(DashMap::new()),
            checkpoints: Arc::new(DashMap::new()),
            recovery_queue: Arc::new(RwLock::new(VecDeque::new())),
            agents: Arc::new(DashMap::new()),
            trust_manager,
            execution_engine,
            is_running: Arc::new(AtomicBool::new(false)),
            shutdown_notify: Arc::new(Notify::new()),
            recovery_stats: Arc::new(RwLock::new(RecoveryStatistics::default())),
        }
    }

    /// Start the fault tolerance system
    pub async fn start(&self) -> SwarmResult<()> {
        info!("Starting fault tolerance manager");
        
        self.is_running.store(true, Ordering::SeqCst);
        
        // Start heartbeat monitoring
        let heartbeat_task = {
            let manager = self.clone_for_task();
            tokio::spawn(async move {
                manager.heartbeat_monitor().await;
            })
        };

        // Start recovery processor
        let recovery_task = {
            let manager = self.clone_for_task();
            tokio::spawn(async move {
                manager.recovery_processor().await;
            })
        };

        // Start checkpoint manager
        let checkpoint_task = {
            let manager = self.clone_for_task();
            tokio::spawn(async move {
                manager.checkpoint_manager().await;
            })
        };

        info!("Fault tolerance manager started successfully");
        Ok(())
    }

    /// Stop the fault tolerance system
    pub async fn stop(&self) -> SwarmResult<()> {
        info!("Stopping fault tolerance manager");
        
        self.is_running.store(false, Ordering::SeqCst);
        self.shutdown_notify.notify_waiters();
        
        info!("Fault tolerance manager stopped");
        Ok(())
    }

    /// Register an agent for monitoring
    pub async fn register_agent(&self, agent: Arc<AutonomousAgent>) -> SwarmResult<()> {
        let agent_id = agent.id();
        
        // Store agent reference
        self.agents.insert(agent_id, agent);
        
        // Initialize heartbeat tracking
        let heartbeat = AgentHeartbeat {
            agent_id,
            last_heartbeat: Instant::now(),
            health_status: AgentHealthStatus::Healthy,
            missed_heartbeats: 0,
            current_task: None,
            performance_metrics: AgentPerformanceMetrics::default(),
        };
        self.heartbeats.insert(agent_id, heartbeat);
        
        // Initialize circuit breaker
        let circuit_breaker = CircuitBreakerState {
            state: CircuitBreakerStatus::Closed,
            failure_count: 0,
            last_failure: None,
            last_state_change: Instant::now(),
            request_count: 0,
        };
        self.circuit_breakers.insert(agent_id, circuit_breaker);
        
        info!("Registered agent {} for fault tolerance monitoring", agent_id);
        Ok(())
    }

    /// Unregister an agent from monitoring
    pub async fn unregister_agent(&self, agent_id: AgentId) -> SwarmResult<()> {
        self.agents.remove(&agent_id);
        self.heartbeats.remove(&agent_id);
        self.circuit_breakers.remove(&agent_id);
        
        // Clean up checkpoints for this agent's tasks
        let mut tasks_to_clean = Vec::new();
        for entry in self.checkpoints.iter() {
            let task_id = *entry.key();
            if let Some(latest_checkpoint) = entry.value().back() {
                if latest_checkpoint.agent_id == agent_id {
                    tasks_to_clean.push(task_id);
                }
            }
        }
        
        for task_id in tasks_to_clean {
            self.checkpoints.remove(&task_id);
        }
        
        info!("Unregistered agent {} from fault tolerance monitoring", agent_id);
        Ok(())
    }

    /// Record agent heartbeat
    pub async fn record_heartbeat(&self, agent_id: AgentId, current_task: Option<TaskId>) -> SwarmResult<()> {
        if let Some(mut heartbeat) = self.heartbeats.get_mut(&agent_id) {
            heartbeat.last_heartbeat = Instant::now();
            heartbeat.missed_heartbeats = 0;
            heartbeat.current_task = current_task;
            
            // Update health status if it was degraded
            if heartbeat.health_status != AgentHealthStatus::Healthy {
                heartbeat.health_status = AgentHealthStatus::Healthy;
                info!("Agent {} recovered and is now healthy", agent_id);
            }
        }
        
        Ok(())
    }

    /// Check if an agent can handle a task (circuit breaker check)
    pub async fn can_handle_task(&self, agent_id: AgentId) -> bool {
        if let Some(circuit_breaker) = self.circuit_breakers.get(&agent_id) {
            match circuit_breaker.state {
                CircuitBreakerStatus::Closed => true,
                CircuitBreakerStatus::Open => {
                    // Check if timeout has elapsed
                    let timeout_duration = Duration::from_millis(self.config.circuit_breaker_timeout_ms);
                    if circuit_breaker.last_state_change.elapsed() > timeout_duration {
                        // Try to move to half-open state
                        drop(circuit_breaker);
                        self.transition_circuit_breaker(agent_id, CircuitBreakerStatus::HalfOpen).await;
                        true
                    } else {
                        false
                    }
                }
                CircuitBreakerStatus::HalfOpen => true,
            }
        } else {
            false
        }
    }

    /// Record task execution result for circuit breaker
    pub async fn record_task_result(&self, agent_id: AgentId, success: bool, duration_ms: u64) -> SwarmResult<()> {
        // Update circuit breaker
        if let Some(mut circuit_breaker) = self.circuit_breakers.get_mut(&agent_id) {
            circuit_breaker.request_count += 1;
            
            if success {
                // Success - reset failure count if in half-open state
                if circuit_breaker.state == CircuitBreakerStatus::HalfOpen {
                    circuit_breaker.failure_count = 0;
                    drop(circuit_breaker);
                    self.transition_circuit_breaker(agent_id, CircuitBreakerStatus::Closed).await;
                }
            } else {
                // Failure - increment failure count
                circuit_breaker.failure_count += 1;
                circuit_breaker.last_failure = Some(Instant::now());
                
                // Check if we need to open the circuit
                if circuit_breaker.failure_count >= self.config.circuit_breaker_failure_threshold &&
                   circuit_breaker.state != CircuitBreakerStatus::Open {
                    drop(circuit_breaker);
                    self.transition_circuit_breaker(agent_id, CircuitBreakerStatus::Open).await;
                }
            }
        }
        
        // Update performance metrics
        if let Some(mut heartbeat) = self.heartbeats.get_mut(&agent_id) {
            let metrics = &mut heartbeat.performance_metrics;
            if success {
                metrics.tasks_completed += 1;
            } else {
                metrics.tasks_failed += 1;
            }
            
            // Update average execution time
            let total_tasks = metrics.tasks_completed + metrics.tasks_failed;
            if total_tasks > 0 {
                let current_avg = metrics.avg_execution_time_ms;
                metrics.avg_execution_time_ms = 
                    (current_avg * (total_tasks - 1) as f64 + duration_ms as f64) / total_tasks as f64;
                
                // Update success rate
                metrics.success_rate = metrics.tasks_completed as f64 / total_tasks as f64;
            }
            
            metrics.last_updated = Some(Instant::now());
        }
        
        Ok(())
    }

    /// Create a task checkpoint
    pub async fn create_checkpoint(
        &self,
        task_id: TaskId,
        agent_id: AgentId,
        task_state: TaskState,
        progress: TaskProgress,
        context: HashMap<String, serde_json::Value>,
    ) -> SwarmResult<Uuid> {
        let checkpoint_id = Uuid::new_v4();
        
        let checkpoint = TaskCheckpoint {
            checkpoint_id,
            task_id,
            agent_id,
            timestamp: Utc::now(),
            task_state,
            progress,
            context_snapshot: context,
        };
        
        // Add checkpoint to the task's checkpoint history
        let mut checkpoints = self.checkpoints.entry(task_id).or_insert_with(VecDeque::new);
        
        // Remove old checkpoints if we exceed the limit
        while checkpoints.len() >= self.config.max_checkpoints {
            checkpoints.pop_front();
        }
        
        checkpoints.push_back(checkpoint);
        
        debug!("Created checkpoint {} for task {} on agent {}", 
               checkpoint_id, task_id, agent_id);
        
        Ok(checkpoint_id)
    }

    /// Get the latest checkpoint for a task
    pub async fn get_latest_checkpoint(&self, task_id: TaskId) -> Option<TaskCheckpoint> {
        self.checkpoints.get(&task_id)?.back().cloned()
    }

    /// Handle agent failure
    pub async fn handle_agent_failure(&self, agent_id: AgentId, reason: String) -> SwarmResult<()> {
        warn!("Handling failure for agent {}: {}", agent_id, reason);
        
        // Update agent health status
        if let Some(mut heartbeat) = self.heartbeats.get_mut(&agent_id) {
            heartbeat.health_status = AgentHealthStatus::Failed;
        }
        
        // Open circuit breaker
        self.transition_circuit_breaker(agent_id, CircuitBreakerStatus::Open).await;
        
        // Find tasks assigned to this agent and redistribute them
        let current_task = self.heartbeats.get(&agent_id)
            .and_then(|hb| hb.current_task);
        
        if let Some(task_id) = current_task {
            self.queue_recovery_action(RecoveryAction::RedistributeTask {
                task_id,
                from_agent: agent_id,
                to_agent: None,
            }).await;
        }
        
        // Queue agent restart if auto-recovery is enabled
        if self.config.enable_auto_recovery {
            self.queue_recovery_action(RecoveryAction::RestartAgent {
                agent_id,
                attempt: 1,
            }).await;
        } else {
            self.queue_recovery_action(RecoveryAction::MarkAgentFailed {
                agent_id,
                reason: reason.clone(),
            }).await;
        }
        
        // Update trust score
        self.trust_manager.record_failure(agent_id).await?;
        
        info!("Queued recovery actions for failed agent {}", agent_id);
        Ok(())
    }

    /// Queue a recovery action
    async fn queue_recovery_action(&self, action: RecoveryAction) {
        let mut queue = self.recovery_queue.write().await;
        queue.push_back(action);
        debug!("Queued recovery action");
    }

    /// Heartbeat monitoring loop
    async fn heartbeat_monitor(&self) {
        let mut interval = interval(Duration::from_millis(self.config.heartbeat_interval_ms));
        
        while self.is_running.load(Ordering::SeqCst) {
            tokio::select! {
                _ = interval.tick() => {
                    self.check_agent_heartbeats().await;
                }
                _ = self.shutdown_notify.notified() => {
                    break;
                }
            }
        }
        
        debug!("Heartbeat monitor stopped");
    }

    /// Check all agent heartbeats
    async fn check_agent_heartbeats(&self) {
        let timeout_threshold = Duration::from_millis(self.config.agent_timeout_ms);
        let now = Instant::now();
        
        for mut entry in self.heartbeats.iter_mut() {
            let agent_id = *entry.key();
            let heartbeat = entry.value_mut();
            
            let elapsed = now.duration_since(heartbeat.last_heartbeat);
            
            if elapsed > timeout_threshold {
                heartbeat.missed_heartbeats += 1;
                
                match heartbeat.health_status {
                    AgentHealthStatus::Healthy => {
                        heartbeat.health_status = AgentHealthStatus::Unresponsive;
                        warn!("Agent {} is unresponsive", agent_id);
                    }
                    AgentHealthStatus::Unresponsive => {
                        if heartbeat.missed_heartbeats >= 3 {
                            heartbeat.health_status = AgentHealthStatus::Failed;
                            error!("Agent {} has failed (too many missed heartbeats)", agent_id);
                            
                            // Handle failure asynchronously
                            let manager = self.clone_for_task();
                            tokio::spawn(async move {
                                if let Err(e) = manager.handle_agent_failure(
                                    agent_id, 
                                    "Too many missed heartbeats".to_string()
                                ).await {
                                    error!("Failed to handle agent failure: {}", e);
                                }
                            });
                        }
                    }
                    _ => {} // Already handling or failed
                }
            }
        }
    }

    /// Recovery processor loop
    async fn recovery_processor(&self) {
        let mut interval = interval(Duration::from_millis(1000)); // Check every second
        
        while self.is_running.load(Ordering::SeqCst) {
            tokio::select! {
                _ = interval.tick() => {
                    self.process_recovery_actions().await;
                }
                _ = self.shutdown_notify.notified() => {
                    break;
                }
            }
        }
        
        debug!("Recovery processor stopped");
    }

    /// Process recovery actions from the queue
    async fn process_recovery_actions(&self) {
        let mut actions = Vec::new();
        
        // Collect all pending actions
        {
            let mut queue = self.recovery_queue.write().await;
            while let Some(action) = queue.pop_front() {
                actions.push(action);
            }
        }
        
        // Process each action
        for action in actions {
            if let Err(e) = self.execute_recovery_action(action).await {
                error!("Recovery action failed: {}", e);
                
                let mut stats = self.recovery_stats.write().await;
                stats.failed_recoveries += 1;
            } else {
                let mut stats = self.recovery_stats.write().await;
                stats.successful_recoveries += 1;
                stats.last_recovery = Some(Utc::now());
            }
        }
    }

    /// Execute a specific recovery action
    async fn execute_recovery_action(&self, action: RecoveryAction) -> SwarmResult<()> {
        let mut stats = self.recovery_stats.write().await;
        stats.total_recovery_attempts += 1;
        drop(stats);
        
        match action {
            RecoveryAction::RestartAgent { agent_id, attempt } => {
                self.restart_agent(agent_id, attempt).await?;
                let mut stats = self.recovery_stats.write().await;
                stats.agent_restarts += 1;
            }
            RecoveryAction::RedistributeTask { task_id, from_agent, to_agent } => {
                self.redistribute_task(task_id, from_agent, to_agent).await?;
                let mut stats = self.recovery_stats.write().await;
                stats.task_redistributions += 1;
            }
            RecoveryAction::RollbackTask { task_id, checkpoint_id } => {
                self.rollback_task(task_id, checkpoint_id).await?;
                let mut stats = self.recovery_stats.write().await;
                stats.task_rollbacks += 1;
            }
            RecoveryAction::MarkAgentFailed { agent_id, reason } => {
                self.mark_agent_failed(agent_id, reason).await?;
            }
        }
        
        Ok(())
    }

    /// Restart a failed agent
    async fn restart_agent(&self, agent_id: AgentId, attempt: u32) -> SwarmResult<()> {
        info!("Attempting to restart agent {} (attempt {})", agent_id, attempt);
        
        if attempt > self.config.max_restart_attempts {
            warn!("Max restart attempts exceeded for agent {}", agent_id);
            return self.mark_agent_failed(
                agent_id, 
                format!("Max restart attempts ({}) exceeded", self.config.max_restart_attempts)
            ).await;
        }
        
        // Wait before restart
        if attempt > 1 {
            sleep(Duration::from_millis(self.config.restart_delay_ms)).await;
        }
        
        // Get agent reference
        let agent = self.agents.get(&agent_id)
            .ok_or_else(|| SwarmError::AgentNotFound(agent_id))?
            .clone();
        
        // Try to reinitialize the agent
        match timeout(
            Duration::from_millis(self.config.recovery_confirmation_timeout_ms),
            agent.initialize()
        ).await {
            Ok(Ok(())) => {
                info!("Successfully restarted agent {}", agent_id);
                
                // Update health status
                if let Some(mut heartbeat) = self.heartbeats.get_mut(&agent_id) {
                    heartbeat.health_status = AgentHealthStatus::Recovering;
                    heartbeat.missed_heartbeats = 0;
                    heartbeat.last_heartbeat = Instant::now();
                }
                
                // Reset circuit breaker to half-open
                self.transition_circuit_breaker(agent_id, CircuitBreakerStatus::HalfOpen).await;
                
                Ok(())
            }
            Ok(Err(e)) => {
                warn!("Failed to restart agent {} (attempt {}): {}", agent_id, attempt, e);
                
                // Queue another restart attempt
                self.queue_recovery_action(RecoveryAction::RestartAgent {
                    agent_id,
                    attempt: attempt + 1,
                }).await;
                
                Err(SwarmError::Other(anyhow::anyhow!("Agent restart failed: {}", e)))
            }
            Err(_) => {
                warn!("Agent {} restart timed out (attempt {})", agent_id, attempt);
                
                // Queue another restart attempt
                self.queue_recovery_action(RecoveryAction::RestartAgent {
                    agent_id,
                    attempt: attempt + 1,
                }).await;
                
                Err(SwarmError::CoordinationTimeout(
                    self.config.recovery_confirmation_timeout_ms / 1000
                ))
            }
        }
    }

    /// Redistribute a task from a failed agent to a healthy one
    async fn redistribute_task(&self, task_id: TaskId, from_agent: AgentId, to_agent: Option<AgentId>) -> SwarmResult<()> {
        info!("Redistributing task {} from agent {} to {:?}", task_id, from_agent, to_agent);
        
        if !self.config.enable_task_redistribution {
            warn!("Task redistribution is disabled");
            return Ok(());
        }
        
        // Wait for redistribution delay
        sleep(Duration::from_millis(self.config.task_redistribution_delay_ms)).await;
        
        // Get the latest checkpoint for this task
        let checkpoint = self.get_latest_checkpoint(task_id).await;
        
        if let Some(checkpoint) = checkpoint {
            // Find a healthy agent to take over
            let target_agent = if let Some(agent_id) = to_agent {
                agent_id
            } else {
                self.find_healthy_agent().await
                    .ok_or_else(|| SwarmError::Other(anyhow::anyhow!("No healthy agent available for redistribution")))?
            };
            
            info!("Redistributing task {} to agent {}", task_id, target_agent);
            
            // Here we would normally interact with the coordinator to reassign the task
            // For now, we just log the action and queue a rollback if needed
            if let Err(e) = self.rollback_task(task_id, checkpoint.checkpoint_id).await {
                error!("Failed to rollback task during redistribution: {}", e);
            }
        } else {
            warn!("No checkpoint found for task {}, cannot redistribute", task_id);
        }
        
        Ok(())
    }

    /// Rollback a task to a previous checkpoint
    async fn rollback_task(&self, task_id: TaskId, checkpoint_id: Uuid) -> SwarmResult<()> {
        info!("Rolling back task {} to checkpoint {}", task_id, checkpoint_id);
        
        // Find the specific checkpoint
        let checkpoint = self.checkpoints.get(&task_id)
            .and_then(|checkpoints| {
                checkpoints.iter().find(|cp| cp.checkpoint_id == checkpoint_id).cloned()
            })
            .ok_or_else(|| SwarmError::Other(anyhow::anyhow!("Checkpoint not found")))?;
        
        // Here we would normally restore the task state and resume execution
        // This would involve coordinating with the execution engine
        debug!("Task {} rolled back to checkpoint at step {} ({}% complete)", 
               task_id, 
               checkpoint.task_state.current_step,
               (checkpoint.progress.percentage * 100.0) as u32);
        
        Ok(())
    }

    /// Mark an agent as permanently failed
    async fn mark_agent_failed(&self, agent_id: AgentId, reason: String) -> SwarmResult<()> {
        error!("Marking agent {} as permanently failed: {}", agent_id, reason);
        
        // Update health status
        if let Some(mut heartbeat) = self.heartbeats.get_mut(&agent_id) {
            heartbeat.health_status = AgentHealthStatus::Failed;
        }
        
        // Open circuit breaker permanently (or until manual intervention)
        self.transition_circuit_breaker(agent_id, CircuitBreakerStatus::Open).await;
        
        // Update trust score severely
        self.trust_manager.record_permanent_failure(agent_id).await?;
        
        Ok(())
    }

    /// Find a healthy agent for task redistribution
    async fn find_healthy_agent(&self) -> Option<AgentId> {
        for entry in self.heartbeats.iter() {
            let agent_id = *entry.key();
            let heartbeat = entry.value();
            
            if heartbeat.health_status == AgentHealthStatus::Healthy &&
               heartbeat.current_task.is_none() {
                if self.can_handle_task(agent_id).await {
                    return Some(agent_id);
                }
            }
        }
        
        None
    }

    /// Transition circuit breaker state
    async fn transition_circuit_breaker(&self, agent_id: AgentId, new_state: CircuitBreakerStatus) {
        if let Some(mut circuit_breaker) = self.circuit_breakers.get_mut(&agent_id) {
            if circuit_breaker.state != new_state {
                info!("Circuit breaker for agent {} transitioning from {:?} to {:?}", 
                      agent_id, circuit_breaker.state, new_state);
                
                circuit_breaker.state = new_state;
                circuit_breaker.last_state_change = Instant::now();
                
                // Reset failure count when closing
                if new_state == CircuitBreakerStatus::Closed {
                    circuit_breaker.failure_count = 0;
                }
            }
        }
    }

    /// Checkpoint manager loop
    async fn checkpoint_manager(&self) {
        let mut interval = interval(Duration::from_millis(self.config.checkpoint_interval_ms));
        
        while self.is_running.load(Ordering::SeqCst) {
            tokio::select! {
                _ = interval.tick() => {
                    self.cleanup_old_checkpoints().await;
                }
                _ = self.shutdown_notify.notified() => {
                    break;
                }
            }
        }
        
        debug!("Checkpoint manager stopped");
    }

    /// Clean up old checkpoints
    async fn cleanup_old_checkpoints(&self) {
        let cutoff_time = Utc::now() - chrono::Duration::hours(1); // Keep checkpoints for 1 hour
        let mut cleaned_count = 0;
        
        for mut entry in self.checkpoints.iter_mut() {
            let checkpoints = entry.value_mut();
            let original_len = checkpoints.len();
            
            checkpoints.retain(|checkpoint| checkpoint.timestamp > cutoff_time);
            
            cleaned_count += original_len - checkpoints.len();
        }
        
        if cleaned_count > 0 {
            debug!("Cleaned up {} old checkpoints", cleaned_count);
        }
    }

    /// Get recovery statistics
    pub async fn get_recovery_stats(&self) -> RecoveryStatistics {
        self.recovery_stats.read().await.clone()
    }

    /// Get agent health status
    pub async fn get_agent_health(&self, agent_id: AgentId) -> Option<AgentHealthStatus> {
        self.heartbeats.get(&agent_id).map(|hb| hb.health_status.clone())
    }

    /// Get all agent health statuses
    pub async fn get_all_agent_health(&self) -> HashMap<AgentId, AgentHealthStatus> {
        self.heartbeats
            .iter()
            .map(|entry| (*entry.key(), entry.value().health_status.clone()))
            .collect()
    }

    /// Get circuit breaker status for an agent
    pub async fn get_circuit_breaker_status(&self, agent_id: AgentId) -> Option<CircuitBreakerStatus> {
        self.circuit_breakers.get(&agent_id).map(|cb| cb.state.clone())
    }

    /// Clone for async task spawning
    fn clone_for_task(&self) -> FaultToleranceManager {
        Self {
            config: self.config.clone(),
            heartbeats: self.heartbeats.clone(),
            circuit_breakers: self.circuit_breakers.clone(),
            checkpoints: self.checkpoints.clone(),
            recovery_queue: self.recovery_queue.clone(),
            agents: self.agents.clone(),
            trust_manager: self.trust_manager.clone(),
            execution_engine: self.execution_engine.clone(),
            is_running: self.is_running.clone(),
            shutdown_notify: self.shutdown_notify.clone(),
            recovery_stats: self.recovery_stats.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::ExecutionConfig;

    #[tokio::test]
    async fn test_fault_tolerance_manager_creation() {
        let config = FaultToleranceConfig::default();
        let trust_manager = Arc::new(TrustManager::new());
        let execution_engine = Arc::new(ExecutionEngine::with_config(ExecutionConfig::default()));
        
        let manager = FaultToleranceManager::new(config, trust_manager, execution_engine);
        assert!(!manager.is_running.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_agent_registration() {
        let config = FaultToleranceConfig::default();
        let trust_manager = Arc::new(TrustManager::new());
        let execution_engine = Arc::new(ExecutionEngine::with_config(ExecutionConfig::default()));
        
        let manager = FaultToleranceManager::new(config, trust_manager, execution_engine);
        
        // Create a mock agent
        let agent = Arc::new(AutonomousAgent::new(
            Uuid::new_v4(),
            "test-agent".to_string(),
            vec!["test".to_string()],
        ));
        
        let result = manager.register_agent(agent.clone()).await;
        assert!(result.is_ok());
        
        // Check that agent is registered
        assert!(manager.heartbeats.contains_key(&agent.id()));
        assert!(manager.circuit_breakers.contains_key(&agent.id()));
    }

    #[tokio::test]
    async fn test_heartbeat_recording() {
        let config = FaultToleranceConfig::default();
        let trust_manager = Arc::new(TrustManager::new());
        let execution_engine = Arc::new(ExecutionEngine::with_config(ExecutionConfig::default()));
        
        let manager = FaultToleranceManager::new(config, trust_manager, execution_engine);
        
        let agent = Arc::new(AutonomousAgent::new(
            Uuid::new_v4(),
            "test-agent".to_string(),
            vec!["test".to_string()],
        ));
        
        manager.register_agent(agent.clone()).await.unwrap();
        
        let task_id = Uuid::new_v4();
        let result = manager.record_heartbeat(agent.id(), Some(task_id)).await;
        assert!(result.is_ok());
        
        // Check heartbeat was recorded
        let heartbeat = manager.heartbeats.get(&agent.id()).unwrap();
        assert_eq!(heartbeat.current_task, Some(task_id));
        assert_eq!(heartbeat.missed_heartbeats, 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_functionality() {
        let config = FaultToleranceConfig::default();
        let trust_manager = Arc::new(TrustManager::new());
        let execution_engine = Arc::new(ExecutionEngine::with_config(ExecutionConfig::default()));
        
        let manager = FaultToleranceManager::new(config.clone(), trust_manager, execution_engine);
        
        let agent = Arc::new(AutonomousAgent::new(
            Uuid::new_v4(),
            "test-agent".to_string(),
            vec!["test".to_string()],
        ));
        
        manager.register_agent(agent.clone()).await.unwrap();
        
        // Agent should be able to handle tasks initially
        assert!(manager.can_handle_task(agent.id()).await);
        
        // Record multiple failures
        for _ in 0..config.circuit_breaker_failure_threshold {
            manager.record_task_result(agent.id(), false, 1000).await.unwrap();
        }
        
        // Circuit should be open now
        let cb_status = manager.get_circuit_breaker_status(agent.id()).await;
        assert_eq!(cb_status, Some(CircuitBreakerStatus::Open));
        
        // Agent should not be able to handle tasks
        assert!(!manager.can_handle_task(agent.id()).await);
    }

    #[tokio::test]
    async fn test_checkpoint_creation() {
        let config = FaultToleranceConfig::default();
        let trust_manager = Arc::new(TrustManager::new());
        let execution_engine = Arc::new(ExecutionEngine::with_config(ExecutionConfig::default()));
        
        let manager = FaultToleranceManager::new(config, trust_manager, execution_engine);
        
        let task_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        
        let task_state = TaskState {
            current_step: 2,
            completed_steps: vec![],
            remaining_steps: vec![],
            metadata: HashMap::new(),
        };
        
        let progress = TaskProgress {
            percentage: 0.5,
            completed_steps: 2,
            total_steps: 4,
            estimated_remaining_ms: Some(5000),
        };
        
        let context = HashMap::new();
        
        let checkpoint_id = manager
            .create_checkpoint(task_id, agent_id, task_state, progress, context)
            .await
            .unwrap();
        
        // Verify checkpoint was created
        let retrieved_checkpoint = manager.get_latest_checkpoint(task_id).await;
        assert!(retrieved_checkpoint.is_some());
        assert_eq!(retrieved_checkpoint.unwrap().checkpoint_id, checkpoint_id);
    }

    #[tokio::test]
    async fn test_recovery_statistics() {
        let config = FaultToleranceConfig::default();
        let trust_manager = Arc::new(TrustManager::new());
        let execution_engine = Arc::new(ExecutionEngine::with_config(ExecutionConfig::default()));
        
        let manager = FaultToleranceManager::new(config, trust_manager, execution_engine);
        
        let stats = manager.get_recovery_stats().await;
        assert_eq!(stats.total_recovery_attempts, 0);
        assert_eq!(stats.successful_recoveries, 0);
        assert_eq!(stats.failed_recoveries, 0);
    }
}