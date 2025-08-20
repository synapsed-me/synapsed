//! Unit tests for authentication module
//! 
//! These tests verify password hashing, verification, and authentication flows.

#![cfg(test)]

use synapsed_identity::auth::*;
use synapsed_identity::{Error, Result};
use synapsed_crypto::ml_kem::*;
use crate::test_framework::{*, performance::*, security::*};

#[cfg(test)]
mod password_tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "SecureP@ssw0rd123!";
        
        // Hash password
        let hash_result = hash_password(password);
        assert!(hash_result.is_ok(), "Failed to hash password");
        
        let hash = hash_result.unwrap();
        assert!(!hash.is_empty(), "Password hash should not be empty");
        assert_ne!(hash, password, "Hash should not equal plaintext");
    }

    #[test]
    fn test_password_verification_success() {
        let password = "TestPassword123!";
        let hash = hash_password(password).unwrap();
        
        // Verify correct password
        let verify_result = verify_password(password, &hash);
        assert!(verify_result.is_ok(), "Password verification failed");
        assert!(verify_result.unwrap(), "Valid password should verify successfully");
    }

    #[test]
    fn test_password_verification_failure() {
        let password = "CorrectPassword123!";
        let wrong_password = "WrongPassword123!";
        let hash = hash_password(password).unwrap();
        
        // Verify wrong password
        let verify_result = verify_password(wrong_password, &hash);
        assert!(verify_result.is_ok(), "Verification should not error on wrong password");
        assert!(!verify_result.unwrap(), "Wrong password should not verify");
    }

    #[test]
    fn test_password_hash_uniqueness() {
        let password = "SamePassword123!";
        
        // Hash same password twice
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();
        
        // Hashes should be different due to salt
        assert_ne!(hash1, hash2, "Same password should produce different hashes");
    }

    #[test]
    fn test_password_strength_validation() {
        // Test weak passwords
        assert!(validate_password_strength("weak").is_err());
        assert!(validate_password_strength("12345678").is_err());
        assert!(validate_password_strength("password").is_err());
        
        // Test strong passwords
        assert!(validate_password_strength("Str0ng!P@ssw0rd").is_ok());
        assert!(validate_password_strength("C0mpl3x#P@ssphrase").is_ok());
    }

    #[test]
    fn test_constant_time_password_verification() {
        let password = "ConstantTimeTest123!";
        let hash = hash_password(password).unwrap();
        
        assert_constant_time!(|_| {
            verify_password(password, &hash).unwrap_or(false)
        });
    }
}

#[cfg(test)]
mod authentication_flow_tests {
    use super::*;

    #[test]
    fn test_basic_authentication() {
        let mut auth_service = AuthenticationService::new();
        
        // Register user
        let username = "testuser";
        let password = "SecureP@ss123";
        let register_result = auth_service.register(username, password);
        assert!(register_result.is_ok(), "Registration failed");
        
        // Authenticate user
        let auth_result = auth_service.authenticate(username, password);
        assert!(auth_result.is_ok(), "Authentication failed");
        
        let auth_token = auth_result.unwrap();
        assert!(!auth_token.is_empty(), "Auth token should not be empty");
    }

    #[test]
    fn test_authentication_with_wrong_password() {
        let mut auth_service = AuthenticationService::new();
        
        // Register user
        let username = "testuser2";
        let password = "CorrectP@ss123";
        auth_service.register(username, password).unwrap();
        
        // Try to authenticate with wrong password
        let auth_result = auth_service.authenticate(username, "WrongP@ss123");
        assert!(auth_result.is_err(), "Authentication should fail with wrong password");
    }

    #[test]
    fn test_authentication_nonexistent_user() {
        let auth_service = AuthenticationService::new();
        
        // Try to authenticate non-existent user
        let auth_result = auth_service.authenticate("nonexistent", "AnyP@ss123");
        assert!(auth_result.is_err(), "Authentication should fail for non-existent user");
    }

