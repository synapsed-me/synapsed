//! Unit tests for Composer
//!
//! Intent: Test composition engine and dependency resolution
//! Verification: Compositions are created correctly

use crate::composer::{Composer, CompositionResult, DependencyGraph, DependencyType, Constraints};
use crate::registry::{ComponentRegistry, Component, Capability, ComponentCategory, ResourceRequirements};
use crate::recipe::{Connection, ConnectionPoint, ConnectionProperties};
use std::collections::{HashMap, HashSet};
use serde_json::json;

#[test]
fn test_compose_simple() {
    let registry = create_test_registry();
    let composer = Composer::new(registry);
    
    let components = vec!["test-core".to_string()];
    let connections = vec![];
    let configurations = HashMap::new();
    
    let result = composer.compose(components, connections, configurations);
    assert!(result.is_ok());
    
    let composition = result.unwrap();
    assert_eq!(composition.components.len(), 1);
}

#[test]
fn test_dependency_graph_construction() {
    let registry = create_test_registry_with_dependencies();
    let composer = Composer::new(registry);
    
    // Components with A -> B -> C dependency chain
    let components = vec![
        "comp-a".to_string(),
        "comp-b".to_string(),
        "comp-c".to_string(),
    ];
    
    let result = composer.compose(components.clone(), vec![], HashMap::new());
    assert!(result.is_ok());
    
    let composition = result.unwrap();
    
    // Get initialization order
    let init_order = composition.dependency_graph.initialization_order();
    assert!(init_order.is_ok());
    
    let order = init_order.unwrap();
    // C should come before B, B before A (reverse dependency order)
    let c_index = order.iter().position(|x| x == "comp-c").unwrap();
    let b_index = order.iter().position(|x| x == "comp-b").unwrap();
    let a_index = order.iter().position(|x| x == "comp-a").unwrap();
    
    assert!(c_index < b_index);
    assert!(b_index < a_index);
}

#[test]
fn test_circular_dependency_detection() {
    let mut registry = ComponentRegistry::new();
    
    // Create circular dependency: A -> B -> A
    let comp_a = Component {
        name: "comp-a".to_string(),
        version: "0.1.0".to_string(),
        description: "Component A".to_string(),
        category: ComponentCategory::Core,
        provides: HashSet::new(),
        requires: HashSet::new(),
        dependencies: vec!["comp-b".to_string()],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    };
    
    let comp_b = Component {
        name: "comp-b".to_string(),
        version: "0.1.0".to_string(),
        description: "Component B".to_string(),
        category: ComponentCategory::Core,
        provides: HashSet::new(),
        requires: HashSet::new(),
        dependencies: vec!["comp-a".to_string()], // Circular!
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    };
    
    registry.register(comp_a).unwrap();
    registry.register(comp_b).unwrap();
    
    let composer = Composer::new(registry);
    let result = composer.compose(
        vec!["comp-a".to_string(), "comp-b".to_string()],
        vec![],
        HashMap::new()
    );
    
    // Should fail due to circular dependency
    assert!(result.is_err());
}

#[test]
fn test_connection_dependencies() {
    let registry = create_test_registry();
    let composer = Composer::new(registry);
    
    let components = vec![
        "test-core".to_string(),
        "test-storage".to_string(),
    ];
    
    let connections = vec![
        Connection {
            from: ConnectionPoint {
                component: "test-core".to_string(),
                port: "output".to_string(),
            },
            to: ConnectionPoint {
                component: "test-storage".to_string(),
                port: "input".to_string(),
            },
            transform: None,
            properties: ConnectionProperties::default(),
        }
    ];
    
    let result = composer.compose(components, connections, HashMap::new());
    assert!(result.is_ok());
    
    // Connection should create a data flow dependency
    let composition = result.unwrap();
    let deps = composition.dependency_graph.get_dependencies("test-core");
    assert!(deps.contains(&"test-storage".to_string()));
}

#[test]
fn test_manifest_generation() {
    let registry = create_test_registry();
    let composer = Composer::new(registry);
    
    let components = vec!["test-core".to_string()];
    let result = composer.compose(components, vec![], HashMap::new());
    assert!(result.is_ok());
    
    let composition = result.unwrap();
    assert_eq!(composition.manifest.package_name, "synapsed-app");
    assert_eq!(composition.manifest.version, "0.1.0");
    assert!(composition.manifest.dependencies.contains_key("test-core"));
}

#[test]
fn test_config_generation() {
    let registry = create_test_registry();
    let composer = Composer::new(registry);
    
    let components = vec!["test-core".to_string()];
    let mut configurations = HashMap::new();
    configurations.insert("test-core".to_string(), json!({
        "option": "value",
        "number": 42
    }));
    
    let result = composer.compose(components, vec![], configurations);
    assert!(result.is_ok());
    
    let composition = result.unwrap();
    assert!(composition.config_files.contains_key("test-core.json"));
    assert!(composition.config_files.contains_key("app.json"));
}

