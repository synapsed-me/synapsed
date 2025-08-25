//! Unit tests for Validator
//!
//! Intent: Test validation logic for compositions
//! Verification: All validation rules work correctly

use crate::validator::{Validator, ValidationResult};
use crate::registry::{ComponentRegistry, Component, Capability, ComponentCategory, ResourceRequirements};
use crate::recipe::{
    Recipe, ComponentSpec, Connection, ConnectionPoint, ConnectionProperties,
    RecipeStep, StepType, Validation, ValidationCheck,
};
use crate::builder::ValidationReport;
use std::collections::{HashMap, HashSet};
use serde_json::json;

#[test]
fn test_validate_components_exist() {
    let registry = create_test_registry();
    let validator = Validator::new(&registry);
    
    // Valid components
    let valid_components = vec!["test-core".to_string(), "test-storage".to_string()];
    let result = validator.validate_composition(&valid_components, &[]);
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(!report.has_errors());
    
    // Invalid component
    let invalid_components = vec!["non-existent".to_string()];
    let result = validator.validate_composition(&invalid_components, &[]);
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(report.has_errors());
    assert!(report.errors[0].contains("not found"));
}

#[test]
fn test_validate_dependencies() {
    let mut registry = ComponentRegistry::new();
    
    // Component with dependency
    registry.register(Component {
        name: "dependent".to_string(),
        version: "0.1.0".to_string(),
        description: "Dependent component".to_string(),
        category: ComponentCategory::Core,
        provides: HashSet::new(),
        requires: HashSet::new(),
        dependencies: vec!["required".to_string()],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    }).unwrap();
    
    registry.register(Component {
        name: "required".to_string(),
        version: "0.1.0".to_string(),
        description: "Required component".to_string(),
        category: ComponentCategory::Core,
        provides: HashSet::new(),
        requires: HashSet::new(),
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    }).unwrap();
    
    let validator = Validator::new(&registry);
    
    // Missing dependency
    let components = vec!["dependent".to_string()];
    let result = validator.validate_composition(&components, &[]);
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(report.has_errors());
    assert!(report.errors[0].contains("requires 'required'"));
    
    // Dependency satisfied
    let components = vec!["dependent".to_string(), "required".to_string()];
    let result = validator.validate_composition(&components, &[]);
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(!report.has_errors());
}

#[test]
fn test_validate_capabilities() {
    let mut registry = ComponentRegistry::new();
    
    // Component requiring a capability
    registry.register(Component {
        name: "needs-storage".to_string(),
        version: "0.1.0".to_string(),
        description: "Needs storage".to_string(),
        category: ComponentCategory::Application,
        provides: HashSet::new(),
        requires: hashset![Capability::Storage],
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements::default(),
    }).unwrap();
    
    // Component providing the capability
    registry.register(Component {
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
    }).unwrap();
    
    let validator = Validator::new(&registry);
    
    // Missing capability provider
    let components = vec!["needs-storage".to_string()];
    let result = validator.validate_composition(&components, &[]);
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(report.warnings.len() > 0);
    assert!(report.warnings[0].contains("Storage"));
    
    // Capability satisfied
    let components = vec!["needs-storage".to_string(), "storage-provider".to_string()];
    let result = validator.validate_composition(&components, &[]);
    assert!(result.is_ok());
    let report = result.unwrap();
    assert_eq!(report.warnings.len(), 0);
}