    #[test]
    fn test_multi_factor_authentication() {
        let mut auth_service = AuthenticationService::new();
        
        // Register user with MFA
        let username = "mfauser";
        let password = "SecureP@ss123";
        auth_service.register_with_mfa(username, password).unwrap();
        
        // First factor: password
        let first_factor = auth_service.authenticate_first_factor(username, password);
        assert!(first_factor.is_ok(), "First factor authentication failed");
        
        let mfa_token = first_factor.unwrap();
        
        // Second factor: TOTP
        let totp_code = "123456"; // In real tests, generate valid TOTP
        let second_factor = auth_service.authenticate_second_factor(&mfa_token, totp_code);
        assert!(second_factor.is_ok(), "Second factor authentication failed");
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let mut auth_service = AuthenticationService::new();
        let username = "sessionuser";
        let password = "SessionP@ss123";
        
        // Register and authenticate
        auth_service.register(username, password).unwrap();
        let auth_token = auth_service.authenticate(username, password).unwrap();
        
        // Create session
        let session = auth_service.create_session(&auth_token).unwrap();
        assert!(!session.id().is_empty(), "Session ID should not be empty");
        assert!(session.is_valid(), "New session should be valid");
    }

    #[test]
    fn test_session_expiration() {
        let mut auth_service = AuthenticationService::new();
        let username = "expireuser";
        let password = "ExpireP@ss123";
        
        // Create session with short TTL
        auth_service.register(username, password).unwrap();
        let auth_token = auth_service.authenticate(username, password).unwrap();
        let mut session = auth_service.create_session_with_ttl(&auth_token, 1).unwrap(); // 1 second TTL
        
        // Session should be valid initially
        assert!(session.is_valid(), "Session should be valid initially");
        
        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        // Session should be expired
        assert!(!session.is_valid(), "Session should be expired after TTL");
    }

    #[test]
    fn test_session_refresh() {
        let mut auth_service = AuthenticationService::new();
        let username = "refreshuser";
        let password = "RefreshP@ss123";
        
        // Create session
        auth_service.register(username, password).unwrap();
        let auth_token = auth_service.authenticate(username, password).unwrap();
        let mut session = auth_service.create_session(&auth_token).unwrap();
        
        let original_expiry = session.expiry_time();
        
        // Refresh session
        std::thread::sleep(std::time::Duration::from_secs(1));
        let refresh_result = session.refresh();
        assert!(refresh_result.is_ok(), "Session refresh failed");
        
        let new_expiry = session.expiry_time();
        assert!(new_expiry > original_expiry, "Session expiry should be extended after refresh");
    }

