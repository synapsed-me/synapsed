//! Server components for the monitoring system

mod websocket;
mod rest_api;

pub use websocket::{WebSocketHandler, WsMessage};
pub use rest_api::{create_router, ApiState};

use crate::{
    collector::ObservabilityCollector,
    aggregator::EventAggregator,
    narrator::EventNarrator,
    Result,
};
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
    pub max_connections: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            enable_cors: true,
            max_connections: 100,
        }
    }
}

/// Main monitor server
pub struct MonitorServer {
    config: ServerConfig,
    collector: Arc<ObservabilityCollector>,
    aggregator: Arc<RwLock<EventAggregator>>,
    narrator: Arc<EventNarrator>,
    router: Router,
}

impl MonitorServer {
    /// Create a new monitor server
    pub fn new(
        config: ServerConfig,
        collector: Arc<ObservabilityCollector>,
        aggregator: Arc<RwLock<EventAggregator>>,
        narrator: Arc<EventNarrator>,
    ) -> Self {
        let api_state = ApiState {
            collector: collector.clone(),
            aggregator: aggregator.clone(),
            narrator: narrator.clone(),
            ws_handler: Arc::new(RwLock::new(WebSocketHandler::new())),
            storage_path: std::env::var("SYNAPSED_INTENT_STORAGE_PATH")
                .ok()
                .map(std::path::PathBuf::from),
        };
        
        let router = create_router(api_state);
        
        Self {
            config,
            collector,
            aggregator,
            narrator,
            router,
        }
    }
    
    /// Start the server
    pub async fn start(self) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| crate::MonitorError::ServerError(format!("Invalid address: {}", e)))?;
        
        tracing::info!("Starting monitor server on {}", addr);
        
        let listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| crate::MonitorError::ServerError(e.to_string()))?;
        axum::serve(listener, self.router)
            .await
            .map_err(|e| crate::MonitorError::ServerError(e.to_string()))?;
        
        Ok(())
    }
}