#[test]
fn test_validate_connections() {
    let registry = create_test_registry();
    let validator = Validator::new(&registry);
    
    let components = vec!["test-core".to_string(), "test-storage".to_string()];
    
    // Valid connection
    let valid_connections = vec![
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
    
    let result = validator.validate_composition(&components, &valid_connections);
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(!report.has_errors());
    
    // Invalid connection (component not in list)
    let invalid_connections = vec![
        Connection {
            from: ConnectionPoint {
                component: "non-existent".to_string(),
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
    
    let result = validator.validate_composition(&components, &invalid_connections);
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(report.has_errors());
    assert!(report.errors[0].contains("not in component list"));
}

#[test]
fn test_validate_wildcard_connections() {
    let registry = create_test_registry();
    let validator = Validator::new(&registry);
    
    let components = vec!["test-core".to_string()];
    
    // Wildcard connections should be allowed
    let wildcard_connections = vec![
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
    
    let result = validator.validate_composition(&components, &wildcard_connections);
    assert!(result.is_ok());
    let report = result.unwrap();
    // Wildcard connections are skipped in validation
    assert!(!report.has_errors());
}

#[test]
fn test_validate_resources() {
    let mut registry = ComponentRegistry::new();
    
    // Components with resource requirements
    registry.register(Component {
        name: "memory-heavy".to_string(),
        version: "0.1.0".to_string(),
        description: "Memory heavy".to_string(),
        category: ComponentCategory::Compute,
        provides: HashSet::new(),
        requires: HashSet::new(),
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements {
            min_memory_mb: Some(2048),
            min_cpu_cores: Some(2.0),
            requires_gpu: true,
            requires_network: true,
            requires_filesystem: true,
        },
    }).unwrap();
    
    registry.register(Component {
        name: "cpu-heavy".to_string(),
        version: "0.1.0".to_string(),
        description: "CPU heavy".to_string(),
        category: ComponentCategory::Compute,
        provides: HashSet::new(),
        requires: HashSet::new(),
        dependencies: vec![],
        interfaces: vec![],
        config_schema: None,
        observable: true,
        resources: ResourceRequirements {
            min_memory_mb: Some(512),
            min_cpu_cores: Some(4.0),
            requires_gpu: false,
            requires_network: false,
            requires_filesystem: false,
        },
    }).unwrap();
    
    let validator = Validator::new(&registry);
    
    let components = vec!["memory-heavy".to_string(), "cpu-heavy".to_string()];
    let result = validator.validate_composition(&components, &[]);
    assert!(result.is_ok());
    
    let report = result.unwrap();
    assert!(!report.has_errors());
    
    // Check info messages about resources
    assert!(report.info.iter().any(|msg| msg.contains("2560 MB"))); // 2048 + 512
    assert!(report.info.iter().any(|msg| msg.contains("6.0"))); // 2.0 + 4.0 cores
    assert!(report.info.iter().any(|msg| msg.contains("GPU")));
    assert!(report.info.iter().any(|msg| msg.contains("Network")));
    assert!(report.info.iter().any(|msg| msg.contains("Filesystem")));
}

#[test]
fn test_validate_recipe() {
    let registry = create_test_registry();
    let validator = Validator::new(&registry);
    
    let recipe = Recipe {
        name: "test-recipe".to_string(),
        version: "1.0.0".to_string(),
        description: "Test recipe".to_string(),
        category: "test".to_string(),
        components: vec![
            ComponentSpec {
                name: "test-core".to_string(),
                version: None,
                features: vec![],
                optional: false,
                alias: None,
            },
            ComponentSpec {
                name: "test-storage".to_string(),
                version: None,
                features: vec![],
                optional: false,
                alias: None,
            },
        ],
        connections: vec![
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
        ],
        configurations: HashMap::new(),
        environment: HashMap::new(),
        steps: vec![
            RecipeStep {
                name: "step1".to_string(),
                step_type: StepType::Initialize,
                params: json!({}),
                depends_on: vec![],
            },
            RecipeStep {
                name: "step2".to_string(),
                step_type: StepType::Start,
                params: json!({}),
                depends_on: vec!["step1".to_string()],
            },
        ],
        validations: vec![
            Validation {
                name: "check-core".to_string(),
                check: ValidationCheck::ComponentExists("test-core".to_string()),
                critical: true,
            }
        ],
        tags: vec![],
    };
    
    let result = validator.validate_recipe(&recipe);
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(!report.has_errors());
}

#[test]
fn test_circular_step_dependencies() {
    let registry = create_test_registry();
    let validator = Validator::new(&registry);
    
    let mut recipe = create_minimal_recipe();
    
    // Create circular dependency: step1 -> step2 -> step3 -> step1
    recipe.steps = vec![
        RecipeStep {
            name: "step1".to_string(),
            step_type: StepType::Initialize,
            params: json!({}),
            depends_on: vec!["step3".to_string()], // Circular!
        },
        RecipeStep {
            name: "step2".to_string(),
            step_type: StepType::Configure,
            params: json!({}),
            depends_on: vec!["step1".to_string()],
        },
        RecipeStep {
            name: "step3".to_string(),
            step_type: StepType::Start,
            params: json!({}),
            depends_on: vec!["step2".to_string()],
        },
    ];
    
    let result = validator.validate_recipe(&recipe);
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(report.has_errors());
    assert!(report.errors[0].contains("Circular dependency"));
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

fn create_minimal_recipe() -> Recipe {
    Recipe {
        name: "minimal".to_string(),
        version: "1.0.0".to_string(),
        description: "Minimal recipe".to_string(),
        category: "test".to_string(),
        components: vec![
            ComponentSpec {
                name: "test-core".to_string(),
                version: None,
                features: vec![],
                optional: false,
                alias: None,
            }
        ],
        connections: vec![],
        configurations: HashMap::new(),
        environment: HashMap::new(),
        steps: vec![],
        validations: vec![],
        tags: vec![],
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