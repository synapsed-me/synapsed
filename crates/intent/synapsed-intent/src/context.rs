//! Context management for intent execution

use crate::{
    types::*, IntentError, Result
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Context for intent execution
#[derive(Clone)]
pub struct IntentContext {
    /// Context ID
    id: Uuid,
    /// Parent context (if this is a child context)
    parent: Option<Arc<IntentContext>>,
    /// Context variables
    variables: Arc<RwLock<HashMap<String, Value>>>,
    /// Context bounds (restrictions)
    bounds: ContextBounds,
    /// Metadata
    metadata: ContextMetadata,
    /// Injected services/capabilities
    services: Arc<RwLock<HashMap<String, Arc<dyn ContextService>>>>,
    /// Verification requirements
    verification_requirements: Vec<VerificationRequirement>,
    /// Audit log
    audit_log: Arc<RwLock<Vec<AuditEntry>>>,
}

/// Metadata for a context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    /// Creator of the context
    pub creator: String,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Context purpose
    pub purpose: String,
    /// Tags
    pub tags: Vec<String>,
    /// Agent ID if from an agent
    pub agent_id: Option<String>,
    /// Session ID
    pub session_id: Option<Uuid>,
}

/// Entry in the audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Entry ID
    pub id: Uuid,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Action performed
    pub action: String,
    /// Actor (who performed it)
    pub actor: String,
    /// Result of the action
    pub result: AuditResult,
    /// Additional data
    pub data: Option<Value>,
}

/// Result of an audited action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditResult {
    Success,
    Failure,
    Blocked,
    Warning,
}

/// Service that can be injected into context
pub trait ContextService: Send + Sync {
    /// Service name
    fn name(&self) -> &str;
    
    /// Execute service function
    fn execute(&self, function: &str, args: Vec<Value>) -> Result<Value>;
    
    /// Check if function is available
    fn has_function(&self, function: &str) -> bool;
}

impl IntentContext {
    /// Creates a new root context
    pub fn new(bounds: ContextBounds) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent: None,
            variables: Arc::new(RwLock::new(HashMap::new())),
            bounds,
            metadata: ContextMetadata {
                creator: "system".to_string(),
                created_at: Utc::now(),
                purpose: "root".to_string(),
                tags: Vec::new(),
                agent_id: None,
                session_id: None,
            },
            services: Arc::new(RwLock::new(HashMap::new())),
            verification_requirements: Vec::new(),
            audit_log: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Creates a child context with additional restrictions
    pub fn create_child_context(&self, additional_bounds: ContextBounds) -> Self {
        // Merge bounds (more restrictive)
        let merged_bounds = self.merge_bounds(&additional_bounds);
        
        Self {
            id: Uuid::new_v4(),
            parent: Some(Arc::new(self.clone())),
            variables: Arc::new(RwLock::new(HashMap::new())),
            bounds: merged_bounds,
            metadata: ContextMetadata {
                creator: self.metadata.creator.clone(),
                created_at: Utc::now(),
                purpose: "child".to_string(),
                tags: self.metadata.tags.clone(),
                agent_id: self.metadata.agent_id.clone(),
                session_id: self.metadata.session_id,
            },
            services: Arc::clone(&self.services),
            verification_requirements: self.verification_requirements.clone(),
            audit_log: Arc::clone(&self.audit_log),
        }
    }
    
