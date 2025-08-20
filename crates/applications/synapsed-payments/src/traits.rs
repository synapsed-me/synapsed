use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    error::{PaymentError, PaymentResult},
    types::{
        Amount, Currency, PaymentMethod, PaymentRequest, PaymentResponse, 
        Refund, Subscription, Transaction, TransactionStatus
    },
};

/// Core trait for payment processing
#[async_trait]
pub trait PaymentProcessor: Send + Sync {
    /// Process a payment transaction
    async fn process_payment(&self, request: PaymentRequest) -> PaymentResult<PaymentResponse>;
    
    /// Capture a previously authorized payment
    async fn capture_payment(&self, transaction_id: Uuid, amount: Option<Amount>) -> PaymentResult<Transaction>;
    
    /// Cancel/void a pending or authorized payment
    async fn cancel_payment(&self, transaction_id: Uuid) -> PaymentResult<Transaction>;
    
    /// Refund a completed payment
    async fn refund_payment(&self, transaction_id: Uuid, amount: Option<Amount>, reason: String) -> PaymentResult<Refund>;
    
    /// Get transaction details
    async fn get_transaction(&self, transaction_id: Uuid) -> PaymentResult<Transaction>;
    
    /// Get transactions for a user
    async fn get_user_transactions(&self, user_id: &str, limit: Option<u32>, offset: Option<u32>) -> PaymentResult<Vec<Transaction>>;
    
    /// Validate payment method
    async fn validate_payment_method(&self, payment_method: &PaymentMethod) -> PaymentResult<bool>;
}

/// Trait for payment gateway integration
#[async_trait]
pub trait PaymentGateway: Send + Sync {
    /// Gateway identifier (e.g., "stripe", "paypal", "square")
    fn gateway_id(&self) -> &str;
    
    /// Check if gateway supports the payment method
    fn supports_payment_method(&self, payment_method: &PaymentMethod) -> bool;
    
    /// Check if gateway supports the currency
    fn supports_currency(&self, currency: &Currency) -> bool;
    
    /// Create payment intent with gateway
    async fn create_payment_intent(&self, request: PaymentRequest) -> PaymentResult<String>;
    
    /// Confirm payment intent
    async fn confirm_payment_intent(&self, intent_id: &str, payment_method: &PaymentMethod) -> PaymentResult<Transaction>;
    
    /// Capture authorized payment
    async fn capture_payment(&self, gateway_transaction_id: &str, amount: Option<Amount>) -> PaymentResult<Transaction>;
    
    /// Refund payment through gateway
    async fn refund_payment(&self, gateway_transaction_id: &str, amount: Option<Amount>) -> PaymentResult<Refund>;
    
    /// Get transaction status from gateway
    async fn get_transaction_status(&self, gateway_transaction_id: &str) -> PaymentResult<TransactionStatus>;
    
    /// Handle webhook from gateway
    async fn handle_webhook(&self, payload: &[u8], signature: &str) -> PaymentResult<Option<Transaction>>;
}

/// Trait for storing payment data
#[async_trait]
pub trait PaymentStorage: Send + Sync {
    /// Store a new transaction
    async fn store_transaction(&self, transaction: &Transaction) -> PaymentResult<()>;
    
    /// Update existing transaction
    async fn update_transaction(&self, transaction: &Transaction) -> PaymentResult<()>;
    
    /// Get transaction by ID
    async fn get_transaction(&self, transaction_id: Uuid) -> PaymentResult<Option<Transaction>>;
    
    /// Get transactions by user ID
    async fn get_user_transactions(&self, user_id: &str, limit: u32, offset: u32) -> PaymentResult<Vec<Transaction>>;
    
    /// Store payment method (tokenized)
    async fn store_payment_method(&self, user_id: &str, payment_method: &PaymentMethod) -> PaymentResult<String>;
    
    /// Get user's payment methods
    async fn get_user_payment_methods(&self, user_id: &str) -> PaymentResult<Vec<PaymentMethod>>;
    
    /// Delete payment method
    async fn delete_payment_method(&self, user_id: &str, method_id: &str) -> PaymentResult<()>;
    
    /// Store refund
    async fn store_refund(&self, refund: &Refund) -> PaymentResult<()>;
    
    /// Get refund by ID
    async fn get_refund(&self, refund_id: Uuid) -> PaymentResult<Option<Refund>>;
    
    /// Get refunds for transaction
    async fn get_transaction_refunds(&self, transaction_id: Uuid) -> PaymentResult<Vec<Refund>>;
}

/// Trait for currency conversion
#[async_trait]
pub trait CurrencyConverter: Send + Sync {
    /// Get exchange rate between two currencies
    async fn get_exchange_rate(&self, from: &Currency, to: &Currency) -> PaymentResult<Decimal>;
    
