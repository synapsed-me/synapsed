//! Mock implementations for testing synapsed-core traits

use async_trait::async_trait;
use mockall::mock;
use std::collections::HashMap;
use synapsed_core::{
    error::{SynapsedError, SynapsedResult},
    network::{
        ConnectionMetadata, ConnectionState, NetworkAddress, NetworkConnection, NetworkEvent,
        NetworkEventHandler, NetworkListener, NetworkMessage, NetworkStats,
    },
    traits::{
        Cacheable, Configurable, HealthCheck, HealthLevel, HealthStatus, Identifiable, Lifecycle,
        LifecycleState, Observable, ObservableState, ObservableStatus, Retryable, Validatable,
        VersionedSerializable,
    },
};
use uuid::Uuid;

// Mock Observable implementation
mock! {
    pub TestObservable {}
    
    #[async_trait]
    impl Observable for TestObservable {
        async fn status(&self) -> SynapsedResult<ObservableStatus>;
        async fn health(&self) -> SynapsedResult<HealthStatus>;
        async fn metrics(&self) -> SynapsedResult<HashMap<String, f64>>;
        fn describe(&self) -> String;
    }
    
    impl Identifiable for TestObservable {
        fn id(&self) -> Uuid;
        fn name(&self) -> &str;
        fn type_name(&self) -> &'static str;
    }
}

// Mock Configurable implementation
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub name: String,
    pub value: i32,
    pub enabled: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            name: "test".to_string(),
            value: 42,
            enabled: true,
        }
    }
}

mock! {
    pub TestConfigurable {}
    
    #[async_trait]
    impl Configurable for TestConfigurable {
        type Config = TestConfig;
        
        async fn configure(&mut self, config: Self::Config) -> SynapsedResult<()>;
        async fn get_config(&self) -> SynapsedResult<Self::Config>;
        async fn validate_config(config: &Self::Config) -> SynapsedResult<()>;
        fn default_config() -> Self::Config;
    }
}

// Mock NetworkConnection implementation
mock! {
    pub TestConnection {}
    
    #[async_trait]
    impl NetworkConnection for TestConnection {
        fn metadata(&self) -> &ConnectionMetadata;
        async fn send(&mut self, data: &[u8]) -> SynapsedResult<usize>;
        async fn receive(&mut self, buffer: &mut [u8]) -> SynapsedResult<usize>;
        async fn close(&mut self) -> SynapsedResult<()>;
        fn is_active(&self) -> bool;
        fn local_address(&self) -> &NetworkAddress;
        fn remote_address(&self) -> &NetworkAddress;
    }
}

// Mock NetworkListener implementation
mock! {
    pub TestListener {}
    
    #[async_trait]
    impl NetworkListener for TestListener {
        type Connection = MockTestConnection;
        
        async fn start(&mut self) -> SynapsedResult<()>;
        async fn stop(&mut self) -> SynapsedResult<()>;
        async fn accept(&mut self) -> SynapsedResult<Self::Connection>;
        fn local_address(&self) -> &NetworkAddress;
        fn is_listening(&self) -> bool;
    }
}

// Mock NetworkEventHandler implementation
mock! {
    pub TestEventHandler {}
    
    #[async_trait]
    impl NetworkEventHandler for TestEventHandler {
        async fn handle_event(&mut self, event: NetworkEvent) -> SynapsedResult<()>;
    }
}

// Mock Lifecycle implementation
mock! {
    pub TestLifecycle {}
    
    #[async_trait]
    impl Lifecycle for TestLifecycle {
        async fn start(&mut self) -> SynapsedResult<()>;
        async fn stop(&mut self) -> SynapsedResult<()>;
        async fn restart(&mut self) -> SynapsedResult<()>;
        fn is_running(&self) -> bool;
        fn lifecycle_state(&self) -> LifecycleState;
    }
}

// Mock Retryable implementation
mock! {
    pub TestRetryable {}
    
    #[async_trait]
    impl Retryable for TestRetryable {
        type Output = String;
        
        async fn execute(&mut self) -> SynapsedResult<Self::Output>;
        fn is_retryable_error(&self, error: &SynapsedError) -> bool;
        fn max_retries(&self) -> usize;
        fn base_delay_ms(&self) -> u64;
        async fn execute_with_retry(&mut self) -> SynapsedResult<Self::Output>;
    }
}

// Concrete implementations for simple traits

#[derive(Debug, Clone)]
pub struct TestIdentifiable {
    pub id: Uuid,
    pub name: String,
}

impl TestIdentifiable {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
        }
    }
}

impl Identifiable for TestIdentifiable {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn type_name(&self) -> &'static str {
        "TestIdentifiable"
    }
}

#[derive(Debug, Clone)]
pub struct TestValidatable {
    pub value: i32,
    pub should_fail: bool,
}

impl TestValidatable {
    pub fn new(value: i32) -> Self {
        Self {
            value,
            should_fail: false,
        }
    }

    pub fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }
}

