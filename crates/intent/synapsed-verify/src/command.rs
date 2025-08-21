//! Command execution verification for AI agent claims

use crate::{types::*, Result, VerifyError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use chrono::Utc;
use uuid::Uuid;
use tempfile::TempDir;
use which::which;

/// Configuration for command verification
#[derive(Debug, Clone)]
pub struct CommandVerifierConfig {
    /// Whether to use sandboxing
    pub use_sandbox: bool,
    /// Timeout for command execution
    pub timeout_ms: u64,
    /// Maximum output size in bytes
    pub max_output_size: usize,
    /// Working directory for commands
    pub working_dir: Option<PathBuf>,
    /// Environment variables to set
    pub env_vars: HashMap<String, String>,
    /// Allowed commands (if Some, only these can run)
    pub allowed_commands: Option<Vec<String>>,
    /// Capture screenshot on failure
    pub capture_on_failure: bool,
}

impl Default for CommandVerifierConfig {
    fn default() -> Self {
        Self {
            use_sandbox: false,
            timeout_ms: 30000,
            max_output_size: 10 * 1024 * 1024, // 10MB
            working_dir: None,
            env_vars: HashMap::new(),
            allowed_commands: None,
            capture_on_failure: false,
        }
    }
}

/// Execution sandbox for safe command execution
pub struct ExecutionSandbox {
    /// Temporary directory for sandbox
    temp_dir: TempDir,
    /// Sandbox configuration
    config: SandboxConfig,
}

/// Sandbox configuration
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Allow network access
    pub allow_network: bool,
    /// Allow file system access outside sandbox
    pub allow_fs_access: bool,
    /// Memory limit in bytes
    pub memory_limit: Option<usize>,
    /// CPU time limit in seconds
    pub cpu_limit: Option<u64>,
    /// Allowed paths outside sandbox
    pub allowed_paths: Vec<PathBuf>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            allow_network: false,
            allow_fs_access: false,
            memory_limit: Some(512 * 1024 * 1024), // 512MB
            cpu_limit: Some(10),
            allowed_paths: Vec::new(),
        }
    }
}

impl ExecutionSandbox {
    /// Creates a new execution sandbox
    pub fn new(config: SandboxConfig) -> Result<Self> {
        let temp_dir = TempDir::new()
            .map_err(|e| VerifyError::SandboxError(format!("Failed to create temp dir: {}", e)))?;
        
        Ok(Self {
            temp_dir,
            config,
        })
    }
    
