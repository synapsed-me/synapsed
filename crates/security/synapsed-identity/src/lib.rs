#![allow(missing_docs)]
//! # Synapsed Identity
//!
//! A comprehensive identity and access management library for the Synapsed framework.
//!
//! ## Features
//!
//! - **Authentication**: Password, token, and OAuth-based authentication
//! - **Authorization**: Role-based and policy-based access control
//! - **Session Management**: Secure session handling with multiple backends
//! - **Storage**: Flexible storage backends (SQLite, PostgreSQL, MySQL, Redis)
//! - **Security**: Industry-standard cryptography and security best practices
//!
//! ## Quick Start
//!
//! ```rust
//! use synapsed_identity::{
//!     auth::{PasswordAuthenticator, Credentials},
//!     storage::MemoryIdentityStore,
//!     IdentityManager,
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create identity manager with in-memory storage
//!     let manager = IdentityManager::builder()
//!         .with_storage(MemoryIdentityStore::new())
//!         .build()
//!         .await?;
//!
//!     // Create a user
//!     let user = manager.create_user("user@example.com", "password").await?;
//!
//!     // Authenticate
//!     let identity = manager.authenticate(Credentials {
//!         username: "user@example.com".to_string(),
//!         password: "password".to_string(),
//!     }).await?;
//!
//!     Ok(())
//! }
//! ```

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]
#![warn(rust_2018_idioms)]

/// Authentication module providing various authentication mechanisms
pub mod auth;

/// Authorization module for access control
pub mod authorization;

/// Cryptographic utilities
pub mod crypto;

/// Error types for the library
pub mod error;

/// Identity implementations
pub mod identity;

/// Session management
pub mod session;

/// Storage backends for identity data
pub mod storage;

/// Decentralized Identifier (DID) implementation
#[cfg(feature = "did-core")]
pub mod did;

/// Progressive Web App (PWA) support
#[cfg(feature = "pwa-support")]
pub mod pwa;

// Re-export commonly used types
pub use error::{Error, Result};
pub use identity::Identity as IdentityTrait;

// Re-export core types for convenience
pub use synapsed_core::{SynapsedError, SynapsedResult};
pub use synapsed_core::traits::{Observable, Configurable, Identifiable, Validatable};

// Map our Error to SynapsedError for better integration
impl From<Error> for SynapsedError {
    fn from(err: Error) -> Self {
        match err {
            Error::AuthenticationFailed(msg) | Error::AuthenticationError(msg) => SynapsedError::Authentication(msg),
            Error::AuthorizationDenied(msg) | Error::AuthorizationFailed(msg) => SynapsedError::PermissionDenied(msg),
            Error::CryptoError(msg) | Error::Crypto(msg) | Error::CryptographicError(msg) => SynapsedError::Cryptographic(msg),
            Error::Configuration(msg) | Error::ConfigurationError(msg) => SynapsedError::Configuration(msg),
            Error::Storage(msg) | Error::StorageError(msg) => SynapsedError::Storage(msg),
            Error::NotFound(msg) => SynapsedError::NotFound(msg),
            Error::InvalidParameter(msg) | Error::Validation(msg) => SynapsedError::InvalidInput(msg),
            Error::DidParsingError(msg) | Error::DidMethodError(msg) | Error::DidResolutionError(msg) | Error::DidDocumentError(msg) => SynapsedError::Did(msg),
            Error::Json(e) => SynapsedError::Serialization(e.to_string()),
            Error::InvalidCredentials => SynapsedError::Authentication("Invalid credentials".to_string()),
            Error::SessionExpired | Error::SessionNotFound => SynapsedError::Authentication("Session error".to_string()),
            Error::SessionError(msg) => SynapsedError::Authentication(msg),
            Error::Other(e) => SynapsedError::Internal(e.to_string()),
            #[cfg(feature = "oauth")]
            Error::Http(e) => SynapsedError::Network(e.to_string()),
            #[cfg(feature = "sqlx")]
            Error::Database(e) => SynapsedError::Storage(e.to_string()),
            #[cfg(feature = "redis")]
            Error::Redis(e) => SynapsedError::Storage(e.to_string()),
            _ => SynapsedError::Internal(err.to_string()),
        }
    }
}

