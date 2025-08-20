//! Integration tests for synapsed-payments
//! 
//! Full end-to-end testing of payment flows including gateway integration,
//! database persistence, webhook handling, and real-world scenarios.

#![cfg(test)]

use synapsed_payments::*;
use synapsed_payments::types::*;
use synapsed_payments::processor::*;
use synapsed_payments::gateways::*;
use synapsed_payments::storage::*;
use synapsed_payments::webhooks::*;

use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;
use tokio_test;
use std::sync::Arc;
use std::time::Duration;
use serde_json;

/// Test helper to create a test payment processor with in-memory storage
async fn create_test_processor() -> PaymentProcessor {
    let storage = Arc::new(InMemoryPaymentStorage::new());
    let gateway = Arc::new(TestPaymentGateway::new());
    
    PaymentProcessor::builder()
        .with_storage(storage)
        .with_gateway(gateway)
        .with_webhook_handler(Arc::new(InMemoryWebhookHandler::new()))
        .build()
        .await
        .expect("Failed to create test processor")
}

/// Test helper to create valid payment request
fn create_test_payment_request(amount: Decimal, currency: Currency) -> PaymentRequest {
    let card = CardDetails {
        number: "4111111111111111".to_string(),
        expiry_month: 12,
        expiry_year: 2025,
        cvv: "123".to_string(),
        holder_name: "Test User".to_string(),
    };
    
    PaymentRequest::new(
        Amount::new(amount, currency),
        PaymentMethod::Card(card),
        format!("order_{}", Uuid::new_v4()),
    )
}

#[cfg(test)]
mod end_to_end_payment_tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_payment_flow() {
        let processor = create_test_processor().await;
        let request = create_test_payment_request(
            Decimal::from_str("99.99").unwrap(),
            Currency::USD
        );
        
        // Step 1: Process payment
        let response = processor.process_payment(request.clone()).await;
        assert!(response.is_ok(), "Payment processing failed: {:?}", response.err());
        
        let payment_response = response.unwrap();
        assert_eq!(payment_response.status(), PaymentStatus::Completed);
        assert!(payment_response.transaction_id().is_some());
        
        let transaction_id = payment_response.transaction_id().unwrap();
        
        // Step 2: Verify payment is stored
        let stored_payment = processor.get_payment_by_id(transaction_id).await;
        assert!(stored_payment.is_ok());
        
        let payment = stored_payment.unwrap();
        assert_eq!(payment.amount().value(), request.amount().value());
        assert_eq!(payment.status(), PaymentStatus::Completed);
        
        // Step 3: Check payment history
        let history = processor.get_payment_history(transaction_id).await;
        assert!(history.is_ok());
        
