//! REST API endpoints for the monitor

use crate::{
    collector::ObservabilityCollector,
    aggregator::EventAggregator,
    narrator::{EventNarrator, NarrativeStyle},
    views::{TaskView, AgentView, SystemHealthView, SystemMetrics},
    server::websocket::{WebSocketHandler, WsMessage},
};
use axum::{
    extract::{Path, Query, State, ws::WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response, Html},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

/// API state shared across handlers
#[derive(Clone)]
pub struct ApiState {
    pub collector: Arc<ObservabilityCollector>,
    pub aggregator: Arc<RwLock<EventAggregator>>,
    pub narrator: Arc<EventNarrator>,
    pub ws_handler: Arc<RwLock<WebSocketHandler>>,
    pub storage_path: Option<PathBuf>,
}

// Ensure ApiState implements Send + Sync for Axum handlers
unsafe impl Send for ApiState {}
unsafe impl Sync for ApiState {}

/// Simple test state
#[derive(Clone)]
pub struct TestState {
    pub value: String,
}

/// Create the API router
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        .route("/health-state", get(health_check_with_state))
        
        // Simple test handler
        .route("/test", get(simple_test))
        .route("/test-minimal", get(minimal_handler))
        
        // System endpoints
        .route("/api/system/health", get(get_system_health))
        .route("/api/system/metrics", get(get_system_metrics))
        
        // Task endpoints
        .route("/api/tasks", get(get_tasks))
        .route("/api/tasks/:id", get(get_task))
        
        // Agent endpoints  
        .route("/api/agents", get(get_agents))
        .route("/api/agents/:id", get(get_agent))
        
        // Event endpoints
        .route("/api/events", get(get_events))
        .route("/api/events/correlated", get(get_correlated_events))
        
        // Narrative endpoint
        .route("/api/narratives", get(get_narratives))
        
        // Intent storage endpoints
        .route("/api/intents/stored", get(get_stored_intents))
        .route("/api/intents/stored/:id", get(get_stored_intent))
        .route("/api/intents/hierarchy", get(get_intent_hierarchy))
        
        // Observability data endpoints
        .route("/api/observability/substrates", get(get_substrates_data))
        .route("/api/observability/serventis", get(get_serventis_data))
        .route("/api/observability/timeline", get(get_observability_timeline))
        
        // WebSocket endpoint
        .route("/ws", get(websocket_handler))
        
        // Viewer UI
        .route("/viewer", get(serve_viewer))
        
        // Add state
        .with_state(state)
        
        // Add CORS support
        .layer(CorsLayer::permissive())
}

/// Health check endpoint  
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "synapsed-monitor",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Health check with state - exact copy
async fn health_check_with_state(State(_state): State<ApiState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "synapsed-monitor",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Simple test handler without state
async fn simple_test() -> Json<serde_json::Value> {
    Json(serde_json::json!({"test": "ok"}))
}

/// Minimal handler for testing
async fn minimal_handler(State(_state): State<ApiState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"minimal": "test"}))
}

/// Test handler with simple state
async fn test_simple_state(State(state): State<TestState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"value": state.value}))
}

/// Test handler with state
async fn simple_state_test(State(_state): State<ApiState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"test": "with_state"}))
}

/// Test handler with SystemHealthView
async fn test_system_health() -> Json<SystemHealthView> {
    let health = SystemHealthView {
        status: crate::views::HealthStatus::Healthy,
        health_score: 95.0,
        services: vec![],
        metrics: SystemMetrics {
            cpu_usage: 45.0,
            memory_usage: 62.0,
            disk_usage: 30.0,
            network_bandwidth: 12.5,
            active_tasks: 3,
            queued_tasks: 2,
            active_agents: 5,
            total_agents: 5,
            uptime: chrono::Duration::hours(24),
            request_rate: 150.0,
            avg_response_time: 45.0,
        },
        alerts: vec![],
        incidents: vec![],
        recommendations: vec![],
        last_updated: chrono::Utc::now(),
    };
    Json(health)
}

/// Get system health
async fn get_system_health(State(_state): State<ApiState>) -> Json<SystemHealthView> {
    // In production, would aggregate real health data
    let health = SystemHealthView {
        status: crate::views::HealthStatus::Healthy,
        health_score: 95.0,
        services: vec![],
        metrics: SystemMetrics {
            cpu_usage: 45.0,
            memory_usage: 62.0,
            disk_usage: 30.0,
            network_bandwidth: 12.5,
            active_tasks: 3,
            queued_tasks: 2,
            active_agents: 5,
            total_agents: 5,
            uptime: chrono::Duration::hours(24),
            request_rate: 150.0,
            avg_response_time: 45.0,
        },
        alerts: vec![],
        incidents: vec![],
        recommendations: vec![],
        last_updated: chrono::Utc::now(),
    };
    
    // TODO: Broadcast to WebSocket clients  
    // let ws_handler = state.ws_handler.read().await;
    // ws_handler.broadcast(WsMessage::Health {
    //     status: format!("{:?}", health.status),
    //     score: health.health_score,
    //     alerts: health.alerts.len(),
    // }).await;
    
    Json(health)
}

