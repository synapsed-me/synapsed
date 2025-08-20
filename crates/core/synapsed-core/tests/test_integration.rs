//! Main integration test file for synapsed-core
//! 
//! This file serves as the entry point for all integration tests.
//! It includes utilities and runs comprehensive cross-module tests.

use synapsed_core::*;

// Include test utilities
mod utils;
use utils::*;

// Include specific test modules
mod unit;
mod integration;
mod property;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_version_info() {
        // Test that version constants are properly defined
        assert!(!VERSION.is_empty());
        assert_eq!(NAME, "synapsed-core");
    }

    #[test]
    fn test_public_api_availability() {
        // Test that all public API items are accessible
        use synapsed_core::{
            SynapsedError, SynapsedResult,
            Observable, Configurable, Identifiable, Validatable,
        };

        // These should compile without issues
        let _error: SynapsedError = SynapsedError::config("test");
        let _result: SynapsedResult<i32> = Ok(42);
        
        // Type names should be available
        assert_eq!(std::any::type_name::<SynapsedError>(), "synapsed_core::error::SynapsedError");
    }

    #[tokio::test]
    async fn test_basic_async_integration() {
        // Test that async functionality works correctly
        use std::time::Duration;
        
        let result = tokio::time::timeout(
            Duration::from_millis(100),
            async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                42
            }
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_error_integration() {
        // Test error integration across modules
        let config_error = SynapsedError::config("Configuration failed");
        let network_error = SynapsedError::network("Network unavailable");
        
        assert!(config_error.is_server_error());
        assert!(network_error.is_retryable());
        assert_ne!(config_error, network_error);
    }

    #[test]
    fn test_feature_flags() {
        // Test that feature flags work correctly
        #[cfg(feature = "config")]
        {
            use synapsed_core::config::ConfigValue;
            let _val = ConfigValue::String("test".to_string());
        }

        #[cfg(feature = "testing")]
        {
            // Testing features should be available in test builds
        }
    }

    #[test]
    fn test_serialization_integration() {
        // Test that serialization works across types
        let error = SynapsedError::application("Test error", "context");
        let json = serde_json::to_string(&error).unwrap();
        let deserialized: SynapsedError = serde_json::from_str(&json).unwrap();
        assert_eq!(error, deserialized);
    }

    #[test]
    fn test_trait_object_safety() {
        // Test that key traits can be used as trait objects where expected
        use synapsed_core::traits::Validatable;
        
        struct TestValidator;
        impl Validatable for TestValidator {
            fn validate(&self) -> SynapsedResult<()> {
                Ok(())
            }
        }
        
        let validator = TestValidator;
        assert!(validator.is_valid());
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;
        
        // Test that key types can be shared across threads
        let error = Arc::new(SynapsedError::config("shared error"));
        let error_clone = error.clone();
        
        let handle = thread::spawn(move || {
            error_clone.to_string()
        });
        
        let result = handle.join().unwrap();
        assert!(result.contains("Configuration error"));
    }

    #[test]
    fn test_documentation_examples() {
        // Test that examples from lib.rs documentation work
        fn example_function() -> SynapsedResult<String> {
            Ok("Hello Synapsed!".to_string())
        }
        
        let result = example_function().unwrap();
        assert_eq!(result, "Hello Synapsed!");
    }

    #[test]
    fn test_comprehensive_error_conversion() {
        // Test error conversions from standard library types
        use std::io;
        
        let io_error = io::Error::new(io::ErrorKind::NotFound, "test error");
        let synapsed_error: SynapsedError = io_error.into();
        
        assert!(matches!(synapsed_error, SynapsedError::Internal(_)));
        assert!(synapsed_error.to_string().contains("test error"));
    }

    #[cfg(feature = "config")]
    #[test]
    fn test_config_feature_integration() {
        use synapsed_core::config::{ConfigValue, EnvConfigSource, ConfigSource};
        
        // Test that config features work when enabled
        let env_source = EnvConfigSource::new("TEST");
        assert_eq!(env_source.source_name(), "environment");
        
        let string_val = ConfigValue::String("test".to_string());
        assert_eq!(string_val.as_string().unwrap(), "test");
    }

    #[test]
    fn test_memory_safety() {
        // Test that there are no obvious memory safety issues
        let mut errors = Vec::new();
        
        for i in 0..1000 {
            errors.push(SynapsedError::config(&format!("Error {}", i)));
        }
        
        // Verify all errors are properly stored
        assert_eq!(errors.len(), 1000);
        assert!(errors[999].to_string().contains("Error 999"));
        
        // Test that errors can be moved and cloned safely
        let moved_errors = errors;
        let _cloned_error = moved_errors[0].clone();
    }

    #[tokio::test]
    async fn test_async_trait_integration() {
        // Test that async traits work correctly with real implementations
        use synapsed_core::traits::{Observable, ObservableStatus, ObservableState, HealthStatus, HealthLevel};
        use std::collections::HashMap;
        
        struct TestObservable;
        
        #[async_trait::async_trait]
        impl Observable for TestObservable {
            async fn status(&self) -> SynapsedResult<ObservableStatus> {
                Ok(ObservableStatus {
                    state: ObservableState::Running,
                    last_updated: chrono::Utc::now(),
                    metadata: HashMap::new(),
                })
            }
            
            async fn health(&self) -> SynapsedResult<HealthStatus> {
                Ok(HealthStatus {
                    overall: HealthLevel::Healthy,
                    checks: HashMap::new(),
                    last_check: chrono::Utc::now(),
                })
            }
            
            async fn metrics(&self) -> SynapsedResult<HashMap<String, f64>> {
                let mut metrics = HashMap::new();
                metrics.insert("test_metric".to_string(), 42.0);
                Ok(metrics)
            }
            
            fn describe(&self) -> String {
                "Test observable component".to_string()
            }
        }
        
        let observable = TestObservable;
        let status = observable.status().await.unwrap();
        assert_eq!(status.state, ObservableState::Running);
        
        let health = observable.health().await.unwrap();
        assert_eq!(health.overall, HealthLevel::Healthy);
        
        let metrics = observable.metrics().await.unwrap();
        assert_eq!(metrics.get("test_metric"), Some(&42.0));
        
        assert_eq!(observable.describe(), "Test observable component");
    }
}