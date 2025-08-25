//! Component registry for discovering and managing Synapsed modules

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::{Result, BuilderError};

/// A capability that a component provides
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Capability {
    // Core capabilities
    Storage,
    Networking,
    Cryptography,
    Observability,
    
    // Intent capabilities
    IntentDeclaration,
    IntentVerification,
    IntentExecution,
    
    // Security capabilities
    Authentication,
    Authorization,
    Encryption,
    ZeroKnowledge,
    
    // Compute capabilities
    WasmExecution,
    GpuAcceleration,
    NeuralCompute,
    
    // Distributed capabilities
    Consensus,
    P2PNetworking,
    CRDT,
    Routing,
    
    // Application capabilities
    PaymentProcessing,
    MCPServer,
    Monitoring,
    
    // Custom capability
    Custom(String),
}

/// Interface that a component exposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    pub name: String,
    pub version: String,
    pub methods: Vec<String>,
    pub events: Vec<String>,
}

/// A Synapsed component/crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    /// Crate name (e.g., "synapsed-intent")
    pub name: String,
    
    /// Version
    pub version: String,
    
    /// Human-readable description
    pub description: String,
    
    /// Category (core, network, storage, etc.)
    pub category: ComponentCategory,
    
    /// Capabilities this component provides
    pub provides: HashSet<Capability>,
    
    /// Capabilities this component requires
    pub requires: HashSet<Capability>,
    
    /// Other components this depends on
    pub dependencies: Vec<String>,
    
    /// Interfaces exposed by this component
    pub interfaces: Vec<Interface>,
    
    /// Configuration schema (JSON Schema)
    pub config_schema: Option<serde_json::Value>,
    
    /// Whether this component is observable
    pub observable: bool,
    
    /// Resource requirements
    pub resources: ResourceRequirements,
}

/// Component categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComponentCategory {
    Core,
    Network,
    Storage,
    Security,
    Intent,
    Observability,
    Compute,
    Application,
    Binding,
}

/// Resource requirements for a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub min_memory_mb: Option<u64>,
    pub min_cpu_cores: Option<f32>,
    pub requires_gpu: bool,
    pub requires_network: bool,
    pub requires_filesystem: bool,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            min_memory_mb: None,
            min_cpu_cores: None,
            requires_gpu: false,
            requires_network: false,
            requires_filesystem: false,
        }
    }
}

/// Registry of all available components
pub struct ComponentRegistry {
    components: HashMap<String, Component>,
    capability_index: HashMap<Capability, HashSet<String>>,
}

