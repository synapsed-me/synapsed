//! Verification interfaces and types for intent execution
//!
//! This module defines the verification interfaces that are implemented
//! by synapsed-verify. This avoids cyclic dependencies while maintaining
//! strong typing.

use crate::{types::*, Result, IntentError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Trait for command verification
#[async_trait]
pub trait CommandVerifierTrait: Send + Sync {
    /// Verifies command execution
    async fn verify(
        &self,
        command: &str,
        args: Option<&[&str]>,
        expected: Option<&serde_json::Value>,
    ) -> Result<CommandVerification>;
}

/// Result of command verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandVerification {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub sandboxed: bool,
    pub duration_ms: u64,
}

/// Trait for file system verification
#[async_trait]
pub trait FileSystemVerifierTrait: Send + Sync {
    /// Takes a snapshot of file system state
    async fn take_snapshot(&self) -> Result<FileSystemSnapshot>;
    
    /// Verifies file system changes
    async fn verify_changes(
        &self,
        before: &FileSystemSnapshot,
        after: &FileSystemSnapshot,
        expected: Option<&serde_json::Value>,
    ) -> Result<FileSystemVerification>;
}

/// File system snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemSnapshot {
    pub files: HashMap<String, FileInfo>,
    pub timestamp: DateTime<Utc>,
}

/// Information about a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub hash: String,
    pub modified: DateTime<Utc>,
}

/// Result of file system verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemVerification {
    pub changes_detected: bool,
    pub files_created: Vec<String>,
    pub files_modified: Vec<String>,
    pub files_deleted: Vec<String>,
    pub matches_expected: bool,
}

/// Trait for network verification
#[async_trait]
pub trait NetworkVerifierTrait: Send + Sync {
    /// Verifies HTTP request
    async fn verify_http_request(
        &self,
        url: &str,
        method: &str,
        expected: Option<&serde_json::Value>,
    ) -> Result<NetworkVerification>;
}

/// Result of network verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkVerification {
    pub success: bool,
    pub status_code: u16,
    pub duration_ms: u64,
    pub response_size: usize,
}

/// Trait for state verification
#[async_trait]
pub trait StateVerifierTrait: Send + Sync {
    /// Takes a state snapshot
    async fn take_snapshot(&self) -> Result<StateSnapshot>;
    
    /// Compares states
    async fn compare(
        &self,
        before: &StateSnapshot,
        after: &StateSnapshot,
    ) -> Result<StateDiff>;
}

/// State snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub variables: HashMap<String, serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

/// Difference between states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    pub added: HashMap<String, serde_json::Value>,
    pub modified: HashMap<String, (serde_json::Value, serde_json::Value)>,
    pub removed: HashMap<String, serde_json::Value>,
}

/// Trait for proof generation
#[async_trait]
pub trait ProofGeneratorTrait: Send + Sync {
    /// Generates a verification proof
    async fn generate_proof(
        &self,
        pre_state: &StateSnapshot,
        post_state: &StateSnapshot,
        verification: Option<&serde_json::Value>,
    ) -> Result<VerificationProof>;
}

/// Verification proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationProof {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub pre_state_hash: String,
    pub post_state_hash: String,
    pub verification_data: Option<serde_json::Value>,
    pub signature: Option<String>,
}

/// Mock implementations for testing
pub mod mock {
    use super::*;
    
    /// Mock command verifier
    pub struct MockCommandVerifier;
    
    #[async_trait]
    impl CommandVerifierTrait for MockCommandVerifier {
        async fn verify(
            &self,
            _command: &str,
            _args: Option<&[&str]>,
            _expected: Option<&serde_json::Value>,
        ) -> Result<CommandVerification> {
            Ok(CommandVerification {
                exit_code: 0,
                stdout: "Mock output".to_string(),
                stderr: String::new(),
                sandboxed: true,
                duration_ms: 100,
            })
        }
    }
    
    /// Mock file system verifier
    pub struct MockFileSystemVerifier;
    
    #[async_trait]
    impl FileSystemVerifierTrait for MockFileSystemVerifier {
        async fn take_snapshot(&self) -> Result<FileSystemSnapshot> {
            Ok(FileSystemSnapshot {
                files: HashMap::new(),
                timestamp: Utc::now(),
            })
        }
        
        async fn verify_changes(
            &self,
            _before: &FileSystemSnapshot,
            _after: &FileSystemSnapshot,
            _expected: Option<&serde_json::Value>,
        ) -> Result<FileSystemVerification> {
            Ok(FileSystemVerification {
                changes_detected: false,
                files_created: Vec::new(),
                files_modified: Vec::new(),
                files_deleted: Vec::new(),
                matches_expected: true,
            })
        }
    }
    
    /// Mock network verifier
    pub struct MockNetworkVerifier;
    
    #[async_trait]
    impl NetworkVerifierTrait for MockNetworkVerifier {
        async fn verify_http_request(
            &self,
            _url: &str,
            _method: &str,
            _expected: Option<&serde_json::Value>,
        ) -> Result<NetworkVerification> {
            Ok(NetworkVerification {
                success: true,
                status_code: 200,
                duration_ms: 50,
                response_size: 1024,
            })
        }
    }
    
    /// Mock state verifier
    pub struct MockStateVerifier;
    
    #[async_trait]
    impl StateVerifierTrait for MockStateVerifier {
        async fn take_snapshot(&self) -> Result<StateSnapshot> {
            Ok(StateSnapshot {
                variables: HashMap::new(),
                timestamp: Utc::now(),
            })
        }
        
        async fn compare(
            &self,
            _before: &StateSnapshot,
            _after: &StateSnapshot,
        ) -> Result<StateDiff> {
            Ok(StateDiff {
                added: HashMap::new(),
                modified: HashMap::new(),
                removed: HashMap::new(),
            })
        }
    }
    
    /// Mock proof generator
    pub struct MockProofGenerator;
    
    #[async_trait]
    impl ProofGeneratorTrait for MockProofGenerator {
        async fn generate_proof(
            &self,
            _pre_state: &StateSnapshot,
            _post_state: &StateSnapshot,
            _verification: Option<&serde_json::Value>,
        ) -> Result<VerificationProof> {
            Ok(VerificationProof {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                pre_state_hash: "mock_pre_hash".to_string(),
                post_state_hash: "mock_post_hash".to_string(),
                verification_data: None,
                signature: Some("mock_signature".to_string()),
            })
        }
    }
}