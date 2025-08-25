//! Recipe system for describing how to compose modules

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::{Result, BuilderError};

/// A recipe describes how to compose Synapsed modules into an application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    /// Recipe name
    pub name: String,
    
    /// Version
    pub version: String,
    
    /// Description of what this recipe creates
    pub description: String,
    
    /// Category (e.g., "ai-agent", "payment-system", "monitoring")
    pub category: String,
    
    /// Required components
    pub components: Vec<ComponentSpec>,
    
    /// Connections between components
    pub connections: Vec<Connection>,
    
    /// Configuration for each component
    pub configurations: HashMap<String, serde_json::Value>,
    
    /// Environment variables needed
    pub environment: HashMap<String, String>,
    
    /// Build steps
    pub steps: Vec<RecipeStep>,
    
    /// Validation checks
    pub validations: Vec<Validation>,
    
    /// Tags for discovery
    pub tags: Vec<String>,
}

/// Specification for a component in a recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSpec {
    /// Component name
    pub name: String,
    
    /// Version constraint (e.g., "^0.1.0", ">=0.1.0")
    pub version: Option<String>,
    
    /// Features to enable
    pub features: Vec<String>,
    
    /// Whether this component is optional
    pub optional: bool,
    
    /// Alias for referencing in connections
    pub alias: Option<String>,
}

/// Connection between components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    /// Source component and event/output
    pub from: ConnectionPoint,
    
    /// Target component and input
    pub to: ConnectionPoint,
    
    /// Optional transformation
    pub transform: Option<Transform>,
    
    /// Connection properties
    pub properties: ConnectionProperties,
}

/// A connection point (component + port)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoint {
    /// Component name or alias
    pub component: String,
    
    /// Port/event name (e.g., "output", "events", "*.declared")
    pub port: String,
}

/// Transformation to apply to data flowing through a connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transform {
    /// No transformation
    None,
    
    /// JSON serialization
    JsonSerialize,
    
    /// JSON deserialization
    JsonDeserialize,
    
    /// Filter with expression
    Filter(String),
    
    /// Map with function name
    Map(String),
    
    /// Custom transformation
    Custom(String),
}

/// Properties of a connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProperties {
    /// Whether this connection is async
    pub async_connection: bool,
    
    /// Buffer size for async connections
    pub buffer_size: Option<usize>,
    
    /// Whether to retry on failure
    pub retry: bool,
    
    /// Timeout in milliseconds
    pub timeout_ms: Option<u64>,
}

impl Default for ConnectionProperties {
    fn default() -> Self {
        Self {
            async_connection: true,
            buffer_size: Some(100),
            retry: true,
            timeout_ms: Some(5000),
        }
    }
}

/// A step in the recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStep {
    /// Step name
    pub name: String,
    
    /// Step type
    pub step_type: StepType,
    
    /// Parameters for the step
    pub params: serde_json::Value,
    
    /// Dependencies (other steps that must complete first)
    pub depends_on: Vec<String>,
}

/// Types of recipe steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    /// Initialize a component
    Initialize,
    
    /// Configure a component
    Configure,
    
    /// Connect components
    Connect,
    
    /// Start a component
    Start,
    
    /// Run a validation
    Validate,
    
    /// Custom step
    Custom(String),
}

/// Validation check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validation {
    /// Validation name
    pub name: String,
    
    /// What to validate
    pub check: ValidationCheck,
    
    /// Whether this validation is critical
    pub critical: bool,
}

/// Types of validation checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationCheck {
    /// Check component exists
    ComponentExists(String),
    
    /// Check connection is valid
    ConnectionValid { from: String, to: String },
    
    /// Check configuration is valid
    ConfigurationValid(String),
    
    /// Check resource requirements are met
    ResourcesAvailable,
    
    /// Custom validation
    Custom(String),
}

/// Recipe loader and manager
pub struct RecipeManager {
    recipes: HashMap<String, Recipe>,
}

impl RecipeManager {
    /// Create new recipe manager
    pub fn new() -> Self {
        Self {
            recipes: HashMap::new(),
        }
    }
    
    /// Load recipe from YAML
    pub fn load_yaml(&mut self, yaml_content: &str) -> Result<String> {
        let recipe: Recipe = serde_yaml::from_str(yaml_content)
            .map_err(|e| BuilderError::RecipeError(format!("Failed to parse YAML: {}", e)))?;
        
        let name = recipe.name.clone();
        self.recipes.insert(name.clone(), recipe);
        Ok(name)
    }
    
    /// Load recipe from JSON
    pub fn load_json(&mut self, json_content: &str) -> Result<String> {
        let recipe: Recipe = serde_json::from_str(json_content)
            .map_err(|e| BuilderError::RecipeError(format!("Failed to parse JSON: {}", e)))?;
        
        let name = recipe.name.clone();
        self.recipes.insert(name.clone(), recipe);
        Ok(name)
    }
    
    /// Get a recipe by name
    pub fn get(&self, name: &str) -> Option<&Recipe> {
        self.recipes.get(name)
    }
    
    /// List all recipes
    pub fn list(&self) -> Vec<&Recipe> {
        self.recipes.values().collect()
    }
    
    /// Find recipes by category
    pub fn find_by_category(&self, category: &str) -> Vec<&Recipe> {
        self.recipes
            .values()
            .filter(|r| r.category == category)
            .collect()
    }
    
