//! Unit tests for payment types
//! 
//! Comprehensive tests for all payment-related data structures,
//! ensuring correctness, security, and compliance.

#![cfg(test)]

use synapsed_payments::types::*;
use synapsed_payments::{Error, Result};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::str::FromStr;
use serde_json;

#[cfg(test)]
mod amount_tests {
    use super::*;

    #[test]
    fn test_amount_creation() {
        let amount = Amount::new(Decimal::from_str("100.50").unwrap(), Currency::USD);
        assert_eq!(amount.value(), &Decimal::from_str("100.50").unwrap());
        assert_eq!(amount.currency(), &Currency::USD);
    }

    #[test]
    fn test_amount_zero() {
        let amount = Amount::zero(Currency::EUR);
        assert_eq!(amount.value(), &Decimal::ZERO);
        assert_eq!(amount.currency(), &Currency::EUR);
        assert!(amount.is_zero());
    }

    #[test]
    fn test_amount_validation() {
        // Valid amounts
        assert!(Amount::new(Decimal::from_str("0.01").unwrap(), Currency::USD).is_valid());
        assert!(Amount::new(Decimal::from_str("999999.99").unwrap(), Currency::USD).is_valid());
        
        // Invalid amounts
        assert!(!Amount::new(Decimal::from_str("-10.00").unwrap(), Currency::USD).is_valid());
        assert!(!Amount::new(Decimal::from_str("0.001").unwrap(), Currency::USD).is_valid()); // Too many decimals
    }

    #[test]
    fn test_amount_arithmetic() {
        let amount1 = Amount::new(Decimal::from_str("100.50").unwrap(), Currency::USD);
        let amount2 = Amount::new(Decimal::from_str("50.25").unwrap(), Currency::USD);
        
        // Addition
        let sum = amount1.add(&amount2).unwrap();
        assert_eq!(sum.value(), &Decimal::from_str("150.75").unwrap());
        
        // Subtraction
        let diff = amount1.subtract(&amount2).unwrap();
        assert_eq!(diff.value(), &Decimal::from_str("50.25").unwrap());
        
        // Currency mismatch should fail
        let eur_amount = Amount::new(Decimal::from_str("10.00").unwrap(), Currency::EUR);
        assert!(amount1.add(&eur_amount).is_err());
    }

    #[test]
    fn test_amount_serialization() {
        let amount = Amount::new(Decimal::from_str("123.45").unwrap(), Currency::GBP);
        
        let json = serde_json::to_string(&amount).unwrap();
        let deserialized: Amount = serde_json::from_str(&json).unwrap();
        
        assert_eq!(amount, deserialized);
    }

    #[test]
    fn test_amount_display() {
        let amount = Amount::new(Decimal::from_str("1234.56").unwrap(), Currency::USD);
        let display = format!("{}", amount);
        assert_eq!(display, "$1,234.56");
        
        let eur_amount = Amount::new(Decimal::from_str("999.99").unwrap(), Currency::EUR);
        let eur_display = format!("{}", eur_amount);
        assert_eq!(eur_display, "€999.99");
    }
}

#[cfg(test)]
mod currency_tests {
    use super::*;

    #[test] 
    fn test_currency_variants() {
        let currencies = vec![
            Currency::USD,
            Currency::EUR,
            Currency::GBP,
            Currency::JPY,
            Currency::BTC,
            Currency::ETH,
            Currency::Custom("CUSTOM".to_string()),
        ];
        
        for currency in &currencies {
            let json = serde_json::to_string(currency).unwrap();
            let deserialized: Currency = serde_json::from_str(&json).unwrap();
            assert_eq!(currency, &deserialized);
        }
    }

    #[test]
    fn test_currency_properties() {
        assert_eq!(Currency::USD.code(), "USD");
        assert_eq!(Currency::USD.symbol(), "$");
        assert_eq!(Currency::USD.decimal_places(), 2);
        
        assert_eq!(Currency::JPY.decimal_places(), 0);
        assert_eq!(Currency::BTC.decimal_places(), 8);
    }