// Re-export DID types when feature is enabled
#[cfg(feature = "did-core")]
pub use did::{
    Did, DidDocument, DidMethod, DidKey, DidWeb,
    DidResolver, ResolutionResult,
    KeyRotationManager, KeyHierarchy,
    ZkpVerifier, AnonymousCredential,
    LocalFirstStorage,
};

// Re-export PWA types when feature is enabled
#[cfg(feature = "pwa-support")]
pub use pwa::{PwaDidManager, PwaIdentity, BrowserCapabilities};

/// Credential storage structure
#[derive(Debug, Clone)]
pub struct Credential {
    /// ID of the credential
    pub id: String,
    /// Type of credential (e.g., "password", "token")
    pub credential_type: String,
    /// Credential data (e.g., hashed password)
    pub data: String,
    /// When the credential was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the credential expires (if applicable)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

// async_trait is used by other modules but not directly here
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

/// Core identity type representing an authenticated user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Unique identifier for the user
    pub id: Uuid,
    /// Username or email
    pub username: String,
    /// User's display name
    pub display_name: Option<String>,
    /// User's roles
    pub roles: Vec<String>,
    /// Additional attributes
    pub attributes: std::collections::HashMap<String, serde_json::Value>,
    /// When the identity was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the identity was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// Note: We don't implement identity::Identity for Identity struct
// as it's meant to be a high-level user identity, not a cryptographic identity.
// The identity::Identity trait is for low-level cryptographic identities.

/// Main entry point for identity operations
pub struct IdentityManager<S, A, Z, M> {
    storage: S,
    authenticator: A,
    authorizer: Z,
    session_manager: M,
}

// Implement core traits for Identity
impl Identifiable for Identity {
    fn id(&self) -> uuid::Uuid {
        self.id
    }

    fn name(&self) -> &str {
        &self.username
    }

    fn type_name(&self) -> &'static str {
        "Identity"
    }
}