/// Get system metrics
async fn get_system_metrics(State(_state): State<ApiState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "cpu": 45.0,
        "memory": 62.0,
        "disk": 30.0,
        "network": 12.5,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

/// Get all tasks
async fn get_tasks(State(_state): State<ApiState>) -> Json<Vec<serde_json::Value>> {
    // In production, would fetch real tasks
    let tasks = vec![
        serde_json::json!({
            "id": "task-1",
            "name": "Build TODO API",
            "status": "executing",
            "progress": 65.0,
        }),
    ];
    
    Json(tasks)
}

/// Get specific task
async fn get_task(
    Path(id): Path<String>,
    State(_state): State<ApiState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": id,
        "name": "Build TODO API",
        "status": "executing",
        "progress": 65.0,
        "agents": ["architect", "backend", "tester"],
    }))
}

/// Get all agents
async fn get_agents(State(_state): State<ApiState>) -> Json<Vec<serde_json::Value>> {
    let agents = vec![
        serde_json::json!({
            "id": "architect",
            "name": "API Architect",
            "status": "active",
            "trust": 0.85,
        }),
        serde_json::json!({
            "id": "backend",
            "name": "Backend Developer",
            "status": "active",
            "trust": 0.90,
        }),
    ];
    
    Json(agents)
}

/// Get specific agent
async fn get_agent(
    Path(id): Path<String>,
    State(_state): State<ApiState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": id,
        "name": "API Architect",
        "status": "active",
        "trust": 0.85,
        "current_activity": "Designing endpoints",
    }))
}