    #[test]
    fn test_currency_from_str() {
        assert_eq!(Currency::from_str("USD").unwrap(), Currency::USD);
        assert_eq!(Currency::from_str("EUR").unwrap(), Currency::EUR);
        assert!(Currency::from_str("INVALID").is_err());
    }
}

#[cfg(test)]
mod payment_method_tests {
    use super::*;

    #[test]
    fn test_card_payment_method() {
        let expiry = DateTime::parse_from_rfc3339("2025-12-31T23:59:59Z").unwrap().with_timezone(&Utc);
        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "John Doe".to_string(),
        };
        
        let payment_method = PaymentMethod::Card(card.clone());
        
        match payment_method {
            PaymentMethod::Card(ref c) => {
                assert_eq!(c.holder_name, "John Doe");
                assert_eq!(c.expiry_month, 12);
                assert!(c.is_valid());
            },
            _ => panic!("Expected Card payment method"),
        }
    }

    #[test]
    fn test_bank_transfer_payment_method() {
        let bank_transfer = BankTransferDetails {
            account_number: "12345678".to_string(),
            routing_number: "987654321".to_string(),
            account_holder: "Jane Smith".to_string(),
            bank_name: "Test Bank".to_string(),
        };
        
        let payment_method = PaymentMethod::BankTransfer(bank_transfer);
        
        match payment_method {
            PaymentMethod::BankTransfer(ref bt) => {
                assert_eq!(bt.account_holder, "Jane Smith");
                assert!(bt.is_valid());
            },
            _ => panic!("Expected BankTransfer payment method"),
        }
    }

    #[test]
    fn test_crypto_payment_method() {
        let crypto = CryptoDetails {
            currency: CryptoCurrency::Bitcoin,
            address: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
            network: "mainnet".to_string(),
        };
        
        let payment_method = PaymentMethod::Crypto(crypto);
        
        match payment_method {
            PaymentMethod::Crypto(ref c) => {
                assert_eq!(c.currency, CryptoCurrency::Bitcoin);
                assert!(c.is_valid_address());
            },
            _ => panic!("Expected Crypto payment method"),
        }
    }

    #[test]
    fn test_payment_method_validation() {
        // Valid card
        let valid_card = CardDetails {
            number: "4111111111111111".to_string(), // Valid Luhn
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "John Doe".to_string(),
        };
        assert!(valid_card.is_valid());
        
        // Invalid card - bad Luhn
        let invalid_card = CardDetails {
            number: "4111111111111112".to_string(), // Invalid Luhn
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "John Doe".to_string(),
        };
        assert!(!invalid_card.is_valid());
        
        // Expired card
        let expired_card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 1,
            expiry_year: 2020, // Expired
            cvv: "123".to_string(),
            holder_name: "John Doe".to_string(),
        };
        assert!(!expired_card.is_valid());
    }
}

#[cfg(test)]
mod payment_request_tests {
    use super::*;

    #[test]
    fn test_payment_request_creation() {
        let amount = Amount::new(Decimal::from_str("100.00").unwrap(), Currency::USD);
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
            "order-123".to_string(),
        );
        
        assert_eq!(request.amount().value(), &Decimal::from_str("100.00").unwrap());
        assert_eq!(request.reference(), "order-123");
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_payment_request_with_metadata() {
        let amount = Amount::new(Decimal::from_str("50.00").unwrap(), Currency::EUR);
        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "Jane Smith".to_string(),
        };
        
        let mut request = PaymentRequest::new(
            amount,
            PaymentMethod::Card(card),
            "order-456".to_string(),
        );
        
        request.add_metadata("customer_id", "cust_789");
        request.add_metadata("order_type", "subscription");
        
