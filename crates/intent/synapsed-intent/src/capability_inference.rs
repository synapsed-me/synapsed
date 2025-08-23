//! Tool capability inference engine
//! 
//! This module infers agent capabilities from tool combinations and usage patterns,
//! building a knowledge graph of tool relationships and capabilities.

use crate::{
    dynamic_agents::{SubAgentDefinition, RiskLevel},
};
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

/// Capability inference engine that understands tool relationships
pub struct CapabilityInferenceEngine {
    tool_graph: ToolRelationshipGraph,
    capability_rules: Vec<InferenceRule>,
    learned_patterns: HashMap<String, LearnedPattern>,
}

/// Graph of tool relationships and dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRelationshipGraph {
    pub nodes: HashMap<String, ToolNode>,
    pub edges: Vec<ToolEdge>,
}

/// Node representing a tool in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolNode {
    pub name: String,
    pub category: ToolCategory,
    pub primary_capabilities: Vec<String>,
    pub required_tools: Vec<String>,  // Tools this one depends on
    pub complements: Vec<String>,     // Tools that work well with this
    pub risk_level: RiskLevel,
}

/// Edge representing relationship between tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEdge {
    pub from: String,
    pub to: String,
    pub relationship: ToolRelationship,
    pub strength: f64,  // 0.0 to 1.0
}

/// Types of tool relationships
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolRelationship {
    Requires,      // Tool A requires Tool B
    Complements,   // Tools work well together
    Substitutes,   // Tools can replace each other
    Enhances,      // Tool A enhances Tool B's capabilities
    Conflicts,     // Tools shouldn't be used together
}

/// Category of tools
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolCategory {
    FileSystem,
    Network,
    Execution,
    Analysis,
    Testing,
    Deployment,
    Monitoring,
    Documentation,
    Security,
}

/// Rule for inferring capabilities
#[derive(Debug, Clone)]
pub struct InferenceRule {
    pub name: String,
    pub condition: RuleCondition,
    pub inferred_capabilities: Vec<String>,
    pub confidence: f64,
}

/// Condition for applying an inference rule
#[derive(Debug, Clone)]
pub enum RuleCondition {
    HasTools(Vec<String>),
    HasAnyTools(Vec<String>),
    ToolCombination { all_of: Vec<String>, any_of: Vec<String> },
    CategoryPresent(ToolCategory),
    PatternMatch(String),  // Regex pattern
}

/// Pattern learned from agent behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    pub pattern_id: String,
    pub tool_sequence: Vec<String>,
    pub observed_capabilities: Vec<String>,
    pub frequency: usize,
    pub success_rate: f64,
    pub avg_execution_time_ms: u64,
}

/// Inferred capability with confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredCapability {
    pub capability: String,
    pub confidence: f64,
    pub reasoning: Vec<String>,
    pub supporting_tools: Vec<String>,
}

impl CapabilityInferenceEngine {
    pub fn new() -> Self {
        Self {
            tool_graph: Self::build_tool_graph(),
            capability_rules: Self::create_inference_rules(),
            learned_patterns: HashMap::new(),
        }
    }

    /// Infer capabilities from an agent definition
    pub fn infer_capabilities(&self, agent: &SubAgentDefinition) -> Vec<InferredCapability> {
        let mut inferred = Vec::new();
        let mut reasoning_map: HashMap<String, (f64, Vec<String>)> = HashMap::new();

        // Apply inference rules
        for rule in &self.capability_rules {
            if self.evaluate_condition(&rule.condition, &agent.tools) {
                for capability in &rule.inferred_capabilities {
                    let entry = reasoning_map.entry(capability.clone()).or_insert((0.0, Vec::new()));
                    entry.0 = entry.0.max(rule.confidence);
                    entry.1.push(format!("Rule '{}' matched", rule.name));
                }
            }
        }

        // Analyze tool combinations
        let combinations = self.analyze_tool_combinations(&agent.tools);
        for (capability, confidence, reason) in combinations {
            let entry = reasoning_map.entry(capability).or_insert((0.0, Vec::new()));
            entry.0 = entry.0.max(confidence);
            entry.1.push(reason);
        }

        // Check learned patterns
        for (pattern_id, pattern) in &self.learned_patterns {
            if self.matches_pattern(&agent.tools, &pattern.tool_sequence) {
                for capability in &pattern.observed_capabilities {
                    let confidence = pattern.success_rate * 0.8; // Slightly discount learned patterns
                    let entry = reasoning_map.entry(capability.clone()).or_insert((0.0, Vec::new()));
                    entry.0 = entry.0.max(confidence);
                    entry.1.push(format!("Learned pattern '{}' ({}% success)", pattern_id, (pattern.success_rate * 100.0) as u32));
                }
            }
        }

        // Build final capability list
        for (capability, (confidence, reasoning)) in reasoning_map {
            if confidence > 0.3 {  // Minimum confidence threshold
                inferred.push(InferredCapability {
                    capability,
                    confidence,
                    reasoning,
                    supporting_tools: self.get_supporting_tools(&agent.tools),
                });
            }
        }

        // Sort by confidence
        inferred.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        inferred
    }

