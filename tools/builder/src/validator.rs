//! Validation framework for composed applications

use crate::{
    registry::{ComponentRegistry, Component, Capability},
    recipe::{Connection, Recipe},
    builder::ValidationReport,
    Result, BuilderError,
};
use std::collections::{HashMap, HashSet};

/// Validation result
pub type ValidationResult = Result<ValidationReport>;

/// Validator for compositions
pub struct Validator<'a> {
    registry: &'a ComponentRegistry,
}

impl<'a> Validator<'a> {
    /// Create new validator
    pub fn new(registry: &'a ComponentRegistry) -> Self {
        Self { registry }
    }
    
    /// Validate a composition
    pub fn validate_composition(
        &self,
        components: &[String],
        connections: &[Connection],
    ) -> ValidationResult {
        let mut report = ValidationReport::default();
        
        // Check components exist
        self.validate_components_exist(components, &mut report)?;
        
        // Check dependencies are satisfied
        self.validate_dependencies(components, &mut report)?;
        
        // Check required capabilities are provided
        self.validate_capabilities(components, &mut report)?;
        
        // Check connections are valid
        self.validate_connections(components, connections, &mut report)?;
        
        // Check for incompatibilities
        self.validate_compatibility(components, &mut report)?;
        
        // Check resource requirements
        self.validate_resources(components, &mut report)?;
        
        Ok(report)
    }
    
    /// Validate all components exist
    fn validate_components_exist(
        &self,
        components: &[String],
        report: &mut ValidationReport,
    ) -> Result<()> {
        for component_name in components {
            if self.registry.get(component_name).is_none() {
                report.errors.push(format!(
                    "Component '{}' not found in registry",
                    component_name
                ));
            }
        }
        Ok(())
    }
    
