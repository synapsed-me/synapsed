use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{PaymentError, PaymentResult};
use crate::gateway::{GatewayConfig, GatewayFactory};
use crate::processor::{BasicRiskEngine, PaymentProcessor, PaymentStorage, ProcessorConfig, RetryConfig, RiskEngine};
use crate::storage::MemoryPaymentStorage;
use crate::types::{Currency, PaymentConfig};

/// Builder for creating a complete PaymentManager instance
pub struct PaymentManagerBuilder {
    payment_config: Option<PaymentConfig>,
    gateway_configs: HashMap<String, GatewayConfig>,
    risk_threshold: u8,
    retry_config: Option<RetryConfig>,
    storage: Option<Arc<dyn PaymentStorage + Send + Sync>>,
    risk_engine: Option<Arc<dyn RiskEngine + Send + Sync>>,
}

/// Complete payment management system
pub struct PaymentManager {
    processor: PaymentProcessor,
}

impl PaymentManagerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            payment_config: None,
            gateway_configs: HashMap::new(),
            risk_threshold: 70,
            retry_config: None,
            storage: None,
            risk_engine: None,
        }
    }

    /// Set the payment configuration
    pub fn with_payment_config(mut self, config: PaymentConfig) -> Self {
        self.payment_config = Some(config);
        self
    }

    /// Add a payment gateway configuration
    pub fn with_gateway_config(mut self, config: GatewayConfig) -> Self {
        self.gateway_configs.insert(config.gateway_id.clone(), config);
        self
    }

    /// Set the risk threshold (0-100, higher = stricter)
    pub fn with_risk_threshold(mut self, threshold: u8) -> Self {
        self.risk_threshold = threshold.min(100);
        self
    }

    /// Set retry configuration
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = Some(config);
        self
    }

    /// Set custom storage implementation
    pub fn with_storage(mut self, storage: Arc<dyn PaymentStorage + Send + Sync>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Set custom risk engine
    pub fn with_risk_engine(mut self, engine: Arc<dyn RiskEngine + Send + Sync>) -> Self {
        self.risk_engine = Some(engine);
        self
    }

    /// Add a quick Stripe gateway configuration
    pub fn with_stripe_gateway(
        mut self,
        gateway_id: String,
        api_key: String,
        webhook_secret: Option<String>,
    ) -> Self {
        let mut custom_config = HashMap::new();
        custom_config.insert("api_key".to_string(), serde_json::Value::String(api_key));
        
        if let Some(secret) = webhook_secret {
            custom_config.insert("webhook_secret".to_string(), serde_json::Value::String(secret));
        }

        let config = GatewayConfig {
            gateway_id: gateway_id.clone(),
            name: "stripe".to_string(),
            enabled: true,
            api_url: "https://api.stripe.com/v1".to_string(),
            timeout_seconds: 30,
            max_retries: 3,
            supported_currencies: vec![
                Currency::Fiat(crate::types::FiatCurrency::USD),
                Currency::Fiat(crate::types::FiatCurrency::EUR),
                Currency::Fiat(crate::types::FiatCurrency::GBP),
            ],
            supported_payment_methods: vec![
                "card".to_string(),
                "bank_transfer".to_string(),
                "wallet".to_string(),
            ],
            webhook_config: None,
            custom_config,
        };

        self.gateway_configs.insert(gateway_id, config);
        self
    }

    /// Add a quick PayPal gateway configuration
    pub fn with_paypal_gateway(
        mut self,
        gateway_id: String,
        client_id: String,
        client_secret: String,
        sandbox: bool,
    ) -> Self {
        let mut custom_config = HashMap::new();
        custom_config.insert("client_id".to_string(), serde_json::Value::String(client_id));
        custom_config.insert("client_secret".to_string(), serde_json::Value::String(client_secret));
        custom_config.insert("sandbox".to_string(), serde_json::Value::Bool(sandbox));

        let api_url = if sandbox {
            "https://api.sandbox.paypal.com"
        } else {
            "https://api.paypal.com"
        };

        let config = GatewayConfig {
            gateway_id: gateway_id.clone(),
            name: "paypal".to_string(),
            enabled: true,
            api_url: api_url.to_string(),
            timeout_seconds: 30,
            max_retries: 3,
            supported_currencies: vec![
                Currency::Fiat(crate::types::FiatCurrency::USD),
                Currency::Fiat(crate::types::FiatCurrency::EUR),
                Currency::Fiat(crate::types::FiatCurrency::GBP),
                Currency::Fiat(crate::types::FiatCurrency::CAD),
                Currency::Fiat(crate::types::FiatCurrency::AUD),
            ],
            supported_payment_methods: vec![
                "paypal".to_string(),
                "card".to_string(),
            ],
            webhook_config: None,
            custom_config,
        };

        self.gateway_configs.insert(gateway_id, config);
        self
    }

    /// Add a mock gateway for testing
    pub fn with_mock_gateway(mut self, gateway_id: String) -> Self {
        let config = GatewayConfig {
            gateway_id: gateway_id.clone(),
            name: "mock".to_string(),
            enabled: true,
            api_url: "http://localhost:8080".to_string(),
            timeout_seconds: 10,
            max_retries: 1,
            supported_currencies: vec![
                Currency::Fiat(crate::types::FiatCurrency::USD),
                Currency::Fiat(crate::types::FiatCurrency::EUR),
            ],
            supported_payment_methods: vec![
                "card".to_string(),
                "bank_transfer".to_string(),
            ],
            webhook_config: None,
            custom_config: HashMap::new(),
        };

        self.gateway_configs.insert(gateway_id, config);
        self
    }

    /// Create a development configuration with sensible defaults
    pub fn development() -> Self {
        let payment_config = PaymentConfig {
            merchant_id: "dev_merchant".to_string(),
            supported_currencies: vec![
                Currency::Fiat(crate::types::FiatCurrency::USD),
                Currency::Fiat(crate::types::FiatCurrency::EUR),
            ],
            supported_payment_methods: vec![
                "card".to_string(),
                "bank_transfer".to_string(),
                "wallet".to_string(),
            ],
            webhook_url: Some("http://localhost:3000/webhooks/payments".to_string()),
            return_url: Some("http://localhost:3000/payment/success".to_string()),
            cancel_url: Some("http://localhost:3000/payment/cancel".to_string()),
            auto_capture: true,
            capture_delay_hours: None,
            max_retry_attempts: 3,
        };

        Self::new()
            .with_payment_config(payment_config)
            .with_mock_gateway("mock_primary".to_string())
            .with_risk_threshold(50) // Lower risk threshold for development
    }

    /// Create a production configuration template
    pub fn production_template() -> Self {
        let payment_config = PaymentConfig {
            merchant_id: "REPLACE_WITH_MERCHANT_ID".to_string(),
            supported_currencies: vec![
                Currency::Fiat(crate::types::FiatCurrency::USD),
                Currency::Fiat(crate::types::FiatCurrency::EUR),
                Currency::Fiat(crate::types::FiatCurrency::GBP),
            ],
            supported_payment_methods: vec![
                "card".to_string(),
                "bank_transfer".to_string(),
                "digital_wallet".to_string(),
            ],
            webhook_url: Some("https://api.yourdomain.com/webhooks/payments".to_string()),
            return_url: Some("https://yourdomain.com/payment/success".to_string()),
            cancel_url: Some("https://yourdomain.com/payment/cancel".to_string()),
            auto_capture: false, // Manual capture for production safety
            capture_delay_hours: Some(24),
            max_retry_attempts: 5,
        };

        let retry_config = RetryConfig {
            max_attempts: 5,
            base_delay_ms: 2000,
            max_delay_ms: 60000,
            backoff_multiplier: 2.5,
        };

        Self::new()
            .with_payment_config(payment_config)
            .with_retry_config(retry_config)
            .with_risk_threshold(80) // Higher risk threshold for production
    }

    /// Build the PaymentManager
    pub fn build(self) -> PaymentResult<PaymentManager> {
        // Validate required configuration
        let payment_config = self.payment_config.ok_or_else(|| {
            PaymentError::ConfigurationError {
                message: "Payment configuration is required".to_string(),
            }
        })?;

        if self.gateway_configs.is_empty() {
            return Err(PaymentError::ConfigurationError {
                message: "At least one gateway configuration is required".to_string(),
            });
        }

        // Create processor configuration
        let processor_config = ProcessorConfig {
            payment_config,
            gateway_configs: self.gateway_configs.clone(),
            risk_threshold: self.risk_threshold,
            retry_config: self.retry_config.unwrap_or_default(),
        };

        // Create default implementations if not provided
        let storage = self.storage.unwrap_or_else(|| {
            Arc::new(MemoryPaymentStorage::new())
        });

        let risk_engine = self.risk_engine.unwrap_or_else(|| {
            Arc::new(BasicRiskEngine::new(self.risk_threshold))
        });

        // Create processor
        let mut processor = PaymentProcessor::new(processor_config, risk_engine, storage);

        // Initialize gateways
        for (gateway_id, config) in self.gateway_configs {
            if config.enabled {
                let gateway = GatewayFactory::create_gateway(config)?;
                processor.register_gateway(gateway_id, Arc::from(gateway));
            }
        }

        Ok(PaymentManager { processor })
    }
}

