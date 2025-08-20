//! Test fixtures and utilities for synapsed-identity tests
//! 
//! Provides common test data, mocks, and helper functions.

#![cfg(test)]

use synapsed_identity::{
    auth::{AuthenticationService, JwtService},
    authorization::{AuthorizationService, Role},
    storage::{UserStore, UserData},
    session::SessionManager,
    Identity, Error, Result,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Test fixture for creating pre-configured services
pub struct TestFixture {
    pub user_store: Arc<RwLock<UserStore>>,
    pub auth_service: Arc<RwLock<AuthenticationService>>,
    pub authz_service: Arc<RwLock<AuthorizationService>>,
    pub jwt_service: Arc<JwtService>,
    pub session_manager: Arc<RwLock<SessionManager>>,
}

impl TestFixture {
    /// Create a new test fixture with all services initialized
    pub async fn new() -> Self {
        Self {
            user_store: Arc::new(RwLock::new(UserStore::new())),
            auth_service: Arc::new(RwLock::new(AuthenticationService::new())),
            authz_service: Arc::new(RwLock::new(AuthorizationService::new())),
            jwt_service: Arc::new(JwtService::new()),
            session_manager: Arc::new(RwLock::new(SessionManager::new())),
        }
    }

    /// Create a test fixture with sample data
    pub async fn with_sample_data() -> Self {
        let fixture = Self::new().await;
        
        // Create sample users
        let users = vec![
            ("admin", "admin@example.com", "Admin User", "AdminP@ss123!"),
            ("user1", "user1@example.com", "User One", "User1P@ss123!"),
            ("user2", "user2@example.com", "User Two", "User2P@ss123!"),
        ];
        
        for (username, email, full_name, password) in users {
            let user_data = UserData {
                id: None,
                username: username.to_string(),
                email: email.to_string(),
                full_name: full_name.to_string(),
                metadata: HashMap::new(),
            };
            
            let user = fixture.user_store.write().await
                .create_user(&user_data).await.unwrap();
            
            fixture.auth_service.write().await
                .register(&user.id, password).await.unwrap();
        }
        
        // Create sample roles
        let admin_role = Role {
            id: "admin".to_string(),
            name: "Administrator".to_string(),
            description: "Full system access".to_string(),
            permissions: vec!["*".to_string()],
        };
        
        let user_role = Role {
            id: "user".to_string(),
            name: "User".to_string(),
            description: "Standard user access".to_string(),
            permissions: vec![
                "profile:read".to_string(),
                "profile:write".to_string(),
            ],
        };
        
        fixture.authz_service.write().await
            .create_role(&admin_role).await.unwrap();
        fixture.authz_service.write().await
            .create_role(&user_role).await.unwrap();
        
        // Assign roles
        let admin_user = fixture.user_store.read().await
            .get_user_by_username("admin").await.unwrap();
        fixture.authz_service.write().await
            .assign_role(&admin_user.id, "admin").await.unwrap();
        
        let user1 = fixture.user_store.read().await
            .get_user_by_username("user1").await.unwrap();
        fixture.authz_service.write().await
            .assign_role(&user1.id, "user").await.unwrap();
        
        fixture
    }

    /// Create an authenticated session for a user
    pub async fn create_authenticated_session(&self, username: &str, password: &str) -> Result<String> {
        let auth_token = self.auth_service.read().await
            .authenticate(username, password).await?;
        
        let user = self.user_store.read().await
            .get_user_by_username(username).await?;
        
        let jwt_claims = JwtClaims {
            sub: user.id.clone(),
            ..Default::default()
        };
        
        let jwt_token = self.jwt_service.create_token(&jwt_claims)?;
        
        let session = self.session_manager.write().await
            .create_session(&user.id, &jwt_token).await?;
        
        Ok(session.id().to_string())
    }
}

/// Mock implementations for testing
pub mod mocks {
    use super::*;
    use async_trait::async_trait;
    
    /// Mock identity for testing
    pub struct MockIdentity {
        id: String,
        public_key: Vec<u8>,
        private_key: Vec<u8>,
    }
    
    impl MockIdentity {
        pub fn new(id: &str) -> Self {
            Self {
                id: id.to_string(),
                public_key: vec![0u8; 32], // Mock public key
                private_key: vec![1u8; 32], // Mock private key
            }
        }
    }
    
    #[async_trait]
    impl Identity for MockIdentity {
        fn id(&self) -> &str {
            &self.id
        }
        
        fn public_key(&self) -> &[u8] {
            &self.public_key
        }
        
        async fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
            // Mock signature
            let mut signature = vec![2u8; 64];
            signature.extend_from_slice(&data[..data.len().min(32)]);
            Ok(signature)
        }
        
        async fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool> {
            // Mock verification
            Ok(signature.len() >= 64 && signature[0] == 2)
        }
    }
    
    /// Mock storage for testing
    pub struct MockStorage {
        users: Arc<RwLock<HashMap<String, UserData>>>,
        identities: Arc<RwLock<HashMap<String, MockIdentity>>>,
    }
    
    impl MockStorage {
        pub fn new() -> Self {
            Self {
                users: Arc::new(RwLock::new(HashMap::new())),
                identities: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }
}

/// Test data generators
pub mod generators {
    use super::*;
    use rand::{thread_rng, Rng, distributions::Alphanumeric};
    
    /// Generate a random username
    pub fn random_username() -> String {
        let suffix: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        format!("user_{}", suffix)
    }
    
    /// Generate a random email
    pub fn random_email() -> String {
        let local: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();
        format!("{}@test.example.com", local.to_lowercase())
    }
    
    /// Generate a secure password
    pub fn random_password() -> String {
        let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";
        let mut rng = thread_rng();
        (0..16)
            .map(|_| {
                let idx = rng.gen_range(0..chars.len());
                chars.chars().nth(idx).unwrap()
            })
            .collect()
    }
    
    /// Generate random user data
    pub fn random_user_data() -> UserData {
        UserData {
            id: None,
            username: random_username(),
            email: random_email(),
            full_name: format!("Test User {}", thread_rng().gen::<u16>()),
            metadata: HashMap::new(),
        }
    }
    
    /// Generate multiple users
    pub fn generate_users(count: usize) -> Vec<UserData> {
        (0..count).map(|_| random_user_data()).collect()
    }
    
    /// Generate a role with random permissions
    pub fn random_role() -> Role {
        let id: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        
        let permissions = vec![
            format!("resource:{}:read", id),
            format!("resource:{}:write", id),
        ];
        
        Role {
            id: format!("role_{}", id),
            name: format!("Role {}", id),
            description: format!("Test role {}", id),
            permissions,
        }
    }
}

/// Assertion helpers
pub mod assertions {
    use super::*;
    
    /// Assert that a user exists and has expected properties
    pub async fn assert_user_exists(
        user_store: &UserStore,
        username: &str,
        email: &str,
    ) -> Result<()> {
        let user = user_store.get_user_by_username(username).await?;
        assert_eq!(user.email, email);
        Ok(())
    }
    
    /// Assert that a user has a specific role
    pub async fn assert_user_has_role(
        authz_service: &AuthorizationService,
        user_id: &str,
        role_id: &str,
    ) -> Result<()> {
        let roles = authz_service.get_user_roles(user_id).await?;
        assert!(
            roles.contains(&role_id.to_string()),
            "User {} does not have role {}",
            user_id,
            role_id
        );
        Ok(())
    }
    
    /// Assert that a session is valid
    pub async fn assert_session_valid(
        session_manager: &SessionManager,
        session_id: &str,
    ) -> Result<()> {
        let is_valid = session_manager.validate_session(session_id).await?;
        assert!(is_valid, "Session {} is not valid", session_id);
        Ok(())
    }
    
    /// Assert that a JWT token contains expected claims
    pub fn assert_jwt_claims(
        jwt_service: &JwtService,
        token: &str,
        expected_sub: &str,
    ) -> Result<()> {
        let claims = jwt_service.decode_token(token)?;
        assert_eq!(claims.sub, expected_sub);
        Ok(())
    }
}

/// Performance testing utilities
pub mod performance {
    use super::*;
    use std::time::Instant;
    
    /// Measure the time taken to create multiple users
    pub async fn benchmark_user_creation(
        user_store: &mut UserStore,
        count: usize,
    ) -> Duration {
        let users = generators::generate_users(count);
        let start = Instant::now();
        
        for user_data in users {
            user_store.create_user(&user_data).await.unwrap();
        }
        
        start.elapsed()
    }
    
    /// Measure authentication performance
    pub async fn benchmark_authentication(
        auth_service: &AuthenticationService,
        username: &str,
        password: &str,
        iterations: usize,
    ) -> Duration {
        let start = Instant::now();
        
        for _ in 0..iterations {
            auth_service.authenticate(username, password).await.unwrap();
        }
        
        start.elapsed()
    }
    
    /// Measure permission checking performance
    pub async fn benchmark_permission_check(
        authz_service: &AuthorizationService,
        user_id: &str,
        permission: &str,
        iterations: usize,
    ) -> Duration {
        let start = Instant::now();
        
        for _ in 0..iterations {
            authz_service.has_permission(user_id, permission).await.unwrap();
        }
        
        start.elapsed()
    }
}

/// Security testing utilities
pub mod security {
    use super::*;
    
    /// Test for SQL injection vulnerabilities
    pub async fn test_sql_injection(user_store: &mut UserStore) -> Result<()> {
        let malicious_inputs = vec![
            "admin'; DROP TABLE users; --",
            "' OR '1'='1",
            "'; DELETE FROM users WHERE '1'='1'; --",
            "\"; DROP TABLE users; --",
        ];
        
        for input in malicious_inputs {
            let user_data = UserData {
                id: None,
                username: input.to_string(),
                email: format!("{}@test.com", input.replace("'", "")),
                full_name: input.to_string(),
                metadata: HashMap::new(),
            };
            
            // Should either safely handle or reject the input
            let _ = user_store.create_user(&user_data).await;
        }
        
        // Verify the store is still functional
        let test_user = UserData {
            id: None,
            username: "normaluser".to_string(),
            email: "normal@example.com".to_string(),
            full_name: "Normal User".to_string(),
            metadata: HashMap::new(),
        };
        
        user_store.create_user(&test_user).await?;
        Ok(())
    }
    
    /// Test for timing attack vulnerabilities
    pub async fn test_timing_attack(
        auth_service: &AuthenticationService,
    ) -> Result<()> {
        // Create a user
        auth_service.register("timing_user", "TestP@ss123!").await?;
        
        let mut valid_times = vec![];
        let mut invalid_times = vec![];
        
        // Measure authentication times
        for _ in 0..100 {
            let start = Instant::now();
            let _ = auth_service.authenticate("timing_user", "TestP@ss123!").await;
            valid_times.push(start.elapsed());
            
            let start = Instant::now();
            let _ = auth_service.authenticate("timing_user", "WrongP@ss123!").await;
            invalid_times.push(start.elapsed());
        }
        
        // Calculate averages
        let valid_avg = valid_times.iter().sum::<Duration>() / valid_times.len() as u32;
        let invalid_avg = invalid_times.iter().sum::<Duration>() / invalid_times.len() as u32;
        
        // Times should be similar (constant-time comparison)
        let diff = if valid_avg > invalid_avg {
            valid_avg - invalid_avg
        } else {
            invalid_avg - valid_avg
        };
        
        assert!(
            diff < Duration::from_millis(5),
            "Timing difference detected: {:?}",
            diff
        );
        
        Ok(())
    }
}