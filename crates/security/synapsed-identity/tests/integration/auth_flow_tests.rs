//! Integration tests for complete authentication flows
//! 
//! These tests verify end-to-end authentication scenarios.

#![cfg(test)]

use synapsed_identity::{
    auth::{AuthenticationService, JwtService},
    authorization::AuthorizationService,
    storage::{UserStore, IdentityStorage},
    session::SessionManager,
    Error, Result,
};
use crate::test_framework::{*, performance::*, security::*};
use std::time::Duration;
use tokio;

#[tokio::test]
async fn test_complete_registration_and_login_flow() {
    // Initialize services
    let mut user_store = UserStore::new();
    let mut auth_service = AuthenticationService::new();
    let jwt_service = JwtService::new();
    let mut session_manager = SessionManager::new();
    
    // Step 1: User Registration
    let username = "newuser";
    let email = "newuser@example.com";
    let password = "SecureP@ssw0rd123!";
    
    // Create user account
    let user_data = UserData {
        id: None,
        username: username.to_string(),
        email: email.to_string(),
        full_name: "New User".to_string(),
        metadata: Default::default(),
    };
    
    let user = user_store.create_user(&user_data).await.unwrap();
    let user_id = user.id.clone();
    
    // Register authentication credentials
    auth_service.register(&user_id, password).await.unwrap();
    
    // Step 2: User Login
    let login_result = auth_service.authenticate(username, password).await;
    assert!(login_result.is_ok(), "Login failed");
    
    let auth_token = login_result.unwrap();
    
    // Step 3: Create JWT
    let jwt_claims = JwtClaims {
        sub: user_id.clone(),
        exp: (SystemTime::now() + Duration::from_secs(3600))
            .duration_since(UNIX_EPOCH).unwrap().as_secs(),
        ..Default::default()
    };
    
    let jwt_token = jwt_service.create_token(&jwt_claims).unwrap();
    
    // Step 4: Create Session
    let session = session_manager.create_session(&user_id, &jwt_token).await.unwrap();
    assert!(session.is_valid(), "Session should be valid");
    
    // Step 5: Verify Complete Authentication State
    assert!(session_manager.validate_session(&session.id()).await.unwrap());
    assert!(jwt_service.validate_token(&jwt_token).is_ok());
    
    // Step 6: Logout
    session_manager.invalidate_session(&session.id()).await.unwrap();
    assert!(!session_manager.validate_session(&session.id()).await.unwrap());
}

#[tokio::test]
async fn test_multi_factor_authentication_flow() {
    let mut auth_service = AuthenticationService::new();
    let mut session_manager = SessionManager::new();
    
    // Register user with MFA enabled
    let user_id = "mfa-user";
    let password = "MfaP@ssw0rd123!";
    
    auth_service.register_with_mfa(user_id, password).await.unwrap();
    
    // Step 1: First factor - password
    let first_factor_result = auth_service
        .authenticate_first_factor(user_id, password).await;
    assert!(first_factor_result.is_ok(), "First factor failed");
    
    let mfa_challenge = first_factor_result.unwrap();
    assert!(!mfa_challenge.is_complete);
    assert!(mfa_challenge.requires_second_factor);
    
    // Step 2: Generate TOTP code (in real scenario, from authenticator app)
    let totp_secret = auth_service.get_totp_secret(user_id).await.unwrap();
    let totp_code = generate_totp_code(&totp_secret);
    
    // Step 3: Second factor - TOTP
    let second_factor_result = auth_service
        .authenticate_second_factor(&mfa_challenge.token, &totp_code).await;
    assert!(second_factor_result.is_ok(), "Second factor failed");
    
    let complete_auth = second_factor_result.unwrap();
    assert!(complete_auth.is_complete);
    
    // Step 4: Create session with MFA
    let session = session_manager
        .create_mfa_session(user_id, &complete_auth.token).await.unwrap();
    assert!(session.is_mfa_verified());
}

#[tokio::test]
async fn test_password_reset_flow() {
    let mut user_store = UserStore::new();
    let mut auth_service = AuthenticationService::new();
    
    // Create user
    let user_data = UserData {
        id: None,
        username: "resetuser".to_string(),
        email: "reset@example.com".to_string(),
        full_name: "Reset User".to_string(),
        metadata: Default::default(),
    };
    
    let user = user_store.create_user(&user_data).await.unwrap();
    let user_id = user.id.clone();
    
    // Register with initial password
    let old_password = "OldP@ssw0rd123!";
    auth_service.register(&user_id, old_password).await.unwrap();
    
    // Step 1: Request password reset
    let reset_token = auth_service
        .request_password_reset("reset@example.com").await.unwrap();
    
    // Step 2: Verify reset token is valid
    let token_valid = auth_service
        .validate_reset_token(&reset_token).await.unwrap();
    assert!(token_valid);
    
    // Step 3: Reset password with token
    let new_password = "NewP@ssw0rd456!";
    auth_service
        .reset_password(&reset_token, new_password).await.unwrap();
    
    // Step 4: Verify old password no longer works
    let old_auth = auth_service
        .authenticate("resetuser", old_password).await;
    assert!(old_auth.is_err());
    
    // Step 5: Verify new password works
    let new_auth = auth_service
        .authenticate("resetuser", new_password).await;
    assert!(new_auth.is_ok());
    
    // Step 6: Verify reset token is consumed
    let token_reuse = auth_service
        .validate_reset_token(&reset_token).await.unwrap();
    assert!(!token_reuse, "Reset token should be consumed");
}