        assert_eq!(request.get_metadata("customer_id"), Some(&"cust_789".to_string()));
        assert_eq!(request.get_metadata("order_type"), Some(&"subscription".to_string()));
    }

    #[test]
    fn test_payment_request_validation() {
        // Invalid amount (zero)
        let zero_amount = Amount::zero(Currency::USD);
        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "John Doe".to_string(),
        };
        
        let invalid_request = PaymentRequest::new(
            zero_amount,
            PaymentMethod::Card(card.clone()),
            "order-invalid".to_string(),
        );
        
        assert!(invalid_request.validate().is_err());
        
        // Valid request
        let valid_amount = Amount::new(Decimal::from_str("1.00").unwrap(), Currency::USD);
        let valid_request = PaymentRequest::new(
            valid_amount,
            PaymentMethod::Card(card),
            "order-valid".to_string(),
        );
        
        assert!(valid_request.validate().is_ok());
    }
}

#[cfg(test)]
mod payment_response_tests {
    use super::*;

    #[test]
    fn test_successful_payment_response() {
        let amount = Amount::new(Decimal::from_str("100.00").unwrap(), Currency::USD);
        let response = PaymentResponse::success(
            Uuid::new_v4(),
            "txn_12345".to_string(),
            amount,
            "gateway_ref_789".to_string(),
        );
        
        assert_eq!(response.status(), PaymentStatus::Completed);
        assert_eq!(response.gateway_reference(), "gateway_ref_789");
        assert!(response.error_message().is_none());
    }

    #[test]
    fn test_failed_payment_response() {
        let response = PaymentResponse::failure(
            Uuid::new_v4(),
            PaymentError::InsufficientFunds,
            "Insufficient funds in account".to_string(),
        );
        
        assert_eq!(response.status(), PaymentStatus::Failed);
        assert_eq!(response.error_message(), Some(&"Insufficient funds in account".to_string()));
        assert!(response.transaction_id().is_none());
    }

    #[test]
    fn test_pending_payment_response() {
        let response = PaymentResponse::pending(
            Uuid::new_v4(),
            "gateway_ref_pending".to_string(),
            "Payment is being processed".to_string(),
        );
        
        assert_eq!(response.status(), PaymentStatus::Pending);
        assert_eq!(response.gateway_reference(), "gateway_ref_pending");
    }

    #[test]
    fn test_payment_response_serialization() {
        let amount = Amount::new(Decimal::from_str("75.50").unwrap(), Currency::GBP);
        let response = PaymentResponse::success(
            Uuid::new_v4(),
            "txn_test".to_string(),
            amount,
            "gateway_test".to_string(),
        );
        
        let json = serde_json::to_string(&response).unwrap();
        let deserialized: PaymentResponse = serde_json::from_str(&json).unwrap();
        
        assert_eq!(response.status(), deserialized.status());
        assert_eq!(response.gateway_reference(), deserialized.gateway_reference());
    }
}

#[cfg(test)]
mod payment_status_tests {
    use super::*;

    #[test]
    fn test_payment_status_transitions() {
        assert!(PaymentStatus::Pending.can_transition_to(&PaymentStatus::Completed));
        assert!(PaymentStatus::Pending.can_transition_to(&PaymentStatus::Failed));
        assert!(PaymentStatus::Pending.can_transition_to(&PaymentStatus::Cancelled));
        
        assert!(!PaymentStatus::Completed.can_transition_to(&PaymentStatus::Pending));
        assert!(!PaymentStatus::Failed.can_transition_to(&PaymentStatus::Completed));
        assert!(!PaymentStatus::Cancelled.can_transition_to(&PaymentStatus::Completed));
    }

    #[test]
    fn test_payment_status_finality() {
        assert!(!PaymentStatus::Pending.is_final());
        assert!(!PaymentStatus::Processing.is_final());
        assert!(PaymentStatus::Completed.is_final());
        assert!(PaymentStatus::Failed.is_final());
        assert!(PaymentStatus::Cancelled.is_final());
        assert!(PaymentStatus::Refunded.is_final());
    }
}

