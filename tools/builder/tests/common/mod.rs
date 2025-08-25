//! Common test utilities and fixtures for builder tests

use synapsed_builder::prelude::*;
use synapsed_builder::{
    builder::{SynapsedBuilder, StorageBackend, ObservabilityLevel, NetworkType},
    registry::{Component, ComponentRegistry, Capability},
    recipe::Recipe,
};
use std::collections::HashMap;
use serde_json::json;

/// Create a test component with default values
pub fn create_test_component(name: &str) -> Component {
    Component {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![Capability::Core],
        description: format!("Test component: {}", name),
        dependencies: vec![],
    }
}

/// Create a component with specific capabilities
pub fn create_component_with_capabilities(
    name: &str,
    capabilities: Vec<Capability>
) -> Component {
    Component {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        capabilities,
        description: format!("Component with capabilities: {}", name),
        dependencies: vec![],
    }
}

/// Create a component with dependencies
pub fn create_component_with_deps(
    name: &str,
    deps: Vec<String>
) -> Component {
    Component {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![Capability::Core],
        description: format!("Component with dependencies: {}", name),
        dependencies: deps,
    }
}

/// Create a minimal test application
pub fn create_minimal_app() -> Application {
    Application {
        name: "minimal-app".to_string(),
        description: "Minimal test application".to_string(),
        components: vec!["synapsed-core".to_string()],
        connections: vec![],
        config: HashMap::new(),
        env: HashMap::new(),
    }
}

/// Create a standard test application
pub fn create_standard_app() -> Application {
    SynapsedBuilder::new("standard-app")
        .description("Standard test application")
        .add_intent_verification()
        .add_storage(StorageBackend::Sqlite)
        .build()
        .unwrap()
}

/// Create a complex test application
pub fn create_complex_app() -> Application {
    SynapsedBuilder::new("complex-app")
        .description("Complex test application")
        .add_intent_verification()
        .add_consensus()
        .add_storage(StorageBackend::Postgres)
        .add_observability(ObservabilityLevel::Full)
        .add_payments()
        .connect(
            "synapsed-payments", "transaction",
            "synapsed-consensus", "propose"
        )
        .env("RUST_LOG", "debug")
        .configure("synapsed-consensus", json!({
            "committee_size": 5
        }))
        .build()
        .unwrap()
}

/// Create a test recipe
pub fn create_test_recipe(name: &str) -> Recipe {
    Recipe {
        name: name.to_string(),
        description: format!("Test recipe: {}", name),
        version: "1.0.0".to_string(),
        components: vec![
            "synapsed-core".to_string(),
            "synapsed-intent".to_string(),
        ],
        connections: vec![],
        config: HashMap::new(),
    }
}

/// Create a recipe with connections
pub fn create_recipe_with_connections() -> Recipe {
    Recipe {
        name: "connected-recipe".to_string(),
        description: "Recipe with connections".to_string(),
        version: "1.0.0".to_string(),
        components: vec![
            "synapsed-intent".to_string(),
            "synapsed-verify".to_string(),
        ],
        connections: vec![
            Connection {
                from: "synapsed-intent".to_string(),
                event: "intent_declared".to_string(),
                to: "synapsed-verify".to_string(),
                handler: "verify_intent".to_string(),
            }
        ],
        config: HashMap::new(),
    }
}

/// Create a test connection
pub fn create_test_connection(from: &str, to: &str) -> Connection {
    Connection {
        from: from.to_string(),
        event: "test_event".to_string(),
        to: to.to_string(),
        handler: "test_handler".to_string(),
    }
}

/// Create a configured recipe
pub fn create_configured_recipe() -> Recipe {
    let mut config = HashMap::new();
    config.insert(
        "synapsed-storage".to_string(),
        json!({
            "backend": "sqlite",
            "path": "./test.db"
        })
    );
    
    Recipe {
        name: "configured-recipe".to_string(),
        description: "Recipe with configuration".to_string(),
        version: "1.0.0".to_string(),
        components: vec![
            "synapsed-core".to_string(),
            "synapsed-storage".to_string(),
        ],
        connections: vec![],
        config,
    }
}

/// Builder for test applications with fluent API
pub struct TestAppBuilder {
    name: String,
    components: Vec<String>,
    connections: Vec<Connection>,
    config: HashMap<String, serde_json::Value>,
    env: HashMap<String, String>,
}

