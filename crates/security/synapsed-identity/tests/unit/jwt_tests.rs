//! Unit tests for JWT token management
//! 
//! These tests verify JWT creation, validation, refresh, and revocation.

#![cfg(test)]

use synapsed_identity::auth::jwt::*;
use synapsed_identity::{Error, Result};
use synapsed_crypto::ml_dsa::*;
use crate::test_framework::{*, performance::*, security::*};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(test)]
mod jwt_creation_tests {
    use super::*;

    #[test]
    fn test_create_jwt_token() {
        let jwt_service = JwtService::new();
        let user_id = "test-user-123";
        let claims = JwtClaims {
            sub: user_id.to_string(),
            exp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 3600,
            iat: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            nbf: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            jti: uuid::Uuid::new_v4().to_string(),
            aud: vec!["api.example.com".to_string()],
            iss: "auth.example.com".to_string(),
        };
        
        let token_result = jwt_service.create_token(&claims);
        assert!(token_result.is_ok(), "Failed to create JWT token");
        
        let token = token_result.unwrap();
        assert!(!token.is_empty(), "JWT token should not be empty");
        
        // Verify token has three parts (header.payload.signature)
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT should have three parts");
    }

    #[test]
    fn test_create_jwt_with_custom_claims() {
        let jwt_service = JwtService::new();
        let mut claims = JwtClaims::default();
        claims.sub = "user-456".to_string();
        claims.custom.insert("role".to_string(), "admin".to_string());
        claims.custom.insert("department".to_string(), "engineering".to_string());
        
        let token = jwt_service.create_token(&claims).unwrap();
        
        // Verify token can be decoded and contains custom claims
        let decoded_claims = jwt_service.decode_token(&token).unwrap();
        assert_eq!(decoded_claims.custom.get("role"), Some(&"admin".to_string()));
        assert_eq!(decoded_claims.custom.get("department"), Some(&"engineering".to_string()));
    }

    #[test]
    fn test_create_jwt_with_post_quantum_signature() {
        let jwt_service = JwtService::with_post_quantum();
        let claims = JwtClaims::default();
        
        let token = jwt_service.create_token(&claims).unwrap();
        
        // Verify token uses post-quantum algorithm
        let header = jwt_service.decode_header(&token).unwrap();
        assert_eq!(header.alg, "ML-DSA-87", "Should use ML-DSA post-quantum algorithm");
    }
}

#[cfg(test)]
mod jwt_validation_tests {
    use super::*;

    #[test]
    fn test_validate_valid_token() {
        let jwt_service = JwtService::new();
        let claims = JwtClaims {
            sub: "test-user".to_string(),
            exp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 3600,
            ..Default::default()
        };
        
        let token = jwt_service.create_token(&claims).unwrap();
        
        // Validate token
        let validation_result = jwt_service.validate_token(&token);
        assert!(validation_result.is_ok(), "Valid token should pass validation");
        
        let validated_claims = validation_result.unwrap();
        assert_eq!(validated_claims.sub, claims.sub);
    }

    #[test]
    fn test_validate_expired_token() {
        let jwt_service = JwtService::new();
        let claims = JwtClaims {
            sub: "test-user".to_string(),
            exp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 3600, // Expired 1 hour ago
            ..Default::default()
        };
        
        let token = jwt_service.create_token(&claims).unwrap();
        
        // Validation should fail
        let validation_result = jwt_service.validate_token(&token);
        assert!(validation_result.is_err(), "Expired token should fail validation");
        assert!(matches!(validation_result.unwrap_err(), Error::TokenExpired));
    }

    #[test]
    fn test_validate_not_before_token() {
        let jwt_service = JwtService::new();
        let claims = JwtClaims {
            sub: "test-user".to_string(),
            exp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 7200,
            nbf: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 3600, // Not valid for 1 hour
            ..Default::default()
        };
        
        let token = jwt_service.create_token(&claims).unwrap();
        
        // Validation should fail
        let validation_result = jwt_service.validate_token(&token);
        assert!(validation_result.is_err(), "Token not yet valid should fail validation");
        assert!(matches!(validation_result.unwrap_err(), Error::TokenNotYetValid));
    }

    #[test]
    fn test_validate_tampered_token() {
        let jwt_service = JwtService::new();
        let claims = JwtClaims::default();
        
        let mut token = jwt_service.create_token(&claims).unwrap();
        
        // Tamper with the token
        let parts: Vec<&str> = token.split('.').collect();
        token = format!("{}.tampered.{}", parts[0], parts[2]);
        
        // Validation should fail
        let validation_result = jwt_service.validate_token(&token);
        assert!(validation_result.is_err(), "Tampered token should fail validation");
        assert!(matches!(validation_result.unwrap_err(), Error::InvalidSignature));
    }