impl Validatable for TestValidatable {
    fn validate(&self) -> SynapsedResult<()> {
        if self.should_fail {
            Err(SynapsedError::invalid_input("Validation failed"))
        } else if self.value < 0 {
            Err(SynapsedError::invalid_input("Value must be non-negative"))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestCacheable {
    pub key: String,
    pub value: String,
    pub ttl: Option<u64>,
    pub cacheable: bool,
}

impl TestCacheable {
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
            ttl: None,
            cacheable: true,
        }
    }

    pub fn with_ttl(mut self, ttl: u64) -> Self {
        self.ttl = Some(ttl);
        self
    }

    pub fn non_cacheable(mut self) -> Self {
        self.cacheable = false;
        self
    }
}

#[async_trait]
impl Cacheable for TestCacheable {
    type Key = String;

    fn cache_key(&self) -> Self::Key {
        self.key.clone()
    }

    fn cache_ttl(&self) -> Option<u64> {
        self.ttl
    }

    fn is_cacheable(&self) -> bool {
        self.cacheable
    }
}

// Helper functions for creating test objects

/// Create a test connection metadata
pub fn create_test_connection_metadata() -> ConnectionMetadata {
    ConnectionMetadata {
        id: Uuid::new_v4(),
        local_address: NetworkAddress::Socket("127.0.0.1:8080".parse().unwrap()),
        remote_address: NetworkAddress::Socket("127.0.0.1:8081".parse().unwrap()),
        state: ConnectionState::Connected,
        connected_at: Some(chrono::Utc::now()),
        last_activity: chrono::Utc::now(),
        protocol_version: "1.0".to_string(),
        metadata: HashMap::new(),
    }
}

/// Create a test observable status
pub fn create_test_observable_status(state: ObservableState) -> ObservableStatus {
    ObservableStatus {
        state,
        last_updated: chrono::Utc::now(),
        metadata: HashMap::new(),
    }
}

/// Create a test health status
pub fn create_test_health_status(level: HealthLevel) -> HealthStatus {
    let mut checks = HashMap::new();
    checks.insert(
        "test_check".to_string(),
        HealthCheck {
            level: level.clone(),
            message: "Test health check".to_string(),
            timestamp: chrono::Utc::now(),
        },
    );

    HealthStatus {
        overall: level,
        checks,
        last_check: chrono::Utc::now(),
    }
}

/// Create a test network message
pub fn create_test_network_message(message_type: &str, payload: &[u8]) -> NetworkMessage {
    NetworkMessage::new(message_type, payload.to_vec())
        .with_header("test-header", "test-value")
        .with_sender(NetworkAddress::PeerId("sender".to_string()))
        .with_recipient(NetworkAddress::PeerId("recipient".to_string()))
}

/// Create test network statistics
pub fn create_test_network_stats() -> NetworkStats {
    let mut stats = NetworkStats::new();
    stats.record_bytes_sent(1000);
    stats.record_bytes_received(2000);
    stats.record_message_sent();
    stats.record_message_received();
    stats.record_connection();
    stats.update_uptime(60);
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identifiable_mock() {
        let identifiable = TestIdentifiable::new("test");
        assert!(!identifiable.id().is_nil());
        assert_eq!(identifiable.name(), "test");
        assert_eq!(identifiable.type_name(), "TestIdentifiable");
    }

    #[test]
    fn test_validatable_mock() {
        let valid = TestValidatable::new(42);
        assert!(valid.validate().is_ok());
        assert!(valid.is_valid());

        let invalid = TestValidatable::new(-1);
        assert!(invalid.validate().is_err());
        assert!(!invalid.is_valid());

        let failure = TestValidatable::new(42).with_failure();
        assert!(failure.validate().is_err());
    }

    #[test]
    fn test_cacheable_mock() {
        let cacheable = TestCacheable::new("test-key", "test-value").with_ttl(300);
        assert_eq!(cacheable.cache_key(), "test-key");
        assert_eq!(cacheable.cache_ttl(), Some(300));
        assert!(cacheable.is_cacheable());

        let non_cacheable = TestCacheable::new("key", "value").non_cacheable();
        assert!(!non_cacheable.is_cacheable());
    }

    #[test]
    fn test_helper_functions() {
        let metadata = create_test_connection_metadata();
        assert_eq!(metadata.state, ConnectionState::Connected);
        assert!(metadata.connected_at.is_some());

        let status = create_test_observable_status(ObservableState::Running);
        assert_eq!(status.state, ObservableState::Running);

        let health = create_test_health_status(HealthLevel::Healthy);
        assert_eq!(health.overall, HealthLevel::Healthy);
        assert!(health.checks.contains_key("test_check"));

        let message = create_test_network_message("test.message", b"test payload");
        assert_eq!(message.message_type, "test.message");
        assert_eq!(message.payload, b"test payload");
        assert_eq!(message.get_header("test-header"), Some("test-value"));

        let stats = create_test_network_stats();
        assert_eq!(stats.bytes_sent, 1000);
        assert_eq!(stats.bytes_received, 2000);
        assert_eq!(stats.connection_count, 1);
    }
}