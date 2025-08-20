//! Basic Payment Processing Example
//! 
//! This example demonstrates the fundamental usage of the synapsed-payments library
//! including processing payments, handling refunds, and managing webhooks.

use synapsed_payments::*;
use synapsed_payments::types::*;
use synapsed_payments::processor::*;
use synapsed_payments::gateways::*;

use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("synapsed_payments=debug,basic_payment=info")
        .init();

    println!("🚀 Synapsed Payments - Basic Example");
    println!("=====================================\n");

    // Example 1: Basic Payment Processing
    basic_payment_example().await?;
    
    // Example 2: Payment with Refund
    payment_with_refund_example().await?;
    
    // Example 3: Multi-currency Payments
    multi_currency_example().await?;
    
    // Example 4: Webhook Handling
    webhook_example().await?;
    
    // Example 5: Error Handling
    error_handling_example().await?;

    println!("\n✅ All examples completed successfully!");
    Ok(())
}

/// Example 1: Basic Payment Processing
async fn basic_payment_example() -> Result<()> {
    println!("📝 Example 1: Basic Payment Processing");
    println!("-------------------------------------");

    // Step 1: Create a payment processor with test gateway
    let gateway = Arc::new(TestPaymentGateway::new());
    let processor = PaymentProcessor::builder()
        .with_gateway(gateway)
        .build()
        .await?;

    // Step 2: Create card details
    let card = CardDetails {
        number: "4111111111111111".to_string(), // Test Visa card
        expiry_month: 12,
        expiry_year: 2025,
        cvv: "123".to_string(),
        holder_name: "John Doe".to_string(),
    };

    // Step 3: Create payment amount
    let amount = Amount::new(
        Decimal::from_str("99.99")?,
        Currency::USD
    );

    // Step 4: Create payment request
    let request = PaymentRequest::new(
        amount,
        PaymentMethod::Card(card),
        "order_12345".to_string(),
    );

    println!("  💳 Processing payment for ${}", request.amount().value());
    println!("  📄 Order reference: {}", request.reference());

    // Step 5: Process the payment
    match processor.process_payment(request).await {
        Ok(response) => {
            println!("  ✅ Payment successful!");
            println!("     Transaction ID: {}", response.transaction_id().unwrap_or(&"N/A".to_string()));
            println!("     Gateway Reference: {}", response.gateway_reference());
            println!("     Status: {:?}", response.status());
            
            if let Some(amount) = response.processed_amount() {
                println!("     Processed Amount: {}", amount);
            }
        },
        Err(e) => {
            println!("  ❌ Payment failed: {}", e);
            return Err(e.into());
        }
    }

    println!();
    Ok(())
}

/// Example 2: Payment with Refund
async fn payment_with_refund_example() -> Result<()> {
    println!("📝 Example 2: Payment with Refund");
    println!("---------------------------------");

    let gateway = Arc::new(TestPaymentGateway::new());
    let processor = PaymentProcessor::builder()
        .with_gateway(gateway)
        .build()
        .await?;

    // Create and process original payment
    let card = CardDetails {
        number: "4111111111111111".to_string(),
        expiry_month: 6,
        expiry_year: 2026,
        cvv: "456".to_string(),
        holder_name: "Jane Smith".to_string(),
    };

    let amount = Amount::new(
        Decimal::from_str("150.00")?,
        Currency::USD
    );

    let request = PaymentRequest::new(
        amount,
        PaymentMethod::Card(card),
        "order_refund_test".to_string(),
    );

    println!("  💳 Processing original payment...");
    let payment_response = processor.process_payment(request).await?;
    let transaction_id = payment_response.transaction_id().unwrap();
    
    println!("  ✅ Original payment successful: {}", transaction_id);

    // Wait a moment (simulating time between payment and refund)
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Process full refund
    println!("  🔄 Processing full refund...");
    match processor.refund_payment(transaction_id, None).await {
        Ok(refund_response) => {
            println!("  ✅ Refund successful!");
            println!("     Refund ID: {}", refund_response.transaction_id().unwrap_or(&"N/A".to_string()));
            println!("     Status: {:?}", refund_response.status());
            
            if let Some(amount) = refund_response.processed_amount() {
                println!("     Refunded Amount: {}", amount);
            }
        },
        Err(e) => {
            println!("  ❌ Refund failed: {}", e);
            return Err(e.into());
        }
    }

    // Demonstrate partial refund
    println!("  🔄 Processing another payment for partial refund demo...");
    let partial_amount = Amount::new(
        Decimal::from_str("200.00")?,
        Currency::USD
    );
    
    let partial_request = PaymentRequest::new(
        partial_amount,
        PaymentMethod::Card(CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 3,
            expiry_year: 2027,
            cvv: "789".to_string(),
            holder_name: "Bob Wilson".to_string(),
        }),
        "order_partial_refund".to_string(),
    );

    let partial_payment = processor.process_payment(partial_request).await?;
    let partial_transaction_id = partial_payment.transaction_id().unwrap();

    // Process partial refund (50%)
    let refund_amount = Amount::new(
        Decimal::from_str("100.00")?,
        Currency::USD
    );

    println!("  🔄 Processing partial refund of $100...");
    match processor.refund_payment(partial_transaction_id, Some(refund_amount)).await {
        Ok(partial_refund) => {
            println!("  ✅ Partial refund successful!");
            println!("     Refunded: {}", partial_refund.processed_amount().unwrap());
        },
        Err(e) => {
            println!("  ❌ Partial refund failed: {}", e);
        }
    }

    println!();
    Ok(())
}

