//! Storage traits for identity persistence

use crate::{Result, IdentityTrait};

// String, Vec, and Box are available in std prelude

/// User data structure for storage
#[derive(Debug, Clone)]
pub struct User {
    /// User ID
    pub id: String,
    /// Username
    pub username: String,
    /// Email address
    pub email: Option<String>,
    /// Phone number
    pub phone: Option<String>,
    /// Display name
    pub display_name: Option<String>,
    /// Account active
    pub active: bool,
    /// Account verified
    pub verified: bool,
    /// Password hash (stored securely)
    pub password_hash: Option<String>,
    /// Multi-factor enabled
    pub mfa_enabled: bool,
    /// MFA secret (encrypted)
    pub mfa_secret: Option<Vec<u8>>,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Stored credential data
#[derive(Debug, Clone)]
pub struct StoredCredential {
    /// Credential ID
    pub id: String,
    /// User ID (owner)
    pub user_id: String,
    /// Credential type
    pub credential_type: String,
    /// Credential data (encrypted)
    pub data: Vec<u8>,
    /// Issuer
    pub issuer: String,
    /// Subject
    pub subject: String,
    /// Issued at
    pub issued_at: chrono::DateTime<chrono::Utc>,
    /// Expires at
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Revoked
    pub revoked: bool,
    /// Revocation reason
    pub revocation_reason: Option<String>,
}

/// Stored session data
#[derive(Debug, Clone)]
pub struct StoredSession {
    /// Session ID
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Session token (hashed)
    pub token_hash: String,
    /// IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Created at
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last accessed
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    /// Expires at
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Active
    pub active: bool,
}

/// User storage operations
pub trait UserStore: Send + Sync {
    /// Create a new user
    fn create_user(&self, user: &User) -> Result<()>;
    
    /// Get user by ID
    fn get_user(&self, user_id: &str) -> Result<Option<User>>;
    
    /// Get user by username
    fn get_user_by_username(&self, username: &str) -> Result<Option<User>>;
    
    /// Get user by email
    fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;
    
    /// Update user
    fn update_user(&self, user: &User) -> Result<()>;
    
    /// Delete user
    fn delete_user(&self, user_id: &str) -> Result<()>;
    
    /// List users with pagination
    fn list_users(&self, offset: usize, limit: usize) -> Result<Vec<User>>;
    
    /// Search users
    fn search_users(&self, query: &str) -> Result<Vec<User>>;
}

/// Credential storage operations
pub trait CredentialStore: Send + Sync {
    /// Store a credential
    fn store_credential(&self, credential: &StoredCredential) -> Result<()>;
    
    /// Get credential by ID
    fn get_credential(&self, credential_id: &str) -> Result<Option<StoredCredential>>;
    
    /// Get credentials for user
    fn get_user_credentials(&self, user_id: &str) -> Result<Vec<StoredCredential>>;
    
    /// Update credential
    fn update_credential(&self, credential: &StoredCredential) -> Result<()>;
    
    /// Revoke credential
    fn revoke_credential(&self, credential_id: &str, reason: Option<&str>) -> Result<()>;
    
    /// Delete credential
    fn delete_credential(&self, credential_id: &str) -> Result<()>;
    
    /// Clean up expired credentials
    fn cleanup_expired(&self) -> Result<usize>;
}

/// Session storage operations
pub trait SessionStore: Send + Sync {
    /// Store session
    fn store_session(&self, session: &StoredSession) -> Result<()>;
    
    /// Get session by ID
    fn get_session(&self, session_id: &str) -> Result<Option<StoredSession>>;
    
    /// Get sessions for user
    fn get_user_sessions(&self, user_id: &str) -> Result<Vec<StoredSession>>;
    
    /// Update session
    fn update_session(&self, session: &StoredSession) -> Result<()>;
    
    /// Delete session
    fn delete_session(&self, session_id: &str) -> Result<()>;
    
    /// Delete all sessions for user
    fn delete_user_sessions(&self, user_id: &str) -> Result<()>;
    
    /// Clean up expired sessions
    fn cleanup_expired(&self) -> Result<usize>;
}

/// Identity storage operations
pub trait IdentityStore: Send + Sync {
    /// Store identity
    fn store_identity(&self, identity: &dyn IdentityTrait) -> Result<()>;
    
    /// Get identity by ID
    fn get_identity(&self, identity_id: &str) -> Result<Option<Box<dyn IdentityTrait>>>;
    
    /// Update identity
    fn update_identity(&self, identity: &dyn IdentityTrait) -> Result<()>;
    
    /// Delete identity
    fn delete_identity(&self, identity_id: &str) -> Result<()>;
    
    /// List identity IDs
    fn list_identity_ids(&self) -> Result<Vec<String>>;
}