//! # Synapsed Verify
//! 
//! Multi-strategy verification for intent execution and agent claims.
//! Provides comprehensive verification mechanisms to ensure AI agents (especially Claude sub-agents)
//! actually do what they claim.
//!
//! ## Key Features
//! 
//! - Command execution verification with sandboxing
//! - File system state verification
//! - Network/API response verification
//! - State snapshot and comparison
//! - Cryptographic proof generation
//! - Multi-party consensus verification

pub mod command;
pub mod filesystem;
pub mod network;
pub mod state;
pub mod proof;
pub mod strategy;
pub mod types;

pub use command::{CommandVerifier, CommandVerification, ExecutionSandbox};
pub use filesystem::{FileSystemVerifier, FileVerification, FileSystemSnapshot};
pub use network::{NetworkVerifier, NetworkVerification, ApiVerification};
pub use state::{StateVerifier, StateSnapshot, StateDiff};
pub use proof::{ProofGenerator, VerificationProof, ProofChain};
pub use strategy::{VerificationStrategy, StrategyBuilder, ConsensusVerifier};
pub use types::*;

use synapsed_core::SynapsedError;

/// Result type for verification operations
pub type Result<T> = std::result::Result<T, VerifyError>;

/// Verification-specific errors
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    
    #[error("Command execution error: {0}")]
    CommandError(String),
    
    #[error("File system error: {0}")]
    FileSystemError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("State mismatch: expected {expected}, got {actual}")]
    StateMismatch {
        expected: String,
        actual: String,
    },
    
    #[error("Proof generation failed: {0}")]
    ProofError(String),
    
    #[error("Consensus not reached: {0}")]
    ConsensusError(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    #[error("Sandbox error: {0}")]
    SandboxError(String),
    
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

impl From<VerifyError> for SynapsedError {
    fn from(err: VerifyError) -> Self {
        match err {
            VerifyError::VerificationFailed(msg) => SynapsedError::Validation(msg),
            VerifyError::CommandError(msg) => SynapsedError::Execution(msg),
            VerifyError::NetworkError(msg) => SynapsedError::Network(msg),
            VerifyError::Timeout(msg) => SynapsedError::Timeout(msg),
            _ => SynapsedError::Internal(err.to_string()),
        }
    }
}

/// Main verification coordinator
pub struct Verifier {
    command: CommandVerifier,
    filesystem: FileSystemVerifier,
    network: NetworkVerifier,
    state: StateVerifier,
    proof_generator: ProofGenerator,
}

impl Verifier {
    /// Creates a new verifier with default settings
    pub fn new() -> Self {
        Self {
            command: CommandVerifier::new(),
            filesystem: FileSystemVerifier::new(),
            network: NetworkVerifier::new(),
            state: StateVerifier::new(),
            proof_generator: ProofGenerator::new(),
        }
    }
    
    /// Creates a verifier with sandboxing enabled
    pub fn with_sandbox() -> Self {
        Self {
            command: CommandVerifier::with_sandbox(),
            filesystem: FileSystemVerifier::new(),
            network: NetworkVerifier::new(),
            state: StateVerifier::new(),
            proof_generator: ProofGenerator::new(),
        }
    }
    
    /// Verifies a command execution claim
    pub async fn verify_command(
        &self,
        command: &str,
        expected_output: Option<&str>,
        expected_exit_code: Option<i32>,
    ) -> Result<CommandVerification> {
        self.command.verify(command, expected_output, expected_exit_code).await
    }
    
    /// Verifies file system state
    pub async fn verify_files(
        &self,
        paths: &[&str],
        expected_state: FileSystemSnapshot,
    ) -> Result<FileVerification> {
        self.filesystem.verify_snapshot(paths, expected_state).await
    }
    
    /// Verifies network/API response
    pub async fn verify_api(
        &self,
        url: &str,
        expected_status: u16,
        expected_body: Option<serde_json::Value>,
    ) -> Result<ApiVerification> {
        self.network.verify_api(url, expected_status, expected_body).await
    }
    
    /// Takes a state snapshot
    pub async fn snapshot_state(&mut self) -> Result<StateSnapshot> {
        self.state.take_snapshot().await
    }
    
    /// Verifies state against a snapshot
    pub async fn verify_state(&self, snapshot: &StateSnapshot) -> Result<StateDiff> {
        self.state.verify_against_snapshot(snapshot).await
    }
    
    /// Generates a cryptographic proof of verification
    pub async fn generate_proof(
        &mut self,
        verifications: Vec<VerificationResult>,
    ) -> Result<VerificationProof> {
        self.proof_generator.generate_proof(verifications).await
    }
}

impl Default for Verifier {
    fn default() -> Self {
        Self::new()
    }
}