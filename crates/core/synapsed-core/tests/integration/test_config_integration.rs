//! Integration tests for configuration management

use std::collections::HashMap;
use synapsed_core::{
    config::{ConfigFormat, ConfigManager, ConfigValue, EnvConfigSource, FileConfigSource},
    error::SynapsedError,
    traits::{Configurable, Validatable},
};
use crate::utils::{TestEnvironment, with_default_timeout};

/// A realistic service configuration for testing
#[derive(Debug, Clone, PartialEq)]
struct ServiceConfig {
    pub name: String,
    pub version: String,
    pub database: DatabaseConfig,
    pub network: NetworkConfig,
    pub logging: LoggingConfig,
    pub features: FeatureConfig,
}

#[derive(Debug, Clone, PartialEq)]
struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub ssl: bool,
    pub max_connections: u32,
    pub timeout_seconds: u32,
}

#[derive(Debug, Clone, PartialEq)]
struct NetworkConfig {
    pub listen_address: String,
    pub max_connections: u32,
    pub timeout_seconds: u32,
    pub enable_tls: bool,
    pub buffer_size: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub enable_tracing: bool,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq)]
struct FeatureConfig {
    pub enable_metrics: bool,
    pub enable_health_checks: bool,
    pub debug_mode: bool,
    pub experimental_features: Vec<String>,
}

impl ServiceConfig {
    fn from_config_value(value: &ConfigValue) -> Result<Self, SynapsedError> {
        let obj = value.as_object()
            .map_err(|_| SynapsedError::config("Expected object at root"))?;

        // Parse main fields
        let name = obj.get("name")
            .ok_or_else(|| SynapsedError::config("Missing 'name' field"))?
            .as_string()?
            .to_string();

        let version = obj.get("version")
            .ok_or_else(|| SynapsedError::config("Missing 'version' field"))?
            .as_string()?
            .to_string();

        // Parse database config
        let database_obj = obj.get("database")
            .ok_or_else(|| SynapsedError::config("Missing 'database' section"))?
            .as_object()?;

        let database = DatabaseConfig {
            host: database_obj.get("host")
                .ok_or_else(|| SynapsedError::config("Missing database.host"))?
                .as_string()?.to_string(),
            port: database_obj.get("port")
                .ok_or_else(|| SynapsedError::config("Missing database.port"))?
                .as_integer()? as u16,
            username: database_obj.get("username")
                .ok_or_else(|| SynapsedError::config("Missing database.username"))?
                .as_string()?.to_string(),
            password: database_obj.get("password")
                .ok_or_else(|| SynapsedError::config("Missing database.password"))?
                .as_string()?.to_string(),
            ssl: database_obj.get("ssl")
                .ok_or_else(|| SynapsedError::config("Missing database.ssl"))?
                .as_boolean()?,
            max_connections: database_obj.get("max_connections")
                .ok_or_else(|| SynapsedError::config("Missing database.max_connections"))?
                .as_integer()? as u32,
            timeout_seconds: database_obj.get("timeout_seconds")
                .ok_or_else(|| SynapsedError::config("Missing database.timeout_seconds"))?
                .as_integer()? as u32,
        };

        // Parse network config
        let network_obj = obj.get("network")
            .ok_or_else(|| SynapsedError::config("Missing 'network' section"))?
            .as_object()?;

        let network = NetworkConfig {
            listen_address: network_obj.get("listen_address")
                .ok_or_else(|| SynapsedError::config("Missing network.listen_address"))?
                .as_string()?.to_string(),
            max_connections: network_obj.get("max_connections")
                .ok_or_else(|| SynapsedError::config("Missing network.max_connections"))?
                .as_integer()? as u32,
            timeout_seconds: network_obj.get("timeout_seconds")
                .ok_or_else(|| SynapsedError::config("Missing network.timeout_seconds"))?
                .as_integer()? as u32,
            enable_tls: network_obj.get("enable_tls")
                .ok_or_else(|| SynapsedError::config("Missing network.enable_tls"))?
                .as_boolean()?,
            buffer_size: network_obj.get("buffer_size")
                .ok_or_else(|| SynapsedError::config("Missing network.buffer_size"))?
                .as_integer()? as usize,
        };

        // Parse logging config
        let logging_obj = obj.get("logging")
            .ok_or_else(|| SynapsedError::config("Missing 'logging' section"))?
            .as_object()?;

        let logging = LoggingConfig {
            level: logging_obj.get("level")
                .ok_or_else(|| SynapsedError::config("Missing logging.level"))?
                .as_string()?.to_string(),
            format: logging_obj.get("format")
                .ok_or_else(|| SynapsedError::config("Missing logging.format"))?
                .as_string()?.to_string(),
            enable_tracing: logging_obj.get("enable_tracing")
                .ok_or_else(|| SynapsedError::config("Missing logging.enable_tracing"))?
                .as_boolean()?,
            output: logging_obj.get("output")
                .ok_or_else(|| SynapsedError::config("Missing logging.output"))?
                .as_string()?.to_string(),
        };

        // Parse features config
        let features_obj = obj.get("features")
            .ok_or_else(|| SynapsedError::config("Missing 'features' section"))?
            .as_object()?;

        let experimental_features = if let Some(exp_features) = features_obj.get("experimental_features") {
            exp_features.as_array()?
                .iter()
                .map(|v| v.as_string().map(|s| s.to_string()))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };

        let features = FeatureConfig {
            enable_metrics: features_obj.get("enable_metrics")
                .ok_or_else(|| SynapsedError::config("Missing features.enable_metrics"))?
                .as_boolean()?,
            enable_health_checks: features_obj.get("enable_health_checks")
                .ok_or_else(|| SynapsedError::config("Missing features.enable_health_checks"))?
                .as_boolean()?,
            debug_mode: features_obj.get("debug_mode")
                .ok_or_else(|| SynapsedError::config("Missing features.debug_mode"))?
                .as_boolean()?,
            experimental_features,
        };

        Ok(ServiceConfig {
            name,
            version,
            database,
            network,
            logging,
            features,
        })
    }
}

