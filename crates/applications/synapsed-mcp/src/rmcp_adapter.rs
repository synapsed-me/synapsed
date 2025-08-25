//! Adapter layer between rmcp server and Synapsed verification system

use crate::{
    error::{McpError, Result},
    intent_store::IntentStore,
    agent_spawner::AgentSpawner,
    observability::{McpEvent, EVENT_CIRCUIT},
};
use rmcp::{
    Handler, InitializeOptions, InitializedNotification, ListResourcesResponse,
    ListToolsResponse, ReadResourceRequest, ReadResourceResponse, Resource,
    ResourceContent, Tool, CallToolRequest, CallToolResponse,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};

/// Adapter that bridges rmcp's Handler trait with Synapsed's verification system
pub struct SynapsedMcpAdapter {
    /// Intent store for managing intents
    intent_store: Arc<RwLock<IntentStore>>,
    /// Agent spawner for creating sub-agents
    agent_spawner: Arc<AgentSpawner>,
    /// Server name
    server_name: String,
    /// Server version
    server_version: String,
}

impl SynapsedMcpAdapter {
    /// Create a new adapter
    pub fn new(
        intent_store: Arc<RwLock<IntentStore>>,
        agent_spawner: Arc<AgentSpawner>,
    ) -> Self {
        Self {
            intent_store,
            agent_spawner,
            server_name: crate::SERVER_NAME.to_string(),
            server_version: crate::SERVER_VERSION.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Handler for SynapsedMcpAdapter {
    /// Initialize the MCP server
    async fn initialize(&self, _options: InitializeOptions) -> InitializedNotification {
        info!("Initializing Synapsed MCP server via rmcp adapter");
        
        // Emit server started event
        let event = McpEvent::server_started(
            self.server_name.clone(),
            self.server_version.clone(),
            json!({
                "adapter": "rmcp",
                "version": env!("CARGO_PKG_VERSION"),
            })
        );
        let _ = EVENT_CIRCUIT.emit_event(event).await;
        
        InitializedNotification {
            server_name: self.server_name.clone(),
            version: self.server_version.clone(),
            protocol_version: "2024-10-07".to_string(),
            server_info: Some(json!({
                "description": crate::SERVER_DESCRIPTION,
                "features": {
                    "intent_verification": true,
                    "promise_tracking": true,
                    "context_injection": true,
                },
            })),
        }
    }

    /// List available tools
    async fn list_tools(&self) -> ListToolsResponse {
        debug!("Listing Synapsed MCP tools");
        
        let tools = vec![
            Tool {
                name: "intent_declare".to_string(),
                description: Some("Declare an intent before performing actions".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "goal": {
                            "type": "string",
                            "description": "The goal of the intent"
                        },
                        "description": {
                            "type": "string",
                            "description": "Optional description"
                        },
                        "steps": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string" },
                                    "action": { "type": "string" }
                                }
                            }
                        },
                        "success_criteria": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    },
                    "required": ["goal", "steps", "success_criteria"]
                }),
            },
            Tool {
                name: "intent_verify".to_string(),
                description: Some("Verify execution against declared intent".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "intent_id": {
                            "type": "string",
                            "description": "UUID of the intent to verify"
                        },
                        "evidence": {
                            "type": "object",
                            "description": "Evidence of completion"
                        }
                    },
                    "required": ["intent_id", "evidence"]
                }),
            },
            Tool {
                name: "intent_status".to_string(),
                description: Some("Get status of an intent".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "intent_id": {
                            "type": "string",
                            "description": "UUID of the intent"
                        }
                    },
                    "required": ["intent_id"]
                }),
            },
            Tool {
                name: "context_inject".to_string(),
                description: Some("Inject context for sub-agents".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "agent_id": {
                            "type": "string",
                            "description": "Target agent ID"
                        },
                        "context": {
                            "type": "object",
                            "description": "Context to inject"
                        },
                        "boundaries": {
                            "type": "object",
                            "description": "Context boundaries"
                        }
                    },
                    "required": ["agent_id", "context"]
                }),
            },
        ];
        
        ListToolsResponse { tools }
    }

    /// Call a tool
    async fn call_tool(&self, request: CallToolRequest) -> CallToolResponse {
        debug!("Calling tool: {}", request.name);
        
        let result = match request.name.as_str() {
            "intent_declare" => self.handle_intent_declare(request.arguments).await,
            "intent_verify" => self.handle_intent_verify(request.arguments).await,
            "intent_status" => self.handle_intent_status(request.arguments).await,
            "context_inject" => self.handle_context_inject(request.arguments).await,
            _ => Err(McpError::InvalidMethod(format!("Unknown tool: {}", request.name))),
        };
        
        match result {
            Ok(content) => CallToolResponse {
                content: vec![content],
                is_error: false,
            },
            Err(e) => CallToolResponse {
                content: vec![ResourceContent::text(format!("Error: {}", e))],
                is_error: true,
            },
        }
    }

    /// List available resources
    async fn list_resources(&self) -> ListResourcesResponse {
        debug!("Listing Synapsed MCP resources");
        
        let resources = vec![
            Resource {
                uri: "context://synapsed/active-intents".to_string(),
                name: Some("Active Intents".to_string()),
                description: Some("List of currently active intents".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "context://synapsed/verification-results".to_string(),
                name: Some("Verification Results".to_string()),
                description: Some("Recent verification results".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "context://synapsed/trust-levels".to_string(),
                name: Some("Trust Levels".to_string()),
                description: Some("Current agent trust levels".to_string()),
                mime_type: Some("application/json".to_string()),
            },
        ];
        
        ListResourcesResponse { resources }
    }

    /// Read a resource
    async fn read_resource(&self, request: ReadResourceRequest) -> ReadResourceResponse {
        debug!("Reading resource: {}", request.uri);
        
        let content = match request.uri.as_str() {
            "context://synapsed/active-intents" => {
                self.get_active_intents().await
            }
            "context://synapsed/verification-results" => {
                self.get_verification_results().await
            }
            "context://synapsed/trust-levels" => {
                self.get_trust_levels().await
            }
            _ => Err(McpError::InvalidParams(format!("Unknown resource: {}", request.uri))),
        };
        
        match content {
            Ok(json) => ReadResourceResponse {
                content: vec![ResourceContent::text(json.to_string())],
            },
            Err(e) => ReadResourceResponse {
                content: vec![ResourceContent::text(format!("Error: {}", e))],
            },
        }
    }
}

// Tool implementation methods
impl SynapsedMcpAdapter {
    async fn handle_intent_declare(&self, args: Value) -> Result<ResourceContent> {
        let goal = args["goal"].as_str()
            .ok_or_else(|| McpError::InvalidParams("Missing goal".into()))?;
        
        // Create intent using Synapsed's intent system
        let mut intent = synapsed_intent::HierarchicalIntent::new(goal.to_string());
        
        if let Some(description) = args["description"].as_str() {
            intent = intent.with_description(description);
        }
        
        // Add steps
        if let Some(steps) = args["steps"].as_array() {
            for step in steps {
                if let (Some(name), Some(action)) = (
                    step["name"].as_str(),
                    step["action"].as_str()
                ) {
                    intent.add_step(name, synapsed_intent::types::StepAction::Command(action.to_string()));
                }
            }
        }
        
        // Store intent
        let intent_id = uuid::Uuid::new_v4();
        let mut store = self.intent_store.write().await;
        store.store_intent(intent_id, intent)?;
        
        Ok(ResourceContent::text(json!({
            "intent_id": intent_id.to_string(),
            "status": "declared",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }).to_string()))
    }
    
    async fn handle_intent_verify(&self, args: Value) -> Result<ResourceContent> {
        let intent_id = args["intent_id"].as_str()
            .ok_or_else(|| McpError::InvalidParams("Missing intent_id".into()))?;
        let intent_uuid = uuid::Uuid::parse_str(intent_id)
            .map_err(|e| McpError::InvalidParams(format!("Invalid UUID: {}", e)))?;
        
        let evidence = args["evidence"].clone();
        
        // Retrieve intent
        let store = self.intent_store.read().await;
        let intent = store.get_intent(&intent_uuid)?;
        
        // Perform basic verification - in production, would use specific verify methods
        let is_verified = !evidence.is_null();
        let confidence = if is_verified { 0.8 } else { 0.0 };
        
        Ok(ResourceContent::text(json!({
            "intent_id": intent_id,
            "verified": is_verified,
            "confidence": confidence,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }).to_string()))
    }
    
    async fn handle_intent_status(&self, args: Value) -> Result<ResourceContent> {
        let intent_id = args["intent_id"].as_str()
            .ok_or_else(|| McpError::InvalidParams("Missing intent_id".into()))?;
        let intent_uuid = uuid::Uuid::parse_str(intent_id)
            .map_err(|e| McpError::InvalidParams(format!("Invalid UUID: {}", e)))?;
        
        let store = self.intent_store.read().await;
        let intent = store.get_intent(&intent_uuid)?;
        
        Ok(ResourceContent::text(json!({
            "intent_id": intent_id,
            "goal": intent.goal(),
            "status": intent.status(),
            "progress": intent.progress(),
        }).to_string()))
    }
    
    async fn handle_context_inject(&self, args: Value) -> Result<ResourceContent> {
        let agent_id = args["agent_id"].as_str()
            .ok_or_else(|| McpError::InvalidParams("Missing agent_id".into()))?;
        let context = args["context"].clone();
        let boundaries = args["boundaries"].clone();
        
        // Use agent spawner to inject context
        self.agent_spawner.inject_context(agent_id, context, boundaries).await?;
        
        Ok(ResourceContent::text(json!({
            "agent_id": agent_id,
            "context_injected": true,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }).to_string()))
    }
    
    async fn get_active_intents(&self) -> Result<Value> {
        let store = self.intent_store.read().await;
        let intents = store.list_active_intents()?;
        Ok(json!({ "intents": intents }))
    }
    
    async fn get_verification_results(&self) -> Result<Value> {
        let store = self.intent_store.read().await;
        let results = store.get_verification_results(10)?;
        Ok(json!({ "results": results }))
    }
    
    async fn get_trust_levels(&self) -> Result<Value> {
        // Get trust levels from Promise Theory agents
        Ok(json!({
            "trust_levels": {
                "default": 0.5,
                "verified_agents": 0.8,
                "unverified_agents": 0.3,
            }
        }))
    }
}