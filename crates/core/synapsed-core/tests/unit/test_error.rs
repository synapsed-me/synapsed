//! Unit tests for error handling module

use synapsed_core::error::{SynapsedError, SynapsedResult, IntoSynapsedError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation_methods() {
        let config_err = SynapsedError::config("test config error");
        assert_eq!(config_err, SynapsedError::Configuration("test config error".to_string()));

        let network_err = SynapsedError::network("connection failed");
        assert_eq!(network_err, SynapsedError::Network("connection failed".to_string()));

        let crypto_err = SynapsedError::crypto("invalid key");
        assert_eq!(crypto_err, SynapsedError::Cryptographic("invalid key".to_string()));

        let storage_err = SynapsedError::storage("disk full");
        assert_eq!(storage_err, SynapsedError::Storage("disk full".to_string()));

        let auth_err = SynapsedError::auth("unauthorized");
        assert_eq!(auth_err, SynapsedError::Authentication("unauthorized".to_string()));

        let input_err = SynapsedError::invalid_input("bad data");
        assert_eq!(input_err, SynapsedError::InvalidInput("bad data".to_string()));

        let not_found_err = SynapsedError::not_found("resource missing");
        assert_eq!(not_found_err, SynapsedError::NotFound("resource missing".to_string()));

        let permission_err = SynapsedError::permission_denied("access denied");
        assert_eq!(permission_err, SynapsedError::PermissionDenied("access denied".to_string()));

        let timeout_err = SynapsedError::timeout("operation timed out");
        assert_eq!(timeout_err, SynapsedError::Timeout("operation timed out".to_string()));

        let internal_err = SynapsedError::internal("server error");
        assert_eq!(internal_err, SynapsedError::Internal("server error".to_string()));

        let serialization_err = SynapsedError::serialization("parse failed");
        assert_eq!(serialization_err, SynapsedError::Serialization("parse failed".to_string()));

        let did_err = SynapsedError::did("invalid did");
        assert_eq!(did_err, SynapsedError::Did("invalid did".to_string()));

        let p2p_err = SynapsedError::p2p("peer unreachable");
        assert_eq!(p2p_err, SynapsedError::P2P("peer unreachable".to_string()));

        let wasm_err = SynapsedError::wasm("module failed");
        assert_eq!(wasm_err, SynapsedError::Wasm("module failed".to_string()));

        let payment_err = SynapsedError::payment("transaction failed");
        assert_eq!(payment_err, SynapsedError::Payment("transaction failed".to_string()));

        let app_err = SynapsedError::application("process failed", "user_id=123");
        match app_err {
            SynapsedError::Application { message, context } => {
                assert_eq!(message, "process failed");
                assert_eq!(context, "user_id=123");
            }
            _ => panic!("Expected Application error"),
        }
    }

    #[test]
    fn test_error_classification() {
        // Test retryable errors
        assert!(SynapsedError::network("connection failed").is_retryable());
        assert!(SynapsedError::timeout("operation timed out").is_retryable());
        assert!(SynapsedError::internal("server error").is_retryable());

        // Test non-retryable errors
        assert!(!SynapsedError::invalid_input("bad data").is_retryable());
        assert!(!SynapsedError::not_found("missing").is_retryable());
        assert!(!SynapsedError::permission_denied("access denied").is_retryable());
        assert!(!SynapsedError::auth("unauthorized").is_retryable());

        // Test client errors
        assert!(SynapsedError::invalid_input("bad data").is_client_error());
        assert!(SynapsedError::not_found("missing").is_client_error());
        assert!(SynapsedError::permission_denied("access denied").is_client_error());
        assert!(SynapsedError::auth("unauthorized").is_client_error());

        // Test server errors
        assert!(SynapsedError::internal("server error").is_server_error());
        assert!(SynapsedError::storage("disk full").is_server_error());
        assert!(SynapsedError::config("invalid config").is_server_error());

        // Test mutually exclusive classification
        let client_err = SynapsedError::invalid_input("test");
        assert!(client_err.is_client_error());
        assert!(!client_err.is_server_error());

        let server_err = SynapsedError::internal("test");
        assert!(server_err.is_server_error());
        assert!(!server_err.is_client_error());

        let network_err = SynapsedError::network("test");
        assert!(!network_err.is_client_error());
        assert!(!network_err.is_server_error());
    }

    #[test]
    fn test_error_display() {
        let config_err = SynapsedError::config("missing field");
        assert_eq!(config_err.to_string(), "Configuration error: missing field");

        let network_err = SynapsedError::network("connection refused");
        assert_eq!(network_err.to_string(), "Network error: connection refused");

        let app_err = SynapsedError::application("failed to process", "user_id=123");
        assert_eq!(app_err.to_string(), "Application error: failed to process (context: user_id=123)");
    }

    #[test]
    fn test_error_conversions() {
        // Test std::io::Error conversion
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let synapsed_err: SynapsedError = io_err.into();
        assert!(matches!(synapsed_err, SynapsedError::Internal(_)));
        assert!(synapsed_err.to_string().contains("file not found"));

        // Test serde_json::Error conversion
        let json_err: serde_json::Error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let synapsed_err: SynapsedError = json_err.into();
        assert!(matches!(synapsed_err, SynapsedError::Serialization(_)));

        // Test uuid::Error conversion
        let uuid_err = uuid::Uuid::parse_str("invalid-uuid").unwrap_err();
        let synapsed_err: SynapsedError = uuid_err.into();
        assert!(matches!(synapsed_err, SynapsedError::InvalidInput(_)));

        // Test chrono::ParseError conversion
        let chrono_err = chrono::DateTime::parse_from_rfc3339("invalid-date").unwrap_err();
        let synapsed_err: SynapsedError = chrono_err.into();
        assert!(matches!(synapsed_err, SynapsedError::InvalidInput(_)));
    }

    #[test]
    fn test_result_type_alias() {
        let success: SynapsedResult<i32> = Ok(42);
        assert!(success.is_ok());
        assert_eq!(success.unwrap(), 42);

        let error: SynapsedResult<i32> = Err(SynapsedError::invalid_input("test error"));
        assert!(error.is_err());
        let err = error.unwrap_err();
        assert!(matches!(err, SynapsedError::InvalidInput(_)));
    }

    #[test]
    fn test_error_debug_format() {
        let err = SynapsedError::config("test");
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Configuration"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_error_clone() {
        let original = SynapsedError::network("connection failed");
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_error_partial_eq() {
        let err1 = SynapsedError::config("test");
        let err2 = SynapsedError::config("test");
        let err3 = SynapsedError::config("different");
        let err4 = SynapsedError::network("test");

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
        assert_ne!(err1, err4);
    }

    #[test]
    fn test_all_error_variants() {
        let errors = vec![
            SynapsedError::Configuration("test".to_string()),
            SynapsedError::Network("test".to_string()),
            SynapsedError::Cryptographic("test".to_string()),
            SynapsedError::Storage("test".to_string()),
            SynapsedError::Authentication("test".to_string()),
            SynapsedError::InvalidInput("test".to_string()),
            SynapsedError::NotFound("test".to_string()),
            SynapsedError::PermissionDenied("test".to_string()),
            SynapsedError::Timeout("test".to_string()),
            SynapsedError::Internal("test".to_string()),
            SynapsedError::Serialization("test".to_string()),
            SynapsedError::Did("test".to_string()),
            SynapsedError::P2P("test".to_string()),
            SynapsedError::Wasm("test".to_string()),
            SynapsedError::Payment("test".to_string()),
            SynapsedError::Application {
                message: "test".to_string(),
                context: "test".to_string(),
            },
        ];

        for error in errors {
            // Each error should have a string representation
            assert!(!error.to_string().is_empty());
            
            // Each error should be classifiable
            let is_client = error.is_client_error();
            let is_server = error.is_server_error();
            let is_retryable = error.is_retryable();
            
            // Client and server errors should be mutually exclusive
            assert!(!(is_client && is_server));
            
            // Error should be cloneable and debuggable
            let _cloned = error.clone();
            let _debug = format!("{:?}", error);
        }
    }

    #[test]
    fn test_error_chaining() {
        // Test that errors can be chained properly
        fn inner_function() -> SynapsedResult<String> {
            Err(SynapsedError::storage("disk error"))
        }

        fn outer_function() -> SynapsedResult<String> {
            inner_function().map_err(|e| {
                SynapsedError::application("failed to process", &e.to_string())
            })
        }

        let result = outer_function();
        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            SynapsedError::Application { message, context } => {
                assert_eq!(message, "failed to process");
                assert!(context.contains("Storage error: disk error"));
            }
            _ => panic!("Expected Application error"),
        }
    }

    #[test]
    fn test_error_context_preservation() {
        let original_error = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Permission denied: /restricted/file.txt"
        );
        
        let synapsed_error: SynapsedError = original_error.into();
        let error_string = synapsed_error.to_string();
        
        assert!(error_string.contains("Permission denied"));
        assert!(error_string.contains("/restricted/file.txt"));
    }

    #[test]
    fn test_error_type_safety() {
        // Ensure error types maintain their identity through conversions
        let config_error = SynapsedError::config("test");
        let network_error = SynapsedError::network("test");
        
        // These should be different types even with same message
        assert_ne!(config_error, network_error);
        
        // Classification should be consistent
        assert!(config_error.is_server_error());
        assert!(!config_error.is_client_error());
        assert!(!config_error.is_retryable());
        
        assert!(!network_error.is_server_error());
        assert!(!network_error.is_client_error());
        assert!(network_error.is_retryable());
    }

    #[cfg(feature = "config")]
    #[test]
    fn test_config_error_conversion() {
        // This test only runs when the config feature is enabled
        let config_err = config::ConfigError::Message("test config error".to_string());
        let synapsed_err: SynapsedError = config_err.into();
        assert!(matches!(synapsed_err, SynapsedError::Configuration(_)));
    }

    #[test]
    fn test_toml_error_conversion() {
        let invalid_toml = "invalid toml [[[";
        let toml_err: toml::de::Error = toml::from_str::<toml::Value>(invalid_toml).unwrap_err();
        let synapsed_err: SynapsedError = toml_err.into();
        assert!(matches!(synapsed_err, SynapsedError::Configuration(_)));
    }
}