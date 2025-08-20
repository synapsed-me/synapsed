//! Authentication module providing various authentication mechanisms

use async_trait::async_trait;
use crate::{Identity, Result};

/// Core trait for authentication mechanisms
#[async_trait]
pub trait Authenticator: Send + Sync {
    /// Type of credentials this authenticator accepts
    type Credentials: Send;
    
    /// Authenticate with the provided credentials
    async fn authenticate(&self, credentials: Self::Credentials) -> Result<Identity>;
}

/// Password-based authentication
pub mod password;

/// Token-based authentication
pub mod token;

/// OAuth provider integration
// TODO: Implement OAuth module
// #[cfg(feature = "oauth")]
// pub mod oauth;

// Re-export common types
pub use password::{PasswordAuthenticator, PasswordCredentials};
pub use token::{TokenAuthenticator, TokenCredentials};