//! Tool registry with comprehensive security profiles for dynamic agents
//! 
//! This module provides a registry of tools with detailed security profiles,
//! permission requirements, and verification strategies.

use crate::{
    types::*,
    dynamic_agents::{ToolSecurityProfile, ResourceRequirements, RiskLevel},
    Result, IntentError,
};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use chrono::{DateTime, Utc};

/// Tool registry that manages available tools and their security profiles
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, ToolSecurityProfile>>>,
    custom_tools: Arc<RwLock<HashMap<String, CustomTool>>>,
    tool_categories: Arc<RwLock<HashMap<String, ToolCategory>>>,
    permission_matrix: Arc<RwLock<PermissionMatrix>>,
}

/// Custom tool defined by users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTool {
    pub name: String,
    pub description: String,
    pub implementation: ToolImplementation,
    pub security_profile: ToolSecurityProfile,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub approved: bool,
    pub approval_reason: Option<String>,
}

/// Tool implementation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolImplementation {
    /// Shell command
    Command {
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    /// Python script
    PythonScript {
        script: String,
        imports: Vec<String>,
    },
    /// JavaScript function
    JavaScript {
        code: String,
        npm_packages: Vec<String>,
    },
    /// API endpoint
    ApiEndpoint {
        url: String,
        method: String,
        headers: HashMap<String, String>,
    },
    /// Composite tool combining multiple tools
    Composite {
        steps: Vec<String>,
        parallel: bool,
    },
}

/// Tool categories for organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCategory {
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
    pub default_risk_level: RiskLevel,
    pub requires_approval: bool,
}

/// Permission matrix for tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionMatrix {
    /// Maps tool names to required permissions
    pub tool_permissions: HashMap<String, RequiredPermissions>,
    /// Maps user trust levels to allowed permissions
    pub trust_permissions: HashMap<String, AllowedPermissions>,
}

/// Required permissions for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredPermissions {
    pub filesystem: FilesystemPermissions,
    pub network: NetworkPermissions,
    pub process: ProcessPermissions,
    pub system: SystemPermissions,
}

/// Filesystem permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemPermissions {
    pub read_paths: Vec<String>,
    pub write_paths: Vec<String>,
    pub delete_paths: Vec<String>,
    pub create_paths: Vec<String>,
    pub max_file_size_mb: Option<usize>,
}

/// Network permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPermissions {
    pub allowed_hosts: Vec<String>,
    pub allowed_ports: Vec<u16>,
    pub allowed_protocols: Vec<String>,
    pub max_bandwidth_kbps: Option<usize>,
    pub timeout_seconds: Option<u64>,
}

/// Process permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessPermissions {
    pub can_spawn: bool,
    pub allowed_executables: Vec<String>,
    pub max_processes: Option<usize>,
    pub cpu_limit_percent: Option<f32>,
    pub memory_limit_mb: Option<usize>,
}

/// System permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPermissions {
    pub can_read_env: bool,
    pub can_modify_env: bool,
    pub allowed_env_vars: Vec<String>,
    pub can_access_hardware: bool,
    pub can_modify_system_settings: bool,
}

