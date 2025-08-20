//! Integration tests for error handling across modules

use synapsed_core::error::{SynapsedError, SynapsedResult};
use crate::utils::TestEnvironment;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_error_propagation_across_modules() {
        // Test error propagation through different layers
        async fn service_layer() -> SynapsedResult<String> {
            data_layer().await.map_err(|e| {
                SynapsedError::application("Service layer failed", &e.to_string())
            })
        }

        async fn data_layer() -> SynapsedResult<String> {
            Err(SynapsedError::storage("Database connection failed"))
        }

        let result = service_layer().await;
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        let error_string = error.to_string();
        assert!(error_string.contains("Service layer failed"));
        assert!(error_string.contains("Database connection failed"));
    }

    #[test]
    fn test_error_classification_consistency() {
        let errors = vec![
            SynapsedError::config("config error"),
            SynapsedError::network("network error"),
            SynapsedError::invalid_input("validation error"),
            SynapsedError::internal("internal error"),
        ];

        for error in &errors {
            // Test that error classification is consistent
            let is_retryable = error.is_retryable();
            let is_client = error.is_client_error();
            let is_server = error.is_server_error();

            // Client and server should be mutually exclusive
            assert!(!(is_client && is_server));

            // Error should have a non-empty string representation
            assert!(!error.to_string().is_empty());
        }
    }
}