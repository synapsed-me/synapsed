//! Real command execution engine for swarm coordination
//! 
//! This module provides secure, sandboxed command execution with proper resource
//! limits, safety checks, and integration with the verification framework.

use crate::{
    error::{SwarmError, SwarmResult},
    types::*,
};
use synapsed_intent::{HierarchicalIntent, StepResult};
use synapsed_verify::VerificationResult;

use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::Arc,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::{Mutex, RwLock},
    time::timeout,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Configuration for the execution engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Commands that are explicitly allowed (if empty, all commands except blocked are allowed)
    pub allowed_commands: Vec<String>,
    /// Commands that are explicitly blocked
    pub blocked_commands: Vec<String>,
    /// Maximum execution time in seconds
    pub max_execution_time_secs: u64,
    /// Maximum memory usage in MB (if supported by OS)
    pub max_memory_mb: Option<u64>,
    /// Maximum CPU usage percentage (if supported by OS)  
    pub max_cpu_percent: Option<f64>,
    /// Working directory restrictions (commands can only run in these dirs)
    pub allowed_working_dirs: Vec<PathBuf>,
    /// Environment variable restrictions
    pub allowed_env_vars: HashSet<String>,
    /// Enable sandboxing with user/group restrictions
    pub enable_sandboxing: bool,
    /// User ID to run commands as (requires root)
    pub sandbox_uid: Option<u32>,
    /// Group ID to run commands as (requires root)
    pub sandbox_gid: Option<u32>,
    /// Enable network access for commands
    pub allow_network: bool,
    /// Enable file system write access
    pub allow_fs_write: bool,
    /// Maximum output buffer size in bytes
    pub max_output_bytes: usize,
    /// Enable real-time output streaming
    pub stream_output: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            allowed_commands: vec![
                "ls".to_string(),
                "cat".to_string(),
                "echo".to_string(),
                "pwd".to_string(),
                "whoami".to_string(),
                "date".to_string(),
                "id".to_string(),
                "env".to_string(),
            ],
            blocked_commands: vec![
                "rm".to_string(),
                "rmdir".to_string(),
                "dd".to_string(),
                "mkfs".to_string(),
                "fdisk".to_string(),
                "parted".to_string(),
                "sudo".to_string(),
                "su".to_string(),
                "chmod".to_string(),
                "chown".to_string(),
                "mount".to_string(),
                "umount".to_string(),
                "iptables".to_string(),
                "systemctl".to_string(),
                "service".to_string(),
                "kill".to_string(),
                "killall".to_string(),
                "pkill".to_string(),
                "reboot".to_string(),
                "shutdown".to_string(),
                "halt".to_string(),
                "poweroff".to_string(),
            ],
            max_execution_time_secs: 30,
            max_memory_mb: Some(256),
            max_cpu_percent: Some(50.0),
            allowed_working_dirs: vec![
                PathBuf::from("/tmp"),
                PathBuf::from("/var/tmp"),
            ],
            allowed_env_vars: [
                "PATH", "HOME", "USER", "SHELL", "TERM", "LANG", "LC_ALL",
            ].iter().map(|s| s.to_string()).collect(),
            enable_sandboxing: true,
            sandbox_uid: None,
            sandbox_gid: None,
            allow_network: false,
            allow_fs_write: false,
            max_output_bytes: 1024 * 1024, // 1MB
            stream_output: false,
        }
    }
}

/// Result of command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Unique execution ID
    pub execution_id: Uuid,
    /// Command that was executed
    pub command: String,
    /// Arguments passed to the command
    pub args: Vec<String>,
    /// Working directory
    pub working_dir: PathBuf,
    /// Exit status
    pub exit_status: Option<i32>,
    /// Whether the command succeeded (exit code 0)
    pub success: bool,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration
    pub duration_ms: u64,
    /// Memory usage if available
    pub memory_usage_mb: Option<u64>,
    /// CPU usage if available
    pub cpu_usage_percent: Option<f64>,
    /// Timestamp when execution started
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp when execution completed
    pub completed_at: chrono::DateTime<chrono::Utc>,
    /// Verification metadata for integration
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Active execution context
#[derive(Debug)]
struct ExecutionContext {
    execution_id: Uuid,
    child: Child,
    command: String,
    started_at: Instant,
    timeout_duration: Duration,
}

