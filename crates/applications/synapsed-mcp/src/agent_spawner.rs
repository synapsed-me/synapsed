//! Agent spawning and management

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Agent information
#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: String,
    pub agent_type: String,
    pub intent_id: Option<String>,
    pub status: AgentStatus,
    pub process_id: Option<u32>,
    pub created_at: std::time::SystemTime,
}

/// Agent status
#[derive(Debug, Clone)]
pub enum AgentStatus {
    Spawning,
    Running,
    Completed,
    Failed,
    Terminated,
}

/// Agent spawner
pub struct AgentSpawner {
    agents: Arc<RwLock<HashMap<String, AgentInfo>>>,
    intent_store_path: Option<String>,
}

impl AgentSpawner {
    /// Create a new agent spawner
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            intent_store_path: std::env::var("INTENT_STORE_PATH").ok(),
        }
    }

    /// Spawn a new agent
    pub async fn spawn_agent(
        &self,
        agent_type: String,
        config: Option<Value>,
        intent_id: Option<String>,
    ) -> Result<String> {
        let agent_id = Uuid::new_v4().to_string();
        
        // Create agent info
        let agent_info = AgentInfo {
            id: agent_id.clone(),
            agent_type: agent_type.clone(),
            intent_id: intent_id.clone(),
            status: AgentStatus::Spawning,
            process_id: None,
            created_at: std::time::SystemTime::now(),
        };
        
        // Store agent info
        {
            let mut agents = self.agents.write().await;
            agents.insert(agent_id.clone(), agent_info.clone());
        }
        
        // Spawn the agent process
        let agent_id_clone = agent_id.clone();
        let agents = self.agents.clone();
        let intent_store_path = self.intent_store_path.clone();
        
        tokio::spawn(async move {
            match Self::spawn_agent_process(
                &agent_type,
                &agent_id_clone,
                config,
                intent_id,
                intent_store_path,
            ).await {
                Ok(pid) => {
                    let mut agents = agents.write().await;
                    if let Some(agent) = agents.get_mut(&agent_id_clone) {
                        agent.status = AgentStatus::Running;
                        agent.process_id = Some(pid);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to spawn agent process: {}", e);
                    let mut agents = agents.write().await;
                    if let Some(agent) = agents.get_mut(&agent_id_clone) {
                        agent.status = AgentStatus::Failed;
                    }
                }
            }
        });
        
        Ok(agent_id)
    }

    /// Spawn the actual agent process
    async fn spawn_agent_process(
        agent_type: &str,
        agent_id: &str,
        config: Option<Value>,
        intent_id: Option<String>,
        intent_store_path: Option<String>,
    ) -> Result<u32> {
        // Extract workspace path from config
        let workspace_path = config.as_ref()
            .and_then(|c| c.get("workspace"))
            .and_then(|w| w.as_str())
            .unwrap_or("/tmp")
            .to_string();
        
        // Build environment variables
        let mut env_vars = vec![
            ("AGENT_ID".to_string(), agent_id.to_string()),
            ("AGENT_TYPE".to_string(), agent_type.to_string()),
            ("WORKSPACE".to_string(), workspace_path.clone()),
        ];
        
        if let Some(ref intent_id) = intent_id {
            env_vars.push(("INTENT_ID".to_string(), intent_id.clone()));
        }
        
        if let Some(store_path) = intent_store_path {
            env_vars.push(("INTENT_STORE_PATH".to_string(), store_path));
        }
        
        if let Some(ref config) = config {
            env_vars.push(("AGENT_CONFIG".to_string(), config.to_string()));
        }
        
        // Store intent_id for use in args
        let intent_id_str = intent_id.unwrap_or_default();
        
        // For the demo, we'll spawn actual agent executables
        // In production, this would spawn Claude sub-agents using the Claude API
        let (command, args) = match agent_type {
            "architect" | "backend" | "tester" | "documenter" | "reviewer" => {
                // Spawn the live-demo agent runner with the specific agent type
                ("cargo", vec![
                    "run".to_string(), 
                    "--bin".to_string(), "agent-runner".to_string(),
                    "--".to_string(), 
                    "--agent-type".to_string(), agent_type.to_string(),
                    "--workspace".to_string(), workspace_path,
                    "--intent-id".to_string(), intent_id_str,
                ])
            }
            "claude" => {
                // For real Claude sub-agents, we would use the Claude API
                // For now, simulate with a script
                ("echo", vec!["Claude agent would be spawned here".to_string()])
            }
            _ => {
                // Default to a generic agent runner
                ("cargo", vec![
                    "run".to_string(), 
                    "--bin".to_string(), 
                    "agent-runner".to_string(), 
                    "--".to_string(), 
                    "--agent-type".to_string(), 
                    agent_type.to_string()
                ])
            }
        };
        
        // Spawn the process
        let mut child = Command::new(command)
            .args(&args)
            .envs(env_vars)
            .current_dir("/workspaces/synapsed/examples/live-demo")
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn process: {}", e))?;
        
        // Get the process ID
        let pid = child.id().ok_or_else(|| anyhow!("Failed to get process ID"))?;
        
        // Don't wait for the process to complete - let it run in the background
        tokio::spawn(async move {
            let _ = child.wait().await;
        });
        
        Ok(pid)
    }

    /// Get agent status
    pub async fn get_agent_status(&self, agent_id: &str) -> Result<Value> {
        let agents = self.agents.read().await;
        
        if let Some(agent) = agents.get(agent_id) {
            Ok(serde_json::json!({
                "agent_id": agent.id,
                "agent_type": agent.agent_type,
                "intent_id": agent.intent_id,
                "status": format!("{:?}", agent.status),
                "process_id": agent.process_id,
                "created_at": agent.created_at.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default().as_secs(),
            }))
        } else {
            Err(anyhow!("Agent not found"))
        }
    }

    /// Terminate an agent
    pub async fn terminate_agent(&self, agent_id: &str) -> Result<()> {
        let mut agents = self.agents.write().await;
        
        if let Some(agent) = agents.get_mut(agent_id) {
            // If there's a process ID, try to kill the process
            if let Some(pid) = agent.process_id {
                // Use system call to terminate the process
                let _ = Command::new("kill")
                    .arg("-TERM")
                    .arg(pid.to_string())
                    .output()
                    .await;
            }
            
            agent.status = AgentStatus::Terminated;
            Ok(())
        } else {
            Err(anyhow!("Agent not found"))
        }
    }

    /// List all agents
    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }
}