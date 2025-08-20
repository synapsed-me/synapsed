//! Helper functions and utilities for testing synapsed-core

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use synapsed_core::{
    config::{ConfigValue, FileConfigSource},
    error::{SynapsedError, SynapsedResult},
    network::{NetworkAddress, NetworkMessage},
};
use tokio::time::timeout;
use uuid::Uuid;

/// Test environment manager for setup and cleanup
pub struct TestEnvironment {
    temp_dir: PathBuf,
    env_vars: HashMap<String, Option<String>>,
}

impl TestEnvironment {
    /// Create a new test environment
    pub fn new() -> Self {
        let temp_dir = std::env::temp_dir().join(format!("synapsed-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");

        Self {
            temp_dir,
            env_vars: HashMap::new(),
        }
    }

    /// Get the temporary directory path
    pub fn temp_dir(&self) -> &Path {
        &self.temp_dir
    }

    /// Set an environment variable for the test
    pub fn set_env_var<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        let key = key.into();
        let old_value = env::var(&key).ok();
        env::set_var(&key, value.into());
        self.env_vars.insert(key, old_value);
    }

    /// Create a temporary file with content
    pub fn create_temp_file<P: AsRef<Path>>(&self, relative_path: P, content: &str) -> PathBuf {
        let file_path = self.temp_dir.join(relative_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directory");
        }
        fs::write(&file_path, content).expect("Failed to write temp file");
        file_path
    }

