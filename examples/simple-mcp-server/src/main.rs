//! Simple MCP Server Implementation
//! 
//! A minimal MCP server that demonstrates intent declaration and verification
//! without requiring all the complex dependencies.

mod html_streaming;

use anyhow::Result;
use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::error;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use html_streaming::{SSEHandler, HTML_CLIENT};

/// JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

/// JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC error
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

/// Intent record
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Intent {
    id: String,
    goal: String,
    description: Option<String>,
    status: String,
    agent_id: Option<String>,
    created_at: DateTime<Utc>,
    verified: bool,
    verification_proofs: Vec<String>,
}

/// MCP Server state
struct ServerState {
    intents: HashMap<String, Intent>,
    agents: HashMap<String, AgentInfo>,
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentInfo {
    id: String,
    capabilities: Vec<String>,
    trust_score: f64,
    active_intents: Vec<String>,
}

/// Simple MCP Server with HTML streaming support
struct McpServer {
    state: Arc<RwLock<ServerState>>,
    sse_handler: Arc<SSEHandler>,
}

impl McpServer {
    fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(ServerState {
                intents: HashMap::new(),
                agents: HashMap::new(),
            })),
            sse_handler: Arc::new(SSEHandler::new()),
        }
    }
    
    async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        println!("{} {} {}", 
            "[REQUEST]".bright_cyan(),
            request.method.yellow(),
            serde_json::to_string(&request.params).unwrap_or_default().bright_black()
        );
        
        let result = match request.method.as_str() {
            "intent/declare" => self.handle_intent_declare(request.params).await,
            "intent/verify" => self.handle_intent_verify(request.params).await,
            "intent/list" => self.handle_intent_list(request.params).await,
            "intent/status" => self.handle_intent_status(request.params).await,
            "agent/spawn" => self.handle_agent_spawn(request.params).await,
            "system/info" => self.handle_system_info().await,
            _ => Err(format!("Unknown method: {}", request.method)),
        };
        
        match result {
            Ok(value) => {
                println!("{} Success", "[RESPONSE]".bright_green());
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(value),
                    error: None,
                }
            }
            Err(msg) => {
                println!("{} Error: {}", "[RESPONSE]".bright_red(), msg);
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: msg,
                        data: None,
                    }),
                }
            }
        }
    }
    
    async fn handle_intent_declare(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
        let params = params.ok_or("Missing parameters")?;
        let goal = params["goal"].as_str().ok_or("Missing goal")?;
        let description = params["description"].as_str();
        
        let intent_id = Uuid::new_v4().to_string();
        let intent = Intent {
            id: intent_id.clone(),
            goal: goal.to_string(),
            description: description.map(|s| s.to_string()),
            status: "declared".to_string(),
            agent_id: params["agent_id"].as_str().map(|s| s.to_string()),
            created_at: Utc::now(),
            verified: false,
            verification_proofs: Vec::new(),
        };
        
        let mut state = self.state.write().await;
        state.intents.insert(intent_id.clone(), intent.clone());
        
        println!("{} Intent declared: {} - {}", 
            "[INTENT]".bright_blue(),
            intent_id.cyan(),
            goal.green()
        );
        
        // Broadcast to SSE clients
        let _ = self.sse_handler.broadcast_intent_declare(&intent_id, goal).await;
        
        Ok(serde_json::json!({
            "intent_id": intent_id,
            "status": "declared",
            "goal": goal,
            "timestamp": intent.created_at,
        }))
    }
    
    async fn handle_intent_verify(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
        let params = params.ok_or("Missing parameters")?;
        let intent_id = params["intent_id"].as_str().ok_or("Missing intent_id")?;
        let _evidence = &params["evidence"];
        
        let mut state = self.state.write().await;
        let intent = state.intents.get_mut(intent_id).ok_or("Intent not found")?;
        
        // Add verification proof
        let proof = format!("proof_{}", Uuid::new_v4());
        intent.verification_proofs.push(proof.clone());
        
        // Mark as verified if we have enough proofs
        if intent.verification_proofs.len() >= 3 {
            intent.verified = true;
            intent.status = "verified".to_string();
        }
        
        println!("{} Intent {} verified (proofs: {})", 
            "[VERIFY]".bright_cyan(),
            intent_id.yellow(),
            intent.verification_proofs.len()
        );
        
        // Broadcast to SSE clients
        let _ = self.sse_handler.broadcast_intent_verify(intent_id, intent.verified, intent.verification_proofs.len()).await;
        
        Ok(serde_json::json!({
            "intent_id": intent_id,
            "verified": intent.verified,
            "proof": proof,
            "total_proofs": intent.verification_proofs.len(),
        }))
    }
    
    async fn handle_intent_list(&self, _params: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
        let state = self.state.read().await;
        let intents: Vec<_> = state.intents.values().cloned().collect();
        
        println!("{} Listing {} intents", "[LIST]".bright_magenta(), intents.len());
        
        Ok(serde_json::json!({
            "count": intents.len(),
            "intents": intents,
        }))
    }
    
    async fn handle_intent_status(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
        let params = params.ok_or("Missing parameters")?;
        let intent_id = params["intent_id"].as_str().ok_or("Missing intent_id")?;
        
        let state = self.state.read().await;
        let intent = state.intents.get(intent_id).ok_or("Intent not found")?;
        
        Ok(serde_json::to_value(intent).unwrap())
    }
    
    async fn handle_agent_spawn(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
        let params = params.ok_or("Missing parameters")?;
        let agents = params["agents"].as_array().ok_or("Missing agents array")?;
        
        let mut spawned = Vec::new();
        let mut state = self.state.write().await;
        
        for agent_params in agents {
            let agent_id = Uuid::new_v4().to_string();
            let agent = AgentInfo {
                id: agent_id.clone(),
                capabilities: agent_params["capabilities"]
                    .as_array()
                    .map(|arr| arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect())
                    .unwrap_or_default(),
                trust_score: 0.5,
                active_intents: Vec::new(),
            };
            
            state.agents.insert(agent_id.clone(), agent);
            
            println!("{} Spawned agent: {}", 
                "[AGENT]".bright_green(),
                agent_id.yellow()
            );
            
            spawned.push(serde_json::json!({
                "agent_id": agent_id,
                "status": "active",
            }));
        }
        
        Ok(serde_json::json!({
            "agents": spawned,
            "count": spawned.len(),
        }))
    }
    
    async fn handle_system_info(&self) -> Result<serde_json::Value, String> {
        let state = self.state.read().await;
        
        Ok(serde_json::json!({
            "server": "Simple MCP Server",
            "version": "1.0.0",
            "intents_count": state.intents.len(),
            "agents_count": state.agents.len(),
            "capabilities": [
                "intent/declare",
                "intent/verify",
                "intent/list",
                "intent/status",
                "agent/spawn",
            ],
        }))
    }
    
    async fn handle_client(self: Arc<Self>, stream: TcpStream) {
        let addr = stream.peer_addr().unwrap();
        println!("{} Client connected: {}", "[CONNECT]".bright_green(), addr);
        
        // For now, just handle JSON-RPC
        let (reader, writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut writer = BufWriter::new(writer);
        
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    println!("{} Client disconnected: {}", "[DISCONNECT]".bright_yellow(), addr);
                    break;
                }
                Ok(_) => {
                    if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&line) {
                        let response = self.handle_request(request).await;
                        let response_str = serde_json::to_string(&response).unwrap();
                        
                        if let Err(e) = writer.write_all(response_str.as_bytes()).await {
                            error!("Failed to write response: {}", e);
                            break;
                        }
                        if let Err(e) = writer.write_all(b"\n").await {
                            error!("Failed to write newline: {}", e);
                            break;
                        }
                        if let Err(e) = writer.flush().await {
                            error!("Failed to flush: {}", e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("Read error: {}", e);
                    break;
                }
            }
        }
    }
    
    async fn handle_http_client(self: Arc<Self>, mut stream: TcpStream) {
        let mut buf = [0u8; 1024];
        
        // Read request
        if let Ok(n) = stream.read(&mut buf).await {
            let request = String::from_utf8_lossy(&buf[..n]);
            let first_line = request.lines().next().unwrap_or("");
            println!("{} HTTP Request: {}", "[HTTP]".bright_magenta(), first_line);
            
            if first_line.starts_with("GET /stream") {
                // Handle SSE stream
                println!("{} Starting SSE stream", "[SSE]".bright_magenta());
                let _ = self.sse_handler.handle_connection(stream).await;
            } else if first_line.starts_with("GET /") {
                // Serve HTML client
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\n").await;
                let _ = stream.write_all(b"Content-Type: text/html\r\n").await;
                let _ = stream.write_all(b"Connection: close\r\n").await;
                let _ = stream.write_all(b"\r\n").await;
                let _ = stream.write_all(HTML_CLIENT.as_bytes()).await;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    println!("\n{}", "╔══════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║              SIMPLE MCP SERVER                          ║".bright_cyan().bold());
    println!("{}", "║                                                          ║".bright_cyan());
    println!("{}", "║  A minimal MCP server for intent management             ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════╝".bright_cyan());
    
    let server = Arc::new(McpServer::new());
    let jsonrpc_addr = "127.0.0.1:3000";
    let http_addr = "127.0.0.1:3001";
    
    println!("\n{} Starting MCP server", "[SERVER]".bright_yellow());
    println!("  JSON-RPC: {}", jsonrpc_addr.bright_white());
    println!("  HTTP/SSE: {}", http_addr.bright_white());
    
    let jsonrpc_listener = TcpListener::bind(jsonrpc_addr).await?;
    let http_listener = TcpListener::bind(http_addr).await?;
    
    println!("{} Server ready - accepting connections", "[READY]".bright_green());
    println!("\n{} Available methods:", "[INFO]".bright_blue());
    println!("  {} intent/declare - Declare a new intent", "→".green());
    println!("  {} intent/verify - Verify an intent", "→".green());
    println!("  {} intent/list - List all intents", "→".green());
    println!("  {} intent/status - Get intent status", "→".green());
    println!("  {} agent/spawn - Spawn new agents", "→".green());
    println!("  {} system/info - Get system information", "→".green());
    
    println!("\n{} Test with:", "[TEST]".bright_magenta());
    println!("  echo '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"system/info\",\"params\":{{}}}}' | nc localhost 3000");
    
    println!("\n{} HTML Streaming Interface:", "[WEB]".bright_cyan());
    println!("  Open http://localhost:3001/ in your browser for real-time updates");
    println!("  SSE endpoint: http://localhost:3001/stream");
    
    // Spawn HTTP server handler
    let http_server = server.clone();
    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = http_listener.accept().await {
                let server = http_server.clone();
                tokio::spawn(async move {
                    server.handle_http_client(stream).await;
                });
            }
        }
    });
    
    // Handle JSON-RPC connections
    loop {
        let (stream, _) = jsonrpc_listener.accept().await?;
        let server = server.clone();
        tokio::spawn(async move {
            server.handle_client(stream).await;
        });
    }
}