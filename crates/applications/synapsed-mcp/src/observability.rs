//! Observability integration using Substrates event circuits

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use synapsed_substrates::{
    async_trait, BasicCircuit, BasicSink, Circuit, Cortex, Name, Pipe, Script, Subject,
    SubstratesResult, create_cortex
};
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use uuid::Uuid;

/// MCP-specific events that can be observed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum McpEvent {
    /// Intent declaration events
    IntentDeclared {
        intent_id: Uuid,
        goal: String,
        steps_count: usize,
        timestamp: DateTime<Utc>,
        agent_id: Option<String>,
    },
    
    /// Intent verification events
    IntentVerified {
        intent_id: Uuid,
        success: bool,
        evidence: serde_json::Value,
        timestamp: DateTime<Utc>,
        agent_id: Option<String>,
    },
    
    /// Intent status change events
    IntentStatusChanged {
        intent_id: String,
        old_status: String,
        new_status: String,
        timestamp: DateTime<Utc>,
        step_name: Option<String>,
        error: Option<String>,
    },
    
    /// Agent spawning events
    AgentSpawned {
        agent_id: String,
        agent_type: String,
        intent_id: Option<String>,
        timestamp: DateTime<Utc>,
        config: Option<serde_json::Value>,
    },
    
    /// Agent status change events
    AgentStatusChanged {
        agent_id: String,
        old_status: String,
        new_status: String,
        timestamp: DateTime<Utc>,
        process_id: Option<u32>,
    },
    
    /// Agent terminated events
    AgentTerminated {
        agent_id: String,
        reason: String,
        timestamp: DateTime<Utc>,
        success: bool,
    },
    
    /// Trust check events
    TrustChecked {
        agent_id: Uuid,
        trust_level: f64,
        reputation: String,
        timestamp: DateTime<Utc>,
        promises_fulfilled: u32,
        promises_broken: u32,
    },
    
    /// Context injection events
    ContextInjected {
        parent_agent_id: Option<String>,
        child_agent_id: String,
        context_size: usize,
        timestamp: DateTime<Utc>,
        success: bool,
    },
    
    /// Server lifecycle events
    ServerStarted {
        server_name: String,
        version: String,
        timestamp: DateTime<Utc>,
        config: serde_json::Value,
    },
    
    /// Request handling events
    RequestHandled {
        method: String,
        request_id: String,
        success: bool,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
        error: Option<String>,
    },
}

/// Event circuit manager for MCP server observability
pub struct McpEventCircuit {
    cortex: Arc<dyn Cortex>,
    circuit: Arc<dyn Circuit>,
    file_sink: Arc<BasicSink<McpEvent>>,
    log_file_path: String,
}

impl McpEventCircuit {
    /// Create a new MCP event circuit with file logging
    pub async fn new(log_file_path: String) -> SubstratesResult<Self> {
        // Create cortex and circuit
        let cortex = create_cortex();
        let circuit = cortex.circuit_named(Name::from_part("mcp_events")).await?;
        
        // Create a sink for logging events to file
        let sink_subject = Subject::new(Name::from_part("file_logger"), synapsed_substrates::types::SubjectType::Resource);
        let file_sink = Arc::new(BasicSink::new(sink_subject));
        
        // Ensure log directory exists
        if let Some(parent) = std::path::Path::new(&log_file_path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                synapsed_substrates::types::SubstratesError::Internal(format!("Failed to create log directory: {}", e))
            })?;
        }
        
        Ok(Self {
            cortex,
            circuit,
            file_sink,
            log_file_path,
        })
    }
    
    /// Emit an event to all subscribers
    pub async fn emit_event(&self, event: McpEvent) -> SubstratesResult<()> {
        // Log to file
        self.log_to_file(&event).await?;
        
        // Forward to the sink
        let mut pipe = self.file_sink.create_pipe();
        pipe.emit(event).await?;
        
        Ok(())
    }
    
    /// Log event to file in JSON format
    async fn log_to_file(&self, event: &McpEvent) -> SubstratesResult<()> {
        let json_event = serde_json::to_string(event).map_err(|e| {
            synapsed_substrates::types::SubstratesError::Internal(format!("JSON serialization failed: {}", e))
        })?;
        
        // Append to file in a background task to avoid blocking
        let file_path = self.log_file_path.clone();
        let json_line = format!("{}\n", json_event);
        
        tokio::spawn(async move {
            if let Err(e) = Self::write_to_file(&file_path, &json_line).await {
                error!("Failed to write event to log file {}: {}", file_path, e);
            }
        });
        
        Ok(())
    }
    
    /// Write content to file (async)
    async fn write_to_file(file_path: &str, content: &str) -> Result<(), std::io::Error> {
        tokio::task::spawn_blocking({
            let file_path = file_path.to_string();
            let content = content.to_string();
            move || -> Result<(), std::io::Error> {
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&file_path)?;
                file.write_all(content.as_bytes())?;
                file.flush()?;
                Ok(())
            }
        }).await?
    }
    
    /// Drain all events from the sink (for testing/monitoring)
    pub async fn drain_events(&self) -> SubstratesResult<Vec<McpEvent>> {
        let mut sink = Arc::clone(&self.file_sink);
        // Convert captures to events - note: this requires unsafe coercion for Arc<BasicSink<E>>
        // In a real implementation, we'd need a more sophisticated approach
        // For now, we'll return an empty vec and rely on file logging
        Ok(Vec::new())
    }
    
    /// Get the circuit for external integration
    pub fn circuit(&self) -> Arc<dyn Circuit> {
        self.circuit.clone()
    }
    
    /// Get the cortex for external integration  
    pub fn cortex(&self) -> Arc<dyn Cortex> {
        self.cortex.clone()
    }
}

