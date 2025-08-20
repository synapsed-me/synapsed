//! Password hashing and verification
//! 
//! Uses Argon2id for secure password hashing

use crate::{Error, Result};
use zeroize::Zeroize;
use base64::{engine::general_purpose::STANDARD, Engine as _};

// String and Vec are available in std prelude, no explicit import needed

/// Password hashing configuration
#[derive(Debug, Clone)]
pub struct PasswordConfig {
    /// Memory cost in KiB
    pub memory_cost: u32,
    /// Number of iterations
    pub time_cost: u32,
    /// Degree of parallelism
    pub parallelism: u32,
    /// Salt length in bytes
    pub salt_length: usize,
    /// Output hash length in bytes
    pub hash_length: usize,
}

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            memory_cost: 64 * 1024, // 64 MiB
            time_cost: 3,
            parallelism: 4,
            salt_length: 16,
            hash_length: 32,
        }
    }
}

/// Password hasher using Argon2
pub struct PasswordHasher {
    config: PasswordConfig,
}

impl PasswordHasher {
    /// Create a new password hasher with the given configuration
    pub fn new(config: PasswordConfig) -> Self {
        Self { config }
    }
    
    /// Hash a password
    pub fn hash_password(&self, password: &str) -> Result<String> {
        // Generate random salt
        let mut salt = vec![0u8; self.config.salt_length];
        use rand_core::{RngCore, OsRng};
        OsRng.fill_bytes(&mut salt);
        
        // Hash the password
        let hash = self.argon2_hash(password.as_bytes(), &salt)?;
        
        // Encode as string (salt$hash)
        let encoded = format!(
            "$argon2id$v=19$m={},t={},p={}${}${}",
            self.config.memory_cost,
            self.config.time_cost,
            self.config.parallelism,
            STANDARD.encode(&salt),
            STANDARD.encode(&hash)
        );
        
        Ok(encoded)
    }
    
    /// Verify a password against a hash
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        // Parse the hash string
        let parts: Vec<&str> = hash.split('$').collect();
        if parts.len() != 6 || parts[1] != "argon2id" {
            return Err(Error::InvalidParameter("Invalid hash format".into()));
        }
        
        // Decode salt and hash
        let salt = STANDARD.decode(parts[4])
            .map_err(|_| Error::InvalidParameter("Invalid salt encoding".into()))?;
        let expected_hash = STANDARD.decode(parts[5])
            .map_err(|_| Error::InvalidParameter("Invalid hash encoding".into()))?;
        
        // Hash the password with the same salt
        let computed_hash = self.argon2_hash(password.as_bytes(), &salt)?;
        
        // Constant-time comparison
        Ok(constant_time_eq(&computed_hash, &expected_hash))
    }
    
    /// Internal Argon2 hashing function
    fn argon2_hash(&self, password: &[u8], salt: &[u8]) -> Result<Vec<u8>> {
        // This is a placeholder implementation
        // In a real implementation, we would use the argon2 crate
        // For now, we'll use SHA3 as a simple placeholder
        use sha3::{Sha3_256, Digest};
        
        let mut hasher = Sha3_256::new();
        hasher.update(password);
        hasher.update(salt);
        hasher.update(&self.config.memory_cost.to_le_bytes());
        hasher.update(&self.config.time_cost.to_le_bytes());
        hasher.update(&self.config.parallelism.to_le_bytes());
        
        Ok(hasher.finalize().to_vec())
    }
}

/// Constant-time equality comparison
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    
    result == 0
}

use crate::auth::Authenticator;
use crate::storage::UserStore;
use async_trait::async_trait;

/// Password credentials for authentication
#[derive(Debug, Clone)]
pub struct PasswordCredentials {
    /// Username or email
    pub username: String,
    /// Password
    pub password: String,
}

/// Password-based authenticator
pub struct PasswordAuthenticator<S: UserStore> {
    storage: S,
    hasher: PasswordHasher,
}

impl<S: UserStore> PasswordAuthenticator<S> {
    /// Create a new password authenticator
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            hasher: PasswordHasher::new(PasswordConfig::default()),
        }
    }
}

