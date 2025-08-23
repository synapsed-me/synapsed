//! Dynamic agent context generation for user-defined sub-agents
//! 
//! This module handles the creation of secure contexts for dynamically
//! defined Claude sub-agents while maintaining security boundaries.

use crate::{
    types::*,
    context::IntentContext,
    Result, IntentError,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;

/// Represents a user-defined sub-agent from Claude's markdown files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentDefinition {
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
    pub capabilities: Vec<String>,
    pub custom_instructions: Option<String>,
    pub source_file: Option<PathBuf>,
}

/// Security profile for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSecurityProfile {
    pub tool_name: String,
    pub required_commands: Vec<String>,
    pub required_paths: Vec<String>,
    pub required_endpoints: Vec<String>,
    pub risk_level: RiskLevel,
    pub verification_requirements: Vec<VerificationRequirement>,
    pub resource_requirements: ResourceRequirements,
}

/// Resource requirements for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub min_memory_mb: Option<usize>,
    pub min_cpu_cores: Option<f32>,
    pub min_disk_space_mb: Option<usize>,
    pub network_bandwidth_kbps: Option<usize>,
}

/// Risk levels for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskLevel {
    Minimal,
    Low,
    Medium,
    High,
    Critical,
}

/// Security level for zones
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityLevel {
    Sandbox,    // Highly restricted
    Development, // Moderate restrictions
    Staging,    // Production-like with safeguards
    Production, // Minimal modifications allowed
}

/// Dynamic context generator that creates appropriate bounds for user-defined agents
pub struct DynamicContextGenerator {
    tool_registry: Arc<RwLock<HashMap<String, ToolSecurityProfile>>>,
    risk_analyzer: RiskAnalyzer,
    workspace_zones: Arc<RwLock<WorkspaceZones>>,
    default_restrictions: ContextBounds,
}

impl DynamicContextGenerator {
    pub fn new() -> Self {
        Self {
            tool_registry: Arc::new(RwLock::new(Self::create_default_tool_registry())),
            risk_analyzer: RiskAnalyzer::new(),
            workspace_zones: Arc::new(RwLock::new(WorkspaceZones::create_default())),
            default_restrictions: Self::create_default_restrictions(),
        }
    }

    /// Generate context bounds from a user-defined agent
    pub async fn generate_context_from_agent_definition(
        &self,
        agent_def: &SubAgentDefinition,
        user_trust_level: f64,
    ) -> Result<ContextBounds> {
        let mut bounds = self.default_restrictions.clone();
        let mut total_risk = RiskLevel::Minimal;
        let mut required_resources = ResourceRequirements::default();

        // Analyze each tool the agent has access to
        let registry = self.tool_registry.read().await;
        for tool_name in &agent_def.tools {
            if let Some(tool_profile) = registry.get(tool_name) {
                // Merge permissions
                bounds.allowed_commands.extend(tool_profile.required_commands.clone());
                bounds.allowed_paths.extend(tool_profile.required_paths.clone());
                bounds.allowed_endpoints.extend(tool_profile.required_endpoints.clone());
                
                // Update risk level
                if tool_profile.risk_level > total_risk {
                    total_risk = tool_profile.risk_level;
                }
                
                // Aggregate resource requirements
                Self::aggregate_resources(&mut required_resources, &tool_profile.resource_requirements);
            } else {
                // Unknown tool - apply conservative defaults
                tracing::warn!("Unknown tool requested: {}", tool_name);
                total_risk = RiskLevel::High; // Treat unknown tools as high risk
            }
        }

        // Apply risk-based restrictions
        bounds = self.apply_risk_restrictions(bounds, total_risk, user_trust_level)?;

        // Set resource limits based on requirements
        bounds.max_memory_bytes = required_resources.min_memory_mb.map(|mb| mb * 1024 * 1024);
        bounds.max_cpu_seconds = Some(300); // 5 minute default

        // Assign to appropriate zone based on risk and trust
        let zone = self.determine_zone(total_risk, user_trust_level).await;
        bounds = self.apply_zone_restrictions(bounds, zone)?;

        Ok(bounds)
    }