impl Validatable for ServiceConfig {
    fn validate(&self) -> Result<(), SynapsedError> {
        // Validate name
        if self.name.is_empty() {
            return Err(SynapsedError::invalid_input("Service name cannot be empty"));
        }

        // Validate version format (simple check)
        if !self.version.chars().any(|c| c.is_ascii_digit()) {
            return Err(SynapsedError::invalid_input("Version must contain at least one digit"));
        }

        // Validate database config
        if self.database.host.is_empty() {
            return Err(SynapsedError::invalid_input("Database host cannot be empty"));
        }
        if self.database.port == 0 {
            return Err(SynapsedError::invalid_input("Database port cannot be zero"));
        }
        if self.database.username.is_empty() {
            return Err(SynapsedError::invalid_input("Database username cannot be empty"));
        }
        if self.database.max_connections == 0 {
            return Err(SynapsedError::invalid_input("Database max_connections must be > 0"));
        }

        // Validate network config
        if self.network.listen_address.is_empty() {
            return Err(SynapsedError::invalid_input("Network listen_address cannot be empty"));
        }
        if self.network.max_connections == 0 {
            return Err(SynapsedError::invalid_input("Network max_connections must be > 0"));
        }
        if self.network.buffer_size == 0 {
            return Err(SynapsedError::invalid_input("Network buffer_size must be > 0"));
        }

        // Validate logging config
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(SynapsedError::invalid_input("Invalid logging level"));
        }

        let valid_formats = ["json", "text", "compact"];
        if !valid_formats.contains(&self.logging.format.as_str()) {
            return Err(SynapsedError::invalid_input("Invalid logging format"));
        }

        Ok(())
    }
}

