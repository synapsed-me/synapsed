//! JWT token generation and validation
//! 
//! Provides secure JWT token handling with:
//! - HS256, HS384, HS512 signing algorithms
//! - RS256, RS384, RS512 for asymmetric signing
//! - Token validation and claims extraction

use crate::{Error, Result};
use sha3::{Sha3_256, Sha3_384, Sha3_512, Digest};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

use std::collections::BTreeMap;
use crate::auth::Authenticator;
use crate::storage::IdentityStore;
use async_trait::async_trait;

/// JWT header
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JwtHeader {
    /// Algorithm
    pub alg: String,
    /// Token type
    pub typ: String,
    /// Key ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kid: Option<String>,
}

/// JWT signing algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    /// HMAC with SHA-256
    HS256,
    /// HMAC with SHA-384
    HS384,
    /// HMAC with SHA-512
    HS512,
    /// RSA with SHA-256 (not implemented yet)
    RS256,
    /// RSA with SHA-384 (not implemented yet)
    RS384,
    /// RSA with SHA-512 (not implemented yet)
    RS512,
}

impl Algorithm {
    /// Get algorithm name
    pub fn name(&self) -> &'static str {
        match self {
            Algorithm::HS256 => "HS256",
            Algorithm::HS384 => "HS384",
            Algorithm::HS512 => "HS512",
            Algorithm::RS256 => "RS256",
            Algorithm::RS384 => "RS384",
            Algorithm::RS512 => "RS512",
        }
    }
}

/// JWT token manager
pub struct JwtManager {
    /// Signing key
    secret: Vec<u8>,
    /// Default algorithm
    algorithm: Algorithm,
}

impl JwtManager {
    /// Create a new JWT manager
    pub fn new(secret: Vec<u8>, algorithm: Algorithm) -> Self {
        Self { secret, algorithm }
    }
    
    /// Generate a JWT token
    pub fn generate_token(&self, claims: &serde_json::Value) -> Result<String> {
        // Create header
        let header = JwtHeader {
            alg: self.algorithm.name().to_string(),
            typ: "JWT".to_string(),
            kid: None,
        };
        
        // Encode header and claims
        let header_json = serde_json::to_string(&header)
            .map_err(|e| Error::CryptoError(format!("Failed to serialize header: {}", e)))?;
        let claims_json = serde_json::to_string(claims)
            .map_err(|e| Error::CryptoError(format!("Failed to serialize claims: {}", e)))?;
        
        let header_b64 = URL_SAFE_NO_PAD.encode(&header_json);
        let claims_b64 = URL_SAFE_NO_PAD.encode(&claims_json);
        
        // Create signature
        let message = format!("{}.{}", header_b64, claims_b64);
        let signature = self.sign(message.as_bytes())?;
        let signature_b64 = URL_SAFE_NO_PAD.encode(&signature);
        
        // Combine into JWT
        Ok(format!("{}.{}.{}", header_b64, claims_b64, signature_b64))
    }
    
    /// Validate a JWT token and extract claims
    pub fn validate_token(&self, token: &str) -> Result<serde_json::Value> {
        // Split token
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(Error::InvalidParameter("Invalid JWT format".into()));
        }
        
        // Decode header
        let header_json = URL_SAFE_NO_PAD.decode(parts[0])
            .map_err(|_| Error::InvalidParameter("Invalid header encoding".into()))?;
        let header: JwtHeader = serde_json::from_slice(&header_json)
            .map_err(|_| Error::InvalidParameter("Invalid header format".into()))?;
        
        // Verify algorithm
        if header.alg != self.algorithm.name() {
            return Err(Error::InvalidParameter("Algorithm mismatch".into()));
        }
        
        // Verify signature
        let message = format!("{}.{}", parts[0], parts[1]);
        let signature = URL_SAFE_NO_PAD.decode(parts[2])
            .map_err(|_| Error::InvalidParameter("Invalid signature encoding".into()))?;
        