    /// Analyze tool combinations for capability inference
    fn analyze_tool_combinations(&self, tools: &[String]) -> Vec<(String, f64, String)> {
        let mut capabilities = Vec::new();

        // Check for common powerful combinations
        if tools.contains(&"git_ops".to_string()) && tools.contains(&"run_command".to_string()) {
            capabilities.push((
                "deployment".to_string(),
                0.9,
                "Has git_ops + run_command for deployment tasks".to_string(),
            ));
        }

        if tools.contains(&"ast_parser".to_string()) && tools.contains(&"read_file".to_string()) {
            capabilities.push((
                "code_analysis".to_string(),
                0.95,
                "Has ast_parser + read_file for code analysis".to_string(),
            ));
        }

        if tools.contains(&"test_runner".to_string()) && tools.contains(&"debug_shell".to_string()) {
            capabilities.push((
                "debugging".to_string(),
                0.85,
                "Has test_runner + debug_shell for debugging".to_string(),
            ));
        }

        if tools.contains(&"web_search".to_string()) && tools.contains(&"write_file".to_string()) {
            capabilities.push((
                "research_documentation".to_string(),
                0.8,
                "Has web_search + write_file for research and documentation".to_string(),
            ));
        }

        // Check tool categories
        let categories = self.get_tool_categories(tools);
        
        if categories.contains(&ToolCategory::FileSystem) && categories.contains(&ToolCategory::Analysis) {
            capabilities.push((
                "data_processing".to_string(),
                0.7,
                "Has file system and analysis tools".to_string(),
            ));
        }

        if categories.contains(&ToolCategory::Network) && categories.contains(&ToolCategory::Security) {
            capabilities.push((
                "security_testing".to_string(),
                0.75,
                "Has network and security tools".to_string(),
            ));
        }

        capabilities
    }

    /// Get categories of tools
    fn get_tool_categories(&self, tools: &[String]) -> HashSet<ToolCategory> {
        let mut categories = HashSet::new();
        
        for tool in tools {
            if let Some(node) = self.tool_graph.nodes.get(tool) {
                categories.insert(node.category.clone());
            }
        }
        
        categories
    }

    /// Evaluate a rule condition
    fn evaluate_condition(&self, condition: &RuleCondition, tools: &[String]) -> bool {
        match condition {
            RuleCondition::HasTools(required) => {
                required.iter().all(|t| tools.contains(t))
            },
            RuleCondition::HasAnyTools(options) => {
                options.iter().any(|t| tools.contains(t))
            },
            RuleCondition::ToolCombination { all_of, any_of } => {
                all_of.iter().all(|t| tools.contains(t)) &&
                (any_of.is_empty() || any_of.iter().any(|t| tools.contains(t)))
            },
            RuleCondition::CategoryPresent(category) => {
                tools.iter().any(|t| {
                    self.tool_graph.nodes.get(t)
                        .map(|n| &n.category == category)
                        .unwrap_or(false)
                })
            },
            RuleCondition::PatternMatch(pattern) => {
                // Simple pattern matching for now
                tools.iter().any(|t| t.contains(pattern))
            },
        }
    }

    /// Check if tools match a learned pattern
    fn matches_pattern(&self, tools: &[String], pattern: &[String]) -> bool {
        // Check if all pattern tools are present
        pattern.iter().all(|t| tools.contains(t))
    }

    /// Get tools that support the inferred capabilities
    fn get_supporting_tools(&self, tools: &[String]) -> Vec<String> {
        tools.to_vec()
    }