/// Script that logs an event
pub struct EventLoggingScript {
    event: McpEvent,
    circuit: Arc<McpEventCircuit>,
}

impl EventLoggingScript {
    pub fn new(event: McpEvent, circuit: Arc<McpEventCircuit>) -> Self {
        Self { event, circuit }
    }
}

#[async_trait]
impl Script for EventLoggingScript {
    async fn exec(&self, _current: &dyn synapsed_substrates::Current) -> SubstratesResult<()> {
        debug!("Executing event logging script for: {:?}", self.event);
        self.circuit.emit_event(self.event.clone()).await
    }
}

/// Helper functions for creating common events
impl McpEvent {
    /// Create an intent declared event
    pub fn intent_declared(intent_id: Uuid, goal: String, steps_count: usize, agent_id: Option<String>) -> Self {
        Self::IntentDeclared {
            intent_id,
            goal,
            steps_count,
            timestamp: Utc::now(),
            agent_id,
        }
    }
    
    /// Create an intent verified event
    pub fn intent_verified(intent_id: Uuid, success: bool, evidence: serde_json::Value, agent_id: Option<String>) -> Self {
        Self::IntentVerified {
            intent_id,
            success,
            evidence,
            timestamp: Utc::now(),
            agent_id,
        }
    }
    
    /// Create an intent status changed event
    pub fn intent_status_changed(intent_id: String, old_status: String, new_status: String, step_name: Option<String>, error: Option<String>) -> Self {
        Self::IntentStatusChanged {
            intent_id,
            old_status,
            new_status,
            timestamp: Utc::now(),
            step_name,
            error,
        }
    }
    
    /// Create an agent spawned event
    pub fn agent_spawned(agent_id: String, agent_type: String, intent_id: Option<String>, config: Option<serde_json::Value>) -> Self {
        Self::AgentSpawned {
            agent_id,
            agent_type,
            intent_id,
            timestamp: Utc::now(),
            config,
        }
    }
    
    /// Create an agent status changed event
    pub fn agent_status_changed(agent_id: String, old_status: String, new_status: String, process_id: Option<u32>) -> Self {
        Self::AgentStatusChanged {
            agent_id,
            old_status,
            new_status,
            timestamp: Utc::now(),
            process_id,
        }
    }
    
    /// Create an agent terminated event
    pub fn agent_terminated(agent_id: String, reason: String, success: bool) -> Self {
        Self::AgentTerminated {
            agent_id,
            reason,
            timestamp: Utc::now(),
            success,
        }
    }
    
    /// Create a trust checked event
    pub fn trust_checked(agent_id: Uuid, trust_level: f64, reputation: String, promises_fulfilled: u32, promises_broken: u32) -> Self {
        Self::TrustChecked {
            agent_id,
            trust_level,
            reputation,
            timestamp: Utc::now(),
            promises_fulfilled,
            promises_broken,
        }
    }
    
    /// Create a context injected event
    pub fn context_injected(parent_agent_id: Option<String>, child_agent_id: String, context_size: usize, success: bool) -> Self {
        Self::ContextInjected {
            parent_agent_id,
            child_agent_id,
            context_size,
            timestamp: Utc::now(),
            success,
        }
    }
    
    /// Create a server started event
    pub fn server_started(server_name: String, version: String, config: serde_json::Value) -> Self {
        Self::ServerStarted {
            server_name,
            version,
            timestamp: Utc::now(),
            config,
        }
    }
    
    /// Create a request handled event
    pub fn request_handled(method: String, request_id: String, success: bool, duration_ms: u64, error: Option<String>) -> Self {
        Self::RequestHandled {
            method,
            request_id,
            success,
            duration_ms,
            timestamp: Utc::now(),
            error,
        }
    }
}

/// Global event circuit shared across the MCP server
pub struct SharedEventCircuit {
    circuit: Arc<RwLock<Option<Arc<McpEventCircuit>>>>,
}

impl SharedEventCircuit {
    /// Create new shared event circuit
    pub fn new() -> Self {
        Self {
            circuit: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Initialize the event circuit
    pub async fn initialize(&self, log_file_path: String) -> SubstratesResult<()> {
        let circuit = McpEventCircuit::new(log_file_path).await?;
        *self.circuit.write().await = Some(Arc::new(circuit));
        info!("MCP event circuit initialized");
        Ok(())
    }
    
    /// Emit an event if circuit is initialized
    pub async fn emit_event(&self, event: McpEvent) -> SubstratesResult<()> {
        if let Some(circuit) = self.circuit.read().await.as_ref() {
            circuit.emit_event(event).await?;
        }
        Ok(())
    }
    
    /// Get the circuit if initialized
    pub async fn circuit(&self) -> Option<Arc<McpEventCircuit>> {
        self.circuit.read().await.clone()
    }
}

impl Default for SharedEventCircuit {
    fn default() -> Self {
        Self::new()
    }
}

/// Global static instance for easy access
lazy_static::lazy_static! {
    pub static ref EVENT_CIRCUIT: SharedEventCircuit = SharedEventCircuit::new();
}