        let events = history.unwrap();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.status() == PaymentStatus::Processing));
        assert!(events.iter().any(|e| e.status() == PaymentStatus::Completed));
    }

    #[tokio::test]
    async fn test_payment_with_refund_flow() {
        let processor = create_test_processor().await;
        let request = create_test_payment_request(
            Decimal::from_str("150.00").unwrap(),
            Currency::EUR
        );
        
        // Step 1: Process original payment
        let payment_response = processor.process_payment(request).await.unwrap();
        let transaction_id = payment_response.transaction_id().unwrap();
        
        // Step 2: Process full refund
        let refund_response = processor.refund_payment(transaction_id, None).await;
        assert!(refund_response.is_ok());
        
        let refund = refund_response.unwrap();
        assert_eq!(refund.status(), PaymentStatus::Refunded);
        
        // Step 3: Verify original payment status updated
        let updated_payment = processor.get_payment_by_id(transaction_id).await.unwrap();
        assert_eq!(updated_payment.status(), PaymentStatus::Refunded);
        
        // Step 4: Verify refund is stored separately
        let refund_id = refund.transaction_id().unwrap();
        let stored_refund = processor.get_payment_by_id(refund_id).await.unwrap();
        assert_eq!(stored_refund.payment_type(), PaymentType::Refund);
        assert_eq!(stored_refund.parent_payment_id(), Some(transaction_id));
    }

    #[tokio::test]
    async fn test_partial_refund_flow() {
        let processor = create_test_processor().await;
        let original_amount = Decimal::from_str("200.00").unwrap();
        let request = create_test_payment_request(original_amount, Currency::GBP);
        
        // Step 1: Process original payment
        let payment_response = processor.process_payment(request).await.unwrap();
        let transaction_id = payment_response.transaction_id().unwrap();
        
        // Step 2: Process partial refund (50%)
        let partial_amount = Amount::new(
            Decimal::from_str("100.00").unwrap(),
            Currency::GBP
        );
        let refund_response = processor.refund_payment(transaction_id, Some(partial_amount)).await;
        assert!(refund_response.is_ok());
        
        // Step 3: Verify original payment status (should still be completed, not refunded)
        let updated_payment = processor.get_payment_by_id(transaction_id).await.unwrap();
        assert_eq!(updated_payment.status(), PaymentStatus::PartiallyRefunded);
        
        // Step 4: Verify refund amount
        let refund = refund_response.unwrap();
        assert_eq!(refund.processed_amount().unwrap().value(), &Decimal::from_str("100.00").unwrap());
        
        // Step 5: Process second partial refund for remaining amount
        let remaining_amount = Amount::new(
            Decimal::from_str("100.00").unwrap(),
            Currency::GBP
        );
        let second_refund = processor.refund_payment(transaction_id, Some(remaining_amount)).await;
        assert!(second_refund.is_ok());
        
        // Step 6: Verify original payment is now fully refunded
        let final_payment = processor.get_payment_by_id(transaction_id).await.unwrap();
        assert_eq!(final_payment.status(), PaymentStatus::Refunded);
    }

    #[tokio::test]
    async fn test_failed_payment_flow() {
        let processor = create_test_processor().await;
        
        // Create payment request that will fail (invalid card)
        let card = CardDetails {
            number: "4000000000000002".to_string(), // Test card for declined transactions
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "Declined Card".to_string(),
        };
        
        let request = PaymentRequest::new(
            Amount::new(Decimal::from_str("50.00").unwrap(), Currency::USD),
            PaymentMethod::Card(card),
            "failed_order".to_string(),
        );
        
        // Process payment (should fail)
        let response = processor.process_payment(request).await;
        assert!(response.is_err());
        
        match response.unwrap_err() {
            Error::PaymentDeclined(_) => (), // Expected
            other => panic!("Expected PaymentDeclined error, got: {:?}", other),
        }
        
        // Verify failed payment is stored with correct status
        let failed_payments = processor.get_failed_payments(
            chrono::Utc::now() - chrono::Duration::hours(1),
            chrono::Utc::now()
        ).await.unwrap();
        
        assert!(!failed_payments.is_empty());
        assert!(failed_payments.iter().any(|p| p.status() == PaymentStatus::Failed));
    }
}

#[cfg(test)]
mod webhook_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_webhook_payment_completion() {
        let processor = create_test_processor().await;
        let request = create_test_payment_request(
            Decimal::from_str("75.00").unwrap(),
            Currency::USD
        );
        
        // Process payment
        let payment_response = processor.process_payment(request).await.unwrap();
        let payment_id = payment_response.payment_id();
        
