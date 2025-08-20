//! Security abstractions and utilities for the Synapsed ecosystem.
//!
//! This module provides common security traits and utilities that can be
//! used across all security-related Synapsed components.

use crate::SynapsedResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Security level enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    /// No security
    None = 0,
    /// Basic security
    Basic = 1,
    /// Standard security
    Standard = 2,
    /// High security
    High = 3,
    /// Maximum security
    Maximum = 4,
}

impl SecurityLevel {
    /// Get the numeric value of the security level
    #[must_use] pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Check if this level meets the minimum requirement
    #[must_use] pub fn meets_requirement(&self, required: SecurityLevel) -> bool {
        *self >= required
    }
}

impl std::fmt::Display for SecurityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityLevel::None => write!(f, "None"),
            SecurityLevel::Basic => write!(f, "Basic"),
            SecurityLevel::Standard => write!(f, "Standard"),
            SecurityLevel::High => write!(f, "High"),
            SecurityLevel::Maximum => write!(f, "Maximum"),
        }
    }
}

/// Security context for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// Context identifier
    pub id: Uuid,
    /// Principal (user/service) identity
    pub principal: Principal,
    /// Security level
    pub security_level: SecurityLevel,
    /// Permissions granted
    pub permissions: Vec<Permission>,
    /// Authentication method used
    pub auth_method: AuthenticationMethod,
    /// Session information
    pub session: Option<SessionInfo>,
    /// Additional security attributes
    pub attributes: HashMap<String, String>,
    /// Context creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Context expiration time
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl SecurityContext {
    /// Create a new security context
    #[must_use] pub fn new(principal: Principal, security_level: SecurityLevel) -> Self {
        Self {
            id: Uuid::new_v4(),
            principal,
            security_level,
            permissions: Vec::new(),
            auth_method: AuthenticationMethod::None,
            session: None,
            attributes: HashMap::new(),
            created_at: chrono::Utc::now(),
            expires_at: None,
        }
    }

    /// Check if the context has expired
    #[must_use] pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expiry) => chrono::Utc::now() > expiry,
            None => false,
        }
    }

    /// Check if the context has a specific permission
    #[must_use] pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    /// Add a permission to the context
    pub fn add_permission(&mut self, permission: Permission) {
        if !self.permissions.contains(&permission) {
            self.permissions.push(permission);
        }
    }

    /// Set session information
    #[must_use] pub fn with_session(mut self, session: SessionInfo) -> Self {
        self.session = Some(session);
        self
    }

    /// Set expiration time
    #[must_use] pub fn with_expiration(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Add an attribute
    pub fn with_attribute<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// Principal represents an identity (user, service, system)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Principal {
    /// Principal ID
    pub id: String,
    /// Principal type
    pub principal_type: PrincipalType,
    /// Display name
    pub name: String,
    /// Additional attributes
    pub attributes: HashMap<String, String>,
}

impl Principal {
    /// Create a new user principal
    pub fn user<S: Into<String>>(id: S, name: S) -> Self {
        Self {
            id: id.into(),
            principal_type: PrincipalType::User,
            name: name.into(),
            attributes: HashMap::new(),
        }
    }

    /// Create a new service principal
    pub fn service<S: Into<String>>(id: S, name: S) -> Self {
        Self {
            id: id.into(),
            principal_type: PrincipalType::Service,
            name: name.into(),
            attributes: HashMap::new(),
        }
    }

    /// Create a new system principal
    pub fn system<S: Into<String>>(id: S, name: S) -> Self {
        Self {
            id: id.into(),
            principal_type: PrincipalType::System,
            name: name.into(),
            attributes: HashMap::new(),
        }
    }

    /// Add an attribute
    pub fn with_attribute<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// Types of principals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrincipalType {
    /// Human user
    User,
    /// Service account
    Service,
    /// System account
    System,
    /// Anonymous
    Anonymous,
}

/// Permission represents an allowed action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Permission {
    /// Resource being accessed
    pub resource: String,
    /// Action being performed
    pub action: String,
    /// Optional conditions
    pub conditions: Vec<String>,
}

