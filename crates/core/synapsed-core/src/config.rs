//! Configuration management for the Synapsed ecosystem.
//!
//! This module provides utilities for loading, validating, and managing
//! configuration across all Synapsed components.

use crate::{SynapsedError, SynapsedResult, traits::Validatable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Trait for configuration sources
pub trait ConfigSource {
    /// Load configuration from this source
    fn load(&self) -> SynapsedResult<ConfigValue>;
    
    /// Get the source name
    fn source_name(&self) -> &str;
}

/// Configuration value that can hold different types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfigValue {
    /// String value
    String(String),
    /// Integer value
    Integer(i64),
    /// Float value
    Float(f64),
    /// Boolean value
    Boolean(bool),
    /// Array of values
    Array(Vec<ConfigValue>),
    /// Object/map of values
    Object(HashMap<String, ConfigValue>),
    /// Null value
    Null,
}

impl ConfigValue {
    /// Try to convert to string
    pub fn as_string(&self) -> SynapsedResult<&str> {
        match self {
            ConfigValue::String(s) => Ok(s),
            _ => Err(SynapsedError::config("Value is not a string")),
        }
    }

    /// Try to convert to integer
    pub fn as_integer(&self) -> SynapsedResult<i64> {
        match self {
            ConfigValue::Integer(i) => Ok(*i),
            _ => Err(SynapsedError::config("Value is not an integer")),
        }
    }

    /// Try to convert to float
    pub fn as_float(&self) -> SynapsedResult<f64> {
        match self {
            ConfigValue::Float(f) => Ok(*f),
            ConfigValue::Integer(i) => Ok(*i as f64),
            _ => Err(SynapsedError::config("Value is not a number")),
        }
    }

    /// Try to convert to boolean
    pub fn as_boolean(&self) -> SynapsedResult<bool> {
        match self {
            ConfigValue::Boolean(b) => Ok(*b),
            _ => Err(SynapsedError::config("Value is not a boolean")),
        }
    }

    /// Try to convert to array
    pub fn as_array(&self) -> SynapsedResult<&Vec<ConfigValue>> {
        match self {
            ConfigValue::Array(arr) => Ok(arr),
            _ => Err(SynapsedError::config("Value is not an array")),
        }
    }

    /// Try to convert to object
    pub fn as_object(&self) -> SynapsedResult<&HashMap<String, ConfigValue>> {
        match self {
            ConfigValue::Object(obj) => Ok(obj),
            _ => Err(SynapsedError::config("Value is not an object")),
        }
    }

    /// Check if value is null
    #[must_use] pub fn is_null(&self) -> bool {
        matches!(self, ConfigValue::Null)
    }
}

/// File-based configuration source
pub struct FileConfigSource {
    path: std::path::PathBuf,
    format: ConfigFormat,
}

/// Supported configuration formats
#[derive(Debug, Clone, Copy)]
pub enum ConfigFormat {
    /// TOML format
    Toml,
    /// JSON format
    Json,
    /// YAML format (requires yaml feature)
    #[cfg(feature = "yaml")]
    Yaml,
}

impl FileConfigSource {
    /// Create a new file config source
    pub fn new<P: AsRef<Path>>(path: P, format: ConfigFormat) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            format,
        }
    }

    /// Auto-detect format from file extension
    pub fn auto_detect<P: AsRef<Path>>(path: P) -> SynapsedResult<Self> {
        let path = path.as_ref();
        let format = match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => ConfigFormat::Toml,
            Some("json") => ConfigFormat::Json,
            #[cfg(feature = "yaml")]
            Some("yaml") | Some("yml") => ConfigFormat::Yaml,
            _ => return Err(SynapsedError::config("Cannot detect config format from file extension")),
        };

        Ok(Self::new(path, format))
    }
}

impl ConfigSource for FileConfigSource {
    fn load(&self) -> SynapsedResult<ConfigValue> {
        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| SynapsedError::config(format!("Failed to read config file: {e}")))?;

