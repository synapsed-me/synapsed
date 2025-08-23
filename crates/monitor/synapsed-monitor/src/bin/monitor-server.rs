//! Monitor server binary

use synapsed_monitor::{
    collector::ObservabilityCollector,
    aggregator::EventAggregator,
    narrator::{EventNarrator, NarrativeStyle},
    server::{MonitorServer, ServerConfig},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    println!("🖥️  Synapsed Monitor Server");
    println!("==========================\n");
    
    // Create components
    let collector_config = synapsed_monitor::collector::CollectorConfig::default();
    let collector = Arc::new(ObservabilityCollector::new(collector_config));
    let aggregator = Arc::new(RwLock::new(EventAggregator::new()));
    let narrator = Arc::new(EventNarrator::new(NarrativeStyle::Technical));
    
    // Configure server
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        enable_cors: true,
        max_connections: 100,
    };
    
    println!("📡 Starting server on http://{}:{}", config.host, config.port);
    println!("📊 REST API: http://{}:{}/api", config.host, config.port);
    println!("🔌 WebSocket: ws://{}:{}/ws", config.host, config.port);
    println!("\nEndpoints:");
    println!("  • /health - Health check");
    println!("  • /viewer - 🔍 Intent & Observability Viewer UI");
    println!("  • /api/system/health - System health");
    println!("  • /api/tasks - View all tasks");
    println!("  • /api/agents - View all agents");
    println!("  • /api/events - View events");
    println!("  • /api/narratives - Human-readable narratives");
    println!("  • /api/intents/stored - View stored intents");
    println!("  • /api/observability/substrates - Substrates data");
    println!("  • /api/observability/serventis - Serventis data");
    println!("\n💡 Set SYNAPSED_INTENT_STORAGE_PATH=/path/to/storage.db for persistent storage");
    println!("\n🚀 Server running... Press Ctrl+C to stop\n");
    
    // Create and start server
    let server = MonitorServer::new(config, collector, aggregator, narrator);
    server.start().await?;
    
    Ok(())
}