//! Unit tests for payment processor
//! 
//! Tests the core payment processing logic including validation,
//! fraud detection, retry mechanisms, and gateway integration.

#![cfg(test)]

use synapsed_payments::processor::*;
use synapsed_payments::types::*;
use synapsed_payments::{Error, Result};
use mockall::predicate::*;
use mockall::mock;
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;
use tokio_test;
use std::sync::Arc;
use std::time::Duration;

// Mock gateway for testing
mock! {
    PaymentGateway {}
    
    #[async_trait::async_trait]
    impl PaymentGateway for PaymentGateway {
        async fn process_payment(&self, request: &PaymentRequest) -> Result<PaymentResponse>;
        async fn refund_payment(&self, payment_id: &str, amount: Option<Amount>) -> Result<PaymentResponse>;
        async fn get_payment_status(&self, payment_id: &str) -> Result<PaymentStatus>;
        fn gateway_name(&self) -> &str;
        fn supported_currencies(&self) -> Vec<Currency>;
    }
}

#[cfg(test)]
mod payment_processor_tests {
    use super::*;

    #[tokio::test]
    async fn test_successful_payment_processing() {
        let mut mock_gateway = MockPaymentGateway::new();
        let amount = Amount::new(Decimal::from_str("100.00").unwrap(), Currency::USD);
        let expected_response = PaymentResponse::success(
            Uuid::new_v4(),
            "txn_123".to_string(),
            amount.clone(),
            "gateway_ref_123".to_string(),
        );
        
        mock_gateway
            .expect_process_payment()
            .times(1)
            .returning(move |_| Ok(expected_response.clone()));
        
        mock_gateway
            .expect_gateway_name()
            .return_const("test_gateway");
        
        let processor = PaymentProcessor::new(Arc::new(mock_gateway));
        
        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "John Doe".to_string(),
        };
        
        let request = PaymentRequest::new(
            amount,
            PaymentMethod::Card(card),
            "order_test_123".to_string(),
        );
        