impl ComponentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            capability_index: HashMap::new(),
        }
    }
    
    /// Initialize with default Synapsed components
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_default_components();
        registry
    }
    
    /// Register a component
    pub fn register(&mut self, component: Component) -> Result<()> {
        // Update capability index
        for capability in &component.provides {
            self.capability_index
                .entry(capability.clone())
                .or_insert_with(HashSet::new)
                .insert(component.name.clone());
        }
        
        self.components.insert(component.name.clone(), component);
        Ok(())
    }
    
    /// Find components that provide a capability
    pub fn find_by_capability(&self, capability: &Capability) -> Vec<&Component> {
        self.capability_index
            .get(capability)
            .map(|names| {
                names.iter()
                    .filter_map(|name| self.components.get(name))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Find components that provide all specified capabilities
    pub fn find_by_capabilities(&self, capabilities: &[Capability]) -> Vec<&Component> {
        self.components
            .values()
            .filter(|c| capabilities.iter().all(|cap| c.provides.contains(cap)))
            .collect()
    }
    
    /// Get a component by name
    pub fn get(&self, name: &str) -> Option<&Component> {
        self.components.get(name)
    }
    
    /// Resolve dependencies for a set of components
    pub fn resolve_dependencies(&self, component_names: &[String]) -> Result<Vec<String>> {
        let mut resolved = HashSet::new();
        let mut to_process: Vec<String> = component_names.to_vec();
        
        while let Some(name) = to_process.pop() {
            if resolved.contains(&name) {
                continue;
            }
            
            let component = self.components.get(&name)
                .ok_or_else(|| BuilderError::ComponentNotFound(name.clone()))?;
            
            // Add dependencies to process
            for dep in &component.dependencies {
                if !resolved.contains(dep) {
                    to_process.push(dep.clone());
                }
            }
            
            // Find components that provide required capabilities
            for required in &component.requires {
                let providers = self.find_by_capability(required);
                if providers.is_empty() {
                    return Err(BuilderError::MissingDependency(
                        name.clone(),
                        format!("{:?}", required)
                    ));
                }
                // Add the first provider (could be smarter about selection)
                if let Some(provider) = providers.first() {
                    if !resolved.contains(&provider.name) {
                        to_process.push(provider.name.clone());
                    }
                }
            }
            
            resolved.insert(name);
        }
        
        Ok(resolved.into_iter().collect())
    }
    
    /// Check if components are compatible
    pub fn check_compatibility(&self, comp1: &str, comp2: &str) -> Result<()> {
        let c1 = self.get(comp1)
            .ok_or_else(|| BuilderError::ComponentNotFound(comp1.to_string()))?;
        let c2 = self.get(comp2)
            .ok_or_else(|| BuilderError::ComponentNotFound(comp2.to_string()))?;
        
        // Check for conflicting capabilities
        // (This is a simplified check - could be more sophisticated)
        
        Ok(())
    }
    
    /// Register default Synapsed components
    fn register_default_components(&mut self) {
        // Core components
        self.register(Component {
            name: "synapsed-core".to_string(),
            version: "0.1.0".to_string(),
            description: "Core traits and runtime".to_string(),
            category: ComponentCategory::Core,
            provides: hashset![Capability::Custom("core".to_string())],
            requires: HashSet::new(),
            dependencies: vec![],
            interfaces: vec![],
            config_schema: None,
            observable: true,
            resources: ResourceRequirements::default(),
        }).unwrap();
        
        // Intent system
        self.register(Component {
            name: "synapsed-intent".to_string(),
            version: "0.1.0".to_string(),
            description: "Hierarchical intent system".to_string(),
            category: ComponentCategory::Intent,
            provides: hashset![
                Capability::IntentDeclaration,
                Capability::IntentExecution
            ],
            requires: hashset![Capability::Storage],
            dependencies: vec!["synapsed-core".to_string()],
            interfaces: vec![
                Interface {
                    name: "IntentBuilder".to_string(),
                    version: "1.0".to_string(),
                    methods: vec!["build".to_string(), "add_step".to_string()],
                    events: vec!["intent.declared".to_string(), "intent.executed".to_string()],
                }
            ],
            config_schema: Some(json!({
                "type": "object",
                "properties": {
                    "max_depth": {"type": "integer"},
                    "timeout_ms": {"type": "integer"}
                }
            })),
            observable: true,
            resources: ResourceRequirements::default(),
        }).unwrap();
        
        // Verification
        self.register(Component {
            name: "synapsed-verify".to_string(),
            version: "0.1.0".to_string(),
            description: "Multi-strategy verification".to_string(),
            category: ComponentCategory::Intent,
            provides: hashset![Capability::IntentVerification],
            requires: hashset![Capability::IntentDeclaration],
            dependencies: vec!["synapsed-core".to_string(), "synapsed-intent".to_string()],
            interfaces: vec![],
            config_schema: None,
            observable: true,
            resources: ResourceRequirements::default(),
        }).unwrap();
        
        // Storage
        self.register(Component {
            name: "synapsed-storage".to_string(),
            version: "0.1.0".to_string(),
            description: "Multi-backend storage system".to_string(),
            category: ComponentCategory::Storage,
            provides: hashset![Capability::Storage],
            requires: HashSet::new(),
            dependencies: vec!["synapsed-core".to_string()],
            interfaces: vec![
                Interface {
                    name: "Storage".to_string(),
                    version: "1.0".to_string(),
                    methods: vec!["get".to_string(), "put".to_string(), "delete".to_string()],
                    events: vec!["storage.write".to_string(), "storage.read".to_string()],
                }
            ],
            config_schema: Some(json!({
                "type": "object",
                "properties": {
                    "backend": {"type": "string", "enum": ["memory", "sqlite", "rocksdb"]},
                    "path": {"type": "string"}
                }
            })),
            observable: true,
            resources: ResourceRequirements {
                requires_filesystem: true,
                ..Default::default()
            },
        }).unwrap();
        
        // Observability
        self.register(Component {
            name: "synapsed-substrates".to_string(),
            version: "0.1.0".to_string(),
            description: "Event-driven observability".to_string(),
            category: ComponentCategory::Observability,
            provides: hashset![Capability::Observability],
            requires: HashSet::new(),
            dependencies: vec!["synapsed-core".to_string()],
            interfaces: vec![
                Interface {
                    name: "Circuit".to_string(),
                    version: "1.0".to_string(),
                    methods: vec!["emit".to_string(), "subscribe".to_string()],
                    events: vec!["*".to_string()],  // Captures all events
                }
            ],
            config_schema: None,
            observable: false,  // It IS the observability!
            resources: ResourceRequirements::default(),
        }).unwrap();
        
        // Network
        self.register(Component {
            name: "synapsed-net".to_string(),
            version: "0.1.0".to_string(),
            description: "P2P networking with privacy".to_string(),
            category: ComponentCategory::Network,
            provides: hashset![Capability::Networking, Capability::P2PNetworking],
            requires: hashset![Capability::Cryptography],
            dependencies: vec!["synapsed-core".to_string(), "synapsed-crypto".to_string()],
            interfaces: vec![],
            config_schema: Some(json!({
                "type": "object",
                "properties": {
                    "listen_addr": {"type": "string"},
                    "bootstrap_nodes": {"type": "array", "items": {"type": "string"}}
                }
            })),
            observable: true,
            resources: ResourceRequirements {
                requires_network: true,
                ..Default::default()
            },
        }).unwrap();
        
        // Add more default components as needed...
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