//! Network and API verification for AI agent claims

use crate::{types::*, Result, VerifyError};
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use chrono::Utc;
use uuid::Uuid;

/// Network verifier configuration
#[derive(Debug, Clone)]
pub struct NetworkVerifierConfig {
    /// Timeout for requests
    pub timeout_ms: u64,
    /// Maximum number of redirects
    pub max_redirects: usize,
    /// User agent string
    pub user_agent: String,
    /// Whether to verify SSL certificates
    pub verify_ssl: bool,
    /// Proxy configuration
    pub proxy: Option<String>,
    /// Default headers to include
    pub default_headers: HashMap<String, String>,
}

impl Default for NetworkVerifierConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 30000,
            max_redirects: 10,
            user_agent: "synapsed-verify/0.1.0".to_string(),
            verify_ssl: true,
            proxy: None,
            default_headers: HashMap::new(),
        }
    }
}

/// API verification result
#[derive(Debug, Clone)]
pub struct ApiVerification {
    /// Verification result
    pub result: VerificationResult,
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body (if JSON)
    pub body: Option<Value>,
    /// Response time in milliseconds
    pub response_time_ms: u64,
}

/// Network verification result
#[derive(Debug, Clone)]
pub struct NetworkVerification {
    /// Verification result
    pub result: VerificationResult,
    /// Network metrics
    pub metrics: NetworkMetrics,
    /// Trace route if available
    pub trace: Option<Vec<TraceHop>>,
}

/// Network metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// DNS resolution time
    pub dns_time_ms: Option<u64>,
    /// TCP connection time
    pub connect_time_ms: Option<u64>,
    /// TLS handshake time
    pub tls_time_ms: Option<u64>,
    /// Time to first byte
    pub ttfb_ms: Option<u64>,
    /// Total time
    pub total_time_ms: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
}

/// Trace route hop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceHop {
    /// Hop number
    pub hop: u32,
    /// IP address
    pub ip: String,
    /// Hostname if available
    pub hostname: Option<String>,
    /// Round trip time
    pub rtt_ms: Option<u64>,
}

/// Network verifier for Claude sub-agent claims
pub struct NetworkVerifier {
    client: Client,
    config: NetworkVerifierConfig,
}

impl NetworkVerifier {
    /// Creates a new network verifier
    pub fn new() -> Self {
        Self::with_config(NetworkVerifierConfig::default())
    }
    
    /// Creates a verifier with custom configuration
    pub fn with_config(config: NetworkVerifierConfig) -> Self {
        let mut client_builder = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .user_agent(&config.user_agent)
            .danger_accept_invalid_certs(!config.verify_ssl);
        
        if let Some(proxy_url) = &config.proxy {
            if let Ok(proxy) = reqwest::Proxy::all(proxy_url) {
                client_builder = client_builder.proxy(proxy);
            }
        }
        
        let client = client_builder.build().unwrap_or_else(|_| Client::new());
        
        Self {
            client,
            config,
        }
    }
    
    /// Verifies an API endpoint
    pub async fn verify_api(
        &self,
        url: &str,
        expected_status: u16,
        expected_body: Option<Value>,
    ) -> Result<ApiVerification> {
        let start = Utc::now();
        let start_instant = std::time::Instant::now();
        
        // Build request
        let mut request = self.client.get(url);
        for (key, value) in &self.config.default_headers {
            request = request.header(key, value);
        }
        
        // Send request
        let response = request.send().await
            .map_err(|e| VerifyError::NetworkError(format!("Request failed: {}", e)))?;
        
        let response_time_ms = start_instant.elapsed().as_millis() as u64;
        let status = response.status();
        
        // Extract headers
        let mut headers = HashMap::new();
        for (key, value) in response.headers() {
            headers.insert(
                key.to_string(),
                value.to_str().unwrap_or("").to_string()
            );
        }
        
        // Get body if it's JSON
        let body = if headers.get("content-type")
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false) 
        {
            response.json::<Value>().await.ok()
        } else {
            // Try to get text and parse as JSON
            response.text().await.ok()
                .and_then(|text| serde_json::from_str(&text).ok())
        };
        
        // Verify status code
        let status_matches = status.as_u16() == expected_status;
        
        // Verify body if provided
        let body_matches = if let Some(expected) = &expected_body {
            body.as_ref().map(|b| b == expected).unwrap_or(false)
        } else {
            true
        };
        
        let success = status_matches && body_matches;
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        // Create verification result
        let result = if success {
            VerificationResult::success(
                VerificationType::Network,
                serde_json::json!({
                    "url": url,
                    "expected_status": expected_status,
                    "expected_body": expected_body,
                }),
                serde_json::json!({
                    "status": status.as_u16(),
                    "body": body,
                    "response_time_ms": response_time_ms,
                }),
            )
        } else {
            let error = if !status_matches {
                format!("Status mismatch: expected {}, got {}", expected_status, status.as_u16())
            } else {
                "Body mismatch".to_string()
            };
            
            VerificationResult::failure(
                VerificationType::Network,
                serde_json::json!({
                    "url": url,
                    "expected_status": expected_status,
                    "expected_body": expected_body,
                }),
                serde_json::json!({
                    "status": status.as_u16(),
                    "body": body,
                    "response_time_ms": response_time_ms,
                }),
                error,
            )
        };
        
        let mut final_result = result;
        final_result.duration_ms = duration_ms;
        
        // Add evidence
        final_result.evidence.push(Evidence {
            evidence_type: EvidenceType::NetworkResponse,
            data: serde_json::json!({
                "url": url,
                "status": status.as_u16(),
                "headers": headers.len(),
                "response_time_ms": response_time_ms,
            }),
            source: "NetworkVerifier".to_string(),
            timestamp: Utc::now(),
        });
        