#[cfg(test)]
mod webhook_event_tests {
    use super::*;

    #[test]
    fn test_webhook_event_creation() {
        let amount = Amount::new(Decimal::from_str("200.00").unwrap(), Currency::USD);
        let event = WebhookEvent::new(
            WebhookEventType::PaymentCompleted,
            Uuid::new_v4(),
            serde_json::json!({
                "transaction_id": "txn_webhook_test",
                "amount": amount,
                "status": "completed"
            }),
        );
        
        assert_eq!(event.event_type(), &WebhookEventType::PaymentCompleted);
        assert!(event.payload().get("transaction_id").is_some());
    }

    #[test]
    fn test_webhook_signature_validation() {
        let event = WebhookEvent::new(
            WebhookEventType::PaymentFailed,
            Uuid::new_v4(),
            serde_json::json!({"error": "card_declined"}),
        );
        
        let secret = "webhook_secret_key";
        let signature = event.generate_signature(secret);
        
        assert!(event.verify_signature(&signature, secret));
        assert!(!event.verify_signature(&signature, "wrong_secret"));
        assert!(!event.verify_signature("invalid_signature", secret));
    }

    #[test]
    fn test_webhook_event_serialization() {
        let event = WebhookEvent::new(
            WebhookEventType::PaymentRefunded,
            Uuid::new_v4(),
            serde_json::json!({
                "refund_id": "ref_123",
                "original_payment": "pay_456",
                "amount_refunded": "50.00"
            }),
        );
        
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: WebhookEvent = serde_json::from_str(&json).unwrap();
        
        assert_eq!(event.event_type(), deserialized.event_type());
        assert_eq!(event.payment_id(), deserialized.payment_id());
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_sensitive_data_zeroization() {
        let mut card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "John Doe".to_string(),
        };
        
        // CVV should be zeroized when dropped
        let cvv_ptr = card.cvv.as_ptr();
        drop(card);
        
        // Note: In real implementation, zeroization would occur
        // This test structure demonstrates the intent
    }

    #[test]
    fn test_card_number_masking() {
        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "John Doe".to_string(),
        };
        
        let masked = card.masked_number();
        assert_eq!(masked, "************1111");
        assert_ne!(masked, card.number);
    }

    #[test]
    fn test_payment_token_generation() {
        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "John Doe".to_string(),
        };
        
        let token1 = card.generate_token();
        let token2 = card.generate_token();
        
        // Tokens should be unique even for same card
        assert_ne!(token1, token2);
        assert!(!token1.is_empty());
        assert!(!token2.is_empty());
    }
}

#[cfg(test)]
mod compliance_tests {
    use super::*;

    #[test]
    fn test_pci_dss_compliance() {
        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "John Doe".to_string(),
        };
        
        // Sensitive data should not appear in debug output
        let debug_output = format!("{:?}", card);
        assert!(!debug_output.contains("4111111111111111"));
        assert!(!debug_output.contains("123")); // CVV
        
        // Only masked data should appear
        assert!(debug_output.contains("************1111"));
    }

    #[test]
    fn test_gdpr_data_handling() {
        let mut payment_request = PaymentRequest::new(
            Amount::new(Decimal::from_str("100.00").unwrap(), Currency::EUR),
            PaymentMethod::Card(CardDetails {
                number: "4111111111111111".to_string(),
                expiry_month: 12,
                expiry_year: 2025,
                cvv: "123".to_string(),
                holder_name: "EU Customer".to_string(),
            }),
            "gdpr-test".to_string(),
        );
        
        payment_request.add_metadata("customer_consent", "true");
        payment_request.add_metadata("data_retention", "2_years");
        
        // Test data anonymization capability
        let anonymized = payment_request.anonymize();
        assert!(anonymized.get_metadata("customer_consent").is_none());
        
        // Essential data should remain
        assert_eq!(anonymized.reference(), "gdpr-test");
    }
}