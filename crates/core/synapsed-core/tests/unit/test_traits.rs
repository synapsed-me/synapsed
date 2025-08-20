//! Unit tests for traits module

use std::collections::HashMap;
use synapsed_core::{
    error::{SynapsedError, SynapsedResult},
    traits::*,
};
use tokio_test;
use uuid::Uuid;

// Test implementations for traits
#[derive(Debug, Clone)]
struct TestObservable {
    id: Uuid,
    name: String,
    state: ObservableState,
    health: HealthLevel,
}

impl TestObservable {
    fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            state: ObservableState::Running,
            health: HealthLevel::Healthy,
        }
    }

    fn with_state(mut self, state: ObservableState) -> Self {
        self.state = state;
        self
    }

    fn with_health(mut self, health: HealthLevel) -> Self {
        self.health = health;
        self
    }
}

#[async_trait::async_trait]
impl Observable for TestObservable {
    async fn status(&self) -> SynapsedResult<ObservableStatus> {
        Ok(ObservableStatus {
            state: self.state.clone(),
            last_updated: chrono::Utc::now(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("version".to_string(), "1.0.0".to_string());
                meta
            },
        })
    }

    async fn health(&self) -> SynapsedResult<HealthStatus> {
        let mut checks = HashMap::new();
        checks.insert("main".to_string(), HealthCheck {
            level: self.health.clone(),
            message: format!("Component is {:?}", self.health),
            timestamp: chrono::Utc::now(),
        });

        Ok(HealthStatus {
            overall: self.health.clone(),
            checks,
            last_check: chrono::Utc::now(),
        })
    }

    async fn metrics(&self) -> SynapsedResult<HashMap<String, f64>> {
        let mut metrics = HashMap::new();
        metrics.insert("uptime".to_string(), 3600.0);
        metrics.insert("cpu_usage".to_string(), 25.5);
        metrics.insert("memory_usage".to_string(), 512.0);
        Ok(metrics)
    }

    fn describe(&self) -> String {
        format!("TestObservable '{}' ({})", self.name, self.id)
    }
}

impl Identifiable for TestObservable {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn type_name(&self) -> &'static str {
        "TestObservable"
    }
}

#[derive(Debug, Clone)]
struct TestConfig {
    host: String,
    port: u16,
    enabled: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8080,
            enabled: true,
        }
    }
}

#[derive(Debug)]
struct TestConfigurable {
    config: TestConfig,
}

impl TestConfigurable {
    fn new() -> Self {
        Self {
            config: TestConfig::default(),
        }
    }
}

#[async_trait::async_trait]
impl Configurable for TestConfigurable {
    type Config = TestConfig;

    async fn configure(&mut self, config: Self::Config) -> SynapsedResult<()> {
        if config.port == 0 {
            return Err(SynapsedError::invalid_input("Port cannot be zero"));
        }
        self.config = config;
        Ok(())
    }

    async fn get_config(&self) -> SynapsedResult<Self::Config> {
        Ok(self.config.clone())
    }

    async fn validate_config(config: &Self::Config) -> SynapsedResult<()> {
        if config.host.is_empty() {
            return Err(SynapsedError::invalid_input("Host cannot be empty"));
        }
        if config.port == 0 {
            return Err(SynapsedError::invalid_input("Port cannot be zero"));
        }
        Ok(())
    }

    fn default_config() -> Self::Config {
        TestConfig::default()
    }
}

#[derive(Debug)]
struct TestValidatable {
    value: i32,
    should_pass: bool,
}

impl TestValidatable {
    fn new(value: i32, should_pass: bool) -> Self {
        Self { value, should_pass }
    }
}

