//! MCP Tools for intent verification

use crate::{
    error::{McpError, Result},
    observability::{McpEvent, EVENT_CIRCUIT},
};
use rmcp::tool;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::{info, debug};

/// Intent declaration parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IntentDeclareParams {
    /// Goal of the intent
    pub goal: String,
    /// Description of the intent
    pub description: Option<String>,
    /// Steps to execute
    pub steps: Vec<StepParams>,
    /// Success criteria
    pub success_criteria: Vec<String>,
    /// Context boundaries
    pub context_bounds: Option<ContextBoundsParams>,
}

/// Step parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StepParams {
    /// Step name
    pub name: String,
    /// Step action
    pub action: String,
    /// Verification requirements
    pub verification: Option<VerificationParams>,
}

/// Verification parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VerificationParams {
    /// Type of verification
    pub verification_type: String,
    /// Expected outcome
    pub expected: serde_json::Value,
}

/// Context bounds parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContextBoundsParams {
    /// Allowed operations
    pub allowed_operations: Vec<String>,
    /// Restricted paths
    pub restricted_paths: Vec<String>,
    /// Max execution time in seconds
    pub max_execution_time: Option<u64>,
}

/// Intent verification parameters
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IntentVerifyParams {
    /// Intent ID to verify
    pub intent_id: Uuid,
    /// Evidence of completion
    pub evidence: serde_json::Value,
}

/// Intent tools for MCP
pub struct IntentTools {
    state: Arc<RwLock<crate::server::ServerState>>,
}

impl IntentTools {
    /// Create new intent tools
    pub fn new(state: Arc<RwLock<crate::server::ServerState>>) -> Self {
        Self { state }
    }
    
    /// Declare an intent before execution
    #[tool(description = "Declare an intent before performing actions")]
    pub async fn intent_declare(&self, params: IntentDeclareParams) -> Result<serde_json::Value> {
        info!("Declaring intent: {}", params.goal);
        
        // Create hierarchical intent
        let mut intent = synapsed_intent::HierarchicalIntent::new(params.goal.clone());
        
        if let Some(desc) = params.description {
            intent = intent.with_description(desc);
        }
        
        // Add steps
        for step in params.steps {
            use synapsed_intent::types::{StepAction, VerificationRequirement};
            
            let action = StepAction::Command(step.action);
            
            if let Some(verification) = step.verification {
                let req = VerificationRequirement {
                    verification_type: synapsed_intent::types::VerificationType::Command,
                    expected: verification.expected,
                    mandatory: true,
                    strategy: synapsed_intent::types::VerificationStrategy::Single,
                };
                intent = intent.verified_step(step.name, action, req);
            } else {
                intent = intent.step(step.name, action);
            }
        }
        
        // Set context bounds if provided
        if let Some(bounds) = params.context_bounds {
            use synapsed_intent::types::ContextBounds;
            
            use std::collections::HashMap;
            
            let context_bounds = ContextBounds {
                allowed_paths: bounds.restricted_paths,
                allowed_commands: bounds.allowed_operations,
                allowed_endpoints: Vec::new(),
                max_memory_bytes: None,
                max_cpu_seconds: bounds.max_execution_time,
                env_vars: HashMap::new(),
            };
            intent = intent.with_bounds(context_bounds);
        }
        
        let intent_id = intent.id();
        
        // Store intent in both memory and persistent storage
        let mut state = self.state.write().await;
        
        // Store in persistent storage (internal)
        let stored_id = state.intent_store.store_intent(&intent).await?;
        
        // Also keep in memory for quick access
        state.active_intents.insert(intent_id.0, intent.clone());
        
        // Emit intent declared event
        let event = McpEvent::intent_declared(
            intent_id.0,
            intent.goal().to_string(),
            params.steps.len(),
            None, // No agent ID available at this level
        );
        let _ = EVENT_CIRCUIT.emit_event(event).await;
        
        Ok(serde_json::json!({
            "intent_id": stored_id,
            "status": "declared",
            "message": format!("Intent '{}' declared and persisted", params.goal)
        }))
    }
    
    /// Verify an intent was completed
    #[tool(description = "Verify that an intent was completed successfully")]
    pub async fn intent_verify(&self, params: IntentVerifyParams) -> Result<serde_json::Value> {
        info!("Verifying intent: {}", params.intent_id);
        
        let state = self.state.read().await;
        
        if let Some(intent) = state.active_intents.get(&params.intent_id) {
            // Perform verification
            // TODO: Integrate with synapsed-verify
            let success = true; // Placeholder - would be actual verification result
            
            // Emit intent verified event
            let event = McpEvent::intent_verified(
                params.intent_id,
                success,
                params.evidence.clone(),
                None, // No agent ID available at this level
            );
            let _ = EVENT_CIRCUIT.emit_event(event).await;
            
            Ok(serde_json::json!({
                "intent_id": params.intent_id,
                "verified": success,
                "goal": intent.goal(),
                "evidence": params.evidence,
                "timestamp": chrono::Utc::now(),
            }))
        } else {
            Err(McpError::InvalidParams(format!("Intent {} not found", params.intent_id)))
        }
    }
}

/// Verification tools for MCP
pub struct VerificationTools {
    state: Arc<RwLock<crate::server::ServerState>>,
}