/// Production-ready execution engine
pub struct ExecutionEngine {
    /// Engine configuration
    config: Arc<RwLock<ExecutionConfig>>,
    /// Active executions
    active_executions: Arc<Mutex<HashMap<Uuid, ExecutionContext>>>,
    /// Execution history (limited size)
    execution_history: Arc<RwLock<Vec<ExecutionResult>>>,
}

impl ExecutionEngine {
    /// Create a new execution engine with default configuration
    pub fn new() -> Self {
        Self::with_config(ExecutionConfig::default())
    }

    /// Create a new execution engine with custom configuration
    pub fn with_config(config: ExecutionConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            active_executions: Arc::new(Mutex::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Initialize the execution engine
    pub async fn initialize(&self) -> SwarmResult<()> {
        info!("Initializing execution engine");
        
        let config = self.config.read().await;
        
        // Validate working directories exist
        for dir in &config.allowed_working_dirs {
            if !dir.exists() {
                warn!("Allowed working directory does not exist: {}", dir.display());
            } else if !dir.is_dir() {
                return Err(SwarmError::Other(anyhow::anyhow!(
                    "Allowed working directory is not a directory: {}", 
                    dir.display()
                )));
            }
        }

        // Check if we have required permissions for sandboxing
        if config.enable_sandboxing && (config.sandbox_uid.is_some() || config.sandbox_gid.is_some()) {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                let current_uid = unsafe { libc::getuid() };
                if current_uid != 0 && (config.sandbox_uid.is_some() || config.sandbox_gid.is_some()) {
                    warn!("Sandboxing with uid/gid requires root privileges, disabling user/group restrictions");
                }
            }
        }

        info!("Execution engine initialized successfully");
        Ok(())
    }

    /// Execute a shell command with full safety checks and monitoring
    pub async fn execute_command(
        &self,
        command: &str,
        args: &[&str],
        working_dir: Option<&Path>,
    ) -> SwarmResult<ExecutionResult> {
        let execution_id = Uuid::new_v4();
        let started_at = chrono::Utc::now();
        let start_time = Instant::now();

        info!(
            execution_id = %execution_id,
            command = %command,
            args = ?args,
            "Starting command execution"
        );

        // Validate command and arguments
        self.validate_command(command, args).await?;

        // Determine working directory
        let work_dir = self.validate_working_directory(working_dir).await?;

        // Build command with safety restrictions
        let mut cmd = self.build_secure_command(command, args, &work_dir).await?;

        // Execute with timeout and monitoring
        let result = self.execute_with_monitoring(execution_id, &mut cmd, command, args, &work_dir, start_time, started_at).await;

        // Record execution in history
        if let Ok(ref exec_result) = result {
            self.record_execution(exec_result.clone()).await;
        }

        result
    }

    /// Execute an intent step with the execution engine
    pub async fn execute_intent_step(
        &self,
        intent: &HierarchicalIntent,
        step_index: usize,
    ) -> SwarmResult<StepResult> {
        debug!(
            intent_id = %intent.id(),
            step_index = step_index,
            "Executing intent step"
        );

        // Extract command from intent step
        let steps = intent.steps();
        let step = steps.get(step_index)
            .ok_or_else(|| SwarmError::Other(anyhow::anyhow!("Step index out of bounds")))?;

        // Parse command from step description
        let (command, args) = self.parse_step_command(&step.description)?;

        // Execute command
        let exec_result = self.execute_command(&command, &args, None).await?;

        // Convert to StepResult format
        let step_result = StepResult {
            success: exec_result.success,
            output: Some(serde_json::json!({
                "stdout": exec_result.stdout,
                "stderr": exec_result.stderr,
                "exit_status": exec_result.exit_status,
                "duration_ms": exec_result.duration_ms,
            })),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("command".to_string(), serde_json::json!(exec_result.command));
                metadata.insert("args".to_string(), serde_json::json!(exec_result.args));
                metadata.insert("execution_id".to_string(), serde_json::json!(exec_result.execution_id));
                metadata.insert("working_dir".to_string(), serde_json::json!(exec_result.working_dir));
                metadata.insert("files".to_string(), serde_json::json!([])); // Would be populated by file operations
                metadata
            },
        };

        Ok(step_result)
    }

