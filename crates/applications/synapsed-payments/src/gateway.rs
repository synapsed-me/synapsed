use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::error::{PaymentError, PaymentResult};
use crate::types::{
    Currency, GatewayResponse, PaymentIntent, PaymentMethod, Refund,
    WebhookEvent,
};

/// Payment gateway trait - all gateways must implement this
#[async_trait]
pub trait PaymentGateway {
    /// Process a payment through this gateway
    async fn process_payment(
        &self,
        payment: &PaymentIntent,
        method: &PaymentMethod,
    ) -> PaymentResult<GatewayResponse>;

    /// Process a refund through this gateway
    async fn process_refund(
        &self,
        payment: &PaymentIntent,
        refund: &Refund,
    ) -> PaymentResult<GatewayResponse>;

    /// Health check for the gateway
    async fn health_check(&self) -> PaymentResult<()>;

    /// Get gateway capabilities (optional)
    async fn get_capabilities(&self) -> PaymentResult<crate::types::GatewayCapabilities> {
        // Default implementation with basic capabilities
        Ok(crate::types::GatewayCapabilities {
            supports_cards: true,
            supports_bank_transfers: false,
            supports_crypto: false,
            supports_wallets: false,
            supports_subscriptions: false,
            supports_3ds: false,
            supports_refunds: true,
            supports_webhooks: false,
            currencies: vec![Currency::Fiat(crate::types::FiatCurrency::USD)],
            countries: vec!["US".to_string()],
        })
    }

    /// Verify webhook signature (optional)
    async fn verify_webhook(
        &self,
        _payload: &[u8],
        _signature: &str,
        _secret: &str,
    ) -> PaymentResult<bool> {
        Ok(false) // Default: no webhook support
    }

    /// Parse webhook event (optional)
    async fn parse_webhook(&self, _payload: &[u8]) -> PaymentResult<Option<WebhookEvent>> {
        Ok(None) // Default: no webhook support
    }
}

/// Gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub gateway_id: String,
    pub name: String,
    pub enabled: bool,
    pub api_url: String,
    pub timeout_seconds: u64,
    pub max_retries: u8,
    pub supported_currencies: Vec<Currency>,
    pub supported_payment_methods: Vec<String>,
    pub webhook_config: Option<WebhookConfig>,
    pub custom_config: HashMap<String, serde_json::Value>,
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub verify_signature: bool,
}

/// HTTP-based payment gateway implementation
#[cfg(feature = "http-gateway")]
pub struct HttpPaymentGateway {
    config: GatewayConfig,
    client: reqwest::Client,
}

#[cfg(feature = "http-gateway")]
impl HttpPaymentGateway {
    /// Create a new HTTP gateway
    pub fn new(config: GatewayConfig) -> PaymentResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| PaymentError::NetworkError {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(Self { config, client })
    }

    /// Make authenticated request to gateway
    async fn make_request<T: serde::de::DeserializeOwned>(
        &self,
        method: reqwest::Method,
        endpoint: &str,
        body: Option<serde_json::Value>,
    ) -> PaymentResult<T> {
        let url = format!("{}/{}", self.config.api_url.trim_end_matches('/'), endpoint);
        
        let mut request = self.client.request(method, &url);

        // Add authentication headers (implementation depends on gateway)
        request = self.add_auth_headers(request)?;

        // Add body if provided
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await.map_err(|e| PaymentError::NetworkError {
            message: format!("Request failed: {}", e),
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(PaymentError::GatewayError {
                gateway: self.config.gateway_id.clone(),
                message: format!("HTTP {}: {}", status, text),
            });
        }

        response.json().await.map_err(|e| PaymentError::SerializationError {
            message: format!("Failed to parse response: {}", e),
        })
    }

    /// Add authentication headers (to be customized per gateway)
    fn add_auth_headers(&self, request: reqwest::RequestBuilder) -> PaymentResult<reqwest::RequestBuilder> {
        // This is a placeholder - each gateway will have different auth methods
        let api_key = self.config.custom_config
            .get("api_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PaymentError::ConfigurationError {
                message: "API key not configured".to_string(),
            })?;

        Ok(request.header("Authorization", format!("Bearer {}", api_key)))
    }

