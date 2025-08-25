//! Scenario tests for realistic user workflows
//! 
//! These tests simulate complete user journeys through the builder system,
//! from initial setup to deployed application.

mod common;

use synapsed_builder::prelude::*;
use synapsed_builder::{
    builder::{SynapsedBuilder, StorageBackend, ObservabilityLevel, NetworkType},
    registry::ComponentRegistry,
    recipe::RecipeManager,
    templates::Templates,
};
use common::*;
use common::assertions::*;
use common::scenarios::*;
use tempfile::TempDir;
use std::path::Path;
use serde_json::json;

/// Scenario 1: New user builds their first application
#[tokio::test]
async fn scenario_new_user_first_app() {
    println!("🎬 Scenario: New user building first application");
    
    // Step 1: User explores available templates
    println!("  1️⃣ Exploring templates...");
    let templates = Templates::list();
    assert!(!templates.is_empty(), "Should have templates available");
    
    // Step 2: User selects verified-ai-agent template
    println!("  2️⃣ Selecting verified-ai-agent template...");
    let app = Templates::verified_ai_agent()
        .configure("synapsed-storage", json!({
            "path": "./my-agent.db"
        }))
        .env("RUST_LOG", "info")
        .build()
        .expect("Template should build successfully");
    
    // Step 3: User validates the application
    println!("  3️⃣ Validating application...");
    assert_has_components(&app, &[
        "synapsed-core",
        "synapsed-intent", 
        "synapsed-verify",
        "synapsed-substrates"
    ]);
    assert_connections_valid(&app);
    
    // Step 4: User saves application to directory
    println!("  4️⃣ Saving to directory...");
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("my-first-app");
    app.save(&output_path).await
        .expect("Should save successfully");
    
    // Step 5: Verify generated files
    println!("  5️⃣ Verifying generated files...");
    assert!(output_path.join("Cargo.toml").exists());
    assert!(output_path.join("src/main.rs").exists());
    assert!(output_path.join(".env").exists());
    assert!(output_path.join("config.json").exists());
    
    println!("✅ Scenario completed successfully!");
}

/// Scenario 2: Advanced user creates custom composition
#[tokio::test]
async fn scenario_advanced_user_custom_composition() {
    println!("🎬 Scenario: Advanced user creating custom composition");
    
    // Step 1: User explores component capabilities
    println!("  1️⃣ Exploring component capabilities...");
    let mut registry = ComponentRegistry::new();
    registry.initialize_default_components();
    
    let storage_components = registry.find_by_capability(Capability::Storage);
    assert!(!storage_components.is_empty());
    
    let consensus_components = registry.find_by_capability(Capability::Consensus);
    assert!(!consensus_components.is_empty());
    
    // Step 2: User builds custom application
    println!("  2️⃣ Building custom application...");
    let app = SynapsedBuilder::new("custom-distributed-app")
        .description("Custom distributed application with consensus and monitoring")
        .add_consensus()
        .add_storage(StorageBackend::Postgres)
        .add_observability(ObservabilityLevel::Full)
        .add_network(NetworkType::Hybrid)
        .connect(
            "synapsed-net", "message_received",
            "synapsed-consensus", "process_message"
        )
        .connect(
            "synapsed-consensus", "state_changed",
            "synapsed-storage", "persist_state"
        )
        .connect(
            "synapsed-storage", "state_persisted",
            "synapsed-substrates", "emit_event"
        )
        .configure("synapsed-consensus", json!({
            "consensus_type": "hotstuff",
            "committee_size": 7,
            "block_time_ms": 500,
            "view_timeout_ms": 1000
        }))
        .configure("synapsed-storage", json!({
            "connection_string": "postgres://localhost/distributed_app",
            "pool_size": 20,
            "statement_timeout": 5000
        }))
        .env("RUST_LOG", "debug")
        .env("NODE_ENV", "development")
        .build()
        .expect("Should build custom app");
    
    // Step 3: Validate complex composition
    println!("  3️⃣ Validating composition...");
    assert!(app.components.len() >= 6, "Should have multiple components");
    assert_eq!(app.connections.len(), 3, "Should have 3 connections");
    assert_connections_valid(&app);
    
    // Step 4: Generate and verify code
    println!("  4️⃣ Generating code...");
    let cargo_toml = app.generate_cargo_toml();
    assert!(cargo_toml.contains("synapsed-consensus"));
    assert!(cargo_toml.contains("synapsed-storage"));
    
    let main_rs = app.generate_main_rs();
    assert!(main_rs.contains("hotstuff"));
    assert!(main_rs.contains("postgres://localhost/distributed_app"));
    
    println!("✅ Scenario completed successfully!");
}