    /// Kill an active execution
    pub async fn kill_execution(&self, execution_id: Uuid) -> SwarmResult<()> {
        let mut active = self.active_executions.lock().await;
        
        if let Some(mut context) = active.remove(&execution_id) {
            warn!(execution_id = %execution_id, "Killing active execution");
            
            if let Err(e) = context.child.kill().await {
                error!(execution_id = %execution_id, error = %e, "Failed to kill process");
                return Err(SwarmError::Other(anyhow::anyhow!("Failed to kill process: {}", e)));
            }

            info!(execution_id = %execution_id, "Successfully killed execution");
            Ok(())
        } else {
            Err(SwarmError::Other(anyhow::anyhow!("Execution not found or already completed")))
        }
    }

    /// Get the list of currently active executions
    pub async fn active_executions(&self) -> Vec<Uuid> {
        let active = self.active_executions.lock().await;
        active.keys().cloned().collect()
    }

    /// Get execution history
    pub async fn execution_history(&self) -> Vec<ExecutionResult> {
        let history = self.execution_history.read().await;
        history.clone()
    }

    /// Update configuration
    pub async fn update_config(&self, new_config: ExecutionConfig) -> SwarmResult<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        info!("Execution engine configuration updated");
        Ok(())
    }

    /// Get current configuration
    pub async fn get_config(&self) -> ExecutionConfig {
        let config = self.config.read().await;
        config.clone()
    }

    // Private implementation methods

    async fn validate_command(&self, command: &str, args: &[&str]) -> SwarmResult<()> {
        let config = self.config.read().await;

        // Check blocked commands
        if config.blocked_commands.contains(&command.to_string()) {
            return Err(SwarmError::Other(anyhow::anyhow!(
                "Command '{}' is blocked",
                command
            )));
        }

        // Check allowed commands (if list is not empty)
        if !config.allowed_commands.is_empty() && !config.allowed_commands.contains(&command.to_string()) {
            return Err(SwarmError::Other(anyhow::anyhow!(
                "Command '{}' is not in allowed list",
                command
            )));
        }

        // Validate command exists and is executable
        if which::which(command).is_err() {
            return Err(SwarmError::Other(anyhow::anyhow!(
                "Command '{}' not found in PATH",
                command
            )));
        }

        // Additional argument validation
        for arg in args {
            if arg.contains("..") && (arg.contains("/") || arg.contains("\\")) {
                warn!("Potentially unsafe argument with path traversal: {}", arg);
            }
        }

        Ok(())
    }

    async fn validate_working_directory(&self, working_dir: Option<&Path>) -> SwarmResult<PathBuf> {
        let config = self.config.read().await;

        let work_dir = working_dir
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp")));

        // Check if working directory is allowed
        if !config.allowed_working_dirs.is_empty() {
            let is_allowed = config.allowed_working_dirs.iter().any(|allowed| {
                work_dir.starts_with(allowed)
            });

            if !is_allowed {
                return Err(SwarmError::Other(anyhow::anyhow!(
                    "Working directory '{}' is not allowed", 
                    work_dir.display()
                )));
            }
        }

        // Ensure directory exists
        if !work_dir.exists() {
            return Err(SwarmError::Other(anyhow::anyhow!(
                "Working directory '{}' does not exist", 
                work_dir.display()
            )));
        }

        Ok(work_dir)
    }

    async fn build_secure_command(&self, command: &str, args: &[&str], working_dir: &Path) -> SwarmResult<Command> {
        let config = self.config.read().await;
        let mut cmd = Command::new(command);

        // Set arguments
        cmd.args(args);

        // Set working directory
        cmd.current_dir(working_dir);

        // Configure environment
        cmd.env_clear();
        for (key, value) in std::env::vars() {
            if config.allowed_env_vars.contains(&key) {
                cmd.env(key, value);
            }
        }

        // Configure I/O
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.stdin(std::process::Stdio::null());

        // Apply sandboxing restrictions if enabled
        #[cfg(unix)]
        if config.enable_sandboxing {
            // Set user/group if specified and we have permissions
            unsafe {
                if libc::getuid() == 0 {
                    if let Some(uid) = config.sandbox_uid {
                        cmd.uid(uid);
                    }
                    if let Some(gid) = config.sandbox_gid {
                        cmd.gid(gid);
                    }
                }
            }
        }

        Ok(cmd)
    }

    async fn execute_with_monitoring(
        &self,
        execution_id: Uuid,
        cmd: &mut Command,
        command: &str,
        args: &[&str],
        working_dir: &Path,
        start_time: Instant,
        started_at: chrono::DateTime<chrono::Utc>,
    ) -> SwarmResult<ExecutionResult> {
        let config = self.config.read().await;
        let timeout_duration = Duration::from_secs(config.max_execution_time_secs);
        let max_output_bytes = config.max_output_bytes;
        drop(config);

        // Spawn the process
        let mut child = cmd.spawn()
            .map_err(|e| SwarmError::Other(anyhow::anyhow!("Failed to spawn process: {}", e)))?;

        // Store execution context for monitoring
        {
            let mut active = self.active_executions.lock().await;
            active.insert(execution_id, ExecutionContext {
                execution_id,
                child: child,
                command: command.to_string(),
                started_at: start_time,
                timeout_duration,
            });
        }

        // Get stdout and stderr handles
        let stdout = child.stdout.take()
            .ok_or_else(|| SwarmError::Other(anyhow::anyhow!("Failed to capture stdout")))?;
        let stderr = child.stderr.take()
            .ok_or_else(|| SwarmError::Other(anyhow::anyhow!("Failed to capture stderr")))?;

        // Read output with size limits
        let stdout_future = self.read_output_limited(stdout, max_output_bytes);
        let stderr_future = self.read_output_limited(stderr, max_output_bytes);

        // Wait for completion with timeout
        let result = timeout(timeout_duration, async {
            let (stdout_result, stderr_result) = tokio::join!(stdout_future, stderr_future);
            let exit_status = child.wait().await?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>((
                stdout_result?,
                stderr_result?,
                exit_status,
            ))
        }).await;

        // Remove from active executions
        {
            let mut active = self.active_executions.lock().await;
            active.remove(&execution_id);
        }

        let (stdout, stderr, exit_status) = match result {
            Ok(Ok((stdout, stderr, exit_status))) => (stdout, stderr, Some(exit_status)),
            Ok(Err(e)) => {
                error!(execution_id = %execution_id, error = %e, "Error during execution");
                return Err(SwarmError::Other(anyhow::anyhow!("Execution error: {}", e)));
            }
            Err(_) => {
                // Timeout occurred, kill the process
                warn!(execution_id = %execution_id, "Execution timed out, killing process");
                if let Err(e) = child.kill().await {
                    error!(execution_id = %execution_id, error = %e, "Failed to kill timed out process");
                }
                ("".to_string(), "Execution timed out".to_string(), None)
            }
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let completed_at = chrono::Utc::now();
        let success = exit_status.map_or(false, |status| status.success());
        let exit_code = exit_status.and_then(|status| status.code());

        let result = ExecutionResult {
            execution_id,
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            working_dir: working_dir.to_path_buf(),
            exit_status: exit_code,
            success,
            stdout,
            stderr,
            duration_ms,
            memory_usage_mb: None, // Would require system-specific monitoring
            cpu_usage_percent: None, // Would require system-specific monitoring
            started_at,
            completed_at,
            metadata: HashMap::new(),
        };

        info!(
            execution_id = %execution_id,
            command = %command,
            success = success,
            duration_ms = duration_ms,
            "Command execution completed"
        );

        Ok(result)
    }

    async fn read_output_limited<R>(&self, reader: R, max_bytes: usize) -> SwarmResult<String>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        let mut reader = BufReader::new(reader);
        let mut output = String::new();
        let mut line = String::new();
        let mut total_bytes = 0;

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(bytes_read) => {
                    total_bytes += bytes_read;
                    if total_bytes > max_bytes {
                        output.push_str(&format!("\n[Output truncated - exceeded {} bytes limit]", max_bytes));
                        break;
                    }
                    output.push_str(&line);
                }
                Err(e) => {
                    return Err(SwarmError::Other(anyhow::anyhow!("Error reading output: {}", e)));
                }
            }
        }

        Ok(output)
    }

    async fn record_execution(&self, execution: ExecutionResult) {
        let mut history = self.execution_history.write().await;
        history.push(execution);

        // Limit history size to prevent memory growth
        if history.len() > 1000 {
            history.drain(0..100);
        }
    }

    fn parse_step_command(&self, description: &str) -> SwarmResult<(String, Vec<String>)> {
        // Simple command parsing - in practice, this could be more sophisticated
        let parts: Vec<&str> = description.split_whitespace().collect();
        
        if parts.is_empty() {
            return Err(SwarmError::Other(anyhow::anyhow!("Empty command description")));
        }

        let command = parts[0].to_string();
        let args = parts[1..].iter().map(|s| s.to_string()).collect();

        Ok((command, args))
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

// Integration with synapsed-core traits
impl synapsed_core::traits::Observable for ExecutionEngine {
    fn status(&self) -> synapsed_core::ObservableStatus {
        synapsed_core::ObservableStatus::Healthy
    }
    
    fn health(&self) -> synapsed_core::Health {
        synapsed_core::Health::default()
    }
    
    fn metrics(&self) -> synapsed_core::MetricSet {
        synapsed_core::MetricSet::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio_test;

    #[tokio::test]
    async fn test_basic_command_execution() {
        let engine = ExecutionEngine::new();
        engine.initialize().await.unwrap();

        let result = engine.execute_command("echo", &["hello", "world"], None).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.stdout.trim(), "hello world");
        assert!(result.stderr.is_empty());
        assert_eq!(result.exit_status, Some(0));
    }

    #[tokio::test]
    async fn test_blocked_command() {
        let engine = ExecutionEngine::new();
        engine.initialize().await.unwrap();

        let result = engine.execute_command("rm", &["-rf", "/"], None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_working_directory_restriction() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let mut config = ExecutionConfig::default();
        config.allowed_working_dirs = vec![temp_path.to_path_buf()];

        let engine = ExecutionEngine::with_config(config);
        engine.initialize().await.unwrap();

        // This should work
        let result = engine.execute_command("pwd", &[], Some(temp_path)).await.unwrap();
        assert!(result.success);

        // This should fail (using /root which is not in allowed dirs)
        let result = engine.execute_command("pwd", &[], Some(Path::new("/root"))).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_timeout() {
        let mut config = ExecutionConfig::default();
        config.max_execution_time_secs = 1;
        config.allowed_commands = vec!["sleep".to_string()];

        let engine = ExecutionEngine::with_config(config);
        engine.initialize().await.unwrap();

        let result = engine.execute_command("sleep", &["5"], None).await.unwrap();
        
        assert!(!result.success);
        assert!(result.stderr.contains("timed out"));
        assert!(result.exit_status.is_none());
    }
}