impl Permission {
    /// Create a new permission
    pub fn new<R: Into<String>, A: Into<String>>(resource: R, action: A) -> Self {
        Self {
            resource: resource.into(),
            action: action.into(),
            conditions: Vec::new(),
        }
    }

    /// Add a condition to the permission
    pub fn with_condition<C: Into<String>>(mut self, condition: C) -> Self {
        self.conditions.push(condition.into());
        self
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.conditions.is_empty() {
            write!(f, "{}:{}", self.resource, self.action)
        } else {
            write!(f, "{}:{} ({})", self.resource, self.action, self.conditions.join(", "))
        }
    }
}

/// Authentication methods
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthenticationMethod {
    /// No authentication
    None,
    /// Password-based authentication
    Password,
    /// Token-based authentication
    Token,
    /// Certificate-based authentication
    Certificate,
    /// Multi-factor authentication
    MultiFactor,
    /// Biometric authentication
    Biometric,
    /// OAuth authentication
    OAuth,
    /// DID-based authentication
    Did,
    /// Post-quantum authentication
    PostQuantum,
}

impl std::fmt::Display for AuthenticationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthenticationMethod::None => write!(f, "None"),
            AuthenticationMethod::Password => write!(f, "Password"),
            AuthenticationMethod::Token => write!(f, "Token"),
            AuthenticationMethod::Certificate => write!(f, "Certificate"),
            AuthenticationMethod::MultiFactor => write!(f, "Multi-Factor"),
            AuthenticationMethod::Biometric => write!(f, "Biometric"),
            AuthenticationMethod::OAuth => write!(f, "OAuth"),
            AuthenticationMethod::Did => write!(f, "DID"),
            AuthenticationMethod::PostQuantum => write!(f, "Post-Quantum"),
        }
    }
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID
    pub session_id: String,
    /// Session start time
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Last activity time
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// Client information
    pub client_info: ClientInfo,
    /// Session attributes
    pub attributes: HashMap<String, String>,
}

/// Client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Client type
    pub client_type: String,
    /// Client version
    pub client_version: Option<String>,
}

/// Trait for security-aware components
#[async_trait]
pub trait SecurityAware: Send + Sync {
    /// Get the required security level for this component
    fn required_security_level(&self) -> SecurityLevel;

    /// Validate a security context
    async fn validate_security_context(&self, context: &SecurityContext) -> SynapsedResult<bool>;

    /// Get required permissions for an operation
    fn required_permissions(&self, operation: &str) -> Vec<Permission>;

