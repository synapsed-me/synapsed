//! Secure session management with key rotation.

use crate::crypto::key_derivation::{derive_session_keys, KeyDerivationFunction, KeyRatchet, SessionKeys};
use crate::error::{NetworkError, Result, SecurityError};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Session state information.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionState {
    /// Unique session identifier
    pub id: Uuid,
    
    /// Peer identifier
    pub peer_id: String,
    
    /// Session creation time
    pub created_at: SystemTime,
    
    /// Last activity time
    pub last_activity: SystemTime,
    
    /// Session expiry time
    pub expires_at: SystemTime,
    
    /// Key rotation counter
    pub rotation_count: u64,
    
    /// Whether the session is authenticated
    pub authenticated: bool,
}

/// Session manager for handling multiple secure sessions.
pub struct SessionManager {
    /// Active sessions
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    
    /// Default session lifetime
    default_lifetime: Duration,
    
    /// Key rotation interval
    rotation_interval: Duration,
    
    /// Maximum session idle time
    max_idle_time: Duration,
}

impl SessionManager {
    /// Creates a new session manager.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            default_lifetime: Duration::from_secs(3600), // 1 hour
            rotation_interval: Duration::from_secs(300), // 5 minutes
            max_idle_time: Duration::from_secs(900), // 15 minutes
        }
    }
    
    /// Creates a new session manager with custom configuration.
    /// Useful for testing with shorter timeouts.
    #[cfg(test)]
    pub fn new_with_config(
        default_lifetime: Duration,
        rotation_interval: Duration,
        max_idle_time: Duration,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            default_lifetime,
            rotation_interval,
            max_idle_time,
        }
    }
    
    /// Creates a new session with the given peer.
    pub fn create_session(
        &self,
        peer_id: String,
        master_secret: Vec<u8>,
        kdf: KeyDerivationFunction,
    ) -> Result<Uuid> {
        let session_id = Uuid::new_v4();
        let now = SystemTime::now();
        
        // Derive initial session keys
        let salt = session_id.as_bytes();
        let info = format!("session-{}", peer_id).into_bytes();
        let keys = derive_session_keys(
            kdf,
            &master_secret,
            Some(salt),
            &info,
            32, // key size
            16, // IV size
        )?;
        
        // Create key ratchet for forward secrecy
        let ratchet = KeyRatchet::new(master_secret, kdf);
        
        let session = Session {
            state: SessionState {
                id: session_id,
                peer_id,
                created_at: now,
                last_activity: now,
                expires_at: now + self.default_lifetime,
                rotation_count: 0,
                authenticated: false,
            },
            keys,
            ratchet: Arc::new(RwLock::new(ratchet)),
            last_rotation: now,
        };
        
        self.sessions.write()
            .map_err(|_| NetworkError::Security(SecurityError::Encryption(
                "Failed to acquire sessions write lock".to_string()
            )))?
            .insert(session_id, session);
        
        Ok(session_id)
    }
    
    /// Retrieves a session by ID.
    pub fn get_session(&self, session_id: &Uuid) -> Result<Arc<Session>> {
        let sessions = self.sessions.read()
            .map_err(|_| NetworkError::Security(SecurityError::Encryption(
                "Failed to acquire sessions read lock".to_string()
            )))?;
        let session = sessions.get(session_id)
            .ok_or_else(|| NetworkError::Security(SecurityError::SessionExpired(
                "Session not found".to_string()
            )))?;
        
        // Check if session is expired
        let now = SystemTime::now();
        if now > session.state.expires_at {
            return Err(NetworkError::Security(SecurityError::SessionExpired(
                "Session has expired".to_string()
            )));
        }
        
        // Check idle timeout
        if now.duration_since(session.state.last_activity)
            .map_err(|_| NetworkError::Security(SecurityError::Encryption(
                "Invalid session activity timestamp".to_string()
            )))? > self.max_idle_time {
            return Err(NetworkError::Security(SecurityError::SessionExpired(
                "Session idle timeout".to_string()
            )));
        }
        
        Ok(Arc::new(session.clone()))
    }
    
    /// Updates session activity timestamp.
    pub fn touch_session(&self, session_id: &Uuid) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| NetworkError::Security(SecurityError::SessionExpired(
                "Session not found".to_string()
            )))?;
        
        session.state.last_activity = SystemTime::now();
        
        // Check if key rotation is needed
        let time_since_rotation = SystemTime::now()
            .duration_since(session.last_rotation)
            .unwrap();
        
        if time_since_rotation > self.rotation_interval {
            self.rotate_session_keys(session)?;
        }
        
        Ok(())
    }
    
    /// Marks a session as authenticated.
    pub fn authenticate_session(&self, session_id: &Uuid) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| NetworkError::Security(SecurityError::SessionExpired(
                "Session not found".to_string()
            )))?;
        
        session.state.authenticated = true;
        Ok(())
    }
    
    /// Rotates session keys for forward secrecy.
    fn rotate_session_keys(&self, session: &mut Session) -> Result<()> {
        let mut ratchet = session.ratchet.write().unwrap();
        let new_master = ratchet.advance()?;
        
        // Derive new session keys
        let salt = format!("rotation-{}", session.state.rotation_count).into_bytes();
        let info = format!("session-{}-rotated", session.state.peer_id).into_bytes();
        
        let new_keys = derive_session_keys(
            KeyDerivationFunction::HkdfSha256,
            &new_master,
            Some(&salt),
            &info,
            32,
            16,
        )?;
        
        // Update session
        session.keys = new_keys;
        session.state.rotation_count += 1;
        session.last_rotation = SystemTime::now();
        
        Ok(())
    }
    
    /// Removes a session.
    pub fn remove_session(&self, session_id: &Uuid) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        sessions.remove(session_id)
            .ok_or_else(|| NetworkError::Security(SecurityError::SessionExpired(
                "Session not found".to_string()
            )))?;
        Ok(())
    }
    
    /// Cleans up expired sessions.
    pub fn cleanup_expired(&self) {
        let mut sessions = self.sessions.write().unwrap();
        let now = SystemTime::now();
        
        sessions.retain(|_, session| {
            now <= session.state.expires_at &&
            now.duration_since(session.state.last_activity).unwrap() <= self.max_idle_time
        });
    }
    
    /// Returns the number of active sessions.
    pub fn active_session_count(&self) -> usize {
        self.sessions.read().unwrap().len()
    }
    
    /// Lists all active session IDs.
    pub fn list_sessions(&self) -> Vec<Uuid> {
        self.sessions.read().unwrap().keys().copied().collect()
    }
}

