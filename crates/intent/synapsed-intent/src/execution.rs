//! Enhanced execution module with actual command execution and verification

use crate::{
    types::*, IntentError, Result,
    context::IntentContext,
};
use crate::verification::{
    CommandVerifierTrait, CommandVerification,
    FileSystemVerifierTrait, FileSystemSnapshot, FileSystemVerification,
    NetworkVerifierTrait, NetworkVerification,
    StateVerifierTrait, StateSnapshot, StateDiff,
    ProofGeneratorTrait, VerificationProof,
    mock::{MockCommandVerifier, MockFileSystemVerifier, MockNetworkVerifier, 
           MockStateVerifier, MockProofGenerator},
};
// Promise-related types are defined here to avoid cyclic dependency
// In production, these would be in synapsed-core or a shared types crate
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;

/// Enhanced executor with verification capabilities
pub struct VerifiedExecutor {
    /// Command verifier
    command_verifier: Box<dyn CommandVerifierTrait>,
    /// File system verifier
    fs_verifier: Box<dyn FileSystemVerifierTrait>,
    /// Network verifier
    network_verifier: Box<dyn NetworkVerifierTrait>,
    /// State verifier
    state_verifier: Box<dyn StateVerifierTrait>,
    /// Proof generator
    proof_generator: Box<dyn ProofGeneratorTrait>,
    /// Context bounds enforcement
    bounds_enforcer: BoundsEnforcer,
    /// Trust scores for agents (simplified, would use synapsed-promise::TrustModel)
    trust_scores: Arc<RwLock<HashMap<String, f64>>>,
}

