//! Property-based tests for error handling

use proptest::prelude::*;
use synapsed_core::error::{SynapsedError, SynapsedResult};
use crate::utils::generators::*;

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn test_error_display_never_empty(error in arb_synapsed_error()) {
            let display_str = error.to_string();
            prop_assert!(!display_str.is_empty());
            prop_assert!(display_str.len() > 5); // Should be more than just whitespace
        }

        #[test]
        fn test_error_debug_never_empty(error in arb_synapsed_error()) {
            let debug_str = format!("{:?}", error);
            prop_assert!(!debug_str.is_empty());
            prop_assert!(debug_str.len() > 5);
        }

        #[test]
        fn test_error_clone_equality(error in arb_synapsed_error()) {
            let cloned = error.clone();
            prop_assert_eq!(error, cloned);
        }

        #[test]
        fn test_error_classification_consistency(error in arb_synapsed_error()) {
            let is_client = error.is_client_error();
            let is_server = error.is_server_error();
            let is_retryable = error.is_retryable();

            // Client and server errors should be mutually exclusive
            prop_assert!(!(is_client && is_server));

            // Specific error types should have consistent classification
            match &error {
                SynapsedError::Network(_) | SynapsedError::Timeout(_) | SynapsedError::Internal(_) => {
                    prop_assert!(is_retryable);
                    prop_assert!(!is_client);
                }
                SynapsedError::InvalidInput(_) | SynapsedError::NotFound(_) | 
                SynapsedError::PermissionDenied(_) | SynapsedError::Authentication(_) => {
                    prop_assert!(is_client);
                    prop_assert!(!is_retryable);
                    prop_assert!(!is_server);
                }
                SynapsedError::Internal(_) | SynapsedError::Storage(_) | SynapsedError::Configuration(_) => {
                    prop_assert!(is_server || is_retryable); // Internal errors can be both
                }
                _ => {
                    // Other errors may have varying classification
                }
            }
        }

        #[test]
        fn test_error_result_type_consistency(
            success_value in any::<i32>(),
            error in arb_synapsed_error()
        ) {
            let success_result: SynapsedResult<i32> = Ok(success_value);
            prop_assert!(success_result.is_ok());
            prop_assert_eq!(success_result.unwrap(), success_value);

            let error_result: SynapsedResult<i32> = Err(error.clone());
            prop_assert!(error_result.is_err());
            prop_assert_eq!(error_result.unwrap_err(), error);
        }

        #[test]
        fn test_error_conversion_preserves_information(message in arb_string()) {
            // Test that error messages are preserved through conversions
            let original_error = SynapsedError::config(&message);
            let error_string = original_error.to_string();
            prop_assert!(error_string.contains(&message));

            let network_error = SynapsedError::network(&message);
            let error_string = network_error.to_string();
            prop_assert!(error_string.contains(&message));
        }

        #[test]
        fn test_application_error_context_preservation(
            message in arb_string(),
            context in arb_string()
        ) {
            let app_error = SynapsedError::application(&message, &context);
            let error_string = app_error.to_string();
            
            prop_assert!(error_string.contains(&message));
            prop_assert!(error_string.contains(&context));
            
            match app_error {
                SynapsedError::Application { message: msg, context: ctx } => {
                    prop_assert_eq!(msg, message);
                    prop_assert_eq!(ctx, context);
                }
                _ => prop_assert!(false, "Expected Application error variant"),
            }
        }

        #[test]
        fn test_error_chaining_preserves_causality(
            inner_message in arb_string(),
            outer_message in arb_string()
        ) {
            let inner_error = SynapsedError::storage(&inner_message);
            let chained_error = SynapsedError::application(&outer_message, &inner_error.to_string());
            
            let chained_string = chained_error.to_string();
            prop_assert!(chained_string.contains(&outer_message));
            prop_assert!(chained_string.contains(&inner_message));
        }

        #[test]
        fn test_error_type_stability(error in arb_synapsed_error()) {
            // Error properties should remain stable across multiple calls
            let is_retryable_1 = error.is_retryable();
            let is_retryable_2 = error.is_retryable();
            prop_assert_eq!(is_retryable_1, is_retryable_2);

            let is_client_1 = error.is_client_error();
            let is_client_2 = error.is_client_error();
            prop_assert_eq!(is_client_1, is_client_2);

            let is_server_1 = error.is_server_error();
            let is_server_2 = error.is_server_error();
            prop_assert_eq!(is_server_1, is_server_2);
        }

        #[test]
        fn test_error_serialization_roundtrip(message in arb_string()) {
            // Test that error information survives serialization/deserialization
            let original_error = SynapsedError::config(&message);
            let serialized = serde_json::to_string(&original_error).unwrap();
            let deserialized: SynapsedError = serde_json::from_str(&serialized).unwrap();
            
            prop_assert_eq!(original_error, deserialized);
            prop_assert_eq!(original_error.to_string(), deserialized.to_string());
        }

        #[test]
        fn test_error_variant_completeness(error in arb_synapsed_error()) {
            // Ensure all error variants are handled consistently
            let variant_name = match &error {
                SynapsedError::Configuration(_) => "Configuration",
                SynapsedError::Network(_) => "Network",
                SynapsedError::Cryptographic(_) => "Cryptographic",
                SynapsedError::Storage(_) => "Storage",
                SynapsedError::Authentication(_) => "Authentication",
                SynapsedError::InvalidInput(_) => "InvalidInput",
                SynapsedError::NotFound(_) => "NotFound",
                SynapsedError::PermissionDenied(_) => "PermissionDenied",
                SynapsedError::Timeout(_) => "Timeout",
                SynapsedError::Internal(_) => "Internal",
                SynapsedError::Serialization(_) => "Serialization",
                SynapsedError::Did(_) => "Did",
                SynapsedError::P2P(_) => "P2P",
                SynapsedError::Wasm(_) => "Wasm",
                SynapsedError::Payment(_) => "Payment",
                SynapsedError::Application { .. } => "Application",
            };

            let debug_str = format!("{:?}", error);
            prop_assert!(debug_str.contains(variant_name));
        }

        #[test]
        fn test_error_message_boundaries(
            short_msg in "[a-z]{1,5}",
            long_msg in "[a-z]{100,200}"
        ) {
            // Test error handling with very short and very long messages
            let short_error = SynapsedError::config(&short_msg);
            prop_assert!(!short_error.to_string().is_empty());
            prop_assert!(short_error.to_string().contains(&short_msg));

            let long_error = SynapsedError::config(&long_msg);
            prop_assert!(!long_error.to_string().is_empty());
            prop_assert!(long_error.to_string().contains(&long_msg));
        }

        #[test]
        fn test_error_unicode_handling(
            unicode_msg in "[\u{0080}-\u{00FF}]{10,20}"
        ) {
            // Test that errors handle Unicode characters properly
            let unicode_error = SynapsedError::config(&unicode_msg);
            let error_string = unicode_error.to_string();
            
            prop_assert!(!error_string.is_empty());
            prop_assert!(error_string.contains(&unicode_msg));
            
            // Ensure UTF-8 validity
            prop_assert!(error_string.is_ascii() || unicode_msg.chars().all(|c| c.is_alphabetic()));
        }
    }

    proptest! {
        // Test error result composition properties
        #[test]
        fn test_result_map_preserves_errors(
            value in any::<i32>(),
            error in arb_synapsed_error()
        ) {
            let success_result: SynapsedResult<i32> = Ok(value);
            let mapped_result = success_result.map(|v| v * 2);
            prop_assert!(mapped_result.is_ok());
            prop_assert_eq!(mapped_result.unwrap(), value * 2);

            let error_result: SynapsedResult<i32> = Err(error.clone());
            let mapped_error_result = error_result.map(|v| v * 2);
            prop_assert!(mapped_error_result.is_err());
            prop_assert_eq!(mapped_error_result.unwrap_err(), error);
        }

        #[test]
        fn test_result_and_then_short_circuits(
            value in any::<i32>(),
            error in arb_synapsed_error()
        ) {
            let error_result: SynapsedResult<i32> = Err(error.clone());
            let chained_result = error_result.and_then(|v| Ok(v.to_string()));
            
            prop_assert!(chained_result.is_err());
            prop_assert_eq!(chained_result.unwrap_err(), error);
        }

        #[test]
        fn test_result_or_else_recovers_from_errors(
            value in any::<i32>(),
            error in arb_synapsed_error(),
            recovery_value in any::<i32>()
        ) {
            let error_result: SynapsedResult<i32> = Err(error);
            let recovered_result = error_result.or_else(|_| Ok(recovery_value));
            
            prop_assert!(recovered_result.is_ok());
            prop_assert_eq!(recovered_result.unwrap(), recovery_value);

            let success_result: SynapsedResult<i32> = Ok(value);
            let not_recovered_result = success_result.or_else(|_| Ok(recovery_value));
            
            prop_assert!(not_recovered_result.is_ok());
            prop_assert_eq!(not_recovered_result.unwrap(), value);
        }
    }

    proptest! {
        // Test error context and metadata properties
        #[test]
        fn test_error_context_composition((base_error, overlay_error) in arb_error_scenario()) {
            let (scenario_type, error) = (base_error, overlay_error);
            
            // Create a composed error
            let composed = SynapsedError::application(
                "Operation failed",
                &format!("scenario: {}, error: {}", scenario_type, error)
            );
            
            let composed_string = composed.to_string();
            prop_assert!(composed_string.contains("Operation failed"));
            prop_assert!(composed_string.contains(&scenario_type));
            prop_assert!(composed_string.contains(&error.to_string()));
        }

        #[test]
        fn test_error_hierarchy_consistency(errors in prop::collection::vec(arb_synapsed_error(), 1..10)) {
            // Test that error hierarchies maintain consistent properties
            for error in &errors {
                let is_retryable = error.is_retryable();
                let is_client = error.is_client_error();
                let is_server = error.is_server_error();
                
                // Create a composed error
                let composed = SynapsedError::application("Batch operation failed", &error.to_string());
                
                // The composed error should not inherit retryability from inner error
                // (application errors are typically not retryable at the same level)
                prop_assert!(!composed.is_retryable() || is_retryable);
                
                // But it should preserve the error information
                prop_assert!(composed.to_string().contains(&error.to_string()));
            }
        }
    }

    // Standard unit tests for property test setup validation
    #[test]
    fn test_property_generators_work() {
        // Verify that our property generators actually produce valid data
        let mut runner = proptest::test_runner::TestRunner::default();
        
        runner.run(&arb_synapsed_error(), |error| {
            // Basic sanity checks
            assert!(!error.to_string().is_empty());
            assert!(!format!("{:?}", error).is_empty());
            
            // Clone equality
            assert_eq!(error, error.clone());
            
            Ok(())
        }).unwrap();
    }

    #[test]
    fn test_error_properties_edge_cases() {
        // Test some specific edge cases that might not be covered by property tests
        
        // Empty string message
        let empty_error = SynapsedError::config("");
        assert!(!empty_error.to_string().is_empty()); // Should still have error type info
        
        // Very long message
        let long_message = "a".repeat(10000);
        let long_error = SynapsedError::config(&long_message);
        assert!(long_error.to_string().contains(&long_message));
        
        // Application error with empty context
        let app_error = SynapsedError::application("message", "");
        match app_error {
            SynapsedError::Application { message, context } => {
                assert_eq!(message, "message");
                assert_eq!(context, "");
            }
            _ => panic!("Expected Application error"),
        }
    }
}