#[test]
fn test_optimize_selection() {
    let mut registry = ComponentRegistry::new();
    
    // Register components with different capabilities
    registry.register(Component {
        name: "storage-heavy".to_string(),
        version: "0.1.0".to_string(),
        description: "Heavy storage".to_string(),
        category: ComponentCategory::Storage,
        provides: hashset![Capability::Storage],
        requires: HashSet::new(),
        dependencies: vec!["dep1".to_string(), "dep2".to_string(), "dep3".to_string()],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements {
            min_memory_mb: Some(2000),
            ..Default::default()
        },
    }).unwrap();
    
    registry.register(Component {
        name: "storage-light".to_string(),
        version: "0.1.0".to_string(),
        description: "Light storage".to_string(),
        category: ComponentCategory::Storage,
        provides: hashset![Capability::Storage],
        requires: HashSet::new(),
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements {
            min_memory_mb: Some(100),
            ..Default::default()
        },
    }).unwrap();
    
    // Add dummy dependencies
    registry.register(create_simple_component("dep1")).unwrap();
    registry.register(create_simple_component("dep2")).unwrap();
    registry.register(create_simple_component("dep3")).unwrap();
    
    let composer = Composer::new(registry);
    
    // Request storage capability
    let requirements = vec!["storage".to_string()];
    let constraints = Constraints {
        max_memory_mb: Some(500),
        prefer_minimal: true,
        ..Default::default()
    };
    
    let selected = composer.optimize_selection(&requirements, &constraints);
    assert!(selected.is_ok());
    
    let components = selected.unwrap();
    // Should select the light storage due to memory constraint
    assert!(components.contains(&"storage-light".to_string()));
    assert!(!components.contains(&"storage-heavy".to_string()));
}

#[test]
fn test_wildcard_connections() {
    let registry = create_test_registry();
    let composer = Composer::new(registry);
    
    let components = vec![
        "test-core".to_string(),
        "test-storage".to_string(),
    ];
    
    // Wildcard connection (all to observability)
    let connections = vec![
        Connection {
            from: ConnectionPoint {
                component: "*".to_string(),
                port: "*".to_string(),
            },
            to: ConnectionPoint {
                component: "observability".to_string(),
                port: "events".to_string(),
            },
            transform: None,
            properties: ConnectionProperties::default(),
        }
    ];
    
    // Should not create dependencies for wildcards
    let result = composer.compose(components, connections, HashMap::new());
    assert!(result.is_ok());
}

// Helper functions

fn create_test_registry() -> ComponentRegistry {
    let mut registry = ComponentRegistry::new();
    
    registry.register(Component {
        name: "test-core".to_string(),
        version: "0.1.0".to_string(),
        description: "Test core".to_string(),
        category: ComponentCategory::Core,
        provides: HashSet::new(),
        requires: HashSet::new(),
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    }).unwrap();
    
    registry.register(Component {
        name: "test-storage".to_string(),
        version: "0.1.0".to_string(),
        description: "Test storage".to_string(),
        category: ComponentCategory::Storage,
        provides: hashset![Capability::Storage],
        requires: HashSet::new(),
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    }).unwrap();
    
    registry
}

fn create_test_registry_with_dependencies() -> ComponentRegistry {
    let mut registry = ComponentRegistry::new();
    
    // Create dependency chain: A -> B -> C
    registry.register(Component {
        name: "comp-c".to_string(),
        version: "0.1.0".to_string(),
        description: "Component C".to_string(),
        category: ComponentCategory::Core,
        provides: HashSet::new(),
        requires: HashSet::new(),
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    }).unwrap();
    
    registry.register(Component {
        name: "comp-b".to_string(),
        version: "0.1.0".to_string(),
        description: "Component B".to_string(),
        category: ComponentCategory::Core,
        provides: HashSet::new(),
        requires: HashSet::new(),
        dependencies: vec!["comp-c".to_string()],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    }).unwrap();
    
    registry.register(Component {
        name: "comp-a".to_string(),
        version: "0.1.0".to_string(),
        description: "Component A".to_string(),
        category: ComponentCategory::Core,
        provides: HashSet::new(),
        requires: HashSet::new(),
        dependencies: vec!["comp-b".to_string()],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    }).unwrap();
    
    registry
}

fn create_simple_component(name: &str) -> Component {
    Component {
        name: name.to_string(),
        version: "0.1.0".to_string(),
        description: format!("Component {}", name),
        category: ComponentCategory::Core,
        provides: HashSet::new(),
        requires: HashSet::new(),
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    }
}

// Helper macro
macro_rules! hashset {
    ($($val:expr),*) => {
        {
            let mut set = HashSet::new();
            $(set.insert($val);)*
            set
        }
    };
}

use hashset;