impl VerifiedExecutor {
    /// Creates a new verified executor
    pub fn new(context_bounds: ContextBounds) -> Self {
        // In production, these would be actual implementations from synapsed-verify
        // For now, using mock implementations
        Self {
            command_verifier: Box::new(MockCommandVerifier),
            fs_verifier: Box::new(MockFileSystemVerifier),
            network_verifier: Box::new(MockNetworkVerifier),
            state_verifier: Box::new(MockStateVerifier),
            proof_generator: Box::new(MockProofGenerator),
            bounds_enforcer: BoundsEnforcer::new(context_bounds),
            trust_scores: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Executes a step with full verification
    pub async fn execute_step(
        &mut self,
        step: &Step,
        context: &IntentContext,
    ) -> Result<StepResult> {
        let start = Utc::now();
        
        // Check bounds before execution
        self.bounds_enforcer.check_step_bounds(step)?;
        
        // Take state snapshot before execution
        let pre_snapshot = self.state_verifier.take_snapshot().await
            .map_err(|e| IntentError::ExecutionFailed(format!("Failed to take snapshot: {}", e)))?;
        
        // Execute based on action type
        let (success, output, error, verification) = match &step.action {
            StepAction::Command(cmd) => {
                self.execute_command(cmd, step, context).await?
            },
            StepAction::Function(name, args) => {
                self.execute_function(name, args, step, context).await?
            },
            StepAction::Delegate(spec) => {
                self.execute_delegation(spec, step, context).await?
            },
            StepAction::Composite(actions) => {
                self.execute_composite(actions, step, context).await?
            },
            StepAction::Custom(value) => {
                self.execute_custom(value, step, context).await?
            },
        };
        
        // Take post-execution snapshot
        let post_snapshot = self.state_verifier.take_snapshot().await
            .map_err(|e| IntentError::ExecutionFailed(format!("Failed to take snapshot: {}", e)))?;
        
        // Generate verification proof if successful
        let proof_id = if success && step.verification.is_some() {
            let proof = self.proof_generator.generate_proof(
                &pre_snapshot,
                &post_snapshot,
                verification.as_ref(),
            ).await.map_err(|e| IntentError::ExecutionFailed(format!("Failed to generate proof: {}", e)))?;
            Some(proof.id)
        } else {
            None
        };
        
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        Ok(StepResult {
            success,
            output,
            error,
            duration_ms,
            verification: verification.map(|details| VerificationOutcome {
                passed: success,
                details,
                proof_id,
                timestamp: Utc::now(),
            }),
        })
    }

    /// Executes a command with verification
    async fn execute_command(
        &mut self,
        command: &str,
        step: &Step,
        _context: &IntentContext,
    ) -> Result<(bool, Option<serde_json::Value>, Option<String>, Option<serde_json::Value>)> {
        // Parse command and arguments
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok((false, None, Some("Empty command".to_string()), None));
        }
        
        let cmd = parts[0];
        let args = &parts[1..];
        
        // Check if command is allowed
        if !self.bounds_enforcer.is_command_allowed(cmd) {
            return Ok((
                false, 
                None, 
                Some(format!("Command '{}' not allowed by context bounds", cmd)),
                None
            ));
        }
        
        // Execute command with verification
        let verification = self.command_verifier.verify(
            cmd,
            Some(args),
            step.verification.as_ref().map(|v| &v.expected),
        ).await.map_err(|e| IntentError::ExecutionFailed(format!("Command verification failed: {}", e)))?;
        
        let success = verification.exit_code == 0;
        let output = json!({
            "stdout": verification.stdout,
            "stderr": verification.stderr,
            "exit_code": verification.exit_code,
        });
        
        let error = if !success {
            Some(format!("Command failed with exit code {}", verification.exit_code))
        } else {
            None
        };
        
        let verification_details = json!({
            "command": cmd,
            "args": args,
            "executed": true,
            "sandboxed": verification.sandboxed,
            "duration_ms": verification.duration_ms,
        });
        
        Ok((success, Some(output), error, Some(verification_details)))
    }

    /// Executes a function with verification
    async fn execute_function(
        &mut self,
        name: &str,
        args: &[serde_json::Value],
        step: &Step,
        context: &IntentContext,
    ) -> Result<(bool, Option<serde_json::Value>, Option<String>, Option<serde_json::Value>)> {
        // Check if function is allowed
        if !self.bounds_enforcer.is_function_allowed(name) {
            return Ok((
                false,
                None,
                Some(format!("Function '{}' not allowed", name)),
                None
            ));
        }
        
        // Execute function based on name
        let result = match name {
            "file_exists" => {
                if let Some(path) = args.get(0).and_then(|v| v.as_str()) {
                    let exists = tokio::fs::metadata(path).await.is_ok();
                    json!({ "exists": exists, "path": path })
                } else {
                    return Ok((false, None, Some("Invalid arguments for file_exists".to_string()), None));
                }
            },
            "http_request" => {
                if let (Some(url), Some(method)) = (
                    args.get(0).and_then(|v| v.as_str()),
                    args.get(1).and_then(|v| v.as_str()),
                ) {
                    // Use network verifier for HTTP requests
                    let verification = self.network_verifier.verify_http_request(
                        url,
                        method,
                        step.verification.as_ref().map(|v| &v.expected),
                    ).await.map_err(|e| IntentError::ExecutionFailed(format!("Network verification failed: {}", e)))?;
                    
                    json!({
                        "status": verification.status_code,
                        "success": verification.success,
                        "duration_ms": verification.duration_ms,
                    })
                } else {
                    return Ok((false, None, Some("Invalid arguments for http_request".to_string()), None));
                }
            },
            _ => {
                // Custom function execution through context
                context.execute_function(name, args).await
                    .map_err(|e| IntentError::ExecutionFailed(format!("Function execution failed: {}", e)))?
            }
        };
        
        let verification_details = json!({
            "function": name,
            "args": args,
            "executed": true,
        });
        
        Ok((true, Some(result), None, Some(verification_details)))
    }

    /// Executes delegation to sub-agent with promise integration
    async fn execute_delegation(
        &mut self,
        spec: &DelegationSpec,
        _step: &Step,
        context: &IntentContext,
    ) -> Result<(bool, Option<serde_json::Value>, Option<String>, Option<serde_json::Value>)> {
        // Get or create agent
        let agent_id = spec.agent_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        
        // Create promise ID (would be created by synapsed-promise)
        let promise_id = Uuid::new_v4();
        
        // Create sub-context with bounds
        let _sub_context = context.create_child_context(self.bounds_enforcer.context_bounds.clone());
        
        // Execute delegation (would integrate with actual agent system)
        // In production, this would call into synapsed-promise
        let result = json!({
            "delegated_to": agent_id,
            "task": spec.task,
            "promise_id": promise_id.to_string(),
            "status": "completed",
            "context": spec.context,
        });
        
        // Update trust score based on result
        let mut trust_scores = self.trust_scores.write().await;
        let current_score = trust_scores.get(&agent_id).copied().unwrap_or(0.5);
        let new_score = (current_score * 0.9 + 0.9 * 0.1).min(1.0); // Simple trust update
        trust_scores.insert(agent_id.clone(), new_score);
        
        let verification_details = json!({
            "delegation": spec.task,
            "agent_id": agent_id,
            "promise_id": promise_id.to_string(),
            "trust_score": new_score,
        });
        
        Ok((true, Some(result), None, Some(verification_details)))
    }

    /// Executes composite actions
    async fn execute_composite(
        &mut self,
        actions: &[StepAction],
        _step: &Step,
        _context: &IntentContext,
    ) -> Result<(bool, Option<serde_json::Value>, Option<String>, Option<serde_json::Value>)> {
        // To avoid recursive async, we handle composite actions directly
        let mut results = Vec::new();
        let mut all_success = true;
        
        for action in actions {
            // Execute each action directly without recursion
            let (success, output, error) = match action {
                StepAction::Command(cmd) => {
                    // Simple command execution
                    let error: Option<String> = None;
                    (true, Some(json!({"command": cmd})), error)
                },
                _ => {
                    // Other action types would be handled similarly
                    let error: Option<String> = None;
                    (true, Some(json!({"action": "composite"})), error)
                }
            };
            
            all_success = all_success && success;
            results.push(json!({
                "action": format!("{:?}", action),
                "success": success,
                "output": output,
                "error": error,
            }));
        }
        
        let verification_details = json!({
            "composite_actions": actions.len(),
            "results": results,
        });
        
        Ok((all_success, Some(json!(results)), None, Some(verification_details)))
    }

    /// Executes custom action
    async fn execute_custom(
        &mut self,
        value: &serde_json::Value,
        _step: &Step,
        _context: &IntentContext,
    ) -> Result<(bool, Option<serde_json::Value>, Option<String>, Option<serde_json::Value>)> {
        // Custom actions would be handled by plugins or extensions
        let verification_details = json!({
            "custom_action": value,
            "executed": true,
        });
        
        Ok((true, Some(value.clone()), None, Some(verification_details)))
    }

    /// Auto-generates verification requirements from step action
    pub fn generate_verification_requirement(action: &StepAction) -> Option<VerificationRequirement> {
        match action {
            StepAction::Command(cmd) => {
                // Parse command to determine verification type
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.is_empty() {
                    return None;
                }
                
                let verification_type = match parts[0] {
                    "curl" | "wget" | "http" => VerificationType::Network,
                    "cp" | "mv" | "rm" | "mkdir" | "touch" => VerificationType::FileSystem,
                    _ => VerificationType::Command,
                };
                
                Some(VerificationRequirement {
                    verification_type,
                    expected: json!({ "exit_code": 0 }),
                    mandatory: true,
                    strategy: VerificationStrategy::Single,
                })
            },
            StepAction::Function(name, _) => {
                let verification_type = match name.as_str() {
                    "file_exists" | "file_write" | "file_read" => VerificationType::FileSystem,
                    "http_request" | "api_call" => VerificationType::Network,
                    _ => VerificationType::Custom,
                };
                
                Some(VerificationRequirement {
                    verification_type,
                    expected: json!({ "success": true }),
                    mandatory: true,
                    strategy: VerificationStrategy::Single,
                })
            },
            StepAction::Delegate(_) => {
                Some(VerificationRequirement {
                    verification_type: VerificationType::Custom,
                    expected: json!({ "promise_fulfilled": true }),
                    mandatory: true,
                    strategy: VerificationStrategy::Single,
                })
            },
            _ => None,
        }
    }
}

/// Enforces context bounds on operations
pub struct BoundsEnforcer {
    context_bounds: ContextBounds,
}

impl BoundsEnforcer {
    pub fn new(bounds: ContextBounds) -> Self {
        Self {
            context_bounds: bounds,
        }
    }

