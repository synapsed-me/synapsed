//! Unit tests for ComponentRegistry
//!
//! Intent: Test component registry operations
//! Verification: All registry functions work correctly

use crate::registry::{ComponentRegistry, Component, Capability, ComponentCategory, ResourceRequirements};
use std::collections::HashSet;

#[test]
fn test_register_component() {
    let mut registry = ComponentRegistry::new();
    
    let component = create_test_component("test-component");
    assert!(registry.register(component.clone()).is_ok());
    
    // Should be able to retrieve it
    let retrieved = registry.get("test-component");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "test-component");
}

#[test]
fn test_find_by_capability() {
    let mut registry = ComponentRegistry::new();
    
    // Register components with different capabilities
    let storage_component = Component {
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
    };
    
    let network_component = Component {
        name: "test-network".to_string(),
        version: "0.1.0".to_string(),
        description: "Test network".to_string(),
        category: ComponentCategory::Network,
        provides: hashset![Capability::Networking],
        requires: HashSet::new(),
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    };
    
    registry.register(storage_component).unwrap();
    registry.register(network_component).unwrap();
    
    // Find by capability
    let storage_providers = registry.find_by_capability(&Capability::Storage);
    assert_eq!(storage_providers.len(), 1);
    assert_eq!(storage_providers[0].name, "test-storage");
    
    let network_providers = registry.find_by_capability(&Capability::Networking);
    assert_eq!(network_providers.len(), 1);
    assert_eq!(network_providers[0].name, "test-network");
}

#[test]
fn test_find_by_multiple_capabilities() {
    let mut registry = ComponentRegistry::new();
    
    // Component that provides multiple capabilities
    let multi_component = Component {
        name: "multi-component".to_string(),
        version: "0.1.0".to_string(),
        description: "Multi capability component".to_string(),
        category: ComponentCategory::Core,
        provides: hashset![Capability::Storage, Capability::Networking],
        requires: HashSet::new(),
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    };
    
    registry.register(multi_component).unwrap();
    
    // Should find it when searching for both capabilities
    let found = registry.find_by_capabilities(&[Capability::Storage, Capability::Networking]);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "multi-component");
    
    // Should not find it when searching for capability it doesn't have
    let not_found = registry.find_by_capabilities(&[Capability::Storage, Capability::Cryptography]);
    assert_eq!(not_found.len(), 0);
}

#[test]
fn test_resolve_dependencies() {
    let mut registry = ComponentRegistry::new();
    
    // Create a dependency chain: A -> B -> C
    let component_c = Component {
        name: "component-c".to_string(),
        version: "0.1.0".to_string(),
        description: "Component C".to_string(),
        category: ComponentCategory::Core,
        provides: hashset![Capability::Custom("c".to_string())],
        requires: HashSet::new(),
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    };
    
    let component_b = Component {
        name: "component-b".to_string(),
        version: "0.1.0".to_string(),
        description: "Component B".to_string(),
        category: ComponentCategory::Core,
        provides: hashset![Capability::Custom("b".to_string())],
        requires: HashSet::new(),
        dependencies: vec!["component-c".to_string()],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    };
    
    let component_a = Component {
        name: "component-a".to_string(),
        version: "0.1.0".to_string(),
        description: "Component A".to_string(),
        category: ComponentCategory::Core,
        provides: hashset![Capability::Custom("a".to_string())],
        requires: HashSet::new(),
        dependencies: vec!["component-b".to_string()],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    };
    
    registry.register(component_a).unwrap();
    registry.register(component_b).unwrap();
    registry.register(component_c).unwrap();
    
    // Resolve dependencies for A
    let resolved = registry.resolve_dependencies(&["component-a".to_string()]).unwrap();
    
    // Should include all three components
    assert!(resolved.contains(&"component-a".to_string()));
    assert!(resolved.contains(&"component-b".to_string()));
    assert!(resolved.contains(&"component-c".to_string()));
}

#[test]
fn test_resolve_dependencies_with_capabilities() {
    let mut registry = ComponentRegistry::new();
    
    // Component that requires a capability
    let requiring_component = Component {
        name: "requiring".to_string(),
        version: "0.1.0".to_string(),
        description: "Component requiring storage".to_string(),
        category: ComponentCategory::Application,
        provides: HashSet::new(),
        requires: hashset![Capability::Storage],
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    };
    
    // Component that provides the capability
    let providing_component = Component {
        name: "storage-provider".to_string(),
        version: "0.1.0".to_string(),
        description: "Storage provider".to_string(),
        category: ComponentCategory::Storage,
        provides: hashset![Capability::Storage],
        requires: HashSet::new(),
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    };
    
    registry.register(requiring_component).unwrap();
    registry.register(providing_component).unwrap();
    
    // Resolve dependencies should include the provider
    let resolved = registry.resolve_dependencies(&["requiring".to_string()]).unwrap();
    assert!(resolved.contains(&"requiring".to_string()));
    assert!(resolved.contains(&"storage-provider".to_string()));
}

#[test]
fn test_missing_dependency_error() {
    let mut registry = ComponentRegistry::new();
    
    let component = Component {
        name: "incomplete".to_string(),
        version: "0.1.0".to_string(),
        description: "Component with missing dep".to_string(),
        category: ComponentCategory::Core,
        provides: HashSet::new(),
        requires: HashSet::new(),
        dependencies: vec!["non-existent".to_string()],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    };
    
    registry.register(component).unwrap();
    
    // Should fail to resolve
    let result = registry.resolve_dependencies(&["incomplete".to_string()]);
    assert!(result.is_err());
}

#[test]
fn test_check_compatibility() {
    let mut registry = ComponentRegistry::new();
    
    let comp1 = create_test_component("comp1");
    let comp2 = create_test_component("comp2");
    
    registry.register(comp1).unwrap();
    registry.register(comp2).unwrap();
    
    // For now, compatibility check is simplified
    assert!(registry.check_compatibility("comp1", "comp2").is_ok());
}

#[test]
fn test_with_defaults() {
    let registry = ComponentRegistry::with_defaults();
    
    // Should have default components registered
    assert!(registry.get("synapsed-core").is_some());
    assert!(registry.get("synapsed-intent").is_some());
    assert!(registry.get("synapsed-verify").is_some());
    assert!(registry.get("synapsed-storage").is_some());
    assert!(registry.get("synapsed-substrates").is_some());
    assert!(registry.get("synapsed-net").is_some());
}

#[test]
fn test_resource_requirements() {
    let mut registry = ComponentRegistry::new();
    
    let gpu_component = Component {
        name: "gpu-component".to_string(),
        version: "0.1.0".to_string(),
        description: "GPU requiring component".to_string(),
        category: ComponentCategory::Compute,
        provides: hashset![Capability::Custom("gpu-compute".to_string())],
        requires: HashSet::new(),
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements {
            min_memory_mb: Some(2048),
            min_cpu_cores: Some(4.0),
            requires_gpu: true,
            requires_network: false,
            requires_filesystem: true,
        },
    };
    
    registry.register(gpu_component.clone()).unwrap();
    
    let retrieved = registry.get("gpu-component").unwrap();
    assert_eq!(retrieved.resources.min_memory_mb, Some(2048));
    assert_eq!(retrieved.resources.min_cpu_cores, Some(4.0));
    assert!(retrieved.resources.requires_gpu);
    assert!(retrieved.resources.requires_filesystem);
}

// Helper functions

fn create_test_component(name: &str) -> Component {
    Component {
        name: name.to_string(),
        version: "0.1.0".to_string(),
        description: format!("Test component {}", name),
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

// Helper macro for creating HashSets
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