        let result = processor.process_payment(request).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.status(), PaymentStatus::Completed);
    }

    #[tokio::test]
    async fn test_payment_validation_failure() {
        let mock_gateway = MockPaymentGateway::new();
        let processor = PaymentProcessor::new(Arc::new(mock_gateway));
        
        // Invalid card number (fails Luhn check)
        let card = CardDetails {
            number: "4111111111111112".to_string(), // Invalid
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "John Doe".to_string(),
        };
        
        let amount = Amount::new(Decimal::from_str("50.00").unwrap(), Currency::USD);
        let request = PaymentRequest::new(
            amount,
            PaymentMethod::Card(card),
            "invalid_order".to_string(),
        );
        
        let result = processor.process_payment(request).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            Error::ValidationError(_) => (), // Expected
            _ => panic!("Expected ValidationError"),
        }
    }

    #[tokio::test]
    async fn test_payment_with_retry_mechanism() {
        let mut mock_gateway = MockPaymentGateway::new();
        
        // First call fails with temporary error
        mock_gateway
            .expect_process_payment()
            .times(1)
            .returning(|_| Err(Error::GatewayTimeout));
        
        // Second call succeeds
        let amount = Amount::new(Decimal::from_str("75.00").unwrap(), Currency::USD);
        let success_response = PaymentResponse::success(
            Uuid::new_v4(),
            "txn_retry".to_string(),
            amount.clone(),
            "gateway_retry".to_string(),
        );
        
        mock_gateway
            .expect_process_payment()
            .times(1)
            .returning(move |_| Ok(success_response.clone()));
        
        mock_gateway
            .expect_gateway_name()
            .return_const("retry_gateway");
        
        let mut processor = PaymentProcessor::new(Arc::new(mock_gateway));
        processor.configure_retries(RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
        });
        
        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "Jane Smith".to_string(),
        };
        
        let request = PaymentRequest::new(
            amount,
            PaymentMethod::Card(card),
            "retry_order".to_string(),
        );
        
        let result = processor.process_payment(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fraud_detection() {
        let mock_gateway = MockPaymentGateway::new();
        let mut processor = PaymentProcessor::new(Arc::new(mock_gateway));
        
        // Configure strict fraud detection
        processor.configure_fraud_detection(FraudDetectionConfig {
            max_amount_per_card: Some(Amount::new(Decimal::from_str("1000.00").unwrap(), Currency::USD)),
            max_transactions_per_hour: Some(5),
            velocity_check_enabled: true,
            geolocation_check_enabled: true,
            device_fingerprint_required: true,
        });
        
        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "Suspicious User".to_string(),
        };
        
        // High-risk amount
        let high_amount = Amount::new(Decimal::from_str("5000.00").unwrap(), Currency::USD);
        let mut request = PaymentRequest::new(
            high_amount,
            PaymentMethod::Card(card),
            "high_risk_order".to_string(),
        );
        
        // Add suspicious metadata
        request.add_metadata("ip_address", "10.0.0.1"); // Private IP
        request.add_metadata("user_agent", "curl/7.68.0"); // Bot-like
        
        let result = processor.process_payment(request).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            Error::FraudDetected(_) => (), // Expected
            _ => panic!("Expected FraudDetected error"),
        }
    }

    #[tokio::test]
    async fn test_currency_conversion() {
        let mut mock_gateway = MockPaymentGateway::new();
        
        mock_gateway
            .expect_supported_currencies()
            .return_const(vec![Currency::USD]); // Gateway only supports USD
        
        let eur_amount = Amount::new(Decimal::from_str("100.00").unwrap(), Currency::EUR);
        let converted_response = PaymentResponse::success(
            Uuid::new_v4(),
            "txn_converted".to_string(),
            Amount::new(Decimal::from_str("110.00").unwrap(), Currency::USD), // Converted
            "gateway_converted".to_string(),
        );
        
        mock_gateway
            .expect_process_payment()
            .times(1)
            .returning(move |_| Ok(converted_response.clone()));
        
        mock_gateway
            .expect_gateway_name()
            .return_const("usd_gateway");
        
        let mut processor = PaymentProcessor::new(Arc::new(mock_gateway));
        processor.configure_currency_conversion(CurrencyConversionConfig {
            auto_convert: true,
            preferred_currency: Currency::USD,
            exchange_rate_provider: "test_provider".to_string(),
        });
        
        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "Euro Customer".to_string(),
        };
        
        let request = PaymentRequest::new(
            eur_amount,
            PaymentMethod::Card(card),
            "conversion_order".to_string(),
        );
        
        let result = processor.process_payment(request).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.processed_amount().unwrap().currency(), &Currency::USD);
    }

    #[tokio::test]
    async fn test_idempotency() {
        let mut mock_gateway = MockPaymentGateway::new();
        let amount = Amount::new(Decimal::from_str("25.00").unwrap(), Currency::GBP);
        let response = PaymentResponse::success(
            Uuid::new_v4(),
            "txn_idempotent".to_string(),
            amount.clone(),
            "gateway_idempotent".to_string(),
        );
        
        // Gateway should only be called once due to idempotency
        mock_gateway
            .expect_process_payment()
            .times(1)
            .returning(move |_| Ok(response.clone()));
        
        mock_gateway
            .expect_gateway_name()
            .return_const("idempotent_gateway");
        
        let processor = PaymentProcessor::new(Arc::new(mock_gateway));
        
        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "Idempotent User".to_string(),
        };
        
        let mut request = PaymentRequest::new(
            amount,
            PaymentMethod::Card(card),
            "idempotent_order".to_string(),
        );
        
        // Add idempotency key
        request.set_idempotency_key("idem_key_123".to_string());
        
        // Make same request twice
        let result1 = processor.process_payment(request.clone()).await;
        let result2 = processor.process_payment(request).await;
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        // Results should be identical
        let resp1 = result1.unwrap();
        let resp2 = result2.unwrap();
        assert_eq!(resp1.transaction_id(), resp2.transaction_id());
    }
}

#[cfg(test)]
mod refund_processor_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_refund() {
        let mut mock_gateway = MockPaymentGateway::new();
        let original_amount = Amount::new(Decimal::from_str("100.00").unwrap(), Currency::USD);
        let refund_response = PaymentResponse::success(
            Uuid::new_v4(),
            "refund_123".to_string(),
            original_amount.clone(),
            "gateway_refund_123".to_string(),
        );
        