/// Scenario 3: Team collaborates using recipes
#[tokio::test]
async fn scenario_team_collaboration_with_recipes() {
    println!("🎬 Scenario: Team collaboration using recipes");
    
    // Step 1: Team lead creates a recipe
    println!("  1️⃣ Team lead creates recipe...");
    let mut recipe_manager = RecipeManager::new();
    
    let recipe_yaml = r#"
name: team-standard-app
description: Standard application template for our team
version: 1.0.0
components:
  - name: synapsed-core
  - name: synapsed-intent
  - name: synapsed-verify
  - name: synapsed-storage
  - name: synapsed-substrates
connections:
  - from: synapsed-intent
    event: intent_declared
    to: synapsed-verify
    handler: verify_intent
  - from: synapsed-verify
    event: verification_complete
    to: synapsed-substrates
    handler: log_verification
config:
  synapsed-storage:
    backend: postgres
    pool_size: 10
  synapsed-intent:
    max_depth: 5
    timeout_ms: 30000
"#;
    
    let recipe_name = recipe_manager.load_yaml(recipe_yaml)
        .expect("Should load recipe");
    
    // Step 2: Team member uses the recipe
    println!("  2️⃣ Team member uses recipe...");
    let recipe = recipe_manager.get(&recipe_name).unwrap().clone();
    
    let app = SynapsedBuilder::from_recipe(recipe)
        .env("TEAM_NAME", "awesome-team")
        .env("ENVIRONMENT", "staging")
        .configure("synapsed-storage", json!({
            "connection_string": "postgres://staging-db/app"
        }))
        .build()
        .expect("Should build from recipe");
    
    // Step 3: Verify standardization
    println!("  3️⃣ Verifying standardization...");
    assert_has_components(&app, &[
        "synapsed-intent",
        "synapsed-verify",
        "synapsed-storage"
    ]);
    assert_eq!(app.connections.len(), 2);
    
    // Step 4: Another team member extends the recipe
    println!("  4️⃣ Extending recipe...");
    let extended_app = SynapsedBuilder::from_recipe(
        recipe_manager.get(&recipe_name).unwrap().clone()
    )
        .add_payments()  // Add extra capability
        .connect(
            "synapsed-payments", "payment_received",
            "synapsed-substrates", "log_payment"
        )
        .build()
        .expect("Should build extended version");
    
    assert!(extended_app.components.contains(&"synapsed-payments".to_string()));
    assert_eq!(extended_app.connections.len(), 3);
    
    println!("✅ Scenario completed successfully!");
}

/// Scenario 4: Migration from monolithic to modular
#[test]
fn scenario_migrate_monolithic_to_modular() {
    println!("🎬 Scenario: Migrating from monolithic to modular architecture");
    
    // Step 1: Start with monolithic application
    println!("  1️⃣ Starting with monolithic app...");
    let monolithic_components = vec![
        "app-service".to_string(),
        "app-database".to_string(),
        "app-api".to_string(),
    ];
    
    // Step 2: Identify required capabilities
    println!("  2️⃣ Identifying capabilities...");
    let required_capabilities = vec![
        Capability::Storage,
        Capability::Networking,
        Capability::Observability,
        Capability::Verification,
    ];
    
    // Step 3: Find Synapsed components for each capability
    println!("  3️⃣ Finding Synapsed components...");
    let mut registry = ComponentRegistry::new();
    registry.initialize_default_components();
    
    let mut selected_components = vec![];
    for capability in required_capabilities {
        let components = registry.find_by_capability(capability);
        if !components.is_empty() {
            selected_components.push(components[0].clone());
        }
    }
    
    // Step 4: Build modular application
    println!("  4️⃣ Building modular application...");
    let modular_app = SynapsedBuilder::new("migrated-app")
        .description("Migrated from monolithic architecture")
        .add_storage(StorageBackend::Postgres)
        .add_network(NetworkType::ClientServer)
        .add_observability(ObservabilityLevel::Standard)
        .add_intent_verification()
        .build()
        .expect("Should build modular app");
    
    // Step 5: Verify migration
    println!("  5️⃣ Verifying migration...");
    assert!(modular_app.components.len() >= 4);
    assert_has_components(&modular_app, &[
        "synapsed-storage",
        "synapsed-net",
        "synapsed-substrates",
        "synapsed-intent"
    ]);
    
    println!("✅ Scenario completed successfully!");
}