    /// Gets the sandbox directory path
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }
    
    /// Executes a command in the sandbox
    pub async fn execute(&self, command: &str, args: &[&str]) -> Result<CommandOutput> {
        // In a real implementation, this would use containers or VMs
        // For now, we'll use a restricted process
        
        let mut cmd = Command::new(command);
        cmd.args(args)
            .current_dir(self.temp_dir.path())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        // Set resource limits if available (platform-specific)
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.uid(unsafe { libc::getuid() });
            cmd.gid(unsafe { libc::getgid() });
        }
        
        let output = cmd.output().await
            .map_err(|e| VerifyError::CommandError(format!("Failed to execute: {}", e)))?;
        
        Ok(CommandOutput {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

/// Output from command execution
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Exit code of the command
    pub exit_code: Option<i32>,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
}

/// Result of command verification
#[derive(Debug, Clone)]
pub struct CommandVerification {
    /// Verification result
    pub result: VerificationResult,
    /// Command output
    pub output: CommandOutput,
    /// Sandbox used (if any)
    pub sandbox_path: Option<PathBuf>,
}

/// Command verifier for Claude sub-agent claims
pub struct CommandVerifier {
    config: CommandVerifierConfig,
    sandbox: Option<ExecutionSandbox>,
}

impl CommandVerifier {
    /// Creates a new command verifier
    pub fn new() -> Self {
        Self {
            config: CommandVerifierConfig::default(),
            sandbox: None,
        }
    }
    
    /// Creates a verifier with sandboxing enabled
    pub fn with_sandbox() -> Self {
        let mut config = CommandVerifierConfig::default();
        config.use_sandbox = true;
        
        let sandbox = ExecutionSandbox::new(SandboxConfig::default()).ok();
        
        Self {
            config,
            sandbox,
        }
    }
    
    /// Creates a verifier with custom configuration
    pub fn with_config(config: CommandVerifierConfig) -> Self {
        let sandbox = if config.use_sandbox {
            ExecutionSandbox::new(SandboxConfig::default()).ok()
        } else {
            None
        };
        
        Self {
            config,
            sandbox,
        }
    }
    
    /// Verifies a command execution claim
    pub async fn verify(
        &self,
        command: &str,
        expected_output: Option<&str>,
        expected_exit_code: Option<i32>,
    ) -> Result<CommandVerification> {
        let start = Utc::now();
        
        // Parse command
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(VerifyError::CommandError("Empty command".to_string()));
        }
        
        let cmd = parts[0];
        let args = &parts[1..];
        
        // Check if command is allowed
        if let Some(ref allowed) = self.config.allowed_commands {
            if !allowed.iter().any(|a| a == cmd) {
                return Err(VerifyError::CommandError(
                    format!("Command '{}' not in allowed list", cmd)
                ));
            }
        }
        
        // Check if command exists
        if which(cmd).is_err() {
            return Err(VerifyError::CommandError(
                format!("Command '{}' not found", cmd)
            ));
        }
        
        // Execute command
        let output = if self.config.use_sandbox {
            if let Some(ref sandbox) = self.sandbox {
                sandbox.execute(cmd, args).await?
            } else {
                self.execute_direct(cmd, args).await?
            }
        } else {
            self.execute_direct(cmd, args).await?
        };
        
        // Verify output
        let mut success = true;
        let mut error = None;
        
        // Check exit code
        if let Some(expected) = expected_exit_code {
            if output.exit_code != Some(expected) {
                success = false;
                error = Some(format!(
                    "Exit code mismatch: expected {}, got {:?}",
                    expected, output.exit_code
                ));
            }
        }
        
        // Check output content
        if let Some(expected) = expected_output {
            if !output.stdout.contains(expected) && !output.stderr.contains(expected) {
                success = false;
                error = Some(format!(
                    "Output does not contain expected: '{}'",
                    expected
                ));
            }
        }
        
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        // Create verification result
        let result = if success {
            VerificationResult::success(
                VerificationType::Command,
                serde_json::json!({
                    "command": command,
                    "expected_output": expected_output,
                    "expected_exit_code": expected_exit_code,
                }),
                serde_json::json!({
                    "exit_code": output.exit_code,
                    "stdout": output.stdout.clone(),
                    "stderr": output.stderr.clone(),
                }),
            )
        } else {
            VerificationResult::failure(
                VerificationType::Command,
                serde_json::json!({
                    "command": command,
                    "expected_output": expected_output,
                    "expected_exit_code": expected_exit_code,
                }),
                serde_json::json!({
                    "exit_code": output.exit_code,
                    "stdout": output.stdout.clone(),
                    "stderr": output.stderr.clone(),
                }),
                error.unwrap_or_else(|| "Verification failed".to_string()),
            )
        };
        
        let mut final_result = result;
        final_result.duration_ms = duration_ms;
        
        // Add evidence
        final_result.evidence.push(Evidence {
            evidence_type: EvidenceType::CommandOutput,
            data: serde_json::json!({
                "command": command,
                "exit_code": output.exit_code,
                "stdout_length": output.stdout.len(),
                "stderr_length": output.stderr.len(),
            }),
            source: "CommandVerifier".to_string(),
            timestamp: Utc::now(),
        });
        
        Ok(CommandVerification {
            result: final_result,
            output: output.clone(),
            sandbox_path: self.sandbox.as_ref().map(|s| s.path().to_path_buf()),
        })
    }
    
    /// Executes a command directly (without sandbox)
    async fn execute_direct(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput> {
        let mut command = Command::new(cmd);
        command.args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        // Set working directory
        if let Some(ref dir) = self.config.working_dir {
            command.current_dir(dir);
        }
        
        // Set environment variables
        for (key, value) in &self.config.env_vars {
            command.env(key, value);
        }
        
        // Execute with timeout
        let future = command.output();
        let output = timeout(
            Duration::from_millis(self.config.timeout_ms),
            future
        ).await
        .map_err(|_| VerifyError::Timeout(format!("Command timed out after {}ms", self.config.timeout_ms)))?
        .map_err(|e| VerifyError::CommandError(format!("Failed to execute: {}", e)))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        // Check output size limits
        if stdout.len() > self.config.max_output_size {
            return Err(VerifyError::CommandError(
                format!("Stdout exceeds limit: {} > {}", stdout.len(), self.config.max_output_size)
            ));
        }
        if stderr.len() > self.config.max_output_size {
            return Err(VerifyError::CommandError(
                format!("Stderr exceeds limit: {} > {}", stderr.len(), self.config.max_output_size)
            ));
        }
        
        Ok(CommandOutput {
            exit_code: output.status.code(),
            stdout,
            stderr,
        })
    }
    
    /// Verifies multiple commands in sequence
    pub async fn verify_sequence(
        &self,
        commands: &[(&str, Option<&str>, Option<i32>)],
    ) -> Result<Vec<CommandVerification>> {
        let mut results = Vec::new();
        
        for (command, expected_output, expected_exit_code) in commands {
            let verification = self.verify(command, *expected_output, *expected_exit_code).await?;
            
            // Stop on first failure if in strict mode
            if !verification.result.success {
                results.push(verification);
                break;
            }
            
            results.push(verification);
        }
        
        Ok(results)
    }
    
    /// Verifies a command with pre and post conditions
    pub async fn verify_with_conditions(
        &self,
        command: &str,
        precondition: Option<&str>,
        postcondition: Option<&str>,
    ) -> Result<CommandVerification> {
        // Check precondition
        if let Some(pre) = precondition {
            let pre_result = self.verify(pre, None, Some(0)).await?;
            if !pre_result.result.success {
                return Err(VerifyError::VerificationFailed(
                    format!("Precondition failed: {}", pre)
                ));
            }
        }
        
        // Execute main command
        let result = self.verify(command, None, None).await?;
        
        // Check postcondition
        if let Some(post) = postcondition {
            let post_result = self.verify(post, None, Some(0)).await?;
            if !post_result.result.success {
                return Err(VerifyError::VerificationFailed(
                    format!("Postcondition failed: {}", post)
                ));
            }
        }
        
        Ok(result)
    }
}

impl Default for CommandVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_command_verification() {
        let verifier = CommandVerifier::new();
        
        // Test successful command
        let result = verifier.verify("echo test", Some("test"), Some(0)).await.unwrap();
        assert!(result.result.success);
        assert!(result.output.stdout.contains("test"));
        
        // Test failed exit code
        let result = verifier.verify("false", None, Some(0)).await.unwrap();
        assert!(!result.result.success);
    }
    
    #[tokio::test]
    async fn test_command_not_found() {
        let verifier = CommandVerifier::new();
        
        let result = verifier.verify("nonexistentcommand123", None, None).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_command_sequence() {
        let verifier = CommandVerifier::new();
        
        let commands = vec![
            ("echo first", Some("first"), Some(0)),
            ("echo second", Some("second"), Some(0)),
        ];
        
        let results = verifier.verify_sequence(&commands).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.result.success));
    }
}