impl PaymentManager {
    /// Get a reference to the payment processor
    pub fn processor(&self) -> &PaymentProcessor {
        &self.processor
    }

    /// Get a mutable reference to the payment processor
    pub fn processor_mut(&mut self) -> &mut PaymentProcessor {
        &mut self.processor
    }

    /// Convenience method to create a payment intent
    pub async fn create_payment(
        &self,
        amount: crate::types::Amount,
        description: String,
        customer_id: Option<String>,
    ) -> PaymentResult<crate::types::PaymentIntent> {
        self.processor.create_payment_intent(amount, description, customer_id).await
    }

    /// Convenience method to process a payment
    pub async fn process_payment(
        &self,
        payment_id: uuid::Uuid,
        payment_method: crate::types::PaymentMethod,
    ) -> PaymentResult<crate::types::Transaction> {
        self.processor.process_payment(payment_id, payment_method).await
    }

    /// Convenience method to refund a payment
    pub async fn refund_payment(
        &self,
        payment_id: uuid::Uuid,
        amount: Option<crate::types::Amount>,
        reason: Option<String>,
    ) -> PaymentResult<crate::types::Refund> {
        self.processor.refund_payment(payment_id, amount, reason).await
    }

    /// Get payment status
    pub async fn get_payment_status(
        &self,
        payment_id: uuid::Uuid,
    ) -> PaymentResult<crate::types::PaymentStatus> {
        self.processor.get_payment_status(payment_id).await
    }