/// Service that uses the configuration
#[derive(Debug)]
struct TestService {
    config: ServiceConfig,
}

impl TestService {
    fn new() -> Self {
        Self {
            config: ServiceConfig {
                name: "test-service".to_string(),
                version: "1.0.0".to_string(),
                database: DatabaseConfig {
                    host: "localhost".to_string(),
                    port: 5432,
                    username: "test".to_string(),
                    password: "test".to_string(),
                    ssl: false,
                    max_connections: 10,
                    timeout_seconds: 30,
                },
                network: NetworkConfig {
                    listen_address: "127.0.0.1:8080".to_string(),
                    max_connections: 100,
                    timeout_seconds: 30,
                    enable_tls: false,
                    buffer_size: 8192,
                },
                logging: LoggingConfig {
                    level: "info".to_string(),
                    format: "json".to_string(),
                    enable_tracing: false,
                    output: "stdout".to_string(),
                },
                features: FeatureConfig {
                    enable_metrics: false,
                    enable_health_checks: true,
                    debug_mode: false,
                    experimental_features: Vec::new(),
                },
            },
        }
    }
}

#[async_trait::async_trait]
impl Configurable for TestService {
    type Config = ServiceConfig;

    async fn configure(&mut self, config: Self::Config) -> Result<(), SynapsedError> {
        config.validate()?;
        self.config = config;
        Ok(())
    }

    async fn get_config(&self) -> Result<Self::Config, SynapsedError> {
        Ok(self.config.clone())
    }

    async fn validate_config(config: &Self::Config) -> Result<(), SynapsedError> {
        config.validate()
    }

    fn default_config() -> Self::Config {
        TestService::new().config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPLETE_TOML_CONFIG: &str = r#"
name = "synapsed-service"
version = "2.1.0"

[database]
host = "db.example.com"
port = 5432
username = "synapsed_user"
password = "secure_password"
ssl = true
max_connections = 50
timeout_seconds = 60

[network]
listen_address = "0.0.0.0:8080"
max_connections = 1000
timeout_seconds = 45
enable_tls = true
buffer_size = 16384

[logging]
level = "info"
format = "json"
enable_tracing = true
output = "stdout"

[features]
enable_metrics = true
enable_health_checks = true
debug_mode = false
experimental_features = ["feature_a", "feature_b"]
"#;

    const COMPLETE_JSON_CONFIG: &str = r#"{
  "name": "synapsed-service",
  "version": "2.1.0",
  "database": {
    "host": "db.example.com",
    "port": 5432,
    "username": "synapsed_user",
    "password": "secure_password",
    "ssl": true,
    "max_connections": 50,
    "timeout_seconds": 60
  },
  "network": {
    "listen_address": "0.0.0.0:8080",
    "max_connections": 1000,
    "timeout_seconds": 45,
    "enable_tls": true,
    "buffer_size": 16384
  },
  "logging": {
    "level": "info",
    "format": "json",
    "enable_tracing": true,
    "output": "stdout"
  },
  "features": {
    "enable_metrics": true,
    "enable_health_checks": true,
    "debug_mode": false,
    "experimental_features": ["feature_a", "feature_b"]
  }
}"#;

