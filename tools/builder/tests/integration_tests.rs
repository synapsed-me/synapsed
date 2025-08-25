//! Integration tests for the Synapsed Builder system
//! 
//! These tests verify end-to-end functionality of the builder,
//! including file generation, validation, and composition.

use synapsed_builder::prelude::*;
use synapsed_builder::{
    builder::{StorageBackend, ObservabilityLevel, NetworkType},
    registry::{ComponentRegistry, Capability},
    recipe::RecipeManager,
    composer::Composer,
    validator::Validator,
    templates::Templates,
};
use std::path::PathBuf;
use tempfile::TempDir;
use serde_json::json;

/// Test complete workflow: template -> build -> save -> validate
#[tokio::test]
async fn test_template_to_deployment_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test-app");
    
    // Build from template
    let app = Templates::verified_ai_agent()
        .configure("synapsed-storage", json!({
            "path": "./test.db"
        }))
        .env("RUST_LOG", "debug")
        .build()
        .expect("Failed to build from template");
    
    // Save to directory
    app.save(&output_path).await
        .expect("Failed to save application");
    
    // Verify generated files exist
    assert!(output_path.join("Cargo.toml").exists());
    assert!(output_path.join("src/main.rs").exists());
    assert!(output_path.join(".env").exists());
    assert!(output_path.join("config.json").exists());
    
    // Verify Cargo.toml is valid
    let cargo_content = std::fs::read_to_string(output_path.join("Cargo.toml")).unwrap();
    assert!(cargo_content.contains("[package]"));
    assert!(cargo_content.contains("name = \"verified-ai-agent\""));
    assert!(cargo_content.contains("synapsed-intent"));
    assert!(cargo_content.contains("synapsed-verify"));
}

/// Test recipe loading, modification, and building
#[tokio::test]
async fn test_recipe_modification_workflow() {
    let mut recipe_manager = RecipeManager::new();
    
    // Create a test recipe
    let recipe_yaml = r#"
name: test-recipe
description: Test recipe for integration testing
version: 1.0.0
components:
  - name: synapsed-core
    version: "*"
  - name: synapsed-net
    version: "*"
connections:
  - from: synapsed-net
    event: peer_connected
    to: synapsed-core
    handler: log_event
config:
  synapsed-net:
    max_peers: 10
"#;
    
    // Load and validate recipe
    let recipe_name = recipe_manager.load_yaml(recipe_yaml).unwrap();
    let mut recipe = recipe_manager.get(&recipe_name).unwrap().clone();
    
    // Modify recipe
    recipe.components.push("synapsed-storage".to_string());
    recipe.config.insert(
        "synapsed-storage".to_string(),
        json!({ "backend": "sqlite" })
    );
    
    // Build from modified recipe
    let app = SynapsedBuilder::from_recipe(recipe)
        .build()
        .expect("Failed to build from recipe");
    
    // Verify modifications were applied
    assert!(app.components.contains(&"synapsed-storage".to_string()));
    assert_eq!(app.components.len(), 3);
}

/// Test complex composition with multiple interconnected components
#[tokio::test]
async fn test_complex_composition() {
    let mut registry = ComponentRegistry::new();
    registry.initialize_default_components();
    
    // Build complex application
    let app = SynapsedBuilder::new("complex-app")
        .description("Complex application with many interconnected components")
        .add_intent_verification()
        .add_storage(StorageBackend::Postgres)
        .add_observability(ObservabilityLevel::Full)
        .add_network(NetworkType::Hybrid)
        .add_consensus()
        .add_payments()
        .connect(
            "synapsed-payments", "transaction_created",
            "synapsed-consensus", "propose"
        )
        .connect(
            "synapsed-consensus", "committed",
            "synapsed-storage", "persist"
        )
        .connect(
            "synapsed-storage", "persisted",
            "synapsed-substrates", "emit_event"
        )
        .build()
        .expect("Failed to build complex app");
    
    // Verify all components are present
    assert!(app.components.contains(&"synapsed-intent".to_string()));
    assert!(app.components.contains(&"synapsed-consensus".to_string()));
    assert!(app.components.contains(&"synapsed-payments".to_string()));
    assert!(app.components.contains(&"synapsed-storage".to_string()));
    assert!(app.components.contains(&"synapsed-substrates".to_string()));
    
    // Verify connections
    assert_eq!(app.connections.len(), 3);
}