    /// Learn a new pattern from agent execution
    pub fn learn_pattern(
        &mut self,
        tools_used: Vec<String>,
        capabilities_demonstrated: Vec<String>,
        success: bool,
        execution_time_ms: u64,
    ) {
        let pattern_key = tools_used.join("+");
        
        let pattern = self.learned_patterns.entry(pattern_key.clone()).or_insert(LearnedPattern {
            pattern_id: pattern_key,
            tool_sequence: tools_used,
            observed_capabilities: Vec::new(),
            frequency: 0,
            success_rate: 0.0,
            avg_execution_time_ms: 0,
        });
        
        // Update pattern statistics
        pattern.frequency += 1;
        pattern.success_rate = if success {
            (pattern.success_rate * (pattern.frequency - 1) as f64 + 1.0) / pattern.frequency as f64
        } else {
            (pattern.success_rate * (pattern.frequency - 1) as f64) / pattern.frequency as f64
        };
        
        // Update average execution time
        pattern.avg_execution_time_ms = 
            ((pattern.avg_execution_time_ms * (pattern.frequency - 1) as u64) + execution_time_ms) / pattern.frequency as u64;
        
        // Add new capabilities
        for cap in capabilities_demonstrated {
            if !pattern.observed_capabilities.contains(&cap) {
                pattern.observed_capabilities.push(cap);
            }
        }
    }

    /// Get tool recommendations for a desired capability
    pub fn recommend_tools_for_capability(&self, capability: &str) -> Vec<(String, f64)> {
        let mut recommendations = Vec::new();
        
        // Check rules for tools that provide this capability
        for rule in &self.capability_rules {
            if rule.inferred_capabilities.contains(&capability.to_string()) {
                if let RuleCondition::HasTools(tools) = &rule.condition {
                    for tool in tools {
                        recommendations.push((tool.clone(), rule.confidence));
                    }
                }
            }
        }
        
        // Check learned patterns
        for pattern in self.learned_patterns.values() {
            if pattern.observed_capabilities.contains(&capability.to_string()) {
                for tool in &pattern.tool_sequence {
                    recommendations.push((tool.clone(), pattern.success_rate));
                }
            }
        }
        
        // Sort by confidence and deduplicate
        recommendations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        recommendations.dedup_by_key(|r| r.0.clone());
        
        recommendations
    }

    /// Build the tool relationship graph
    fn build_tool_graph() -> ToolRelationshipGraph {
        let mut nodes = HashMap::new();
        let mut edges = Vec::new();

        // File system tools
        nodes.insert("read_file".to_string(), ToolNode {
            name: "read_file".to_string(),
            category: ToolCategory::FileSystem,
            primary_capabilities: vec!["file_reading".to_string()],
            required_tools: vec![],
            complements: vec!["write_file".to_string(), "str_replace".to_string()],
            risk_level: RiskLevel::Low,
        });

        nodes.insert("write_file".to_string(), ToolNode {
            name: "write_file".to_string(),
            category: ToolCategory::FileSystem,
            primary_capabilities: vec!["file_writing".to_string()],
            required_tools: vec![],
            complements: vec!["read_file".to_string()],
            risk_level: RiskLevel::Medium,
        });

        // Analysis tools
        nodes.insert("ast_parser".to_string(), ToolNode {
            name: "ast_parser".to_string(),
            category: ToolCategory::Analysis,
            primary_capabilities: vec!["code_parsing".to_string(), "syntax_analysis".to_string()],
            required_tools: vec!["read_file".to_string()],
            complements: vec!["test_runner".to_string()],
            risk_level: RiskLevel::Low,
        });

        // Execution tools
        nodes.insert("run_command".to_string(), ToolNode {
            name: "run_command".to_string(),
            category: ToolCategory::Execution,
            primary_capabilities: vec!["command_execution".to_string()],
            required_tools: vec![],
            complements: vec!["debug_shell".to_string()],
            risk_level: RiskLevel::High,
        });

        nodes.insert("test_runner".to_string(), ToolNode {
            name: "test_runner".to_string(),
            category: ToolCategory::Testing,
            primary_capabilities: vec!["test_execution".to_string()],
            required_tools: vec!["run_command".to_string()],
            complements: vec!["debug_shell".to_string()],
            risk_level: RiskLevel::Medium,
        });

        // Network tools
        nodes.insert("web_search".to_string(), ToolNode {
            name: "web_search".to_string(),
            category: ToolCategory::Network,
            primary_capabilities: vec!["information_retrieval".to_string()],
            required_tools: vec![],
            complements: vec!["write_file".to_string()],
            risk_level: RiskLevel::Low,
        });

        // Deployment tools
        nodes.insert("git_ops".to_string(), ToolNode {
            name: "git_ops".to_string(),
            category: ToolCategory::Deployment,
            primary_capabilities: vec!["version_control".to_string()],
            required_tools: vec![],
            complements: vec!["run_command".to_string()],
            risk_level: RiskLevel::Medium,
        });

        // Add edges
        edges.push(ToolEdge {
            from: "ast_parser".to_string(),
            to: "read_file".to_string(),
            relationship: ToolRelationship::Requires,
            strength: 1.0,
        });

        edges.push(ToolEdge {
            from: "test_runner".to_string(),
            to: "run_command".to_string(),
            relationship: ToolRelationship::Requires,
            strength: 0.9,
        });

        edges.push(ToolEdge {
            from: "git_ops".to_string(),
            to: "run_command".to_string(),
            relationship: ToolRelationship::Complements,
            strength: 0.8,
        });

        ToolRelationshipGraph { nodes, edges }
    }

