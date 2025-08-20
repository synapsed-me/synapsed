//! Unit tests for configuration module

use std::collections::HashMap;
use synapsed_core::{
    config::{ConfigFormat, ConfigManager, ConfigSource, ConfigValue, EnvConfigSource, FileConfigSource},
    error::SynapsedError,
    traits::Validatable,
};
use crate::utils::{TestEnvironment, assert_config_value_string, assert_config_value_integer, assert_config_value_boolean};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_value_types() {
        // String values
        let string_val = ConfigValue::String("test".to_string());
        assert_eq!(string_val.as_string().unwrap(), "test");
        assert!(string_val.as_integer().is_err());
        assert!(string_val.as_float().is_err());
        assert!(string_val.as_boolean().is_err());
        assert!(!string_val.is_null());

        // Integer values
        let int_val = ConfigValue::Integer(42);
        assert_eq!(int_val.as_integer().unwrap(), 42);
        assert_eq!(int_val.as_float().unwrap(), 42.0); // Should convert to float
        assert!(int_val.as_string().is_err());
        assert!(int_val.as_boolean().is_err());

        // Float values
        let float_val = ConfigValue::Float(3.14);
        assert_eq!(float_val.as_float().unwrap(), 3.14);
        assert!(float_val.as_integer().is_err());
        assert!(float_val.as_string().is_err());

        // Boolean values
        let bool_val = ConfigValue::Boolean(true);
        assert!(bool_val.as_boolean().unwrap());
        assert!(bool_val.as_string().is_err());
        assert!(bool_val.as_integer().is_err());

        // Array values
        let array_val = ConfigValue::Array(vec![
            ConfigValue::String("item1".to_string()),
            ConfigValue::String("item2".to_string()),
        ]);
        let array = array_val.as_array().unwrap();
        assert_eq!(array.len(), 2);
        assert_eq!(array[0].as_string().unwrap(), "item1");

        // Object values
        let mut obj_map = HashMap::new();
        obj_map.insert("key1".to_string(), ConfigValue::String("value1".to_string()));
        obj_map.insert("key2".to_string(), ConfigValue::Integer(123));
        let obj_val = ConfigValue::Object(obj_map.clone());
        let obj = obj_val.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert!(obj.contains_key("key1"));
        assert!(obj.contains_key("key2"));

        // Null values
        let null_val = ConfigValue::Null;
        assert!(null_val.is_null());
        assert!(null_val.as_string().is_err());
    }

    #[test]
    fn test_config_value_error_messages() {
        let int_val = ConfigValue::Integer(42);
        
        match int_val.as_string() {
            Err(SynapsedError::Configuration(msg)) => {
                assert!(msg.contains("not a string"));
            }
            _ => panic!("Expected configuration error"),
        }

        let string_val = ConfigValue::String("test".to_string());
        match string_val.as_integer() {
            Err(SynapsedError::Configuration(msg)) => {
                assert!(msg.contains("not an integer"));
            }
            _ => panic!("Expected configuration error"),
        }
    }

    #[test]
    fn test_file_config_source_auto_detect() {
        // Test TOML detection
        let toml_source = FileConfigSource::auto_detect("config.toml").unwrap();
        assert_eq!(toml_source.source_name(), "config.toml");

        // Test JSON detection
        let json_source = FileConfigSource::auto_detect("config.json").unwrap();
        assert_eq!(json_source.source_name(), "config.json");

        // Test unsupported format
        let result = FileConfigSource::auto_detect("config.xml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot detect config format"));
    }

    #[test]
    fn test_file_config_source_toml() {
        let mut env = TestEnvironment::new();
        let toml_content = r#"
[database]
host = "localhost"
port = 5432
ssl = true

[network]
timeout = 30
max_connections = 100
"#;
        let config_path = env.create_config_file("test", "toml", toml_content);
        let source = FileConfigSource::new(&config_path, ConfigFormat::Toml);
        
        let config = source.load().unwrap();
        let obj = config.as_object().unwrap();
        
        assert!(obj.contains_key("database"));
        assert!(obj.contains_key("network"));
        
        let database = obj["database"].as_object().unwrap();
        assert_eq!(database["host"].as_string().unwrap(), "localhost");
        assert_eq!(database["port"].as_integer().unwrap(), 5432);
        assert!(database["ssl"].as_boolean().unwrap());
    }

    #[test]
    fn test_file_config_source_json() {
        let mut env = TestEnvironment::new();
        let json_content = r#"{
  "database": {
    "host": "localhost",
    "port": 5432,
    "ssl": true
  },
  "network": {
    "timeout": 30,
    "max_connections": 100
  }
}"#;
        let config_path = env.create_config_file("test", "json", json_content);
        let source = FileConfigSource::new(&config_path, ConfigFormat::Json);
        
        let config = source.load().unwrap();
        let obj = config.as_object().unwrap();
        
        assert!(obj.contains_key("database"));
        assert!(obj.contains_key("network"));
        
        let database = obj["database"].as_object().unwrap();
        assert_eq!(database["host"].as_string().unwrap(), "localhost");
        assert_eq!(database["port"].as_integer().unwrap(), 5432);
        assert!(database["ssl"].as_boolean().unwrap());
    }

    #[test]
    fn test_file_config_source_invalid_toml() {
        let mut env = TestEnvironment::new();
        let invalid_toml = r#"
[database
host = "localhost"
port = not-a-number
"#;
        let config_path = env.create_config_file("invalid", "toml", invalid_toml);
        let source = FileConfigSource::new(&config_path, ConfigFormat::Toml);
        
        let result = source.load();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse TOML"));
    }

    #[test]
    fn test_file_config_source_invalid_json() {
        let mut env = TestEnvironment::new();
        let invalid_json = r#"{
  "database": {
    "host": "localhost",
    "port": 5432,
  }
}"#; // Trailing comma makes it invalid JSON
        let config_path = env.create_config_file("invalid", "json", invalid_json);
        let source = FileConfigSource::new(&config_path, ConfigFormat::Json);
        
        let result = source.load();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse JSON"));
    }

    #[test]
    fn test_file_config_source_missing_file() {
        let source = FileConfigSource::new("nonexistent.toml", ConfigFormat::Toml);
        let result = source.load();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read config file"));
    }

    #[test]
    fn test_env_config_source() {
        let mut env = TestEnvironment::new();
        env.set_env_var("TEST_APP_HOST", "localhost");
        env.set_env_var("TEST_APP_PORT", "8080");
        env.set_env_var("TEST_APP_DEBUG", "true");
        env.set_env_var("OTHER_VAR", "should_not_appear");
        
        let source = EnvConfigSource::new("TEST_APP");
        let config = source.load().unwrap();
        
        let obj = config.as_object().unwrap();
        assert!(obj.contains_key("host"));
        assert!(obj.contains_key("port"));
        assert!(obj.contains_key("debug"));
        assert!(!obj.contains_key("other_var"));
        
        assert_eq!(obj["host"].as_string().unwrap(), "localhost");
        assert_eq!(obj["port"].as_string().unwrap(), "8080");
        assert_eq!(obj["debug"].as_string().unwrap(), "true");
        assert_eq!(source.source_name(), "environment");
    }

    #[test]
    fn test_env_config_source_no_prefix() {
        let source = EnvConfigSource::new("NONEXISTENT_PREFIX");
        let config = source.load().unwrap();
        let obj = config.as_object().unwrap();
        assert!(obj.is_empty());
    }

    #[test]
    fn test_config_manager_empty() {
        let mut manager = ConfigManager::new();
        assert!(!manager.is_loaded());
        assert!(manager.validate().is_err());
        
        let result = manager.get("any.path");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Configuration not loaded"));
    }

    #[test]
    fn test_config_manager_single_source() {
        let mut env = TestEnvironment::new();
        env.set_env_var("TEST_CONFIG_HOST", "localhost");
        env.set_env_var("TEST_CONFIG_PORT", "8080");
        
        let mut manager = ConfigManager::new()
            .add_source(EnvConfigSource::new("TEST_CONFIG"));
        
        manager.load().unwrap();
        assert!(manager.is_loaded());
        assert!(manager.validate().is_ok());
        
        let host = manager.get("host").unwrap();
        assert_eq!(host.as_string().unwrap(), "localhost");
        
        let port = manager.get("port").unwrap();
        assert_eq!(port.as_string().unwrap(), "8080");
    }

    #[test]
    fn test_config_manager_multiple_sources() {
        let mut env = TestEnvironment::new();
        
        // File source
        let toml_content = r#"
[database]
host = "file-host"
port = 5432

[network]
timeout = 30
"#;
        let config_path = env.create_config_file("test", "toml", toml_content);
        
        // Environment source (should override file)
        env.set_env_var("TEST_DATABASE_HOST", "env-host");
        env.set_env_var("TEST_NETWORK_MAX_CONNECTIONS", "200");
        
        let mut manager = ConfigManager::new()
            .add_source(FileConfigSource::new(&config_path, ConfigFormat::Toml))
            .add_source(EnvConfigSource::new("TEST"));
        
        manager.load().unwrap();
        
        // Environment should override file
        let host = manager.get("database.host").unwrap();
        assert_eq!(host.as_string().unwrap(), "env-host");
        
        // File values should still be present
        let port = manager.get("database.port").unwrap();
        assert_eq!(port.as_integer().unwrap(), 5432);
        
        let timeout = manager.get("network.timeout").unwrap();
        assert_eq!(timeout.as_integer().unwrap(), 30);
        
        // Environment-only values should be present
        let max_conn = manager.get("network.max_connections").unwrap();
        assert_eq!(max_conn.as_string().unwrap(), "200");
    }

    #[test]
    fn test_config_manager_path_navigation() {
        let mut env = TestEnvironment::new();
        let toml_content = r#"
[database]
host = "localhost"
port = 5432

[database.connection_pool]
min_size = 5
max_size = 20

[network.security]
enable_tls = true
cert_path = "/etc/ssl/cert.pem"
"#;
        let config_path = env.create_config_file("test", "toml", toml_content);
        
        let mut manager = ConfigManager::new()
            .add_source(FileConfigSource::new(&config_path, ConfigFormat::Toml));
        
        manager.load().unwrap();
        
        // Test simple path
        let host = manager.get("database.host").unwrap();
        assert_eq!(host.as_string().unwrap(), "localhost");
        
        // Test nested path
        let min_size = manager.get("database.connection_pool.min_size").unwrap();
        assert_eq!(min_size.as_integer().unwrap(), 5);
        
        let enable_tls = manager.get("network.security.enable_tls").unwrap();
        assert!(enable_tls.as_boolean().unwrap());
        
        // Test invalid paths
        assert!(manager.get("nonexistent").is_err());
        assert!(manager.get("database.nonexistent").is_err());
        assert!(manager.get("database.host.invalid").is_err());
    }

    #[test]
    fn test_config_manager_source_loading_error() {
        let mut manager = ConfigManager::new()
            .add_source(FileConfigSource::new("nonexistent.toml", ConfigFormat::Toml));
        
        let result = manager.load();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to load from"));
    }

    #[test]
    fn test_config_value_merge() {
        // This tests the internal merge functionality through ConfigManager
        let mut env = TestEnvironment::new();
        
        let base_toml = r#"
[server]
host = "localhost"
port = 8080
debug = false

[database]
host = "db-host"
port = 5432
"#;
        
        let overlay_toml = r#"
[server]
port = 9090
debug = true
workers = 4

[logging]
level = "info"
"#;
        
        let base_path = env.create_config_file("base", "toml", base_toml);
        let overlay_path = env.create_config_file("overlay", "toml", overlay_toml);
        
        let mut manager = ConfigManager::new()
            .add_source(FileConfigSource::new(&base_path, ConfigFormat::Toml))
            .add_source(FileConfigSource::new(&overlay_path, ConfigFormat::Toml));
        
        manager.load().unwrap();
        
        // Base values should be preserved
        let server_host = manager.get("server.host").unwrap();
        assert_eq!(server_host.as_string().unwrap(), "localhost");
        
        let db_host = manager.get("database.host").unwrap();
        assert_eq!(db_host.as_string().unwrap(), "db-host");
        
        // Overlay values should override
        let server_port = manager.get("server.port").unwrap();
        assert_eq!(server_port.as_integer().unwrap(), 9090);
        
        let debug = manager.get("server.debug").unwrap();
        assert!(debug.as_boolean().unwrap());
        
        // New values should be added
        let workers = manager.get("server.workers").unwrap();
        assert_eq!(workers.as_integer().unwrap(), 4);
        
        let log_level = manager.get("logging.level").unwrap();
        assert_eq!(log_level.as_string().unwrap(), "info");
    }

    #[test]
    fn test_config_value_equality() {
        let val1 = ConfigValue::String("test".to_string());
        let val2 = ConfigValue::String("test".to_string());
        let val3 = ConfigValue::String("different".to_string());
        let val4 = ConfigValue::Integer(42);
        
        assert_eq!(val1, val2);
        assert_ne!(val1, val3);
        assert_ne!(val1, val4);
    }

    #[test]
    fn test_config_value_clone() {
        let original = ConfigValue::Object({
            let mut map = HashMap::new();
            map.insert("key".to_string(), ConfigValue::String("value".to_string()));
            map
        });
        
        let cloned = original.clone();
        assert_eq!(original, cloned);
        
        // Ensure deep clone
        if let (ConfigValue::Object(orig_map), ConfigValue::Object(clone_map)) = (&original, &cloned) {
            assert_eq!(orig_map["key"], clone_map["key"]);
        } else {
            panic!("Expected Object values");
        }
    }

    #[test]
    fn test_config_value_debug() {
        let val = ConfigValue::String("test".to_string());
        let debug_str = format!("{:?}", val);
        assert!(debug_str.contains("String"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_config_format_debug() {
        let toml_fmt = ConfigFormat::Toml;
        let json_fmt = ConfigFormat::Json;
        
        let toml_debug = format!("{:?}", toml_fmt);
        let json_debug = format!("{:?}", json_fmt);
        
        assert!(toml_debug.contains("Toml"));
        assert!(json_debug.contains("Json"));
    }

    #[test]
    fn test_helper_assertions() {
        let string_val = ConfigValue::String("test".to_string());
        assert_config_value_string(&string_val, "test");
        
        let int_val = ConfigValue::Integer(42);
        assert_config_value_integer(&int_val, 42);
        
        let bool_val = ConfigValue::Boolean(true);
        assert_config_value_boolean(&bool_val, true);
    }

    #[test]
    #[should_panic(expected = "Expected string config value")]
    fn test_helper_assertion_type_mismatch() {
        let int_val = ConfigValue::Integer(42);
        assert_config_value_string(&int_val, "should fail");
    }

    #[test]
    #[should_panic(expected = "Expected integer config value")]
    fn test_helper_assertion_value_mismatch() {
        let int_val = ConfigValue::Integer(42);
        assert_config_value_integer(&int_val, 43);
    }

    #[test]
    fn test_complex_nested_config() {
        let mut env = TestEnvironment::new();
        let complex_toml = r#"
[app]
name = "synapsed-test"
version = "1.0.0"

[app.database]
primary = { host = "db1.example.com", port = 5432 }
replica = { host = "db2.example.com", port = 5432 }

[app.services]
enabled = ["auth", "api", "websocket"]

[[app.workers]]
name = "worker-1"
threads = 4

[[app.workers]]
name = "worker-2"
threads = 8

[monitoring.metrics]
enabled = true
port = 9090
collectors = ["prometheus", "statsd"]

[monitoring.tracing]
enabled = true
endpoint = "http://jaeger:14268/api/traces"
sample_rate = 0.1
"#;
        
        let config_path = env.create_config_file("complex", "toml", complex_toml);
        let mut manager = ConfigManager::new()
            .add_source(FileConfigSource::new(&config_path, ConfigFormat::Toml));
        
        manager.load().unwrap();
        
        // Test basic values
        let app_name = manager.get("app.name").unwrap();
        assert_eq!(app_name.as_string().unwrap(), "synapsed-test");
        
        // Test nested objects
        let primary_host = manager.get("app.database.primary.host").unwrap();
        assert_eq!(primary_host.as_string().unwrap(), "db1.example.com");
        
        // Test arrays
        let services = manager.get("app.services.enabled").unwrap();
        let services_array = services.as_array().unwrap();
        assert_eq!(services_array.len(), 3);
        assert_eq!(services_array[0].as_string().unwrap(), "auth");
        
        // Test array of objects (workers)
        let workers = manager.get("app.workers").unwrap();
        let workers_array = workers.as_array().unwrap();
        assert_eq!(workers_array.len(), 2);
        
        let worker1 = workers_array[0].as_object().unwrap();
        assert_eq!(worker1["name"].as_string().unwrap(), "worker-1");
        assert_eq!(worker1["threads"].as_integer().unwrap(), 4);
        
        // Test monitoring config
        let tracing_enabled = manager.get("monitoring.tracing.enabled").unwrap();
        assert!(tracing_enabled.as_boolean().unwrap());
        
        let sample_rate = manager.get("monitoring.tracing.sample_rate").unwrap();
        assert_eq!(sample_rate.as_float().unwrap(), 0.1);
    }

    #[test]
    fn test_config_manager_default() {
        let manager = ConfigManager::default();
        assert!(!manager.is_loaded());
        assert!(manager.validate().is_err());
    }

    #[test]
    fn test_env_config_prefix_handling() {
        let mut env = TestEnvironment::new();
        
        // Test various prefix formats
        env.set_env_var("MYAPP_HOST", "host1");
        env.set_env_var("MYAPP_PORT", "8080");
        env.set_env_var("MYAPP_DEBUG_ENABLED", "true");
        env.set_env_var("OTHER_HOST", "should_not_appear");
        
        let source = EnvConfigSource::new("MYAPP");
        let config = source.load().unwrap();
        let obj = config.as_object().unwrap();
        
        assert_eq!(obj["host"].as_string().unwrap(), "host1");
        assert_eq!(obj["port"].as_string().unwrap(), "8080");
        assert_eq!(obj["debug_enabled"].as_string().unwrap(), "true");
        assert!(!obj.contains_key("other_host"));
    }
}