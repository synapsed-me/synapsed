use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{PaymentError, PaymentResult};

/// Monetary amount with currency
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount {
    /// The amount value using Decimal for precise financial calculations
    pub value: Decimal,
    /// ISO 4217 currency code (e.g., "USD", "EUR", "BTC")
    pub currency: Currency,
}

impl Amount {
    /// Create a new amount
    pub fn new(value: Decimal, currency: Currency) -> PaymentResult<Self> {
        if value < Decimal::ZERO {
            return Err(PaymentError::InvalidAmount {
                message: format!("Amount cannot be negative: {}", value),
            });
        }
        Ok(Self { value, currency })
    }

    /// Create amount from string value
    pub fn from_str(value: &str, currency: Currency) -> PaymentResult<Self> {
        let decimal_value = value.parse::<Decimal>()
            .map_err(|_| PaymentError::InvalidAmount {
                message: format!("Invalid decimal value: {}", value),
            })?;
        Self::new(decimal_value, currency)
    }

    /// Convert to another currency (requires exchange rate)
    pub fn convert_to(&self, target_currency: Currency, exchange_rate: Decimal) -> PaymentResult<Self> {
        if self.currency == target_currency {
            return Ok(self.clone());
        }
        
        let converted_value = self.value * exchange_rate;
        Self::new(converted_value, target_currency)
    }

    /// Check if amount is zero
    pub fn is_zero(&self) -> bool {
        self.value == Decimal::ZERO
    }
    
    /// Check if amount is positive
    pub fn is_positive(&self) -> bool {
        self.value > Decimal::ZERO
    }

    /// Add two amounts (must be same currency)
    pub fn add(&self, other: &Amount) -> PaymentResult<Amount> {
        if self.currency != other.currency {
            return Err(PaymentError::CurrencyConversionFailed {
                from: self.currency.to_string(),
                to: other.currency.to_string(),
            });
        }
        Self::new(self.value + other.value, self.currency.clone())
    }

    /// Subtract two amounts (must be same currency)
    pub fn subtract(&self, other: &Amount) -> PaymentResult<Amount> {
        if self.currency != other.currency {
            return Err(PaymentError::CurrencyConversionFailed {
                from: self.currency.to_string(),
                to: other.currency.to_string(),
            });
        }
        Self::new(self.value - other.value, self.currency.clone())
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.value, self.currency)
    }
}

impl Validate for Amount {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        Ok(())
    }
}

/// Fiat currencies
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FiatCurrency {
    USD, // US Dollar
    EUR, // Euro
    GBP, // British Pound
    JPY, // Japanese Yen
    CAD, // Canadian Dollar
    AUD, // Australian Dollar
    CHF, // Swiss Franc
    CNY, // Chinese Yuan
}

/// Cryptocurrencies
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CryptoCurrency {
    Bitcoin,
    Ethereum,
    Litecoin,
    BitcoinCash,
    Ripple,
    Cardano,
    Polkadot,
    Chainlink,
    // Stablecoins
    Tether,
    USDCoin,
    DAI,
    BinanceUSD,
    // Substrate tokens
    Substrate(String),
}

/// Supported currencies
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    /// Fiat currency
    Fiat(FiatCurrency),
    /// Cryptocurrency
    Crypto(CryptoCurrency),
    /// Custom token
    Token(String),
}

impl Currency {
    /// Get currency symbol
    pub fn symbol(&self) -> &str {
        match self {
            Currency::Fiat(fiat) => match fiat {
                FiatCurrency::USD => "$",
                FiatCurrency::EUR => "€",
                FiatCurrency::GBP => "£",
                FiatCurrency::JPY => "¥",
                FiatCurrency::CAD => "C$",
                FiatCurrency::AUD => "A$",
                FiatCurrency::CHF => "CHF",
                FiatCurrency::CNY => "¥",
            },
            Currency::Crypto(crypto) => match crypto {
                CryptoCurrency::Bitcoin => "₿",
                CryptoCurrency::Ethereum => "Ξ",
                CryptoCurrency::Litecoin => "Ł",
                CryptoCurrency::BitcoinCash => "BCH",
                CryptoCurrency::Ripple => "XRP",
                CryptoCurrency::Cardano => "₳",
                CryptoCurrency::Polkadot => "DOT",
                CryptoCurrency::Chainlink => "LINK",
                CryptoCurrency::Tether => "USDT",
                CryptoCurrency::USDCoin => "USDC",
                CryptoCurrency::DAI => "DAI",
                CryptoCurrency::BinanceUSD => "BUSD",
                CryptoCurrency::Substrate(token) => token,
            },
            Currency::Token(name) => name,
        }
    }

