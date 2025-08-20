//! Session management for authenticated users
//! 
//! Provides:
//! - Session creation and validation
//! - Session storage and retrieval
//! - Session expiration and renewal
//! - Concurrent session management

use crate::{Error, Result, IdentityTrait as Identity};
use zeroize::Zeroize;
use base64::{engine::general_purpose::STANDARD, Engine as _};

use std::collections::BTreeMap;

/// Session manager for handling user sessions
pub trait SessionManager: Send + Sync {
    /// Create a new session
    fn create_session(&self, identity: &dyn Identity, metadata: SessionMetadata) -> Result<Session>;
    
    /// Get session by ID
    fn get_session(&self, session_id: &str) -> Result<Option<Session>>;
    
    /// Validate session
    fn validate_session(&self, session_id: &str) -> Result<bool>;
    
    /// Refresh session
    fn refresh_session(&self, session_id: &str) -> Result<Session>;
    
    /// Invalidate session
    fn invalidate_session(&self, session_id: &str) -> Result<()>;
    
    /// Get all active sessions for a user
    fn get_user_sessions(&self, user_id: &str) -> Result<Vec<Session>>;
    
    /// Invalidate all sessions for a user
    fn invalidate_user_sessions(&self, user_id: &str) -> Result<()>;
}

/// Session data
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session ID
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Session token
    pub token: String,
    /// Session metadata
    pub metadata: SessionMetadata,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last accessed timestamp
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    /// Expiration timestamp
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Is session active
    pub active: bool,
}

/// Session metadata
#[derive(Debug, Clone, Default)]
pub struct SessionMetadata {
    /// IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// Device ID
    pub device_id: Option<String>,
    /// Location
    pub location: Option<String>,
    /// Custom attributes
    pub attributes: BTreeMap<String, serde_json::Value>,
}

// Default implementation is automatically derived

/// In-memory session manager implementation
pub struct InMemorySessionManager {
    /// Sessions storage
    sessions: std::sync::RwLock<BTreeMap<String, Session>>,
    /// User to sessions mapping
    user_sessions: std::sync::RwLock<BTreeMap<String, Vec<String>>>,
    /// Session configuration
    config: SessionConfig,
}

/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Session lifetime in seconds
    pub session_lifetime: u64,
    /// Maximum concurrent sessions per user
    pub max_sessions_per_user: Option<usize>,
    /// Allow session refresh
    pub allow_refresh: bool,
    /// Refresh extends lifetime by this amount
    pub refresh_lifetime: u64,
    /// Require re-authentication after this idle time
    pub idle_timeout: Option<u64>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            session_lifetime: 3600 * 24, // 24 hours
            max_sessions_per_user: Some(5),
            allow_refresh: true,
            refresh_lifetime: 3600 * 24, // 24 hours
            idle_timeout: Some(3600), // 1 hour
        }
    }
}

impl InMemorySessionManager {
    /// Create new in-memory session manager
    pub fn new(config: SessionConfig) -> Self {
        Self {
            sessions: std::sync::RwLock::new(BTreeMap::new()),
            user_sessions: std::sync::RwLock::new(BTreeMap::new()),
            config,
        }
    }
    
    /// Generate session token
    fn generate_token() -> String {
        use rand_core::{RngCore, OsRng};
        let mut bytes = vec![0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        STANDARD.encode(&bytes)
    }
    
    /// Clean up expired sessions
    fn cleanup_expired(&self) -> Result<()> {
        let now = chrono::Utc::now();
        let mut sessions = self.sessions.write().unwrap();
        let mut user_sessions = self.user_sessions.write().unwrap();
        
        let expired: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| session.expires_at < now)
            .map(|(id, _)| id.clone())
            .collect();
        
        for session_id in expired {
            if let Some(session) = sessions.remove(&session_id) {
                // Remove from user sessions
                if let Some(user_session_ids) = user_sessions.get_mut(&session.user_id) {
                    user_session_ids.retain(|id| id != &session_id);
                }
            }
        }
        
        Ok(())
    }
}

impl SessionManager for InMemorySessionManager {
    fn create_session(&self, identity: &dyn Identity, metadata: SessionMetadata) -> Result<Session> {
        // Clean up expired sessions first
        self.cleanup_expired()?;
        
        let user_id = identity.id().to_string();
        
        // Check max sessions per user
        if let Some(max_sessions) = self.config.max_sessions_per_user {
            let user_sessions = self.user_sessions.read().unwrap();
            if let Some(session_ids) = user_sessions.get(&user_id) {
                if session_ids.len() >= max_sessions {
                    return Err(Error::SessionError(
                        format!("Maximum sessions ({}) reached for user", max_sessions)
                    ));
                }
            }
        }
        
        // Create new session
        let session = Session {
            id: format!("sess_{}", uuid::Uuid::new_v4()),
            user_id: user_id.clone(),
            token: Self::generate_token(),
            metadata,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(self.config.session_lifetime as i64),
            active: true,
        };
        
        // Store session
        {
            let mut sessions = self.sessions.write().unwrap();
            sessions.insert(session.id.clone(), session.clone());
        }
        
        // Update user sessions
        {
            let mut user_sessions = self.user_sessions.write().unwrap();
            user_sessions
                .entry(user_id)
                .or_insert_with(Vec::new)
                .push(session.id.clone());
        }
        
        Ok(session)
    }
    
    fn get_session(&self, session_id: &str) -> Result<Option<Session>> {
        let sessions = self.sessions.read().unwrap();
        Ok(sessions.get(session_id).cloned())
    }
    