    /// Convert amount from one currency to another
    async fn convert_amount(&self, amount: &Amount, to_currency: &Currency) -> PaymentResult<Amount>;
    
    /// Get supported currencies
    async fn get_supported_currencies(&self) -> PaymentResult<Vec<Currency>>;
    
    /// Get historical exchange rate
    async fn get_historical_rate(&self, from: &Currency, to: &Currency, date: DateTime<Utc>) -> PaymentResult<Decimal>;
}

/// Trait for fraud detection
#[async_trait]
pub trait FraudDetector: Send + Sync {
    /// Analyze transaction for fraud risk
    async fn analyze_transaction(&self, transaction: &Transaction) -> PaymentResult<FraudAnalysisResult>;
    
    /// Check if user is on blocklist
    async fn check_user_blocklist(&self, user_id: &str) -> PaymentResult<bool>;
    
    /// Check if payment method is suspicious
    async fn check_payment_method(&self, payment_method: &PaymentMethod) -> PaymentResult<bool>;
    
    /// Report fraudulent transaction
    async fn report_fraud(&self, transaction_id: Uuid, reason: String) -> PaymentResult<()>;
    
    /// Get fraud score for transaction
    async fn get_fraud_score(&self, transaction: &Transaction) -> PaymentResult<f64>;
}

/// Result of fraud analysis
#[derive(Debug, Clone)]
pub struct FraudAnalysisResult {
    /// Risk score (0.0 = low risk, 1.0 = high risk)
    pub risk_score: f64,
    /// Whether transaction should be blocked
    pub should_block: bool,
    /// Reasons for flagging
    pub flags: Vec<String>,
    /// Recommended action
    pub action: FraudAction,
}

/// Fraud detection actions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FraudAction {
    /// Allow transaction to proceed
    Allow,
    /// Hold transaction for review
    Review,
    /// Block transaction immediately
    Block,
    /// Require additional authentication
    RequireAuth,
}

/// Trait for subscription management
#[async_trait]
pub trait SubscriptionManager: Send + Sync {
    /// Create a new subscription
    async fn create_subscription(&self, subscription: Subscription) -> PaymentResult<Subscription>;
    
    /// Update subscription
    async fn update_subscription(&self, subscription: &Subscription) -> PaymentResult<()>;
    
    /// Cancel subscription
    async fn cancel_subscription(&self, subscription_id: Uuid) -> PaymentResult<()>;
    
    /// Pause subscription
    async fn pause_subscription(&self, subscription_id: Uuid) -> PaymentResult<()>;
    
    /// Resume subscription
    async fn resume_subscription(&self, subscription_id: Uuid) -> PaymentResult<()>;
    
    /// Get subscription by ID
    async fn get_subscription(&self, subscription_id: Uuid) -> PaymentResult<Option<Subscription>>;
    
    /// Get user's subscriptions
    async fn get_user_subscriptions(&self, user_id: &str) -> PaymentResult<Vec<Subscription>>;
    
    /// Process due subscriptions
    async fn process_due_subscriptions(&self) -> PaymentResult<Vec<Transaction>>;
    
    /// Get subscription transactions
    async fn get_subscription_transactions(&self, subscription_id: Uuid) -> PaymentResult<Vec<Transaction>>;
}

