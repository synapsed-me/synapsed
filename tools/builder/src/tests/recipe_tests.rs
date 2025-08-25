//! Unit tests for Recipe system
//!
//! Intent: Test recipe parsing, validation, and management
//! Verification: All recipe operations work correctly

use crate::recipe::{
    Recipe, RecipeManager, ComponentSpec, Connection, ConnectionPoint,
    Transform, ConnectionProperties, RecipeStep, StepType, Validation,
    ValidationCheck,
};
use std::collections::HashMap;
use serde_json::json;

#[test]
fn test_load_recipe_from_yaml() {
    let yaml_content = r#"
name: test-recipe
version: 1.0.0
description: Test recipe
category: test

components:
  - name: synapsed-core
    version: "^0.1.0"
    features: []
    optional: false
    alias: core

connections: []
configurations: {}
environment: {}
steps: []
validations: []
tags: [test]
"#;

    let mut manager = RecipeManager::new();
    let name = manager.load_yaml(yaml_content).unwrap();
    
    assert_eq!(name, "test-recipe");
    
    let recipe = manager.get("test-recipe").unwrap();
    assert_eq!(recipe.name, "test-recipe");
    assert_eq!(recipe.version, "1.0.0");
    assert_eq!(recipe.components.len(), 1);
}

#[test]
fn test_load_recipe_from_json() {
    let json_content = r#"{
        "name": "json-recipe",
        "version": "1.0.0",
        "description": "JSON recipe",
        "category": "test",
        "components": [
            {
                "name": "synapsed-storage",
                "version": "^0.1.0",
                "features": ["sqlite"],
                "optional": false,
                "alias": "storage"
            }
        ],
        "connections": [],
        "configurations": {},
        "environment": {},
        "steps": [],
        "validations": [],
        "tags": ["json", "test"]
    }"#;

    let mut manager = RecipeManager::new();
    let name = manager.load_json(json_content).unwrap();
    
    assert_eq!(name, "json-recipe");
    
    let recipe = manager.get("json-recipe").unwrap();
    assert_eq!(recipe.components[0].features, vec!["sqlite"]);
}

#[test]
fn test_recipe_validation() {
    let mut manager = RecipeManager::new();
    
    let valid_recipe = create_test_recipe();
    assert!(manager.validate_recipe(&valid_recipe).is_ok());
    
    // Recipe with invalid connection (references non-existent component)
    let mut invalid_recipe = create_test_recipe();
    invalid_recipe.connections.push(Connection {
        from: ConnectionPoint {
            component: "non-existent".to_string(),
            port: "output".to_string(),
        },
        to: ConnectionPoint {
            component: "component1".to_string(),
            port: "input".to_string(),
        },
        transform: None,
        properties: ConnectionProperties::default(),
    });
    
    let validation_result = manager.validate_recipe(&invalid_recipe);
    assert!(validation_result.is_err());
}

#[test]
fn test_recipe_step_dependencies() {
    let mut manager = RecipeManager::new();
    
    let mut recipe = create_test_recipe();
    recipe.steps = vec![
        RecipeStep {
            name: "step1".to_string(),
            step_type: StepType::Initialize,
            params: json!({}),
            depends_on: vec![],
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
            depends_on: vec!["step1".to_string(), "step2".to_string()],
        },
    ];
    
    assert!(manager.validate_recipe(&recipe).is_ok());
    
    // Add invalid dependency
    recipe.steps.push(RecipeStep {
        name: "step4".to_string(),
        step_type: StepType::Start,
        params: json!({}),
        depends_on: vec!["non-existent-step".to_string()],
    });
    
    assert!(manager.validate_recipe(&recipe).is_err());
}