        match self.format {
            ConfigFormat::Toml => {
                let value: toml::Value = content.parse()
                    .map_err(|e| SynapsedError::config(format!("Failed to parse TOML: {e}")))?;
                Ok(toml_value_to_config_value(value))
            }
            ConfigFormat::Json => {
                let value: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| SynapsedError::config(format!("Failed to parse JSON: {e}")))?;
                Ok(json_value_to_config_value(value))
            }
            #[cfg(feature = "yaml")]
            ConfigFormat::Yaml => {
                let value: serde_yaml::Value = serde_yaml::from_str(&content)
                    .map_err(|e| SynapsedError::config(format!("Failed to parse YAML: {}", e)))?;
                Ok(yaml_value_to_config_value(value))
            }
        }
    }

    fn source_name(&self) -> &str {
        self.path.to_str().unwrap_or("unknown")
    }
}

/// Environment variable configuration source
pub struct EnvConfigSource {
    prefix: String,
}

impl EnvConfigSource {
    /// Create a new environment config source with prefix
    pub fn new<S: Into<String>>(prefix: S) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

impl ConfigSource for EnvConfigSource {
    fn load(&self) -> SynapsedResult<ConfigValue> {
        let mut config = HashMap::new();
        
        for (key, value) in std::env::vars() {
            if key.starts_with(&self.prefix) {
                let config_key = key.strip_prefix(&self.prefix)
                    .unwrap()
                    .trim_start_matches('_')
                    .to_lowercase();
                
                config.insert(config_key, ConfigValue::String(value));
            }
        }
        
        Ok(ConfigValue::Object(config))
    }

    fn source_name(&self) -> &'static str {
        "environment"
    }
}

/// Configuration manager that combines multiple sources
pub struct ConfigManager {
    sources: Vec<Box<dyn ConfigSource>>,
    cache: Option<ConfigValue>,
}

impl ConfigManager {
    /// Create a new config manager
    #[must_use] pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            cache: None,
        }
    }

    /// Add a configuration source
    pub fn add_source<S: ConfigSource + 'static>(mut self, source: S) -> Self {
        self.sources.push(Box::new(source));
        self
    }

    /// Load configuration from all sources
    pub fn load(&mut self) -> SynapsedResult<&ConfigValue> {
        let mut merged = ConfigValue::Object(HashMap::new());

        for source in &self.sources {
            let config = source.load()
                .map_err(|e| SynapsedError::config(format!("Failed to load from {}: {}", source.source_name(), e)))?;
            
            merged = merge_config_values(merged, config)?;
        }

        self.cache = Some(merged);
        Ok(self.cache.as_ref().unwrap())
    }

    /// Get a configuration value by path (e.g., "database.host")
    pub fn get(&self, path: &str) -> SynapsedResult<&ConfigValue> {
        let config = self.cache.as_ref()
            .ok_or_else(|| SynapsedError::config("Configuration not loaded"))?;
        
        get_config_value_by_path(config, path)
    }

    /// Check if configuration is loaded
    #[must_use] pub fn is_loaded(&self) -> bool {
        self.cache.is_some()
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Validatable for ConfigManager {
    fn validate(&self) -> SynapsedResult<()> {
        if self.cache.is_none() {
            return Err(SynapsedError::config("Configuration not loaded"));
        }
        Ok(())
    }
}

// Helper functions for value conversion and manipulation

fn toml_value_to_config_value(value: toml::Value) -> ConfigValue {
    match value {
        toml::Value::String(s) => ConfigValue::String(s),
        toml::Value::Integer(i) => ConfigValue::Integer(i),
        toml::Value::Float(f) => ConfigValue::Float(f),
        toml::Value::Boolean(b) => ConfigValue::Boolean(b),
        toml::Value::Array(arr) => {
            ConfigValue::Array(arr.into_iter().map(toml_value_to_config_value).collect())
        }
        toml::Value::Table(table) => {
            let mut map = HashMap::new();
            for (k, v) in table {
                map.insert(k, toml_value_to_config_value(v));
            }
            ConfigValue::Object(map)
        }
        toml::Value::Datetime(_) => ConfigValue::String(value.to_string()),
    }
}