    /// Check if an operation is authorized
    async fn is_authorized(&self, context: &SecurityContext, operation: &str) -> SynapsedResult<bool> {
        // Check if context is valid
        if !self.validate_security_context(context).await? {
            return Ok(false);
        }

        // Check if context has expired
        if context.is_expired() {
            return Ok(false);
        }

        // Check security level
        if !context.security_level.meets_requirement(self.required_security_level()) {
            return Ok(false);
        }

        // Check permissions
        let required_perms = self.required_permissions(operation);
        for perm in required_perms {
            if !context.has_permission(&perm) {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

/// Trait for authentication providers
#[async_trait]
pub trait AuthenticationProvider: Send + Sync {
    /// Authenticate a principal with credentials
    async fn authenticate(&self, credentials: &Credentials) -> SynapsedResult<AuthenticationResult>;

    /// Validate an existing authentication token
    async fn validate_token(&self, token: &str) -> SynapsedResult<Principal>;

    /// Refresh an authentication token
    async fn refresh_token(&self, refresh_token: &str) -> SynapsedResult<AuthenticationResult>;

    /// Revoke an authentication token
    async fn revoke_token(&self, token: &str) -> SynapsedResult<()>;

    /// Get supported authentication methods
    fn supported_methods(&self) -> Vec<AuthenticationMethod>;
}

/// Credentials for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Credentials {
    /// Username and password
    Password {
        /// Username
        username: String,
        /// Password
        password: String,
    },
    /// Token-based credentials
    Token {
        /// Authentication token
        token: String,
    },
    /// Certificate-based credentials
    Certificate {
        /// Certificate data
        certificate: Vec<u8>,
        /// Optional private key
        private_key: Option<Vec<u8>>,
    },
    /// DID-based credentials
    Did {
        /// Decentralized identifier
        did: String,
        /// Cryptographic proof
        proof: Vec<u8>,
    },
    /// Custom credentials
    Custom {
        /// Authentication method
        method: String,
        /// Additional data
        data: HashMap<String, String>,
    },
}

/// Authentication result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationResult {
    /// Authenticated principal
    pub principal: Principal,
    /// Access token
    pub access_token: String,
    /// Refresh token
    pub refresh_token: Option<String>,
    /// Token expiration time
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Authentication method used
    pub method: AuthenticationMethod,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Trait for authorization providers
#[async_trait]
pub trait AuthorizationProvider: Send + Sync {
    /// Check if a principal has permission for an operation
    async fn check_permission(&self, principal: &Principal, permission: &Permission) -> SynapsedResult<bool>;

    /// Get all permissions for a principal
    async fn get_permissions(&self, principal: &Principal) -> SynapsedResult<Vec<Permission>>;

    /// Grant a permission to a principal
    async fn grant_permission(&self, principal: &Principal, permission: Permission) -> SynapsedResult<()>;

    /// Revoke a permission from a principal
    async fn revoke_permission(&self, principal: &Principal, permission: &Permission) -> SynapsedResult<()>;
}

/// Security policy for access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Policy ID
    pub id: String,
    /// Policy name
    pub name: String,
    /// Policy description
    pub description: String,
    /// Rules in the policy
    pub rules: Vec<SecurityRule>,
    /// Policy priority
    pub priority: u32,
    /// Policy enabled status
    pub enabled: bool,
}

impl SecurityPolicy {
    /// Create a new security policy
    pub fn new<S: Into<String>>(id: S, name: S, description: S) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            rules: Vec::new(),
            priority: 0,
            enabled: true,
        }
    }

    /// Add a rule to the policy
    pub fn add_rule(&mut self, rule: SecurityRule) -> &mut Self {
        self.rules.push(rule);
        self
    }

    /// Check if the policy applies to a context and operation
    #[must_use] pub fn applies_to(&self, context: &SecurityContext, operation: &str) -> bool {
        if !self.enabled {
            return false;
        }

        self.rules.iter().any(|rule| rule.matches(context, operation))
    }
}

/// Security rule within a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRule {
    /// Rule ID
    pub id: String,
    /// Rule conditions
    pub conditions: Vec<SecurityCondition>,
    /// Rule effect (allow/deny)
    pub effect: SecurityEffect,
    /// Resources this rule applies to
    pub resources: Vec<String>,
    /// Actions this rule applies to
    pub actions: Vec<String>,
}

impl SecurityRule {
    /// Check if this rule matches a context and operation
    #[must_use] pub fn matches(&self, context: &SecurityContext, operation: &str) -> bool {
        // Check if operation matches
        if !self.actions.is_empty() && !self.actions.contains(&operation.to_string()) {
            return false;
        }

        // Check all conditions
        self.conditions.iter().all(|condition| condition.evaluate(context))
    }
}

/// Security rule condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityCondition {
    /// Principal type condition
    PrincipalType(PrincipalType),
    /// Security level condition
    SecurityLevel(SecurityLevel),
    /// Authentication method condition
    AuthMethod(AuthenticationMethod),
    /// Time-based condition
    TimeRange {
        /// Start time
        start: chrono::NaiveTime,
        /// End time
        end: chrono::NaiveTime,
    },
    /// IP address condition
    IpAddress(String),
    /// Custom condition
    Custom {
        /// Condition name
        name: String,
        /// Condition value
        value: String,
    },
}

