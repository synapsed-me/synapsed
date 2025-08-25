//! Smart composition engine for assembling modules

use crate::{
    registry::{ComponentRegistry, Component},
    recipe::Connection,
    Result, BuilderError,
};
use std::collections::{HashMap, HashSet};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;

/// Result of composition
pub struct CompositionResult {
    pub components: Vec<String>,
    pub manifest: crate::builder::CargoManifest,
    pub config_files: HashMap<String, String>,
    pub dependency_graph: DependencyGraph,
}

/// Dependency graph for components
pub struct DependencyGraph {
    graph: DiGraph<String, DependencyType>,
    node_map: HashMap<String, NodeIndex>,
}

/// Type of dependency between components
#[derive(Debug, Clone)]
pub enum DependencyType {
    /// Hard dependency (required)
    Required,
    /// Optional dependency
    Optional,
    /// Data flow dependency
    DataFlow,
    /// Event dependency
    Event,
}

/// Smart composer that assembles components
pub struct Composer {
    registry: ComponentRegistry,
}

impl Composer {
    /// Create new composer
    pub fn new(registry: ComponentRegistry) -> Self {
        Self { registry }
    }
    
    /// Compose components into an application
    pub fn compose(
        &self,
        components: Vec<String>,
        connections: Vec<Connection>,
        configurations: HashMap<String, serde_json::Value>,
    ) -> Result<CompositionResult> {
        // Build dependency graph
        let mut graph = self.build_dependency_graph(&components)?;
        
        // Add connection-based dependencies
        self.add_connection_dependencies(&mut graph, &connections)?;
        
        // Check for cycles
        if let Err(_) = toposort(&graph.graph, None) {
            return Err(BuilderError::CompositionFailed(
                "Circular dependency detected".to_string()
            ));
        }
        
        // Generate Cargo manifest
        let manifest = self.generate_manifest(&components)?;
        
        // Generate config files
        let config_files = self.generate_configs(&components, configurations)?;
        
        Ok(CompositionResult {
            components: components.clone(),
            manifest,
            config_files,
            dependency_graph: graph,
        })
    }
    
    /// Build dependency graph from components
    fn build_dependency_graph(&self, components: &[String]) -> Result<DependencyGraph> {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();
        
        // Add nodes for each component
        for component_name in components {
            let node = graph.add_node(component_name.clone());
            node_map.insert(component_name.clone(), node);
        }
        
        // Add edges for dependencies
        for component_name in components {
            let component = self.registry.get(component_name)
                .ok_or_else(|| BuilderError::ComponentNotFound(component_name.clone()))?;
            
            let from_node = node_map[component_name];
            
            // Add hard dependencies
            for dep in &component.dependencies {
                if let Some(&to_node) = node_map.get(dep) {
                    graph.add_edge(from_node, to_node, DependencyType::Required);
                }
            }
            
            // Add capability-based dependencies
            for required_cap in &component.requires {
                // Find components that provide this capability
                let providers = self.registry.find_by_capability(required_cap);
                for provider in providers {
                    if let Some(&to_node) = node_map.get(&provider.name) {
                        if from_node != to_node {  // Don't add self-dependency
                            graph.add_edge(from_node, to_node, DependencyType::Optional);
                        }
                    }
                }
            }
        }
        
        Ok(DependencyGraph { graph, node_map })
    }
    
    /// Add dependencies based on connections
    fn add_connection_dependencies(
        &self,
        graph: &mut DependencyGraph,
        connections: &[Connection],
    ) -> Result<()> {
        for connection in connections {
            // Skip wildcard connections
            if connection.from.component == "*" || connection.to.component == "*" {
                continue;
            }
            
            if let (Some(&from_node), Some(&to_node)) = (
                graph.node_map.get(&connection.from.component),
                graph.node_map.get(&connection.to.component),
            ) {
                graph.graph.add_edge(from_node, to_node, DependencyType::DataFlow);
            }
        }
        
        Ok(())
    }
    
    /// Generate Cargo manifest
    fn generate_manifest(&self, components: &[String]) -> Result<crate::builder::CargoManifest> {
        let mut dependencies = HashMap::new();
        
        for component_name in components {
            let component = self.registry.get(component_name)
                .ok_or_else(|| BuilderError::ComponentNotFound(component_name.clone()))?;
            
            // Add as path dependency (in real implementation, could be crates.io or git)
            dependencies.insert(
                component_name.clone(),
                format!("{{ path = \"../../crates/{}\", version = \"{}\" }}", 
                    self.component_path(component_name), 
                    component.version
                )
            );
        }
        
        Ok(crate::builder::CargoManifest {
            package_name: "synapsed-app".to_string(),
            version: "0.1.0".to_string(),
            dependencies,
        })
    }
    