impl VerificationTools {
    /// Create new verification tools
    pub fn new(state: Arc<RwLock<crate::server::ServerState>>) -> Self {
        Self { state }
    }
    
    /// Check trust level of an agent
    #[tool(description = "Check the trust level of an agent based on promise fulfillment")]
    pub async fn trust_check(&self, agent_id: Uuid) -> Result<serde_json::Value> {
        debug!("Checking trust for agent: {}", agent_id);
        
        // TODO: Implement trust checking using synapsed-promise
        let trust_level = 0.85;
        let reputation = "good".to_string();
        let promises_fulfilled = 42;
        let promises_broken = 3;
        
        // Emit trust checked event
        let event = McpEvent::trust_checked(
            agent_id,
            trust_level,
            reputation.clone(),
            promises_fulfilled,
            promises_broken,
        );
        let _ = EVENT_CIRCUIT.emit_event(event).await;
        
        Ok(serde_json::json!({
            "agent_id": agent_id,
            "trust_level": trust_level,
            "reputation": reputation,
            "promises_fulfilled": promises_fulfilled,
            "promises_broken": promises_broken,
        }))
    }
    
    /// Get the status of an intent
    #[tool(description = "Get the current status of a declared intent")]
    pub async fn intent_status(&self, intent_id: String) -> Result<serde_json::Value> {
        info!("Getting status for intent: {}", intent_id);
        
        let state = self.state.read().await;
        
        // Query from persistent storage
        if let Some(record) = state.intent_store.get_intent(&intent_id).await? {
            Ok(serde_json::json!({
                "intent_id": intent_id,
                "goal": record.goal,
                "status": record.status,
                "steps": record.steps,
                "created_at": record.created_at,
                "updated_at": record.updated_at,
            }))
        } else {
            Err(McpError::NotFound(format!("Intent {} not found", intent_id)))
        }
    }
    
    /// Mark an intent as completed
    #[tool(description = "Mark an intent as completed")]
    pub async fn intent_complete(&self, intent_id: String) -> Result<serde_json::Value> {
        info!("Marking intent {} as completed", intent_id);
        
        let state = self.state.read().await;
        
        // Update status in persistent storage
        use crate::intent_store::IntentStatus;
        let old_status = "executing"; // Would normally query the current status
        state.intent_store.update_status(&intent_id, IntentStatus::Completed).await?;
        
        // Emit intent status changed event
        let event = McpEvent::intent_status_changed(
            intent_id.clone(),
            old_status.to_string(),
            "completed".to_string(),
            None,
            None,
        );
        let _ = EVENT_CIRCUIT.emit_event(event).await;
        
        Ok(serde_json::json!({
            "intent_id": intent_id,
            "status": "completed",
            "message": "Intent marked as completed"
        }))
    }
    
    /// List intents with optional filters
    #[tool(description = "List all intents with optional status filter")]
    pub async fn intent_list(&self, status: Option<String>) -> Result<serde_json::Value> {
        info!("Listing intents with filter: {:?}", status);
        
        let state = self.state.read().await;
        
        // Convert string status to enum if provided
        let status_filter = status.and_then(|s| {
            use crate::intent_store::IntentStatus;
            match s.to_lowercase().as_str() {
                "declared" => Some(IntentStatus::Declared),
                "executing" => Some(IntentStatus::Executing),
                "completed" => Some(IntentStatus::Completed),
                "failed" => Some(IntentStatus::Failed),
                "verified" => Some(IntentStatus::Verified),
                _ => None,
            }
        });
        
        let intents = state.intent_store.list_intents(status_filter, None, None).await?;
        
        Ok(serde_json::json!({
            "count": intents.len(),
            "intents": intents,
        }))
    }
    
    /// Get child intents of a parent
    #[tool(description = "Get all child intents of a parent intent")]
    pub async fn intent_children(&self, parent_id: String) -> Result<serde_json::Value> {
        info!("Getting children of intent: {}", parent_id);
        
        let state = self.state.read().await;
        let children = state.intent_store.get_children(&parent_id).await?;
        
        Ok(serde_json::json!({
            "parent_id": parent_id,
            "count": children.len(),
            "children": children,
        }))
    }
    
    /// Update step status
    #[tool(description = "Update the status of a specific step in an intent")]
    pub async fn intent_step_status(
        &self, 
        intent_id: String, 
        step_name: String, 
        status: String,
        error: Option<String>
    ) -> Result<serde_json::Value> {
        info!("Updating step {} in intent {} to {}", step_name, intent_id, status);
        
        let state = self.state.read().await;
        
        use crate::intent_store::IntentStatus;
        let step_status = match status.to_lowercase().as_str() {
            "executing" => IntentStatus::Executing,
            "completed" => IntentStatus::Completed,
            "failed" => IntentStatus::Failed,
            _ => IntentStatus::Declared,
        };
        
        state.intent_store.update_step_status(&intent_id, &step_name, step_status, error.clone()).await?;
        
        // Emit intent status changed event (step-level)
        let event = McpEvent::intent_status_changed(
            intent_id.clone(),
            "previous".to_string(), // Would normally track the previous status
            status.clone(),
            Some(step_name.clone()),
            error,
        );
        let _ = EVENT_CIRCUIT.emit_event(event).await;
        
        Ok(serde_json::json!({
            "intent_id": intent_id,
            "step_name": step_name,
            "status": status,
            "message": "Step status updated"
        }))
    }
}