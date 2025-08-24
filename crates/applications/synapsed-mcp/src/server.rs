//! MCP Server implementation

use crate::{
    error::{McpError, Result},
    tools::{IntentTools, VerificationTools},
    resources::ContextResources,
    intent_store::IntentStore,
    protocol::{McpProtocolHandler, JsonRpcRequest, JsonRpcResponse},
    agent_spawner::AgentSpawner,
    observability::{McpEvent, EVENT_CIRCUIT},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// MCP Server configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
    /// Server description
    pub description: String,
    /// Enable intent verification
    pub enable_intent_verification: bool,
    /// Enable promise tracking
    pub enable_promise_tracking: bool,
    /// Enable context injection
    pub enable_context_injection: bool,
    /// Maximum concurrent intents
    pub max_concurrent_intents: usize,
    /// Trust threshold for verification
    pub trust_threshold: f64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: crate::SERVER_NAME.to_string(),
            version: crate::SERVER_VERSION.to_string(),
            description: crate::SERVER_DESCRIPTION.to_string(),
            enable_intent_verification: true,
            enable_promise_tracking: true,
            enable_context_injection: true,
            max_concurrent_intents: 10,
            trust_threshold: 0.8,
        }
    }
}

impl ServerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();
        
        if let Ok(name) = std::env::var("SYNAPSED_MCP_NAME") {
            config.name = name;
        }
        
        if let Ok(val) = std::env::var("SYNAPSED_MCP_MAX_INTENTS") {
            config.max_concurrent_intents = val.parse()
                .map_err(|e| McpError::InvalidParams(format!("Invalid max_intents: {}", e)))?;
        }
        
        if let Ok(val) = std::env::var("SYNAPSED_MCP_TRUST_THRESHOLD") {
            config.trust_threshold = val.parse()
                .map_err(|e| McpError::InvalidParams(format!("Invalid trust_threshold: {}", e)))?;
        }
        
        Ok(config)
    }
}

/// MCP Server state
pub struct ServerState {
    /// Active intents
    pub active_intents: HashMap<uuid::Uuid, synapsed_intent::HierarchicalIntent>,
    /// Active agents
    pub active_agents: HashMap<uuid::Uuid, synapsed_promise::AutonomousAgent>,
    /// Verification results
    pub verification_results: Vec<synapsed_verify::VerificationResult>,
    /// Internal intent store (not exposed to clients)
    pub(crate) intent_store: Arc<IntentStore>,
}

/// MCP Server implementation
pub struct McpServer {
    config: ServerConfig,
    state: Arc<RwLock<ServerState>>,
    intent_tools: Arc<IntentTools>,
    verification_tools: Arc<VerificationTools>,
    context_resources: Arc<ContextResources>,
    protocol_handler: Arc<McpProtocolHandler>,
    agent_spawner: Arc<AgentSpawner>,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(config: ServerConfig) -> Self {
        Self::new_impl(config, None)
    }
    
    /// Create a new MCP server with custom log file path
    pub fn new_with_log_file(config: ServerConfig, log_file_path: String) -> Self {
        Self::new_impl(config, Some(log_file_path))
    }
    