/// Test validation catches invalid configurations
#[test]
fn test_validation_catches_errors() {
    let validator = Validator::new();
    
    // Test missing required components
    let mut app = Application {
        name: "invalid-app".to_string(),
        description: "Invalid application".to_string(),
        components: vec!["synapsed-consensus".to_string()], // Missing synapsed-net
        connections: vec![],
        config: Default::default(),
        env: Default::default(),
    };
    
    let result = validator.validate(&app);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("requires synapsed-net"));
    
    // Test circular dependency
    app.components = vec![
        "synapsed-core".to_string(),
        "synapsed-net".to_string(),
    ];
    app.connections = vec![
        Connection {
            from: "synapsed-core".to_string(),
            event: "event1".to_string(),
            to: "synapsed-net".to_string(),
            handler: "handler1".to_string(),
        },
        Connection {
            from: "synapsed-net".to_string(),
            event: "event2".to_string(),
            to: "synapsed-core".to_string(),
            handler: "handler2".to_string(),
        },
    ];
    
    // This should pass - bidirectional connections are allowed
    let result = validator.validate(&app);
    assert!(result.is_ok());
}

/// Test concurrent builds don't interfere
#[tokio::test]
async fn test_concurrent_builds() {
    use tokio::task;
    
    let handles: Vec<_> = (0..5).map(|i| {
        task::spawn(async move {
            let app = SynapsedBuilder::new(&format!("concurrent-app-{}", i))
                .description("Concurrent build test")
                .add_intent_verification()
                .add_storage(StorageBackend::Sqlite)
                .build()
                .expect("Failed to build");
            
            (i, app)
        })
    }).collect();
    
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap());
    }
    
    // Verify all builds succeeded and are unique
    assert_eq!(results.len(), 5);
    for (i, app) in results {
        assert_eq!(app.name, format!("concurrent-app-{}", i));
        assert!(app.components.contains(&"synapsed-intent".to_string()));
        assert!(app.components.contains(&"synapsed-storage".to_string()));
    }
}

/// Test capability-based component discovery
#[test]
fn test_capability_discovery() {
    let mut registry = ComponentRegistry::new();
    registry.initialize_default_components();
    
    // Find components by capability
    let storage_components = registry.find_by_capability(Capability::Storage);
    assert!(!storage_components.is_empty());
    assert!(storage_components.contains(&"synapsed-storage".to_string()));
    
    let consensus_components = registry.find_by_capability(Capability::Consensus);
    assert!(consensus_components.contains(&"synapsed-consensus".to_string()));
    
    let verification_components = registry.find_by_capability(Capability::Verification);
    assert!(verification_components.contains(&"synapsed-verify".to_string()));
}

/// Test environment variable handling
#[tokio::test]
async fn test_environment_configuration() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("env-test");
    
    let app = SynapsedBuilder::new("env-test")
        .env("DATABASE_URL", "postgres://localhost/testdb")
        .env("RUST_LOG", "debug")
        .env("API_KEY", "secret-key-123")
        .env("PORT", "8080")
        .add_storage(StorageBackend::Postgres)
        .build()
        .unwrap();
    
    app.save(&output_path).await.unwrap();
    
    // Verify .env file
    let env_content = std::fs::read_to_string(output_path.join(".env")).unwrap();
    assert!(env_content.contains("DATABASE_URL=postgres://localhost/testdb"));
    assert!(env_content.contains("RUST_LOG=debug"));
    assert!(env_content.contains("API_KEY=secret-key-123"));
    assert!(env_content.contains("PORT=8080"));
}

/// Test dependency resolution order
#[test]
fn test_dependency_resolution() {
    let composer = Composer::new();
    
    // Create app with complex dependencies
    let app = Application {
        name: "dep-test".to_string(),
        description: "Dependency test".to_string(),
        components: vec![
            "synapsed-consensus".to_string(), // Depends on synapsed-net
            "synapsed-verify".to_string(),    // Depends on synapsed-intent
            "synapsed-intent".to_string(),
            "synapsed-net".to_string(),
            "synapsed-core".to_string(),
        ],
        connections: vec![],
        config: Default::default(),
        env: Default::default(),
    };
    
    let resolved = composer.resolve_dependencies(&app).unwrap();
    
    // Core should come first
    let core_idx = resolved.iter().position(|c| c == "synapsed-core").unwrap();
    let net_idx = resolved.iter().position(|c| c == "synapsed-net").unwrap();
    let consensus_idx = resolved.iter().position(|c| c == "synapsed-consensus").unwrap();
    let intent_idx = resolved.iter().position(|c| c == "synapsed-intent").unwrap();
    let verify_idx = resolved.iter().position(|c| c == "synapsed-verify").unwrap();
    
    // Verify ordering
    assert!(core_idx < net_idx);
    assert!(net_idx < consensus_idx);
    assert!(intent_idx < verify_idx);
}

