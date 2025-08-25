//! Unit tests for SynapsedBuilder DSL
//!
//! Intent: Test fluent builder API and composition methods
//! Verification: All builder methods work correctly

use crate::builder::{
    SynapsedBuilder, BuilderConfig, StorageBackend, ObservabilityLevel,
    NetworkType, ValidationReport, ComposedApplication,
};
use crate::registry::Capability;
use serde_json::json;

#[test]
fn test_builder_creation() {
    let builder = SynapsedBuilder::new("test-app");
    assert_eq!(builder.config.name, "test-app");
    assert_eq!(builder.config.version, "0.1.0");
}

#[test]
fn test_builder_fluent_api() {
    let builder = SynapsedBuilder::new("fluent-app")
        .description("Test fluent API")
        .version("1.0.0")
        .output_dir("./custom-build");
    
    assert_eq!(builder.config.description, "Test fluent API");
    assert_eq!(builder.config.version, "1.0.0");
    assert_eq!(builder.config.output_dir, "./custom-build");
}

#[test]
fn test_add_components() {
    let builder = SynapsedBuilder::new("component-app")
        .add_component("synapsed-core")
        .add_component("synapsed-storage");
    
    assert_eq!(builder.components.len(), 2);
    assert!(builder.components.contains(&"synapsed-core".to_string()));
    assert!(builder.components.contains(&"synapsed-storage".to_string()));
}

#[test]
fn test_add_multiple_components() {
    let builder = SynapsedBuilder::new("multi-component-app")
        .add_components(vec![
            "synapsed-core".to_string(),
            "synapsed-intent".to_string(),
            "synapsed-verify".to_string(),
        ]);
    
    assert_eq!(builder.components.len(), 3);
}

#[test]
fn test_add_intent_verification() {
    let builder = SynapsedBuilder::new("intent-app")
        .add_intent_verification();
    
    assert!(builder.components.contains(&"synapsed-intent".to_string()));
    assert!(builder.components.contains(&"synapsed-verify".to_string()));
    assert_eq!(builder.connections.len(), 1);
}

#[test]
fn test_add_storage_backends() {
    let memory_builder = SynapsedBuilder::new("memory-app")
        .add_storage(StorageBackend::Memory);
    assert!(memory_builder.components.contains(&"synapsed-storage".to_string()));
    assert!(memory_builder.configurations.contains_key("synapsed-storage"));
    
    let sqlite_builder = SynapsedBuilder::new("sqlite-app")
        .add_storage(StorageBackend::Sqlite);
    assert!(sqlite_builder.components.contains(&"synapsed-storage".to_string()));
    
    let config = &sqlite_builder.configurations["synapsed-storage"];
    assert_eq!(config["backend"], "sqlite");
}

#[test]
fn test_add_observability_levels() {
    let basic_builder = SynapsedBuilder::new("basic-obs")
        .add_observability(ObservabilityLevel::Basic);
    assert!(basic_builder.components.contains(&"synapsed-substrates".to_string()));
    
    let full_builder = SynapsedBuilder::new("full-obs")
        .add_observability(ObservabilityLevel::Full);
    assert!(full_builder.components.contains(&"synapsed-substrates".to_string()));
    assert!(full_builder.components.contains(&"synapsed-monitor".to_string()));
}

#[test]
fn test_add_network_types() {
    let simple_builder = SynapsedBuilder::new("simple-net")
        .add_network(NetworkType::Simple);
    assert!(simple_builder.components.contains(&"synapsed-net".to_string()));
    
    let p2p_builder = SynapsedBuilder::new("p2p-net")
        .add_network(NetworkType::P2P);
    assert!(p2p_builder.components.contains(&"synapsed-net".to_string()));
    assert!(p2p_builder.components.contains(&"synapsed-routing".to_string()));
    
    let consensus_builder = SynapsedBuilder::new("consensus-net")
        .add_network(NetworkType::Consensus);
    assert!(consensus_builder.components.contains(&"synapsed-net".to_string()));
    assert!(consensus_builder.components.contains(&"synapsed-consensus".to_string()));
}

#[test]
fn test_add_payments() {
    let builder = SynapsedBuilder::new("payment-app")
        .add_payments();
    
    assert!(builder.components.contains(&"synapsed-payments".to_string()));
    assert!(builder.components.contains(&"synapsed-identity".to_string()));
    assert!(builder.components.contains(&"synapsed-crypto".to_string()));
}

