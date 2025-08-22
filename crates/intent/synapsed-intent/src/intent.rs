//! Hierarchical intent implementation for AI agent planning

use crate::{
    types::*, IntentError, Result,
    context::IntentContext,
    checkpoint::CheckpointManager,
};
use futures::future::BoxFuture;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;
use synapsed_substrates::{Subject, types::{Name, SubjectType}};

/// A hierarchical intent representing a goal to be achieved
#[derive(Debug, Clone)]
pub struct HierarchicalIntent {
    /// Unique intent ID
    id: IntentId,
    /// Goal of the intent
    goal: String,
    /// Description of the intent
    description: Option<String>,
    /// Steps to achieve the goal
    steps: Vec<Step>,
    /// Sub-intents (child intents)
    sub_intents: Vec<HierarchicalIntent>,
    /// Metadata
    metadata: IntentMetadata,
    /// Execution configuration
    config: ExecutionConfig,
    /// Current status
    status: Arc<RwLock<IntentStatus>>,
    /// Context bounds
    bounds: ContextBounds,
    /// Observable substrate
    substrate: Arc<Subject>,
}

impl HierarchicalIntent {
    /// Creates a new hierarchical intent
    pub fn new(goal: impl Into<String>) -> Self {
        let id = IntentId::new();
        let goal = goal.into();
        let substrate = Subject::new(
            Name::from(format!("intent.{}", id.0).as_str()),
            SubjectType::Source
        );
        
        Self {
            id,
            goal: goal.clone(),
            description: None,
            steps: Vec::new(),
            sub_intents: Vec::new(),
            metadata: IntentMetadata {
                creator: "unknown".to_string(),
                created_at: Utc::now(),
                modified_at: Utc::now(),
                tags: Vec::new(),
                parent_intent: None,
                agent_context: None,
                priority: Priority::Normal,
                estimated_duration_ms: None,
            },
            config: ExecutionConfig::default(),
            status: Arc::new(RwLock::new(IntentStatus::Pending)),
            bounds: ContextBounds::default(),
            substrate: Arc::new(substrate),
        }
    }
    
    /// Gets the intent ID
    pub fn id(&self) -> IntentId {
        self.id
    }
    
    /// Gets the goal
    pub fn goal(&self) -> &str {
        &self.goal
    }
    
    /// Sets the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    
    /// Adds a step to the intent
    pub fn step(mut self, name: impl Into<String>, action: StepAction) -> Self {
        let step = Step {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            action,
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            dependencies: Vec::new(),
            verification: None,
            status: StepStatus::Pending,
            result: None,
        };
        self.steps.push(step);
        self
    }
    
    /// Adds a step with verification
    pub fn verified_step(
        mut self,
        name: impl Into<String>,
        action: StepAction,
        verification: VerificationRequirement,
    ) -> Self {
        let step = Step {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            action,
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            dependencies: Vec::new(),
            verification: Some(verification),
            status: StepStatus::Pending,
            result: None,
        };
        self.steps.push(step);
        self
    }
    
    /// Adds a sub-intent
    pub fn sub_intent(mut self, sub_intent: HierarchicalIntent) -> Self {
        let mut sub = sub_intent;
        sub.metadata.parent_intent = Some(self.id);
        self.sub_intents.push(sub);
        self
    }
    
    /// Adds a precondition to the last step
    pub fn requires(mut self, condition: Condition) -> Self {
        if let Some(step) = self.steps.last_mut() {
            step.preconditions.push(condition);
        }
        self
    }
    
    /// Adds a postcondition to the last step
    pub fn ensures(mut self, condition: Condition) -> Self {
        if let Some(step) = self.steps.last_mut() {
            step.postconditions.push(condition);
        }
        self
    }
    
