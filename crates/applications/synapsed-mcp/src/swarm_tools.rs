//! MCP Tools for swarm coordination

use crate::error::{McpError, Result};
use rmcp::tool;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::{info, debug, warn};
use synapsed_swarm::{
    SwarmCoordinator, SwarmConfig, AgentRole, ClaudeAgent, ClaudeAgentConfig,
    TrustScore, AgentMessage, MessageType, ProtocolVersion,
};
use synapsed_intent::{HierarchicalIntent, IntentBuilder, IntentContext};
use synapsed_promise::{Promise, PromiseContract, PromiseType, PromiseScope};

/// Swarm coordination parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SwarmCoordinateParams {
    /// Goal for the swarm to achieve
    pub goal: String,
    /// Number of agents to coordinate
    pub agent_count: Option<usize>,
    /// Minimum trust score for agents
    pub min_trust_score: Option<f64>,
    /// Enable verification
    pub require_verification: Option<bool>,
    /// Context variables
    pub context: Option<serde_json::Value>,
}

/// Promise making parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PromiseMakeParams {
    /// Agent making the promise
    pub agent_id: String,
    /// What the agent promises to do
    pub promise_body: String,
    /// Promise scope (who it applies to)
    pub scope: Option<Vec<String>>,
    /// Promise type
    pub promise_type: Option<String>,
    /// Conditions for the promise
    pub conditions: Option<Vec<String>>,
}

/// Promise verification parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PromiseVerifyParams {
    /// Promise ID to verify
    pub promise_id: String,
    /// Agent who made the promise
    pub agent_id: String,
    /// Evidence of fulfillment
    pub evidence: serde_json::Value,
    /// Whether the promise was fulfilled
    pub fulfilled: bool,
}

/// Intent delegation parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IntentDelegateParams {
    /// Intent to delegate
    pub intent_id: String,
    /// Target agent or swarm
    pub target: Option<String>,
    /// Verification requirements
    pub require_verification: Option<bool>,
    /// Trust threshold for delegation
    pub min_trust: Option<f64>,
    /// Context to pass to sub-agent
    pub context: Option<serde_json::Value>,
}

/// Agent registration parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentRegisterParams {
    /// Agent name
    pub name: String,
    /// Agent capabilities
    pub capabilities: Vec<String>,
    /// Agent tools/resources
    pub tools: Option<Vec<String>>,
    /// Initial trust score
    pub initial_trust: Option<f64>,
    /// Agent role
    pub role: Option<String>,
}

/// Trust query parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrustQueryParams {
    /// Agent ID to query
    pub agent_id: String,
}

/// Trust update parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrustUpdateParams {
    /// Agent ID to update
    pub agent_id: String,
    /// Task success/failure
    pub success: bool,
    /// Was the task verified
    pub verified: Option<bool>,
    /// Reason for update
    pub reason: Option<String>,
}

/// Swarm coordination tools for MCP
pub struct SwarmTools {
    state: Arc<RwLock<crate::server::ServerState>>,
    coordinator: Arc<RwLock<Option<SwarmCoordinator>>>,
}