/// Trait for payment analytics
#[async_trait]
pub trait PaymentAnalytics: Send + Sync {
    /// Get payment volume for period
    async fn get_payment_volume(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> PaymentResult<Amount>;
    
    /// Get transaction count for period
    async fn get_transaction_count(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> PaymentResult<u64>;
    
    /// Get success rate for period
    async fn get_success_rate(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> PaymentResult<f64>;
    
    /// Get popular payment methods
    async fn get_popular_payment_methods(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> PaymentResult<Vec<(PaymentMethod, u64)>>;
    
    /// Get revenue by currency
    async fn get_revenue_by_currency(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> PaymentResult<HashMap<Currency, Amount>>;
    
    /// Get failed transaction reasons
    async fn get_failure_reasons(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> PaymentResult<HashMap<String, u64>>;
    
    /// Get average transaction amount
    async fn get_average_transaction_amount(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> PaymentResult<Amount>;
}

/// Trait for compliance checking
#[async_trait]
pub trait ComplianceChecker: Send + Sync {
    /// Check KYC (Know Your Customer) status
    async fn check_kyc(&self, user_id: &str) -> PaymentResult<KycStatus>;
    
    /// Check AML (Anti-Money Laundering) compliance
    async fn check_aml(&self, transaction: &Transaction) -> PaymentResult<AmlStatus>;
    
    /// Check PCI DSS compliance for payment method storage
    async fn check_pci_compliance(&self, payment_method: &PaymentMethod) -> PaymentResult<bool>;
    
    /// Check transaction limits
    async fn check_transaction_limits(&self, user_id: &str, amount: &Amount) -> PaymentResult<bool>;
    
    /// Check sanctions list
    async fn check_sanctions(&self, user_id: &str) -> PaymentResult<bool>;
    
    /// Generate compliance report
    async fn generate_compliance_report(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> PaymentResult<ComplianceReport>;
}

/// KYC verification status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KycStatus {
    NotVerified,
    Pending,
    Verified,
    Failed,
    Expired,
}

/// AML check status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmlStatus {
    Passed,
    Flagged,
    Blocked,
    Pending,
}

/// Compliance report
#[derive(Debug, Clone)]
pub struct ComplianceReport {
    /// Report period
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    /// Total transactions processed
    pub total_transactions: u64,
    /// Flagged transactions
    pub flagged_transactions: u64,
    /// Blocked transactions
    pub blocked_transactions: u64,
    /// KYC statistics
    pub kyc_stats: HashMap<KycStatus, u64>,
    /// AML statistics
    pub aml_stats: HashMap<AmlStatus, u64>,
}

/// Trait for notification handling
#[async_trait]
pub trait PaymentNotifier: Send + Sync {
    /// Send payment confirmation
    async fn send_payment_confirmation(&self, transaction: &Transaction) -> PaymentResult<()>;
    
    /// Send payment failure notification
    async fn send_payment_failure(&self, transaction: &Transaction, reason: String) -> PaymentResult<()>;
    
    /// Send refund notification
    async fn send_refund_notification(&self, refund: &Refund) -> PaymentResult<()>;
    
    /// Send subscription renewal notification
    async fn send_subscription_renewal(&self, subscription: &Subscription, transaction: &Transaction) -> PaymentResult<()>;
    
    /// Send subscription cancellation notification
    async fn send_subscription_cancellation(&self, subscription: &Subscription) -> PaymentResult<()>;
    
    /// Send fraud alert
    async fn send_fraud_alert(&self, transaction: &Transaction, analysis: &FraudAnalysisResult) -> PaymentResult<()>;
}

/// Trait for audit logging
#[async_trait]
pub trait PaymentAuditor: Send + Sync {
    /// Log payment event
    async fn log_payment_event(&self, event: PaymentEvent) -> PaymentResult<()>;
    
    /// Get audit log for transaction
    async fn get_transaction_audit_log(&self, transaction_id: Uuid) -> PaymentResult<Vec<PaymentEvent>>;
    
    /// Get audit log for user
    async fn get_user_audit_log(&self, user_id: &str) -> PaymentResult<Vec<PaymentEvent>>;
    
    /// Get audit log for period
    async fn get_audit_log(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> PaymentResult<Vec<PaymentEvent>>;
}

/// Payment audit event
#[derive(Debug, Clone)]
pub struct PaymentEvent {
    /// Event ID
    pub id: Uuid,
    /// Event type
    pub event_type: String,
    /// Associated transaction ID
    pub transaction_id: Option<Uuid>,
    /// Associated user ID
    pub user_id: Option<String>,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event data
    pub data: HashMap<String, String>,
    /// IP address of requester
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Currency, Amount};
    use rust_decimal_macros::dec;

    struct MockFraudDetector;

    #[async_trait]
    impl FraudDetector for MockFraudDetector {
        async fn analyze_transaction(&self, _transaction: &Transaction) -> PaymentResult<FraudAnalysisResult> {
            Ok(FraudAnalysisResult {
                risk_score: 0.1,
                should_block: false,
                flags: vec![],
                action: FraudAction::Allow,
            })
        }

        async fn check_user_blocklist(&self, _user_id: &str) -> PaymentResult<bool> {
            Ok(false)
        }

        async fn check_payment_method(&self, _payment_method: &PaymentMethod) -> PaymentResult<bool> {
            Ok(false)
        }

        async fn report_fraud(&self, _transaction_id: Uuid, _reason: String) -> PaymentResult<()> {
            Ok(())
        }

        async fn get_fraud_score(&self, _transaction: &Transaction) -> PaymentResult<f64> {
            Ok(0.1)
        }
    }

    #[tokio::test]
    async fn test_fraud_detector() {
        let detector = MockFraudDetector;
        let transaction = Transaction::new(
            Amount::new(dec!(100.00), Currency::Fiat(crate::types::FiatCurrency::USD)).unwrap(),
            PaymentMethod::Cash,
            "user123".to_string(),
            "Test transaction".to_string(),
        );

        let result = detector.analyze_transaction(&transaction).await.unwrap();
        assert_eq!(result.action, FraudAction::Allow);
        assert!(!result.should_block);
    }
}