//! Error types for synapsed-identity

use thiserror::Error;

/// Result type alias for synapsed-identity operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for synapsed-identity
#[derive(Error, Debug)]
pub enum Error {
    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Authorization denied
    #[error("Authorization denied: {0}")]
    AuthorizationDenied(String),

    /// User not found
    #[error("User not found: {0}")]
    UserNotFound(String),

    /// Invalid credentials
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Invalid token
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    /// Session expired
    #[error("Session expired")]
    SessionExpired,

    /// Session not found
    #[error("Session not found")]
    SessionNotFound,

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Resource already exists
    #[error("Already exists: {0}")]
    AlreadyExists(String),

    /// Session error
    #[error("Session error: {0}")]
    SessionError(String),

    /// Authorization failed
    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),

    /// Operation not supported
    #[error("Not supported: {0}")]
    NotSupported(String),

    /// Cryptographic error
    #[error("Crypto error: {0}")]
    CryptoError(String),

    /// Signature verification failed
    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),

    /// Invalid parameter
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Password validation failed
    #[error("Password validation failed: {0}")]
    PasswordValidation(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Cryptographic operation failed
    #[error("Cryptographic error: {0}")]
    Crypto(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Database error
    #[cfg(feature = "sqlx")]
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Redis error
    #[cfg(feature = "redis")]
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// JWT error
    #[error("JWT error: {0}")]
    Jwt(String),

    /// DID parsing error
    #[error("DID parsing error: {0}")]
    DidParsingError(String),

    /// DID method error
    #[error("DID method error: {0}")]
    DidMethodError(String),

    /// DID resolution error
    #[error("DID resolution error: {0}")]
    DidResolutionError(String),

    /// DID document error
    #[error("DID document error: {0}")]
    DidDocumentError(String),

    /// Key management error
    #[error("Key management error: {0}")]
    KeyManagementError(String),

    /// Zero-knowledge proof error
    #[error("ZK proof error: {0}")]
    ZkProofError(String),

    /// PWA error
    #[error("PWA error: {0}")]
    PwaError(String),

    /// WebAuthn error
    #[error("WebAuthn error: {0}")]
    WebAuthnError(String),

    /// Storage error (alias for compatibility)
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Authentication error (general)
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Cryptographic error (specific)
    #[error("Cryptographic error: {0}")]
    CryptographicError(String),

    /// Configuration error (specific)
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// HTTP error (for OAuth)
    #[cfg(feature = "oauth")]
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Subscription-related errors
    #[error("Subscription error: {0}")]
    SubscriptionError(String),

    /// Generic error
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Storage(_) 
            | Error::Database(_) 
            | Error::Other(_)
        )
    }

    /// Check if this error is a client error (4xx-like)
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Error::AuthenticationFailed(_)
                | Error::AuthorizationDenied(_)
                | Error::InvalidCredentials
                | Error::InvalidToken(_)
                | Error::PasswordValidation(_)
                | Error::Validation(_)
                | Error::RateLimitExceeded
                | Error::DidParsingError(_)
                | Error::DidMethodError(_)
                | Error::DidDocumentError(_)
                | Error::AuthenticationError(_)
                | Error::WebAuthnError(_)
        )
    }

    /// Check if this error is a server error (5xx-like)
    pub fn is_server_error(&self) -> bool {
        matches!(
            self,
            Error::Storage(_)
                | Error::Configuration(_)
                | Error::Crypto(_)
                | Error::Database(_)
                | Error::Other(_)
                | Error::DidResolutionError(_)
                | Error::KeyManagementError(_)
                | Error::ZkProofError(_)
                | Error::StorageError(_)
                | Error::CryptographicError(_)
                | Error::ConfigurationError(_)
                | Error::PwaError(_)
        )
    }
}

/// Authentication-specific errors
#[derive(Error, Debug)]
pub enum AuthError {
    /// Invalid username format
    #[error("Invalid username format")]
    InvalidUsername,

    /// Password too weak
    #[error("Password too weak: {0}")]
    WeakPassword(String),

    /// Account locked
    #[error("Account locked until {0}")]
    AccountLocked(chrono::DateTime<chrono::Utc>),

    /// Two-factor authentication required
    #[error("Two-factor authentication required")]
    TwoFactorRequired,

    /// Invalid two-factor code
    #[error("Invalid two-factor code")]
    InvalidTwoFactorCode,
}

/// Authorization-specific errors
#[derive(Error, Debug)]
pub enum AuthzError {
    /// Role not found
    #[error("Role not found: {0}")]
    RoleNotFound(String),

    /// Permission not found
    #[error("Permission not found: {0}")]
    PermissionNotFound(String),

    /// Policy evaluation failed
    #[error("Policy evaluation failed: {0}")]
    PolicyEvaluationFailed(String),

    /// Insufficient permissions
    #[error("Insufficient permissions for {resource}:{action}")]
    InsufficientPermissions { 
        /// Resource being accessed
        resource: String, 
        /// Action being performed
        action: String 
    },
}

impl From<AuthError> for Error {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::InvalidUsername => Error::Validation("Invalid username format".to_string()),
            AuthError::WeakPassword(msg) => Error::PasswordValidation(msg),
            AuthError::AccountLocked(_) => {
                Error::AuthenticationFailed("Account locked".to_string())
            }
            AuthError::TwoFactorRequired | AuthError::InvalidTwoFactorCode => {
                Error::AuthenticationFailed(err.to_string())
            }
        }
    }
}

impl From<AuthzError> for Error {
    fn from(err: AuthzError) -> Self {
        Error::AuthorizationDenied(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categorization() {
        assert!(Error::InvalidCredentials.is_client_error());
        assert!(!Error::InvalidCredentials.is_server_error());
        assert!(!Error::InvalidCredentials.is_retryable());

        assert!(Error::Storage("connection failed".to_string()).is_server_error());
        assert!(Error::Storage("connection failed".to_string()).is_retryable());
    }

    #[test]
    fn test_auth_error_conversion() {
        let auth_err = AuthError::WeakPassword("too short".to_string());
        let err: Error = auth_err.into();
        match err {
            Error::PasswordValidation(msg) => assert_eq!(msg, "too short"),
            _ => panic!("Wrong error type"),
        }
    }
}