        // Simulate webhook from gateway
        let webhook_event = WebhookEvent::new(
            WebhookEventType::PaymentCompleted,
            payment_id,
            serde_json::json!({
                "transaction_id": payment_response.transaction_id(),
                "gateway_reference": payment_response.gateway_reference(),
                "amount": "75.00",
                "currency": "USD",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        );
        
        // Process webhook
        let webhook_result = processor.process_webhook(webhook_event).await;
        assert!(webhook_result.is_ok());
        
        // Verify webhook was processed and payment updated
        let updated_payment = processor.get_payment_by_id(payment_response.transaction_id().unwrap()).await.unwrap();
        assert!(updated_payment.webhook_confirmed());
    }

    #[tokio::test]
    async fn test_webhook_payment_failure() {
        let processor = create_test_processor().await;
        let payment_id = Uuid::new_v4();
        
        // Simulate webhook for failed payment
        let webhook_event = WebhookEvent::new(
            WebhookEventType::PaymentFailed,
            payment_id,
            serde_json::json!({
                "payment_id": payment_id.to_string(),
                "error_code": "insufficient_funds",
                "error_message": "Insufficient funds in account",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        );
        
        // Process webhook  
        let webhook_result = processor.process_webhook(webhook_event).await;
        assert!(webhook_result.is_ok());
        
        // Verify webhook processing created failure record
        let webhook_logs = processor.get_webhook_logs(
            chrono::Utc::now() - chrono::Duration::minutes(5),
            chrono::Utc::now()
        ).await.unwrap();
        
        assert!(!webhook_logs.is_empty());
        assert!(webhook_logs.iter().any(|log| 
            log.event_type() == WebhookEventType::PaymentFailed
        ));
    }

    #[tokio::test]
    async fn test_webhook_signature_validation() {
        let mut processor = create_test_processor().await;
        processor.configure_webhook_security(WebhookSecurityConfig {
            require_signature: true,
            secret_key: "test_webhook_secret".to_string(),
            timestamp_tolerance: Duration::from_secs(300),
        });
        
        let webhook_event = WebhookEvent::new(
            WebhookEventType::PaymentCompleted,
            Uuid::new_v4(),
            serde_json::json!({"test": "data"}),
        );
        
        // Generate valid signature
        let valid_signature = webhook_event.generate_signature("test_webhook_secret");
        
        // Process with valid signature
        let valid_result = processor.process_webhook_with_signature(
            webhook_event.clone(),
            valid_signature
        ).await;
        assert!(valid_result.is_ok());
        
        // Process with invalid signature
        let invalid_result = processor.process_webhook_with_signature(
            webhook_event,
            "invalid_signature".to_string()
        ).await;
        assert!(invalid_result.is_err());
    }
}

#[cfg(test)]
mod multi_currency_tests {
    use super::*;

    #[tokio::test]
    async fn test_multi_currency_payments() {
        let processor = create_test_processor().await;
        
        let currencies = vec![
            (Currency::USD, "100.00"),
            (Currency::EUR, "85.50"),
            (Currency::GBP, "75.25"),
            (Currency::JPY, "11000"),
        ];
        
        let mut payment_ids = vec![];
        
        // Process payments in different currencies
        for (currency, amount_str) in currencies {
            let amount = Decimal::from_str(amount_str).unwrap();
            let request = create_test_payment_request(amount, currency);
            
            let response = processor.process_payment(request).await;
            assert!(response.is_ok(), "Failed to process {} payment", currency.code());
            
            let payment_response = response.unwrap();
            assert_eq!(payment_response.processed_amount().unwrap().currency(), &currency);
            
            payment_ids.push(payment_response.transaction_id().unwrap());
        }
        
        // Verify all payments were stored correctly
        for payment_id in payment_ids {
            let payment = processor.get_payment_by_id(payment_id).await;
            assert!(payment.is_ok());
            assert_eq!(payment.unwrap().status(), PaymentStatus::Completed);
        }
    }

    #[tokio::test]
    async fn test_currency_conversion_in_processing() {
        let mut processor = create_test_processor().await;
        
        // Configure processor to convert EUR to USD
        processor.configure_currency_conversion(CurrencyConversionConfig {
            auto_convert: true,
            preferred_currency: Currency::USD,
            exchange_rate_provider: "test_provider".to_string(),
        });
        
        // Process EUR payment
        let eur_request = create_test_payment_request(
            Decimal::from_str("100.00").unwrap(),
            Currency::EUR
        );
        
        let response = processor.process_payment(eur_request).await;
        assert!(response.is_ok());
        
        let payment_response = response.unwrap();
        
        // Verify original amount is preserved
        assert_eq!(payment_response.requested_amount().currency(), &Currency::EUR);
        
        // Verify processed amount is in USD (converted)
        assert_eq!(payment_response.processed_amount().unwrap().currency(), &Currency::USD);
        
        // Verify conversion rate was applied
        let conversion_rate = payment_response.conversion_rate().unwrap();
        assert!(conversion_rate > Decimal::ZERO);
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_gateway_timeout_handling() {
        let mut processor = create_test_processor().await;
        
        // Configure short timeout for testing
        processor.configure_timeouts(TimeoutConfig {
            payment_timeout: Duration::from_millis(100),
            gateway_timeout: Duration::from_millis(50),
            retry_timeout: Duration::from_millis(200),
        });
        
        // Configure the test gateway to simulate timeout
        let slow_gateway = Arc::new(SlowTestGateway::new(Duration::from_millis(200)));
        processor.set_gateway(slow_gateway);
        
        let request = create_test_payment_request(
            Decimal::from_str("25.00").unwrap(),
            Currency::USD
        );
        
        let result = processor.process_payment(request).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            Error::GatewayTimeout => (), // Expected
            other => panic!("Expected GatewayTimeout, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_network_error_recovery() {
        let mut processor = create_test_processor().await;
        
        // Configure retry policy
        processor.configure_retries(RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
        });
        
        // Configure gateway that fails first two attempts, succeeds on third
        let unreliable_gateway = Arc::new(UnreliableTestGateway::new(2)); // Fail first 2 attempts
        processor.set_gateway(unreliable_gateway);
        
        let request = create_test_payment_request(
            Decimal::from_str("30.00").unwrap(),
            Currency::USD
        );
        
        let result = processor.process_payment(request).await;
        assert!(result.is_ok(), "Payment should succeed after retries");
        
        let response = result.unwrap();
        assert_eq!(response.status(), PaymentStatus::Completed);
        assert_eq!(response.retry_count(), Some(2)); // Should show retry attempts
    }

    #[tokio::test]
    async fn test_database_error_handling() {
        let failing_storage = Arc::new(FailingPaymentStorage::new());
        let gateway = Arc::new(TestPaymentGateway::new());
        
        let processor = PaymentProcessor::builder()
            .with_storage(failing_storage)
            .with_gateway(gateway)
            .build()
            .await
            .expect("Failed to create processor");
        
        let request = create_test_payment_request(
            Decimal::from_str("40.00").unwrap(),
            Currency::USD
        );
        
        let result = processor.process_payment(request).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            Error::StorageError(_) => (), // Expected
            other => panic!("Expected StorageError, got: {:?}", other),
        }
    }
}

#[cfg(test)]
mod performance_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_payment_processing() {
        let processor = Arc::new(create_test_processor().await);
        let num_payments = 50;
        let payment_amount = Decimal::from_str("10.00").unwrap();
        
        let mut tasks = vec![];
        
        for i in 0..num_payments {
            let processor_clone = Arc::clone(&processor);
            let task = tokio::spawn(async move {
                let request = create_test_payment_request(payment_amount, Currency::USD);
                processor_clone.process_payment(request).await
            });
            tasks.push(task);
        }
        
        let results = futures::future::join_all(tasks).await;
        
        // Verify all payments succeeded
        let mut successful_payments = 0;
        for result in results {
            match result.unwrap() {
                Ok(response) => {
                    assert_eq!(response.status(), PaymentStatus::Completed);
                    successful_payments += 1;
                },
                Err(e) => panic!("Payment failed: {:?}", e),
            }
        }
        
        assert_eq!(successful_payments, num_payments);
        
        // Verify all payments were stored
        let recent_payments = processor.get_recent_payments(
            chrono::Utc::now() - chrono::Duration::minutes(5),
            chrono::Utc::now()
        ).await.unwrap();
        
        assert_eq!(recent_payments.len(), num_payments);
    }

    #[tokio::test]
    async fn test_high_volume_webhook_processing() {
        let processor = Arc::new(create_test_processor().await);
        let num_webhooks = 100;
        
        let mut tasks = vec![];
        
        for i in 0..num_webhooks {
            let processor_clone = Arc::clone(&processor);
            let task = tokio::spawn(async move {
                let webhook = WebhookEvent::new(
                    if i % 2 == 0 { WebhookEventType::PaymentCompleted } else { WebhookEventType::PaymentFailed },
                    Uuid::new_v4(),
                    serde_json::json!({
                        "webhook_id": i,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    }),
                );
                processor_clone.process_webhook(webhook).await
            });
            tasks.push(task);
        }
        
        let results = futures::future::join_all(tasks).await;
        
        // Verify all webhooks were processed successfully
        for result in results {
            assert!(result.unwrap().is_ok(), "Webhook processing failed");
        }
        
        // Verify webhook processing statistics
        let webhook_stats = processor.get_webhook_statistics(
            chrono::Utc::now() - chrono::Duration::minutes(5),
            chrono::Utc::now()
        ).await.unwrap();
        
        assert_eq!(webhook_stats.total_processed, num_webhooks);
        assert_eq!(webhook_stats.failed_count, 0);
    }
}

#[cfg(test)]
mod security_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_fraud_detection_integration() {
        let mut processor = create_test_processor().await;
        
        // Configure strict fraud detection
        processor.configure_fraud_detection(FraudDetectionConfig {
            max_amount_per_card: Some(Amount::new(Decimal::from_str("500.00").unwrap(), Currency::USD)),
            max_transactions_per_hour: Some(3),
            velocity_check_enabled: true,
            geolocation_check_enabled: true,
            device_fingerprint_required: true,
        });
        
        let suspicious_card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "Suspicious User".to_string(),
        };
        
        // First few payments should succeed
        for i in 0..3 {
            let request = PaymentRequest::new(
                Amount::new(Decimal::from_str("100.00").unwrap(), Currency::USD),
                PaymentMethod::Card(suspicious_card.clone()),
                format!("order_{}", i),
            );
            
            let result = processor.process_payment(request).await;
            assert!(result.is_ok(), "Payment {} should succeed", i);
        }
        
        // Fourth payment should trigger velocity check
        let fourth_request = PaymentRequest::new(
            Amount::new(Decimal::from_str("100.00").unwrap(), Currency::USD),
            PaymentMethod::Card(suspicious_card),
            "order_velocity_trigger".to_string(),
        );
        
        let result = processor.process_payment(fourth_request).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            Error::FraudDetected(_) => (), // Expected
            other => panic!("Expected FraudDetected, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_pci_compliance_data_handling() {
        let processor = create_test_processor().await;
        
        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "PCI Test User".to_string(),
        };
        
        let request = PaymentRequest::new(
            Amount::new(Decimal::from_str("50.00").unwrap(), Currency::USD),
            PaymentMethod::Card(card),
            "pci_compliance_test".to_string(),
        );
        
        let response = processor.process_payment(request).await.unwrap();
        let transaction_id = response.transaction_id().unwrap();
        
        // Verify stored payment doesn't contain sensitive card data
        let stored_payment = processor.get_payment_by_id(transaction_id).await.unwrap();
        
        match stored_payment.payment_method() {
            PaymentMethod::Card(stored_card) => {
                // Card number should be masked
                assert!(stored_card.number.contains("****"));
                assert!(!stored_card.number.contains("4111111111111111"));
                
                // CVV should not be stored
                assert!(stored_card.cvv.is_empty() || stored_card.cvv == "***");
            },
            _ => panic!("Expected Card payment method"),
        }
    }