#[test]
fn test_connections() {
    let builder = SynapsedBuilder::new("connected-app")
        .connect("comp1", "output", "comp2", "input");
    
    assert_eq!(builder.connections.len(), 1);
    assert_eq!(builder.connections[0].from.component, "comp1");
    assert_eq!(builder.connections[0].from.port, "output");
    assert_eq!(builder.connections[0].to.component, "comp2");
    assert_eq!(builder.connections[0].to.port, "input");
}

#[test]
fn test_configuration() {
    let config = json!({
        "key": "value",
        "number": 42,
        "nested": {
            "field": true
        }
    });
    
    let builder = SynapsedBuilder::new("config-app")
        .configure("test-component", config.clone());
    
    assert!(builder.configurations.contains_key("test-component"));
    assert_eq!(builder.configurations["test-component"], config);
}

#[test]
fn test_environment_variables() {
    let builder = SynapsedBuilder::new("env-app")
        .env("RUST_LOG", "debug")
        .env("CUSTOM_VAR", "value");
    
    assert_eq!(builder.environment.len(), 2);
    assert_eq!(builder.environment["RUST_LOG"], "debug");
    assert_eq!(builder.environment["CUSTOM_VAR"], "value");
}

#[test]
fn test_with_capability() {
    let builder = SynapsedBuilder::new("capability-app")
        .with_capability(Capability::Storage);
    
    // With defaults loaded, should find a storage component
    assert!(builder.is_ok());
    let builder = builder.unwrap();
    assert!(builder.components.iter().any(|c| c.contains("storage")));
}

#[test]
fn test_with_multiple_capabilities() {
    let builder = SynapsedBuilder::new("multi-capability-app")
        .with_capabilities(vec![
            Capability::Storage,
            Capability::Observability,
        ]);
    
    assert!(builder.is_ok());
    let builder = builder.unwrap();
    assert!(builder.components.len() >= 2);
}

#[test]
fn test_skip_validations() {
    let builder = SynapsedBuilder::new("no-validation-app")
        .skip_validations();
    
    assert!(!builder.validations_enabled);
}

#[test]
fn test_validation_report() {
    let mut report = ValidationReport::default();
    assert!(!report.has_errors());
    
    report.errors.push("Test error".to_string());
    assert!(report.has_errors());
    
    report.warnings.push("Test warning".to_string());
    report.info.push("Test info".to_string());
    
    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.warnings.len(), 1);
    assert_eq!(report.info.len(), 1);
}

#[test]
fn test_storage_backend_strings() {
    assert_eq!(StorageBackend::Memory.to_string(), "memory");
    assert_eq!(StorageBackend::Sqlite.to_string(), "sqlite");
    assert_eq!(StorageBackend::RocksDb.to_string(), "rocksdb");
    assert_eq!(StorageBackend::Redis.to_string(), "redis");
}

#[test]
fn test_storage_backend_paths() {
    assert_eq!(StorageBackend::Memory.default_path(), ":memory:");
    assert_eq!(StorageBackend::Sqlite.default_path(), "./data/app.db");
    assert_eq!(StorageBackend::RocksDb.default_path(), "./data/rocksdb");
    assert_eq!(StorageBackend::Redis.default_path(), "redis://localhost:6379");
}

#[test]
fn test_build_with_minimal_components() {
    let builder = SynapsedBuilder::new("minimal-app")
        .add_component("synapsed-core")
        .skip_validations(); // Skip for testing without full registry
    
    let result = builder.build();
    // Would fail without proper registry setup, but structure is tested
    assert!(result.is_err()); // Expected without mock registry
}

#[test]
fn test_composed_application_generation() {
    use crate::builder::CargoManifest;
    use std::collections::HashMap;
    
    let app = ComposedApplication {
        name: "test-app".to_string(),
        version: "0.1.0".to_string(),
        description: "Test application".to_string(),
        components: vec!["synapsed-core".to_string()],
        manifest: CargoManifest {
            package_name: "test-app".to_string(),
            version: "0.1.0".to_string(),
            dependencies: HashMap::new(),
        },
        config_files: HashMap::new(),
        environment: HashMap::new(),
    };
    
    let cargo_toml = app.generate_cargo_toml();
    assert!(cargo_toml.contains("[package]"));
    assert!(cargo_toml.contains("name = \"test-app\""));
    
    let main_rs = app.generate_main_rs();
    assert!(main_rs.contains("async fn main()"));
    assert!(main_rs.contains("use synapsed_core;"));
}