//! MCP Client implementation with encrypted HTTP transport

use crate::{
    error::{McpError, Result},
    protocol::{JsonRpcRequest, JsonRpcResponse},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{RwLock, oneshot};
use std::collections::HashMap;
use tracing::{info, debug, error, warn};
use uuid::Uuid;

/// MCP Client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Server URL (e.g., "https://localhost:3000")
    pub server_url: String,
    /// Client name for identification
    pub client_name: String,
    /// Client version
    pub client_version: String,
    /// Use TLS encryption
    pub use_tls: bool,
    /// Allow self-signed certificates (for development)
    pub allow_self_signed: bool,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Connection pool size
    pub pool_size: usize,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_url: "https://localhost:3000".to_string(),
            client_name: "synapsed-mcp-client".to_string(),
            client_version: "1.0.0".to_string(),
            use_tls: true,
            allow_self_signed: false,
            request_timeout_secs: 30,
            max_retries: 3,
            pool_size: 4,
        }
    }
}

/// Pending request tracking
struct PendingRequest {
    tx: oneshot::Sender<JsonRpcResponse>,
    method: String,
    timestamp: std::time::Instant,
}

/// MCP Client for intent declaration and verification
pub struct McpClient {
    config: ClientConfig,
    transport: Arc<crate::client_transport::HttpTransport>,
    request_id: AtomicU64,
    pending_requests: Arc<RwLock<HashMap<u64, PendingRequest>>>,
}

impl McpClient {
    /// Create a new MCP client
    pub async fn new(config: ClientConfig) -> Result<Self> {
        info!("Creating MCP client for {}", config.server_url);
        
        let transport = Arc::new(
            crate::client_transport::HttpTransport::new(&config).await?
        );
        
        Ok(Self {
            config,
            transport,
            request_id: AtomicU64::new(1),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Connect to the MCP server
    pub async fn connect(&self) -> Result<()> {
        info!("Connecting to MCP server at {}", self.config.server_url);
        
        // Send initialization request
        let init_result = self.initialize().await?;
        info!("Connected to MCP server: {:?}", init_result);
        
        Ok(())
    }
    
    /// Initialize connection with server
    async fn initialize(&self) -> Result<serde_json::Value> {
        let params = serde_json::json!({
            "client_name": self.config.client_name,
            "client_version": self.config.client_version,
            "capabilities": {
                "intent_declaration": true,
                "verification": true,
                "promise_theory": true,
                "context_injection": true,
            }
        });
        
        self.call_method("initialize", params).await
    }
    
    /// Call an RPC method
    async fn call_method(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(id.into())),
            method: method.to_string(),
            params: Some(params),
        };
        
        debug!("Sending request {}: {}", id, method);
        
        // Create oneshot channel for response
        let (tx, rx) = oneshot::channel();
        
        // Store pending request
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(id, PendingRequest {
                tx,
                method: method.to_string(),
                timestamp: std::time::Instant::now(),
            });
        }
        
        // Send request via transport
        self.transport.send_request(request).await?;
        
        // Wait for response with timeout
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.request_timeout_secs),
            rx
        ).await
            .map_err(|_| McpError::Timeout(format!("Request {} timed out", id)))?
            .map_err(|_| McpError::Transport("Response channel closed".to_string()))?;
        
        // Check for error
        if let Some(error) = response.error {
            return Err(McpError::RpcError {
                code: error.code,
                message: error.message,
                data: error.data,
            });
        }
        
        // Return result
        response.result.ok_or_else(|| 
            McpError::InvalidResponse("No result in response".to_string())
        )
    }
    
    /// Process incoming response
    pub(crate) async fn handle_response(&self, response: JsonRpcResponse) {
        if let Some(id) = &response.id {
            if let Some(id_num) = id.as_u64() {
                let mut pending = self.pending_requests.write().await;
                if let Some(request) = pending.remove(&id_num) {
                    debug!("Received response for request {}: {}", id_num, request.method);
                    let _ = request.tx.send(response);
                }
            }
        }
    }
    
    // ===== Intent Management Methods =====
    
    /// Declare an intent before performing actions
    pub async fn declare_intent(
        &self,
        goal: String,
        description: Option<String>,
        steps: Vec<IntentStep>,
        success_criteria: Vec<String>,
    ) -> Result<IntentDeclaration> {
        let params = serde_json::json!({
            "goal": goal,
            "description": description,
            "steps": steps,
            "success_criteria": success_criteria,
        });
        
        let result = self.call_method("intent/declare", params).await?;
        
        Ok(serde_json::from_value(result)
            .map_err(|e| McpError::SerializationError(e.to_string()))?)
    }
    
    /// Verify intent execution
    pub async fn verify_intent(
        &self,
        intent_id: String,
        evidence: serde_json::Value,
    ) -> Result<VerificationResult> {
        let params = serde_json::json!({
            "intent_id": intent_id,
            "evidence": evidence,
        });
        
        let result = self.call_method("intent/verify", params).await?;
        
        Ok(serde_json::from_value(result)
            .map_err(|e| McpError::SerializationError(e.to_string()))?)
    }
    
    /// Get intent status
    pub async fn get_intent_status(&self, intent_id: String) -> Result<IntentStatus> {
        let params = serde_json::json!({
            "intent_id": intent_id,
        });
        
        let result = self.call_method("intent/status", params).await?;
        
        Ok(serde_json::from_value(result)
            .map_err(|e| McpError::SerializationError(e.to_string()))?)
    }
    
    /// List all intents with optional filter
    pub async fn list_intents(&self, status_filter: Option<String>) -> Result<Vec<IntentRecord>> {
        let params = serde_json::json!({
            "status": status_filter,
        });
        
        let result = self.call_method("intent/list", params).await?;
        
        let response: IntentListResponse = serde_json::from_value(result)
            .map_err(|e| McpError::SerializationError(e.to_string()))?;
        
        Ok(response.intents)
    }
    
    /// Mark intent as completed
    pub async fn complete_intent(&self, intent_id: String) -> Result<()> {
        let params = serde_json::json!({
            "intent_id": intent_id,
        });
        
        self.call_method("intent/complete", params).await?;
        Ok(())
    }
    
    // ===== Agent Spawning Methods =====
    
    /// Spawn agents in parallel with intents
    pub async fn spawn_agents(
        &self,
        agents: Vec<AgentSpawnRequest>,
    ) -> Result<Vec<AgentSpawnResponse>> {
        let params = serde_json::json!({
            "agents": agents,
        });
        
        let result = self.call_method("agent/spawn", params).await?;
        
        Ok(serde_json::from_value(result)
            .map_err(|e| McpError::SerializationError(e.to_string()))?)
    }
    
    /// Get agent status
    pub async fn get_agent_status(&self, agent_id: String) -> Result<AgentStatus> {
        let params = serde_json::json!({
            "agent_id": agent_id,
        });
        
        let result = self.call_method("agent/status", params).await?;
        
        Ok(serde_json::from_value(result)
            .map_err(|e| McpError::SerializationError(e.to_string()))?)
    }
    
    // ===== Trust and Verification Methods =====
    
    /// Check trust level of an agent
    pub async fn check_trust(&self, agent_id: String) -> Result<TrustInfo> {
        let params = serde_json::json!({
            "agent_id": agent_id,
        });
        
        let result = self.call_method("trust/check", params).await?;
        
        Ok(serde_json::from_value(result)
            .map_err(|e| McpError::SerializationError(e.to_string()))?)
    }
    
    /// Inject context for sub-agents
    pub async fn inject_context(&self, context: ContextInjection) -> Result<String> {
        let result = self.call_method("context/inject", serde_json::to_value(context)?).await?;
        
        Ok(result["context_id"].as_str()
            .ok_or_else(|| McpError::InvalidResponse("No context_id in response".to_string()))?
            .to_string())
    }
    
    /// Disconnect from server
    pub async fn disconnect(&self) -> Result<()> {
        info!("Disconnecting from MCP server");
        self.transport.close().await?;
        Ok(())
    }
}

