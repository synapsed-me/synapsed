//! In-memory storage backend implementation

use super::{
    IdentityStorageBackend, StorageConfig,
    UserStore, CredentialStore, SessionStore, IdentityStore,
    User, StoredCredential, StoredSession
};
use crate::{Error, Result, IdentityTrait};

use std::collections::BTreeMap;

/// In-memory storage backend
pub struct InMemoryStorageBackend {
    users: InMemoryUserStore,
    credentials: InMemoryCredentialStore,
    sessions: InMemorySessionStore,
    identities: InMemoryIdentityStore,
}

impl InMemoryStorageBackend {
    /// Create new in-memory storage backend
    pub fn new(_config: StorageConfig) -> Self {
        Self {
            users: InMemoryUserStore::new(),
            credentials: InMemoryCredentialStore::new(),
            sessions: InMemorySessionStore::new(),
            identities: InMemoryIdentityStore::new(),
        }
    }
}

impl IdentityStorageBackend for InMemoryStorageBackend {
    fn user_store(&self) -> &dyn UserStore {
        &self.users
    }
    
    fn credential_store(&self) -> &dyn CredentialStore {
        &self.credentials
    }
    
    fn session_store(&self) -> &dyn SessionStore {
        &self.sessions
    }
    
    fn identity_store(&self) -> &dyn IdentityStore {
        &self.identities
    }
}

/// In-memory user store
struct InMemoryUserStore {
    users: std::sync::RwLock<BTreeMap<String, User>>,
    username_index: std::sync::RwLock<BTreeMap<String, String>>,
    email_index: std::sync::RwLock<BTreeMap<String, String>>,
}

impl InMemoryUserStore {
    fn new() -> Self {
        Self {
            users: std::sync::RwLock::new(BTreeMap::new()),
            username_index: std::sync::RwLock::new(BTreeMap::new()),
            email_index: std::sync::RwLock::new(BTreeMap::new()),
        }
    }
}

impl UserStore for InMemoryUserStore {
    fn create_user(&self, user: &User) -> Result<()> {
        let mut users = self.users.write().unwrap();
        let mut username_index = self.username_index.write().unwrap();
        let mut email_index = self.email_index.write().unwrap();
        
        // Check if user already exists
        if users.contains_key(&user.id) {
            return Err(Error::AlreadyExists(format!("User {} already exists", user.id)));
        }
        
        // Check username uniqueness
        if username_index.contains_key(&user.username) {
            return Err(Error::AlreadyExists(format!("Username {} already taken", user.username)));
        }
        
        // Check email uniqueness
        if let Some(email) = &user.email {
            if email_index.contains_key(email) {
                return Err(Error::AlreadyExists(format!("Email {} already registered", email)));
            }
        }
        
        // Store user
        users.insert(user.id.clone(), user.clone());
        username_index.insert(user.username.clone(), user.id.clone());
        
        if let Some(email) = &user.email {
            email_index.insert(email.clone(), user.id.clone());
        }
        
        Ok(())
    }
    
    fn get_user(&self, user_id: &str) -> Result<Option<User>> {
        let users = self.users.read().unwrap();
        Ok(users.get(user_id).cloned())
    }
    
    fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let username_index = self.username_index.read().unwrap();
        let users = self.users.read().unwrap();
        