    /// Convert payment method to gateway-specific format
    fn format_payment_method(&self, method: &PaymentMethod) -> PaymentResult<serde_json::Value> {
        match method {
            PaymentMethod::CreditCard { last_four, brand, exp_month, exp_year, holder_name, .. } => {
                Ok(serde_json::json!({
                    "type": "card",
                    "last_four": last_four,
                    "brand": brand,
                    "exp_month": exp_month,
                    "exp_year": exp_year,
                    "holder_name": holder_name
                }))
            }
            PaymentMethod::DebitCard { last_four, brand, exp_month, exp_year, holder_name, .. } => {
                Ok(serde_json::json!({
                    "type": "debit_card",
                    "last_four": last_four,
                    "brand": brand,
                    "exp_month": exp_month,
                    "exp_year": exp_year,
                    "holder_name": holder_name
                }))
            }
            PaymentMethod::BankTransfer { bank_name, account_type, last_four, .. } => {
                Ok(serde_json::json!({
                    "type": "bank_transfer",
                    "bank_name": bank_name,
                    "account_type": account_type,
                    "last_four": last_four
                }))
            }
            PaymentMethod::DigitalWallet { provider, user_id } => {
                Ok(serde_json::json!({
                    "type": "digital_wallet",
                    "provider": provider,
                    "user_id": user_id
                }))
            }
            PaymentMethod::Cryptocurrency { currency, address, .. } => {
                Ok(serde_json::json!({
                    "type": "cryptocurrency",
                    "currency": currency.to_string(),
                    "address": address
                }))
            }
            _ => Err(PaymentError::InvalidPaymentMethod {
                method: "Unsupported payment method for HTTP gateway".to_string(),
            }),
        }
    }
}

#[cfg(feature = "http-gateway")]
#[async_trait]
impl PaymentGateway for HttpPaymentGateway {
    async fn process_payment(
        &self,
        payment: &PaymentIntent,
        method: &PaymentMethod,
    ) -> PaymentResult<GatewayResponse> {
        let payment_method_data = self.format_payment_method(method)?;

        let request_body = serde_json::json!({
            "amount": payment.amount.value,
            "currency": payment.amount.currency,
            "description": payment.description,
            "payment_method": payment_method_data,
            "idempotency_key": payment.id,
            "metadata": payment.metadata
        });

        #[derive(Deserialize)]
        struct PaymentResponse {
            id: String,
            status: String,
            message: Option<String>,
        }

        let response: PaymentResponse = self
            .make_request(reqwest::Method::POST, "payments", Some(request_body))
            .await?;

        Ok(GatewayResponse {
            gateway_id: self.config.gateway_id.clone(),
            transaction_id: response.id,
            status_code: response.status,
            message: response.message.unwrap_or_default(),
            raw_response: serde_json::json!({
                "id": response.id,
                "status": response.status,
                "message": response.message
            }),
            timestamp: Utc::now(),
        })
    }

    async fn process_refund(
        &self,
        _payment: &PaymentIntent,
        refund: &Refund,
    ) -> PaymentResult<GatewayResponse> {
        let request_body = serde_json::json!({
            "payment_id": refund.payment_id,
            "amount": refund.amount.value,
            "currency": refund.amount.currency,
            "reason": refund.reason,
            "idempotency_key": refund.id
        });

        #[derive(Deserialize)]
        struct RefundResponse {
            id: String,
            status: String,
            message: Option<String>,
        }

        let response: RefundResponse = self
            .make_request(reqwest::Method::POST, "refunds", Some(request_body))
            .await?;

        Ok(GatewayResponse {
            gateway_id: self.config.gateway_id.clone(),
            transaction_id: response.id,
            status_code: response.status,
            message: response.message.unwrap_or_default(),
            raw_response: serde_json::json!({
                "id": response.id,
                "status": response.status,
                "message": response.message
            }),
            timestamp: Utc::now(),
        })
    }

    async fn health_check(&self) -> PaymentResult<()> {
        #[derive(Deserialize)]
        struct HealthResponse {
            status: String,
        }

        let response: HealthResponse = self
            .make_request(reqwest::Method::GET, "health", None)
            .await?;

        if response.status == "ok" {
            Ok(())
        } else {
            Err(PaymentError::GatewayError {
                gateway: self.config.gateway_id.clone(),
                message: format!("Health check failed: {}", response.status),
            })
        }
    }

    async fn verify_webhook(
        &self,
        payload: &[u8],
        signature: &str,
        secret: &str,
    ) -> PaymentResult<bool> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| PaymentError::CryptographyError {
                message: format!("Invalid secret key: {}", e),
            })?;

        mac.update(payload);
        let expected = mac.finalize().into_bytes();
        let expected_hex = hex::encode(expected);

        // Remove 'sha256=' prefix if present
        let signature = signature.strip_prefix("sha256=").unwrap_or(signature);

        Ok(expected_hex == signature)
    }

    async fn parse_webhook(&self, payload: &[u8]) -> PaymentResult<Option<WebhookEvent>> {
        let raw_event: serde_json::Value = serde_json::from_slice(payload)?;

        let event = WebhookEvent {
            id: raw_event["id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            event_type: raw_event["type"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            payment_id: raw_event["data"]["object"]["id"]
                .as_str()
                .and_then(|s| s.parse().ok()),
            transaction_id: None,
            data: raw_event,
            timestamp: Utc::now(),
            gateway_id: self.config.gateway_id.clone(),
        };

        Ok(Some(event))
    }
}

/// Mock payment gateway for testing
pub struct MockPaymentGateway {
    pub gateway_id: String,
    pub should_fail: bool,
    pub delay_ms: Option<u64>,
}

impl MockPaymentGateway {
    pub fn new(gateway_id: String) -> Self {
        Self {
            gateway_id,
            should_fail: false,
            delay_ms: None,
        }
    }