    /// Gets a variable from the context (checks parent if not found)
    pub fn get_variable(&self, key: &str) -> Option<Value> {
        // Use blocking read to avoid async recursion
        let vars = self.variables.blocking_read();
        if let Some(value) = vars.get(key) {
            Some(value.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_variable(key)
        } else {
            None
        }
    }
    
    /// Sets a variable in the context
    pub async fn set_variable(&self, key: String, value: Value) -> Result<()> {
        // Check if variable setting is allowed
        if !self.is_variable_allowed(&key) {
            self.audit_action(
                "set_variable",
                "system",
                AuditResult::Blocked,
                Some(serde_json::json!({
                    "key": key,
                    "reason": "Variable not allowed by context bounds"
                }))
            ).await;
            
            return Err(IntentError::ContextViolation(
                format!("Variable '{}' not allowed in context", key)
            ));
        }
        
        let mut vars = self.variables.write().await;
        vars.insert(key.clone(), value.clone());
        
        self.audit_action(
            "set_variable",
            "system",
            AuditResult::Success,
            Some(serde_json::json!({
                "key": key,
                "value": value
            }))
        ).await;
        
        Ok(())
    }
    
    /// Checks if a command is allowed in this context
    pub fn is_command_allowed(&self, command: &str) -> bool {
        if self.bounds.allowed_commands.is_empty() {
            // No restrictions
            true
        } else {
            // Check if command is in allowed list
            self.bounds.allowed_commands.iter().any(|allowed| {
                command.starts_with(allowed) || 
                allowed == "*" ||
                glob_match(allowed, command)
            })
        }
    }
    
    /// Checks if a file path is allowed in this context
    pub fn is_path_allowed(&self, path: &str) -> bool {
        if self.bounds.allowed_paths.is_empty() {
            // No restrictions
            true
        } else {
            // Check if path is in allowed list
            self.bounds.allowed_paths.iter().any(|allowed| {
                path.starts_with(allowed) ||
                allowed == "*" ||
                glob_match(allowed, path)
            })
        }
    }
    
    /// Checks if a network endpoint is allowed
    pub fn is_endpoint_allowed(&self, endpoint: &str) -> bool {
        if self.bounds.allowed_endpoints.is_empty() {
            // No restrictions
            true
        } else {
            // Check if endpoint is in allowed list
            self.bounds.allowed_endpoints.iter().any(|allowed| {
                endpoint.starts_with(allowed) ||
                allowed == "*" ||
                glob_match(allowed, endpoint)
            })
        }
    }
    
    /// Registers a service in the context
    pub async fn register_service(&self, name: String, service: Arc<dyn ContextService>) {
        let mut services = self.services.write().await;
        services.insert(name.clone(), service);
        
        self.audit_action(
            "register_service",
            "system",
            AuditResult::Success,
            Some(serde_json::json!({ "service": name }))
        ).await;
    }
    
    /// Calls a service function
    pub async fn call_service(
        &self,
        service_name: &str,
        function: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        let services = self.services.read().await;
        
        if let Some(service) = services.get(service_name) {
            let result = service.execute(function, args.clone());
            
            self.audit_action(
                "call_service",
                "system",
                if result.is_ok() { AuditResult::Success } else { AuditResult::Failure },
                Some(serde_json::json!({
                    "service": service_name,
                    "function": function,
                    "args": args
                }))
            ).await;
            
            result
        } else {
            self.audit_action(
                "call_service",
                "system",
                AuditResult::Failure,
                Some(serde_json::json!({
                    "service": service_name,
                    "error": "Service not found"
                }))
            ).await;
            
            Err(IntentError::ContextViolation(
                format!("Service '{}' not found", service_name)
            ))
        }
    }
    
    /// Adds a verification requirement to the context
    pub fn add_verification_requirement(&mut self, requirement: VerificationRequirement) {
        self.verification_requirements.push(requirement);
    }
    
    /// Gets all verification requirements
    pub fn verification_requirements(&self) -> &[VerificationRequirement] {
        &self.verification_requirements
    }
    
    /// Validates the context
    pub async fn validate(&self) -> Result<()> {
        // Check bounds are reasonable
        if let Some(max_mem) = self.bounds.max_memory_bytes {
            if max_mem == 0 {
                return Err(IntentError::ValidationFailed(
                    "Max memory cannot be zero".to_string()
                ));
            }
        }
        
        if let Some(max_cpu) = self.bounds.max_cpu_seconds {
            if max_cpu == 0 {
                return Err(IntentError::ValidationFailed(
                    "Max CPU time cannot be zero".to_string()
                ));
            }
        }
        
        Ok(())
    }
    
    /// Gets the audit log
    pub async fn get_audit_log(&self) -> Vec<AuditEntry> {
        self.audit_log.read().await.clone()
    }
    
    /// Gets context metadata
    pub fn metadata(&self) -> &ContextMetadata {
        &self.metadata
    }
    
    /// Gets context bounds
    pub fn bounds(&self) -> &ContextBounds {
        &self.bounds
    }
    
    // Helper methods
    
    fn merge_bounds(&self, additional: &ContextBounds) -> ContextBounds {
        ContextBounds {
            // Intersection of allowed paths
            allowed_paths: if self.bounds.allowed_paths.is_empty() {
                additional.allowed_paths.clone()
            } else if additional.allowed_paths.is_empty() {
                self.bounds.allowed_paths.clone()
            } else {
                self.bounds.allowed_paths.iter()
                    .filter(|p| additional.allowed_paths.contains(p))
                    .cloned()
                    .collect()
            },
            
            // Intersection of allowed commands
            allowed_commands: if self.bounds.allowed_commands.is_empty() {
                additional.allowed_commands.clone()
            } else if additional.allowed_commands.is_empty() {
                self.bounds.allowed_commands.clone()
            } else {
                self.bounds.allowed_commands.iter()
                    .filter(|c| additional.allowed_commands.contains(c))
                    .cloned()
                    .collect()
            },
            
            // Intersection of allowed endpoints
            allowed_endpoints: if self.bounds.allowed_endpoints.is_empty() {
                additional.allowed_endpoints.clone()
            } else if additional.allowed_endpoints.is_empty() {
                self.bounds.allowed_endpoints.clone()
            } else {
                self.bounds.allowed_endpoints.iter()
                    .filter(|e| additional.allowed_endpoints.contains(e))
                    .cloned()
                    .collect()
            },
            
            // Take the minimum of resource limits
            max_memory_bytes: match (self.bounds.max_memory_bytes, additional.max_memory_bytes) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            },
            
            max_cpu_seconds: match (self.bounds.max_cpu_seconds, additional.max_cpu_seconds) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            },
            
            // Merge environment variables
            env_vars: {
                let mut merged = self.bounds.env_vars.clone();
                merged.extend(additional.env_vars.clone());
                merged
            },
        }
    }
    
    fn is_variable_allowed(&self, key: &str) -> bool {
        // Could implement variable name restrictions
        // For now, allow all except system variables
        !key.starts_with("__system_")
    }
    
    async fn audit_action(
        &self,
        action: &str,
        actor: &str,
        result: AuditResult,
        data: Option<Value>,
    ) {
        let entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            action: action.to_string(),
            actor: actor.to_string(),
            result,
            data,
        };
        
        let mut log = self.audit_log.write().await;
        log.push(entry);
        
        // Keep only last 1000 entries
        let len = log.len();
        if len > 1000 {
            log.drain(0..len - 1000);
        }
    }
}