fn json_value_to_config_value(value: serde_json::Value) -> ConfigValue {
    match value {
        serde_json::Value::String(s) => ConfigValue::String(s),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                ConfigValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                ConfigValue::Float(f)
            } else {
                ConfigValue::String(n.to_string())
            }
        }
        serde_json::Value::Bool(b) => ConfigValue::Boolean(b),
        serde_json::Value::Array(arr) => {
            ConfigValue::Array(arr.into_iter().map(json_value_to_config_value).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj {
                map.insert(k, json_value_to_config_value(v));
            }
            ConfigValue::Object(map)
        }
        serde_json::Value::Null => ConfigValue::Null,
    }
}

fn merge_config_values(base: ConfigValue, overlay: ConfigValue) -> SynapsedResult<ConfigValue> {
    match (base, overlay) {
        (ConfigValue::Object(mut base_map), ConfigValue::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                if let Some(existing) = base_map.remove(&key) {
                    base_map.insert(key, merge_config_values(existing, value)?);
                } else {
                    base_map.insert(key, value);
                }
            }
            Ok(ConfigValue::Object(base_map))
        }
        (_, overlay) => Ok(overlay),
    }
}

fn get_config_value_by_path<'a>(config: &'a ConfigValue, path: &str) -> SynapsedResult<&'a ConfigValue> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = config;

    for part in parts {
        match current {
            ConfigValue::Object(map) => {
                current = map.get(part)
                    .ok_or_else(|| SynapsedError::config(format!("Path '{path}' not found")))?;
            }
            _ => return Err(SynapsedError::config(format!("Cannot navigate path '{path}' on non-object value"))),
        }
    }

    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_config_value_conversions() {
        let string_val = ConfigValue::String("test".to_string());
        assert_eq!(string_val.as_string().unwrap(), "test");
        assert!(string_val.as_integer().is_err());

        let int_val = ConfigValue::Integer(42);
        assert_eq!(int_val.as_integer().unwrap(), 42);
        assert_eq!(int_val.as_float().unwrap(), 42.0);

        let bool_val = ConfigValue::Boolean(true);
        assert!(bool_val.as_boolean().unwrap());

        let null_val = ConfigValue::Null;
        assert!(null_val.is_null());
    }

    #[test]
    fn test_config_path_navigation() {
        let mut inner_map = HashMap::new();
        inner_map.insert("port".to_string(), ConfigValue::Integer(5432));
        
        let mut outer_map = HashMap::new();
        outer_map.insert("database".to_string(), ConfigValue::Object(inner_map));
        
        let config = ConfigValue::Object(outer_map);
        
        let port = get_config_value_by_path(&config, "database.port").unwrap();
        assert_eq!(port.as_integer().unwrap(), 5432);
        
        assert!(get_config_value_by_path(&config, "database.host").is_err());
        assert!(get_config_value_by_path(&config, "nonexistent").is_err());
    }

    #[test]
    fn test_env_config_source() {
        std::env::set_var("TEST_APP_HOST", "localhost");
        std::env::set_var("TEST_APP_PORT", "8080");
        std::env::set_var("OTHER_VAR", "should_not_appear");
        
        let source = EnvConfigSource::new("TEST_APP");
        let config = source.load().unwrap();
        
        let obj = config.as_object().unwrap();
        assert!(obj.contains_key("host"));
        assert!(obj.contains_key("port"));
        assert!(!obj.contains_key("other_var"));
        
        // Cleanup
        std::env::remove_var("TEST_APP_HOST");
        std::env::remove_var("TEST_APP_PORT");
        std::env::remove_var("OTHER_VAR");
    }

    #[test]
    fn test_config_manager() {
        let mut manager = ConfigManager::new();
        assert!(!manager.is_loaded());
        assert!(manager.validate().is_err());
        
        // Test with env source
        std::env::set_var("TEST_CONFIG_HOST", "localhost");
        
        manager = manager.add_source(EnvConfigSource::new("TEST_CONFIG"));
        manager.load().unwrap();
        
        assert!(manager.is_loaded());
        assert!(manager.validate().is_ok());
        
        let host = manager.get("host").unwrap();
        assert_eq!(host.as_string().unwrap(), "localhost");
        
        // Cleanup
        std::env::remove_var("TEST_CONFIG_HOST");
    }
}