impl TestAppBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            components: vec!["synapsed-core".to_string()],
            connections: vec![],
            config: HashMap::new(),
            env: HashMap::new(),
        }
    }
    
    pub fn with_component(mut self, component: &str) -> Self {
        self.components.push(component.to_string());
        self
    }
    
    pub fn with_connection(mut self, conn: Connection) -> Self {
        self.connections.push(conn);
        self
    }
    
    pub fn with_config(mut self, component: &str, config: serde_json::Value) -> Self {
        self.config.insert(component.to_string(), config);
        self
    }
    
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }
    
    pub fn build(self) -> Application {
        Application {
            name: self.name,
            description: format!("Test app: {}", self.name),
            components: self.components,
            connections: self.connections,
            config: self.config,
            env: self.env,
        }
    }
}

/// Test data generators
pub mod generators {
    use super::*;
    use rand::{Rng, distributions::Alphanumeric};
    
    /// Generate a random component name
    pub fn random_component_name() -> String {
        let suffix: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        format!("synapsed-{}", suffix.to_lowercase())
    }
    
    /// Generate multiple unique component names
    pub fn random_component_names(count: usize) -> Vec<String> {
        let mut names = std::collections::HashSet::new();
        while names.len() < count {
            names.insert(random_component_name());
        }
        names.into_iter().collect()
    }
    
    /// Generate a random capability
    pub fn random_capability() -> Capability {
        let capabilities = vec![
            Capability::Core,
            Capability::Intent,
            Capability::Verification,
            Capability::Consensus,
            Capability::Networking,
            Capability::Storage,
            Capability::Observability,
            Capability::Crypto,
            Capability::Payments,
        ];
        
        let mut rng = rand::thread_rng();
        capabilities[rng.gen_range(0..capabilities.len())].clone()
    }
    
    /// Generate random capabilities
    pub fn random_capabilities(max_count: usize) -> Vec<Capability> {
        let mut rng = rand::thread_rng();
        let count = rng.gen_range(1..=max_count.min(9));
        
        let mut capabilities = std::collections::HashSet::new();
        while capabilities.len() < count {
            capabilities.insert(random_capability());
        }
        
        capabilities.into_iter().collect()
    }
    
    /// Generate a random connection
    pub fn random_connection(components: &[String]) -> Connection {
        if components.len() < 2 {
            panic!("Need at least 2 components for a connection");
        }
        
        let mut rng = rand::thread_rng();
        let from_idx = rng.gen_range(0..components.len());
        let mut to_idx = rng.gen_range(0..components.len());
        while to_idx == from_idx {
            to_idx = rng.gen_range(0..components.len());
        }
        
        Connection {
            from: components[from_idx].clone(),
            event: format!("event_{}", rng.gen_range(1..100)),
            to: components[to_idx].clone(),
            handler: format!("handler_{}", rng.gen_range(1..100)),
        }
    }
    
    /// Generate random configuration
    pub fn random_config() -> serde_json::Value {
        let mut rng = rand::thread_rng();
        json!({
            "param1": rng.gen_range(1..100),
            "param2": random_component_name(),
            "param3": rng.gen::<bool>(),
            "param4": {
                "nested": rng.gen_range(1..10),
                "value": "test"
            }
        })
    }
}

/// Assertion helpers
pub mod assertions {
    use super::*;
    
    /// Assert that an application contains all expected components
    pub fn assert_has_components(app: &Application, expected: &[&str]) {
        for component in expected {
            assert!(
                app.components.contains(&component.to_string()),
                "Application {} should contain component {}, but has: {:?}",
                app.name, component, app.components
            );
        }
    }
    
    /// Assert that an application has exactly the expected components
    pub fn assert_components_exact(app: &Application, expected: &[&str]) {
        let expected_set: std::collections::HashSet<String> = 
            expected.iter().map(|s| s.to_string()).collect();
        let actual_set: std::collections::HashSet<String> = 
            app.components.iter().cloned().collect();
        
        assert_eq!(
            expected_set, actual_set,
            "Application {} components mismatch. Expected: {:?}, Got: {:?}",
            app.name, expected_set, actual_set
        );
    }
    
    /// Assert that connections are valid (from and to components exist)
    pub fn assert_connections_valid(app: &Application) {
        for conn in &app.connections {
            assert!(
                app.components.contains(&conn.from),
                "Connection from '{}' references non-existent component",
                conn.from
            );
            assert!(
                app.components.contains(&conn.to),
                "Connection to '{}' references non-existent component",
                conn.to
            );
        }
    }
    
    /// Assert that a recipe is valid
    pub fn assert_recipe_valid(recipe: &Recipe) {
        assert!(!recipe.name.is_empty(), "Recipe name cannot be empty");
        assert!(!recipe.components.is_empty(), "Recipe must have at least one component");
        assert!(!recipe.version.is_empty(), "Recipe version cannot be empty");
        
        // Check connections reference valid components
        for conn in &recipe.connections {
            assert!(
                recipe.components.contains(&conn.from),
                "Recipe connection from '{}' not in components",
                conn.from
            );
            assert!(
                recipe.components.contains(&conn.to),
                "Recipe connection to '{}' not in components",
                conn.to
            );
        }
    }
}