    #[test]
    fn test_validate_audience() {
        let jwt_service = JwtService::new();
        let claims = JwtClaims {
            sub: "test-user".to_string(),
            aud: vec!["api.example.com".to_string(), "web.example.com".to_string()],
            ..Default::default()
        };
        
        let token = jwt_service.create_token(&claims).unwrap();
        
        // Validate with correct audience
        let validation_options = ValidationOptions {
            validate_aud: true,
            required_aud: Some("api.example.com".to_string()),
            ..Default::default()
        };
        
        let result = jwt_service.validate_token_with_options(&token, &validation_options);
        assert!(result.is_ok(), "Token with correct audience should validate");
        
        // Validate with incorrect audience
        let wrong_options = ValidationOptions {
            validate_aud: true,
            required_aud: Some("wrong.example.com".to_string()),
            ..Default::default()
        };
        
        let wrong_result = jwt_service.validate_token_with_options(&token, &wrong_options);
        assert!(wrong_result.is_err(), "Token with wrong audience should fail");
    }
}

#[cfg(test)]
mod jwt_refresh_tests {
    use super::*;

    #[test]
    fn test_refresh_token_creation() {
        let jwt_service = JwtService::new();
        let user_id = "refresh-user";
        
        let refresh_token = jwt_service.create_refresh_token(user_id).unwrap();
        assert!(!refresh_token.is_empty(), "Refresh token should not be empty");
        
        // Refresh tokens should be longer-lived
        let claims = jwt_service.decode_token(&refresh_token).unwrap();
        let exp_duration = claims.exp - claims.iat;
        assert!(exp_duration > 86400, "Refresh token should be valid for more than 24 hours");
    }

    #[test]
    fn test_token_refresh() {
        let jwt_service = JwtService::new();
        let original_claims = JwtClaims {
            sub: "test-user".to_string(),
            exp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 300, // 5 minutes
            ..Default::default()
        };
        
        let access_token = jwt_service.create_token(&original_claims).unwrap();
        let refresh_token = jwt_service.create_refresh_token(&original_claims.sub).unwrap();
        
        // Refresh the access token
        let new_access_token = jwt_service.refresh_access_token(&refresh_token).unwrap();
        
        // Verify new token has updated expiry
        let new_claims = jwt_service.decode_token(&new_access_token).unwrap();
        assert!(new_claims.exp > original_claims.exp, "New token should have later expiry");
        assert_eq!(new_claims.sub, original_claims.sub, "Subject should remain the same");
    }

    #[test]
    fn test_refresh_token_rotation() {
        let mut jwt_service = JwtService::new();
        let user_id = "rotation-user";
        
        // Enable refresh token rotation
        jwt_service.enable_refresh_rotation(true);
        
        let refresh_token1 = jwt_service.create_refresh_token(user_id).unwrap();
        
        // Use refresh token
        let (new_access, new_refresh) = jwt_service.refresh_with_rotation(&refresh_token1).unwrap();
        
        // Old refresh token should be invalidated
        let reuse_result = jwt_service.refresh_with_rotation(&refresh_token1);
        assert!(reuse_result.is_err(), "Old refresh token should be invalidated");
        
        // New refresh token should work
        let (_, _) = jwt_service.refresh_with_rotation(&new_refresh).unwrap();
    }

    #[test]
    fn test_refresh_token_family_detection() {
        let mut jwt_service = JwtService::new();
        jwt_service.enable_refresh_rotation(true);
        jwt_service.enable_family_detection(true);
        
        let user_id = "family-user";
        let refresh_token = jwt_service.create_refresh_token(user_id).unwrap();
        
        // Normal refresh
        let (_, new_refresh) = jwt_service.refresh_with_rotation(&refresh_token).unwrap();
        
        // Attempt to reuse old token (potential token theft)
        let reuse_result = jwt_service.refresh_with_rotation(&refresh_token);
        assert!(reuse_result.is_err(), "Reused token should be rejected");
        
        // All tokens in family should be invalidated
        let family_result = jwt_service.refresh_with_rotation(&new_refresh);
        assert!(family_result.is_err(), "All tokens in family should be invalidated after reuse");
    }
}

#[cfg(test)]
mod jwt_revocation_tests {
    use super::*;

    #[test]
    fn test_token_revocation() {
        let mut jwt_service = JwtService::new();
        let claims = JwtClaims::default();
        
        let token = jwt_service.create_token(&claims).unwrap();
        let jti = jwt_service.decode_token(&token).unwrap().jti;
        
        // Token should be valid initially
        assert!(jwt_service.validate_token(&token).is_ok());
        
        // Revoke token
        jwt_service.revoke_token(&jti).unwrap();
        
        // Token should now be invalid
        let validation_result = jwt_service.validate_token(&token);
        assert!(validation_result.is_err(), "Revoked token should fail validation");
        assert!(matches!(validation_result.unwrap_err(), Error::TokenRevoked));
    }

    #[test]
    fn test_bulk_revocation() {
        let mut jwt_service = JwtService::new();
        let user_id = "bulk-revoke-user";
        
        // Create multiple tokens
        let mut tokens = vec![];
        for _ in 0..5 {
            let claims = JwtClaims {
                sub: user_id.to_string(),
                ..Default::default()
            };
            tokens.push(jwt_service.create_token(&claims).unwrap());
        }
        
        // All tokens should be valid
        for token in &tokens {
            assert!(jwt_service.validate_token(token).is_ok());
        }
        
        // Revoke all tokens for user
        jwt_service.revoke_all_user_tokens(user_id).unwrap();
        
        // All tokens should now be invalid
        for token in &tokens {
            assert!(jwt_service.validate_token(token).is_err());
        }
    }