    pub fn with_failure(mut self, should_fail: bool) -> Self {
        self.should_fail = should_fail;
        self
    }

    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = Some(delay_ms);
        self
    }
}

#[async_trait]
impl PaymentGateway for MockPaymentGateway {
    async fn process_payment(
        &self,
        payment: &PaymentIntent,
        _method: &PaymentMethod,
    ) -> PaymentResult<GatewayResponse> {
        if let Some(delay) = self.delay_ms {
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }

        if self.should_fail {
            return Err(PaymentError::GatewayError {
                gateway: self.gateway_id.clone(),
                message: "Mock gateway failure".to_string(),
            });
        }

        Ok(GatewayResponse {
            gateway_id: self.gateway_id.clone(),
            transaction_id: format!("mock_tx_{}", payment.id),
            status_code: "success".to_string(),
            message: "Mock payment processed".to_string(),
            raw_response: serde_json::json!({
                "mock": true,
                "payment_id": payment.id
            }),
            timestamp: Utc::now(),
        })
    }

    async fn process_refund(
        &self,
        _payment: &PaymentIntent,
        refund: &Refund,
    ) -> PaymentResult<GatewayResponse> {
        if let Some(delay) = self.delay_ms {
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }

        if self.should_fail {
            return Err(PaymentError::RefundError {
                message: "Mock refund failure".to_string(),
            });
        }

        Ok(GatewayResponse {
            gateway_id: self.gateway_id.clone(),
            transaction_id: format!("mock_refund_{}", refund.id),
            status_code: "success".to_string(),
            message: "Mock refund processed".to_string(),
            raw_response: serde_json::json!({
                "mock": true,
                "refund_id": refund.id
            }),
            timestamp: Utc::now(),
        })
    }

    async fn health_check(&self) -> PaymentResult<()> {
        if self.should_fail {
            Err(PaymentError::GatewayError {
                gateway: self.gateway_id.clone(),
                message: "Mock health check failure".to_string(),
            })
        } else {
            Ok(())
        }
    }
}

/// Gateway factory for creating gateway instances
pub struct GatewayFactory;

impl GatewayFactory {
    /// Create a gateway instance from configuration
    pub fn create_gateway(
        config: GatewayConfig,
    ) -> PaymentResult<Box<dyn PaymentGateway + Send + Sync>> {
        match config.name.as_str() {
            #[cfg(feature = "http-gateway")]
            "http" => {
                let gateway = HttpPaymentGateway::new(config)?;
                Ok(Box::new(gateway))
            }
            "mock" => {
                let gateway = MockPaymentGateway::new(config.gateway_id);
                Ok(Box::new(gateway))
            }
            _ => Err(PaymentError::ConfigurationError {
                message: format!("Unknown gateway type: {}", config.name),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Amount, FiatCurrency, PaymentIntent, PaymentMethod};
    use rust_decimal::Decimal;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_mock_gateway_success() {
        let gateway = MockPaymentGateway::new("test_gateway".to_string());
        
        let amount = Amount::new(Decimal::new(10000, 2), Currency::Fiat(FiatCurrency::USD)).expect("Failed to create amount");
        let payment = PaymentIntent::new(amount, "Test payment".to_string());
        
        let method = PaymentMethod::CreditCard {
            last_four: "4242".to_string(),
            brand: "Visa".to_string(),
            exp_month: 12,
            exp_year: 2025,
            holder_name: "Test User".to_string(),
        };

        let result = gateway.process_payment(&payment, &method).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response.gateway_id, "test_gateway");
        assert_eq!(response.status_code, "success");
    }

    #[tokio::test]
    async fn test_mock_gateway_failure() {
        let gateway = MockPaymentGateway::new("test_gateway".to_string())
            .with_failure(true);
        
        let amount = Amount::new(Decimal::new(10000, 2), Currency::Fiat(FiatCurrency::USD)).expect("Failed to create amount");
        let payment = PaymentIntent::new(amount, "Test payment".to_string());
        
        let method = PaymentMethod::CreditCard {
            last_four: "4242".to_string(),
            brand: "Visa".to_string(),
            exp_month: 12,
            exp_year: 2025,
            holder_name: "Test User".to_string(),
        };

        let result = gateway.process_payment(&payment, &method).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_gateway_health_check() {
        let gateway = MockPaymentGateway::new("test_gateway".to_string());
        let result = gateway.health_check().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_gateway_factory() {
        let config = GatewayConfig {
            gateway_id: "test".to_string(),
            name: "mock".to_string(),
            enabled: true,
            api_url: "https://api.example.com".to_string(),
            timeout_seconds: 30,
            max_retries: 3,
            supported_currencies: vec![Currency::Fiat(FiatCurrency::USD)],
            supported_payment_methods: vec!["card".to_string()],
            webhook_config: None,
            custom_config: HashMap::new(),
        };

        let result = GatewayFactory::create_gateway(config);
        assert!(result.is_ok());
    }
}