    /// Create inference rules
    fn create_inference_rules() -> Vec<InferenceRule> {
        vec![
            InferenceRule {
                name: "Full Stack Development".to_string(),
                condition: RuleCondition::ToolCombination {
                    all_of: vec!["read_file".to_string(), "write_file".to_string()],
                    any_of: vec!["run_command".to_string(), "test_runner".to_string()],
                },
                inferred_capabilities: vec!["full_stack_development".to_string()],
                confidence: 0.85,
            },
            InferenceRule {
                name: "Security Analysis".to_string(),
                condition: RuleCondition::ToolCombination {
                    all_of: vec!["ast_parser".to_string()],
                    any_of: vec!["web_search".to_string()],
                },
                inferred_capabilities: vec!["security_analysis".to_string()],
                confidence: 0.75,
            },
            InferenceRule {
                name: "CI/CD Pipeline".to_string(),
                condition: RuleCondition::HasTools(vec![
                    "git_ops".to_string(),
                    "run_command".to_string(),
                    "test_runner".to_string(),
                ]),
                inferred_capabilities: vec!["continuous_integration".to_string(), "continuous_deployment".to_string()],
                confidence: 0.9,
            },
            InferenceRule {
                name: "Data Processing".to_string(),
                condition: RuleCondition::ToolCombination {
                    all_of: vec!["read_file".to_string()],
                    any_of: vec!["run_command".to_string(), "write_file".to_string()],
                },
                inferred_capabilities: vec!["data_processing".to_string(), "etl".to_string()],
                confidence: 0.7,
            },
            InferenceRule {
                name: "Documentation Generation".to_string(),
                condition: RuleCondition::HasTools(vec![
                    "web_search".to_string(),
                    "write_file".to_string(),
                ]),
                inferred_capabilities: vec!["documentation".to_string(), "research".to_string()],
                confidence: 0.8,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_inference() {
        let engine = CapabilityInferenceEngine::new();
        
        let agent = SubAgentDefinition {
            name: "test_agent".to_string(),
            description: "Test agent".to_string(),
            tools: vec![
                "git_ops".to_string(),
                "run_command".to_string(),
                "test_runner".to_string(),
            ],
            capabilities: vec![],
            custom_instructions: None,
            source_file: None,
        };
        
        let inferred = engine.infer_capabilities(&agent);
        
        // Should infer CI/CD capabilities
        assert!(inferred.iter().any(|c| c.capability == "continuous_integration"));
        assert!(inferred.iter().any(|c| c.capability == "continuous_deployment"));
        assert!(inferred.iter().any(|c| c.capability == "deployment"));
    }

    #[test]
    fn test_tool_recommendations() {
        let engine = CapabilityInferenceEngine::new();
        
        let recommendations = engine.recommend_tools_for_capability("continuous_integration");
        
        // Should recommend CI/CD tools
        assert!(recommendations.iter().any(|(tool, _)| tool == "git_ops"));
        assert!(recommendations.iter().any(|(tool, _)| tool == "test_runner"));
        assert!(recommendations.iter().any(|(tool, _)| tool == "run_command"));
    }

    #[test]
    fn test_pattern_learning() {
        let mut engine = CapabilityInferenceEngine::new();
        
        // Learn a successful pattern
        engine.learn_pattern(
            vec!["read_file".to_string(), "ast_parser".to_string()],
            vec!["code_review".to_string()],
            true,
            1500,
        );
        
        // Pattern should be recorded
        let pattern_key = "read_file+ast_parser";
        assert!(engine.learned_patterns.contains_key(pattern_key));
        
        let pattern = &engine.learned_patterns[pattern_key];
        assert_eq!(pattern.frequency, 1);
        assert_eq!(pattern.success_rate, 1.0);
        assert!(pattern.observed_capabilities.contains(&"code_review".to_string()));
    }
}