//! Core traits for the Synapsed ecosystem.
//!
//! This module defines fundamental traits that provide common interfaces
//! across all Synapsed components.

use crate::{SynapsedError, SynapsedResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Trait for objects that can be observed and monitored
#[async_trait]
pub trait Observable {
    /// Get the current status of this object
    async fn status(&self) -> SynapsedResult<ObservableStatus>;

    /// Get health information about this object
    async fn health(&self) -> SynapsedResult<HealthStatus>;

    /// Get metrics information
    async fn metrics(&self) -> SynapsedResult<HashMap<String, f64>>;

    /// Get a human-readable description of the current state
    fn describe(&self) -> String;
}

/// Status information for observable objects
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObservableStatus {
    /// Current state
    pub state: ObservableState,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Possible states for observable objects
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ObservableState {
    /// Object is initializing
    Initializing,
    /// Object is running normally
    Running,
    /// Object is degraded but functional
    Degraded,
    /// Object has failed
    Failed,
    /// Object is shutting down
    ShuttingDown,
    /// Object is stopped
    Stopped,
}

/// Health status information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthStatus {
    /// Overall health
    pub overall: HealthLevel,
    /// Health checks by component
    pub checks: HashMap<String, HealthCheck>,
    /// Last health check timestamp
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Health levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthLevel {
    /// Healthy
    Healthy,
    /// Warning condition
    Warning,
    /// Critical condition
    Critical,
    /// Unknown health status
    Unknown,
}

/// Individual health check result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthCheck {
    /// Health level
    pub level: HealthLevel,
    /// Description of the check
    pub message: String,
    /// When the check was performed
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Trait for configurable objects
#[async_trait]
pub trait Configurable {
    /// Configuration type
    type Config: Send + Sync + Clone;

    /// Apply configuration
    async fn configure(&mut self, config: Self::Config) -> SynapsedResult<()>;

    /// Get current configuration
    async fn get_config(&self) -> SynapsedResult<Self::Config>;

    /// Validate configuration
    async fn validate_config(config: &Self::Config) -> SynapsedResult<()>;

    /// Get default configuration
    fn default_config() -> Self::Config;
}

/// Trait for objects with unique identities
pub trait Identifiable {
    /// Get the unique identifier for this object
    fn id(&self) -> Uuid;

    /// Get a human-readable name
    fn name(&self) -> &str;

    /// Get the type identifier
    fn type_name(&self) -> &'static str;
}

/// Trait for validatable objects
pub trait Validatable {
    /// Validate this object
    fn validate(&self) -> SynapsedResult<()>;

    /// Check if this object is valid
    fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

/// Trait for serializable objects with versioning
pub trait VersionedSerializable: Serialize + for<'de> Deserialize<'de> {
    /// Get the schema version
    fn schema_version() -> u32;

    /// Migrate from an older version
    fn migrate_from_version(data: &[u8], from_version: u32) -> SynapsedResult<Self>
    where
        Self: Sized;
}

/// Trait for cacheable objects
#[async_trait]
pub trait Cacheable {
    /// Cache key type
    type Key: Send + Sync + Clone + std::hash::Hash + Eq;

    /// Get the cache key for this object
    fn cache_key(&self) -> Self::Key;

    /// Get cache TTL in seconds
    fn cache_ttl(&self) -> Option<u64> {
        None
    }

    /// Check if this object can be cached
    fn is_cacheable(&self) -> bool {
        true
    }
}

/// Trait for retryable operations
#[async_trait]
pub trait Retryable {
    /// The type of operation result
    type Output: Send;

    /// Execute the operation
    async fn execute(&mut self) -> SynapsedResult<Self::Output>;

    /// Check if the error is retryable
    fn is_retryable_error(&self, error: &SynapsedError) -> bool {
        error.is_retryable()
    }

    /// Get maximum retry attempts
    fn max_retries(&self) -> usize {
        3
    }

    /// Get base delay between retries in milliseconds
    fn base_delay_ms(&self) -> u64 {
        1000
    }

    /// Execute with automatic retry logic
    async fn execute_with_retry(&mut self) -> SynapsedResult<Self::Output> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts <= self.max_retries() {
            match self.execute().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if !self.is_retryable_error(&error) || attempts == self.max_retries() {
                        return Err(error);
                    }
                    
                    last_error = Some(error);
                    attempts += 1;
                    
                    // Exponential backoff
                    let delay = self.base_delay_ms() * 2_u64.pow(attempts as u32 - 1);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| SynapsedError::internal("Unexpected retry failure")))
    }
}

/// Trait for components that can be started and stopped
#[async_trait]
pub trait Lifecycle {
    /// Start the component
    async fn start(&mut self) -> SynapsedResult<()>;

    /// Stop the component
    async fn stop(&mut self) -> SynapsedResult<()>;

    /// Restart the component
    async fn restart(&mut self) -> SynapsedResult<()> {
        self.stop().await?;
        self.start().await
    }

    /// Check if the component is running
    fn is_running(&self) -> bool;

    /// Get the current lifecycle state
    fn lifecycle_state(&self) -> LifecycleState;
}

/// Lifecycle states
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LifecycleState {
    /// Not yet started
    Created,
    /// Starting up
    Starting,
    /// Running normally
    Running,
    /// Stopping
    Stopping,
    /// Stopped
    Stopped,
    /// Failed state
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Debug)]
    struct TestObservable {
        id: Uuid,
        name: String,
        state: ObservableState,
    }

    impl TestObservable {
        fn new(name: &str) -> Self {
            Self {
                id: Uuid::new_v4(),
                name: name.to_string(),
                state: ObservableState::Running,
            }
        }
    }

    #[async_trait]
    impl Observable for TestObservable {
        async fn status(&self) -> SynapsedResult<ObservableStatus> {
            Ok(ObservableStatus {
                state: self.state.clone(),
                last_updated: chrono::Utc::now(),
                metadata: HashMap::new(),
            })
        }

        async fn health(&self) -> SynapsedResult<HealthStatus> {
            Ok(HealthStatus {
                overall: HealthLevel::Healthy,
                checks: HashMap::new(),
                last_check: chrono::Utc::now(),
            })
        }

        async fn metrics(&self) -> SynapsedResult<HashMap<String, f64>> {
            let mut metrics = HashMap::new();
            metrics.insert("uptime".to_string(), 100.0);
            Ok(metrics)
        }

        fn describe(&self) -> String {
            format!("TestObservable: {} ({})", self.name, self.id)
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

    #[tokio::test]
    async fn test_observable_trait() {
        let observable = TestObservable::new("test");
        
        let status = observable.status().await.unwrap();
        assert_eq!(status.state, ObservableState::Running);

        let health = observable.health().await.unwrap();
        assert_eq!(health.overall, HealthLevel::Healthy);

        let metrics = observable.metrics().await.unwrap();
        assert!(metrics.contains_key("uptime"));

        let description = observable.describe();
        assert!(description.contains("TestObservable"));
    }

    #[test]
    fn test_identifiable_trait() {
        let observable = TestObservable::new("test");
        
        assert!(!observable.id().is_nil());
        assert_eq!(observable.name(), "test");
        assert_eq!(observable.type_name(), "TestObservable");
    }

    #[test]
    fn test_health_status() {
        let check = HealthCheck {
            level: HealthLevel::Healthy,
            message: "All systems operational".to_string(),
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(check.level, HealthLevel::Healthy);
        assert_eq!(check.message, "All systems operational");
    }

    #[test]
    fn test_observable_state() {
        let state = ObservableState::Running;
        assert_eq!(state, ObservableState::Running);
        assert_ne!(state, ObservableState::Failed);
    }
}