    /// Check if currency is a cryptocurrency
    pub fn is_crypto(&self) -> bool {
        matches!(self, Currency::Crypto(_))
    }

    /// Check if currency is a fiat currency
    pub fn is_fiat(&self) -> bool {
        matches!(self, Currency::Fiat(_))
    }

    /// Get decimal places for currency
    pub fn decimal_places(&self) -> u32 {
        match self {
            Currency::Fiat(fiat) => match fiat {
                FiatCurrency::JPY => 0, // Yen has no decimal places
                _ => 2, // Most fiat currencies have 2 decimal places
            },
            Currency::Crypto(crypto) => match crypto {
                CryptoCurrency::Bitcoin => 8,
                CryptoCurrency::Ethereum => 18,
                _ => 8, // Default for most cryptocurrencies
            },
            Currency::Token(_) => 18, // Default for custom tokens
        }
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Currency::Fiat(fiat) => write!(f, "{:?}", fiat),
            Currency::Crypto(crypto) => write!(f, "{:?}", crypto),
            Currency::Token(name) => write!(f, "{}", name),
        }
    }
}

/// Transaction status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Transaction is being initialized
    Pending,
    /// Transaction is being processed by payment gateway
    Processing,
    /// Transaction completed successfully
    Completed,
    /// Transaction failed
    Failed,
    /// Transaction was cancelled
    Cancelled,
    /// Transaction expired
    Expired,
    /// Transaction is being refunded
    Refunding,
    /// Transaction was refunded
    Refunded,
    /// Transaction is on hold for review
    OnHold,
    /// Transaction requires additional authentication
    RequiresAuth,
}

impl TransactionStatus {
    /// Check if status is final (cannot be changed)
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            TransactionStatus::Completed
                | TransactionStatus::Failed
                | TransactionStatus::Cancelled
                | TransactionStatus::Expired
                | TransactionStatus::Refunded
        )
    }

    /// Check if status indicates success
    pub fn is_successful(&self) -> bool {
        matches!(self, TransactionStatus::Completed)
    }

    /// Check if status indicates failure
    pub fn is_failed(&self) -> bool {
        matches!(
            self,
            TransactionStatus::Failed | TransactionStatus::Cancelled | TransactionStatus::Expired
        )
    }
}

/// Payment method types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentMethod {
    /// Credit card payment
    CreditCard {
        /// Last 4 digits of card number
        last_four: String,
        /// Card brand (Visa, MasterCard, etc.)
        brand: String,
        /// Expiration month (1-12)
        exp_month: u8,
        /// Expiration year
        exp_year: u16,
        /// Cardholder name
        holder_name: String,
    },
    /// Debit card payment
    DebitCard {
        last_four: String,
        brand: String,
        exp_month: u8,
        exp_year: u16,
        holder_name: String,
    },
    /// Bank transfer
    BankTransfer {
        /// Bank name
        bank_name: String,
        /// Account type (checking, savings)
        account_type: String,
        /// Last 4 digits of account number
        last_four: String,
    },
    /// Digital wallet (PayPal, Apple Pay, Google Pay, etc.)
    DigitalWallet {
        /// Wallet provider
        provider: String,
        /// User identifier in wallet system
        user_id: String,
    },
    /// Cryptocurrency payment
    Cryptocurrency {
        /// Currency type
        currency: Currency,
        /// Wallet address
        address: String,
    },
    /// Substrate blockchain payment
    Substrate {
        /// Account ID on the substrate network
        account_id: String,
        /// Network name (e.g., "polkadot", "kusama")
        network: String,
        /// Token symbol (e.g., "DOT", "KSM")
        token: String,
    },
    /// Buy now, pay later services
    BuyNowPayLater {
        /// BNPL provider (Klarna, Afterpay, etc.)
        provider: String,
        /// Number of installments
        installments: u8,
    },
    /// Cash payment (for in-person transactions)
    Cash,
    /// Gift card or store credit
    GiftCard {
        /// Last 4 digits of gift card number
        last_four: String,
        /// Remaining balance
        balance: Amount,
    },
}