    /// Validate dependencies are satisfied
    fn validate_dependencies(
        &self,
        components: &[String],
        report: &mut ValidationReport,
    ) -> Result<()> {
        let component_set: HashSet<_> = components.iter().cloned().collect();
        
        for component_name in components {
            if let Some(component) = self.registry.get(component_name) {
                // Check hard dependencies
                for dep in &component.dependencies {
                    if !component_set.contains(dep) {
                        report.errors.push(format!(
                            "Component '{}' requires '{}' which is not included",
                            component_name, dep
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate required capabilities are provided
    fn validate_capabilities(
        &self,
        components: &[String],
        report: &mut ValidationReport,
    ) -> Result<()> {
        // Collect all provided capabilities
        let mut provided = HashSet::new();
        for component_name in components {
            if let Some(component) = self.registry.get(component_name) {
                provided.extend(component.provides.clone());
            }
        }
        
        // Check all required capabilities are satisfied
        for component_name in components {
            if let Some(component) = self.registry.get(component_name) {
                for required in &component.requires {
                    if !provided.contains(required) {
                        report.warnings.push(format!(
                            "Component '{}' requires capability '{:?}' which may not be fully provided",
                            component_name, required
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate connections
    fn validate_connections(
        &self,
        components: &[String],
        connections: &[Connection],
        report: &mut ValidationReport,
    ) -> Result<()> {
        let component_set: HashSet<_> = components.iter().cloned().collect();
        
        for connection in connections {
            // Skip wildcard connections
            if connection.from.component == "*" || connection.to.component == "*" {
                continue;
            }
            
            // Check components exist
            if !component_set.contains(&connection.from.component) {
                report.errors.push(format!(
                    "Connection source '{}' not in component list",
                    connection.from.component
                ));
            }
            
            if !component_set.contains(&connection.to.component) {
                report.errors.push(format!(
                    "Connection target '{}' not in component list",
                    connection.to.component
                ));
            }
            
            // Validate ports exist (if we have interface information)
            self.validate_connection_ports(connection, report)?;
        }
        
        Ok(())
    }
    
    /// Validate connection ports
    fn validate_connection_ports(
        &self,
        connection: &Connection,
        report: &mut ValidationReport,
    ) -> Result<()> {
        // Check if source component has the specified output port
        if let Some(source) = self.registry.get(&connection.from.component) {
            let has_port = source.interfaces.iter().any(|iface| {
                iface.events.contains(&connection.from.port) ||
                iface.methods.contains(&connection.from.port) ||
                connection.from.port == "*"
            });
            
            if !has_port && connection.from.port != "*" {
                report.warnings.push(format!(
                    "Component '{}' may not have port '{}'",
                    connection.from.component, connection.from.port
                ));
            }
        }
        
        // Check if target component has the specified input port
        if let Some(target) = self.registry.get(&connection.to.component) {
            let has_port = target.interfaces.iter().any(|iface| {
                iface.methods.contains(&connection.to.port) ||
                connection.to.port == "*"
            });
            
            if !has_port && connection.to.port != "*" {
                report.warnings.push(format!(
                    "Component '{}' may not have port '{}'",
                    connection.to.component, connection.to.port
                ));
            }
        }
        
        Ok(())
    }
    
    /// Check for incompatible components
    fn validate_compatibility(
        &self,
        components: &[String],
        report: &mut ValidationReport,
    ) -> Result<()> {
        // Check for known incompatibilities
        let incompatible_pairs = vec![
            // Example: These components can't be used together
            // ("synapsed-storage-sqlite", "synapsed-storage-rocksdb"),
        ];
        
        for (comp1, comp2) in incompatible_pairs {
            if components.contains(&comp1.to_string()) && 
               components.contains(&comp2.to_string()) {
                report.errors.push(format!(
                    "Components '{}' and '{}' are incompatible",
                    comp1, comp2
                ));
            }
        }
        
        Ok(())
    }
    
    /// Validate resource requirements
    fn validate_resources(
        &self,
        components: &[String],
        report: &mut ValidationReport,
    ) -> Result<()> {
        let mut total_memory = 0u64;
        let mut total_cpu = 0.0f32;
        let mut requires_gpu = false;
        let mut requires_network = false;
        let mut requires_filesystem = false;
        
        for component_name in components {
            if let Some(component) = self.registry.get(component_name) {
                if let Some(mem) = component.resources.min_memory_mb {
                    total_memory += mem;
                }
                if let Some(cpu) = component.resources.min_cpu_cores {
                    total_cpu += cpu;
                }
                requires_gpu |= component.resources.requires_gpu;
                requires_network |= component.resources.requires_network;
                requires_filesystem |= component.resources.requires_filesystem;
            }
        }
        
        // Add info about resource requirements
        if total_memory > 0 {
            report.info.push(format!("Minimum memory required: {} MB", total_memory));
        }
        if total_cpu > 0.0 {
            report.info.push(format!("Minimum CPU cores required: {:.1}", total_cpu));
        }
        if requires_gpu {
            report.info.push("GPU acceleration required".to_string());
        }
        if requires_network {
            report.info.push("Network access required".to_string());
        }
        if requires_filesystem {
            report.info.push("Filesystem access required".to_string());
        }
        
        Ok(())
    }
    
    /// Validate a recipe
    pub fn validate_recipe(&self, recipe: &Recipe) -> ValidationResult {
        let mut report = ValidationReport::default();
        
        // Extract component names
        let components: Vec<String> = recipe.components
            .iter()
            .map(|spec| spec.name.clone())
            .collect();
        
        // Validate composition
        self.validate_composition(&components, &recipe.connections)?;
        
        // Validate configurations
        self.validate_configurations(recipe, &mut report)?;
        
        // Validate steps
        self.validate_steps(recipe, &mut report)?;
        
        Ok(report)
    }
    
    /// Validate configurations
    fn validate_configurations(
        &self,
        recipe: &Recipe,
        report: &mut ValidationReport,
    ) -> Result<()> {
        for (component_name, config) in &recipe.configurations {
            if let Some(component) = self.registry.get(component_name) {
                if let Some(schema) = &component.config_schema {
                    // In a real implementation, validate against JSON schema
                    report.info.push(format!(
                        "Configuration for '{}' should be validated against schema",
                        component_name
                    ));
                }
            } else {
                report.warnings.push(format!(
                    "Configuration provided for unknown component '{}'",
                    component_name
                ));
            }
        }
        
        Ok(())
    }
    
    /// Validate recipe steps
    fn validate_steps(
        &self,
        recipe: &Recipe,
        report: &mut ValidationReport,
    ) -> Result<()> {
        let step_names: HashSet<_> = recipe.steps.iter().map(|s| &s.name).collect();
        
        for step in &recipe.steps {
            // Check dependencies exist
            for dep in &step.depends_on {
                if !step_names.contains(&dep) {
                    report.errors.push(format!(
                        "Step '{}' depends on unknown step '{}'",
                        step.name, dep
                    ));
                }
            }
        }
        
        // Check for circular dependencies
        if self.has_circular_deps(&recipe.steps) {
            report.errors.push("Circular dependency detected in recipe steps".to_string());
        }
        
        Ok(())
    }
    
    /// Check for circular dependencies in steps
    fn has_circular_deps(&self, steps: &[crate::recipe::RecipeStep]) -> bool {
        // Simple cycle detection using DFS
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        
        for step in steps {
            if self.has_cycle_dfs(&step.name, steps, &mut visited, &mut rec_stack) {
                return true;
            }
        }
        
        false
    }
    
    /// DFS helper for cycle detection
    fn has_cycle_dfs(
        &self,
        node: &str,
        steps: &[crate::recipe::RecipeStep],
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        
        if let Some(step) = steps.iter().find(|s| s.name == node) {
            for dep in &step.depends_on {
                if !visited.contains(dep) {
                    if self.has_cycle_dfs(dep, steps, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(dep) {
                    return true;
                }
            }
        }
        
        rec_stack.remove(node);
        false
    }
}