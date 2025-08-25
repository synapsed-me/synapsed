//! Builder DSL for composing Synapsed applications

use crate::{
    registry::{Component, ComponentRegistry, Capability},
    recipe::{Recipe, Connection, ConnectionPoint, Transform, ConnectionProperties},
    composer::Composer,
    validator::Validator,
    Result, BuilderError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json::json;

/// Builder configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderConfig {
    pub name: String,
    pub version: String,
    pub description: String,
    pub output_dir: String,
}

/// Fluent builder for composing Synapsed applications
pub struct SynapsedBuilder {
    config: BuilderConfig,
    registry: ComponentRegistry,
    components: Vec<String>,
    connections: Vec<Connection>,
    configurations: HashMap<String, serde_json::Value>,
    environment: HashMap<String, String>,
    validations_enabled: bool,
}

impl SynapsedBuilder {
    /// Create a new builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config: BuilderConfig {
                name: name.into(),
                version: "0.1.0".to_string(),
                description: String::new(),
                output_dir: "./build".to_string(),
            },
            registry: ComponentRegistry::with_defaults(),
            components: Vec::new(),
            connections: Vec::new(),
            configurations: HashMap::new(),
            environment: HashMap::new(),
            validations_enabled: true,
        }
    }
    
    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.config.description = desc.into();
        self
    }
    
    /// Set version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.config.version = version.into();
        self
    }
    
    /// Set output directory
    pub fn output_dir(mut self, dir: impl Into<String>) -> Self {
        self.config.output_dir = dir.into();
        self
    }
    
    /// Add a component by name
    pub fn add_component(mut self, name: impl Into<String>) -> Self {
        self.components.push(name.into());
        self
    }
    
    /// Add multiple components
    pub fn add_components(mut self, names: Vec<String>) -> Self {
        self.components.extend(names);
        self
    }
    
    /// Add intent verification capability
    pub fn add_intent_verification(mut self) -> Self {
        self.components.push("synapsed-intent".to_string());
        self.components.push("synapsed-verify".to_string());
        self.connect(
            "synapsed-intent", "declared",
            "synapsed-verify", "queue"
        )
    }
    
    /// Add storage with specified backend
    pub fn add_storage(mut self, backend: StorageBackend) -> Self {
        self.components.push("synapsed-storage".to_string());
        self.configurations.insert(
            "synapsed-storage".to_string(),
            json!({
                "backend": backend.to_string(),
                "path": backend.default_path(),
            })
        );
        self
    }
    
    /// Add observability
    pub fn add_observability(mut self, level: ObservabilityLevel) -> Self {
        self.components.push("synapsed-substrates".to_string());
        
        if level == ObservabilityLevel::Full {
            self.components.push("synapsed-monitor".to_string());
            self.connect(
                "*", "*",
                "synapsed-substrates", "events"
            )
        }
        
        self
    }
    
    /// Add networking capability
    pub fn add_network(mut self, network_type: NetworkType) -> Self {
        match network_type {
            NetworkType::P2P => {
                self.components.push("synapsed-net".to_string());
                self.components.push("synapsed-routing".to_string());
            }
            NetworkType::Consensus => {
                self.components.push("synapsed-net".to_string());
                self.components.push("synapsed-consensus".to_string());
            }
            NetworkType::Simple => {
                self.components.push("synapsed-net".to_string());
            }
        }
        self
    }
    
    /// Add payment processing
    pub fn add_payments(mut self) -> Self {
        self.components.push("synapsed-payments".to_string());
        self.components.push("synapsed-identity".to_string());
        self.components.push("synapsed-crypto".to_string());
        self
    }
    
    /// Connect two components
    pub fn connect(
        mut self,
        from_component: impl Into<String>,
        from_port: impl Into<String>,
        to_component: impl Into<String>,
        to_port: impl Into<String>,
    ) -> Self {
        self.connections.push(Connection {
            from: ConnectionPoint {
                component: from_component.into(),
                port: from_port.into(),
            },
            to: ConnectionPoint {
                component: to_component.into(),
                port: to_port.into(),
            },
            transform: None,
            properties: ConnectionProperties::default(),
        });
        self
    }
    
    /// Connect with transformation
    pub fn connect_with_transform(
        mut self,
        from: ConnectionPoint,
        to: ConnectionPoint,
        transform: Transform,
    ) -> Self {
        self.connections.push(Connection {
            from,
            to,
            transform: Some(transform),
            properties: ConnectionProperties::default(),
        });
        self
    }
    
    /// Configure a component
    pub fn configure(
        mut self,
        component: impl Into<String>,
        config: serde_json::Value,
    ) -> Self {
        self.configurations.insert(component.into(), config);
        self
    }
    
    /// Set environment variable
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }
    
    /// Add components by capability
    pub fn with_capability(mut self, capability: Capability) -> Result<Self> {
        let components = self.registry.find_by_capability(&capability);
        if components.is_empty() {
            return Err(BuilderError::ComponentNotFound(
                format!("No component provides {:?}", capability)
            ));
        }
        
        // Add the first component that provides this capability
        if let Some(component) = components.first() {
            self.components.push(component.name.clone());
        }
        
        Ok(self)
    }
    
    /// Add components by multiple capabilities
    pub fn with_capabilities(mut self, capabilities: Vec<Capability>) -> Result<Self> {
        for capability in capabilities {
            self = self.with_capability(capability)?;
        }
        Ok(self)
    }
    
    /// Load from a recipe
    pub fn from_recipe(recipe: Recipe) -> Self {
        let mut builder = Self::new(&recipe.name)
            .description(&recipe.description)
            .version(&recipe.version);
        
        // Add components
        for component in recipe.components {
            builder.components.push(component.name);
        }
        
        // Add connections
        builder.connections = recipe.connections;
        
        // Add configurations
        builder.configurations = recipe.configurations;
        
        // Add environment
        builder.environment = recipe.environment;
        
        builder
    }
    
    /// Disable validations (for testing)
    pub fn skip_validations(mut self) -> Self {
        self.validations_enabled = false;
        self
    }
    
    /// Validate the composition
    pub fn validate(&self) -> Result<ValidationReport> {
        if !self.validations_enabled {
            return Ok(ValidationReport::default());
        }
        
        let validator = Validator::new(&self.registry);
        validator.validate_composition(&self.components, &self.connections)
    }
    
    /// Build the application
    pub fn build(self) -> Result<ComposedApplication> {
        // Validate first
        let validation_report = self.validate()?;
        if validation_report.has_errors() {
            return Err(BuilderError::ValidationFailed(
                format!("{:?}", validation_report.errors)
            ));
        }
        
        // Resolve dependencies
        let all_components = self.registry.resolve_dependencies(&self.components)?;
        
        // Create composer
        let composer = Composer::new(self.registry);
        
        // Compose the application
        let result = composer.compose(
            all_components,
            self.connections,
            self.configurations,
        )?;
        
        Ok(ComposedApplication {
            name: self.config.name,
            version: self.config.version,
            description: self.config.description,
            components: result.components,
            manifest: result.manifest,
            config_files: result.config_files,
            environment: self.environment,
        })
    }
}

