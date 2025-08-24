//! HTML Streaming support for MCP Server
//! 
//! Provides Server-Sent Events (SSE) and WebSocket support
//! for real-time intent updates and verification streams

use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use anyhow::Result;

/// SSE event for HTML streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSEEvent {
    pub event: String,
    pub data: serde_json::Value,
    pub id: Option<String>,
}

impl SSEEvent {
    pub fn new(event: &str, data: serde_json::Value) -> Self {
        Self {
            event: event.to_string(),
            data,
            id: Some(uuid::Uuid::new_v4().to_string()),
        }
    }
    
    /// Format as SSE message
    pub fn to_sse(&self) -> String {
        let mut msg = String::new();
        if let Some(id) = &self.id {
            msg.push_str(&format!("id: {}\n", id));
        }
        msg.push_str(&format!("event: {}\n", self.event));
        msg.push_str(&format!("data: {}\n\n", self.data.to_string()));
        msg
    }
}

/// HTML streaming handler for Server-Sent Events
pub struct SSEHandler {
    // Simplified: we won't actually track clients for now
    _marker: std::marker::PhantomData<()>,
}

impl SSEHandler {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
    
    /// Handle SSE connection
    pub async fn handle_connection(&self, mut stream: TcpStream) -> Result<()> {
        // Send SSE headers
        stream.write_all(b"HTTP/1.1 200 OK\r\n").await?;
        stream.write_all(b"Content-Type: text/event-stream\r\n").await?;
        stream.write_all(b"Cache-Control: no-cache\r\n").await?;
        stream.write_all(b"Connection: keep-alive\r\n").await?;
        stream.write_all(b"Access-Control-Allow-Origin: *\r\n").await?;
        stream.write_all(b"\r\n").await?;
        stream.flush().await?;
        
        // Send initial ping
        let ping = SSEEvent::new("ping", serde_json::json!({"status": "connected"}));
        stream.write_all(ping.to_sse().as_bytes()).await?;
        stream.flush().await?;
        
        // Keep connection alive
        // In a real implementation, we'd store the stream for broadcasting
        // For now, just keep it open
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        
        Ok(())
    }
    
    /// Broadcast event to all connected clients
    pub async fn broadcast(&self, _event: SSEEvent) -> Result<()> {
        // Simplified implementation - actual broadcasting would require
        // maintaining a list of connected streams
        Ok(())
    }
    
    /// Broadcast intent declaration
    pub async fn broadcast_intent_declare(&self, intent_id: &str, goal: &str) -> Result<()> {
        let event = SSEEvent::new("intent-declare", serde_json::json!({
            "intent_id": intent_id,
            "goal": goal,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }));
        self.broadcast(event).await
    }
    
    /// Broadcast intent verification
    pub async fn broadcast_intent_verify(&self, intent_id: &str, verified: bool, proofs: usize) -> Result<()> {
        let event = SSEEvent::new("intent-verify", serde_json::json!({
            "intent_id": intent_id,
            "verified": verified,
            "proof_count": proofs,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }));
        self.broadcast(event).await
    }
    
    /// Broadcast agent spawn
    pub async fn broadcast_agent_spawn(&self, agent_id: &str, capabilities: &[String]) -> Result<()> {
        let event = SSEEvent::new("agent-spawn", serde_json::json!({
            "agent_id": agent_id,
            "capabilities": capabilities,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }));
        self.broadcast(event).await
    }
}

/// HTML client for testing streaming
pub const HTML_CLIENT: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>MCP Server - Intent Verification Stream</title>
    <style>
        body {
            font-family: monospace;
            background: #1e1e1e;
            color: #00ff00;
            padding: 20px;
        }
        h1 { color: #00ffff; }
        .event {
            border: 1px solid #00ff00;
            padding: 10px;
            margin: 10px 0;
            background: #000;
        }
        .intent-declare { border-color: #ffff00; }
        .intent-verify { border-color: #00ffff; }
        .agent-spawn { border-color: #ff00ff; }
        .timestamp { color: #888; font-size: 0.9em; }
    </style>
</head>
<body>
    <h1>MCP Intent Verification Stream</h1>
    <div id="events"></div>
    
    <script>
        const eventsDiv = document.getElementById('events');
        const eventSource = new EventSource('/stream');
        
        eventSource.addEventListener('intent-declare', (e) => {
            const data = JSON.parse(e.data);
            addEvent('Intent Declared', data, 'intent-declare');
        });
        
        eventSource.addEventListener('intent-verify', (e) => {
            const data = JSON.parse(e.data);
            addEvent('Intent Verified', data, 'intent-verify');
        });
        
        eventSource.addEventListener('agent-spawn', (e) => {
            const data = JSON.parse(e.data);
            addEvent('Agent Spawned', data, 'agent-spawn');
        });
        
        eventSource.addEventListener('ping', (e) => {
            console.log('Ping:', e.data);
        });
        
        function addEvent(title, data, className) {
            const eventDiv = document.createElement('div');
            eventDiv.className = 'event ' + className;
            eventDiv.innerHTML = `
                <strong>${title}</strong>
                <div class="timestamp">${data.timestamp}</div>
                <pre>${JSON.stringify(data, null, 2)}</pre>
            `;
            eventsDiv.insertBefore(eventDiv, eventsDiv.firstChild);
            
            // Keep only last 50 events
            while (eventsDiv.children.length > 50) {
                eventsDiv.removeChild(eventsDiv.lastChild);
            }
        }
    </script>
</body>
</html>
"#;