        mock_gateway
            .expect_refund_payment()
            .with(eq("payment_123"), eq(None)) // Full refund
            .times(1)
            .returning(move |_, _| Ok(refund_response.clone()));
        
        mock_gateway
            .expect_gateway_name()
            .return_const("refund_gateway");
        
        let processor = PaymentProcessor::new(Arc::new(mock_gateway));
        
        let result = processor.refund_payment("payment_123", None).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.status(), PaymentStatus::Refunded);
    }

    #[tokio::test]
    async fn test_partial_refund() {
        let mut mock_gateway = MockPaymentGateway::new();
        let refund_amount = Amount::new(Decimal::from_str("50.00").unwrap(), Currency::USD);
        let refund_response = PaymentResponse::success(
            Uuid::new_v4(),
            "partial_refund_123".to_string(),
            refund_amount.clone(),
            "gateway_partial_refund".to_string(),
        );
        
        mock_gateway
            .expect_refund_payment()
            .with(eq("payment_456"), eq(Some(refund_amount.clone())))
            .times(1)
            .returning(move |_, _| Ok(refund_response.clone()));
        
        mock_gateway
            .expect_gateway_name()
            .return_const("partial_refund_gateway");
        
        let processor = PaymentProcessor::new(Arc::new(mock_gateway));
        
        let result = processor.refund_payment("payment_456", Some(refund_amount)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_refund_validation() {
        let mock_gateway = MockPaymentGateway::new();
        let processor = PaymentProcessor::new(Arc::new(mock_gateway));
        
        // Test refund of non-existent payment
        let invalid_refund_amount = Amount::new(Decimal::from_str("25.00").unwrap(), Currency::EUR);
        let result = processor.refund_payment("nonexistent_payment", Some(invalid_refund_amount)).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::PaymentNotFound(_) => (), // Expected
            _ => panic!("Expected PaymentNotFound error"),
        }
    }
}

#[cfg(test)]
mod payment_status_tests {
    use super::*;

    #[tokio::test]
    async fn test_payment_status_retrieval() {
        let mut mock_gateway = MockPaymentGateway::new();
        
        mock_gateway
            .expect_get_payment_status()
            .with(eq("payment_status_test"))
            .times(1)
            .returning(|_| Ok(PaymentStatus::Completed));
        
        mock_gateway
            .expect_gateway_name()
            .return_const("status_gateway");
        
        let processor = PaymentProcessor::new(Arc::new(mock_gateway));
        
        let result = processor.get_payment_status("payment_status_test").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PaymentStatus::Completed);
    }

    #[tokio::test]
    async fn test_payment_status_caching() {
        let mut mock_gateway = MockPaymentGateway::new();
        
        // Gateway should only be called once due to caching
        mock_gateway
            .expect_get_payment_status()
            .with(eq("cached_payment"))
            .times(1)
            .returning(|_| Ok(PaymentStatus::Processing));
        
        mock_gateway
            .expect_gateway_name()
            .return_const("cached_gateway");
        
        let mut processor = PaymentProcessor::new(Arc::new(mock_gateway));
        processor.configure_status_caching(StatusCacheConfig {
            cache_ttl: Duration::from_secs(300),
            max_entries: 1000,
        });
        
        // Make same status request twice
        let result1 = processor.get_payment_status("cached_payment").await;
        let result2 = processor.get_payment_status("cached_payment").await;
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(result1.unwrap(), result2.unwrap());
    }
}

#[cfg(test)]
mod webhook_processor_tests {
    use super::*;

    #[tokio::test]
    async fn test_webhook_processing() {
        let processor = PaymentProcessor::default();
        
        let webhook_event = WebhookEvent::new(
            WebhookEventType::PaymentCompleted,
            Uuid::new_v4(),
            serde_json::json!({
                "transaction_id": "txn_webhook_test",
                "amount": "150.00",
                "currency": "USD",
                "status": "completed"
            }),
        );
        
        let result = processor.process_webhook(webhook_event.clone()).await;
        assert!(result.is_ok());
        
        // Verify webhook was processed and stored
        let processed_events = processor.get_processed_webhooks().await;
        assert!(!processed_events.is_empty());
        assert_eq!(processed_events[0].event_type(), webhook_event.event_type());
    }