/// Builder for creating contexts
pub struct ContextBuilder {
    bounds: ContextBounds,
    metadata: ContextMetadata,
    variables: HashMap<String, Value>,
    verification_requirements: Vec<VerificationRequirement>,
}

impl ContextBuilder {
    /// Creates a new context builder
    pub fn new() -> Self {
        Self {
            bounds: ContextBounds::default(),
            metadata: ContextMetadata {
                creator: "builder".to_string(),
                created_at: Utc::now(),
                purpose: "custom".to_string(),
                tags: Vec::new(),
                agent_id: None,
                session_id: None,
            },
            variables: HashMap::new(),
            verification_requirements: Vec::new(),
        }
    }
    
    /// Sets the creator
    pub fn creator(mut self, creator: impl Into<String>) -> Self {
        self.metadata.creator = creator.into();
        self
    }
    
    /// Sets the purpose
    pub fn purpose(mut self, purpose: impl Into<String>) -> Self {
        self.metadata.purpose = purpose.into();
        self
    }
    
    /// Adds allowed paths
    pub fn allow_paths(mut self, paths: Vec<String>) -> Self {
        self.bounds.allowed_paths.extend(paths);
        self
    }
    
    /// Adds allowed commands
    pub fn allow_commands(mut self, commands: Vec<String>) -> Self {
        self.bounds.allowed_commands.extend(commands);
        self
    }
    
    /// Adds allowed endpoints
    pub fn allow_endpoints(mut self, endpoints: Vec<String>) -> Self {
        self.bounds.allowed_endpoints.extend(endpoints);
        self
    }
    
    /// Sets memory limit
    pub fn max_memory(mut self, bytes: usize) -> Self {
        self.bounds.max_memory_bytes = Some(bytes);
        self
    }
    
    /// Sets CPU time limit
    pub fn max_cpu_time(mut self, seconds: u64) -> Self {
        self.bounds.max_cpu_seconds = Some(seconds);
        self
    }
    
    /// Adds a variable
    pub fn variable(mut self, key: impl Into<String>, value: Value) -> Self {
        self.variables.insert(key.into(), value);
        self
    }
    
    /// Adds a verification requirement
    pub fn verification(mut self, requirement: VerificationRequirement) -> Self {
        self.verification_requirements.push(requirement);
        self
    }
    
    /// Builds the context
    pub async fn build(self) -> IntentContext {
        let mut context = IntentContext::new(self.bounds);
        context.metadata = self.metadata;
        context.verification_requirements = self.verification_requirements;
        
        // Set initial variables
        for (key, value) in self.variables {
            let _ = context.set_variable(key, value).await;
        }
        
        context
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Helper function for glob matching
fn glob_match(pattern: &str, text: &str) -> bool {
    // Simple glob matching (could be enhanced)
    if pattern == "*" {
        return true;
    }
    
    if pattern.ends_with("*") {
        let prefix = &pattern[..pattern.len() - 1];
        text.starts_with(prefix)
    } else if pattern.starts_with("*") {
        let suffix = &pattern[1..];
        text.ends_with(suffix)
    } else {
        pattern == text
    }
}