    /// Health check for the entire payment system
    pub async fn health_check(&self) -> PaymentResult<HashMap<String, String>> {
        self.processor.health_check().await
    }
}

impl Default for PaymentManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FiatCurrency, Amount};
    use rust_decimal::Decimal;

    #[test]
    fn test_builder_creation() {
        let builder = PaymentManagerBuilder::new();
        assert!(builder.payment_config.is_none());
        assert!(builder.gateway_configs.is_empty());
        assert_eq!(builder.risk_threshold, 70);
    }

    #[test]
    fn test_development_config() {
        let builder = PaymentManagerBuilder::development();
        assert!(builder.payment_config.is_some());
        assert!(!builder.gateway_configs.is_empty());
        assert_eq!(builder.risk_threshold, 50);
    }

    #[test]
    fn test_production_template() {
        let builder = PaymentManagerBuilder::production_template();
        assert!(builder.payment_config.is_some());
        assert_eq!(builder.risk_threshold, 80);
        assert!(builder.retry_config.is_some());
    }

    #[tokio::test]
    async fn test_build_development_manager() {
        let manager = PaymentManagerBuilder::development()
            .build()
            .expect("Failed to build development manager");

        // Test health check
        let health = manager.health_check().await;
        assert!(health.is_ok());
    }

    #[tokio::test]
    async fn test_manager_create_payment() {
        let manager = PaymentManagerBuilder::development()
            .build()
            .expect("Failed to build manager");

        let amount = Amount::new(
            Decimal::new(10000, 2), // $100.00
            Currency::Fiat(FiatCurrency::USD),
        ).expect("Failed to create amount");

        let result = manager
            .create_payment(amount, "Test payment".to_string(), None)
            .await;

        assert!(result.is_ok());
        let payment = result.unwrap();
        assert_eq!(payment.description, "Test payment");
        assert_eq!(payment.status, crate::types::PaymentStatus::Pending);
    }

    #[test]
    fn test_stripe_gateway_config() {
        let builder = PaymentManagerBuilder::new()
            .with_stripe_gateway(
                "stripe_main".to_string(),
                "sk_test_123".to_string(),
                Some("whsec_123".to_string()),
            );

        assert!(builder.gateway_configs.contains_key("stripe_main"));
        let config = &builder.gateway_configs["stripe_main"];
        assert_eq!(config.name, "stripe");
        assert_eq!(config.api_url, "https://api.stripe.com/v1");
    }

    #[test]
    fn test_paypal_gateway_config() {
        let builder = PaymentManagerBuilder::new()
            .with_paypal_gateway(
                "paypal_main".to_string(),
                "client_id_123".to_string(),
                "client_secret_123".to_string(),
                true, // sandbox
            );

        assert!(builder.gateway_configs.contains_key("paypal_main"));
        let config = &builder.gateway_configs["paypal_main"];
        assert_eq!(config.name, "paypal");
        assert_eq!(config.api_url, "https://api.sandbox.paypal.com");
    }

    #[test]
    fn test_build_without_config_fails() {
        let result = PaymentManagerBuilder::new().build();
        assert!(result.is_err());
        
        if let Err(PaymentError::ConfigurationError { message }) = result {
            assert!(message.contains("Payment configuration is required"));
        } else {
            panic!("Expected configuration error");
        }
    }

    #[test]
    fn test_build_without_gateway_fails() {
        let payment_config = PaymentConfig {
            merchant_id: "test".to_string(),
            supported_currencies: vec![Currency::Fiat(FiatCurrency::USD)],
            supported_payment_methods: vec!["card".to_string()],
            webhook_url: None,
            return_url: None,
            cancel_url: None,
            auto_capture: true,
            capture_delay_hours: None,
            max_retry_attempts: 3,
        };

        let result = PaymentManagerBuilder::new()
            .with_payment_config(payment_config)
            .build();

        assert!(result.is_err());
        if let Err(PaymentError::ConfigurationError { message }) = result {
            assert!(message.contains("At least one gateway configuration is required"));
        } else {
            panic!("Expected configuration error");
        }
    }
}