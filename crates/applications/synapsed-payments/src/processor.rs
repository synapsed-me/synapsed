use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};
use uuid::Uuid;
use validator::Validate;

use crate::error::{PaymentError, PaymentResult};
use crate::gateway::{PaymentGateway, GatewayConfig};
use crate::types::{
    Amount, Currency, Customer, PaymentConfig, PaymentIntent, PaymentMethod, 
    PaymentStatus, Refund, RiskAssessment, RiskLevel, Transaction, TransactionType,
};

/// Core payment processor that orchestrates payment workflows
pub struct PaymentProcessor {
    config: PaymentConfig,
    gateways: HashMap<String, Arc<dyn PaymentGateway + Send + Sync>>,
    risk_engine: Arc<dyn RiskEngine + Send + Sync>,
    storage: Arc<dyn PaymentStorage + Send + Sync>,
    active_payments: Arc<RwLock<HashMap<Uuid, PaymentSession>>>,
}

/// Payment session tracking
#[derive(Debug, Clone)]
pub struct PaymentSession {
    pub payment_id: Uuid,
    pub status: PaymentStatus,
    pub gateway_id: String,
    pub attempts: u8,
    pub last_attempt: chrono::DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Payment processing configuration
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    pub payment_config: PaymentConfig,
    pub gateway_configs: HashMap<String, GatewayConfig>,
    pub risk_threshold: u8,
    pub retry_config: RetryConfig,
}

/// Retry configuration for failed payments
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u8,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

/// Risk assessment engine trait
#[async_trait]
pub trait RiskEngine {
    /// Assess payment risk
    async fn assess_risk(
        &self,
        payment: &PaymentIntent,
        customer: Option<&Customer>,
    ) -> PaymentResult<RiskAssessment>;

    /// Check if payment should be blocked
    async fn should_block_payment(&self, assessment: &RiskAssessment) -> bool;
}

/// Payment storage trait
#[async_trait]
pub trait PaymentStorage {
    /// Store payment intent
    async fn store_payment(&self, payment: &PaymentIntent) -> PaymentResult<()>;

    /// Retrieve payment intent
    async fn get_payment(&self, payment_id: Uuid) -> PaymentResult<PaymentIntent>;

    /// Update payment status
    async fn update_payment_status(
        &self,
        payment_id: Uuid,
        status: PaymentStatus,
    ) -> PaymentResult<()>;

    /// Store transaction
    async fn store_transaction(&self, transaction: &Transaction) -> PaymentResult<()>;

    /// Get payment transactions
    async fn get_payment_transactions(&self, payment_id: Uuid) -> PaymentResult<Vec<Transaction>>;

    /// Store refund
    async fn store_refund(&self, refund: &Refund) -> PaymentResult<()>;

    /// Get customer
    async fn get_customer(&self, customer_id: &str) -> PaymentResult<Option<Customer>>;
}