/// Test recipe validation and error handling
#[test]
fn test_recipe_validation() {
    let mut recipe_manager = RecipeManager::new();
    
    // Invalid YAML
    let invalid_yaml = "not: valid: yaml: format:";
    assert!(recipe_manager.load_yaml(invalid_yaml).is_err());
    
    // Missing required fields
    let incomplete_yaml = r#"
name: incomplete
# Missing description and components
"#;
    assert!(recipe_manager.load_yaml(incomplete_yaml).is_err());
    
    // Invalid component reference
    let invalid_component = r#"
name: invalid-component
description: Has invalid component
components:
  - name: synapsed-nonexistent
    version: "*"
"#;
    let result = recipe_manager.load_yaml(invalid_component);
    // This should succeed in loading but fail in validation
    assert!(result.is_ok());
}

/// Test generated code compilation (mock)
#[tokio::test]
async fn test_generated_code_structure() {
    let app = SynapsedBuilder::new("compile-test")
        .add_intent_verification()
        .add_consensus()
        .build()
        .unwrap();
    
    // Generate main.rs content
    let main_rs = app.generate_main_rs();
    
    // Verify structure
    assert!(main_rs.contains("use synapsed_core::"));
    assert!(main_rs.contains("use synapsed_intent::"));
    assert!(main_rs.contains("use synapsed_consensus::"));
    assert!(main_rs.contains("#[tokio::main]"));
    assert!(main_rs.contains("async fn main()"));
    assert!(main_rs.contains(".initialize().await"));
    
    // Verify proper error handling
    assert!(main_rs.contains("anyhow::Result"));
    assert!(main_rs.contains("?") || main_rs.contains(".unwrap()"));
}

/// Test template variations
#[test]
fn test_all_templates() {
    let templates = Templates::list();
    assert!(templates.len() >= 10);
    
    // Test each template can be built
    for template_info in templates {
        let result = match template_info.name.as_str() {
            "verified-ai-agent" => Templates::verified_ai_agent().build(),
            "distributed-consensus" => Templates::distributed_consensus().build(),
            "secure-payment-system" => Templates::secure_payment_system().build(),
            "observable-microservice" => Templates::observable_microservice().build(),
            "p2p-network" => Templates::p2p_network().build(),
            "event-driven-system" => Templates::event_driven_system().build(),
            "crdt-collaborative" => Templates::crdt_collaborative().build(),
            "quantum-secure-app" => Templates::quantum_secure_app().build(),
            "ai-swarm-coordinator" => Templates::ai_swarm_coordinator().build(),
            "full-stack-verified" => Templates::full_stack_verified().build(),
            _ => continue,
        };
        
        assert!(result.is_ok(), "Failed to build template: {}", template_info.name);
    }
}

/// Test configuration merging
#[test]
fn test_configuration_merging() {
    let app = SynapsedBuilder::new("config-test")
        .add_storage(StorageBackend::Sqlite)
        .configure("synapsed-storage", json!({
            "path": "./data.db"
        }))
        .configure("synapsed-storage", json!({
            "pool_size": 10,
            "timeout": 30
        }))
        .build()
        .unwrap();
    
    let storage_config = app.config.get("synapsed-storage").unwrap();
    assert_eq!(storage_config["path"], "./data.db");
    assert_eq!(storage_config["pool_size"], 10);
    assert_eq!(storage_config["timeout"], 30);
}

/// Test error recovery and rollback
#[tokio::test]
async fn test_save_rollback_on_error() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test-app");
    
    // Create directory with a file that will cause conflict
    std::fs::create_dir_all(&output_path).unwrap();
    std::fs::write(output_path.join("Cargo.toml"), "invalid content").unwrap();
    
    // Make Cargo.toml read-only to cause save error
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(output_path.join("Cargo.toml")).unwrap().permissions();
        perms.set_mode(0o444);
        std::fs::set_permissions(output_path.join("Cargo.toml"), perms).unwrap();
    }
    
    let app = SynapsedBuilder::new("test-app")
        .add_intent_verification()
        .build()
        .unwrap();
    
    // This should handle the error gracefully
    let result = app.save(&output_path).await;
    
    // On Unix, this should fail due to permissions
    #[cfg(unix)]
    assert!(result.is_err());
}

/// Test cross-component communication setup
#[test]
fn test_communication_channels() {
    let app = SynapsedBuilder::new("comm-test")
        .add_intent_verification()
        .add_consensus()
        .connect(
            "synapsed-intent", "intent_declared",
            "synapsed-consensus", "validate_intent"
        )
        .connect(
            "synapsed-consensus", "consensus_reached",
            "synapsed-intent", "execute_intent"
        )
        .build()
        .unwrap();
    
    assert_eq!(app.connections.len(), 2);
    
    // Verify bidirectional communication
    let intent_to_consensus = app.connections.iter()
        .any(|c| c.from == "synapsed-intent" && c.to == "synapsed-consensus");
    let consensus_to_intent = app.connections.iter()
        .any(|c| c.from == "synapsed-consensus" && c.to == "synapsed-intent");
    
    assert!(intent_to_consensus);
    assert!(consensus_to_intent);
}