    /// Create default tool registry with Claude's standard tools
    fn create_default_tool_registry() -> HashMap<String, ToolSecurityProfile> {
        let mut registry = HashMap::new();

        // Read file tool
        registry.insert("read_file".to_string(), ToolSecurityProfile {
            tool_name: "read_file".to_string(),
            required_commands: vec!["cat".to_string(), "head".to_string(), "tail".to_string()],
            required_paths: vec!["${workspace}/**".to_string()],
            required_endpoints: vec![],
            risk_level: RiskLevel::Low,
            verification_requirements: vec![
                VerificationRequirement {
                    verification_type: VerificationType::FileSystem,
                    expected: serde_json::json!({"operation": "read"}),
                    mandatory: false,
                    strategy: VerificationStrategy::Single,
                }
            ],
            resource_requirements: ResourceRequirements {
                min_memory_mb: Some(10),
                min_cpu_cores: Some(0.1),
                min_disk_space_mb: None,
                network_bandwidth_kbps: None,
            },
        });

        // Write file tool
        registry.insert("write_file".to_string(), ToolSecurityProfile {
            tool_name: "write_file".to_string(),
            required_commands: vec!["echo".to_string(), "tee".to_string()],
            required_paths: vec!["${workspace}/**".to_string()],
            required_endpoints: vec![],
            risk_level: RiskLevel::Medium,
            verification_requirements: vec![
                VerificationRequirement {
                    verification_type: VerificationType::FileSystem,
                    expected: serde_json::json!({"operation": "write", "backup": true}),
                    mandatory: true,
                    strategy: VerificationStrategy::Single,
                }
            ],
            resource_requirements: ResourceRequirements {
                min_memory_mb: Some(50),
                min_cpu_cores: Some(0.2),
                min_disk_space_mb: Some(100),
                network_bandwidth_kbps: None,
            },
        });

        // Execute command tool
        registry.insert("run_command".to_string(), ToolSecurityProfile {
            tool_name: "run_command".to_string(),
            required_commands: vec![], // Dynamic based on needs
            required_paths: vec!["${workspace}/**".to_string()],
            required_endpoints: vec![],
            risk_level: RiskLevel::High,
            verification_requirements: vec![
                VerificationRequirement {
                    verification_type: VerificationType::Command,
                    expected: serde_json::json!({"sandboxed": true}),
                    mandatory: true,
                    strategy: VerificationStrategy::All,
                }
            ],
            resource_requirements: ResourceRequirements {
                min_memory_mb: Some(100),
                min_cpu_cores: Some(0.5),
                min_disk_space_mb: Some(500),
                network_bandwidth_kbps: None,
            },
        });

        // Web search tool
        registry.insert("web_search".to_string(), ToolSecurityProfile {
            tool_name: "web_search".to_string(),
            required_commands: vec!["curl".to_string(), "wget".to_string()],
            required_paths: vec!["/tmp/**".to_string()],
            required_endpoints: vec![
                "https://www.google.com".to_string(),
                "https://api.duckduckgo.com".to_string(),
            ],
            risk_level: RiskLevel::Low,
            verification_requirements: vec![
                VerificationRequirement {
                    verification_type: VerificationType::Network,
                    expected: serde_json::json!({"https_only": true}),
                    mandatory: true,
                    strategy: VerificationStrategy::Single,
                }
            ],
            resource_requirements: ResourceRequirements {
                min_memory_mb: Some(50),
                min_cpu_cores: Some(0.1),
                min_disk_space_mb: None,
                network_bandwidth_kbps: Some(1000),
            },
        });

        registry
    }

    /// Create default restrictions for all agents
    fn create_default_restrictions() -> ContextBounds {
        ContextBounds {
            allowed_paths: vec!["/tmp".to_string()], // Minimal by default
            allowed_commands: vec!["echo".to_string(), "pwd".to_string()], // Safe commands
            allowed_endpoints: vec![], // No network by default
            max_memory_bytes: Some(50 * 1024 * 1024), // 50MB default
            max_cpu_seconds: Some(60), // 1 minute default
            env_vars: HashMap::new(),
        }
    }