    #[test]
    fn test_session_invalidation() {
        let mut auth_service = AuthenticationService::new();
        let username = "invalidateuser";
        let password = "InvalidateP@ss123";
        
        // Create session
        auth_service.register(username, password).unwrap();
        let auth_token = auth_service.authenticate(username, password).unwrap();
        let mut session = auth_service.create_session(&auth_token).unwrap();
        
        // Session should be valid
        assert!(session.is_valid(), "Session should be valid initially");
        
        // Invalidate session
        let invalidate_result = session.invalidate();
        assert!(invalidate_result.is_ok(), "Session invalidation failed");
        
        // Session should no longer be valid
        assert!(!session.is_valid(), "Session should be invalid after invalidation");
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use criterion::black_box;

    #[test]
    fn test_password_hashing_performance() {
        let password = "PerformanceTestP@ss123";
        
        assert_performance!(
            || {
                hash_password(black_box(password)).unwrap();
            },
            500 // 500ms threshold for password hashing
        );
    }

    #[test]
    fn test_authentication_performance() {
        let mut auth_service = AuthenticationService::new();
        let username = "perfuser";
        let password = "PerfP@ss123";
        
        auth_service.register(username, password).unwrap();
        
        assert_performance!(
            || {
                auth_service.authenticate(black_box(username), black_box(password)).unwrap();
            },
            100 // 100ms threshold for authentication
        );
    }

    #[test]
    fn test_concurrent_authentication() {
        use std::sync::Arc;
        use std::thread;
        
        let auth_service = Arc::new(std::sync::Mutex::new(AuthenticationService::new()));
        let num_threads = 10;
        let auths_per_thread = 100;
        
        // Register test users
        for i in 0..num_threads {
            let username = format!("concurrentuser{}", i);
            let password = format!("ConcurrentP@ss{}", i);
            auth_service.lock().unwrap().register(&username, &password).unwrap();
        }
        
        let (_, elapsed) = measure_time(|| {
            let mut handles = vec![];
            
            for i in 0..num_threads {
                let auth_service_clone = Arc::clone(&auth_service);
                let handle = thread::spawn(move || {
                    let username = format!("concurrentuser{}", i);
                    let password = format!("ConcurrentP@ss{}", i);
                    
                    for _ in 0..auths_per_thread {
                        auth_service_clone.lock().unwrap()
                            .authenticate(&username, &password).unwrap();
                    }
                });
                handles.push(handle);
            }
            
            for handle in handles {
                handle.join().unwrap();
            }
        });
        
        let total_auths = num_threads * auths_per_thread;
        let avg_time = elapsed as f64 / total_auths as f64;
        
        assert!(
            avg_time < 10.0,
            "Average authentication time too high: {:.2} ms",
            avg_time
        );
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_timing_attack_resistance() {
        let mut auth_service = AuthenticationService::new();
        let username = "timinguser";
        let password = "TimingP@ss123";
        
        auth_service.register(username, password).unwrap();
        
        // Test that authentication time is consistent regardless of username length
        let short_user = "a";
        let long_user = "a".repeat(100);
        
        let mut short_times = vec![];
        let mut long_times = vec![];
        
        for _ in 0..100 {
            let (_, time) = measure_time(|| {
                let _ = auth_service.authenticate(short_user, "wrongpass");
            });
            short_times.push(time);
            
            let (_, time) = measure_time(|| {
                let _ = auth_service.authenticate(&long_user, "wrongpass");
            });
            long_times.push(time);
        }
        
        let short_avg = short_times.iter().sum::<u128>() / short_times.len() as u128;
        let long_avg = long_times.iter().sum::<u128>() / long_times.len() as u128;
        
        let diff = (short_avg as i128 - long_avg as i128).abs();
        assert!(
            diff < 5,
            "Timing difference detected: {} ms",
            diff
        );
    }

    #[test]
    fn test_brute_force_protection() {
        let mut auth_service = AuthenticationService::new();
        let username = "bruteuser";
        let password = "BruteP@ss123";
        
        auth_service.register(username, password).unwrap();
        
        // Attempt multiple failed logins
        for i in 0..5 {
            let wrong_password = format!("WrongPass{}", i);
            let _ = auth_service.authenticate(username, &wrong_password);
        }
        
        // Account should be locked after failed attempts
        let result = auth_service.authenticate(username, password);
        assert!(
            matches!(result, Err(Error::AccountLocked)),
            "Account should be locked after multiple failed attempts"
        );
    }

    #[test]
    fn test_session_fixation_prevention() {
        let mut auth_service = AuthenticationService::new();
        let username = "fixationuser";
        let password = "FixationP@ss123";
        
        auth_service.register(username, password).unwrap();
        
        // Create initial session
        let auth_token1 = auth_service.authenticate(username, password).unwrap();
        let session1 = auth_service.create_session(&auth_token1).unwrap();
        let session_id1 = session1.id().to_string();
        
        // Authenticate again (simulating login from different location)
        let auth_token2 = auth_service.authenticate(username, password).unwrap();
        let session2 = auth_service.create_session(&auth_token2).unwrap();
        let session_id2 = session2.id().to_string();
        
        // Session IDs should be different
        assert_ne!(
            session_id1, session_id2,
            "Session IDs should be different to prevent fixation"
        );
    }
}