//! Monitor server for the live demo
//! 
//! Provides a web interface and WebSocket endpoint for real-time monitoring
//! of the multi-agent system building a REST API.

use synapsed_monitor::{
    MonitorServer, ServerConfig,
    ObservabilityCollector, CollectorConfig,
    EventAggregator,
    narrator::{EventNarrator, NarrativeStyle},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,synapsed=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Synapsed Monitor Server...");

    // Create collector with default config
    let collector_config = CollectorConfig::default();
    let collector = Arc::new(ObservabilityCollector::new(collector_config));

    // Create event aggregator
    let aggregator = EventAggregator::new();

    // Create server configuration
    let server_config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        enable_cors: true,
        max_connections: 100,
    };

    // Create narrator
    let narrator = Arc::new(EventNarrator::new(NarrativeStyle::Conversational));
    
    // Create and start the monitor server
    let server = MonitorServer::new(
        server_config,
        collector,
        Arc::new(RwLock::new(aggregator)),
        narrator,
    );

    tracing::info!("Monitor server starting on http://localhost:8080");
    tracing::info!("WebSocket endpoint: ws://localhost:8080/ws");
    tracing::info!("Dashboard: http://localhost:8080/dashboard");

    // Run the server
    server.start().await?;

    Ok(())
}