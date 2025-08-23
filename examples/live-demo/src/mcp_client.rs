//! MCP Client for spawning agents with intent

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use uuid::Uuid;

/// MCP Client for communicating with the MCP server
pub struct McpClient {
    server_process: Option<tokio::process::Child>,
    stdin: Option<BufWriter<tokio::process::ChildStdin>>,
    stdout: Option<BufReader<tokio::process::ChildStdout>>,
}

impl McpClient {
    /// Create a new MCP client and start the server
    pub async fn new() -> Result<Self> {
        // Start the MCP server process
        let mut server_process = Command::new("cargo")
            .args(&["run", "-p", "synapsed-mcp", "--bin", "synapsed-mcp-server"])
            .env("SYNAPSED_INTENT_STORAGE_PATH", "/tmp/synapsed-intents.json")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        
        let stdin = server_process.stdin.take()
            .ok_or_else(|| anyhow!("Failed to get stdin"))?;
        let stdout = server_process.stdout.take()
            .ok_or_else(|| anyhow!("Failed to get stdout"))?;
        
        Ok(Self {
            server_process: Some(server_process),
            stdin: Some(BufWriter::new(stdin)),
            stdout: Some(BufReader::new(stdout)),
        })
    }
    
    /// Send a JSON-RPC request and get the response
    async fn send_request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let request_id = Uuid::new_v4().to_string();
        
        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": request_id
        });
        
        // Send request
        if let Some(stdin) = &mut self.stdin {
            stdin.write_all(request.to_string().as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        } else {
            return Err(anyhow!("No stdin available"));
        }
        
        // Read response
        if let Some(stdout) = &mut self.stdout {
            let mut line = String::new();
            stdout.read_line(&mut line).await?;
            
            let response: JsonRpcResponse = serde_json::from_str(&line)?;
            
            if let Some(error) = response.error {
                return Err(anyhow!("MCP error: {}", error.message));
            }
            
            response.result.ok_or_else(|| anyhow!("No result in response"))
        } else {
            Err(anyhow!("No stdout available"))
        }
    }
    
    /// Declare an intent
    pub async fn declare_intent(
        &mut self,
        agent_id: &str,
        description: &str,
        metadata: Option<Value>,
    ) -> Result<String> {
        let params = json!({
            "agent_id": agent_id,
            "description": description,
            "metadata": metadata
        });
        
        let result = self.send_request("intent/declare", Some(params)).await?;
        
        result["intent_id"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No intent_id in response"))
    }
    
    /// Update intent status
    pub async fn update_intent(
        &mut self,
        intent_id: &str,
        status: &str,
        result: Option<Value>,
    ) -> Result<()> {
        let params = json!({
            "intent_id": intent_id,
            "status": status,
            "result": result
        });
        
        self.send_request("intent/update", Some(params)).await?;
        Ok(())
    }
    
    /// Verify an intent
    pub async fn verify_intent(&mut self, intent_id: &str) -> Result<bool> {
        let params = json!({
            "intent_id": intent_id
        });
        
        let result = self.send_request("intent/verify", Some(params)).await?;
        
        Ok(result["verified"].as_bool().unwrap_or(false))
    }
    
    /// Spawn an agent with intent
    pub async fn spawn_agent(
        &mut self,
        agent_type: &str,
        config: Option<Value>,
        intent_id: Option<String>,
    ) -> Result<String> {
        let params = json!({
            "agent_type": agent_type,
            "config": config,
            "intent_id": intent_id
        });
        
        let result = self.send_request("agent/spawn", Some(params)).await?;
        
        result["agent_id"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No agent_id in response"))
    }
    
    /// Get agent status
    pub async fn get_agent_status(&mut self, agent_id: &str) -> Result<Value> {
        let params = json!({
            "agent_id": agent_id
        });
        
        self.send_request("agent/status", Some(params)).await
    }
    
    /// List all intents
    pub async fn list_intents(&mut self) -> Result<Vec<Value>> {
        let result = self.send_request("intent/list", None).await?;
        
        result["intents"]
            .as_array()
            .map(|arr| arr.clone())
            .ok_or_else(|| anyhow!("No intents array in response"))
    }
    
    /// Shutdown the MCP server
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(mut process) = self.server_process.take() {
            process.kill().await?;
        }
        Ok(())
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        if let Some(mut process) = self.server_process.take() {
            let _ = process.start_kill();
        }
    }
}

/// JSON-RPC response structure
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    #[allow(dead_code)]
    id: Option<Value>,
}

/// JSON-RPC error structure
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i32,
    message: String,
    #[allow(dead_code)]
    data: Option<Value>,
}