//! Main swarm coordination engine

use crate::{
    error::{SwarmError, SwarmResult},
    types::*,
    protocol::AgentProtocol,
    trust::TrustManager,
    verification::SwarmVerifier,
    execution::{ExecutionEngine, ExecutionConfig},
    recovery::{RecoveryManager, RecoveryResult},
};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, warn, error, debug};
use synapsed_intent::{HierarchicalIntent, IntentContext, VerifiedExecutor};
use synapsed_promise::{AutonomousAgent, Promise, PromiseContract, Willingness};
use synapsed_verify::VerificationResult;

/// Configuration for the swarm coordinator
#[derive(Debug, Clone)]
pub struct SwarmConfig {
    /// Maximum number of agents in the swarm
    pub max_agents: usize,
    /// Minimum trust score for task assignment
    pub min_trust_score: f64,
    /// Enable verification for all tasks
    pub require_verification: bool,
    /// Timeout for task execution in seconds
    pub task_timeout_secs: u64,
    /// Enable promise tracking
    pub track_promises: bool,
    /// Enable consensus for critical decisions
    pub require_consensus: bool,
    /// Consensus threshold (percentage of agents that must agree)
    pub consensus_threshold: f64,
    /// Execution engine configuration
    pub execution_config: ExecutionConfig,
    /// Fault tolerance configuration
    pub fault_tolerance_config: FaultToleranceConfig,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            max_agents: crate::MAX_SWARM_SIZE,
            min_trust_score: 0.3,
            require_verification: true,
            task_timeout_secs: 300,
            track_promises: true,
            require_consensus: false,
            consensus_threshold: 0.66,
            execution_config: ExecutionConfig::default(),
            fault_tolerance_config: FaultToleranceConfig::default(),
        }
    }
}

/// Current state of the swarm
#[derive(Debug, Clone)]
pub struct SwarmState {
    /// Unique swarm ID
    pub swarm_id: SwarmId,
    /// Number of active agents
    pub active_agents: usize,
    /// Number of pending tasks
    pub pending_tasks: usize,
    /// Number of running tasks
    pub running_tasks: usize,
    /// Current phase
    pub phase: SwarmPhase,
    /// Swarm metrics
    pub metrics: SwarmMetrics,
}

/// Phase of swarm operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwarmPhase {
    /// Swarm is initializing
    Initializing,
    /// Swarm is ready for tasks
    Ready,
    /// Swarm is actively coordinating
    Coordinating,
    /// Swarm is in consensus phase
    Consensus,
    /// Swarm is shutting down
    ShuttingDown,
}

/// Main swarm coordinator
pub struct SwarmCoordinator {
    /// Unique swarm ID
    swarm_id: SwarmId,
    /// Configuration
    config: Arc<SwarmConfig>,
    /// Current state
    state: Arc<RwLock<SwarmState>>,
    /// Active agents in the swarm
    agents: Arc<DashMap<AgentId, Arc<AutonomousAgent>>>,
    /// Agent statuses
    agent_statuses: Arc<DashMap<AgentId, AgentStatus>>,
    /// Active task assignments
    tasks: Arc<DashMap<TaskId, TaskAssignment>>,
    /// Task results
    results: Arc<DashMap<TaskId, TaskResult>>,
    /// Trust manager
    trust_manager: Arc<TrustManager>,
    /// Verification system
    verifier: Arc<SwarmVerifier>,
    /// Protocol handler
    protocol: Arc<AgentProtocol>,
    /// Intent executor
    intent_executor: Arc<RwLock<VerifiedExecutor>>,
    /// Execution engine for real command execution
    execution_engine: Arc<ExecutionEngine>,
    /// Fault tolerance manager
    fault_tolerance_manager: Arc<FaultToleranceManager>,
    /// Event log
    events: Arc<RwLock<Vec<SwarmEvent>>>,
}