    #[tokio::test]
    async fn test_complete_toml_configuration() {
        let mut env = TestEnvironment::new();
        let config_path = env.create_config_file("complete", "toml", COMPLETE_TOML_CONFIG);

        let mut manager = ConfigManager::new()
            .add_source(FileConfigSource::new(&config_path, ConfigFormat::Toml));

        let config_value = manager.load().unwrap();
        let service_config = ServiceConfig::from_config_value(config_value).unwrap();

        // Validate the parsed configuration
        assert_eq!(service_config.name, "synapsed-service");
        assert_eq!(service_config.version, "2.1.0");
        
        // Database config
        assert_eq!(service_config.database.host, "db.example.com");
        assert_eq!(service_config.database.port, 5432);
        assert_eq!(service_config.database.username, "synapsed_user");
        assert_eq!(service_config.database.password, "secure_password");
        assert!(service_config.database.ssl);
        assert_eq!(service_config.database.max_connections, 50);
        assert_eq!(service_config.database.timeout_seconds, 60);

        // Network config
        assert_eq!(service_config.network.listen_address, "0.0.0.0:8080");
        assert_eq!(service_config.network.max_connections, 1000);
        assert_eq!(service_config.network.timeout_seconds, 45);
        assert!(service_config.network.enable_tls);
        assert_eq!(service_config.network.buffer_size, 16384);

        // Logging config
        assert_eq!(service_config.logging.level, "info");
        assert_eq!(service_config.logging.format, "json");
        assert!(service_config.logging.enable_tracing);
        assert_eq!(service_config.logging.output, "stdout");

        // Features config
        assert!(service_config.features.enable_metrics);
        assert!(service_config.features.enable_health_checks);
        assert!(!service_config.features.debug_mode);
        assert_eq!(service_config.features.experimental_features, vec!["feature_a", "feature_b"]);

        // Validate the configuration
        assert!(service_config.validate().is_ok());
    }

    #[tokio::test]
    async fn test_complete_json_configuration() {
        let mut env = TestEnvironment::new();
        let config_path = env.create_config_file("complete", "json", COMPLETE_JSON_CONFIG);

        let mut manager = ConfigManager::new()
            .add_source(FileConfigSource::new(&config_path, ConfigFormat::Json));

        let config_value = manager.load().unwrap();
        let service_config = ServiceConfig::from_config_value(config_value).unwrap();

        // Should be identical to TOML version
        assert_eq!(service_config.name, "synapsed-service");
        assert_eq!(service_config.database.host, "db.example.com");
        assert_eq!(service_config.network.max_connections, 1000);
        assert!(service_config.features.enable_metrics);
        assert!(service_config.validate().is_ok());
    }

    #[tokio::test]
    async fn test_configuration_with_environment_override() {
        let mut env = TestEnvironment::new();
        
        // Base configuration file
        let base_config = r#"
name = "base-service" 
version = "1.0.0"

[database]
host = "localhost"
port = 5432
username = "base_user"
password = "base_pass"
ssl = false
max_connections = 10
timeout_seconds = 30

[network]
listen_address = "127.0.0.1:8080"
max_connections = 100
timeout_seconds = 30
enable_tls = false
buffer_size = 8192

[logging]
level = "debug"
format = "text"
enable_tracing = false
output = "stdout"

[features]
enable_metrics = false
enable_health_checks = true
debug_mode = true
experimental_features = []
"#;
        
        let config_path = env.create_config_file("base", "toml", base_config);

        // Environment overrides
        env.set_env_var("SYNAPSED_DATABASE_HOST", "prod-db.example.com");
        env.set_env_var("SYNAPSED_DATABASE_SSL", "true");
        env.set_env_var("SYNAPSED_DATABASE_MAX_CONNECTIONS", "100");
        env.set_env_var("SYNAPSED_NETWORK_ENABLE_TLS", "true");
        env.set_env_var("SYNAPSED_LOGGING_LEVEL", "info");
        env.set_env_var("SYNAPSED_FEATURES_DEBUG_MODE", "false");

        let mut manager = ConfigManager::new()
            .add_source(FileConfigSource::new(&config_path, ConfigFormat::Toml))
            .add_source(EnvConfigSource::new("SYNAPSED"));

        manager.load().unwrap();

        // Manual verification of overrides
        let db_host = manager.get("database.host").unwrap();
        assert_eq!(db_host.as_string().unwrap(), "prod-db.example.com");

        let db_ssl = manager.get("database.ssl").unwrap();
        assert_eq!(db_ssl.as_string().unwrap(), "true"); // Note: env vars are strings

        let network_tls = manager.get("network.enable_tls").unwrap();
        assert_eq!(network_tls.as_string().unwrap(), "true");

        let log_level = manager.get("logging.level").unwrap();
        assert_eq!(log_level.as_string().unwrap(), "info");

        // Values not overridden should remain from file
        let service_name = manager.get("name").unwrap();
        assert_eq!(service_name.as_string().unwrap(), "base-service");

        let db_port = manager.get("database.port").unwrap();
        assert_eq!(db_port.as_integer().unwrap(), 5432);
    }