impl PaymentMethod {
    /// Get display name for payment method
    pub fn display_name(&self) -> String {
        match self {
            PaymentMethod::CreditCard { brand, last_four, .. } => {
                format!("{} •••• {}", brand, last_four)
            }
            PaymentMethod::DebitCard { brand, last_four, .. } => {
                format!("{} Debit •••• {}", brand, last_four)
            }
            PaymentMethod::BankTransfer { bank_name, last_four, .. } => {
                format!("{} •••• {}", bank_name, last_four)
            }
            PaymentMethod::DigitalWallet { provider, .. } => provider.clone(),
            PaymentMethod::Cryptocurrency { currency, .. } => {
                format!("{} Wallet", currency)
            }
            PaymentMethod::Substrate { network, token, .. } => {
                format!("{} {} Account", network, token)
            }
            PaymentMethod::BuyNowPayLater { provider, installments } => {
                format!("{} ({} payments)", provider, installments)
            }
            PaymentMethod::Cash => "Cash".to_string(),
            PaymentMethod::GiftCard { last_four, .. } => {
                format!("Gift Card •••• {}", last_four)
            }
        }
    }

    /// Check if payment method requires online processing
    pub fn requires_online_processing(&self) -> bool {
        !matches!(self, PaymentMethod::Cash)
    }

    /// Check if payment method supports refunds
    pub fn supports_refunds(&self) -> bool {
        !matches!(self, PaymentMethod::Cash | PaymentMethod::GiftCard { .. })
    }
}

/// Core transaction data
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Transaction {
    /// Unique transaction identifier
    pub id: Uuid,
    /// Associated payment ID
    pub payment_id: Uuid,
    /// Transaction amount
    pub amount: Amount,
    /// Payment method used
    pub payment_method: PaymentMethod,
    /// Current transaction status
    pub status: TransactionStatus,
    /// User who initiated the transaction
    pub user_id: String,
    /// Optional merchant/recipient identifier
    pub merchant_id: Option<String>,
    /// Transaction description
    pub description: String,
    /// Reference number for external systems
    pub reference: Option<String>,
    /// Transaction metadata
    pub metadata: HashMap<String, String>,
    /// When transaction was created
    pub created_at: DateTime<Utc>,
    /// When transaction was last updated
    pub updated_at: DateTime<Utc>,
    /// When transaction expires (for pending transactions)
    pub expires_at: Option<DateTime<Utc>>,
    /// Gateway transaction ID
    pub gateway_transaction_id: Option<String>,
    /// Gateway used for processing
    pub gateway: Option<String>,
    /// Transaction fees
    pub fees: Option<Amount>,
    /// Parent transaction ID (for refunds, recurring payments)
    pub parent_transaction_id: Option<Uuid>,
    /// Gateway response data
    pub gateway_response: Option<GatewayResponse>,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(
        amount: Amount,
        payment_method: PaymentMethod,
        user_id: String,
        description: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            payment_id: Uuid::new_v4(), // Generate a new payment ID
            amount,
            payment_method,
            status: TransactionStatus::Pending,
            user_id,
            merchant_id: None,
            description,
            reference: None,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            expires_at: None,
            gateway_transaction_id: None,
            gateway: None,
            fees: None,
            parent_transaction_id: None,
            gateway_response: None,
        }
    }
    
    /// Create a new transaction with payment ID and type
    pub fn new_with_payment_id(
        payment_id: Uuid,
        transaction_type: TransactionType,
        amount: Amount,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            payment_id, // Use the provided payment_id
            amount,
            payment_method: PaymentMethod::Cash, // Default, will be updated
            status: TransactionStatus::Pending,
            user_id: String::new(), // Will be set from payment intent
            merchant_id: None,
            description: String::new(),
            reference: Some(payment_id.to_string()),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            expires_at: None,
            gateway_transaction_id: None,
            gateway: None,
            fees: None,
            parent_transaction_id: if matches!(transaction_type, TransactionType::Refund) {
                Some(payment_id)
            } else {
                None
            },
            gateway_response: None,
        }
    }
    
    /// Mark transaction as completed
    pub fn mark_completed(&mut self) {
        self.status = TransactionStatus::Completed;
        self.updated_at = Utc::now();
    }
    
    /// Mark transaction as failed
    pub fn mark_failed(&mut self) {
        self.status = TransactionStatus::Failed;
        self.updated_at = Utc::now();
    }

    /// Update transaction status
    pub fn update_status(&mut self, status: TransactionStatus) -> PaymentResult<()> {
        if self.status.is_final() && status != self.status {
            return Err(PaymentError::TransactionAlreadyProcessed {
                transaction_id: self.id.to_string(),
            });
        }
        
        self.status = status;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Check if transaction is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Get net amount (amount minus fees)
    pub fn net_amount(&self) -> PaymentResult<Amount> {
        if let Some(ref fees) = self.fees {
            self.amount.subtract(fees)
        } else {
            Ok(self.amount.clone())
        }
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = Utc::now();
    }

    /// Set expiration time
    pub fn set_expiration(&mut self, expires_at: DateTime<Utc>) {
        self.expires_at = Some(expires_at);
        self.updated_at = Utc::now();
    }
}