    /// Internal implementation for server creation
    fn new_impl(config: ServerConfig, log_file_path: Option<String>) -> Self {
        // Create intent store based on environment configuration
        let intent_store = if let Ok(storage_path) = std::env::var("SYNAPSED_INTENT_STORAGE_PATH") {
            if storage_path.ends_with(".db") {
                // SQLite storage
                Arc::new(IntentStore::with_sqlite_storage(&storage_path)
                    .expect("Failed to create SQLite intent store"))
            } else {
                // File-based storage
                Arc::new(IntentStore::with_file_storage(&storage_path)
                    .expect("Failed to create file intent store"))
            }
        } else {
            // Default to memory storage
            Arc::new(IntentStore::new().expect("Failed to create intent store"))
        };
        
        let state = Arc::new(RwLock::new(ServerState {
            active_intents: HashMap::new(),
            active_agents: HashMap::new(),
            verification_results: Vec::new(),
            intent_store,
        }));
        
        let intent_tools = Arc::new(IntentTools::new(state.clone()));
        let verification_tools = Arc::new(VerificationTools::new(state.clone()));
        let context_resources = Arc::new(ContextResources::new(state.clone()));
        
        // Create agent spawner
        let agent_spawner = Arc::new(AgentSpawner::new());
        
        // Create protocol handler with a new intent store using the same storage path
        let protocol_intent_store = if let Ok(storage_path) = std::env::var("SYNAPSED_INTENT_STORAGE_PATH") {
            if storage_path.ends_with(".db") {
                Arc::new(RwLock::new(IntentStore::with_sqlite_storage(&storage_path)
                    .expect("Failed to create SQLite intent store for protocol")))
            } else {
                Arc::new(RwLock::new(IntentStore::with_file_storage(&storage_path)
                    .expect("Failed to create file intent store for protocol")))
            }
        } else {
            Arc::new(RwLock::new(IntentStore::new().expect("Failed to create intent store for protocol")))
        };
        
        let protocol_handler = Arc::new(McpProtocolHandler::new(
            protocol_intent_store,
            agent_spawner.clone(),
        ));
        
        // Initialize event circuit
        let log_path = log_file_path.unwrap_or_else(|| "/tmp/synapsed_substrates.log".to_string());
        tokio::spawn(async move {
            if let Err(e) = EVENT_CIRCUIT.initialize(log_path).await {
                tracing::error!("Failed to initialize event circuit: {}", e);
            }
        });
        
        Self {
            config,
            state,
            intent_tools,
            verification_tools,
            context_resources,
            protocol_handler,
            agent_spawner,
        }
    }
    
    /// Handle a JSON-RPC request
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        self.protocol_handler.handle_request(request).await
    }
    
    /// Serve over stdio transport
    pub async fn serve_stdio(self) -> Result<()> {
        info!("Starting MCP server on stdio transport");
        
        // Emit server started event
        let config_json = serde_json::to_value(&self.config).unwrap_or_else(|_| serde_json::json!({}));
        let event = McpEvent::server_started(
            self.config.name.clone(),
            self.config.version.clone(),
            config_json
        );
        let _ = EVENT_CIRCUIT.emit_event(event).await;
        
        // Create a simple JSON-RPC server over stdio
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        
        info!("MCP server ready - accepting JSON-RPC requests");
        info!("Available methods:");
        info!("  - intent/declare: Declare intentions before actions");
        info!("  - intent/verify: Verify execution against declarations");
        info!("  - intent/list: List all stored intents");
        info!("  - agent/spawn: Spawn a new agent with intent");
        info!("  - agent/status: Get agent status");
        info!("  - trust/check: Check agent trust levels");
        info!("  - context/inject: Inject context for sub-agents");
        
        // For now, use a simple loop to read JSON-RPC requests
        let mut reader = tokio::io::BufReader::new(stdin);
        let mut writer = tokio::io::BufWriter::new(stdout);
        
        loop {
            // Read a line of JSON
            use tokio::io::AsyncBufReadExt;
            let mut line = String::new();
            if reader.read_line(&mut line).await
                .map_err(|e| McpError::Transport(format!("Read error: {}", e)))? == 0 {
                break; // EOF
            }
            
            // Parse as JSON-RPC request
            if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&line) {
                let response = self.handle_request(request).await;
                
                // Write response
                use tokio::io::AsyncWriteExt;
                let response_json = serde_json::to_string(&response)
                    .map_err(|e| McpError::Transport(format!("Serialize error: {}", e)))?;
                writer.write_all(response_json.as_bytes()).await
                    .map_err(|e| McpError::Transport(format!("Write error: {}", e)))?;
                writer.write_all(b"\n").await
                    .map_err(|e| McpError::Transport(format!("Write error: {}", e)))?;
                writer.flush().await
                    .map_err(|e| McpError::Transport(format!("Flush error: {}", e)))?;
            }
        }
        
        info!("MCP server shutting down");
        Ok(())
    }
}