impl SecurityCondition {
    /// Evaluate the condition against a security context
    #[must_use] pub fn evaluate(&self, context: &SecurityContext) -> bool {
        match self {
            SecurityCondition::PrincipalType(expected) => {
                context.principal.principal_type == *expected
            }
            SecurityCondition::SecurityLevel(required) => {
                context.security_level.meets_requirement(*required)
            }
            SecurityCondition::AuthMethod(expected) => {
                context.auth_method == *expected
            }
            SecurityCondition::TimeRange { start, end } => {
                let current_time = chrono::Utc::now().time();
                current_time >= *start && current_time <= *end
            }
            SecurityCondition::IpAddress(expected) => {
                context.session.as_ref()
                    .and_then(|s| s.client_info.ip_address.as_ref())
                    .is_some_and(|ip| ip == expected)
            }
            SecurityCondition::Custom { name, value } => {
                context.attributes.get(name)
                    .is_some_and(|v| v == value)
            }
        }
    }
}

/// Security rule effect
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecurityEffect {
    /// Allow the operation
    Allow,
    /// Deny the operation
    Deny,
}

/// Utility functions for common security operations
pub mod utils {
    use super::{Uuid, SecurityContext, Principal, SecurityLevel};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Generate a secure random session ID
    #[must_use] pub fn generate_session_id() -> String {
        format!("sess_{}", Uuid::new_v4())
    }

    /// Generate a secure random token
    #[must_use] pub fn generate_token() -> String {
        format!("tok_{}", Uuid::new_v4())
    }

    /// Get current timestamp as seconds since epoch
    #[must_use] pub fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Create a basic security context for testing
    #[must_use] pub fn create_test_context() -> SecurityContext {
        let principal = Principal::user("test_user", "Test User");
        SecurityContext::new(principal, SecurityLevel::Basic)
    }

    /// Create a service security context
    #[must_use] pub fn create_service_context(service_id: &str, service_name: &str) -> SecurityContext {
        let principal = Principal::service(service_id, service_name);
        SecurityContext::new(principal, SecurityLevel::Standard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_level_comparison() {
        assert!(SecurityLevel::High > SecurityLevel::Basic);
        assert!(SecurityLevel::Maximum.meets_requirement(SecurityLevel::Standard));
        assert!(!SecurityLevel::Basic.meets_requirement(SecurityLevel::High));
    }

    #[test]
    fn test_principal_creation() {
        let user = Principal::user("user123", "John Doe")
            .with_attribute("department", "engineering");
        
        assert_eq!(user.id, "user123");
        assert_eq!(user.name, "John Doe");
        assert_eq!(user.principal_type, PrincipalType::User);
        assert_eq!(user.attributes.get("department"), Some(&"engineering".to_string()));
    }

    #[test]
    fn test_permission() {
        let perm = Permission::new("users", "read")
            .with_condition("own_data_only");
        
        assert_eq!(perm.resource, "users");
        assert_eq!(perm.action, "read");
        assert_eq!(perm.conditions, vec!["own_data_only"]);
    }

    #[test]
    fn test_security_context() {
        let principal = Principal::user("test", "Test User");
        let mut context = SecurityContext::new(principal, SecurityLevel::Standard);
        
        let permission = Permission::new("documents", "read");
        context.add_permission(permission.clone());
        
        assert!(context.has_permission(&permission));
        assert!(!context.is_expired());
    }

    #[test]
    fn test_security_condition_evaluation() {
        let principal = Principal::user("test", "Test User");
        let context = SecurityContext::new(principal, SecurityLevel::High);
        
        let condition = SecurityCondition::SecurityLevel(SecurityLevel::Standard);
        assert!(condition.evaluate(&context));
        
        let condition = SecurityCondition::PrincipalType(PrincipalType::User);
        assert!(condition.evaluate(&context));
        
        let condition = SecurityCondition::PrincipalType(PrincipalType::Service);
        assert!(!condition.evaluate(&context));
    }

    #[test]
    fn test_utils() {
        let session_id = utils::generate_session_id();
        assert!(session_id.starts_with("sess_"));
        
        let token = utils::generate_token();
        assert!(token.starts_with("tok_"));
        
        let timestamp = utils::current_timestamp();
        assert!(timestamp > 0);
    }
}