/// Individual session with its keys and state.
#[derive(Clone)]
pub struct Session {
    /// Session state
    pub state: SessionState,
    
    /// Current session keys
    pub keys: SessionKeys,
    
    /// Key ratchet for rotation
    pub ratchet: Arc<RwLock<KeyRatchet>>,
    
    /// Last key rotation time
    pub last_rotation: SystemTime,
}

/// Session ticket for resumption.
#[derive(Debug, Clone)]
pub struct SessionTicket {
    /// Ticket identifier
    pub id: Vec<u8>,
    
    /// Encrypted session state
    pub encrypted_state: Vec<u8>,
    
    /// Ticket creation time
    pub created_at: SystemTime,
    
    /// Ticket lifetime
    pub lifetime: Duration,
}

impl SessionTicket {
    /// Creates a new session ticket.
    pub fn new(session: &SessionState, encryption_key: &[u8]) -> Result<Self> {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        
        // Generate ticket ID
        let mut id = vec![0u8; 32];
        rng.fill_bytes(&mut id);
        
        // Serialize session state
        let state_bytes = serde_json::to_vec(session)
            .map_err(|e| NetworkError::Security(SecurityError::Serialization(e.to_string())))?;
        
        // Encrypt using ChaCha20Poly1305 AEAD
        use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, KeyInit};
        