    /// Apply restrictions based on risk level and trust
    fn apply_risk_restrictions(
        &self,
        mut bounds: ContextBounds,
        risk: RiskLevel,
        trust: f64,
    ) -> Result<ContextBounds> {
        match (risk, trust) {
            (RiskLevel::Critical, t) if t < 0.9 => {
                // High risk, low trust: Maximum restrictions
                bounds.max_memory_bytes = Some(10 * 1024 * 1024); // 10MB
                bounds.max_cpu_seconds = Some(10); // 10 seconds
                bounds.allowed_endpoints.clear(); // No network
            },
            (RiskLevel::High, t) if t < 0.7 => {
                // High risk, medium trust: Strong restrictions
                bounds.max_memory_bytes = Some(50 * 1024 * 1024); // 50MB
                bounds.max_cpu_seconds = Some(30); // 30 seconds
            },
            (RiskLevel::Medium, _) => {
                // Medium risk: Moderate restrictions
                bounds.max_memory_bytes = Some(100 * 1024 * 1024); // 100MB
                bounds.max_cpu_seconds = Some(60); // 1 minute
            },
            (RiskLevel::Low, t) if t > 0.8 => {
                // Low risk, high trust: Relaxed restrictions
                bounds.max_memory_bytes = Some(500 * 1024 * 1024); // 500MB
                bounds.max_cpu_seconds = Some(300); // 5 minutes
            },
            _ => {} // Keep defaults
        }
        
        Ok(bounds)
    }

    /// Determine which zone an agent should operate in
    async fn determine_zone(&self, risk: RiskLevel, trust: f64) -> SecurityLevel {
        match (risk, trust) {
            (RiskLevel::Critical, _) => SecurityLevel::Sandbox,
            (RiskLevel::High, t) if t < 0.5 => SecurityLevel::Sandbox,
            (RiskLevel::High, _) => SecurityLevel::Development,
            (RiskLevel::Medium, t) if t < 0.7 => SecurityLevel::Development,
            (RiskLevel::Medium, t) if t > 0.8 => SecurityLevel::Staging,
            (RiskLevel::Low, t) if t > 0.9 => SecurityLevel::Staging,
            _ => SecurityLevel::Development,
        }
    }

    /// Apply zone-specific restrictions
    fn apply_zone_restrictions(
        &self,
        mut bounds: ContextBounds,
        zone: SecurityLevel,
    ) -> Result<ContextBounds> {
        match zone {
            SecurityLevel::Sandbox => {
                // Sandbox: Maximum isolation
                bounds.allowed_paths = vec!["/tmp/sandbox".to_string()];
                bounds.allowed_endpoints.clear();
            },
            SecurityLevel::Development => {
                // Development: Workspace access
                bounds.allowed_paths = vec![
                    "/workspace".to_string(),
                    "/tmp".to_string(),
                ];
            },
            SecurityLevel::Staging => {
                // Staging: Near-production
                bounds.allowed_paths = vec![
                    "/workspace".to_string(),
                    "/staging".to_string(),
                    "/tmp".to_string(),
                ];
            },
            SecurityLevel::Production => {
                // Production: Read-only by default
                // Would require explicit permission escalation
                bounds.allowed_commands = vec![
                    "cat".to_string(),
                    "ls".to_string(),
                    "grep".to_string(),
                ];
            },
        }
        
        Ok(bounds)
    }

    /// Aggregate resource requirements
    fn aggregate_resources(total: &mut ResourceRequirements, new: &ResourceRequirements) {
        if let Some(mem) = new.min_memory_mb {
            total.min_memory_mb = Some(total.min_memory_mb.unwrap_or(0) + mem);
        }
        if let Some(cpu) = new.min_cpu_cores {
            total.min_cpu_cores = Some(total.min_cpu_cores.unwrap_or(0.0) + cpu);
        }
        if let Some(disk) = new.min_disk_space_mb {
            total.min_disk_space_mb = Some(total.min_disk_space_mb.unwrap_or(0) + disk);
        }
        if let Some(bw) = new.network_bandwidth_kbps {
            total.network_bandwidth_kbps = Some(total.network_bandwidth_kbps.unwrap_or(0) + bw);
        }
    }
}

/// Analyzes risk of operations
pub struct RiskAnalyzer {
    dangerous_commands: HashSet<String>,
    dangerous_paths: HashSet<String>,
    dangerous_patterns: Vec<String>,
}

impl RiskAnalyzer {
    pub fn new() -> Self {
        let mut dangerous_commands = HashSet::new();
        dangerous_commands.insert("rm".to_string());
        dangerous_commands.insert("sudo".to_string());
        dangerous_commands.insert("eval".to_string());
        dangerous_commands.insert("exec".to_string());
        dangerous_commands.insert("kill".to_string());
        dangerous_commands.insert("pkill".to_string());
        dangerous_commands.insert("shutdown".to_string());
        dangerous_commands.insert("reboot".to_string());

        let mut dangerous_paths = HashSet::new();
        dangerous_paths.insert("/etc".to_string());
        dangerous_paths.insert("/sys".to_string());
        dangerous_paths.insert("/proc".to_string());
        dangerous_paths.insert("/boot".to_string());
        dangerous_paths.insert("/root".to_string());

        let dangerous_patterns = vec![
            ":(){ :|:& };:".to_string(), // Fork bomb
            "rm -rf /".to_string(),
            "> /dev/sda".to_string(),
        ];

        Self {
            dangerous_commands,
            dangerous_paths,
            dangerous_patterns,
        }
    }