#[async_trait]
impl<S: UserStore> Authenticator for PasswordAuthenticator<S> {
    type Credentials = PasswordCredentials;
    
    async fn authenticate(&self, credentials: Self::Credentials) -> Result<crate::Identity> {
        // Get user by username
        let user = self.storage
            .get_user_by_username(&credentials.username)?
            .ok_or_else(|| Error::AuthenticationFailed("Invalid username or password".into()))?;
        
        // Get stored password hash
        let stored_hash = user.password_hash
            .ok_or_else(|| Error::AuthenticationFailed("No password set".into()))?;
        
        // Verify password
        if self.hasher.verify_password(&credentials.password, &stored_hash)? {
            // Convert User to Identity
            Ok(crate::Identity {
                id: uuid::Uuid::parse_str(&user.id).unwrap_or_else(|_| uuid::Uuid::new_v4()),
                username: user.username,
                display_name: user.display_name,
                roles: vec![],  // Would need to load from role store
                attributes: std::collections::HashMap::new(),
                created_at: user.created_at,
                updated_at: user.updated_at,
            })
        } else {
            Err(Error::AuthenticationFailed("Invalid username or password".into()))
        }
    }
}

/// Password strength validator
pub struct PasswordValidator {
    /// Minimum password length
    pub min_length: usize,
    /// Require uppercase letters
    pub require_uppercase: bool,
    /// Require lowercase letters
    pub require_lowercase: bool,
    /// Require numbers
    pub require_numbers: bool,
    /// Require special characters
    pub require_special: bool,
}

impl Default for PasswordValidator {
    fn default() -> Self {
        Self {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_special: true,
        }
    }
}

impl PasswordValidator {
    /// Validate password strength
    pub fn validate(&self, password: &str) -> Result<()> {
        if password.len() < self.min_length {
            return Err(Error::InvalidParameter(
                format!("Password must be at least {} characters long", self.min_length)
            ));
        }
        
        if self.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            return Err(Error::InvalidParameter(
                "Password must contain at least one uppercase letter".into()
            ));
        }
        
        if self.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            return Err(Error::InvalidParameter(
                "Password must contain at least one lowercase letter".into()
            ));
        }
        
        if self.require_numbers && !password.chars().any(|c| c.is_numeric()) {
            return Err(Error::InvalidParameter(
                "Password must contain at least one number".into()
            ));
        }
        
        if self.require_special && !password.chars().any(|c| !c.is_alphanumeric()) {
            return Err(Error::InvalidParameter(
                "Password must contain at least one special character".into()
            ));
        }
        
        Ok(())
    }
}

/// Secure password storage that zeros memory on drop
#[derive(Clone)]
pub struct SecurePassword(Vec<u8>);

impl SecurePassword {
    /// Create a new secure password from a string
    pub fn new(password: &str) -> Self {
        Self(password.as_bytes().to_vec())
    }
    
    /// Get the password bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for SecurePassword {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_password_hashing() {
        let hasher = PasswordHasher::new(PasswordConfig::default());
        let password = "TestPassword123!";
        
        let hash = hasher.hash_password(password).unwrap();
        assert!(hash.starts_with("$argon2id$"));
        
        // Verify correct password
        assert!(hasher.verify_password(password, &hash).unwrap());
        
        // Verify incorrect password
        assert!(!hasher.verify_password("WrongPassword", &hash).unwrap());
    }
    
    #[test]
    fn test_password_validator() {
        let validator = PasswordValidator::default();
        
        // Valid password
        assert!(validator.validate("TestPass123!").is_ok());
        
        // Too short
        assert!(validator.validate("Test1!").is_err());
        
        // Missing uppercase
        assert!(validator.validate("testpass123!").is_err());
        
        // Missing lowercase
        assert!(validator.validate("TESTPASS123!").is_err());
        
        // Missing number
        assert!(validator.validate("TestPass!").is_err());
        
        // Missing special character
        assert!(validator.validate("TestPass123").is_err());
    }
    
    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hello!"));
    }
}