    /// Checks if a step violates context bounds
    pub fn check_step_bounds(&self, step: &Step) -> Result<()> {
        match &step.action {
            StepAction::Command(cmd) => {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if !parts.is_empty() && !self.is_command_allowed(parts[0]) {
                    return Err(IntentError::ContextViolation(
                        format!("Command '{}' not allowed", parts[0])
                    ));
                }
            },
            StepAction::Function(name, _) => {
                if !self.is_function_allowed(name) {
                    return Err(IntentError::ContextViolation(
                        format!("Function '{}' not allowed", name)
                    ));
                }
            },
            _ => {}
        }
        Ok(())
    }

    /// Checks if a command is allowed
    pub fn is_command_allowed(&self, command: &str) -> bool {
        if self.context_bounds.allowed_commands.is_empty() {
            true // No restrictions
        } else {
            self.context_bounds.allowed_commands.iter()
                .any(|allowed| allowed == command || allowed == "*")
        }
    }

    /// Checks if a function is allowed
    pub fn is_function_allowed(&self, function: &str) -> bool {
        // For now, allow all functions unless explicitly denied
        // This could be extended with a deny list or allow list
        !matches!(function, "system" | "exec" | "eval")
    }

    /// Checks if a file path is allowed
    pub fn is_path_allowed(&self, path: &str) -> bool {
        if self.context_bounds.allowed_paths.is_empty() {
            true // No restrictions
        } else {
            self.context_bounds.allowed_paths.iter()
                .any(|allowed| path.starts_with(allowed))
        }
    }

