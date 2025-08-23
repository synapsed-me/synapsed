//! MCP Protocol Handler with JSON-RPC support

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    agent_spawner::AgentSpawner,
    intent_store::{IntentStore, IntentStatus},
    server::McpServer,
};

/// JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

/// JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<Value>,
}

/// JSON-RPC error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP Protocol handler
pub struct McpProtocolHandler {
    intent_store: Arc<RwLock<IntentStore>>,
    agent_spawner: Arc<AgentSpawner>,
}

impl McpProtocolHandler {
    /// Create a new protocol handler
    pub fn new(
        intent_store: Arc<RwLock<IntentStore>>,
        agent_spawner: Arc<AgentSpawner>,
    ) -> Self {
        Self {
            intent_store,
            agent_spawner,
        }
    }

    /// Handle a JSON-RPC request
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            // Intent management
            "intent/declare" => self.handle_intent_declare(request).await,
            "intent/update" => self.handle_intent_update(request).await,
            "intent/verify" => self.handle_intent_verify(request).await,
            "intent/get" => self.handle_intent_get(request).await,
            "intent/list" => self.handle_intent_list(request).await,
            
            // Agent management
            "agent/spawn" => self.handle_agent_spawn(request).await,
            "agent/status" => self.handle_agent_status(request).await,
            "agent/terminate" => self.handle_agent_terminate(request).await,
            
            // Context management
            "context/inject" => self.handle_context_inject(request).await,
            "context/get" => self.handle_context_get(request).await,
            