    /// Create a test configuration file
    pub fn create_config_file(&self, name: &str, format: &str, content: &str) -> PathBuf {
        let filename = format!("{}.{}", name, format);
        self.create_temp_file(filename, content)
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        // Restore environment variables
        for (key, old_value) in &self.env_vars {
            match old_value {
                Some(value) => env::set_var(key, value),
                None => env::remove_var(key),
            }
        }

        // Clean up temporary directory
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Assertion helpers for complex types

/// Assert that a result contains a specific error type
pub fn assert_error_type<T>(result: SynapsedResult<T>, expected: &str) {
    assert!(result.is_err(), "Expected error but got success");
    let error = result.unwrap_err();
    let error_str = error.to_string();
    assert!(
        error_str.contains(expected),
        "Expected error containing '{}', got '{}'",
        expected,
        error_str
    );
}

/// Assert that an error is retryable
pub fn assert_retryable_error(error: &SynapsedError) {
    assert!(error.is_retryable(), "Expected retryable error: {}", error);
}

/// Assert that an error is not retryable
pub fn assert_non_retryable_error(error: &SynapsedError) {
    assert!(!error.is_retryable(), "Expected non-retryable error: {}", error);
}

/// Assert that an error is a client error
pub fn assert_client_error(error: &SynapsedError) {
    assert!(error.is_client_error(), "Expected client error: {}", error);
}

/// Assert that an error is a server error
pub fn assert_server_error(error: &SynapsedError) {
    assert!(error.is_server_error(), "Expected server error: {}", error);
}

/// Assert that a network address has the expected protocol
pub fn assert_network_address_protocol(address: &NetworkAddress, expected_protocol: &str) {
    assert_eq!(
        address.protocol(),
        expected_protocol,
        "Expected protocol '{}', got '{}'",
        expected_protocol,
        address.protocol()
    );
}

/// Assert that a config value matches expected type and value
pub fn assert_config_value_string(value: &ConfigValue, expected: &str) {
    match value {
        ConfigValue::String(s) => assert_eq!(s, expected),
        _ => panic!("Expected string config value '{}', got {:?}", expected, value),
    }
}

/// Assert that a config value is an integer with expected value
pub fn assert_config_value_integer(value: &ConfigValue, expected: i64) {
    match value {
        ConfigValue::Integer(i) => assert_eq!(*i, expected),
        _ => panic!("Expected integer config value {}, got {:?}", expected, value),
    }
}

/// Assert that a config value is a boolean with expected value
pub fn assert_config_value_boolean(value: &ConfigValue, expected: bool) {
    match value {
        ConfigValue::Boolean(b) => assert_eq!(*b, expected),
        _ => panic!("Expected boolean config value {}, got {:?}", expected, value),
    }
}

/// Timeout helpers for async tests

/// Run an async test with a timeout
pub async fn with_timeout<F, T>(duration: Duration, future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    timeout(duration, future)
        .await
        .expect("Test timed out")
}

/// Run an async test with a default timeout of 5 seconds
pub async fn with_default_timeout<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    with_timeout(Duration::from_secs(5), future).await
}

/// Data generation helpers

/// Generate test data of specified size
pub fn generate_test_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

/// Generate random test string
pub fn generate_test_string(length: usize) -> String {
    use std::iter;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                             abcdefghijklmnopqrstuvwxyz\
                             0123456789";

    let mut rng = fastrand::Rng::new();
    (0..length)
        .map(|_| {
            let idx = rng.usize(..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Generate test network addresses
pub fn generate_test_network_addresses() -> Vec<NetworkAddress> {
    vec![
        NetworkAddress::Socket("127.0.0.1:8080".parse().unwrap()),
        NetworkAddress::Socket("[::1]:8080".parse().unwrap()),
        NetworkAddress::PeerId("12D3KooWTest123".to_string()),
        NetworkAddress::Did("did:example:123".to_string()),
        NetworkAddress::Multiaddr("/ip4/127.0.0.1/tcp/8080".to_string()),
        NetworkAddress::WebRtc("webrtc://example.com:8080".to_string()),
        NetworkAddress::Custom {
            protocol: "custom".to_string(),
            address: "custom-address".to_string(),
        },
    ]
}

/// Test configuration builders

/// Build a test TOML configuration
pub fn build_test_toml_config() -> String {
    r#"
[database]
host = "localhost"
port = 5432
username = "test_user"
password = "test_pass"
ssl = true

[network]
listen_address = "0.0.0.0:8080"
max_connections = 100
timeout_seconds = 30

[logging]
level = "info"
format = "json"
enable_tracing = true

[features]
enable_metrics = true
enable_health_checks = true
debug_mode = false
"#
    .to_string()
}

/// Build a test JSON configuration
pub fn build_test_json_config() -> String {
    r#"{
  "database": {
    "host": "localhost",
    "port": 5432,
    "username": "test_user",
    "password": "test_pass",
    "ssl": true
  },
  "network": {
    "listen_address": "0.0.0.0:8080",
    "max_connections": 100,
    "timeout_seconds": 30
  },
  "logging": {
    "level": "info",
    "format": "json",
    "enable_tracing": true
  },
  "features": {
    "enable_metrics": true,
    "enable_health_checks": true,
    "debug_mode": false
  }
}"#
    .to_string()
}

/// Logging helpers for tests

/// Initialize test logging
pub fn init_test_logging() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();
}

/// Create a test logger that captures output
pub struct TestLogger {
    entries: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl TestLogger {
    pub fn new() -> Self {
        Self {
            entries: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn entries(&self) -> Vec<String> {
        self.entries.lock().unwrap().clone()
    }

    pub fn contains(&self, message: &str) -> bool {
        self.entries().iter().any(|entry| entry.contains(message))
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }
}

impl Default for TestLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Async test utilities

/// Create a test tokio runtime
pub fn create_test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Failed to create test runtime")
}

/// Block on async test with custom runtime
pub fn block_on_test<F>(future: F) -> F::Output
where
    F: std::future::Future,
{
    let rt = create_test_runtime();
    rt.block_on(future)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_setup() {
        let mut env = TestEnvironment::new();
        
        // Test temp directory creation
        assert!(env.temp_dir().exists());
        
        // Test environment variable setting
        env.set_env_var("TEST_VAR", "test_value");
        assert_eq!(env::var("TEST_VAR").unwrap(), "test_value");
        
        // Test file creation
        let file_path = env.create_temp_file("test.txt", "test content");
        assert!(file_path.exists());
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "test content");
        
        // Test config file creation
        let config_path = env.create_config_file("config", "toml", "[test]\nvalue = 42");
        assert!(config_path.exists());
        assert!(config_path.to_string_lossy().ends_with(".toml"));
    }

    #[test]
    fn test_error_assertions() {
        let error_result: SynapsedResult<()> = Err(SynapsedError::invalid_input("test error"));
        assert_error_type(error_result, "Invalid input");

        let network_error = SynapsedError::network("connection failed");
        assert_retryable_error(&network_error);

        let validation_error = SynapsedError::invalid_input("bad input");
        assert_non_retryable_error(&validation_error);
        assert_client_error(&validation_error);

        let internal_error = SynapsedError::internal("server error");
        assert_server_error(&internal_error);
    }

    #[test]
    fn test_network_address_assertions() {
        let socket_addr = NetworkAddress::Socket("127.0.0.1:8080".parse().unwrap());
        assert_network_address_protocol(&socket_addr, "tcp");

        let peer_addr = NetworkAddress::PeerId("peer123".to_string());
        assert_network_address_protocol(&peer_addr, "p2p");
    }

    #[test]
    fn test_config_value_assertions() {
        let string_val = ConfigValue::String("test".to_string());
        assert_config_value_string(&string_val, "test");

        let int_val = ConfigValue::Integer(42);
        assert_config_value_integer(&int_val, 42);

        let bool_val = ConfigValue::Boolean(true);
        assert_config_value_boolean(&bool_val, true);
    }

    #[test]
    fn test_data_generation() {
        let data = generate_test_data(100);
        assert_eq!(data.len(), 100);
        assert_eq!(data[0], 0);
        assert_eq!(data[99], 99);

        let string = generate_test_string(50);
        assert_eq!(string.len(), 50);
        assert!(string.chars().all(|c| c.is_ascii_alphanumeric()));

        let addresses = generate_test_network_addresses();
        assert_eq!(addresses.len(), 7);
        assert!(addresses.iter().any(|addr| matches!(addr, NetworkAddress::Socket(_))));
        assert!(addresses.iter().any(|addr| matches!(addr, NetworkAddress::PeerId(_))));
    }

    #[test]
    fn test_config_builders() {
        let toml_config = build_test_toml_config();
        assert!(toml_config.contains("[database]"));
        assert!(toml_config.contains("host = \"localhost\""));

        let json_config = build_test_json_config();
        assert!(json_config.contains("\"database\""));
        assert!(json_config.contains("\"host\": \"localhost\""));
    }

    #[test]
    fn test_logger() {
        let logger = TestLogger::new();
        assert!(logger.entries().is_empty());
        assert!(!logger.contains("test"));
        
        logger.clear();
        assert!(logger.entries().is_empty());
    }

    #[tokio::test]
    async fn test_timeout_helpers() {
        // Test successful completion within timeout
        let result = with_timeout(Duration::from_millis(100), async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            42
        }).await;
        assert_eq!(result, 42);

        // Test default timeout
        let result = with_default_timeout(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            "success"
        }).await;
        assert_eq!(result, "success");
    }

    #[tokio::test]
    #[should_panic(expected = "Test timed out")]
    async fn test_timeout_failure() {
        with_timeout(Duration::from_millis(10), async {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }).await;
    }
}