/// Payment request from client
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PaymentRequest {
    /// Amount to charge
    pub amount: Amount,
    /// Payment method to use
    pub payment_method: PaymentMethod,
    /// User making the payment
    pub user_id: String,
    /// Payment description
    pub description: String,
    /// Optional merchant ID
    pub merchant_id: Option<String>,
    /// Return URL for redirects
    #[validate(url)]
    pub return_url: Option<String>,
    /// Cancel URL for redirects
    #[validate(url)]
    pub cancel_url: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Idempotency key to prevent duplicate payments
    pub idempotency_key: Option<String>,
}

/// Payment response to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResponse {
    /// Transaction that was created
    pub transaction: Transaction,
    /// Whether payment requires additional action (3D Secure, etc.)
    pub requires_action: bool,
    /// Client secret for frontend processing
    pub client_secret: Option<String>,
    /// Redirect URL if payment requires redirect
    pub redirect_url: Option<String>,
    /// Additional response data
    pub additional_data: HashMap<String, String>,
}

/// Sensitive payment data that should be zeroized
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct SensitivePaymentData {
    /// Full credit card number (PCI sensitive)
    pub card_number: Option<String>,
    /// CVV code (PCI sensitive)
    pub cvv: Option<String>,
    /// Bank account number (sensitive)
    pub account_number: Option<String>,
    /// Bank routing number (sensitive)
    pub routing_number: Option<String>,
    /// Cryptocurrency private key (extremely sensitive)
    pub private_key: Option<String>,
}

/// Refund information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refund {
    /// Unique refund identifier
    pub id: Uuid,
    /// Original payment ID
    pub payment_id: Uuid,
    /// Original transaction being refunded
    pub transaction_id: Uuid,
    /// Refund amount (can be partial)
    pub amount: Amount,
    /// Reason for refund
    pub reason: Option<String>,
    /// Refund status
    pub status: PaymentStatus,
    /// When refund was initiated
    pub created_at: DateTime<Utc>,
    /// When refund was processed
    pub processed_at: Option<DateTime<Utc>>,
    /// Gateway refund ID
    pub gateway_refund_id: Option<String>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl Refund {
    /// Create a new refund
    pub fn new(payment_id: Uuid, transaction_id: Uuid, amount: Amount, reason: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            payment_id,
            transaction_id,
            amount,
            reason,
            status: PaymentStatus::Pending,
            created_at: Utc::now(),
            processed_at: None,
            gateway_refund_id: None,
            metadata: HashMap::new(),
        }
    }
}

/// Recurring payment subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    /// Unique subscription identifier
    pub id: Uuid,
    /// User who owns the subscription
    pub user_id: String,
    /// Subscription amount
    pub amount: Amount,
    /// Payment method for recurring charges
    pub payment_method: PaymentMethod,
    /// Billing interval
    pub interval: BillingInterval,
    /// Subscription status
    pub status: SubscriptionStatus,
    /// Next billing date
    pub next_billing_date: DateTime<Utc>,
    /// When subscription was created
    pub created_at: DateTime<Utc>,
    /// When subscription was last updated
    pub updated_at: DateTime<Utc>,
    /// When subscription ends (if applicable)
    pub ends_at: Option<DateTime<Utc>>,
    /// Trial period end date
    pub trial_ends_at: Option<DateTime<Utc>>,
    /// Number of billing cycles (None for unlimited)
    pub billing_cycles: Option<u32>,
    /// Current billing cycle
    pub current_cycle: u32,
}