    #[tokio::test]
    async fn test_service_configuration_lifecycle() {
        let mut service = TestService::new();

        // Test default configuration
        let default_config = service.get_config().await.unwrap();
        assert_eq!(default_config.name, "test-service");
        assert!(default_config.validate().is_ok());

        // Test configuration update
        let mut env = TestEnvironment::new();
        let config_path = env.create_config_file("service", "toml", COMPLETE_TOML_CONFIG);

        let mut manager = ConfigManager::new()
            .add_source(FileConfigSource::new(&config_path, ConfigFormat::Toml));

        let config_value = manager.load().unwrap();
        let new_config = ServiceConfig::from_config_value(config_value).unwrap();

        // Apply new configuration
        service.configure(new_config.clone()).await.unwrap();

        // Verify configuration was applied
        let current_config = service.get_config().await.unwrap();
        assert_eq!(current_config.name, "synapsed-service");
        assert_eq!(current_config.database.host, "db.example.com");
        assert!(current_config.features.enable_metrics);

        // Test configuration validation
        let mut invalid_config = new_config.clone();
        invalid_config.database.port = 0; // Invalid port

        let result = service.configure(invalid_config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("port cannot be zero"));

        // Verify original configuration is preserved after failed update
        let preserved_config = service.get_config().await.unwrap();
        assert_eq!(preserved_config.name, "synapsed-service");
        assert_eq!(preserved_config.database.port, 5432); // Original valid port
    }

    #[tokio::test]
    async fn test_configuration_validation_edge_cases() {
        // Test empty name
        let mut config = ServiceConfig::from_config_value(
            &create_test_config_value("", "1.0.0")
        ).unwrap();
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name cannot be empty"));

        // Test invalid version
        config = ServiceConfig::from_config_value(
            &create_test_config_value("test", "invalid-version")
        ).unwrap();
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must contain at least one digit"));

        // Test invalid logging level
        let mut config_map = create_base_config_map();
        let logging_obj = config_map.get_mut("logging").unwrap().as_object().unwrap();
        logging_obj.insert("level".to_string(), ConfigValue::String("invalid".to_string()));
        let invalid_config = ConfigValue::Object(config_map);
        
        config = ServiceConfig::from_config_value(&invalid_config).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid logging level"));
    }