            // Trust management
            "trust/check" => self.handle_trust_check(request).await,
            "trust/update" => self.handle_trust_update(request).await,
            
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: None,
                }),
                id: request.id,
            },
        }
    }

    // Intent handlers
    async fn handle_intent_declare(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        if let Some(params) = request.params {
            if let Ok(intent_params) = serde_json::from_value::<IntentDeclareParams>(params) {
                // Create and store the intent
                let intent = synapsed_intent::HierarchicalIntent::new(
                    intent_params.description.clone()
                ).with_description(format!("Agent {} declared intent", intent_params.agent_id));
                
                let store = self.intent_store.read().await;
                let intent_id = match store.store_intent(&intent).await {
                    Ok(id) => id,
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32603,
                                message: format!("Failed to declare intent: {}", e),
                                data: None,
                            }),
                            id: request.id,
                        };
                    }
                };
                
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(serde_json::json!({
                        "intent_id": intent_id,
                        "status": "declared"
                    })),
                    error: None,
                    id: request.id,
                };
            }
        }
        
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
                data: None,
            }),
            id: request.id,
        }
    }

    async fn handle_intent_update(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        if let Some(params) = request.params {
            if let Ok(update_params) = serde_json::from_value::<IntentUpdateParams>(params) {
                let store = self.intent_store.read().await;
                
                // Convert status string to IntentStatus enum
                let status = match update_params.status.as_str() {
                    "completed" => IntentStatus::Completed,
                    "failed" => IntentStatus::Failed,
                    "executing" => IntentStatus::Executing,
                    "verified" => IntentStatus::Verified,
                    _ => IntentStatus::Declared,
                };
                
                if let Err(e) = store.update_status(&update_params.intent_id, status).await {
                    return JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32603,
                            message: format!("Failed to update intent: {}", e),
                            data: None,
                        }),
                        id: request.id,
                    };
                }
                
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(serde_json::json!({
                        "intent_id": update_params.intent_id,
                        "status": update_params.status
                    })),
                    error: None,
                    id: request.id,
                };
            }
        }
        
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
                data: None,
            }),
            id: request.id,
        }
    }

    async fn handle_intent_verify(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        if let Some(params) = request.params {
            if let Ok(verify_params) = serde_json::from_value::<IntentVerifyParams>(params) {
                let store = self.intent_store.read().await;
                
                // Get the intent and check its status
                match store.get_intent(&verify_params.intent_id).await {
                    Ok(Some(intent_record)) => {
                        // Create a verification result based on intent status
                        let verified = matches!(
                            intent_record.status,
                            IntentStatus::Completed | IntentStatus::Verified
                        );
                        
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: Some(serde_json::json!({
                                "intent_id": verify_params.intent_id,
                                "verified": verified,
                                "status": format!("{:?}", intent_record.status),
                                "created_at": intent_record.created_at,
                            })),
                            error: None,
                            id: request.id,
                        };
                    }
                    Ok(None) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32603,
                                message: "Intent not found".to_string(),
                                data: None,
                            }),
                            id: request.id,
                        };
                    }
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32603,
                                message: format!("Failed to verify intent: {}", e),
                                data: None,
                            }),
                            id: request.id,
                        };
                    }
                }
            }
        }
        
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
                data: None,
            }),
            id: request.id,
        }
    }

    async fn handle_intent_get(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        if let Some(params) = request.params {
            if let Ok(get_params) = serde_json::from_value::<IntentGetParams>(params) {
                let store = self.intent_store.read().await;
                
                match store.get_intent(&get_params.intent_id).await {
                    Ok(Some(intent)) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: Some(serde_json::to_value(intent).unwrap_or(serde_json::json!({}))),
                            error: None,
                            id: request.id,
                        };
                    }
                    Ok(None) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32603,
                                message: "Intent not found".to_string(),
                                data: None,
                            }),
                            id: request.id,
                        };
                    }
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32603,
                                message: format!("Failed to get intent: {}", e),
                                data: None,
                            }),
                            id: request.id,
                        };
                    }
                }
            }
        }
        
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
                data: None,
            }),
            id: request.id,
        }
    }

    async fn handle_intent_list(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let store = self.intent_store.read().await;
        
        match store.list_intents(None, None, None).await {
            Ok(intents) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::json!({ "intents": intents })),
                error: None,
                id: request.id,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: format!("Failed to list intents: {}", e),
                    data: None,
                }),
                id: request.id,
            },
        }
    }

    // Agent handlers
    async fn handle_agent_spawn(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        if let Some(params) = request.params {
            if let Ok(spawn_params) = serde_json::from_value::<AgentSpawnParams>(params) {
                match self.agent_spawner.spawn_agent(
                    spawn_params.agent_type,
                    spawn_params.config,
                    spawn_params.intent_id,
                ).await {
                    Ok(agent_id) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: Some(serde_json::json!({
                                "agent_id": agent_id,
                                "status": "spawned"
                            })),
                            error: None,
                            id: request.id,
                        };
                    }
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32603,
                                message: format!("Failed to spawn agent: {}", e),
                                data: None,
                            }),
                            id: request.id,
                        };
                    }
                }
            }
        }
        
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
                data: None,
            }),
            id: request.id,
        }
    }

    async fn handle_agent_status(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        if let Some(params) = request.params {
            if let Ok(status_params) = serde_json::from_value::<AgentStatusParams>(params) {
                match self.agent_spawner.get_agent_status(&status_params.agent_id).await {
                    Ok(status) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: Some(status),
                            error: None,
                            id: request.id,
                        };
                    }
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32603,
                                message: format!("Failed to get agent status: {}", e),
                                data: None,
                            }),
                            id: request.id,
                        };
                    }
                }
            }
        }
        
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
                data: None,
            }),
            id: request.id,
        }
    }

    async fn handle_agent_terminate(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        if let Some(params) = request.params {
            if let Ok(terminate_params) = serde_json::from_value::<AgentTerminateParams>(params) {
                match self.agent_spawner.terminate_agent(&terminate_params.agent_id).await {
                    Ok(_) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: Some(serde_json::json!({
                                "agent_id": terminate_params.agent_id,
                                "status": "terminated"
                            })),
                            error: None,
                            id: request.id,
                        };
                    }
                    Err(e) => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32603,
                                message: format!("Failed to terminate agent: {}", e),
                                data: None,
                            }),
                            id: request.id,
                        };
                    }
                }
            }
        }
        
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
                data: None,
            }),
            id: request.id,
        }
    }

    // Context handlers
    async fn handle_context_inject(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // TODO: Implement context injection
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({"status": "not_implemented"})),
            error: None,
            id: request.id,
        }
    }

    async fn handle_context_get(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // TODO: Implement context retrieval
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({"status": "not_implemented"})),
            error: None,
            id: request.id,
        }
    }

    // Trust handlers
    async fn handle_trust_check(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // TODO: Implement trust checking
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({"trust_level": 0.5})),
            error: None,
            id: request.id,
        }
    }

    async fn handle_trust_update(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // TODO: Implement trust updates
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({"status": "not_implemented"})),
            error: None,
            id: request.id,
        }
    }
}

// Parameter structures
#[derive(Debug, Deserialize)]
struct IntentDeclareParams {
    agent_id: String,
    description: String,
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct IntentUpdateParams {
    intent_id: String,
    status: String,
    result: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct IntentVerifyParams {
    intent_id: String,
}

#[derive(Debug, Deserialize)]
struct IntentGetParams {
    intent_id: String,
}

#[derive(Debug, Deserialize)]
struct AgentSpawnParams {
    agent_type: String,
    config: Option<Value>,
    intent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgentStatusParams {
    agent_id: String,
}

#[derive(Debug, Deserialize)]
struct AgentTerminateParams {
    agent_id: String,
}