/// Storage backend types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageBackend {
    Memory,
    Sqlite,
    RocksDb,
    Redis,
}

impl StorageBackend {
    fn to_string(&self) -> String {
        match self {
            Self::Memory => "memory".to_string(),
            Self::Sqlite => "sqlite".to_string(),
            Self::RocksDb => "rocksdb".to_string(),
            Self::Redis => "redis".to_string(),
        }
    }
    
    fn default_path(&self) -> String {
        match self {
            Self::Memory => ":memory:".to_string(),
            Self::Sqlite => "./data/app.db".to_string(),
            Self::RocksDb => "./data/rocksdb".to_string(),
            Self::Redis => "redis://localhost:6379".to_string(),
        }
    }
}

/// Observability levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservabilityLevel {
    None,
    Basic,
    Full,
}

/// Network types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkType {
    Simple,
    P2P,
    Consensus,
}

/// Validation report
#[derive(Debug, Default)]
pub struct ValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

impl ValidationReport {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// A composed application ready to be built
#[derive(Debug)]
pub struct ComposedApplication {
    pub name: String,
    pub version: String,
    pub description: String,
    pub components: Vec<String>,
    pub manifest: CargoManifest,
    pub config_files: HashMap<String, String>,
    pub environment: HashMap<String, String>,
}

impl ComposedApplication {
    /// Generate the Cargo.toml file
    pub fn generate_cargo_toml(&self) -> String {
        self.manifest.to_string()
    }
    