impl SwarmCoordinator {
    /// Create a new swarm coordinator
    pub fn new(config: SwarmConfig) -> Self {
        let swarm_id = Uuid::new_v4();
        
        let state = SwarmState {
            swarm_id,
            active_agents: 0,
            pending_tasks: 0,
            running_tasks: 0,
            phase: SwarmPhase::Initializing,
            metrics: SwarmMetrics::default(),
        };
        
        let trust_manager = Arc::new(TrustManager::new());
        let execution_engine = Arc::new(ExecutionEngine::with_config(config.execution_config.clone()));
        let fault_tolerance_manager = Arc::new(FaultToleranceManager::new(
            config.fault_tolerance_config.clone(),
            trust_manager.clone(),
            execution_engine.clone(),
        ));
        
        Self {
            swarm_id,
            config: Arc::new(config),
            state: Arc::new(RwLock::new(state)),
            agents: Arc::new(DashMap::new()),
            agent_statuses: Arc::new(DashMap::new()),
            tasks: Arc::new(DashMap::new()),
            results: Arc::new(DashMap::new()),
            trust_manager,
            verifier: Arc::new(SwarmVerifier::new()),
            protocol: Arc::new(AgentProtocol::new()),
            intent_executor: Arc::new(RwLock::new(VerifiedExecutor::new())),
            execution_engine,
            fault_tolerance_manager,
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Initialize the swarm
    pub async fn initialize(&self) -> SwarmResult<()> {
        info!("Initializing swarm {}", self.swarm_id);
        
        // Initialize trust manager
        self.trust_manager.initialize().await?;
        
        // Initialize verifier
        self.verifier.initialize().await?;
        
        // Initialize execution engine
        self.execution_engine.initialize().await?;
        
        // Initialize recovery manager
        self.recovery_manager.start_monitoring().await;
        
        // Update state
        let mut state = self.state.write().await;
        state.phase = SwarmPhase::Ready;
        
        info!("Swarm {} initialized successfully", self.swarm_id);
        Ok(())
    }
    
    /// Add an agent to the swarm
    pub async fn add_agent(
        &self,
        agent: Arc<AutonomousAgent>,
        role: AgentRole,
    ) -> SwarmResult<AgentId> {
        let mut state = self.state.write().await;
        
        // Check swarm size limit
        if state.active_agents >= self.config.max_agents {
            return Err(SwarmError::SwarmSizeLimitExceeded {
                current: state.active_agents,
                max: self.config.max_agents,
            });
        }
        
        let agent_id = agent.id();
        
        // Initialize agent
        agent.initialize().await?;
        
        // Add to swarm
        self.agents.insert(agent_id, agent.clone());
        self.agent_statuses.insert(agent_id, AgentStatus::Ready);
        
        // Initialize trust score
        self.trust_manager.initialize_agent(agent_id, crate::DEFAULT_TRUST_SCORE).await?;
        
        // Register agent for fault tolerance monitoring
        self.fault_tolerance_manager.register_agent(agent.clone()).await?;
        
        // Update state
        state.active_agents += 1;
        
        // Log event
        self.log_event(SwarmEvent::AgentJoined {
            agent_id,
            role,
            timestamp: Utc::now(),
        }).await;
        
        info!("Agent {} joined swarm {}", agent_id, self.swarm_id);
        Ok(agent_id)
    }
    
    /// Delegate an intent to the swarm
    pub async fn delegate_intent(
        &self,
        intent: HierarchicalIntent,
        context: IntentContext,
    ) -> SwarmResult<TaskId> {
        let task_id = Uuid::new_v4();
        
        info!("Delegating intent {} as task {}", intent.id(), task_id);
        
        // Find suitable agent (fault tolerance aware)
        let agent_id = self.select_agent_for_task(&intent, &context).await?;
        
        // Get agent
        let agent = self.agents.get(&agent_id)
            .ok_or_else(|| SwarmError::AgentNotFound(agent_id))?
            .clone();
        
        // Negotiate promise with agent
        let promise = self.negotiate_promise(&agent, &intent, &context).await?;
        
        // Create task assignment
        let assignment = TaskAssignment {
            task_id,
            agent_id,
            intent: intent.clone(),
            promise: Some(promise.clone()),
            parent_task: None,
            context: context.variables().clone(),
            verification_required: self.config.require_verification,
            deadline: None,
        };
        
        // Store assignment
        self.tasks.insert(task_id, assignment.clone());
        
        // Update agent status
        self.agent_statuses.insert(agent_id, AgentStatus::Busy);
        
        // Update state
        let mut state = self.state.write().await;
        state.pending_tasks += 1;
        
        // Log event
        self.log_event(SwarmEvent::TaskAssigned {
            task_id,
            agent_id,
            timestamp: Utc::now(),
        }).await;
        
        // Execute task asynchronously
        let coordinator = self.clone_inner();
        tokio::spawn(async move {
            if let Err(e) = coordinator.execute_task(task_id).await {
                error!("Task {} execution failed: {}", task_id, e);
            }
        });
        
        Ok(task_id)
    }
    
    /// Execute a task
    async fn execute_task(&self, task_id: TaskId) -> SwarmResult<()> {
        let assignment = self.tasks.get(&task_id)
            .ok_or_else(|| SwarmError::Other(anyhow::anyhow!("Task not found")))?
            .clone();
        
        let agent = self.agents.get(&assignment.agent_id)
            .ok_or_else(|| SwarmError::AgentNotFound(assignment.agent_id))?
            .clone();
        
        debug!("Executing task {} with agent {}", task_id, assignment.agent_id);
        
        // Update state
        {
            let mut state = self.state.write().await;
            state.pending_tasks = state.pending_tasks.saturating_sub(1);
            state.running_tasks += 1;
        }
        
        let start_time = Utc::now();
        
        // Record heartbeat at start of execution
        self.fault_tolerance_manager
            .record_heartbeat(assignment.agent_id, Some(task_id))
            .await?;
        
        // Execute with verification
        let result = if assignment.verification_required {
            self.execute_with_verification(&assignment, &agent).await
        } else {
            self.execute_without_verification(&assignment, &agent).await
        };
        
        let duration_ms = (Utc::now() - start_time).num_milliseconds() as u64;
        
        // Create task result
        let task_result = match result {
            Ok((output, proof)) => TaskResult {
                task_id,
                agent_id: assignment.agent_id,
                success: true,
                output: Some(output),
                error: None,
                verification_proof: proof,
                duration_ms,
                completed_at: Utc::now(),
            },
            Err(e) => TaskResult {
                task_id,
                agent_id: assignment.agent_id,
                success: false,
                output: None,
                error: Some(e.to_string()),
                verification_proof: None,
                duration_ms,
                completed_at: Utc::now(),
            },
        };
        
        // Store result
        self.results.insert(task_id, task_result.clone());
        
        // Update promise status
        if let Some(promise) = assignment.promise {
            if task_result.success {
                agent.fulfill_promise(promise.id()).await?;
                self.log_event(SwarmEvent::PromiseFulfilled {
                    agent_id: assignment.agent_id,
                    promise_id: promise.id(),
                    timestamp: Utc::now(),
                }).await;
            } else {
                agent.break_promise(promise.id(), task_result.error.clone().unwrap_or_default()).await?;
                self.log_event(SwarmEvent::PromiseBroken {
                    agent_id: assignment.agent_id,
                    promise_id: promise.id(),
                    reason: task_result.error.clone().unwrap_or_default(),
                    timestamp: Utc::now(),
                }).await;
            }
        }
        
        // Update trust score
        self.trust_manager.update_trust(
            assignment.agent_id,
            task_result.success,
            task_result.verification_proof.is_some(),
        ).await?;
        
        // Record task result for fault tolerance (circuit breaker)
        self.fault_tolerance_manager.record_task_result(
            assignment.agent_id,
            task_result.success,
            duration_ms,
        ).await?;
        
        // Update agent status
        self.agent_statuses.insert(assignment.agent_id, AgentStatus::Ready);
        
        // Update state
        {
            let mut state = self.state.write().await;
            state.running_tasks = state.running_tasks.saturating_sub(1);
            if task_result.success {
                state.metrics.tasks_succeeded += 1;
            } else {
                state.metrics.tasks_failed += 1;
            }
        }
        
        // Log event
        self.log_event(SwarmEvent::TaskCompleted {
            task_id,
            agent_id: assignment.agent_id,
            success: task_result.success,
            timestamp: Utc::now(),
        }).await;
        
        Ok(())
    }
    
    /// Execute with verification using real execution engine
    async fn execute_with_verification(
        &self,
        assignment: &TaskAssignment,
        agent: &Arc<AutonomousAgent>,
    ) -> SwarmResult<(serde_json::Value, Option<synapsed_verify::VerificationProof>)> {
        debug!("Executing task {} with verification using real execution engine", assignment.task_id);
        
        // Execute each step of the intent using the real execution engine
        let mut step_results = Vec::new();
        let steps = assignment.intent.steps();
        
        for (step_index, step) in steps.iter().enumerate() {
            info!("Executing step {} of {}: {}", step_index + 1, steps.len(), step.description);
            
            // Execute step using the execution engine
            let step_result = self.execution_engine
                .execute_intent_step(&assignment.intent, step_index)
                .await?;
            
            // If step failed, stop execution
            if !step_result.success {
                let error_msg = format!("Step {} failed: {:?}", step_index + 1, step_result.output);
                error!("{}", error_msg);
                return Err(SwarmError::Other(anyhow::anyhow!(error_msg)));
            }
            
            step_results.push(step_result);
        }
        
        // Combine step results into final result
        let final_result = Self::combine_step_results(&step_results);
        
        // Verify execution using the verification system
        let verification_report = self.verifier.verify_execution(
            &assignment.intent,
            &final_result,
            assignment.agent_id,
        ).await?;
        
        // Generate proof if verification passed
        let proof = if verification_report.verified {
            verification_report.proof
        } else {
            warn!("Verification failed for task {}", assignment.task_id);
            return Err(SwarmError::Other(anyhow::anyhow!("Verification failed")));
        };
        
        Ok((final_result.output.unwrap_or_default(), proof))
    }
    
    /// Execute without verification using real execution engine
    async fn execute_without_verification(
        &self,
        assignment: &TaskAssignment,
        agent: &Arc<AutonomousAgent>,
    ) -> SwarmResult<(serde_json::Value, Option<synapsed_verify::VerificationProof>)> {
        debug!("Executing task {} without verification using real execution engine", assignment.task_id);
        
        // Execute each step of the intent using the real execution engine
        let mut step_results = Vec::new();
        let steps = assignment.intent.steps();
        
        for (step_index, step) in steps.iter().enumerate() {
            info!("Executing step {} of {}: {}", step_index + 1, steps.len(), step.description);
            
            // Execute step using the execution engine
            let step_result = self.execution_engine
                .execute_intent_step(&assignment.intent, step_index)
                .await?;
            
            // Continue even if step fails in non-verification mode
            if !step_result.success {
                warn!("Step {} failed but continuing: {:?}", step_index + 1, step_result.output);
            }
            
            step_results.push(step_result);
        }
        
        // Combine step results into final result
        let final_result = Self::combine_step_results(&step_results);
        
        Ok((final_result.output.unwrap_or_default(), None))
    }
    
    /// Combine multiple step results into a single result
    fn combine_step_results(step_results: &[synapsed_intent::StepResult]) -> synapsed_intent::StepResult {
        let all_successful = step_results.iter().all(|r| r.success);
        let combined_outputs: Vec<_> = step_results.iter()
            .filter_map(|r| r.output.as_ref())
            .collect();
        
        let combined_metadata = step_results.iter()
            .flat_map(|r| r.metadata.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        
        synapsed_intent::StepResult {
            success: all_successful,
            output: Some(serde_json::json!(combined_outputs)),
            metadata: combined_metadata,
        }
    }
    
    /// Select an agent for a task
    async fn select_agent_for_task(
        &self,
        intent: &HierarchicalIntent,
        context: &IntentContext,
    ) -> SwarmResult<AgentId> {
        let mut candidates = Vec::new();
        
        for entry in self.agents.iter() {
            let agent_id = *entry.key();
            let agent = entry.value();
            
            // Check if agent is available
            if let Some(status) = self.agent_statuses.get(&agent_id) {
                if *status != AgentStatus::Ready {
                    continue;
                }
            }
            
            // Check fault tolerance - circuit breaker
            if !self.fault_tolerance_manager.can_handle_task(agent_id).await {
                continue;
            }
            
            // Check trust score
            let trust_score = self.trust_manager.get_trust(agent_id).await?;
            if trust_score < self.config.min_trust_score {
                continue;
            }
            
            // Check agent capabilities
            if agent.can_handle(intent).await {
                candidates.push((agent_id, trust_score));
            }
        }
        
        // Select agent with highest trust score
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        candidates.first()
            .map(|(id, _)| *id)
            .ok_or_else(|| SwarmError::Other(anyhow::anyhow!("No suitable agent found")))
    }
    
    /// Negotiate a promise with an agent
    async fn negotiate_promise(
        &self,
        agent: &Arc<AutonomousAgent>,
        intent: &HierarchicalIntent,
        context: &IntentContext,
    ) -> SwarmResult<Promise> {
        // Create promise contract
        let contract = PromiseContract::new(
            format!("Execute intent {}", intent.id()),
            synapsed_promise::PromiseType::Offer,
            synapsed_promise::PromiseScope::Specific(vec![self.swarm_id]),
        );
        
        // Check agent willingness
        let willingness = agent.evaluate_willingness(&contract).await?;
        
        match willingness {
            Willingness::Willing { confidence } if confidence > 0.5 => {
                // Agent is willing, make promise
                let promise = agent.make_promise(contract).await?;
                
                // Log event
                self.log_event(SwarmEvent::PromiseMade {
                    agent_id: agent.id(),
                    promise_id: promise.id(),
                    timestamp: Utc::now(),
                }).await;
                
                Ok(promise)
            }
            _ => {
                Err(SwarmError::Other(anyhow::anyhow!(
                    "Agent {} unwilling to make promise",
                    agent.id()
                )))
            }
        }
    }
    
    /// Get swarm state
    pub async fn state(&self) -> SwarmState {
        self.state.read().await.clone()
    }
    
    /// Get swarm metrics
    pub async fn metrics(&self) -> SwarmMetrics {
        let state = self.state.read().await;
        state.metrics.clone()
    }
    
    /// Get task result
    pub async fn get_task_result(&self, task_id: TaskId) -> Option<TaskResult> {
        self.results.get(&task_id).map(|r| r.clone())
    }
    
    /// Get the execution engine for direct access
    pub fn execution_engine(&self) -> &Arc<ExecutionEngine> {
        &self.execution_engine
    }
    
    /// Update execution engine configuration
    pub async fn update_execution_config(&self, new_config: ExecutionConfig) -> SwarmResult<()> {
        self.execution_engine.update_config(new_config).await?;
        info!("Swarm execution configuration updated");
        Ok(())
    }
    
    /// Get the recovery manager for direct access
    pub fn recovery_manager(&self) -> &Arc<RecoveryManager> {
        &self.recovery_manager
    }
    
    /// Create a recovery checkpoint of the current swarm state
    pub async fn create_checkpoint(&self) -> SwarmResult<uuid::Uuid> {
        self.recovery_manager.create_checkpoint(self).await
    }
    
    /// Attempt to recover from an error using available strategies
    pub async fn recover_from_error(
        &self,
        error: SwarmError,
        failed_task_id: Option<TaskId>,
        failed_agent_id: Option<AgentId>,
    ) -> SwarmResult<RecoveryResult> {
        let coordinator_arc = Arc::new(Self::new(self.config.as_ref().clone())); // Simplified for now
        self.recovery_manager
            .recover(coordinator_arc, error, failed_task_id, failed_agent_id)
            .await
    }
    
    /// Log an event
    async fn log_event(&self, event: SwarmEvent) {
        let mut events = self.events.write().await;
        events.push(event);
    }
    
    /// Get fault tolerance manager
    pub fn fault_tolerance_manager(&self) -> &Arc<FaultToleranceManager> {
        &self.fault_tolerance_manager
    }
    
    /// Get agent health status
    pub async fn get_agent_health(&self, agent_id: AgentId) -> Option<crate::fault_tolerance::AgentHealthStatus> {
        self.fault_tolerance_manager.get_agent_health(agent_id).await
    }
    
    /// Get all agent health statuses
    pub async fn get_all_agent_health(&self) -> HashMap<AgentId, crate::fault_tolerance::AgentHealthStatus> {
        self.fault_tolerance_manager.get_all_agent_health().await
    }
    
    /// Get recovery statistics
    pub async fn get_recovery_stats(&self) -> crate::fault_tolerance::RecoveryStatistics {
        self.fault_tolerance_manager.get_recovery_stats().await
    }
    
    /// Create a checkpoint for a task
    pub async fn create_task_checkpoint(
        &self, 
        task_id: TaskId, 
        agent_id: AgentId,
        current_step: usize,
        progress_percentage: f64,
        context: HashMap<String, serde_json::Value>,
    ) -> SwarmResult<uuid::Uuid> {
        use crate::fault_tolerance::{TaskState, TaskProgress};
        
        let task_state = TaskState {
            current_step,
            completed_steps: Vec::new(), // Would be populated in real implementation
            remaining_steps: Vec::new(), // Would be populated in real implementation
            metadata: HashMap::new(),
        };
        
        let progress = TaskProgress {
            percentage: progress_percentage,
            completed_steps: current_step,
            total_steps: 0, // Would be calculated in real implementation
            estimated_remaining_ms: None,
        };
        
        self.fault_tolerance_manager
            .create_checkpoint(task_id, agent_id, task_state, progress, context)
            .await
    }
    
    /// Remove an agent from the swarm
    pub async fn remove_agent(&self, agent_id: AgentId) -> SwarmResult<()> {
        // Unregister from fault tolerance monitoring
        self.fault_tolerance_manager.unregister_agent(agent_id).await?;
        
        // Remove from swarm
        self.agents.remove(&agent_id);
        self.agent_statuses.remove(&agent_id);
        
        // Update state
        let mut state = self.state.write().await;
        state.active_agents = state.active_agents.saturating_sub(1);
        
        // Log event
        self.log_event(SwarmEvent::AgentLeft {
            agent_id,
            reason: "Removed by coordinator".to_string(),
            timestamp: Utc::now(),
        }).await;
        
        info!("Agent {} removed from swarm {}", agent_id, self.swarm_id);
        Ok(())
    }
    
    /// Shutdown the swarm
    pub async fn shutdown(&self) -> SwarmResult<()> {
        info!("Shutting down swarm {}", self.swarm_id);
        
        // Update state
        {
            let mut state = self.state.write().await;
            state.phase = SwarmPhase::ShuttingDown;
        }
        
        // Stop fault tolerance manager
        self.fault_tolerance_manager.stop().await?;
        
        // Clear agents
        self.agents.clear();
        self.agent_statuses.clear();
        
        info!("Swarm {} shutdown complete", self.swarm_id);
        Ok(())
    }
    
    /// Clone inner references for spawning
    fn clone_inner(&self) -> Arc<Self> {
        // This would need proper implementation with Arc<Self>
        // For now, returning a placeholder
        Arc::new(Self::new(self.config.as_ref().clone()))
    }
}

// Implement Observable trait
impl synapsed_core::traits::Observable for SwarmCoordinator {
    fn status(&self) -> synapsed_core::ObservableStatus {
        synapsed_core::ObservableStatus::Healthy
    }
    
    fn health(&self) -> synapsed_core::Health {
        synapsed_core::Health::default()
    }
    
    fn metrics(&self) -> synapsed_core::MetricSet {
        synapsed_core::MetricSet::default()
    }
}