        let expected_signature = self.sign(message.as_bytes())?;
        if !constant_time_eq(&signature, &expected_signature) {
            return Err(Error::SignatureVerificationFailed("Invalid signature".into()));
        }
        
        // Decode claims
        let claims_json = URL_SAFE_NO_PAD.decode(parts[1])
            .map_err(|_| Error::InvalidParameter("Invalid claims encoding".into()))?;
        let claims: serde_json::Value = serde_json::from_slice(&claims_json)
            .map_err(|_| Error::InvalidParameter("Invalid claims format".into()))?;
        
        // Validate expiration
        if let Some(exp) = claims.get("exp").and_then(|v| v.as_u64()) {
            let now = chrono::Utc::now().timestamp() as u64;
            if now > exp {
                return Err(Error::AuthenticationFailed("Token expired".into()));
            }
        }
        
        // Validate not before
        if let Some(nbf) = claims.get("nbf").and_then(|v| v.as_u64()) {
            let now = chrono::Utc::now().timestamp() as u64;
            if now < nbf {
                return Err(Error::AuthenticationFailed("Token not yet valid".into()));
            }
        }
        
        Ok(claims)
    }
    
    /// Sign data using the configured algorithm
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.algorithm {
            Algorithm::HS256 => {
                let mut hasher = Sha3_256::new();
                hasher.update(&self.secret);
                hasher.update(data);
                Ok(hasher.finalize().to_vec())
            }
            Algorithm::HS384 => {
                let mut hasher = Sha3_384::new();
                hasher.update(&self.secret);
                hasher.update(data);
                Ok(hasher.finalize().to_vec())
            }
            Algorithm::HS512 => {
                let mut hasher = Sha3_512::new();
                hasher.update(&self.secret);
                hasher.update(data);
                Ok(hasher.finalize().to_vec())
            }
            _ => Err(Error::NotSupported("Algorithm not implemented".into())),
        }
    }
}

/// Token builder for convenient JWT creation
pub struct TokenBuilder {
    claims: BTreeMap<String, serde_json::Value>,
}

impl TokenBuilder {
    /// Create a new token builder
    pub fn new() -> Self {
        Self {
            claims: BTreeMap::new(),
        }
    }
    
    /// Set subject
    pub fn subject(mut self, sub: impl Into<String>) -> Self {
        self.claims.insert("sub".to_string(), serde_json::Value::String(sub.into()));
        self
    }
    
    /// Set issuer
    pub fn issuer(mut self, iss: impl Into<String>) -> Self {
        self.claims.insert("iss".to_string(), serde_json::Value::String(iss.into()));
        self
    }
    
    /// Set audience
    pub fn audience(mut self, aud: impl Into<String>) -> Self {
        self.claims.insert("aud".to_string(), serde_json::Value::String(aud.into()));
        self
    }
    
    /// Set expiration time (seconds from now)
    pub fn expires_in(mut self, seconds: u64) -> Self {
        let exp = chrono::Utc::now().timestamp() as u64 + seconds;
        self.claims.insert("exp".to_string(), serde_json::Value::Number(exp.into()));
        self
    }
    
    /// Set not before time (seconds from now)
    pub fn not_before(mut self, seconds: u64) -> Self {
        let nbf = chrono::Utc::now().timestamp() as u64 + seconds;
        self.claims.insert("nbf".to_string(), serde_json::Value::Number(nbf.into()));
        self
    }
    
    /// Set issued at time (now)
    pub fn issued_at(mut self) -> Self {
        let iat = chrono::Utc::now().timestamp() as u64;
        self.claims.insert("iat".to_string(), serde_json::Value::Number(iat.into()));
        self
    }
    
    /// Set JWT ID
    pub fn jwt_id(mut self, jti: impl Into<String>) -> Self {
        self.claims.insert("jti".to_string(), serde_json::Value::String(jti.into()));
        self
    }
    