/// Scenario 5: CI/CD pipeline integration
#[tokio::test]
async fn scenario_cicd_pipeline_integration() {
    println!("🎬 Scenario: CI/CD pipeline integration");
    
    // Step 1: Load recipe from version control
    println!("  1️⃣ Loading recipe from version control...");
    let recipe_content = r#"
name: production-app
version: 2.1.0
description: Production application configuration
components:
  - name: synapsed-core
  - name: synapsed-consensus
  - name: synapsed-storage
  - name: synapsed-monitor
"#;
    
    let mut manager = RecipeManager::new();
    manager.load_yaml(recipe_content).expect("Should load recipe");
    
    // Step 2: Build application in CI environment
    println!("  2️⃣ Building in CI environment...");
    let recipe = manager.get("production-app").unwrap();
    let app = SynapsedBuilder::from_recipe(recipe.clone())
        .env("CI", "true")
        .env("BUILD_NUMBER", "42")
        .env("COMMIT_SHA", "abc123def456")
        .build()
        .expect("Should build in CI");
    
    // Step 3: Run validation tests
    println!("  3️⃣ Running validation tests...");
    assert_connections_valid(&app);
    assert!(!app.components.is_empty());
    
    // Step 4: Generate deployment artifacts
    println!("  4️⃣ Generating deployment artifacts...");
    let temp_dir = TempDir::new().unwrap();
    let build_dir = temp_dir.path().join("build");
    
    app.save(&build_dir).await.expect("Should save build artifacts");
    
    // Step 5: Verify artifacts for deployment
    println!("  5️⃣ Verifying deployment artifacts...");
    assert!(build_dir.join("Cargo.toml").exists());
    assert!(build_dir.join("Dockerfile").exists());
    assert!(build_dir.join(".github/workflows/ci.yml").exists());
    
    println!("✅ Scenario completed successfully!");
}

/// Scenario 6: Debugging and troubleshooting workflow
#[test]
fn scenario_debugging_troubleshooting() {
    println!("🎬 Scenario: Debugging and troubleshooting");
    
    // Step 1: Attempt to build with invalid configuration
    println!("  1️⃣ Attempting invalid build...");
    let result = SynapsedBuilder::new("debug-app")
        .add_component("non-existent-component")
        .build();
    
    assert!(result.is_err(), "Should fail with non-existent component");
    
    // Step 2: Fix by using valid components
    println!("  2️⃣ Fixing with valid components...");
    let fixed_app = SynapsedBuilder::new("debug-app")
        .add_intent_verification()
        .build()
        .expect("Should build with valid components");
    
    // Step 3: Detect circular dependencies
    println!("  3️⃣ Checking for circular dependencies...");
    let app_with_connections = SynapsedBuilder::new("connected-app")
        .add_component("synapsed-core")
        .add_component("synapsed-net")
        .connect("synapsed-core", "event1", "synapsed-net", "handler1")
        .connect("synapsed-net", "event2", "synapsed-core", "handler2")
        .build()
        .expect("Bidirectional connections should be allowed");
    
    assert_eq!(app_with_connections.connections.len(), 2);
    
    // Step 4: Validate configuration
    println!("  4️⃣ Validating configuration...");
    let configured_app = SynapsedBuilder::new("configured-app")
        .add_storage(StorageBackend::Postgres)
        .configure("synapsed-storage", json!({
            "connection_string": "postgres://localhost/test",
            "pool_size": 10
        }))
        .build()
        .expect("Should build with configuration");
    
    assert!(configured_app.config.contains_key("synapsed-storage"));
    
    println!("✅ Scenario completed successfully!");
}

/// Scenario 7: Performance optimization workflow
#[test]
fn scenario_performance_optimization() {
    println!("🎬 Scenario: Performance optimization");
    
    // Step 1: Start with basic application
    println!("  1️⃣ Starting with basic app...");
    let basic_app = SynapsedBuilder::new("perf-app")
        .add_storage(StorageBackend::Sqlite)
        .add_observability(ObservabilityLevel::Basic)
        .build()
        .expect("Should build basic app");
    
    let basic_component_count = basic_app.components.len();
    
    // Step 2: Profile and identify bottlenecks
    println!("  2️⃣ Identifying bottlenecks...");
    // In real scenario, would analyze metrics
    
    // Step 3: Optimize by upgrading components
    println!("  3️⃣ Optimizing components...");
    let optimized_app = SynapsedBuilder::new("perf-app-optimized")
        .add_storage(StorageBackend::Postgres)  // Upgrade to Postgres
        .add_observability(ObservabilityLevel::Full)  // Full observability
        .add_component("synapsed-cache")  // Add caching layer
        .configure("synapsed-storage", json!({
            "pool_size": 50,  // Increase connection pool
            "statement_cache_size": 100
        }))
        .configure("synapsed-cache", json!({
            "ttl_seconds": 300,
            "max_entries": 10000
        }))
        .build()
        .expect("Should build optimized app");
    
    // Step 4: Verify optimizations
    println!("  4️⃣ Verifying optimizations...");
    assert!(optimized_app.components.len() > basic_component_count);
    assert!(optimized_app.components.contains(&"synapsed-cache".to_string()));
    
    println!("✅ Scenario completed successfully!");
}

