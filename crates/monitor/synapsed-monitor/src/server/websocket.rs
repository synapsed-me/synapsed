//! WebSocket handler for real-time updates

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Task update
    TaskUpdate {
        task_id: String,
        name: String,
        status: String,
        progress: f32,
    },
    
    /// Agent status update
    AgentStatus {
        agent_id: String,
        name: String,
        status: String,
        trust: f32,
        activity: Option<String>,
    },
    
    /// Narrative event
    Narrative {
        text: String,
        importance: String,
        timestamp: String,
    },
    
    /// System health update
    Health {
        status: String,
        score: f32,
        alerts: usize,
    },
    
    /// Event feed item
    Event {
        event_type: String,
        description: String,
        timestamp: String,
    },
    
    /// Metric update
    Metric {
        name: String,
        value: f64,
        unit: String,
    },
}

/// WebSocket connection
struct WsConnection {
    id: Uuid,
    sender: mpsc::UnboundedSender<Message>,
}

/// WebSocket handler
#[derive(Clone)]
pub struct WebSocketHandler {
    connections: Arc<RwLock<HashMap<Uuid, WsConnection>>>,
}

impl WebSocketHandler {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Handle WebSocket upgrade
    pub async fn handle_upgrade(self: Arc<Self>, ws: WebSocketUpgrade) -> Response {
        ws.on_upgrade(move |socket| self.handle_socket(socket))
    }
    
    /// Handle WebSocket connection
    async fn handle_socket(self: Arc<Self>, socket: WebSocket) {
        let connection_id = Uuid::new_v4();
        let (mut sender, mut receiver) = socket.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        
        // Store connection
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id, WsConnection {
                id: connection_id,
                sender: tx,
            });
        }
        
        tracing::info!("WebSocket client {} connected", connection_id);
        
        // Spawn task to forward messages to client
        let forward_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if sender.send(msg).await.is_err() {
                    break;
                }
            }
        });
        
        // Handle incoming messages
        while let Some(result) = receiver.next().await {
            match result {
                Ok(msg) => {
                    if let Message::Text(text) = msg {
                        // Handle client messages if needed
                        tracing::debug!("Received from {}: {}", connection_id, text);
                    } else if let Message::Close(_) = msg {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("WebSocket error for {}: {}", connection_id, e);
                    break;
                }
            }
        }
        
        // Clean up
        forward_task.abort();
        {
            let mut connections = self.connections.write().await;
            connections.remove(&connection_id);
        }
        
        tracing::info!("WebSocket client {} disconnected", connection_id);
    }
    
    /// Broadcast message to all connected clients
    pub async fn broadcast(&self, message: WsMessage) {
        let json = match serde_json::to_string(&message) {
            Ok(json) => json,
            Err(e) => {
                tracing::error!("Failed to serialize message: {}", e);
                return;
            }
        };
        
        let connections = self.connections.read().await;
        for (_, conn) in connections.iter() {
            let _ = conn.sender.send(Message::Text(json.clone()));
        }
    }
    
    /// Send message to specific client
    pub async fn send_to(&self, client_id: Uuid, message: WsMessage) {
        let json = match serde_json::to_string(&message) {
            Ok(json) => json,
            Err(e) => {
                tracing::error!("Failed to serialize message: {}", e);
                return;
            }
        };
        
        let connections = self.connections.read().await;
        if let Some(conn) = connections.get(&client_id) {
            let _ = conn.sender.send(Message::Text(json));
        }
    }
    
    /// Get number of connected clients
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }
}