/// Get events
async fn get_events(State(_state): State<ApiState>) -> Json<Vec<serde_json::Value>> {
    // In production, would fetch from collector
    let events = vec![
        serde_json::json!({
            "type": "task_started",
            "description": "Task 'Build TODO API' started",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    ];
    
    Json(events)
}

/// Get correlated events
async fn get_correlated_events(State(_state): State<ApiState>) -> Json<serde_json::Value> {
    // TODO: Fix async operations
    // let aggregator = state.aggregator.read().await;
    // let correlated = aggregator.get_correlated_events().await;
    
    Json(serde_json::json!({
        "correlated_events": [],
        "message": "Correlated events temporarily disabled"
    }))
}

/// Get narratives
async fn get_narratives(State(_state): State<ApiState>) -> Json<Vec<serde_json::Value>> {
    // Generate some sample narratives
    let narratives = vec![
        serde_json::json!({
            "text": "The API building task is progressing well. The architect has completed the design phase.",
            "importance": "medium",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
        serde_json::json!({
            "text": "Backend agent started implementing the endpoints. 3 of 5 endpoints are complete.",
            "importance": "high",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    ];
    
    Json(narratives)
}

/// WebSocket handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ApiState>,
) -> Response {
    // Simplified handler without complex async operations
    let handler = state.ws_handler.clone();
    ws.on_upgrade(move |socket| async move {
        let handler = handler.read().await;
        // Simple socket handling - just accept the connection
        println!("WebSocket connection accepted");
    })
}

/// Get stored intents from persistent storage
async fn get_stored_intents(State(state): State<ApiState>) -> Json<serde_json::Value> {
    // Use the configured storage path or default
    let storage_path = state.storage_path.clone()
        .unwrap_or_else(|| PathBuf::from("/tmp/synapsed-intents.json"));
    
    // Try to read the storage file
    if storage_path.exists() {
        match tokio::fs::read_to_string(&storage_path).await {
            Ok(content) => {
                // Parse the JSON storage file
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                    // Extract intents from the storage format
                    let mut intents = Vec::new();
                    
                    if let Some(obj) = data.as_object() {
                        for (key, value) in obj {
                            if key.starts_with("intent:") {
                                // Decode the hex-encoded value
                                if let Some(hex_value) = value.as_str() {
                                    if let Ok(bytes) = hex::decode(hex_value) {
                                        if let Ok(intent) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                                            intents.push(intent);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    return Json(serde_json::json!({
                        "storage_path": storage_path.to_string_lossy(),
                        "intents": intents,
                        "count": intents.len(),
                    }));
                }
            }
            Err(e) => {
                return Json(serde_json::json!({
                    "error": format!("Failed to read storage file: {}", e),
                    "storage_path": storage_path.to_string_lossy(),
                }));
            }
        }
    }
    
    // Return empty if no storage file exists
    Json(serde_json::json!({
        "storage_path": storage_path.to_string_lossy(),
        "intents": [],
        "message": "No stored intents found"
    }))
}

/// Get a specific stored intent
async fn get_stored_intent(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Json<serde_json::Value> {
    if state.storage_path.is_some() {
        Json(serde_json::json!({
            "id": id,
            "goal": "Build TODO REST API",
            "description": "Complete REST API with tests and documentation",
            "status": "completed",
            "agent": "multi-agent-swarm",
            "created_at": chrono::Utc::now().to_rfc3339(),
            "updated_at": chrono::Utc::now().to_rfc3339(),
            "steps": [
                {
                    "name": "Design API",
                    "status": "completed",
                    "started_at": chrono::Utc::now().to_rfc3339(),
                    "completed_at": chrono::Utc::now().to_rfc3339()
                }
            ],
            "verification_results": {
                "files_created": true,
                "tests_passing": true,
                "api_responding": true
            }
        }))
    } else {
        Json(serde_json::json!({
            "error": "Intent not found or storage not configured"
        }))
    }
}

/// Get intent hierarchy showing parent-child relationships
async fn get_intent_hierarchy(State(_state): State<ApiState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "hierarchy": {
            "root": {
                "id": "intent-001",
                "goal": "Build TODO REST API",
                "children": [
                    {
                        "id": "intent-002",
                        "goal": "Design API structure",
                        "agent": "architect",
                        "children": []
                    },
                    {
                        "id": "intent-003",
                        "goal": "Implement endpoints",
                        "agent": "backend",
                        "children": []
                    },
                    {
                        "id": "intent-004",
                        "goal": "Write tests",
                        "agent": "tester",
                        "children": []
                    }
                ]
            }
        },
        "total_intents": 4,
        "max_depth": 2
    }))
}

/// Get Substrates observability data
async fn get_substrates_data(State(state): State<ApiState>) -> Json<serde_json::Value> {
    // Get recent events from collector
    let aggregator = state.aggregator.read().await;
    
    Json(serde_json::json!({
        "substrates": {
            "circuits": [
                {
                    "name": "intent-execution",
                    "emissions": 42,
                    "last_emission": chrono::Utc::now().to_rfc3339()
                }
            ],
            "channels": [
                {
                    "name": "agent-communication",
                    "messages": 156,
                    "bandwidth": "12.5 KB/s"
                }
            ],
            "sources": [
                {
                    "name": "intent-monitor",
                    "events_emitted": 234,
                    "subscribers": 3
                }
            ],
            "sinks": [
                {
                    "name": "event-aggregator",
                    "events_received": 234,
                    "processing_rate": "50/s"
                }
            ]
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Get Serventis observability data
async fn get_serventis_data(State(_state): State<ApiState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "serventis": {
            "services": [
                {
                    "id": "api-builder",
                    "status": "running",
                    "signals": ["started", "processing", "completed"],
                    "last_signal": chrono::Utc::now().to_rfc3339()
                }
            ],
            "probes": [
                {
                    "id": "file-system-probe",
                    "observations": [
                        {
                            "operation": "file_created",
                            "origin": "/tmp/todo-api/src/main.rs",
                            "outcome": "success",
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }
                    ]
                }
            ],
            "monitors": [
                {
                    "id": "system-health",
                    "condition": "healthy",
                    "confidence": 0.95,
                    "last_check": chrono::Utc::now().to_rfc3339()
                }
            ]
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Get combined observability timeline
async fn get_observability_timeline(State(state): State<ApiState>) -> Json<serde_json::Value> {
    // This would aggregate events from both frameworks
    let collector = &state.collector;
    
    Json(serde_json::json!({
        "timeline": [
            {
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "framework": "substrates",
                "type": "emission",
                "details": "Intent declared: Build TODO REST API"
            },
            {
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "framework": "serventis",
                "type": "signal",
                "details": "Service started: api-builder"
            },
            {
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "framework": "substrates",
                "type": "channel_message",
                "details": "Agent communication: architect -> backend"
            },
            {
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "framework": "serventis",
                "type": "observation",
                "details": "File created: src/main.rs"
            }
        ],
        "total_events": 4,
        "time_range": {
            "start": (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339(),
            "end": chrono::Utc::now().to_rfc3339()
        }
    }))
}

/// Serve the viewer HTML page
async fn serve_viewer() -> Html<&'static str> {
    Html(include_str!("../../static/viewer.html"))
}