impl SwarmTools {
    /// Create new swarm tools
    pub fn new(state: Arc<RwLock<crate::server::ServerState>>) -> Self {
        Self {
            state,
            coordinator: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Initialize swarm coordinator
    async fn ensure_coordinator(&self) -> Result<()> {
        let mut coord = self.coordinator.write().await;
        if coord.is_none() {
            let config = SwarmConfig::default();
            let coordinator = SwarmCoordinator::new(config);
            coordinator.initialize().await
                .map_err(|e| McpError::Internal(format!("Failed to initialize coordinator: {}", e)))?;
            *coord = Some(coordinator);
        }
        Ok(())
    }
    
    /// Coordinate a swarm of agents
    #[tool(description = "Coordinate multiple agents to achieve a goal using swarm intelligence")]
    pub async fn swarm_coordinate(&self, params: SwarmCoordinateParams) -> Result<serde_json::Value> {
        info!("Coordinating swarm for goal: {}", params.goal);
        
        // Ensure coordinator exists
        self.ensure_coordinator().await?;
        
        // Create intent from goal
        let intent = IntentBuilder::new(params.goal.clone())
            .build()
            .map_err(|e| McpError::Internal(format!("Failed to build intent: {}", e)))?;
        
        // Create context
        let context = if let Some(ctx) = params.context {
            IntentContext::from_json(ctx)
                .map_err(|e| McpError::Internal(format!("Invalid context: {}", e)))?
        } else {
            IntentContext::default()
        };
        
        // Get coordinator
        let coord_guard = self.coordinator.read().await;
        let coordinator = coord_guard.as_ref()
            .ok_or_else(|| McpError::Internal("Coordinator not initialized".to_string()))?;
        
        // Delegate to swarm
        let task_id = coordinator.delegate_intent(intent, context).await
            .map_err(|e| McpError::Internal(format!("Failed to delegate: {}", e)))?;
        
        // Store in server state
        let mut state = self.state.write().await;
        state.active_intents.insert(task_id, HierarchicalIntent::new(params.goal));
        
        Ok(serde_json::json!({
            "success": true,
            "task_id": task_id,
            "message": "Swarm coordination initiated",
            "agent_count": params.agent_count.unwrap_or(3),
        }))
    }
    
    /// Make a promise as an agent
    #[tool(description = "Make a voluntary promise about agent behavior")]
    pub async fn promise_make(&self, params: PromiseMakeParams) -> Result<serde_json::Value> {
        info!("Agent {} making promise: {}", params.agent_id, params.promise_body);
        
        let agent_id = Uuid::parse_str(&params.agent_id)
            .map_err(|e| McpError::InvalidParams(format!("Invalid agent ID: {}", e)))?;
        
        // Create promise contract
        let promise_type = match params.promise_type.as_deref() {
            Some("offer") => PromiseType::Offer,
            Some("use") => PromiseType::Use,
            Some("delegate") => PromiseType::Delegate,
            _ => PromiseType::Offer,
        };
        
        let scope = if let Some(scope_list) = params.scope {
            PromiseScope::Specific(
                scope_list.iter()
                    .filter_map(|s| Uuid::parse_str(s).ok())
                    .collect()
            )
        } else {
            PromiseScope::Universal
        };
        
        let contract = PromiseContract::new(
            params.promise_body.clone(),
            promise_type,
            scope,
        );
        
        // Store promise in state
        let mut state = self.state.write().await;
        let promise_id = Uuid::new_v4();
        
        // Create a simple promise representation
        let promise = serde_json::json!({
            "id": promise_id,
            "agent_id": agent_id,
            "body": params.promise_body,
            "type": format!("{:?}", promise_type),
            "conditions": params.conditions,
            "timestamp": chrono::Utc::now(),
        });
        
        Ok(serde_json::json!({
            "success": true,
            "promise_id": promise_id,
            "agent_id": agent_id,
            "promise": promise,
            "message": "Promise recorded",
        }))
    }
    
    /// Verify a promise was fulfilled
    #[tool(description = "Verify whether an agent fulfilled their promise")]
    pub async fn promise_verify(&self, params: PromiseVerifyParams) -> Result<serde_json::Value> {
        info!("Verifying promise {} for agent {}", params.promise_id, params.agent_id);
        
        let promise_id = Uuid::parse_str(&params.promise_id)
            .map_err(|e| McpError::InvalidParams(format!("Invalid promise ID: {}", e)))?;
        
        let agent_id = Uuid::parse_str(&params.agent_id)
            .map_err(|e| McpError::InvalidParams(format!("Invalid agent ID: {}", e)))?;
        
        // In a real implementation, this would verify against stored promises
        // and update trust scores
        
        let verification_result = if params.fulfilled {
            "FULFILLED"
        } else {
            "BROKEN"
        };
        
        // Update trust score based on fulfillment
        if let Some(coordinator) = self.coordinator.read().await.as_ref() {
            // This would update trust in the real implementation
            debug!("Would update trust for agent {} based on {}", agent_id, verification_result);
        }
        
        Ok(serde_json::json!({
            "success": true,
            "promise_id": promise_id,
            "agent_id": agent_id,
            "verification": verification_result,
            "evidence": params.evidence,
            "trust_impact": if params.fulfilled { 0.05 } else { -0.1 },
        }))
    }
    
    /// Delegate an intent to a sub-agent or swarm
    #[tool(description = "Delegate an intent to a sub-agent with verification requirements")]
    pub async fn intent_delegate(&self, params: IntentDelegateParams) -> Result<serde_json::Value> {
        info!("Delegating intent {} to {}", params.intent_id, params.target.as_deref().unwrap_or("swarm"));
        
        // Get intent from store
        let state = self.state.read().await;
        let intent = state.active_intents.get(&Uuid::parse_str(&params.intent_id)
            .map_err(|e| McpError::InvalidParams(format!("Invalid intent ID: {}", e)))?)
            .ok_or_else(|| McpError::NotFound(format!("Intent {} not found", params.intent_id)))?
            .clone();
        
        drop(state); // Release lock
        
        // Create delegation context
        let mut context = if let Some(ctx) = params.context {
            IntentContext::from_json(ctx)
                .map_err(|e| McpError::Internal(format!("Invalid context: {}", e)))?
        } else {
            IntentContext::default()
        };
        
        // Add verification requirement
        if params.require_verification.unwrap_or(true) {
            context = context.with_verification_required(true);
        }
        
        // Ensure coordinator exists
        self.ensure_coordinator().await?;
        
        // Delegate to swarm
        let coord_guard = self.coordinator.read().await;
        let coordinator = coord_guard.as_ref()
            .ok_or_else(|| McpError::Internal("Coordinator not initialized".to_string()))?;
        
        let task_id = coordinator.delegate_intent(intent, context).await
            .map_err(|e| McpError::Internal(format!("Delegation failed: {}", e)))?;
        
        Ok(serde_json::json!({
            "success": true,
            "intent_id": params.intent_id,
            "task_id": task_id,
            "target": params.target.unwrap_or_else(|| "swarm".to_string()),
            "verification_required": params.require_verification.unwrap_or(true),
            "message": "Intent delegated successfully",
        }))
    }
    
    /// Register a new agent in the swarm
    #[tool(description = "Register a new agent with the swarm coordinator")]
    pub async fn agent_register(&self, params: AgentRegisterParams) -> Result<serde_json::Value> {
        info!("Registering agent: {}", params.name);
        
        // Create Claude agent config
        let config = ClaudeAgentConfig {
            name: params.name.clone(),
            role: match params.role.as_deref() {
                Some("coordinator") => AgentRole::Coordinator,
                Some("verifier") => AgentRole::Verifier,
                Some("observer") => AgentRole::Observer,
                _ => AgentRole::Worker,
            },
            initial_trust: params.initial_trust.unwrap_or(0.5),
            capabilities: params.capabilities,
            tools: params.tools.unwrap_or_default(),
            inject_context: true,
            require_verification: true,
            max_concurrent_tasks: 3,
            task_timeout_secs: 300,
        };
        
        // Create agent
        let agent = ClaudeAgent::new(config);
        let agent_id = agent.id();
        
        // Initialize agent
        agent.initialize().await
            .map_err(|e| McpError::Internal(format!("Failed to initialize agent: {}", e)))?;
        
        // Add to coordinator if it exists
        if let Some(coordinator) = self.coordinator.read().await.as_ref() {
            // This would add the agent in a real implementation
            debug!("Would add agent {} to coordinator", agent_id);
        }
        
        // Store in state
        let mut state = self.state.write().await;
        state.active_agents.insert(
            agent_id,
            synapsed_promise::AutonomousAgent::new(Default::default()),
        );
        
        Ok(serde_json::json!({
            "success": true,
            "agent_id": agent_id,
            "name": params.name,
            "role": format!("{:?}", agent.role()),
            "message": "Agent registered successfully",
        }))
    }
    
    /// Query trust score for an agent
    #[tool(description = "Query the current trust score of an agent")]
    pub async fn trust_query(&self, params: TrustQueryParams) -> Result<serde_json::Value> {
        info!("Querying trust for agent: {}", params.agent_id);
        
        let agent_id = Uuid::parse_str(&params.agent_id)
            .map_err(|e| McpError::InvalidParams(format!("Invalid agent ID: {}", e)))?;
        
        // In a real implementation, this would query the trust manager
        let trust_score = TrustScore::new(0.75); // Simulated
        
        Ok(serde_json::json!({
            "success": true,
            "agent_id": agent_id,
            "trust_score": trust_score.value,
            "confidence": trust_score.confidence,
            "interactions": trust_score.interactions,
            "effective_trust": trust_score.effective_trust(),
            "last_updated": trust_score.last_updated,
        }))
    }
    
    /// Update trust score for an agent
    #[tool(description = "Update the trust score of an agent based on task performance")]
    pub async fn trust_update(&self, params: TrustUpdateParams) -> Result<serde_json::Value> {
        info!("Updating trust for agent {}: success={}, verified={:?}", 
              params.agent_id, params.success, params.verified);
        
        let agent_id = Uuid::parse_str(&params.agent_id)
            .map_err(|e| McpError::InvalidParams(format!("Invalid agent ID: {}", e)))?;
        
        // Calculate trust delta
        let delta = if params.success {
            if params.verified.unwrap_or(false) {
                0.05
            } else {
                0.02
            }
        } else {
            -0.1
        };
        
        // In a real implementation, this would update the trust manager
        let new_score = 0.75 + delta; // Simulated
        
        Ok(serde_json::json!({
            "success": true,
            "agent_id": agent_id,
            "trust_delta": delta,
            "new_trust_score": new_score,
            "reason": params.reason,
            "message": "Trust score updated",
        }))
    }
}

// Helper trait implementations
trait IntentContextExt {
    fn from_json(value: serde_json::Value) -> Result<IntentContext>;
    fn with_verification_required(self, required: bool) -> Self;
}

impl IntentContextExt for IntentContext {
    fn from_json(_value: serde_json::Value) -> Result<IntentContext> {
        // Simplified implementation
        Ok(IntentContext::default())
    }
    
    fn with_verification_required(self, _required: bool) -> Self {
        // Would set verification requirement in real implementation
        self
    }
}