    pub fn assess_bounds(&self, bounds: &ContextBounds) -> RiskLevel {
        let mut risk = RiskLevel::Minimal;

        // Check for dangerous commands
        for cmd in &bounds.allowed_commands {
            if self.dangerous_commands.contains(cmd) {
                return RiskLevel::Critical;
            }
        }

        // Check for dangerous paths
        for path in &bounds.allowed_paths {
            for dangerous in &self.dangerous_paths {
                if path.starts_with(dangerous) {
                    risk = RiskLevel::High;
                }
            }
        }

        // Check for unlimited resources
        if bounds.max_memory_bytes.is_none() || bounds.max_cpu_seconds.is_none() {
            if risk < RiskLevel::Medium {
                risk = RiskLevel::Medium;
            }
        }

        // Check for network access
        if !bounds.allowed_endpoints.is_empty() {
            if risk < RiskLevel::Low {
                risk = RiskLevel::Low;
            }
        }

        risk
    }
}

/// Workspace zones for agent isolation
pub struct WorkspaceZones {
    pub zones: HashMap<String, Zone>,
}

/// A security zone in the workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub name: String,
    pub path: PathBuf,
    pub security_level: SecurityLevel,
    pub allowed_operations: Vec<Operation>,
    pub max_agents: usize,
    pub current_agents: Vec<String>,
}

/// Operations that can be performed in a zone
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    Read,
    Write,
    Execute,
    CreateFile,
    DeleteFile,
    ModifyPermissions,
    NetworkAccess,
}

impl WorkspaceZones {
    pub fn create_default() -> Self {
        let mut zones = HashMap::new();

        // Sandbox zone - highly restricted
        zones.insert("sandbox".to_string(), Zone {
            name: "sandbox".to_string(),
            path: PathBuf::from("/tmp/sandbox"),
            security_level: SecurityLevel::Sandbox,
            allowed_operations: vec![Operation::Read, Operation::Write],
            max_agents: 10,
            current_agents: vec![],
        });

        // Development zone - moderate restrictions
        zones.insert("development".to_string(), Zone {
            name: "development".to_string(),
            path: PathBuf::from("/workspace"),
            security_level: SecurityLevel::Development,
            allowed_operations: vec![
                Operation::Read,
                Operation::Write,
                Operation::Execute,
                Operation::CreateFile,
            ],
            max_agents: 5,
            current_agents: vec![],
        });

        // Staging zone - production-like
        zones.insert("staging".to_string(), Zone {
            name: "staging".to_string(),
            path: PathBuf::from("/staging"),
            security_level: SecurityLevel::Staging,
            allowed_operations: vec![
                Operation::Read,
                Operation::Write,
                Operation::Execute,
                Operation::NetworkAccess,
            ],
            max_agents: 3,
            current_agents: vec![],
        });

        // Production zone - minimal access
        zones.insert("production".to_string(), Zone {
            name: "production".to_string(),
            path: PathBuf::from("/production"),
            security_level: SecurityLevel::Production,
            allowed_operations: vec![Operation::Read],
            max_agents: 2,
            current_agents: vec![],
        });

        Self { zones }
    }

    /// Assign an agent to a zone
    pub fn assign_agent_to_zone(
        &mut self,
        agent_id: &str,
        zone_name: &str,
    ) -> Result<()> {
        if let Some(zone) = self.zones.get_mut(zone_name) {
            if zone.current_agents.len() >= zone.max_agents {
                return Err(IntentError::ValidationFailed(
                    format!("Zone {} is at capacity", zone_name)
                ));
            }
            zone.current_agents.push(agent_id.to_string());
            Ok(())
        } else {
            Err(IntentError::ValidationFailed(
                format!("Zone {} not found", zone_name)
            ))
        }
    }
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            min_memory_mb: None,
            min_cpu_cores: None,
            min_disk_space_mb: None,
            network_bandwidth_kbps: None,
        }
    }
}