impl PaymentProcessor {
    /// Create a new payment processor
    pub fn new(
        config: ProcessorConfig,
        risk_engine: Arc<dyn RiskEngine + Send + Sync>,
        storage: Arc<dyn PaymentStorage + Send + Sync>,
    ) -> Self {
        Self {
            config: config.payment_config,
            gateways: HashMap::new(),
            risk_engine,
            storage,
            active_payments: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a payment gateway
    pub fn register_gateway(
        &mut self,
        gateway_id: String,
        gateway: Arc<dyn PaymentGateway + Send + Sync>,
    ) {
        self.gateways.insert(gateway_id, gateway);
    }

    /// Create a new payment intent
    pub async fn create_payment_intent(
        &self,
        amount: Amount,
        description: String,
        customer_id: Option<String>,
    ) -> PaymentResult<PaymentIntent> {
        // Validate amount
        amount.validate()?;

        if !self.config.supported_currencies.contains(&amount.currency) {
            return Err(PaymentError::UnsupportedCurrency {
                currency: amount.currency.to_string(),
            });
        }

        if !amount.is_positive() {
            return Err(PaymentError::InvalidAmount {
                message: "Amount must be positive".to_string(),
            });
        }

        // Create payment intent
        let mut intent = PaymentIntent::new(amount, description);
        intent.customer_id = customer_id;

        // Store in database
        self.storage.store_payment(&intent).await?;

        info!(
            payment_id = %intent.id,
            amount = %intent.amount,
            "Payment intent created"
        );

        Ok(intent)
    }

    /// Process a payment
    pub async fn process_payment(
        &self,
        payment_id: Uuid,
        payment_method: PaymentMethod,
    ) -> PaymentResult<Transaction> {
        // Get payment intent
        let mut payment = self.storage.get_payment(payment_id).await?;

        // Validate payment can be processed
        if !payment.can_be_processed() {
            return Err(PaymentError::PaymentExpired {
                payment_id: payment_id.to_string(),
            });
        }

        // Get customer for risk assessment
        let customer = if let Some(customer_id) = &payment.customer_id {
            self.storage.get_customer(customer_id).await?
        } else {
            None
        };

        // Risk assessment
        let risk_assessment = self
            .risk_engine
            .assess_risk(&payment, customer.as_ref())
            .await?;

        if self.risk_engine.should_block_payment(&risk_assessment).await {
            let reason = format!("Risk level: {:?}", risk_assessment.level);
            return Err(PaymentError::risk_blocked(reason));
        }

        // Update payment with method and set processing
        payment.payment_method = Some(payment_method.clone());
        payment.status = PaymentStatus::Processing;
        self.storage
            .update_payment_status(payment_id, PaymentStatus::Processing)
            .await?;

        // Select appropriate gateway
        let gateway_id = self.select_gateway(&payment_method, &payment.amount.currency)?;
        let gateway = self
            .gateways
            .get(&gateway_id)
            .ok_or_else(|| PaymentError::ConfigurationError {
                message: format!("Gateway not found: {}", gateway_id),
            })?;

        // Create payment session
        let session = PaymentSession {
            payment_id,
            status: PaymentStatus::Processing,
            gateway_id: gateway_id.clone(),
            attempts: 1,
            last_attempt: Utc::now(),
            metadata: HashMap::new(),
        };

        // Store session
        {
            let mut sessions = self.active_payments.write().await;
            sessions.insert(payment_id, session);
        }

        // Create transaction record
        let mut transaction = Transaction::new_with_payment_id(
            payment_id,
            TransactionType::Payment,
            payment.amount.clone(),
        );
        transaction.payment_method = payment_method.clone();
        transaction.user_id = payment.customer_id.clone().unwrap_or_default();
        transaction.description = payment.description.clone();

        // Process payment through gateway
        match gateway.process_payment(&payment, &payment_method).await {
            Ok(gateway_response) => {
                transaction.gateway_transaction_id = Some(gateway_response.transaction_id.clone());
                transaction.gateway_response = Some(gateway_response);
                transaction.mark_completed();

                // Update payment status
                self.storage
                    .update_payment_status(payment_id, PaymentStatus::Completed)
                    .await?;

                info!(
                    payment_id = %payment_id,
                    transaction_id = %transaction.id,
                    gateway = %gateway_id,
                    "Payment processed successfully"
                );
            }
            Err(e) => {
                transaction.mark_failed();

                // Update payment status
                self.storage
                    .update_payment_status(payment_id, PaymentStatus::Failed)
                    .await?;

                error!(
                    payment_id = %payment_id,
                    error = %e,
                    gateway = %gateway_id,
                    "Payment processing failed"
                );

                // Remove session
                {
                    let mut sessions = self.active_payments.write().await;
                    sessions.remove(&payment_id);
                }

                return Err(e);
            }
        }

        // Store transaction
        self.storage.store_transaction(&transaction).await?;

        // Remove session on success
        {
            let mut sessions = self.active_payments.write().await;
            sessions.remove(&payment_id);
        }

        Ok(transaction)
    }

    /// Refund a payment
    pub async fn refund_payment(
        &self,
        payment_id: Uuid,
        amount: Option<Amount>,
        reason: Option<String>,
    ) -> PaymentResult<Refund> {
        // Get original payment
        let payment = self.storage.get_payment(payment_id).await?;

        // Validate payment can be refunded
        if payment.status != PaymentStatus::Completed {
            return Err(PaymentError::RefundError {
                message: "Only completed payments can be refunded".to_string(),
            });
        }

        // Determine refund amount
        let refund_amount = amount.unwrap_or(payment.amount.clone());

        // Validate refund amount
        if refund_amount.currency != payment.amount.currency {
            return Err(PaymentError::RefundError {
                message: "Refund currency must match payment currency".to_string(),
            });
        }

        if refund_amount.value > payment.amount.value {
            return Err(PaymentError::RefundError {
                message: "Refund amount cannot exceed payment amount".to_string(),
            });
        }

        // Create refund record
        let mut refund = Refund::new(
            payment_id, 
            Uuid::new_v4(), // Generate a new transaction ID for the refund
            refund_amount, 
            reason
        );
        refund.status = PaymentStatus::Processing;

        // Get gateway from payment method
        if let Some(payment_method) = &payment.payment_method {
            let gateway_id = self.select_gateway(payment_method, &payment.amount.currency)?;
            let gateway = self
                .gateways
                .get(&gateway_id)
                .ok_or_else(|| PaymentError::ConfigurationError {
                    message: format!("Gateway not found: {}", gateway_id),
                })?;

            // Process refund through gateway
            match gateway.process_refund(&payment, &refund).await {
                Ok(gateway_response) => {
                    refund.gateway_refund_id = Some(gateway_response.transaction_id);
                    refund.status = PaymentStatus::Completed;
                    refund.processed_at = Some(Utc::now());

                    info!(
                        payment_id = %payment_id,
                        refund_id = %refund.id,
                        amount = %refund.amount,
                        "Refund processed successfully"
                    );
                }
                Err(e) => {
                    refund.status = PaymentStatus::Failed;
                    refund.processed_at = Some(Utc::now());

                    error!(
                        payment_id = %payment_id,
                        refund_id = %refund.id,
                        error = %e,
                        "Refund processing failed"
                    );

                    self.storage.store_refund(&refund).await?;
                    return Err(e);
                }
            }
        } else {
            return Err(PaymentError::RefundError {
                message: "Payment method not found".to_string(),
            });
        }

        // Store refund
        self.storage.store_refund(&refund).await?;

        Ok(refund)
    }

    /// Get payment status
    pub async fn get_payment_status(&self, payment_id: Uuid) -> PaymentResult<PaymentStatus> {
        let payment = self.storage.get_payment(payment_id).await?;
        Ok(payment.status)
    }

    /// Get payment details
    pub async fn get_payment(&self, payment_id: Uuid) -> PaymentResult<PaymentIntent> {
        self.storage.get_payment(payment_id).await
    }

    /// Get payment transactions
    pub async fn get_payment_transactions(
        &self,
        payment_id: Uuid,
    ) -> PaymentResult<Vec<Transaction>> {
        self.storage.get_payment_transactions(payment_id).await
    }

    /// Cancel a payment
    pub async fn cancel_payment(&self, payment_id: Uuid) -> PaymentResult<()> {
        let payment = self.storage.get_payment(payment_id).await?;

        if !matches!(payment.status, PaymentStatus::Pending | PaymentStatus::Processing) {
            return Err(PaymentError::ProcessingFailed {
                message: "Payment cannot be cancelled in current state".to_string(),
                code: Some("INVALID_STATE".to_string()),
            });
        }

        self.storage
            .update_payment_status(payment_id, PaymentStatus::Cancelled)
            .await?;

        // Remove from active sessions
        {
            let mut sessions = self.active_payments.write().await;
            sessions.remove(&payment_id);
        }

        info!(payment_id = %payment_id, "Payment cancelled");

        Ok(())
    }

    /// Retry a failed payment
    pub async fn retry_payment(&self, payment_id: Uuid) -> PaymentResult<Transaction> {
        // Check session for retry attempts
        let mut should_retry = false;
        {
            let sessions = self.active_payments.read().await;
            if let Some(session) = sessions.get(&payment_id) {
                if session.attempts < self.config.max_retry_attempts {
                    should_retry = true;
                }
            }
        }

        if !should_retry {
            return Err(PaymentError::ProcessingFailed {
                message: "Maximum retry attempts exceeded".to_string(),
                code: Some("MAX_RETRIES_EXCEEDED".to_string()),
            });
        }

        // Get payment and retry
        let payment = self.storage.get_payment(payment_id).await?;
        if let Some(payment_method) = payment.payment_method {
            self.process_payment(payment_id, payment_method).await
        } else {
            Err(PaymentError::ProcessingFailed {
                message: "Payment method not found for retry".to_string(),
                code: Some("NO_PAYMENT_METHOD".to_string()),
            })
        }
    }

    /// Select appropriate gateway for payment method and currency
    fn select_gateway(
        &self,
        _payment_method: &PaymentMethod,
        _currency: &Currency,
    ) -> PaymentResult<String> {
        // Simple implementation - return first available gateway
        // In production, this would involve complex routing logic
        self.gateways
            .keys()
            .next()
            .map(|k| k.clone())
            .ok_or_else(|| PaymentError::ConfigurationError {
                message: "No gateways configured".to_string(),
            })
    }

    /// Get active payment sessions count
    pub async fn get_active_sessions_count(&self) -> usize {
        let sessions = self.active_payments.read().await;
        sessions.len()
    }

    /// Health check
    pub async fn health_check(&self) -> PaymentResult<HashMap<String, String>> {
        let mut status = HashMap::new();

        // Check gateway health
        for (gateway_id, gateway) in &self.gateways {
            match gateway.health_check().await {
                Ok(_) => {
                    status.insert(format!("gateway_{}", gateway_id), "healthy".to_string());
                }
                Err(e) => {
                    status.insert(format!("gateway_{}", gateway_id), format!("unhealthy: {}", e));
                }
            }
        }

        // Check active sessions
        let active_sessions = self.get_active_sessions_count().await;
        status.insert("active_sessions".to_string(), active_sessions.to_string());

        Ok(status)
    }
}

/// Default retry configuration
impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Basic risk engine implementation
pub struct BasicRiskEngine {
    risk_threshold: u8,
}

impl BasicRiskEngine {
    pub fn new(risk_threshold: u8) -> Self {
        Self { risk_threshold }
    }
}

#[async_trait]
impl RiskEngine for BasicRiskEngine {
    async fn assess_risk(
        &self,
        payment: &PaymentIntent,
        _customer: Option<&Customer>,
    ) -> PaymentResult<RiskAssessment> {
        let mut score = 0u8;
        let mut factors = Vec::new();

        // Simple risk factors
        if payment.amount.value > rust_decimal::Decimal::new(100000, 2) {
            // > $1000
            score += 30;
            factors.push(crate::types::RiskFactor::UnusualAmount);
        }

        if payment.customer_id.is_none() {
            score += 20;
            factors.push(crate::types::RiskFactor::NewCustomer);
        }

        let level = match score {
            0..=25 => RiskLevel::Low,
            26..=50 => RiskLevel::Medium,
            51..=75 => RiskLevel::High,
            _ => RiskLevel::Critical,
        };

        Ok(RiskAssessment {
            score,
            level,
            factors,
            recommendations: vec!["Standard processing".to_string()],
            timestamp: Utc::now(),
        })
    }

    async fn should_block_payment(&self, assessment: &RiskAssessment) -> bool {
        assessment.score > self.risk_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FiatCurrency, Currency};
    use rust_decimal::Decimal;
    use std::sync::Arc;
    use tokio_test;

    // Mock implementations for testing
    struct MockGateway;
    
    #[async_trait]
    impl PaymentGateway for MockGateway {
        async fn process_payment(
            &self,
            _payment: &PaymentIntent,
            _method: &PaymentMethod,
        ) -> PaymentResult<crate::types::GatewayResponse> {
            Ok(crate::types::GatewayResponse {
                gateway_id: "mock".to_string(),
                transaction_id: "tx_123".to_string(),
                status_code: "success".to_string(),
                message: "Payment successful".to_string(),
                raw_response: serde_json::json!({}),
                timestamp: Utc::now(),
            })
        }

        async fn process_refund(
            &self,
            _payment: &PaymentIntent,
            _refund: &Refund,
        ) -> PaymentResult<crate::types::GatewayResponse> {
            Ok(crate::types::GatewayResponse {
                gateway_id: "mock".to_string(),
                transaction_id: "refund_123".to_string(),
                status_code: "success".to_string(),
                message: "Refund successful".to_string(),
                raw_response: serde_json::json!({}),
                timestamp: Utc::now(),
            })
        }

        async fn health_check(&self) -> PaymentResult<()> {
            Ok(())
        }
    }

    struct MockStorage;

    #[async_trait]
    impl PaymentStorage for MockStorage {
        async fn store_payment(&self, _payment: &PaymentIntent) -> PaymentResult<()> {
            Ok(())
        }

        async fn get_payment(&self, _payment_id: Uuid) -> PaymentResult<PaymentIntent> {
            let amount = Amount::new(Decimal::new(10000, 2), Currency::Fiat(FiatCurrency::USD))?;
            let mut payment = PaymentIntent::new(amount, "Test payment".to_string());
            payment.status = PaymentStatus::Completed; // For testing refunds
            // Add a payment method for refund testing
            payment.payment_method = Some(PaymentMethod::CreditCard {
                last_four: "4242".to_string(),
                brand: "Visa".to_string(),
                exp_month: 12,
                exp_year: 2025,
                holder_name: "Test User".to_string(),
            });
            Ok(payment)
        }

        async fn update_payment_status(
            &self,
            _payment_id: Uuid,
            _status: PaymentStatus,
        ) -> PaymentResult<()> {
            Ok(())
        }

        async fn store_transaction(&self, _transaction: &Transaction) -> PaymentResult<()> {
            Ok(())
        }

        async fn get_payment_transactions(&self, _payment_id: Uuid) -> PaymentResult<Vec<Transaction>> {
            Ok(vec![])
        }

        async fn store_refund(&self, _refund: &Refund) -> PaymentResult<()> {
            Ok(())
        }

        async fn get_customer(&self, _customer_id: &str) -> PaymentResult<Option<Customer>> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_payment_processor_creation() {
        let config = ProcessorConfig {
            payment_config: PaymentConfig {
                merchant_id: "test_merchant".to_string(),
                supported_currencies: vec![Currency::Fiat(FiatCurrency::USD)],
                supported_payment_methods: vec!["card".to_string()],
                webhook_url: None,
                return_url: None,
                cancel_url: None,
                auto_capture: true,
                capture_delay_hours: None,
                max_retry_attempts: 3,
            },
            gateway_configs: HashMap::new(),
            risk_threshold: 70,
            retry_config: RetryConfig::default(),
        };

        let risk_engine = Arc::new(BasicRiskEngine::new(70));
        let storage = Arc::new(MockStorage);

        let processor = PaymentProcessor::new(config, risk_engine, storage);
        assert_eq!(processor.gateways.len(), 0);
    }

    #[tokio::test]
    async fn test_create_payment_intent() {
        let config = ProcessorConfig {
            payment_config: PaymentConfig {
                merchant_id: "test_merchant".to_string(),
                supported_currencies: vec![Currency::Fiat(FiatCurrency::USD)],
                supported_payment_methods: vec!["card".to_string()],
                webhook_url: None,
                return_url: None,
                cancel_url: None,
                auto_capture: true,
                capture_delay_hours: None,
                max_retry_attempts: 3,
            },
            gateway_configs: HashMap::new(),
            risk_threshold: 70,
            retry_config: RetryConfig::default(),
        };

        let risk_engine = Arc::new(BasicRiskEngine::new(70));
        let storage = Arc::new(MockStorage);
        let processor = PaymentProcessor::new(config, risk_engine, storage);

        let amount = Amount::new(Decimal::new(10000, 2), Currency::Fiat(FiatCurrency::USD)).expect("Failed to create amount");
        let result = processor
            .create_payment_intent(amount, "Test payment".to_string(), None)
            .await;

        assert!(result.is_ok());
        let intent = result.unwrap();
        assert_eq!(intent.description, "Test payment");
        assert_eq!(intent.status, PaymentStatus::Pending);
    }
}