/// Example 3: Multi-currency Payments
async fn multi_currency_example() -> Result<()> {
    println!("📝 Example 3: Multi-currency Payments");
    println!("-------------------------------------");

    let gateway = Arc::new(TestPaymentGateway::new());
    let processor = PaymentProcessor::builder()
        .with_gateway(gateway)
        .build()
        .await?;

    // Define test payments in different currencies
    let currencies = vec![
        (Currency::USD, "100.00", "US Dollar"),
        (Currency::EUR, "85.50", "Euro"),
        (Currency::GBP, "75.25", "British Pound"),
        (Currency::JPY, "11000", "Japanese Yen"),
    ];

    for (currency, amount_str, currency_name) in currencies {
        println!("  💱 Processing {} payment...", currency_name);
        
        let amount = Amount::new(
            Decimal::from_str(amount_str)?,
            currency
        );

        let card = CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: format!("{} Customer", currency_name),
        };

        let request = PaymentRequest::new(
            amount.clone(),
            PaymentMethod::Card(card),
            format!("order_{}_{}", currency.code().to_lowercase(), amount_str.replace(".", "_")),
        );

        match processor.process_payment(request).await {
            Ok(response) => {
                println!("  ✅ {} payment successful: {}", 
                         currency_name, 
                         response.processed_amount().unwrap());
            },
            Err(e) => {
                println!("  ❌ {} payment failed: {}", currency_name, e);
            }
        }
    }

    println!();
    Ok(())
}