    /// Generate main.rs file
    pub fn generate_main_rs(&self) -> String {
        let mut code = String::new();
        
        code.push_str("//! Auto-generated Synapsed application\n\n");
        
        // Add imports
        for component in &self.components {
            code.push_str(&format!("use {};\n", component.replace("-", "_")));
        }
        
        code.push_str("\n#[tokio::main]\n");
        code.push_str("async fn main() -> anyhow::Result<()> {\n");
        code.push_str("    // Initialize tracing\n");
        code.push_str("    tracing_subscriber::fmt::init();\n\n");
        
        // Add initialization code for each component
        code.push_str("    // Initialize components\n");
        for component in &self.components {
            code.push_str(&format!("    // Initialize {}\n", component));
        }
        
        code.push_str("\n    Ok(())\n");
        code.push_str("}\n");
        
        code
    }
    
    /// Save to directory
    pub async fn save(&self, output_dir: &str) -> Result<()> {
        use tokio::fs;
        use std::path::Path;
        
        // Create output directory
        fs::create_dir_all(output_dir).await
            .map_err(|e| BuilderError::IoError(e))?;
        
        // Write Cargo.toml
        let cargo_path = Path::new(output_dir).join("Cargo.toml");
        fs::write(cargo_path, self.generate_cargo_toml()).await
            .map_err(|e| BuilderError::IoError(e))?;
        
        // Create src directory
        let src_dir = Path::new(output_dir).join("src");
        fs::create_dir_all(&src_dir).await
            .map_err(|e| BuilderError::IoError(e))?;
        
        // Write main.rs
        let main_path = src_dir.join("main.rs");
        fs::write(main_path, self.generate_main_rs()).await
            .map_err(|e| BuilderError::IoError(e))?;
        
        // Write config files
        let config_dir = Path::new(output_dir).join("config");
        fs::create_dir_all(&config_dir).await
            .map_err(|e| BuilderError::IoError(e))?;
        
        for (name, content) in &self.config_files {
            let config_path = config_dir.join(name);
            fs::write(config_path, content).await
                .map_err(|e| BuilderError::IoError(e))?;
        }
        
        // Write .env file
        if !self.environment.is_empty() {
            let env_path = Path::new(output_dir).join(".env");
            let env_content: String = self.environment
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("\n");
            fs::write(env_path, env_content).await
                .map_err(|e| BuilderError::IoError(e))?;
        }
        
        Ok(())
    }
}

/// Cargo manifest representation
#[derive(Debug)]
pub struct CargoManifest {
    pub package_name: String,
    pub version: String,
    pub dependencies: HashMap<String, String>,
}

impl CargoManifest {
    fn to_string(&self) -> String {
        let mut toml = String::new();
        
        toml.push_str("[package]\n");
        toml.push_str(&format!("name = \"{}\"\n", self.package_name));
        toml.push_str(&format!("version = \"{}\"\n", self.version));
        toml.push_str("edition = \"2021\"\n\n");
        
        toml.push_str("[dependencies]\n");
        for (name, version) in &self.dependencies {
            toml.push_str(&format!("{} = {}\n", name, version));
        }
        
        toml.push_str("\ntokio = { version = \"1.41\", features = [\"full\"] }\n");
        toml.push_str("anyhow = \"1.0\"\n");
        toml.push_str("tracing = \"0.1\"\n");
        toml.push_str("tracing-subscriber = \"0.3\"\n");
        
        toml
    }
}