        // Derive encryption key if needed (ensure it's 32 bytes)
        let key: Vec<u8> = if encryption_key.len() == 32 {
            encryption_key.to_vec()
        } else {
            // Use HKDF to derive proper key
            let salt = b"session-ticket-salt";
            let info = b"session-ticket-encryption";
            let mut okm = [0u8; 32];
            hkdf::Hkdf::<sha2::Sha256>::new(Some(salt), encryption_key)
                .expand(info, &mut okm)
                .map_err(|_| NetworkError::Security(SecurityError::KeyDerivation(
                    "Failed to derive ticket encryption key".to_string()
                )))?;
            okm.to_vec()
        };
        
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|_| NetworkError::Security(SecurityError::Encryption(
                "Invalid key length".to_string()
            )))?;
        
        // Generate nonce (first 12 bytes of ticket ID)
        let nonce = &id[..12];
        
        let encrypted_state = cipher.encrypt(nonce.into(), state_bytes.as_ref())
            .map_err(|_| NetworkError::Security(SecurityError::Encryption(
                "Failed to encrypt session ticket".to_string()
            )))?;
        
        Ok(Self {
            id,
            encrypted_state,
            created_at: SystemTime::now(),
            lifetime: Duration::from_secs(86400), // 24 hours
        })
    }
    
    /// Decrypts and validates a session ticket.
    pub fn decrypt(&self, encryption_key: &[u8]) -> Result<SessionState> {
        // Check if ticket is still valid
        if SystemTime::now() > self.created_at + self.lifetime {
            return Err(NetworkError::Security(SecurityError::SessionExpired(
                "Session ticket expired".to_string()
            )));
        }
        
        // Decrypt using ChaCha20Poly1305 AEAD
        use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, KeyInit};
        
        // Derive decryption key if needed (ensure it's 32 bytes)
        let key: Vec<u8> = if encryption_key.len() == 32 {
            encryption_key.to_vec()
        } else {
            // Use HKDF to derive proper key
            let salt = b"session-ticket-salt";
            let info = b"session-ticket-encryption";
            let mut okm = [0u8; 32];
            hkdf::Hkdf::<sha2::Sha256>::new(Some(salt), encryption_key)
                .expand(info, &mut okm)
                .map_err(|_| NetworkError::Security(SecurityError::KeyDerivation(
                    "Failed to derive ticket decryption key".to_string()
                )))?;
            okm.to_vec()
        };
        
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|_| NetworkError::Security(SecurityError::Decryption(
                "Invalid key length".to_string()
            )))?;
        
        // Use first 12 bytes of ticket ID as nonce
        let nonce = &self.id[..12];
        
        let decrypted = cipher.decrypt(nonce.into(), self.encrypted_state.as_ref())
            .map_err(|_| NetworkError::Security(SecurityError::Decryption(
                "Failed to decrypt session ticket - authentication failed".to_string()
            )))?;
        
        // Deserialize
        serde_json::from_slice(&decrypted)
            .map_err(|e| NetworkError::Security(SecurityError::Deserialization(e.to_string())))
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let manager = SessionManager::new();
        let master_secret = vec![0u8; 32];
        
        let session_id = manager.create_session(
            "peer123".to_string(),
            master_secret,
            KeyDerivationFunction::HkdfSha256,
        ).unwrap();
        
        // Verify session exists
        let session = manager.get_session(&session_id).unwrap();
        assert_eq!(session.state.peer_id, "peer123");
        assert!(!session.state.authenticated);
    }
    
    #[test]
    fn test_session_authentication() {
        let manager = SessionManager::new();
        let master_secret = vec![0u8; 32];
        
        let session_id = manager.create_session(
            "peer123".to_string(),
            master_secret,
            KeyDerivationFunction::HkdfSha256,
        ).unwrap();
        
        // Authenticate session
        manager.authenticate_session(&session_id).unwrap();
        
        // Verify authentication
        let session = manager.get_session(&session_id).unwrap();
        assert!(session.state.authenticated);
    }
    
    #[test]
    fn test_session_cleanup() {
        let manager = SessionManager::new_with_config(
            Duration::from_secs(3600),
            Duration::from_secs(300),
            Duration::from_millis(100), // Short idle timeout for testing
        );
        
        let master_secret = vec![0u8; 32];
        let session_id = manager.create_session(
            "peer123".to_string(),
            master_secret,
            KeyDerivationFunction::HkdfSha256,
        ).unwrap();
        
        // Initial count
        assert_eq!(manager.active_session_count(), 1);
        
        // Wait for idle timeout
        std::thread::sleep(Duration::from_millis(200));
        
        // Session should be expired due to idle timeout
        assert!(manager.get_session(&session_id).is_err());
    }
    
    #[test]
    fn test_session_ticket() {
        let state = SessionState {
            id: Uuid::new_v4(),
            peer_id: "peer123".to_string(),
            created_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(3600),
            rotation_count: 0,
            authenticated: true,
        };
        
        let encryption_key = b"ticket-encryption-key";
        
        // Create ticket
        let ticket = SessionTicket::new(&state, encryption_key).unwrap();
        
        // Decrypt ticket
        let decrypted_state = ticket.decrypt(encryption_key).unwrap();
        assert_eq!(decrypted_state.peer_id, state.peer_id);
        assert_eq!(decrypted_state.authenticated, state.authenticated);
    }
}