        if let Some(user_id) = username_index.get(username) {
            Ok(users.get(user_id).cloned())
        } else {
            Ok(None)
        }
    }
    
    fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let email_index = self.email_index.read().unwrap();
        let users = self.users.read().unwrap();
        
        if let Some(user_id) = email_index.get(email) {
            Ok(users.get(user_id).cloned())
        } else {
            Ok(None)
        }
    }
    
    fn update_user(&self, user: &User) -> Result<()> {
        let mut users = self.users.write().unwrap();
        let mut username_index = self.username_index.write().unwrap();
        let mut email_index = self.email_index.write().unwrap();
        
        // Get existing user
        let existing = users.get(&user.id).ok_or_else(|| {
            Error::NotFound(format!("User {} not found", user.id))
        })?;
        
        // Update indices if username or email changed
        if existing.username != user.username {
            username_index.remove(&existing.username);
            username_index.insert(user.username.clone(), user.id.clone());
        }
        
        if existing.email != user.email {
            if let Some(old_email) = &existing.email {
                email_index.remove(old_email);
            }
            if let Some(new_email) = &user.email {
                email_index.insert(new_email.clone(), user.id.clone());
            }
        }
        
        // Update user
        users.insert(user.id.clone(), user.clone());
        Ok(())
    }
    
    fn delete_user(&self, user_id: &str) -> Result<()> {
        let mut users = self.users.write().unwrap();
        let mut username_index = self.username_index.write().unwrap();
        let mut email_index = self.email_index.write().unwrap();
        
        if let Some(user) = users.remove(user_id) {
            username_index.remove(&user.username);
            if let Some(email) = &user.email {
                email_index.remove(email);
            }
            Ok(())
        } else {
            Err(Error::NotFound(format!("User {} not found", user_id)))
        }
    }
    
    fn list_users(&self, offset: usize, limit: usize) -> Result<Vec<User>> {
        let users = self.users.read().unwrap();
        Ok(users.values()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect())
    }
    
    fn search_users(&self, query: &str) -> Result<Vec<User>> {
        let users = self.users.read().unwrap();
        let query_lower = query.to_lowercase();
        
        Ok(users.values()
            .filter(|user| {
                user.username.to_lowercase().contains(&query_lower) ||
                user.display_name.as_ref()
                    .map(|n| n.to_lowercase().contains(&query_lower))
                    .unwrap_or(false) ||
                user.email.as_ref()
                    .map(|e| e.to_lowercase().contains(&query_lower))
                    .unwrap_or(false)
            })
            .cloned()
            .collect())
    }
}

/// In-memory credential store
struct InMemoryCredentialStore {
    credentials: std::sync::RwLock<BTreeMap<String, StoredCredential>>,
    user_credentials: std::sync::RwLock<BTreeMap<String, Vec<String>>>,
}

impl InMemoryCredentialStore {
    fn new() -> Self {
        Self {
            credentials: std::sync::RwLock::new(BTreeMap::new()),
            user_credentials: std::sync::RwLock::new(BTreeMap::new()),
        }
    }
}

impl CredentialStore for InMemoryCredentialStore {
    fn store_credential(&self, credential: &StoredCredential) -> Result<()> {
        let mut credentials = self.credentials.write().unwrap();
        let mut user_credentials = self.user_credentials.write().unwrap();
        
        credentials.insert(credential.id.clone(), credential.clone());
        
        user_credentials
            .entry(credential.user_id.clone())
            .or_insert_with(Vec::new)
            .push(credential.id.clone());
        
        Ok(())
    }
    
    fn get_credential(&self, credential_id: &str) -> Result<Option<StoredCredential>> {
        let credentials = self.credentials.read().unwrap();
        Ok(credentials.get(credential_id).cloned())
    }
    
    fn get_user_credentials(&self, user_id: &str) -> Result<Vec<StoredCredential>> {
        let credentials = self.credentials.read().unwrap();
        let user_credentials = self.user_credentials.read().unwrap();
        
        if let Some(cred_ids) = user_credentials.get(user_id) {
            Ok(cred_ids.iter()
                .filter_map(|id| credentials.get(id).cloned())
                .collect())
        } else {
            Ok(Vec::new())
        }
    }
    
    fn update_credential(&self, credential: &StoredCredential) -> Result<()> {
        let mut credentials = self.credentials.write().unwrap();
        
        if credentials.contains_key(&credential.id) {
            credentials.insert(credential.id.clone(), credential.clone());
            Ok(())
        } else {
            Err(Error::NotFound(format!("Credential {} not found", credential.id)))
        }
    }
    
    fn revoke_credential(&self, credential_id: &str, reason: Option<&str>) -> Result<()> {
        let mut credentials = self.credentials.write().unwrap();
        
        if let Some(credential) = credentials.get_mut(credential_id) {
            credential.revoked = true;
            credential.revocation_reason = reason.map(|r| r.to_string());
            Ok(())
        } else {
            Err(Error::NotFound(format!("Credential {} not found", credential_id)))
        }
    }
    
    fn delete_credential(&self, credential_id: &str) -> Result<()> {
        let mut credentials = self.credentials.write().unwrap();
        let mut user_credentials = self.user_credentials.write().unwrap();
        
        if let Some(credential) = credentials.remove(credential_id) {
            if let Some(cred_ids) = user_credentials.get_mut(&credential.user_id) {
                cred_ids.retain(|id| id != credential_id);
            }
            Ok(())
        } else {
            Err(Error::NotFound(format!("Credential {} not found", credential_id)))
        }
    }
    