// ===== Data Types =====

/// Intent step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentStep {
    pub name: String,
    pub action: String,
    pub verification: Option<StepVerification>,
}

/// Step verification requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepVerification {
    pub verification_type: String,
    pub expected: serde_json::Value,
}

/// Intent declaration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDeclaration {
    pub intent_id: String,
    pub status: String,
    pub message: String,
}

/// Verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub intent_id: String,
    pub verified: bool,
    pub goal: String,
    pub evidence: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Intent status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentStatus {
    pub intent_id: String,
    pub goal: String,
    pub status: String,
    pub steps: Vec<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Intent record from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentRecord {
    pub id: String,
    pub goal: String,
    pub description: Option<String>,
    pub status: String,
    pub agent_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub verification_results: HashMap<String, bool>,
}

/// Intent list response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IntentListResponse {
    pub count: usize,
    pub intents: Vec<IntentRecord>,
}

/// Agent spawn request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpawnRequest {
    pub name: String,
    pub intent_goal: String,
    pub capabilities: Vec<String>,
    pub context: Option<serde_json::Value>,
}

/// Agent spawn response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpawnResponse {
    pub agent_id: String,
    pub intent_id: String,
    pub status: String,
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub agent_id: String,
    pub status: String,
    pub current_intent: Option<String>,
    pub trust_score: f64,
}

/// Trust information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustInfo {
    pub agent_id: String,
    pub trust_level: f64,
    pub reputation: String,
    pub promises_fulfilled: u32,
    pub promises_broken: u32,
}

/// Context injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextInjection {
    pub parent_intent: String,
    pub allowed_operations: Vec<String>,
    pub restricted_paths: Vec<String>,
    pub max_execution_time: Option<u64>,
    pub verification_required: bool,
}