    /// Find recipes by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<&Recipe> {
        self.recipes
            .values()
            .filter(|r| r.tags.contains(&tag.to_string()))
            .collect()
    }
    
    /// Validate a recipe
    pub fn validate_recipe(&self, recipe: &Recipe) -> Result<()> {
        // Check all components are specified
        if recipe.components.is_empty() {
            return Err(BuilderError::RecipeError("No components specified".to_string()));
        }
        
        // Check all connections reference valid components
        for connection in &recipe.connections {
            let from_exists = recipe.components.iter().any(|c| {
                c.name == connection.from.component || 
                c.alias.as_ref() == Some(&connection.from.component)
            });
            
            let to_exists = recipe.components.iter().any(|c| {
                c.name == connection.to.component || 
                c.alias.as_ref() == Some(&connection.to.component)
            });
            
            if !from_exists {
                return Err(BuilderError::RecipeError(
                    format!("Connection references unknown component: {}", connection.from.component)
                ));
            }
            
            if !to_exists {
                return Err(BuilderError::RecipeError(
                    format!("Connection references unknown component: {}", connection.to.component)
                ));
            }
        }
        
        // Check step dependencies are valid
        let step_names: Vec<_> = recipe.steps.iter().map(|s| &s.name).collect();
        for step in &recipe.steps {
            for dep in &step.depends_on {
                if !step_names.contains(&dep) {
                    return Err(BuilderError::RecipeError(
                        format!("Step {} depends on unknown step: {}", step.name, dep)
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    /// Load default recipes
    pub fn load_defaults(&mut self) {
        // Verified AI Agent recipe
        let verified_agent = Recipe {
            name: "verified-ai-agent".to_string(),
            version: "1.0.0".to_string(),
            description: "AI agent with intent verification and observability".to_string(),
            category: "ai-agent".to_string(),
            components: vec![
                ComponentSpec {
                    name: "synapsed-intent".to_string(),
                    version: Some("^0.1.0".to_string()),
                    features: vec![],
                    optional: false,
                    alias: Some("intent".to_string()),
                },
                ComponentSpec {
                    name: "synapsed-verify".to_string(),
                    version: Some("^0.1.0".to_string()),
                    features: vec![],
                    optional: false,
                    alias: Some("verify".to_string()),
                },
                ComponentSpec {
                    name: "synapsed-storage".to_string(),
                    version: Some("^0.1.0".to_string()),
                    features: vec!["sqlite".to_string()],
                    optional: false,
                    alias: Some("storage".to_string()),
                },
                ComponentSpec {
                    name: "synapsed-substrates".to_string(),
                    version: Some("^0.1.0".to_string()),
                    features: vec![],
                    optional: false,
                    alias: Some("observability".to_string()),
                },
            ],
            connections: vec![
                Connection {
                    from: ConnectionPoint {
                        component: "intent".to_string(),
                        port: "declared".to_string(),
                    },
                    to: ConnectionPoint {
                        component: "verify".to_string(),
                        port: "queue".to_string(),
                    },
                    transform: Some(Transform::None),
                    properties: ConnectionProperties::default(),
                },
                Connection {
                    from: ConnectionPoint {
                        component: "verify".to_string(),
                        port: "completed".to_string(),
                    },
                    to: ConnectionPoint {
                        component: "storage".to_string(),
                        port: "write".to_string(),
                    },
                    transform: Some(Transform::JsonSerialize),
                    properties: ConnectionProperties::default(),
                },
                Connection {
                    from: ConnectionPoint {
                        component: "*".to_string(),
                        port: "*".to_string(),
                    },
                    to: ConnectionPoint {
                        component: "observability".to_string(),
                        port: "events".to_string(),
                    },
                    transform: Some(Transform::None),
                    properties: ConnectionProperties::default(),
                },
            ],
            configurations: HashMap::from([
                ("storage".to_string(), json!({
                    "backend": "sqlite",
                    "path": "/data/intents.db"
                })),
                ("intent".to_string(), json!({
                    "max_depth": 5,
                    "timeout_ms": 30000
                })),
            ]),
            environment: HashMap::from([
                ("RUST_LOG".to_string(), "info".to_string()),
            ]),
            steps: vec![
                RecipeStep {
                    name: "init-storage".to_string(),
                    step_type: StepType::Initialize,
                    params: json!({"component": "storage"}),
                    depends_on: vec![],
                },
                RecipeStep {
                    name: "init-observability".to_string(),
                    step_type: StepType::Initialize,
                    params: json!({"component": "observability"}),
                    depends_on: vec![],
                },
                RecipeStep {
                    name: "connect-all".to_string(),
                    step_type: StepType::Connect,
                    params: json!({"connections": "all"}),
                    depends_on: vec!["init-storage".to_string(), "init-observability".to_string()],
                },
            ],
            validations: vec![
                Validation {
                    name: "components-exist".to_string(),
                    check: ValidationCheck::ComponentExists("synapsed-intent".to_string()),
                    critical: true,
                },
                Validation {
                    name: "resources-available".to_string(),
                    check: ValidationCheck::ResourcesAvailable,
                    critical: true,
                },
            ],
            tags: vec!["ai".to_string(), "verification".to_string(), "observable".to_string()],
        };
        
        self.recipes.insert("verified-ai-agent".to_string(), verified_agent);
        
        // Add more default recipes...
    }
}

// Re-export json! macro for convenience
use serde_json::json;