    /// Generate configuration files
    fn generate_configs(
        &self,
        components: &[String],
        configurations: HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, String>> {
        let mut config_files = HashMap::new();
        
        // Generate component-specific configs
        for component_name in components {
            if let Some(config) = configurations.get(component_name) {
                let config_str = serde_json::to_string_pretty(config)?;
                config_files.insert(
                    format!("{}.json", component_name),
                    config_str,
                );
            }
        }
        
        // Generate main config file
        let main_config = serde_json::json!({
            "components": components,
            "configurations": configurations,
        });
        
        config_files.insert(
            "app.json".to_string(),
            serde_json::to_string_pretty(&main_config)?,
        );
        
        Ok(config_files)
    }
    
    /// Determine crate path from component name
    fn component_path(&self, name: &str) -> String {
        // Map component names to crate paths
        match name {
            n if n.starts_with("synapsed-") => {
                let parts: Vec<&str> = n.strip_prefix("synapsed-").unwrap().split('-').collect();
                match parts[0] {
                    "core" | "crypto" | "gpu" => format!("core/{}", n),
                    "intent" | "promise" | "verify" | "swarm" => format!("intent/{}", n),
                    "net" | "consensus" | "routing" => format!("network/{}", n),
                    "storage" | "crdt" => format!("storage/{}", n),
                    "identity" | "safety" => format!("security/{}", n),
                    "substrates" | "serventis" => format!("observability/{}", n),
                    "monitor" => format!("monitor/{}", n),
                    "wasm" | "neural" => format!("compute/{}", n),
                    "mcp" | "payments" => format!("applications/{}", n),
                    _ => n.to_string(),
                }
            }
            _ => name.to_string(),
        }
    }
    
    /// Optimize component selection based on requirements
    pub fn optimize_selection(
        &self,
        requirements: &[String],
        constraints: &Constraints,
    ) -> Result<Vec<String>> {
        let mut selected = HashSet::new();
        let mut to_process = requirements.to_vec();
        
        while let Some(req) = to_process.pop() {
            // Check if it's a component name
            if let Some(component) = self.registry.get(&req) {
                selected.insert(component.name.clone());
                
                // Add dependencies
                for dep in &component.dependencies {
                    if !selected.contains(dep) {
                        to_process.push(dep.clone());
                    }
                }
            }
            // Otherwise, treat as a capability requirement
            else if let Ok(capability) = self.parse_capability(&req) {
                let providers = self.registry.find_by_capability(&capability);
                
                // Select best provider based on constraints
                if let Some(best) = self.select_best_provider(providers, constraints) {
                    selected.insert(best.name.clone());
                    
                    // Add its dependencies
                    for dep in &best.dependencies {
                        if !selected.contains(dep) {
                            to_process.push(dep.clone());
                        }
                    }
                }
            }
        }
        
        Ok(selected.into_iter().collect())
    }
    
    /// Parse capability from string
    fn parse_capability(&self, s: &str) -> Result<crate::registry::Capability> {
        use crate::registry::Capability;
        
        match s {
            "storage" => Ok(Capability::Storage),
            "network" | "networking" => Ok(Capability::Networking),
            "crypto" | "cryptography" => Ok(Capability::Cryptography),
            "observability" | "monitoring" => Ok(Capability::Observability),
            "intent" => Ok(Capability::IntentDeclaration),
            "verification" => Ok(Capability::IntentVerification),
            "payment" | "payments" => Ok(Capability::PaymentProcessing),
            _ => Ok(Capability::Custom(s.to_string())),
        }
    }
    
    /// Select best provider based on constraints
    fn select_best_provider<'a>(
        &self,
        providers: Vec<&'a Component>,
        constraints: &Constraints,
    ) -> Option<&'a Component> {
        providers.into_iter()
            .filter(|c| self.meets_constraints(c, constraints))
            .min_by_key(|c| {
                // Simple scoring: prefer components with fewer dependencies
                c.dependencies.len() + c.requires.len()
            })
    }
    
    /// Check if component meets constraints
    fn meets_constraints(&self, component: &Component, constraints: &Constraints) -> bool {
        // Check resource constraints
        if let Some(max_memory) = constraints.max_memory_mb {
            if let Some(min_memory) = component.resources.min_memory_mb {
                if min_memory > max_memory {
                    return false;
                }
            }
        }
        
        // Check other constraints...
        
        true
    }
}

/// Constraints for component selection
#[derive(Debug, Default)]
pub struct Constraints {
    pub max_memory_mb: Option<u64>,
    pub max_cpu_cores: Option<f32>,
    pub allow_network: bool,
    pub allow_filesystem: bool,
    pub prefer_minimal: bool,
}

impl DependencyGraph {
    /// Get initialization order
    pub fn initialization_order(&self) -> Result<Vec<String>> {
        match toposort(&self.graph, None) {
            Ok(order) => {
                Ok(order.into_iter()
                    .map(|idx| self.graph[idx].clone())
                    .collect())
            }
            Err(_) => Err(BuilderError::CompositionFailed(
                "Cannot determine initialization order due to circular dependencies".to_string()
            ))
        }
    }
    
    /// Get all dependencies of a component
    pub fn get_dependencies(&self, component: &str) -> Vec<String> {
        if let Some(&node) = self.node_map.get(component) {
            self.graph.neighbors(node)
                .map(|idx| self.graph[idx].clone())
                .collect()
        } else {
            Vec::new()
        }
    }
}