    /// Sets the priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.metadata.priority = priority;
        self
    }
    
    /// Sets the context bounds
    pub fn with_bounds(mut self, bounds: ContextBounds) -> Self {
        self.bounds = bounds;
        self
    }
    
    /// Sets the execution configuration
    pub fn with_config(mut self, config: ExecutionConfig) -> Self {
        self.config = config;
        self
    }
    
    /// Gets the current status
    pub async fn status(&self) -> IntentStatus {
        *self.status.read().await
    }
    
    /// Validates the intent structure
    pub fn validate(&self) -> futures::future::BoxFuture<'_, Result<()>> {
        Box::pin(async move {
            // Check for circular dependencies in steps
            let graph = self.build_dependency_graph()?;
            if toposort(&graph, None).is_err() {
                return Err(IntentError::ValidationFailed(
                    "Circular dependencies detected in steps".to_string()
                ));
            }
            
            // Validate each step
            for step in &self.steps {
                self.validate_step(step)?;
            }
            
            // Validate sub-intents
            for sub in &self.sub_intents {
                sub.validate().await?;
            }
            
            Ok(())
        })
    }
    
    /// Plans the intent execution
    pub fn plan(&self) -> futures::future::BoxFuture<'_, Result<ExecutionPlan>> {
        Box::pin(async move {
            let mut plan = ExecutionPlan {
                intent_id: self.id,
                steps: Vec::new(),
                sub_plans: Vec::new(),
                estimated_duration_ms: 0,
                parallelizable_groups: Vec::new(),
            };
            
            // Build dependency graph
            let graph = self.build_dependency_graph()?;
            
            // Topological sort for execution order
            let sorted = toposort(&graph, None)
                .map_err(|_| IntentError::ValidationFailed("Cannot sort dependencies".to_string()))?;
            
            // Group parallelizable steps
            let groups = self.identify_parallel_groups(&graph, &sorted);
            plan.parallelizable_groups = groups;
            
            // Add steps to plan
            for node_idx in sorted {
                if let Some(step_id) = graph.node_weight(node_idx) {
                    if let Some(step) = self.steps.iter().find(|s| &s.id == step_id) {
                        plan.steps.push(step.id);
                    }
                }
            }
            
            // Plan sub-intents
            for sub in &self.sub_intents {
                let sub_plan = sub.plan().await?;
                plan.sub_plans.push(sub_plan);
            }
            
            Ok(plan)
        })
    }
    
    /// Executes the intent
    pub fn execute<'a>(&'a self, context: &'a IntentContext) -> BoxFuture<'a, Result<IntentResult>> {
        Box::pin(async move {
        // Update status
        *self.status.write().await = IntentStatus::Executing;
        
        // Emit start event
        self.emit_event(EventType::Started, serde_json::json!({
            "goal": self.goal,
            "steps": self.steps.len(),
            "sub_intents": self.sub_intents.len(),
        })).await;
        
        let start = Utc::now();
        let mut results = Vec::new();
        let mut success = true;
        
        // Create checkpoint manager
        let checkpoint_manager = CheckpointManager::new();
        
        // Plan execution
        let plan = self.plan().await?;
        
        // Execute steps according to plan
        for step_id in &plan.steps {
            if let Some(step) = self.steps.iter().find(|s| s.id == *step_id) {
                // Check preconditions
                if !self.check_conditions(&step.preconditions, context).await? {
                    if self.config.stop_on_failure {
                        success = false;
                        break;
                    }
                    continue;
                }
                
                // Create checkpoint if rollback enabled
                if self.config.enable_rollback {
                    checkpoint_manager.create_checkpoint(self.id, step.id).await?;
                }
                
                // Execute step
                let step_result = self.execute_step(step, context).await?;
                results.push(step_result.clone());
                
                // Check postconditions
                if !self.check_conditions(&step.postconditions, context).await? {
                    if self.config.stop_on_failure {
                        success = false;
                        if self.config.enable_rollback {
                            checkpoint_manager.rollback_to_last().await?;
                        }
                        break;
                    }
                }
                
                if !step_result.success && self.config.stop_on_failure {
                    success = false;
                    break;
                }
            }
        }
        
        // Execute sub-intents
        for sub in &self.sub_intents {
            let sub_context = context.create_child_context(sub.bounds.clone());
            let sub_result = sub.execute(&sub_context).await?;
            
            if !sub_result.success && self.config.stop_on_failure {
                success = false;
                break;
            }
        }
        
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        // Update status
        *self.status.write().await = if success {
            IntentStatus::Completed
        } else {
            IntentStatus::Failed
        };
        
        // Emit completion event
        self.emit_event(
            if success { EventType::Completed } else { EventType::Failed },
            serde_json::json!({
                "duration_ms": duration_ms,
                "steps_executed": results.len(),
            })
        ).await;
        
        Ok(IntentResult {
            intent_id: self.id,
            success,
            step_results: results,
            duration_ms,
            verification_proofs: Vec::new(),
        })
        })
    }
    
    // Helper methods
    
    fn validate_step(&self, step: &Step) -> Result<()> {
        // Check that dependencies exist
        for dep_id in &step.dependencies {
            if !self.steps.iter().any(|s| &s.id == dep_id) {
                return Err(IntentError::ValidationFailed(
                    format!("Step dependency {} not found", dep_id)
                ));
            }
        }
        
        Ok(())
    }
    
    fn build_dependency_graph(&self) -> Result<DiGraph<Uuid, ()>> {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();
        
        // Add nodes for each step
        for step in &self.steps {
            let node = graph.add_node(step.id);
            node_map.insert(step.id, node);
        }
        
        // Add edges for dependencies
        for step in &self.steps {
            let step_node = node_map[&step.id];
            for dep_id in &step.dependencies {
                if let Some(&dep_node) = node_map.get(dep_id) {
                    graph.add_edge(dep_node, step_node, ());
                }
            }
        }
        
        Ok(graph)
    }
    
    fn identify_parallel_groups(
        &self,
        graph: &DiGraph<Uuid, ()>,
        sorted: &[NodeIndex],
    ) -> Vec<Vec<Uuid>> {
        let mut groups = Vec::new();
        let mut processed = HashSet::new();
        
        for &node in sorted {
            if processed.contains(&node) {
                continue;
            }
            
            let mut group = Vec::new();
            let node_id = graph[node];
            
            // Find all nodes at the same level (no dependencies between them)
            for &other in sorted {
                if processed.contains(&other) || other == node {
                    continue;
                }
                
                let other_id = graph[other];
                
                // Check if there's a path between nodes
                if !petgraph::algo::has_path_connecting(graph, node, other, None) &&
                   !petgraph::algo::has_path_connecting(graph, other, node, None) {
                    group.push(other_id);
                    processed.insert(other);
                }
            }
            
            if !group.is_empty() {
                group.push(node_id);
                groups.push(group);
            }
            processed.insert(node);
        }
        
        groups
    }
    
    async fn check_conditions(
        &self,
        conditions: &[Condition],
        _context: &IntentContext,
    ) -> Result<bool> {
        for condition in conditions {
            let met = match condition.condition_type {
                ConditionType::FileExists => {
                    // Would integrate with synapsed-verify
                    true
                },
                ConditionType::CommandSuccess => {
                    // Would integrate with synapsed-verify
                    true
                },
                ConditionType::StateMatch => {
                    // Would integrate with synapsed-verify
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
    
    async fn execute_step(
        &self,
        step: &Step,
        _context: &IntentContext,
    ) -> Result<StepResult> {
        let start = Utc::now();
        
        self.emit_event(EventType::StepStarted, serde_json::json!({
            "step_id": step.id,
            "step_name": step.name,
        })).await;
        
        // Execute the action
        let (success, output, error) = match &step.action {
            StepAction::Command(cmd) => {
                // Would integrate with synapsed-verify command verifier
                (true, Some(serde_json::json!({"command": cmd})), None)
            },
            StepAction::Function(name, args) => {
                // Execute function
                (true, Some(serde_json::json!({"function": name, "args": args})), None)
            },
            StepAction::Delegate(spec) => {
                // Would integrate with synapsed-promise for delegation
                (true, Some(serde_json::json!({"delegated": spec.task})), None)
            },
            StepAction::Composite(actions) => {
                // Execute composite actions
                (true, Some(serde_json::json!({"actions": actions.len()})), None)
            },
            StepAction::Custom(value) => {
                (true, Some(value.clone()), None)
            },
        };
        
        // Perform verification if required
        let verification = if let Some(_req) = &step.verification {
            if self.config.verify_steps {
                // Would integrate with synapsed-verify
                Some(VerificationOutcome {
                    passed: true,
                    details: serde_json::json!({"verified": true}),
                    proof_id: if self.config.generate_proofs {
                        Some(Uuid::new_v4())
                    } else {
                        None
                    },
                    timestamp: Utc::now(),
                })
            } else {
                None
            }
        } else {
            None
        };
        
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        self.emit_event(
            if success { EventType::StepCompleted } else { EventType::StepFailed },
            serde_json::json!({
                "step_id": step.id,
                "duration_ms": duration_ms,
            })
        ).await;
        
        Ok(StepResult {
            success,
            output,
            error,
            duration_ms,
            verification,
        })
    }
    
    async fn emit_event(&self, event_type: EventType, data: serde_json::Value) {
        let event = IntentEvent {
            id: Uuid::new_v4(),
            intent_id: self.id,
            event_type,
            data,
            timestamp: Utc::now(),
        };
        
        // TODO: Emit event through circuit/channel when available
        // For now, just serialize the event
        let _ = serde_json::to_value(event).unwrap();
    }
}

/// Builder for creating intents fluently
pub struct IntentBuilder {
    intent: HierarchicalIntent,
}

impl IntentBuilder {
    /// Creates a new intent builder
    pub fn new(goal: impl Into<String>) -> Self {
        Self {
            intent: HierarchicalIntent::new(goal),
        }
    }
    
    /// Adds a description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.intent = self.intent.with_description(desc);
        self
    }
    
    /// Adds a step
    pub fn step(mut self, name: impl Into<String>, action: StepAction) -> Self {
        self.intent = self.intent.step(name, action);
        self
    }
    
    /// Adds a verified step
    pub fn verified_step(
        mut self,
        name: impl Into<String>,
        action: StepAction,
        verification: VerificationRequirement,
    ) -> Self {
        self.intent = self.intent.verified_step(name, action, verification);
        self
    }
    
    /// Adds a sub-intent
    pub fn sub_intent(mut self, sub: HierarchicalIntent) -> Self {
        self.intent = self.intent.sub_intent(sub);
        self
    }
    
    /// Sets priority
    pub fn priority(mut self, priority: Priority) -> Self {
        self.intent = self.intent.with_priority(priority);
        self
    }
    
    /// Builds the intent
    pub fn build(self) -> HierarchicalIntent {
        self.intent
    }
}

/// Execution plan for an intent
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// Intent ID
    pub intent_id: IntentId,
    /// Ordered steps to execute
    pub steps: Vec<Uuid>,
    /// Sub-intent plans
    pub sub_plans: Vec<ExecutionPlan>,
    /// Estimated total duration
    pub estimated_duration_ms: u64,
    /// Groups of steps that can run in parallel
    pub parallelizable_groups: Vec<Vec<Uuid>>,
}

/// Result from intent execution
#[derive(Debug, Clone)]
pub struct IntentResult {
    /// Intent ID
    pub intent_id: IntentId,
    /// Whether the intent succeeded
    pub success: bool,
    /// Results from each step
    pub step_results: Vec<StepResult>,
    /// Total duration
    pub duration_ms: u64,
    /// Verification proofs generated
    pub verification_proofs: Vec<Uuid>,
}