/// Billing intervals for subscriptions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BillingInterval {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
    Custom { days: u32 },
}

/// Subscription status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    Active,
    Paused,
    Cancelled,
    Expired,
    PastDue,
    Trialing,
}

/// Payment status (different from TransactionStatus, used for payment intents)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    /// Payment intent created but not yet processed
    Pending,
    /// Payment is being processed
    Processing,
    /// Payment completed successfully
    Completed,
    /// Payment failed
    Failed,
    /// Payment was cancelled
    Cancelled,
    /// Payment requires additional action
    RequiresAction,
    /// Payment expired
    Expired,
}

/// Transaction type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    /// Regular payment
    Payment,
    /// Refund transaction
    Refund,
    /// Pre-authorization
    PreAuth,
    /// Capture of pre-authorized amount
    Capture,
    /// Void/cancellation
    Void,
    /// Recurring payment
    Recurring,
    /// Fee transaction
    Fee,
    /// Payout/withdrawal
    Payout,
}

/// Customer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    /// Unique customer ID
    pub id: String,
    /// Customer email
    pub email: String,
    /// Customer name
    pub name: Option<String>,
    /// Phone number
    pub phone: Option<String>,
    /// Billing address
    pub billing_address: Option<Address>,
    /// Shipping address  
    pub shipping_address: Option<Address>,
    /// Customer metadata
    pub metadata: HashMap<String, String>,
    /// When customer was created
    pub created_at: DateTime<Utc>,
    /// When customer was last updated
    pub updated_at: DateTime<Utc>,
}

/// Address information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    /// Street address line 1
    pub line1: String,
    /// Street address line 2
    pub line2: Option<String>,
    /// City
    pub city: String,
    /// State/Province
    pub state: Option<String>,
    /// Postal/ZIP code
    pub postal_code: String,
    /// Country code (ISO 3166-1 alpha-2)
    pub country: String,
}

/// Payment intent - represents intent to collect payment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntent {
    /// Unique ID
    pub id: Uuid,
    /// Amount to collect
    pub amount: Amount,
    /// Payment status
    pub status: PaymentStatus,
    /// Description
    pub description: String,
    /// Customer ID
    pub customer_id: Option<String>,
    /// Payment method to use
    pub payment_method: Option<PaymentMethod>,
    /// When the payment intent was created
    pub created_at: DateTime<Utc>,
    /// When the payment intent was last updated
    pub updated_at: DateTime<Utc>,
    /// When the payment intent expires
    pub expires_at: Option<DateTime<Utc>>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl PaymentIntent {
    /// Create new payment intent
    pub fn new(amount: Amount, description: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            amount,
            status: PaymentStatus::Pending,
            description,
            customer_id: None,
            payment_method: None,
            created_at: now,
            updated_at: now,
            expires_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Check if payment intent is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Check if payment can be processed
    pub fn can_be_processed(&self) -> bool {
        !self.is_expired() && matches!(self.status, PaymentStatus::Pending | PaymentStatus::RequiresAction)
    }
}

/// Payment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentConfig {
    /// Merchant ID
    pub merchant_id: String,
    /// Supported currencies
    pub supported_currencies: Vec<Currency>,
    /// Supported payment methods
    pub supported_payment_methods: Vec<String>,
    /// Webhook URL for notifications
    pub webhook_url: Option<String>,
    /// Return URL after payment
    pub return_url: Option<String>,
    /// Cancel URL for cancelled payments
    pub cancel_url: Option<String>,
    /// Auto-capture payments
    pub auto_capture: bool,
    /// Capture delay in hours
    pub capture_delay_hours: Option<u32>,
    /// Maximum retry attempts
    pub max_retry_attempts: u8,
}

/// Risk level for transactions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk - proceed normally
    Low,
    /// Medium risk - may require additional verification
    Medium,
    /// High risk - require manual review
    High,
    /// Critical risk - block transaction
    Critical,
}

/// Risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Risk score (0-100)
    pub score: u8,
    /// Risk level
    pub level: RiskLevel,
    /// Risk factors that contributed to the score
    pub factors: Vec<RiskFactor>,
    /// Recommendations for handling the payment
    pub recommendations: Vec<String>,
    /// When the assessment was performed
    pub timestamp: DateTime<Utc>,
}