/// Allowed permissions based on trust level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowedPermissions {
    pub trust_level_min: f64,
    pub trust_level_max: f64,
    pub allowed_categories: Vec<String>,
    pub forbidden_tools: Vec<String>,
    pub max_risk_level: RiskLevel,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(Self::create_claude_tools())),
            custom_tools: Arc::new(RwLock::new(HashMap::new())),
            tool_categories: Arc::new(RwLock::new(Self::create_default_categories())),
            permission_matrix: Arc::new(RwLock::new(Self::create_permission_matrix())),
        }
    }

    /// Register a custom tool
    pub async fn register_custom_tool(
        &self,
        tool: CustomTool,
        user_trust_level: f64,
    ) -> Result<()> {
        // Check if user has permission to register tools
        if user_trust_level < 0.5 {
            return Err(IntentError::PermissionDenied(
                "Insufficient trust level to register custom tools".to_string()
            ));
        }

        // Validate tool implementation
        self.validate_tool_implementation(&tool.implementation)?;

        // Check risk level
        if tool.security_profile.risk_level >= RiskLevel::High && user_trust_level < 0.8 {
            return Err(IntentError::PermissionDenied(
                "High-risk tools require trust level >= 0.8".to_string()
            ));
        }

        // Add to custom tools
        let mut custom_tools = self.custom_tools.write().await;
        custom_tools.insert(tool.name.clone(), tool);

        Ok(())
    }

    /// Get tool security profile
    pub async fn get_tool_profile(&self, tool_name: &str) -> Option<ToolSecurityProfile> {
        // Check standard tools first
        let tools = self.tools.read().await;
        if let Some(profile) = tools.get(tool_name) {
            return Some(profile.clone());
        }

        // Check custom tools
        let custom_tools = self.custom_tools.read().await;
        if let Some(custom) = custom_tools.get(tool_name) {
            if custom.approved {
                return Some(custom.security_profile.clone());
            }
        }

        None
    }

    /// Calculate composite permissions for multiple tools
    pub async fn calculate_composite_permissions(
        &self,
        tool_names: &[String],
    ) -> Result<RequiredPermissions> {
        let mut composite = RequiredPermissions {
            filesystem: FilesystemPermissions {
                read_paths: vec![],
                write_paths: vec![],
                delete_paths: vec![],
                create_paths: vec![],
                max_file_size_mb: None,
            },
            network: NetworkPermissions {
                allowed_hosts: vec![],
                allowed_ports: vec![],
                allowed_protocols: vec![],
                max_bandwidth_kbps: None,
                timeout_seconds: None,
            },
            process: ProcessPermissions {
                can_spawn: false,
                allowed_executables: vec![],
                max_processes: None,
                cpu_limit_percent: None,
                memory_limit_mb: None,
            },
            system: SystemPermissions {
                can_read_env: false,
                can_modify_env: false,
                allowed_env_vars: vec![],
                can_access_hardware: false,
                can_modify_system_settings: false,
            },
        };

        let matrix = self.permission_matrix.read().await;
        
        for tool_name in tool_names {
            if let Some(perms) = matrix.tool_permissions.get(tool_name) {
                // Merge filesystem permissions
                composite.filesystem.read_paths.extend(perms.filesystem.read_paths.clone());
                composite.filesystem.write_paths.extend(perms.filesystem.write_paths.clone());
                composite.filesystem.delete_paths.extend(perms.filesystem.delete_paths.clone());
                composite.filesystem.create_paths.extend(perms.filesystem.create_paths.clone());
                
                // Take maximum file size
                if let Some(size) = perms.filesystem.max_file_size_mb {
                    composite.filesystem.max_file_size_mb = Some(
                        composite.filesystem.max_file_size_mb.unwrap_or(0).max(size)
                    );
                }

                // Merge network permissions
                composite.network.allowed_hosts.extend(perms.network.allowed_hosts.clone());
                composite.network.allowed_ports.extend(perms.network.allowed_ports.clone());
                composite.network.allowed_protocols.extend(perms.network.allowed_protocols.clone());

                // Merge process permissions
                composite.process.can_spawn |= perms.process.can_spawn;
                composite.process.allowed_executables.extend(perms.process.allowed_executables.clone());

                // Merge system permissions
                composite.system.can_read_env |= perms.system.can_read_env;
                composite.system.can_modify_env |= perms.system.can_modify_env;
                composite.system.allowed_env_vars.extend(perms.system.allowed_env_vars.clone());
                composite.system.can_access_hardware |= perms.system.can_access_hardware;
                composite.system.can_modify_system_settings |= perms.system.can_modify_system_settings;
            }
        }

        // Deduplicate
        composite.filesystem.read_paths.sort();
        composite.filesystem.read_paths.dedup();
        composite.filesystem.write_paths.sort();
        composite.filesystem.write_paths.dedup();
        composite.network.allowed_hosts.sort();
        composite.network.allowed_hosts.dedup();

        Ok(composite)
    }

    /// Create standard Claude tools
    fn create_claude_tools() -> HashMap<String, ToolSecurityProfile> {
        let mut tools = HashMap::new();

        // File manipulation tools
        tools.insert("str_replace".to_string(), ToolSecurityProfile {
            tool_name: "str_replace".to_string(),
            required_commands: vec!["sed".to_string()],
            required_paths: vec!["${workspace}/**".to_string()],
            required_endpoints: vec![],
            risk_level: RiskLevel::Medium,
            verification_requirements: vec![
                VerificationRequirement {
                    verification_type: VerificationType::FileSystem,
                    expected: serde_json::json!({"operation": "modify", "backup": true}),
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

        // Debugging tools
        tools.insert("debug_shell".to_string(), ToolSecurityProfile {
            tool_name: "debug_shell".to_string(),
            required_commands: vec!["bash".to_string(), "sh".to_string()],
            required_paths: vec!["${workspace}/**".to_string()],
            required_endpoints: vec![],
            risk_level: RiskLevel::High,
            verification_requirements: vec![
                VerificationRequirement {
                    verification_type: VerificationType::Command,
                    expected: serde_json::json!({"sandboxed": true, "timeout": 30}),
                    mandatory: true,
                    strategy: VerificationStrategy::All,
                }
            ],
            resource_requirements: ResourceRequirements {
                min_memory_mb: Some(200),
                min_cpu_cores: Some(1.0),
                min_disk_space_mb: Some(500),
                network_bandwidth_kbps: None,
            },
        });

        // Code analysis tools
        tools.insert("ast_parser".to_string(), ToolSecurityProfile {
            tool_name: "ast_parser".to_string(),
            required_commands: vec!["python3".to_string(), "node".to_string()],
            required_paths: vec!["${workspace}/**".to_string()],
            required_endpoints: vec![],
            risk_level: RiskLevel::Low,
            verification_requirements: vec![
                VerificationRequirement {
                    verification_type: VerificationType::State,
                    expected: serde_json::json!({"read_only": true}),
                    mandatory: false,
                    strategy: VerificationStrategy::Single,
                }
            ],
            resource_requirements: ResourceRequirements {
                min_memory_mb: Some(100),
                min_cpu_cores: Some(0.5),
                min_disk_space_mb: None,
                network_bandwidth_kbps: None,
            },
        });

        // Testing tools
        tools.insert("test_runner".to_string(), ToolSecurityProfile {
            tool_name: "test_runner".to_string(),
            required_commands: vec!["pytest".to_string(), "cargo".to_string(), "npm".to_string()],
            required_paths: vec!["${workspace}/**".to_string(), "/tmp/**".to_string()],
            required_endpoints: vec!["http://localhost:*".to_string()],
            risk_level: RiskLevel::Medium,
            verification_requirements: vec![
                VerificationRequirement {
                    verification_type: VerificationType::Command,
                    expected: serde_json::json!({"capture_output": true}),
                    mandatory: true,
                    strategy: VerificationStrategy::Single,
                }
            ],
            resource_requirements: ResourceRequirements {
                min_memory_mb: Some(500),
                min_cpu_cores: Some(2.0),
                min_disk_space_mb: Some(1000),
                network_bandwidth_kbps: Some(100),
            },
        });

        // Git operations
        tools.insert("git_ops".to_string(), ToolSecurityProfile {
            tool_name: "git_ops".to_string(),
            required_commands: vec!["git".to_string()],
            required_paths: vec!["${workspace}/**".to_string()],
            required_endpoints: vec!["https://github.com".to_string(), "https://gitlab.com".to_string()],
            risk_level: RiskLevel::Medium,
            verification_requirements: vec![
                VerificationRequirement {
                    verification_type: VerificationType::Command,
                    expected: serde_json::json!({"verify_remote": true, "sign_commits": true}),
                    mandatory: false,
                    strategy: VerificationStrategy::Single,
                }
            ],
            resource_requirements: ResourceRequirements {
                min_memory_mb: Some(100),
                min_cpu_cores: Some(0.5),
                min_disk_space_mb: Some(500),
                network_bandwidth_kbps: Some(1000),
            },
        });

        tools
    }

    /// Create default tool categories
    fn create_default_categories() -> HashMap<String, ToolCategory> {
        let mut categories = HashMap::new();

        categories.insert("file_ops".to_string(), ToolCategory {
            name: "file_ops".to_string(),
            description: "File system operations".to_string(),
            tools: vec!["read_file".to_string(), "write_file".to_string(), "str_replace".to_string()],
            default_risk_level: RiskLevel::Medium,
            requires_approval: false,
        });

        categories.insert("code_analysis".to_string(), ToolCategory {
            name: "code_analysis".to_string(),
            description: "Code analysis and parsing".to_string(),
            tools: vec!["ast_parser".to_string()],
            default_risk_level: RiskLevel::Low,
            requires_approval: false,
        });

        categories.insert("execution".to_string(), ToolCategory {
            name: "execution".to_string(),
            description: "Code execution and testing".to_string(),
            tools: vec!["run_command".to_string(), "debug_shell".to_string(), "test_runner".to_string()],
            default_risk_level: RiskLevel::High,
            requires_approval: true,
        });

        categories.insert("network".to_string(), ToolCategory {
            name: "network".to_string(),
            description: "Network operations".to_string(),
            tools: vec!["web_search".to_string(), "git_ops".to_string()],
            default_risk_level: RiskLevel::Medium,
            requires_approval: false,
        });

        categories
    }

    /// Create permission matrix
    fn create_permission_matrix() -> PermissionMatrix {
        let mut tool_permissions = HashMap::new();
        let mut trust_permissions = HashMap::new();

        // Define permissions for each tool
        tool_permissions.insert("read_file".to_string(), RequiredPermissions {
            filesystem: FilesystemPermissions {
                read_paths: vec!["${workspace}/**".to_string()],
                write_paths: vec![],
                delete_paths: vec![],
                create_paths: vec![],
                max_file_size_mb: Some(10),
            },
            network: NetworkPermissions {
                allowed_hosts: vec![],
                allowed_ports: vec![],
                allowed_protocols: vec![],
                max_bandwidth_kbps: None,
                timeout_seconds: None,
            },
            process: ProcessPermissions {
                can_spawn: false,
                allowed_executables: vec![],
                max_processes: None,
                cpu_limit_percent: None,
                memory_limit_mb: Some(50),
            },
            system: SystemPermissions {
                can_read_env: false,
                can_modify_env: false,
                allowed_env_vars: vec![],
                can_access_hardware: false,
                can_modify_system_settings: false,
            },
        });

        // Define trust-based permissions
        trust_permissions.insert("untrusted".to_string(), AllowedPermissions {
            trust_level_min: 0.0,
            trust_level_max: 0.3,
            allowed_categories: vec!["file_ops".to_string()],
            forbidden_tools: vec!["debug_shell".to_string(), "run_command".to_string()],
            max_risk_level: RiskLevel::Low,
        });

        trust_permissions.insert("basic".to_string(), AllowedPermissions {
            trust_level_min: 0.3,
            trust_level_max: 0.6,
            allowed_categories: vec!["file_ops".to_string(), "code_analysis".to_string()],
            forbidden_tools: vec!["debug_shell".to_string()],
            max_risk_level: RiskLevel::Medium,
        });

        trust_permissions.insert("trusted".to_string(), AllowedPermissions {
            trust_level_min: 0.6,
            trust_level_max: 0.9,
            allowed_categories: vec!["file_ops".to_string(), "code_analysis".to_string(), "execution".to_string()],
            forbidden_tools: vec![],
            max_risk_level: RiskLevel::High,
        });

        trust_permissions.insert("admin".to_string(), AllowedPermissions {
            trust_level_min: 0.9,
            trust_level_max: 1.0,
            allowed_categories: vec!["file_ops".to_string(), "code_analysis".to_string(), "execution".to_string(), "network".to_string()],
            forbidden_tools: vec![],
            max_risk_level: RiskLevel::Critical,
        });

        PermissionMatrix {
            tool_permissions,
            trust_permissions,
        }
    }

    /// Validate tool implementation
    fn validate_tool_implementation(&self, implementation: &ToolImplementation) -> Result<()> {
        match implementation {
            ToolImplementation::Command { command, .. } => {
                // Check for dangerous commands
                let dangerous = vec!["rm -rf", "sudo", "eval", "exec"];
                for danger in dangerous {
                    if command.contains(danger) {
                        return Err(IntentError::ValidationFailed(
                            format!("Dangerous command pattern detected: {}", danger)
                        ));
                    }
                }
            },
            ToolImplementation::PythonScript { imports, .. } => {
                // Check for dangerous imports
                let dangerous = vec!["os.system", "subprocess", "eval", "exec"];
                for import in imports {
                    for danger in &dangerous {
                        if import.contains(danger) {
                            return Err(IntentError::ValidationFailed(
                                format!("Dangerous Python import: {}", danger)
                            ));
                        }
                    }
                }
            },
            ToolImplementation::JavaScript { npm_packages, .. } => {
                // Check for dangerous packages
                let dangerous = vec!["child_process", "fs-extra"];
                for package in npm_packages {
                    if dangerous.contains(&package.as_str()) {
                        return Err(IntentError::ValidationFailed(
                            format!("Dangerous npm package: {}", package)
                        ));
                    }
                }
            },
            ToolImplementation::ApiEndpoint { url, .. } => {
                // Ensure HTTPS
                if !url.starts_with("https://") && !url.starts_with("http://localhost") {
                    return Err(IntentError::ValidationFailed(
                        "API endpoints must use HTTPS".to_string()
                    ));
                }
            },
            ToolImplementation::Composite { steps, .. } => {
                // Validate all referenced tools exist
                // This would check against registered tools
                if steps.is_empty() {
                    return Err(IntentError::ValidationFailed(
                        "Composite tools must have at least one step".to_string()
                    ));
                }
            },
        }
        
        Ok(())
    }

    /// Check if a tool is allowed for a given trust level
    pub async fn is_tool_allowed(&self, tool_name: &str, trust_level: f64) -> bool {
        let matrix = self.permission_matrix.read().await;
        
        // Find the appropriate trust permission level
        for (_, allowed) in &matrix.trust_permissions {
            if trust_level >= allowed.trust_level_min && trust_level <= allowed.trust_level_max {
                // Check if tool is explicitly forbidden
                if allowed.forbidden_tools.contains(&tool_name.to_string()) {
                    return false;
                }
                
                // Check if tool's category is allowed
                let categories = self.tool_categories.read().await;
                for (_, category) in categories.iter() {
                    if category.tools.contains(&tool_name.to_string()) {
                        return allowed.allowed_categories.contains(&category.name);
                    }
                }
            }
        }
        
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_registry() {
        let registry = ToolRegistry::new();
        
        // Test getting a standard tool profile
        let profile = registry.get_tool_profile("read_file").await;
        assert!(profile.is_some());
        
        // Test custom tool registration
        let custom_tool = CustomTool {
            name: "my_tool".to_string(),
            description: "Custom tool".to_string(),
            implementation: ToolImplementation::Command {
                command: "echo".to_string(),
                args: vec!["hello".to_string()],
                env: HashMap::new(),
            },
            security_profile: ToolSecurityProfile {
                tool_name: "my_tool".to_string(),
                required_commands: vec!["echo".to_string()],
                required_paths: vec![],
                required_endpoints: vec![],
                risk_level: RiskLevel::Low,
                verification_requirements: vec![],
                resource_requirements: ResourceRequirements::default(),
            },
            created_by: "test_user".to_string(),
            created_at: Utc::now(),
            approved: true,
            approval_reason: None,
        };
        
        let result = registry.register_custom_tool(custom_tool, 0.7).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_permission_checking() {
        let registry = ToolRegistry::new();
        
        // Low trust user shouldn't access debug_shell
        assert!(!registry.is_tool_allowed("debug_shell", 0.2).await);
        
        // High trust user should access debug_shell
        assert!(registry.is_tool_allowed("debug_shell", 0.8).await);
    }
}