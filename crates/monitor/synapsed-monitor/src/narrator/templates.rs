//! Template engine for generating narratives

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Template engine for narrative generation
pub struct TemplateEngine {
    templates: HashMap<String, NarrativeTemplate>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        
        // Task templates
        templates.insert("task.started".to_string(), NarrativeTemplate {
            template: "Task '{name}' has started. Goal: {description}".to_string(),
            variables: vec!["name".to_string(), "description".to_string()],
        });
        
        templates.insert("task.progress".to_string(), NarrativeTemplate {
            template: "Task '{name}' is {progress}% complete. {agents} agents are working on it.".to_string(),
            variables: vec!["name".to_string(), "progress".to_string(), "agents".to_string()],
        });
        
        templates.insert("task.completed".to_string(), NarrativeTemplate {
            template: "Task '{name}' completed successfully in {duration}. {subtasks} sub-tasks were executed.".to_string(),
            variables: vec!["name".to_string(), "duration".to_string(), "subtasks".to_string()],
        });
        
        // Agent templates
        templates.insert("agent.active".to_string(), NarrativeTemplate {
            template: "Agent '{name}' is now {activity}. Trust level: {trust}%".to_string(),
            variables: vec!["name".to_string(), "activity".to_string(), "trust".to_string()],
        });
        
        templates.insert("agent.tool_use".to_string(), NarrativeTemplate {
            template: "Agent '{name}' is using tool '{tool}' to {purpose}".to_string(),
            variables: vec!["name".to_string(), "tool".to_string(), "purpose".to_string()],
        });
        
        templates.insert("agent.anomaly".to_string(), NarrativeTemplate {
            template: "⚠️ Agent '{name}' exhibited unusual behavior: {description}. Severity: {severity}".to_string(),
            variables: vec!["name".to_string(), "description".to_string(), "severity".to_string()],
        });
        
        // System templates
        templates.insert("system.healthy".to_string(), NarrativeTemplate {
            template: "System is operating normally. {services} services online, {agents} agents active.".to_string(),
            variables: vec!["services".to_string(), "agents".to_string()],
        });
        
        templates.insert("system.degraded".to_string(), NarrativeTemplate {
            template: "System performance degraded. {issue}. {recommendation}".to_string(),
            variables: vec!["issue".to_string(), "recommendation".to_string()],
        });
        
        templates.insert("system.alert".to_string(), NarrativeTemplate {
            template: "🔔 {severity} Alert: {message}. Affected: {component}".to_string(),
            variables: vec!["severity".to_string(), "message".to_string(), "component".to_string()],
        });
        
        // Pattern templates
        templates.insert("pattern.error_recovery".to_string(), NarrativeTemplate {
            template: "Good news! The system recovered from the error. {action} was successful after retry.".to_string(),
            variables: vec!["action".to_string()],
        });
        
        templates.insert("pattern.performance".to_string(), NarrativeTemplate {
            template: "Performance issue detected: {metric} is {value}, expected {expected}. Consider {action}.".to_string(),
            variables: vec!["metric".to_string(), "value".to_string(), "expected".to_string(), "action".to_string()],
        });
        
        Self { templates }
    }
    
    /// Generate narrative from template
    pub fn generate(&self, template_name: &str, variables: HashMap<String, String>) -> Option<String> {
        self.templates.get(template_name).map(|template| {
            let mut result = template.template.clone();
            
            for (key, value) in variables {
                result = result.replace(&format!("{{{}}}", key), &value);
            }
            
            result
        })
    }
    
    /// Add custom template
    pub fn add_template(&mut self, name: String, template: NarrativeTemplate) {
        self.templates.insert(name, template);
    }
}

/// Narrative template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeTemplate {
    pub template: String,
    pub variables: Vec<String>,
}

impl NarrativeTemplate {
    pub fn new(template: String) -> Self {
        // Extract variables from template
        let mut variables = Vec::new();
        let mut in_var = false;
        let mut current_var = String::new();
        
        for ch in template.chars() {
            if ch == '{' {
                in_var = true;
                current_var.clear();
            } else if ch == '}' && in_var {
                if !current_var.is_empty() {
                    variables.push(current_var.clone());
                }
                in_var = false;
            } else if in_var {
                current_var.push(ch);
            }
        }
        
        Self { template, variables }
    }
    
    /// Apply variables to template
    pub fn apply(&self, values: &HashMap<String, String>) -> String {
        let mut result = self.template.clone();
        
        for var in &self.variables {
            if let Some(value) = values.get(var) {
                result = result.replace(&format!("{{{}}}", var), value);
            }
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_template_extraction() {
        let template = NarrativeTemplate::new(
            "Agent {name} is {status} with trust {trust}%".to_string()
        );
        
        assert_eq!(template.variables, vec!["name", "status", "trust"]);
    }
    
    #[test]
    fn test_template_application() {
        let template = NarrativeTemplate::new(
            "Task {task} is {progress}% complete".to_string()
        );
        
        let mut values = HashMap::new();
        values.insert("task".to_string(), "Build API".to_string());
        values.insert("progress".to_string(), "75".to_string());
        
        let result = template.apply(&values);
        assert_eq!(result, "Task Build API is 75% complete");
    }
}