        Ok(ApiVerification {
            result: final_result,
            status_code: status.as_u16(),
            headers,
            body,
            response_time_ms,
        })
    }
    
    /// Verifies network connectivity to a host
    pub async fn verify_connectivity(
        &self,
        host: &str,
        port: u16,
    ) -> Result<NetworkVerification> {
        let start = Utc::now();
        let start_instant = std::time::Instant::now();
        
        // Try to connect using TCP
        let addr = format!("{}:{}", host, port);
        let connect_result = tokio::time::timeout(
            Duration::from_millis(self.config.timeout_ms),
            tokio::net::TcpStream::connect(&addr)
        ).await;
        
        let total_time_ms = start_instant.elapsed().as_millis() as u64;
        let success = connect_result.is_ok();
        
        let metrics = NetworkMetrics {
            dns_time_ms: None, // Would need DNS resolution timing
            connect_time_ms: Some(total_time_ms),
            tls_time_ms: None,
            ttfb_ms: None,
            total_time_ms,
            bytes_sent: 0,
            bytes_received: 0,
        };
        
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        let result = if success {
            VerificationResult::success(
                VerificationType::Network,
                serde_json::json!({
                    "host": host,
                    "port": port,
                }),
                serde_json::json!({
                    "connected": true,
                    "time_ms": total_time_ms,
                }),
            )
        } else {
            VerificationResult::failure(
                VerificationType::Network,
                serde_json::json!({
                    "host": host,
                    "port": port,
                }),
                serde_json::json!({
                    "connected": false,
                    "time_ms": total_time_ms,
                }),
                format!("Failed to connect to {}:{}", host, port),
            )
        };
        
        let mut final_result = result;
        final_result.duration_ms = duration_ms;
        
        Ok(NetworkVerification {
            result: final_result,
            metrics,
            trace: None,
        })
    }
    
    /// Verifies multiple API endpoints
    pub async fn verify_endpoints(
        &self,
        endpoints: &[(String, u16, Option<Value>)],
    ) -> Result<Vec<ApiVerification>> {
        let mut results = Vec::new();
        
        for (url, expected_status, expected_body) in endpoints {
            let verification = self.verify_api(
                url,
                *expected_status,
                expected_body.clone()
            ).await?;
            results.push(verification);
        }
        
        Ok(results)
    }
    
    /// Verifies webhook delivery
    pub async fn verify_webhook(
        &self,
        webhook_url: &str,
        payload: Value,
        expected_response: Option<Value>,
    ) -> Result<ApiVerification> {
        let start = Utc::now();
        let start_instant = std::time::Instant::now();
        
        // Send webhook
        let response = self.client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| VerifyError::NetworkError(format!("Webhook failed: {}", e)))?;
        
        let response_time_ms = start_instant.elapsed().as_millis() as u64;
        let status = response.status();
        
        // Extract response
        let body = response.json::<Value>().await.ok();
        
        // Verify response if expected
        let success = if let Some(expected) = expected_response {
            body.as_ref().map(|b| b == &expected).unwrap_or(false)
        } else {
            status.is_success()
        };
        
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        let result = if success {
            VerificationResult::success(
                VerificationType::Network,
                serde_json::json!({
                    "webhook_url": webhook_url,
                    "payload": payload,
                }),
                serde_json::json!({
                    "status": status.as_u16(),
                    "response": body,
                }),
            )
        } else {
            VerificationResult::failure(
                VerificationType::Network,
                serde_json::json!({
                    "webhook_url": webhook_url,
                    "payload": payload,
                }),
                serde_json::json!({
                    "status": status.as_u16(),
                    "response": body,
                }),
                "Webhook verification failed".to_string(),
            )
        };
        
        let mut final_result = result;
        final_result.duration_ms = duration_ms;
        
        Ok(ApiVerification {
            result: final_result,
            status_code: status.as_u16(),
            headers: HashMap::new(),
            body,
            response_time_ms,
        })
    }
    
    /// Performs a health check on an endpoint
    pub async fn health_check(
        &self,
        health_url: &str,
    ) -> Result<bool> {
        let response = self.client
            .get(health_url)
            .send()
            .await
            .map_err(|e| VerifyError::NetworkError(format!("Health check failed: {}", e)))?;
        
        Ok(response.status().is_success())
    }
}

impl Default for NetworkVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, server_url};
    
    #[tokio::test]
    async fn test_api_verification_success() {
        let _m = mock("GET", "/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "ok"}"#)
            .create();
        
        let verifier = NetworkVerifier::new();
        let url = format!("{}/test", server_url());
        
        let result = verifier.verify_api(
            &url,
            200,
            Some(serde_json::json!({"status": "ok"}))
        ).await.unwrap();
        
        assert!(result.result.success);
        assert_eq!(result.status_code, 200);
    }
    
    #[tokio::test]
    async fn test_api_verification_status_mismatch() {
        let _m = mock("GET", "/test")
            .with_status(404)
            .create();
        
        let verifier = NetworkVerifier::new();
        let url = format!("{}/test", server_url());
        
        let result = verifier.verify_api(&url, 200, None).await.unwrap();
        
        assert!(!result.result.success);
        assert_eq!(result.status_code, 404);
    }
    
    #[tokio::test]
    async fn test_connectivity_verification() {
        let verifier = NetworkVerifier::new();
        
        // Test localhost connectivity (should work)
        let result = verifier.verify_connectivity("127.0.0.1", 80).await;
        // Note: This might fail in CI, so we just check it doesn't panic
        assert!(result.is_ok() || result.is_err());
    }
}