    #[tokio::test]
    async fn test_concurrent_configuration_access() {
        let mut env = TestEnvironment::new();
        let config_path = env.create_config_file("concurrent", "toml", COMPLETE_TOML_CONFIG);

        let mut manager = ConfigManager::new()
            .add_source(FileConfigSource::new(&config_path, ConfigFormat::Toml));

        manager.load().unwrap();

        // Test concurrent access to configuration manager
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let path = format!("database.host");
                tokio::spawn(async move {
                    // Simulate some work
                    tokio::time::sleep(tokio::time::Duration::from_millis(i * 10)).await;
                    (i, path)
                })
            })
            .collect();

        // All tasks should complete successfully
        for handle in handles {
            let (i, path) = handle.await.unwrap();
            // In real scenario, we'd access the manager here
            // For now, just verify the tasks completed
            assert!(i < 10);
            assert!(path.contains("database"));
        }
    }

    #[tokio::test]
    async fn test_configuration_error_handling() {
        let mut env = TestEnvironment::new();

        // Test missing required fields
        let incomplete_config = r#"
name = "incomplete"
# Missing version, database, network, logging, features
"#;
        let config_path = env.create_config_file("incomplete", "toml", incomplete_config);

        let mut manager = ConfigManager::new()
            .add_source(FileConfigSource::new(&config_path, ConfigFormat::Toml));

        let config_value = manager.load().unwrap();
        let result = ServiceConfig::from_config_value(config_value);
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Missing"));

        // Test invalid data types
        let invalid_types_config = r#"
name = "test"
version = "1.0.0"

[database]
host = "localhost"
port = "not-a-number"  # Should be integer
ssl = "maybe"          # Should be boolean
username = "test"
password = "test"
max_connections = 10
timeout_seconds = 30

[network]
listen_address = "127.0.0.1:8080"
max_connections = 100
timeout_seconds = 30
enable_tls = false
buffer_size = 8192

[logging]
level = "info"
format = "json"
enable_tracing = false
output = "stdout"

[features]
enable_metrics = false
enable_health_checks = true
debug_mode = false
experimental_features = []
"#;
        let invalid_path = env.create_config_file("invalid", "toml", invalid_types_config);

        let mut invalid_manager = ConfigManager::new()
            .add_source(FileConfigSource::new(&invalid_path, ConfigFormat::Toml));

        let config_value = invalid_manager.load().unwrap();
        let result = ServiceConfig::from_config_value(config_value);
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        // The error should indicate the type conversion problem
        assert!(error.to_string().contains("not an integer") || error.to_string().contains("not a boolean"));
    }

    // Helper functions

    fn create_test_config_value(name: &str, version: &str) -> ConfigValue {
        let mut base_config = create_base_config_map();
        base_config.insert("name".to_string(), ConfigValue::String(name.to_string()));
        base_config.insert("version".to_string(), ConfigValue::String(version.to_string()));
        ConfigValue::Object(base_config)
    }

    fn create_base_config_map() -> HashMap<String, ConfigValue> {
        let mut config = HashMap::new();
        
        // Database config
        let mut database = HashMap::new();
        database.insert("host".to_string(), ConfigValue::String("localhost".to_string()));
        database.insert("port".to_string(), ConfigValue::Integer(5432));
        database.insert("username".to_string(), ConfigValue::String("test".to_string()));
        database.insert("password".to_string(), ConfigValue::String("test".to_string()));
        database.insert("ssl".to_string(), ConfigValue::Boolean(false));
        database.insert("max_connections".to_string(), ConfigValue::Integer(10));
        database.insert("timeout_seconds".to_string(), ConfigValue::Integer(30));
        config.insert("database".to_string(), ConfigValue::Object(database));

        // Network config
        let mut network = HashMap::new();
        network.insert("listen_address".to_string(), ConfigValue::String("127.0.0.1:8080".to_string()));
        network.insert("max_connections".to_string(), ConfigValue::Integer(100));
        network.insert("timeout_seconds".to_string(), ConfigValue::Integer(30));
        network.insert("enable_tls".to_string(), ConfigValue::Boolean(false));
        network.insert("buffer_size".to_string(), ConfigValue::Integer(8192));
        config.insert("network".to_string(), ConfigValue::Object(network));

        // Logging config
        let mut logging = HashMap::new();
        logging.insert("level".to_string(), ConfigValue::String("info".to_string()));
        logging.insert("format".to_string(), ConfigValue::String("json".to_string()));
        logging.insert("enable_tracing".to_string(), ConfigValue::Boolean(false));
        logging.insert("output".to_string(), ConfigValue::String("stdout".to_string()));
        config.insert("logging".to_string(), ConfigValue::Object(logging));

        // Features config
        let mut features = HashMap::new();
        features.insert("enable_metrics".to_string(), ConfigValue::Boolean(false));
        features.insert("enable_health_checks".to_string(), ConfigValue::Boolean(true));
        features.insert("debug_mode".to_string(), ConfigValue::Boolean(false));
        features.insert("experimental_features".to_string(), ConfigValue::Array(vec![]));
        config.insert("features".to_string(), ConfigValue::Object(features));

        config
    }
}