/// Risk factors
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskFactor {
    /// New customer
    NewCustomer,
    /// Unusual transaction amount
    UnusualAmount,
    /// High-risk country
    HighRiskCountry,
    /// Velocity limit exceeded
    VelocityLimit,
    /// Suspicious card
    SuspiciousCard,
    /// Multiple failed attempts
    FailedAttempts,
    /// Mismatched billing/shipping
    AddressMismatch,
    /// Known fraudulent pattern
    FraudPattern,
}

/// Gateway response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayResponse {
    /// Gateway identifier
    pub gateway_id: String,
    /// Gateway transaction ID
    pub transaction_id: String,
    /// Response status code
    pub status_code: String,
    /// Response message
    pub message: String,
    /// Raw response data
    pub raw_response: serde_json::Value,
    /// Response timestamp
    pub timestamp: DateTime<Utc>,
}

/// Card brand
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardBrand {
    Visa,
    Mastercard,
    AmericanExpress,
    Discover,
    DinersClub,
    JCB,
    UnionPay,
    Other(String),
}

/// Gateway capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayCapabilities {
    pub supports_cards: bool,
    pub supports_bank_transfers: bool,
    pub supports_crypto: bool,
    pub supports_wallets: bool,
    pub supports_subscriptions: bool,
    pub supports_3ds: bool,
    pub supports_refunds: bool,
    pub supports_webhooks: bool,
    pub currencies: Vec<Currency>,
    pub countries: Vec<String>,
}

/// Webhook event from payment gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub id: String,
    pub event_type: String,
    pub payment_id: Option<Uuid>,
    pub transaction_id: Option<String>,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub gateway_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_amount_creation() {
        let amount = Amount::new(dec!(100.50), Currency::Fiat(FiatCurrency::USD)).unwrap();
        assert_eq!(amount.value, dec!(100.50));
        assert_eq!(amount.currency, Currency::Fiat(FiatCurrency::USD));
    }

    #[test]
    fn test_amount_invalid() {
        let result = Amount::new(dec!(-10.00), Currency::Fiat(FiatCurrency::USD));
        assert!(result.is_err());
    }

    #[test]
    fn test_amount_addition() {
        let amount1 = Amount::new(dec!(100.00), Currency::Fiat(FiatCurrency::USD)).unwrap();
        let amount2 = Amount::new(dec!(50.00), Currency::Fiat(FiatCurrency::USD)).unwrap();
        let result = amount1.add(&amount2).unwrap();
        assert_eq!(result.value, dec!(150.00));
    }

    #[test]
    fn test_currency_display() {
        assert_eq!(Currency::Fiat(FiatCurrency::USD).symbol(), "$");
        assert_eq!(Currency::Fiat(FiatCurrency::EUR).symbol(), "€");
        assert!(Currency::Crypto(CryptoCurrency::Bitcoin).is_crypto());
        assert!(Currency::Fiat(FiatCurrency::USD).is_fiat());
    }

    #[test]
    fn test_transaction_creation() {
        let amount = Amount::new(dec!(99.99), Currency::Fiat(FiatCurrency::USD)).unwrap();
        let payment_method = PaymentMethod::CreditCard {
            last_four: "1234".to_string(),
            brand: "Visa".to_string(),
            exp_month: 12,
            exp_year: 2025,
            holder_name: "John Doe".to_string(),
        };
        
        let transaction = Transaction::new(
            amount,
            payment_method,
            "user123".to_string(),
            "Test payment".to_string(),
        );
        
        assert_eq!(transaction.status, TransactionStatus::Pending);
        assert_eq!(transaction.user_id, "user123");
    }

    #[test]
    fn test_transaction_status_transitions() {
        let mut transaction = Transaction::new(
            Amount::new(dec!(50.00), Currency::Fiat(FiatCurrency::USD)).unwrap(),
            PaymentMethod::Cash,
            "user123".to_string(),
            "Test".to_string(),
        );

        assert!(transaction.update_status(TransactionStatus::Completed).is_ok());
        assert_eq!(transaction.status, TransactionStatus::Completed);

        // Should not allow status change after completion
        assert!(transaction.update_status(TransactionStatus::Failed).is_err());
    }
}