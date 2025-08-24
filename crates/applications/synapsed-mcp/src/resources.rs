//! MCP Resources for context management

use crate::{
    error::{McpError, Result},
    observability::{McpEvent, EVENT_CIRCUIT},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::debug;

/// Context resources for MCP
pub struct ContextResources {
    state: Arc<RwLock<crate::server::ServerState>>,
}

impl ContextResources {
    /// Create new context resources
    pub fn new(state: Arc<RwLock<crate::server::ServerState>>) -> Self {
        Self { state }
    }
    
    /// Get current context for an intent
    pub async fn get_context(&self, intent_id: Uuid) -> Result<serde_json::Value> {
        debug!("Getting context for intent: {}", intent_id);
        
        let state = self.state.read().await;
        
        if let Some(intent) = state.active_intents.get(&intent_id) {
            Ok(serde_json::json!({
                "intent_id": intent_id,
                "goal": intent.goal(),
                "status": format!("{:?}", intent.status().await),
                "context": {
                    "boundaries": "active",
                    "allowed_operations": ["read", "write", "execute"],
                    "restricted_paths": [],
                }
            }))
        } else {
            Err(McpError::InvalidParams(format!("Intent {} not found", intent_id)))
        }
    }
    
    /// Inject context for a sub-agent
    pub async fn inject_context(&self, parent_intent_id: Uuid, sub_agent_id: Uuid) -> Result<serde_json::Value> {
        debug!("Injecting context from intent {} to agent {}", parent_intent_id, sub_agent_id);
        
        // TODO: Implement context injection using synapsed-intent context management
        let context_size = 150; // Placeholder - would be actual context size
        let success = true; // Placeholder - would be actual injection result
        
        // Emit context injection event
        let event = McpEvent::context_injected(
            Some(parent_intent_id.to_string()),
            sub_agent_id.to_string(),
            context_size,
            success,
        );
        let _ = EVENT_CIRCUIT.emit_event(event).await;
        
        Ok(serde_json::json!({
            "parent_intent": parent_intent_id,
            "sub_agent": sub_agent_id,
            "context_injected": success,
            "context_size": context_size,
            "boundaries": {
                "inherited": true,
                "additional_restrictions": []
            }
        }))
    }
}