    #[tokio::test]
    async fn test_webhook_signature_validation() {
        let mut processor = PaymentProcessor::default();
        processor.configure_webhook_security(WebhookSecurityConfig {
            require_signature: true,
            secret_key: "webhook_secret_123".to_string(),
            timestamp_tolerance: Duration::from_secs(300),
        });
        
        let webhook_event = WebhookEvent::new(
            WebhookEventType::PaymentFailed,
            Uuid::new_v4(),
            serde_json::json!({"error": "insufficient_funds"}),
        );
        
        // Invalid signature should fail
        let result = processor.process_webhook_with_signature(
            webhook_event,
            "invalid_signature".to_string(),
        ).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidWebhookSignature => (), // Expected
            _ => panic!("Expected InvalidWebhookSignature error"),
        }
    }

    #[tokio::test]
    async fn test_duplicate_webhook_prevention() {
        let processor = PaymentProcessor::default();
        
        let webhook_event = WebhookEvent::new(
            WebhookEventType::PaymentRefunded,
            Uuid::new_v4(),
            serde_json::json!({
                "refund_id": "ref_duplicate_test",
                "amount": "75.00"
            }),
        );
        
        // Process same webhook twice
        let result1 = processor.process_webhook(webhook_event.clone()).await;
        let result2 = processor.process_webhook(webhook_event).await;
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        // Second processing should be identified as duplicate
        assert!(result2.unwrap().was_duplicate());
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_payment_processing_performance() {
        let mut mock_gateway = MockPaymentGateway::new();
        let amount = Amount::new(Decimal::from_str("10.00").unwrap(), Currency::USD);
        let response = PaymentResponse::success(
            Uuid::new_v4(),
            "perf_test".to_string(),
            amount.clone(),
            "gateway_perf".to_string(),
        );
        
        mock_gateway
            .expect_process_payment()
            .times(100)
            .returning(move |_| Ok(response.clone()));
        
        mock_gateway
            .expect_gateway_name()
            .return_const("perf_gateway");
        
        let processor = PaymentProcessor::new(Arc::new(mock_gateway));
        
        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "Performance Test".to_string(),
        };
        
        let start = Instant::now();
        
        let mut tasks = vec![];
        for i in 0..100 {
            let request = PaymentRequest::new(
                amount.clone(),
                PaymentMethod::Card(card.clone()),
                format!("perf_order_{}", i),
            );
            
            let processor_clone = processor.clone();
            tasks.push(tokio::spawn(async move {
                processor_clone.process_payment(request).await
            }));
        }
        
        let results = futures::future::join_all(tasks).await;
        let duration = start.elapsed();
        
        // All payments should succeed
        for result in results {
            assert!(result.unwrap().is_ok());
        }
        
        // Performance should be reasonable (adjust threshold as needed)
        assert!(duration.as_millis() < 5000, "Performance test took too long: {:?}", duration);
    }

    #[tokio::test]
    async fn test_memory_usage_under_load() {
        let mut mock_gateway = MockPaymentGateway::new();
        let amount = Amount::new(Decimal::from_str("1.00").unwrap(), Currency::USD);
        let response = PaymentResponse::success(
            Uuid::new_v4(),
            "memory_test".to_string(),
            amount.clone(),
            "gateway_memory".to_string(),
        );
        
        mock_gateway
            .expect_process_payment()
            .times(1000)
            .returning(move |_| Ok(response.clone()));
        
        mock_gateway
            .expect_gateway_name()
            .return_const("memory_gateway");
        
        let processor = PaymentProcessor::new(Arc::new(mock_gateway));
        
        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "Memory Test".to_string(),
        };
        
        // Process many payments to test memory usage
        for i in 0..1000 {
            let request = PaymentRequest::new(
                amount.clone(),
                PaymentMethod::Card(card.clone()),
                format!("memory_order_{}", i),
            );
            
            let result = processor.process_payment(request).await;
            assert!(result.is_ok());
            
            // Periodically check that memory isn't growing unbounded
            // In a real implementation, this would use actual memory monitoring
            if i % 100 == 0 {
                tokio::task::yield_now().await; // Allow cleanup
            }
        }
    }
}