    #[test]
    fn test_revocation_list_persistence() {
        let mut jwt_service = JwtService::new();
        let token = jwt_service.create_token(&JwtClaims::default()).unwrap();
        let jti = jwt_service.decode_token(&token).unwrap().jti;
        
        // Revoke token
        jwt_service.revoke_token(&jti).unwrap();
        
        // Save revocation list
        let revocation_list = jwt_service.export_revocation_list().unwrap();
        
        // Create new service and import list
        let mut new_service = JwtService::new();
        new_service.import_revocation_list(&revocation_list).unwrap();
        
        // Token should still be revoked in new service
        assert!(new_service.validate_token(&token).is_err());
    }
}

#[cfg(test)]
mod jwt_performance_tests {
    use super::*;

    #[test]
    fn test_token_creation_performance() {
        let jwt_service = JwtService::new();
        let claims = JwtClaims::default();
        
        assert_performance!(
            || {
                jwt_service.create_token(&claims).unwrap();
            },
            10 // 10ms threshold for token creation
        );
    }

    #[test]
    fn test_token_validation_performance() {
        let jwt_service = JwtService::new();
        let token = jwt_service.create_token(&JwtClaims::default()).unwrap();
        
        assert_performance!(
            || {
                jwt_service.validate_token(&token).unwrap();
            },
            5 // 5ms threshold for token validation
        );
    }

    #[test]
    fn test_bulk_token_operations() {
        let jwt_service = JwtService::new();
        let num_tokens = 1000;
        
        // Test bulk creation
        let (tokens, create_time) = measure_time(|| {
            let mut tokens = Vec::with_capacity(num_tokens);
            for i in 0..num_tokens {
                let claims = JwtClaims {
                    sub: format!("user-{}", i),
                    ..Default::default()
                };
                tokens.push(jwt_service.create_token(&claims).unwrap());
            }
            tokens
        });
        
        let avg_create = create_time as f64 / num_tokens as f64;
        assert!(avg_create < 5.0, "Average token creation time too high: {:.2} ms", avg_create);
        
        // Test bulk validation
        let (_, validate_time) = measure_time(|| {
            for token in &tokens {
                jwt_service.validate_token(token).unwrap();
            }
        });
        
        let avg_validate = validate_time as f64 / num_tokens as f64;
        assert!(avg_validate < 2.0, "Average token validation time too high: {:.2} ms", avg_validate);
    }
}

#[cfg(test)]
mod jwt_security_tests {
    use super::*;

    #[test]
    fn test_algorithm_confusion_attack() {
        let jwt_service = JwtService::new();
        let claims = JwtClaims::default();
        
        // Create token with RS256
        let token = jwt_service.create_token(&claims).unwrap();
        
        // Try to create a token with "none" algorithm
        let parts: Vec<&str> = token.split('.').collect();
        let header = r#"{"alg":"none","typ":"JWT"}"#;
        let encoded_header = base64::encode_config(header, base64::URL_SAFE_NO_PAD);
        let malicious_token = format!("{}.{}.", encoded_header, parts[1]);
        
        // Validation should reject "none" algorithm
        let result = jwt_service.validate_token(&malicious_token);
        assert!(result.is_err(), "Token with 'none' algorithm should be rejected");
    }

    #[test]
    fn test_key_confusion_attack() {
        let jwt_service = JwtService::new();
        let claims = JwtClaims::default();
        
        // Create token with asymmetric key
        let token = jwt_service.create_token(&claims).unwrap();
        
        // Try to validate with symmetric key (simulating key confusion)
        let fake_service = JwtService::with_symmetric_key(b"fake-symmetric-key");
        let result = fake_service.validate_token(&token);
        
        assert!(result.is_err(), "Token should not validate with wrong key type");
    }

    #[test]
    fn test_jti_uniqueness() {
        let jwt_service = JwtService::new();
        let mut jtis = std::collections::HashSet::new();
        
        // Create many tokens and verify JTI uniqueness
        for _ in 0..1000 {
            let token = jwt_service.create_token(&JwtClaims::default()).unwrap();
            let jti = jwt_service.decode_token(&token).unwrap().jti;
            
            assert!(jtis.insert(jti), "JTI should be unique");
        }
    }

    #[test]
    fn test_constant_time_signature_verification() {
        let jwt_service = JwtService::new();
        let token = jwt_service.create_token(&JwtClaims::default()).unwrap();
        
        // Tamper with signature
        let mut parts: Vec<&str> = token.split('.').collect();
        let tampered_sig = base64::encode_config(b"wrong-signature", base64::URL_SAFE_NO_PAD);
        let tampered_token = format!("{}.{}.{}", parts[0], parts[1], tampered_sig);
        
        assert_constant_time!(|_| {
            jwt_service.validate_token(&tampered_token).is_ok()
        });
    }
}