    fn cleanup_expired(&self) -> Result<usize> {
        let now = chrono::Utc::now();
        let mut credentials = self.credentials.write().unwrap();
        let mut count = 0;
        
        let expired: Vec<String> = credentials
            .iter()
            .filter(|(_, cred)| {
                cred.expires_at.map(|exp| exp < now).unwrap_or(false)
            })
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in expired {
            credentials.remove(&id);
            count += 1;
        }
        
        Ok(count)
    }
}

/// In-memory session store
struct InMemorySessionStore {
    sessions: std::sync::RwLock<BTreeMap<String, StoredSession>>,
    user_sessions: std::sync::RwLock<BTreeMap<String, Vec<String>>>,
}

impl InMemorySessionStore {
    fn new() -> Self {
        Self {
            sessions: std::sync::RwLock::new(BTreeMap::new()),
            user_sessions: std::sync::RwLock::new(BTreeMap::new()),
        }
    }
}

impl SessionStore for InMemorySessionStore {
    fn store_session(&self, session: &StoredSession) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let mut user_sessions = self.user_sessions.write().unwrap();
        
        sessions.insert(session.id.clone(), session.clone());
        
        user_sessions
            .entry(session.user_id.clone())
            .or_insert_with(Vec::new)
            .push(session.id.clone());
        
        Ok(())
    }
    
    fn get_session(&self, session_id: &str) -> Result<Option<StoredSession>> {
        let sessions = self.sessions.read().unwrap();
        Ok(sessions.get(session_id).cloned())
    }
    
    fn get_user_sessions(&self, user_id: &str) -> Result<Vec<StoredSession>> {
        let sessions = self.sessions.read().unwrap();
        let user_sessions = self.user_sessions.read().unwrap();
        
        if let Some(session_ids) = user_sessions.get(user_id) {
            Ok(session_ids.iter()
                .filter_map(|id| sessions.get(id).cloned())
                .collect())
        } else {
            Ok(Vec::new())
        }
    }
    
    fn update_session(&self, session: &StoredSession) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        
        if sessions.contains_key(&session.id) {
            sessions.insert(session.id.clone(), session.clone());
            Ok(())
        } else {
            Err(Error::NotFound(format!("Session {} not found", session.id)))
        }
    }
    
    fn delete_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let mut user_sessions = self.user_sessions.write().unwrap();
        
        if let Some(session) = sessions.remove(session_id) {
            if let Some(session_ids) = user_sessions.get_mut(&session.user_id) {
                session_ids.retain(|id| id != session_id);
            }
            Ok(())
        } else {
            Err(Error::NotFound(format!("Session {} not found", session_id)))
        }
    }
    
    fn delete_user_sessions(&self, user_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let mut user_sessions = self.user_sessions.write().unwrap();
        
        if let Some(session_ids) = user_sessions.remove(user_id) {
            for session_id in session_ids {
                sessions.remove(&session_id);
            }
        }
        
        Ok(())
    }
    
    fn cleanup_expired(&self) -> Result<usize> {
        let now = chrono::Utc::now();
        let mut sessions = self.sessions.write().unwrap();
        let mut count = 0;
        
        let expired: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| session.expires_at < now)
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in expired {
            sessions.remove(&id);
            count += 1;
        }
        
        Ok(count)
    }
}

/// In-memory identity store
struct InMemoryIdentityStore {
    identities: std::sync::RwLock<BTreeMap<String, Vec<u8>>>,
}

impl InMemoryIdentityStore {
    fn new() -> Self {
        Self {
            identities: std::sync::RwLock::new(BTreeMap::new()),
        }
    }
}

impl IdentityStore for InMemoryIdentityStore {
    fn store_identity(&self, identity: &dyn IdentityTrait) -> Result<()> {
        // For now, just store the ID and public key
        let mut identities = self.identities.write().unwrap();
        identities.insert(identity.id().to_string(), identity.public_key().to_vec());
        Ok(())
    }
    
    fn get_identity(&self, _identity_id: &str) -> Result<Option<Box<dyn IdentityTrait>>> {
        // This would need a proper implementation with serialization
        Ok(None)
    }
    
    fn update_identity(&self, identity: &dyn IdentityTrait) -> Result<()> {
        self.store_identity(identity)
    }
    
    fn delete_identity(&self, identity_id: &str) -> Result<()> {
        let mut identities = self.identities.write().unwrap();
        identities.remove(identity_id);
        Ok(())
    }
    
    fn list_identity_ids(&self) -> Result<Vec<String>> {
        let identities = self.identities.read().unwrap();
        Ok(identities.keys().cloned().collect())
    }
}