impl Validatable for TestValidatable {
    fn validate(&self) -> SynapsedResult<()> {
        if !self.should_pass {
            return Err(SynapsedError::invalid_input("Validation configured to fail"));
        }
        if self.value < 0 {
            return Err(SynapsedError::invalid_input("Value must be non-negative"));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct TestRetryable {
    attempts: std::cell::RefCell<usize>,
    max_attempts: usize,
    should_succeed_after: usize,
    result_value: String,
}

impl TestRetryable {
    fn new(max_attempts: usize, should_succeed_after: usize, result_value: String) -> Self {
        Self {
            attempts: std::cell::RefCell::new(0),
            max_attempts,
            should_succeed_after,
            result_value,
        }
    }

    fn attempt_count(&self) -> usize {
        *self.attempts.borrow()
    }
}

#[async_trait::async_trait]
impl Retryable for TestRetryable {
    type Output = String;

    async fn execute(&mut self) -> SynapsedResult<Self::Output> {
        let mut attempts = self.attempts.borrow_mut();
        *attempts += 1;

        if *attempts <= self.should_succeed_after {
            if *attempts == self.should_succeed_after {
                Ok(self.result_value.clone())
            } else {
                Err(SynapsedError::network("Temporary failure"))
            }
        } else {
            Err(SynapsedError::invalid_input("Non-retryable error"))
        }
    }

    fn max_retries(&self) -> usize {
        self.max_attempts
    }

    fn base_delay_ms(&self) -> u64 {
        10 // Fast for testing
    }
}

#[derive(Debug)]
struct TestLifecycle {
    state: LifecycleState,
    should_fail_start: bool,
    should_fail_stop: bool,
}

impl TestLifecycle {
    fn new() -> Self {
        Self {
            state: LifecycleState::Created,
            should_fail_start: false,
            should_fail_stop: false,
        }
    }

    fn with_start_failure(mut self) -> Self {
        self.should_fail_start = true;
        self
    }

    fn with_stop_failure(mut self) -> Self {
        self.should_fail_stop = true;
        self
    }
}

#[async_trait::async_trait]
impl Lifecycle for TestLifecycle {
    async fn start(&mut self) -> SynapsedResult<()> {
        if self.should_fail_start {
            self.state = LifecycleState::Failed;
            return Err(SynapsedError::internal("Start failed"));
        }
        
        self.state = LifecycleState::Starting;
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        self.state = LifecycleState::Running;
        Ok(())
    }

    async fn stop(&mut self) -> SynapsedResult<()> {
        if self.should_fail_stop {
            self.state = LifecycleState::Failed;
            return Err(SynapsedError::internal("Stop failed"));
        }
        
        self.state = LifecycleState::Stopping;
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        self.state = LifecycleState::Stopped;
        Ok(())
    }

    fn is_running(&self) -> bool {
        matches!(self.state, LifecycleState::Running)
    }

    fn lifecycle_state(&self) -> LifecycleState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_observable_trait() {
        let observable = TestObservable::new("test-component");

        // Test status
        let status = observable.status().await.unwrap();
        assert_eq!(status.state, ObservableState::Running);
        assert!(status.metadata.contains_key("version"));

        // Test health
        let health = observable.health().await.unwrap();
        assert_eq!(health.overall, HealthLevel::Healthy);
        assert!(health.checks.contains_key("main"));

        // Test metrics
        let metrics = observable.metrics().await.unwrap();
        assert!(metrics.contains_key("uptime"));
        assert!(metrics.contains_key("cpu_usage"));
        assert_eq!(metrics["uptime"], 3600.0);

        // Test describe
        let description = observable.describe();
        assert!(description.contains("TestObservable"));
        assert!(description.contains("test-component"));
    }

    #[tokio::test]
    async fn test_observable_states() {
        let degraded = TestObservable::new("degraded").with_state(ObservableState::Degraded);
        let status = degraded.status().await.unwrap();
        assert_eq!(status.state, ObservableState::Degraded);

        let failed = TestObservable::new("failed").with_state(ObservableState::Failed);
        let status = failed.status().await.unwrap();
        assert_eq!(status.state, ObservableState::Failed);
    }

    #[test]
    fn test_identifiable_trait() {
        let identifiable = TestObservable::new("test");
        
        let id = identifiable.id();
        assert!(!id.is_nil());
        
        assert_eq!(identifiable.name(), "test");
        assert_eq!(identifiable.type_name(), "TestObservable");
    }

    #[tokio::test]
    async fn test_configurable_trait() {
        let mut configurable = TestConfigurable::new();

        // Test default config
        let default_config = TestConfigurable::default_config();
        assert_eq!(default_config.host, "localhost");
        assert_eq!(default_config.port, 8080);
        assert!(default_config.enabled);

        // Test get config
        let current_config = configurable.get_config().await.unwrap();
        assert_eq!(current_config.host, default_config.host);

        // Test valid configuration
        let new_config = TestConfig {
            host: "example.com".to_string(),
            port: 9090,
            enabled: false,
        };
        
        TestConfigurable::validate_config(&new_config).await.unwrap();
        configurable.configure(new_config.clone()).await.unwrap();
        
        let updated_config = configurable.get_config().await.unwrap();
        assert_eq!(updated_config.host, "example.com");
        assert_eq!(updated_config.port, 9090);
        assert!(!updated_config.enabled);

        // Test invalid configuration - empty host
        let invalid_config = TestConfig {
            host: "".to_string(),
            port: 8080,
            enabled: true,
        };
        
        let result = TestConfigurable::validate_config(&invalid_config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Host cannot be empty"));

        // Test invalid configuration - zero port
        let invalid_config = TestConfig {
            host: "localhost".to_string(),
            port: 0,
            enabled: true,
        };
        
        let result = TestConfigurable::validate_config(&invalid_config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Port cannot be zero"));

        let result = configurable.configure(invalid_config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Port cannot be zero"));
    }

    #[test]
    fn test_validatable_trait() {
        // Test valid object
        let valid = TestValidatable::new(42, true);
        assert!(valid.validate().is_ok());
        assert!(valid.is_valid());

        // Test invalid value
        let invalid_value = TestValidatable::new(-1, true);
        assert!(invalid_value.validate().is_err());
        assert!(!invalid_value.is_valid());
        let error = invalid_value.validate().unwrap_err();
        assert!(error.to_string().contains("non-negative"));

        // Test configured failure
        let configured_failure = TestValidatable::new(42, false);
        assert!(configured_failure.validate().is_err());
        assert!(!configured_failure.is_valid());
        let error = configured_failure.validate().unwrap_err();
        assert!(error.to_string().contains("configured to fail"));
    }

    #[tokio::test]
    async fn test_retryable_trait() {
        // Test successful execution without retries
        let mut immediate_success = TestRetryable::new(3, 1, "success".to_string());
        let result = immediate_success.execute().await.unwrap();
        assert_eq!(result, "success");
        assert_eq!(immediate_success.attempt_count(), 1);

        // Test successful execution with retries
        let mut retry_success = TestRetryable::new(3, 2, "success".to_string());
        let result = retry_success.execute_with_retry().await.unwrap();
        assert_eq!(result, "success");
        assert_eq!(retry_success.attempt_count(), 2);

        // Test failure after max retries
        let mut retry_failure = TestRetryable::new(2, 5, "never".to_string());
        let result = retry_failure.execute_with_retry().await;
        assert!(result.is_err());
        assert_eq!(retry_failure.attempt_count(), 3); // 1 initial + 2 retries

        // Test non-retryable error
        let mut non_retryable = TestRetryable::new(3, 0, "never".to_string());
        let result = non_retryable.execute_with_retry().await;
        assert!(result.is_err());
        assert_eq!(non_retryable.attempt_count(), 1); // Should not retry
    }

    #[tokio::test]
    async fn test_lifecycle_trait() {
        // Test successful lifecycle
        let mut lifecycle = TestLifecycle::new();
        assert_eq!(lifecycle.lifecycle_state(), LifecycleState::Created);
        assert!(!lifecycle.is_running());

        // Start
        lifecycle.start().await.unwrap();
        assert_eq!(lifecycle.lifecycle_state(), LifecycleState::Running);
        assert!(lifecycle.is_running());

        // Stop
        lifecycle.stop().await.unwrap();
        assert_eq!(lifecycle.lifecycle_state(), LifecycleState::Stopped);
        assert!(!lifecycle.is_running());

        // Restart
        lifecycle.restart().await.unwrap();
        assert_eq!(lifecycle.lifecycle_state(), LifecycleState::Running);
        assert!(lifecycle.is_running());
    }

    #[tokio::test]
    async fn test_lifecycle_start_failure() {
        let mut lifecycle = TestLifecycle::new().with_start_failure();
        
        let result = lifecycle.start().await;
        assert!(result.is_err());
        assert_eq!(lifecycle.lifecycle_state(), LifecycleState::Failed);
        assert!(!lifecycle.is_running());
    }

    #[tokio::test]
    async fn test_lifecycle_stop_failure() {
        let mut lifecycle = TestLifecycle::new();
        lifecycle.start().await.unwrap();
        
        lifecycle.should_fail_stop = true;
        let result = lifecycle.stop().await;
        assert!(result.is_err());
        assert_eq!(lifecycle.lifecycle_state(), LifecycleState::Failed);
    }

    #[tokio::test]
    async fn test_lifecycle_restart_failure() {
        let mut lifecycle = TestLifecycle::new();
        lifecycle.start().await.unwrap();
        
        // Set to fail on stop
        lifecycle.should_fail_stop = true;
        let result = lifecycle.restart().await;
        assert!(result.is_err());
        assert_eq!(lifecycle.lifecycle_state(), LifecycleState::Failed);
    }

    #[test]
    fn test_health_status_types() {
        let healthy_check = HealthCheck {
            level: HealthLevel::Healthy,
            message: "All good".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(healthy_check.level, HealthLevel::Healthy);

        let warning_check = HealthCheck {
            level: HealthLevel::Warning,
            message: "Minor issue".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(warning_check.level, HealthLevel::Warning);

        let critical_check = HealthCheck {
            level: HealthLevel::Critical,
            message: "Major issue".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(critical_check.level, HealthLevel::Critical);

        let unknown_check = HealthCheck {
            level: HealthLevel::Unknown,
            message: "Cannot determine".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(unknown_check.level, HealthLevel::Unknown);
    }

    #[test]
    fn test_observable_state_types() {
        let states = vec![
            ObservableState::Initializing,
            ObservableState::Running,
            ObservableState::Degraded,
            ObservableState::Failed,
            ObservableState::ShuttingDown,
            ObservableState::Stopped,
        ];

        for state in states {
            // Each state should be cloneable and debuggable
            let _cloned = state.clone();
            let _debug = format!("{:?}", state);
            
            // States should be comparable
            assert_eq!(state, state.clone());
        }
    }

    #[test]
    fn test_lifecycle_state_types() {
        let states = vec![
            LifecycleState::Created,
            LifecycleState::Starting,
            LifecycleState::Running,
            LifecycleState::Stopping,
            LifecycleState::Stopped,
            LifecycleState::Failed,
        ];

        for state in states {
            // Each state should be serializable
            let serialized = serde_json::to_string(&state).unwrap();
            let deserialized: LifecycleState = serde_json::from_str(&serialized).unwrap();
            assert_eq!(state, deserialized);
        }
    }

    #[test]
    fn test_trait_bounds() {
        // Ensure our test types implement Send + Sync where required
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send::<TestObservable>();
        assert_sync::<TestObservable>();
        assert_send_sync::<TestObservable>();

        assert_send::<TestConfig>();
        assert_sync::<TestConfig>();
        assert_send_sync::<TestConfig>();

        assert_send::<TestValidatable>();
        // Note: TestValidatable is not Sync due to RefCell in TestRetryable,
        // but that's intentional for testing
    }

    #[tokio::test]
    async fn test_complex_health_status() {
        let complex = TestObservable::new("complex")
            .with_health(HealthLevel::Warning);
            
        let health = complex.health().await.unwrap();
        assert_eq!(health.overall, HealthLevel::Warning);
        
        // Should have at least one check
        assert!(!health.checks.is_empty());
        
        // Check should match overall health
        let main_check = health.checks.get("main").unwrap();
        assert_eq!(main_check.level, HealthLevel::Warning);
    }

    #[test]
    fn test_error_propagation_in_traits() {
        // Test that errors are properly propagated through trait methods
        let invalid = TestValidatable::new(-5, true);
        match invalid.validate() {
            Err(SynapsedError::InvalidInput(msg)) => {
                assert!(msg.contains("non-negative"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[tokio::test]
    async fn test_concurrent_trait_operations() {
        // Test that trait implementations work correctly under concurrent access
        let observable = std::sync::Arc::new(TestObservable::new("concurrent"));
        
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let obs = observable.clone();
                tokio::spawn(async move {
                    let status = obs.status().await.unwrap();
                    let health = obs.health().await.unwrap();
                    let metrics = obs.metrics().await.unwrap();
                    (status, health, metrics)
                })
            })
            .collect();

        // All operations should complete successfully
        for handle in handles {
            let (status, health, metrics) = handle.await.unwrap();
            assert_eq!(status.state, ObservableState::Running);
            assert_eq!(health.overall, HealthLevel::Healthy);
            assert!(!metrics.is_empty());
        }
    }
}