/// Example 4: Webhook Handling
async fn webhook_example() -> Result<()> {
    println!("📝 Example 4: Webhook Handling");
    println!("------------------------------");

    let gateway = Arc::new(TestPaymentGateway::new());
    let processor = PaymentProcessor::builder()
        .with_gateway(gateway)
        .with_webhook_handler(Arc::new(ExampleWebhookHandler::new()))
        .build()
        .await?;

    // Configure webhook security
    processor.configure_webhook_security(WebhookSecurityConfig {
        require_signature: true,
        secret_key: "example_webhook_secret_key".to_string(),
        timestamp_tolerance: std::time::Duration::from_secs(300),
    });

    // Simulate webhook events
    let payment_id = uuid::Uuid::new_v4();
    
    println!("  📡 Simulating webhook events...");

    // Payment completed webhook
    let completed_webhook = WebhookEvent::new(
        WebhookEventType::PaymentCompleted,
        payment_id,
        serde_json::json!({
            "transaction_id": "txn_webhook_example",
            "amount": "75.00",
            "currency": "USD",
            "gateway_reference": "gw_ref_12345",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
    );

    // Generate signature for webhook
    let signature = completed_webhook.generate_signature("example_webhook_secret_key");

    match processor.process_webhook_with_signature(completed_webhook, signature).await {
        Ok(result) => {
            println!("  ✅ Payment completed webhook processed successfully");
            if result.was_duplicate() {
                println!("     (Webhook was identified as duplicate)");
            }
        },
        Err(e) => {
            println!("  ❌ Webhook processing failed: {}", e);
        }
    }

    // Payment failed webhook
    let failed_webhook = WebhookEvent::new(
        WebhookEventType::PaymentFailed,
        uuid::Uuid::new_v4(),
        serde_json::json!({
            "error_code": "insufficient_funds",
            "error_message": "Insufficient funds in account",
            "attempted_amount": "500.00",
            "currency": "USD",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
    );

    let failed_signature = failed_webhook.generate_signature("example_webhook_secret_key");

    match processor.process_webhook_with_signature(failed_webhook, failed_signature).await {
        Ok(_) => {
            println!("  ✅ Payment failed webhook processed successfully");
        },
        Err(e) => {
            println!("  ❌ Failed webhook processing failed: {}", e);
        }
    }

    println!();
    Ok(())
}

/// Example 5: Error Handling and Edge Cases
async fn error_handling_example() -> Result<()> {
    println!("📝 Example 5: Error Handling");
    println!("----------------------------");

    let gateway = Arc::new(TestPaymentGateway::new());
    let processor = PaymentProcessor::builder()
        .with_gateway(gateway)
        .build()
        .await?;

    // Example 1: Invalid card number (fails Luhn check)
    println!("  🔍 Testing invalid card number...");
    let invalid_card = CardDetails {
        number: "4111111111111112".to_string(), // Invalid Luhn
        expiry_month: 12,
        expiry_year: 2025,
        cvv: "123".to_string(),
        holder_name: "Invalid Card".to_string(),
    };

    let invalid_request = PaymentRequest::new(
        Amount::new(Decimal::from_str("50.00")?, Currency::USD),
        PaymentMethod::Card(invalid_card),
        "invalid_card_test".to_string(),
    );

    match processor.process_payment(invalid_request).await {
        Ok(_) => println!("  ⚠️  Unexpected success with invalid card"),
        Err(e) => {
            println!("  ✅ Correctly rejected invalid card: {}", e);
            match e {
                Error::ValidationError(msg) => {
                    println!("     Validation error: {}", msg);
                },
                _ => println!("     Other error type: {:?}", e),
            }
        }
    }

    // Example 2: Expired card
    println!("  🔍 Testing expired card...");
    let expired_card = CardDetails {
        number: "4111111111111111".to_string(),
        expiry_month: 1,
        expiry_year: 2020, // Expired
        cvv: "123".to_string(),
        holder_name: "Expired Card".to_string(),
    };

    let expired_request = PaymentRequest::new(
        Amount::new(Decimal::from_str("25.00")?, Currency::USD),
        PaymentMethod::Card(expired_card),
        "expired_card_test".to_string(),
    );

    match processor.process_payment(expired_request).await {
        Ok(_) => println!("  ⚠️  Unexpected success with expired card"),
        Err(e) => {
            println!("  ✅ Correctly rejected expired card: {}", e);
        }
    }

    // Example 3: Zero amount payment
    println!("  🔍 Testing zero amount payment...");
    let zero_amount_request = PaymentRequest::new(
        Amount::zero(Currency::USD),
        PaymentMethod::Card(CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "Zero Amount".to_string(),
        }),
        "zero_amount_test".to_string(),
    );

    match processor.process_payment(zero_amount_request).await {
        Ok(_) => println!("  ⚠️  Unexpected success with zero amount"),
        Err(e) => {
            println!("  ✅ Correctly rejected zero amount: {}", e);
        }
    }

    // Example 4: Empty reference
    println!("  🔍 Testing empty order reference...");
    let empty_ref_request = PaymentRequest::new(
        Amount::new(Decimal::from_str("10.00")?, Currency::USD),
        PaymentMethod::Card(CardDetails {
            number: "4111111111111111".to_string(),
            expiry_month: 12,
            expiry_year: 2025,
            cvv: "123".to_string(),
            holder_name: "Empty Reference".to_string(),
        }),
        "".to_string(), // Empty reference
    );

    match processor.process_payment(empty_ref_request).await {
        Ok(_) => println!("  ⚠️  Unexpected success with empty reference"),
        Err(e) => {
            println!("  ✅ Correctly rejected empty reference: {}", e);
        }
    }

    println!();
    Ok(())
}

/// Example webhook handler implementation
struct ExampleWebhookHandler;

impl ExampleWebhookHandler {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl WebhookHandler for ExampleWebhookHandler {
    async fn handle_payment_completed(&self, event: &WebhookEvent) -> Result<(), Error> {
        println!("    📨 Handling payment completed webhook");
        println!("       Payment ID: {}", event.payment_id());
        println!("       Event data: {}", serde_json::to_string_pretty(event.payload())?);
        Ok(())
    }

    async fn handle_payment_failed(&self, event: &WebhookEvent) -> Result<(), Error> {
        println!("    📨 Handling payment failed webhook");
        println!("       Payment ID: {}", event.payment_id());
        
        if let Some(error_code) = event.payload().get("error_code") {
            println!("       Error code: {}", error_code);
        }
        
        if let Some(error_message) = event.payload().get("error_message") {
            println!("       Error message: {}", error_message);
        }
        
        Ok(())
    }

    async fn handle_payment_refunded(&self, event: &WebhookEvent) -> Result<(), Error> {
        println!("    📨 Handling payment refunded webhook");
        println!("       Payment ID: {}", event.payment_id());
        Ok(())
    }

    async fn handle_unknown_event(&self, event: &WebhookEvent) -> Result<(), Error> {
        println!("    📨 Handling unknown webhook event: {:?}", event.event_type());
        Ok(())
    }
}