/// Mock implementations for testing
pub mod mocks {
    use super::*;
    
    /// Mock component registry
    pub struct MockRegistry {
        components: HashMap<String, Component>,
    }
    
    impl MockRegistry {
        pub fn new() -> Self {
            Self {
                components: HashMap::new(),
            }
        }
        
        pub fn with_defaults() -> Self {
            let mut registry = Self::new();
            
            // Add common test components
            registry.add(create_test_component("synapsed-core"));
            registry.add(create_component_with_capabilities(
                "synapsed-intent",
                vec![Capability::Intent, Capability::Verification]
            ));
            registry.add(create_component_with_capabilities(
                "synapsed-storage",
                vec![Capability::Storage]
            ));
            
            registry
        }
        
        pub fn add(&mut self, component: Component) {
            self.components.insert(component.name.clone(), component);
        }
        
        pub fn get(&self, name: &str) -> Option<&Component> {
            self.components.get(name)
        }
        
        pub fn find_by_capability(&self, capability: Capability) -> Vec<String> {
            self.components
                .values()
                .filter(|c| c.capabilities.contains(&capability))
                .map(|c| c.name.clone())
                .collect()
        }
    }
    
    /// Mock validator that always succeeds
    pub struct AlwaysValidValidator;
    
    impl AlwaysValidValidator {
        pub fn validate(&self, _app: &Application) -> Result<(), String> {
            Ok(())
        }
    }
    
    /// Mock validator that always fails
    pub struct AlwaysInvalidValidator;
    
    impl AlwaysInvalidValidator {
        pub fn validate(&self, _app: &Application) -> Result<(), String> {
            Err("Validation always fails in test".to_string())
        }
    }
    
    /// Mock composer
    pub struct MockComposer {
        should_fail: bool,
    }
    
    impl MockComposer {
        pub fn new() -> Self {
            Self { should_fail: false }
        }
        
        pub fn that_fails() -> Self {
            Self { should_fail: true }
        }
        
        pub fn compose(&self, recipe: &Recipe) -> Result<Application, String> {
            if self.should_fail {
                Err("Composition failed in test".to_string())
            } else {
                Ok(Application {
                    name: recipe.name.clone(),
                    description: recipe.description.clone(),
                    components: recipe.components.clone(),
                    connections: recipe.connections.clone(),
                    config: recipe.config.clone(),
                    env: HashMap::new(),
                })
            }
        }
    }
}

/// Test scenarios for end-to-end testing
pub mod scenarios {
    use super::*;
    
    /// Scenario: Build a minimal viable application
    pub fn minimal_app_scenario() -> Application {
        SynapsedBuilder::new("minimal-scenario")
            .description("Minimal viable application")
            .build()
            .expect("Should build minimal app")
    }
    
    /// Scenario: Build an AI agent application
    pub fn ai_agent_scenario() -> Application {
        SynapsedBuilder::new("ai-agent-scenario")
            .description("AI agent with verification")
            .add_intent_verification()
            .add_observability(ObservabilityLevel::Standard)
            .configure("synapsed-intent", json!({
                "max_depth": 10,
                "planning_enabled": true
            }))
            .env("AI_MODEL", "gpt-4")
            .build()
            .expect("Should build AI agent app")
    }
    
    /// Scenario: Build a distributed system
    pub fn distributed_system_scenario() -> Application {
        SynapsedBuilder::new("distributed-scenario")
            .description("Distributed system with consensus")
            .add_consensus()
            .add_network(NetworkType::P2P)
            .add_storage(StorageBackend::Postgres)
            .connect(
                "synapsed-net", "peer_joined",
                "synapsed-consensus", "update_committee"
            )
            .configure("synapsed-consensus", json!({
                "consensus_type": "raft",
                "election_timeout_ms": 150
            }))
            .build()
            .expect("Should build distributed system")
    }
    
    /// Scenario: Build a payment processing system
    pub fn payment_system_scenario() -> Application {
        SynapsedBuilder::new("payment-scenario")
            .description("Payment processing system")
            .add_payments()
            .add_storage(StorageBackend::Postgres)
            .add_observability(ObservabilityLevel::Full)
            .configure("synapsed-payments", json!({
                "currencies": ["USD", "EUR", "GBP"],
                "fraud_detection": true,
                "pci_compliance": true
            }))
            .env("PAYMENT_ENV", "sandbox")
            .env("STRIPE_KEY", "sk_test_...")
            .build()
            .expect("Should build payment system")
    }
}