#[tokio::test]
async fn test_session_management_flow() {
    let mut auth_service = AuthenticationService::new();
    let mut session_manager = SessionManager::new();
    let jwt_service = JwtService::new();
    
    // Create authenticated user
    let user_id = "session-user";
    let password = "SessionP@ss123!";
    
    auth_service.register(user_id, password).await.unwrap();
    let auth_token = auth_service.authenticate(user_id, password).await.unwrap();
    
    // Create multiple sessions (different devices)
    let mut sessions = vec![];
    for device in &["mobile", "desktop", "tablet"] {
        let jwt_claims = JwtClaims {
            sub: user_id.to_string(),
            device: Some(device.to_string()),
            ..Default::default()
        };
        
        let jwt = jwt_service.create_token(&jwt_claims).unwrap();
        let session = session_manager
            .create_session_with_metadata(user_id, &jwt, device).await.unwrap();
        sessions.push(session);
    }
    
    // Verify all sessions are active
    let active_sessions = session_manager
        .get_user_sessions(user_id).await.unwrap();
    assert_eq!(active_sessions.len(), 3);
    
    // Refresh one session
    let mobile_session = &sessions[0];
    session_manager.refresh_session(&mobile_session.id()).await.unwrap();
    
    // Invalidate one session (logout from desktop)
    let desktop_session = &sessions[1];
    session_manager.invalidate_session(&desktop_session.id()).await.unwrap();
    
    // Verify session count
    let remaining_sessions = session_manager
        .get_user_sessions(user_id).await.unwrap();
    assert_eq!(remaining_sessions.len(), 2);
    
    // Invalidate all sessions (logout from all devices)
    session_manager.invalidate_all_user_sessions(user_id).await.unwrap();
    
    let final_sessions = session_manager
        .get_user_sessions(user_id).await.unwrap();
    assert_eq!(final_sessions.len(), 0);
}

#[tokio::test]
async fn test_role_based_access_flow() {
    let mut user_store = UserStore::new();
    let mut auth_service = AuthenticationService::new();
    let mut authz_service = AuthorizationService::new();
    let jwt_service = JwtService::new();
    
    // Create users with different roles
    let admin_data = UserData {
        id: None,
        username: "admin".to_string(),
        email: "admin@example.com".to_string(),
        full_name: "Admin User".to_string(),
        metadata: Default::default(),
    };
    let admin = user_store.create_user(&admin_data).await.unwrap();
    
    let user_data = UserData {
        id: None,
        username: "regular".to_string(),
        email: "user@example.com".to_string(),
        full_name: "Regular User".to_string(),
        metadata: Default::default(),
    };
    let regular_user = user_store.create_user(&user_data).await.unwrap();
    
    // Set up roles
    let admin_role = Role {
        id: "admin".to_string(),
        name: "Administrator".to_string(),
        permissions: vec!["*".to_string()], // All permissions
    };
    
    let user_role = Role {
        id: "user".to_string(),
        name: "User".to_string(),
        permissions: vec!["profile:read".to_string(), "profile:write".to_string()],
    };
    
    authz_service.create_role(&admin_role).await.unwrap();
    authz_service.create_role(&user_role).await.unwrap();
    
    // Assign roles
    authz_service.assign_role(&admin.id, "admin").await.unwrap();
    authz_service.assign_role(&regular_user.id, "user").await.unwrap();
    
    // Test admin access
    assert!(authz_service.has_permission(&admin.id, "users:delete").await.unwrap());
    assert!(authz_service.has_permission(&admin.id, "system:manage").await.unwrap());
    
    // Test regular user access
    assert!(authz_service.has_permission(&regular_user.id, "profile:read").await.unwrap());
    assert!(!authz_service.has_permission(&regular_user.id, "users:delete").await.unwrap());
    
    // Create JWTs with roles
    let admin_jwt = jwt_service.create_token(&JwtClaims {
        sub: admin.id.clone(),
        roles: vec!["admin".to_string()],
        ..Default::default()
    }).unwrap();
    
    let user_jwt = jwt_service.create_token(&JwtClaims {
        sub: regular_user.id.clone(),
        roles: vec!["user".to_string()],
        ..Default::default()
    }).unwrap();
    
    // Verify role claims in tokens
    let admin_claims = jwt_service.decode_token(&admin_jwt).unwrap();
    assert!(admin_claims.roles.contains(&"admin".to_string()));
    
    let user_claims = jwt_service.decode_token(&user_jwt).unwrap();
    assert!(user_claims.roles.contains(&"user".to_string()));
}