    fn validate_session(&self, session_id: &str) -> Result<bool> {
        let now = chrono::Utc::now();
        let mut sessions = self.sessions.write().unwrap();
        
        if let Some(session) = sessions.get_mut(session_id) {
            // Check if expired
            if session.expires_at < now {
                session.active = false;
                return Ok(false);
            }
            
            // Check idle timeout
            if let Some(idle_timeout) = self.config.idle_timeout {
                let idle_duration = now - session.last_accessed;
                if idle_duration.num_seconds() as u64 > idle_timeout {
                    session.active = false;
                    return Ok(false);
                }
            }
            
            // Update last accessed
            session.last_accessed = now;
            
            Ok(session.active)
        } else {
            Ok(false)
        }
    }
    
    fn refresh_session(&self, session_id: &str) -> Result<Session> {
        if !self.config.allow_refresh {
            return Err(Error::SessionError("Session refresh not allowed".into()));
        }
        
        let mut sessions = self.sessions.write().unwrap();
        
        if let Some(session) = sessions.get_mut(session_id) {
            if !session.active {
                return Err(Error::SessionError("Cannot refresh inactive session".into()));
            }
            
            // Extend expiration
            session.expires_at = chrono::Utc::now() + 
                chrono::Duration::seconds(self.config.refresh_lifetime as i64);
            session.last_accessed = chrono::Utc::now();
            
            // Generate new token
            session.token = Self::generate_token();
            
            Ok(session.clone())
        } else {
            Err(Error::NotFound("Session not found".into()))
        }
    }
    
    fn invalidate_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let mut user_sessions = self.user_sessions.write().unwrap();
        
        if let Some(mut session) = sessions.remove(session_id) {
            // Clear sensitive data
            session.token.zeroize();
            
            // Remove from user sessions
            if let Some(user_session_ids) = user_sessions.get_mut(&session.user_id) {
                user_session_ids.retain(|id| id != session_id);
            }
            
            Ok(())
        } else {
            Err(Error::NotFound("Session not found".into()))
        }
    }
    
    fn get_user_sessions(&self, user_id: &str) -> Result<Vec<Session>> {
        let sessions = self.sessions.read().unwrap();
        let user_sessions = self.user_sessions.read().unwrap();
        
        if let Some(session_ids) = user_sessions.get(user_id) {
            let user_sessions: Vec<Session> = session_ids
                .iter()
                .filter_map(|id| sessions.get(id).cloned())
                .collect();
            
            Ok(user_sessions)
        } else {
            Ok(Vec::new())
        }
    }
    
    fn invalidate_user_sessions(&self, user_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let mut user_sessions = self.user_sessions.write().unwrap();
        
        if let Some(session_ids) = user_sessions.remove(user_id) {
            for session_id in session_ids {
                if let Some(mut session) = sessions.remove(&session_id) {
                    session.token.zeroize();
                }
            }
        }
        
        Ok(())
    }
}

/// Secure session token that zeros memory on drop
pub struct SecureToken(Vec<u8>);

impl SecureToken {
    /// Create new secure token
    pub fn new(token: Vec<u8>) -> Self {
        Self(token)
    }
    
    /// Get token bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for SecureToken {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

// Add uuid dependency temporarily
mod uuid {
    pub struct Uuid;
    impl Uuid {
        pub fn new_v4() -> String {
            use rand_core::{RngCore, OsRng};
            let mut bytes = vec![0u8; 16];
            OsRng.fill_bytes(&mut bytes);
            format!("{:x}", bytes.iter().fold(0u128, |acc, &b| (acc << 8) | b as u128))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct MockIdentity {
        id: String,
    }
    
    impl Identity for MockIdentity {
        fn id(&self) -> &str {
            &self.id
        }
        
        fn public_key(&self) -> &[u8] {
            b"mock_public_key"
        }
        
        fn sign(&self, _data: &[u8]) -> Result<Vec<u8>> {
            Ok(vec![0; 64])
        }
        
        fn verify(&self, _data: &[u8], _signature: &[u8]) -> Result<bool> {
            Ok(true)
        }
    }
    
    #[test]
    fn test_session_creation() {
        let manager = InMemorySessionManager::new(SessionConfig::default());
        let identity = MockIdentity {
            id: "user123".to_string(),
        };
        
        let metadata = SessionMetadata {
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            ..Default::default()
        };
        
        let session = manager.create_session(&identity, metadata).unwrap();
        
        assert_eq!(session.user_id, "user123");
        assert!(session.active);
        assert!(session.id.starts_with("sess_"));
    }
    
    #[test]
    fn test_session_validation() {
        let manager = InMemorySessionManager::new(SessionConfig::default());
        let identity = MockIdentity {
            id: "user123".to_string(),
        };
        
        let session = manager.create_session(&identity, SessionMetadata::default()).unwrap();
        
        // Validate existing session
        assert!(manager.validate_session(&session.id).unwrap());
        
        // Validate non-existent session
        assert!(!manager.validate_session("invalid_id").unwrap());
    }
    
    #[test]
    fn test_max_sessions_per_user() {
        let mut config = SessionConfig::default();
        config.max_sessions_per_user = Some(2);
        
        let manager = InMemorySessionManager::new(config);
        let identity = MockIdentity {
            id: "user123".to_string(),
        };
        
        // Create max sessions
        manager.create_session(&identity, SessionMetadata::default()).unwrap();
        manager.create_session(&identity, SessionMetadata::default()).unwrap();
        
        // Third session should fail
        let result = manager.create_session(&identity, SessionMetadata::default());
        assert!(result.is_err());
    }
}