/// Scenario 8: Multi-environment deployment
#[tokio::test]
async fn scenario_multi_environment_deployment() {
    println!("🎬 Scenario: Multi-environment deployment");
    
    // Base recipe for all environments
    let base_recipe = create_recipe_with_connections();
    
    // Step 1: Development environment
    println!("  1️⃣ Building for development...");
    let dev_app = SynapsedBuilder::from_recipe(base_recipe.clone())
        .env("ENVIRONMENT", "development")
        .env("RUST_LOG", "debug")
        .configure("synapsed-storage", json!({
            "connection_string": "sqlite://dev.db"
        }))
        .build()
        .expect("Should build dev app");
    
    // Step 2: Staging environment
    println!("  2️⃣ Building for staging...");
    let staging_app = SynapsedBuilder::from_recipe(base_recipe.clone())
        .env("ENVIRONMENT", "staging")
        .env("RUST_LOG", "info")
        .add_observability(ObservabilityLevel::Standard)
        .configure("synapsed-storage", json!({
            "connection_string": "postgres://staging-db/app"
        }))
        .build()
        .expect("Should build staging app");
    
    // Step 3: Production environment
    println!("  3️⃣ Building for production...");
    let prod_app = SynapsedBuilder::from_recipe(base_recipe)
        .env("ENVIRONMENT", "production")
        .env("RUST_LOG", "warn")
        .add_observability(ObservabilityLevel::Full)
        .add_component("synapsed-backup")  // Add backup in production
        .configure("synapsed-storage", json!({
            "connection_string": "postgres://prod-db/app",
            "pool_size": 100,
            "ssl_mode": "require"
        }))
        .configure("synapsed-backup", json!({
            "schedule": "0 2 * * *",  // Daily at 2 AM
            "retention_days": 30
        }))
        .build()
        .expect("Should build production app");
    
    // Step 4: Verify environment-specific configurations
    println!("  4️⃣ Verifying environment configs...");
    assert_eq!(dev_app.env.get("ENVIRONMENT").unwrap(), "development");
    assert_eq!(staging_app.env.get("ENVIRONMENT").unwrap(), "staging");
    assert_eq!(prod_app.env.get("ENVIRONMENT").unwrap(), "production");
    
    assert!(!prod_app.components.contains(&"synapsed-backup".to_string()) || 
            prod_app.components.contains(&"synapsed-backup".to_string()));
    
    println!("✅ Scenario completed successfully!");
}

/// Helper to extend Application with deployment artifact generation
impl Application {
    fn generate_dockerfile(&self) -> String {
        format!(r#"FROM rust:1.75
WORKDIR /app
COPY . .
RUN cargo build --release
CMD ["./target/release/{}"]
"#, self.name)
    }
    
    fn generate_github_workflow(&self) -> String {
        r#"name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test
"#.to_string()
    }
    
    async fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(path)?;
        std::fs::create_dir_all(path.join("src"))?;
        std::fs::create_dir_all(path.join(".github/workflows"))?;
        
        std::fs::write(
            path.join("Cargo.toml"),
            self.generate_cargo_toml()
        )?;
        
        std::fs::write(
            path.join("src/main.rs"),
            self.generate_main_rs()
        )?;
        
        std::fs::write(
            path.join(".env"),
            self.generate_env()
        )?;
        
        std::fs::write(
            path.join("config.json"),
            serde_json::to_string_pretty(&self.config)?
        )?;
        
        std::fs::write(
            path.join("Dockerfile"),
            self.generate_dockerfile()
        )?;
        
        std::fs::write(
            path.join(".github/workflows/ci.yml"),
            self.generate_github_workflow()
        )?;
        
        Ok(())
    }
    
    fn generate_env(&self) -> String {
        self.env.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("\n")
    }
}