#[test]
fn test_find_recipes_by_category() {
    let mut manager = RecipeManager::new();
    
    let recipe1 = Recipe {
        name: "recipe1".to_string(),
        category: "ai-agent".to_string(),
        ..create_test_recipe()
    };
    
    let recipe2 = Recipe {
        name: "recipe2".to_string(),
        category: "payment".to_string(),
        ..create_test_recipe()
    };
    
    let recipe3 = Recipe {
        name: "recipe3".to_string(),
        category: "ai-agent".to_string(),
        ..create_test_recipe()
    };
    
    manager.recipes.insert("recipe1".to_string(), recipe1);
    manager.recipes.insert("recipe2".to_string(), recipe2);
    manager.recipes.insert("recipe3".to_string(), recipe3);
    
    let ai_recipes = manager.find_by_category("ai-agent");
    assert_eq!(ai_recipes.len(), 2);
    
    let payment_recipes = manager.find_by_category("payment");
    assert_eq!(payment_recipes.len(), 1);
}

#[test]
fn test_find_recipes_by_tag() {
    let mut manager = RecipeManager::new();
    
    let recipe1 = Recipe {
        name: "recipe1".to_string(),
        tags: vec!["ai".to_string(), "verification".to_string()],
        ..create_test_recipe()
    };
    
    let recipe2 = Recipe {
        name: "recipe2".to_string(),
        tags: vec!["payment".to_string(), "security".to_string()],
        ..create_test_recipe()
    };
    
    manager.recipes.insert("recipe1".to_string(), recipe1);
    manager.recipes.insert("recipe2".to_string(), recipe2);
    
    let ai_recipes = manager.find_by_tag("ai");
    assert_eq!(ai_recipes.len(), 1);
    
    let security_recipes = manager.find_by_tag("security");
    assert_eq!(security_recipes.len(), 1);
}

#[test]
fn test_connection_properties() {
    let props = ConnectionProperties::default();
    assert!(props.async_connection);
    assert_eq!(props.buffer_size, Some(100));
    assert!(props.retry);
    assert_eq!(props.timeout_ms, Some(5000));
}

#[test]
fn test_transform_types() {
    let transforms = vec![
        Transform::None,
        Transform::JsonSerialize,
        Transform::JsonDeserialize,
        Transform::Filter("test".to_string()),
        Transform::Map("mapper".to_string()),
        Transform::Custom("custom".to_string()),
    ];
    
    // Just ensure all variants can be created
    assert_eq!(transforms.len(), 6);
}

#[test]
fn test_validation_checks() {
    let checks = vec![
        ValidationCheck::ComponentExists("test".to_string()),
        ValidationCheck::ConnectionValid {
            from: "comp1".to_string(),
            to: "comp2".to_string(),
        },
        ValidationCheck::ConfigurationValid("config".to_string()),
        ValidationCheck::ResourcesAvailable,
        ValidationCheck::Custom("custom check".to_string()),
    ];
    
    assert_eq!(checks.len(), 5);
}

#[test]
fn test_component_spec() {
    let spec = ComponentSpec {
        name: "test-component".to_string(),
        version: Some("^1.0.0".to_string()),
        features: vec!["feature1".to_string(), "feature2".to_string()],
        optional: true,
        alias: Some("test".to_string()),
    };
    
    assert_eq!(spec.name, "test-component");
    assert_eq!(spec.version, Some("^1.0.0".to_string()));
    assert_eq!(spec.features.len(), 2);
    assert!(spec.optional);
    assert_eq!(spec.alias, Some("test".to_string()));
}

#[test]
fn test_load_defaults() {
    let mut manager = RecipeManager::new();
    manager.load_defaults();
    
    // Should have at least one default recipe
    assert!(manager.get("verified-ai-agent").is_some());
    
    let recipe = manager.get("verified-ai-agent").unwrap();
    assert!(!recipe.components.is_empty());
    assert!(!recipe.connections.is_empty());
}

// Helper functions

fn create_test_recipe() -> Recipe {
    Recipe {
        name: "test-recipe".to_string(),
        version: "1.0.0".to_string(),
        description: "Test recipe".to_string(),
        category: "test".to_string(),
        components: vec![
            ComponentSpec {
                name: "component1".to_string(),
                version: None,
                features: vec![],
                optional: false,
                alias: None,
            },
            ComponentSpec {
                name: "component2".to_string(),
                version: None,
                features: vec![],
                optional: false,
                alias: None,
            },
        ],
        connections: vec![],
        configurations: HashMap::new(),
        environment: HashMap::new(),
        steps: vec![],
        validations: vec![],
        tags: vec![],
    }
}