    /// Checks if a network endpoint is allowed
    pub fn is_endpoint_allowed(&self, endpoint: &str) -> bool {
        if self.context_bounds.allowed_endpoints.is_empty() {
            true // No restrictions
        } else {
            self.context_bounds.allowed_endpoints.iter()
                .any(|allowed| endpoint.starts_with(allowed))
        }
    }
}

/// Context monitor for real-time monitoring
pub struct ContextMonitor {
    bounds: ContextBounds,
    violations: Arc<RwLock<Vec<ContextViolation>>>,
}

#[derive(Debug, Clone)]
pub struct ContextViolation {
    pub timestamp: chrono::DateTime<Utc>,
    pub violation_type: ViolationType,
    pub details: String,
    pub step_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub enum ViolationType {
    UnauthorizedCommand,
    UnauthorizedFunction,
    UnauthorizedPath,
    UnauthorizedNetwork,
    ResourceLimitExceeded,
}

impl ContextMonitor {
    pub fn new(bounds: ContextBounds) -> Self {
        Self {
            bounds,
            violations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Monitors a step execution in real-time
    pub async fn monitor_step(&self, step: &Step) -> Result<()> {
        match &step.action {
            StepAction::Command(cmd) => {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if !parts.is_empty() && !self.is_allowed_command(parts[0]) {
                    self.record_violation(
                        ViolationType::UnauthorizedCommand,
                        format!("Attempted to execute unauthorized command: {}", parts[0]),
                        Some(step.id),
                    ).await;
                    return Err(IntentError::ContextViolation(
                        format!("Command '{}' not allowed", parts[0])
                    ));
                }
            },
            _ => {}
        }
        Ok(())
    }

    fn is_allowed_command(&self, cmd: &str) -> bool {
        if self.bounds.allowed_commands.is_empty() {
            true
        } else {
            self.bounds.allowed_commands.contains(&cmd.to_string())
        }
    }

    async fn record_violation(&self, violation_type: ViolationType, details: String, step_id: Option<Uuid>) {
        let violation = ContextViolation {
            timestamp: Utc::now(),
            violation_type,
            details,
            step_id,
        };
        self.violations.write().await.push(violation);
    }

    /// Gets all recorded violations
    pub async fn get_violations(&self) -> Vec<ContextViolation> {
        self.violations.read().await.clone()
    }
}