    #[tokio::test]
    async fn test_audit_trail_completeness() {
        let processor = create_test_processor().await;
        let request = create_test_payment_request(
            Decimal::from_str("125.00").unwrap(),
            Currency::EUR
        );
        
        // Process payment and refund
        let payment_response = processor.process_payment(request).await.unwrap();
        let transaction_id = payment_response.transaction_id().unwrap();
        
        let refund_response = processor.refund_payment(transaction_id, None).await.unwrap();
        
        // Verify complete audit trail
        let audit_trail = processor.get_audit_trail(transaction_id).await.unwrap();
        
        assert!(!audit_trail.is_empty());
        
        // Should contain all major events
        let event_types: Vec<_> = audit_trail.iter()
            .map(|event| event.event_type())
            .collect();
        
        assert!(event_types.contains(&AuditEventType::PaymentRequested));
        assert!(event_types.contains(&AuditEventType::PaymentProcessed));
        assert!(event_types.contains(&AuditEventType::PaymentCompleted));
        assert!(event_types.contains(&AuditEventType::RefundRequested));
        assert!(event_types.contains(&AuditEventType::RefundCompleted));
        
        // Verify each event has required metadata
        for event in audit_trail {
            assert!(event.timestamp() > chrono::Utc::now() - chrono::Duration::minutes(5));
            assert!(!event.user_id().is_empty());
            assert!(!event.ip_address().is_empty());
            assert!(event.metadata().contains_key("transaction_id"));
        }
    }
}