impl Validatable for Identity {
    fn validate(&self) -> SynapsedResult<()> {
        if self.username.is_empty() {
            return Err(SynapsedError::InvalidInput("Username cannot be empty".to_string()));
        }
        
        if self.roles.is_empty() {
            return Err(SynapsedError::InvalidInput("Identity must have at least one role".to_string()));
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Observable for Identity {
    async fn status(&self) -> SynapsedResult<synapsed_core::traits::ObservableStatus> {
        use synapsed_core::traits::*;
        use std::collections::HashMap;
        
        Ok(ObservableStatus {
            state: ObservableState::Running,
            last_updated: self.updated_at,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("username".to_string(), self.username.clone());
                meta.insert("roles_count".to_string(), self.roles.len().to_string());
                meta.insert("attributes_count".to_string(), self.attributes.len().to_string());
                meta
            },
        })
    }

    async fn health(&self) -> SynapsedResult<synapsed_core::traits::HealthStatus> {
        use synapsed_core::traits::*;
        use std::collections::HashMap;
        
        let mut checks = HashMap::new();
        
        // Check if identity is valid
        let validation_check = if self.is_valid() {
            HealthCheck {
                level: HealthLevel::Healthy,
                message: "Identity validation passed".to_string(),
                timestamp: chrono::Utc::now(),
            }
        } else {
            HealthCheck {
                level: HealthLevel::Critical,
                message: "Identity validation failed".to_string(),
                timestamp: chrono::Utc::now(),
            }
        };
        checks.insert("validation".to_string(), validation_check);
        
        // Check age of identity
        let age = chrono::Utc::now().signed_duration_since(self.created_at);
        let age_check = if age > chrono::Duration::days(365) {
            HealthCheck {
                level: HealthLevel::Warning,
                message: "Identity is older than 1 year".to_string(),
                timestamp: chrono::Utc::now(),
            }
        } else {
            HealthCheck {
                level: HealthLevel::Healthy,
                message: "Identity age is acceptable".to_string(),
                timestamp: chrono::Utc::now(),
            }
        };
        checks.insert("age".to_string(), age_check);
        
        let overall = if checks.values().any(|c| c.level == HealthLevel::Critical) {
            HealthLevel::Critical
        } else if checks.values().any(|c| c.level == HealthLevel::Warning) {
            HealthLevel::Warning
        } else {
            HealthLevel::Healthy
        };
        
        Ok(HealthStatus {
            overall,
            checks,
            last_check: chrono::Utc::now(),
        })
    }

    async fn metrics(&self) -> SynapsedResult<HashMap<String, f64>> {
        let mut metrics = HashMap::new();
        
        metrics.insert("roles_count".to_string(), self.roles.len() as f64);
        metrics.insert("attributes_count".to_string(), self.attributes.len() as f64);
        
        let age_seconds = chrono::Utc::now()
            .signed_duration_since(self.created_at)
            .num_seconds() as f64;
        metrics.insert("age_seconds".to_string(), age_seconds);
        
        let days_since_update = chrono::Utc::now()
            .signed_duration_since(self.updated_at)
            .num_days() as f64;
        metrics.insert("days_since_update".to_string(), days_since_update);
        
        Ok(metrics)
    }

    fn describe(&self) -> String {
        format!(
            "Identity {} ({}): {} roles, {} attributes, created {}, updated {}",
            self.username,
            self.id,
            self.roles.len(),
            self.attributes.len(),
            self.created_at.format("%Y-%m-%d"),
            self.updated_at.format("%Y-%m-%d")
        )
    }
}

impl<S, A, Z, M> IdentityManager<S, A, Z, M>
where
    S: storage::IdentityStore + Clone,
    A: auth::Authenticator,
    Z: authorization::Authorizer,
    M: session::SessionManager,
{
    /// Create a new identity manager builder
    pub fn builder() -> IdentityManagerBuilder<S, A, Z, M> {
        IdentityManagerBuilder::new()
    }

    /// Create a DID-based identity manager (when DID feature is enabled)
    #[cfg(feature = "did-core")]
    pub fn with_did_support() -> DidIdentityManagerBuilder {
        DidIdentityManagerBuilder::new()
    }

    /// Authenticate a user with credentials
    pub async fn authenticate(&self, credentials: A::Credentials) -> Result<Identity> {
        self.authenticator.authenticate(credentials).await
    }

    /// Check if an identity is authorized for a resource/action
    pub async fn authorize(
        &self,
        identity: &Identity,
        resource: &str,
        action: &str,
    ) -> Result<bool> {
        self.authorizer.authorize(identity, resource, action).await
    }

    /// Create a new session for an identity
    pub async fn create_session(&self, identity: &Identity) -> Result<String> {
        use session::SessionMetadata;
        use crate::identity::BasicIdentity;
        
        // Create a basic identity wrapper for session management
        let basic_identity = BasicIdentity::new(
            identity.username.clone(),
            crate::crypto::KeyType::Ed25519 // Default key type
        )?;
        
        let session = self.session_manager.create_session(
            &basic_identity,
            SessionMetadata::default()
        )?;
        Ok(session.token)
    }

    /// Get identity from session token
    pub async fn get_identity_from_session(&self, token: &str) -> Result<Option<Identity>> {
        let session = self.session_manager.get_session(token)?;
        // This would need to be implemented based on how sessions store identity info
        Ok(None)
    }
}

/// Builder for IdentityManager
pub struct IdentityManagerBuilder<S, A, Z, M> {
    storage: Option<S>,
    authenticator: Option<A>,
    authorizer: Option<Z>,
    session_manager: Option<M>,
}

impl<S, A, Z, M> IdentityManagerBuilder<S, A, Z, M> {
    fn new() -> Self {
        Self {
            storage: None,
            authenticator: None,
            authorizer: None,
            session_manager: None,
        }
    }

    /// Set the storage backend
    pub fn with_storage(mut self, storage: S) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Set the authenticator
    pub fn with_authenticator(mut self, authenticator: A) -> Self {
        self.authenticator = Some(authenticator);
        self
    }

    /// Set the authorizer
    pub fn with_authorizer(mut self, authorizer: Z) -> Self {
        self.authorizer = Some(authorizer);
        self
    }

    /// Build the IdentityManager
    pub async fn build(self) -> Result<IdentityManager<S, A, Z, session::InMemorySessionManager>>
    where
        S: storage::IdentityStore + Clone,
    {
        let storage = self.storage.ok_or(Error::Configuration(
            "Storage backend is required".to_string(),
        ))?;
        let authenticator = self.authenticator.ok_or(Error::Configuration(
            "Authenticator is required".to_string(),
        ))?;
        let authorizer = self.authorizer.ok_or(Error::Configuration(
            "Authorizer is required".to_string(),
        ))?;

            // For now, use InMemorySessionManager as default
        let session_manager = session::InMemorySessionManager::new(
            session::SessionConfig::default()
        );

        Ok(IdentityManager {
            storage,
            authenticator,
            authorizer,
            session_manager,
        })
    }
}

/// DID-based identity manager builder (when DID feature is enabled)
#[cfg(feature = "did-core")]
pub struct DidIdentityManagerBuilder {
    resolver: Option<did::DidResolver>,
    key_manager: Option<did::KeyRotationManager>,
    zkp_verifier: Option<did::ZkpVerifier>,
    storage: Option<did::LocalFirstStorage>,
}

#[cfg(feature = "did-core")]
impl DidIdentityManagerBuilder {
    fn new() -> Self {
        Self {
            resolver: None,
            key_manager: None,
            zkp_verifier: None,
            storage: None,
        }
    }

    /// Set DID resolver
    pub fn with_resolver(mut self, resolver: did::DidResolver) -> Self {
        self.resolver = Some(resolver);
        self
    }

    /// Set key rotation manager
    pub fn with_key_manager(mut self, key_manager: did::KeyRotationManager) -> Self {
        self.key_manager = Some(key_manager);
        self
    }

    /// Set ZKP verifier
    pub fn with_zkp_verifier(mut self, zkp_verifier: did::ZkpVerifier) -> Self {
        self.zkp_verifier = Some(zkp_verifier);
        self
    }

    /// Set local-first storage
    pub fn with_storage(mut self, storage: did::LocalFirstStorage) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Build the DID identity manager
    pub async fn build(self) -> Result<DidIdentityManager> {
        let resolver = self.resolver.unwrap_or_default();
        let key_manager = self.key_manager.ok_or_else(|| {
            Error::Configuration("Key rotation manager is required".to_string())
        })?;
        let zkp_verifier = self.zkp_verifier.unwrap_or_default();
        let storage = self.storage.ok_or_else(|| {
            Error::Configuration("Storage is required".to_string())
        })?;

        Ok(DidIdentityManager {
            resolver,
            key_manager,
            zkp_verifier,
            storage,
        })
    }
}

/// DID-based identity manager
#[cfg(feature = "did-core")]
pub struct DidIdentityManager {
    resolver: did::DidResolver,
    key_manager: did::KeyRotationManager,
    zkp_verifier: did::ZkpVerifier,
    storage: did::LocalFirstStorage,
}

#[cfg(feature = "did-core")]
impl DidIdentityManager {
    /// Create a new DID
    pub async fn create_did(&mut self, method: &str) -> Result<did::Did> {
        match method {
            "key" => {
                let mut did_key = did::DidKey::new();
                did_key.generate()
            }
            "web" => {
                Err(Error::Configuration("did:web requires domain specification".to_string()))
            }
            _ => {
                Err(Error::Configuration(format!("Unsupported DID method: {}", method)))
            }
        }
    }

    /// Resolve a DID to its document
    pub async fn resolve_did(&mut self, did: &did::Did) -> Result<Option<did::DidDocument>> {
        let result = self.resolver.resolve(did, did::DidResolutionOptions::default()).await?;
        Ok(result.document)
    }

    /// Rotate keys for a DID
    pub fn rotate_keys(&mut self, did: &did::Did, reason: did::key_management::RotationReason) -> Result<did::key_management::RotationResult> {
        self.key_manager.rotate_keys(did, reason)
    }

    /// Verify anonymous credential
    pub fn verify_credential(&mut self, presentation: &did::zkp::CredentialPresentation, request: &did::zkp::ProofRequest) -> Result<bool> {
        self.zkp_verifier.verify_credential_presentation(presentation, request)
    }

    /// Store DID document
    pub async fn store_document(&mut self, document: &did::DidDocument) -> Result<()> {
        self.storage.store_did_document(document).await
    }

    /// Load DID document
    pub async fn load_document(&self, did: &did::Did) -> Result<Option<did::DidDocument>> {
        self.storage.load_did_document(did).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_creation() {
        let identity = Identity {
            id: Uuid::new_v4(),
            username: "test@example.com".to_string(),
            display_name: Some("Test User".to_string()),
            roles: vec!["user".to_string()],
            attributes: std::collections::HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert_eq!(identity.username, "test@example.com");
        assert_eq!(identity.roles, vec!["user"]);
    }
}