#[tokio::test]
async fn test_account_lockout_and_recovery() {
    let mut auth_service = AuthenticationService::new();
    
    // Register user
    let user_id = "lockout-user";
    let password = "LockoutP@ss123!";
    auth_service.register(user_id, password).await.unwrap();
    
    // Configure lockout policy
    auth_service.set_lockout_policy(LockoutPolicy {
        max_attempts: 3,
        lockout_duration: Duration::from_secs(300), // 5 minutes
        reset_attempts_after: Duration::from_secs(600), // 10 minutes
    });
    
    // Attempt login with wrong password multiple times
    for _ in 0..3 {
        let result = auth_service.authenticate(user_id, "WrongPassword").await;
        assert!(result.is_err());
    }
    
    // Account should now be locked
    let locked_result = auth_service.authenticate(user_id, password).await;
    assert!(matches!(locked_result.unwrap_err(), Error::AccountLocked));
    
    // Check lockout status
    let status = auth_service.get_account_status(user_id).await.unwrap();
    assert!(status.is_locked);
    assert!(status.locked_until.is_some());
    
    // Admin unlock
    auth_service.admin_unlock_account(user_id).await.unwrap();
    
    // Should be able to login now
    let unlocked_result = auth_service.authenticate(user_id, password).await;
    assert!(unlocked_result.is_ok());
}

#[tokio::test]
async fn test_token_refresh_flow() {
    let jwt_service = JwtService::new();
    let mut session_manager = SessionManager::new();
    
    let user_id = "refresh-user";
    
    // Create initial tokens
    let access_token = jwt_service.create_access_token(user_id).unwrap();
    let refresh_token = jwt_service.create_refresh_token(user_id).unwrap();
    
    // Create session with tokens
    let session = session_manager
        .create_session_with_tokens(user_id, &access_token, &refresh_token)
        .await.unwrap();
    
    // Wait for access token to near expiry
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Refresh tokens
    let (new_access, new_refresh) = jwt_service
        .refresh_tokens(&refresh_token).unwrap();
    
    // Update session with new tokens
    session_manager
        .update_session_tokens(&session.id(), &new_access, &new_refresh)
        .await.unwrap();
    
    // Verify old refresh token is revoked
    let old_refresh_result = jwt_service.refresh_tokens(&refresh_token);
    assert!(old_refresh_result.is_err());
    
    // Verify new tokens work
    assert!(jwt_service.validate_token(&new_access).is_ok());
    assert!(jwt_service.validate_token(&new_refresh).is_ok());
}

#[tokio::test]
async fn test_federated_authentication_flow() {
    let mut auth_service = AuthenticationService::new();
    let mut user_store = UserStore::new();
    let jwt_service = JwtService::new();
    
    // Simulate OAuth/OIDC callback
    let oauth_profile = OAuthProfile {
        provider: "google".to_string(),
        provider_user_id: "google-123456".to_string(),
        email: "oauth@example.com".to_string(),
        name: "OAuth User".to_string(),
        picture: Some("https://example.com/picture.jpg".to_string()),
    };
    
    // Check if user exists
    let existing_user = user_store
        .get_user_by_federated_id(&oauth_profile.provider, &oauth_profile.provider_user_id)
        .await;
    
    let user = match existing_user {
        Ok(user) => user,
        Err(_) => {
            // Create new user from OAuth profile
            let user_data = UserData {
                id: None,
                username: format!("{}_{}", oauth_profile.provider, oauth_profile.provider_user_id),
                email: oauth_profile.email.clone(),
                full_name: oauth_profile.name.clone(),
                metadata: hashmap! {
                    "provider".to_string() => oauth_profile.provider.clone(),
                    "provider_id".to_string() => oauth_profile.provider_user_id.clone(),
                },
            };
            
            user_store.create_user(&user_data).await.unwrap()
        }
    };
    
    // Create federated session
    let jwt_claims = JwtClaims {
        sub: user.id.clone(),
        auth_method: Some("oauth".to_string()),
        provider: Some(oauth_profile.provider),
        ..Default::default()
    };
    
    let token = jwt_service.create_token(&jwt_claims).unwrap();
    assert!(!token.is_empty());
    
    // Verify federated authentication claims
    let decoded = jwt_service.decode_token(&token).unwrap();
    assert_eq!(decoded.auth_method, Some("oauth".to_string()));
}

// Helper function to generate TOTP code
fn generate_totp_code(secret: &str) -> String {
    // Simplified TOTP generation for testing
    // In production, use proper TOTP library
    "123456".to_string()
}

// Helper macro for creating hashmaps
macro_rules! hashmap {
    ($($key:expr => $value:expr),*) => {
        {
            let mut map = std::collections::HashMap::new();
            $(map.insert($key, $value);)*
            map
        }
    };
}