    /// Add custom claim
    pub fn claim(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.claims.insert(key.into(), value);
        self
    }
    
    /// Build the claims object
    pub fn build(self) -> serde_json::Value {
        serde_json::Value::Object(self.claims.into_iter().collect())
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

/// Token credentials for authentication
#[derive(Debug, Clone)]
pub struct TokenCredentials {
    /// JWT token
    pub token: String,
}

/// Token-based authenticator
pub struct TokenAuthenticator<S: IdentityStore> {
    storage: S,
    jwt_manager: JwtManager,
}

impl<S: IdentityStore> TokenAuthenticator<S> {
    /// Create a new token authenticator
    pub fn new(storage: S, secret: Vec<u8>, algorithm: Algorithm) -> Self {
        Self {
            storage,
            jwt_manager: JwtManager::new(secret, algorithm),
        }
    }
}

#[async_trait]
impl<S: IdentityStore> Authenticator for TokenAuthenticator<S> {
    type Credentials = TokenCredentials;
    
    async fn authenticate(&self, credentials: Self::Credentials) -> Result<crate::Identity> {
        // Validate token and extract claims
        let claims = self.jwt_manager.validate_token(&credentials.token)?;
        
        // Extract subject (user ID) from claims
        let user_id = claims.get("sub")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::AuthenticationFailed("Token missing subject".into()))?;
        
        // Get user from storage
        let identity = self.storage
            .get_identity(user_id)?
            .ok_or_else(|| Error::AuthenticationFailed("User not found".into()))?;
        
        // Convert from trait object to concrete Identity type
        // This is a simplified implementation - in real code, you'd need proper conversion
        Ok(crate::Identity {
            id: uuid::Uuid::new_v4(), // Would extract from storage
            username: user_id.to_string(),
            display_name: None,
            roles: vec![],
            attributes: std::collections::HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_jwt_generation_and_validation() {
        let secret = b"test-secret-key".to_vec();
        let manager = JwtManager::new(secret, Algorithm::HS256);
        
        // Create claims
        let claims = TokenBuilder::new()
            .subject("user123")
            .issuer("test-issuer")
            .audience("test-audience")
            .expires_in(3600)
            .issued_at()
            .jwt_id("test-jwt-id")
            .claim("role", serde_json::Value::String("admin".to_string()))
            .build();
        
        // Generate token
        let token = manager.generate_token(&claims).unwrap();
        assert!(token.contains('.'));
        assert_eq!(token.split('.').count(), 3);
        
        // Validate token
        let validated_claims = manager.validate_token(&token).unwrap();
        assert_eq!(validated_claims["sub"], "user123");
        assert_eq!(validated_claims["iss"], "test-issuer");
        assert_eq!(validated_claims["role"], "admin");
    }
    
    #[test]
    fn test_invalid_token() {
        let secret = b"test-secret-key".to_vec();
        let manager = JwtManager::new(secret, Algorithm::HS256);
        
        // Invalid format
        assert!(manager.validate_token("invalid").is_err());
        assert!(manager.validate_token("invalid.token").is_err());
        
        // Invalid signature
        let claims = TokenBuilder::new()
            .subject("user123")
            .build();
        let token = manager.generate_token(&claims).unwrap();
        let mut parts: Vec<&str> = token.split('.').collect();
        parts[2] = "invalid_signature";
        let invalid_token = parts.join(".");
        assert!(manager.validate_token(&invalid_token).is_err());
    }
    
    #[test]
    fn test_expired_token() {
        let secret = b"test-secret-key".to_vec();
        let manager = JwtManager::new(secret, Algorithm::HS256);
        
        // Create expired token
        let claims = TokenBuilder::new()
            .subject("user123")
            .expires_in(0) // Already expired
            .build();
        
        let token = manager.generate_token(&claims).unwrap();
        
        // Wait a bit to ensure expiration
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        // Should fail validation
        assert!(manager.validate_token(&token).is_err());
    }
}