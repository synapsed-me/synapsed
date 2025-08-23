//! Parser for Claude's markdown agent definition files with observability
//! 
//! This module parses markdown files that define Claude sub-agents,
//! extracting tools, capabilities, instructions, and other metadata.
//! All parsing operations are observable through Substrates.

use crate::{
    dynamic_agents::SubAgentDefinition,
    Result, IntentError,
};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use regex::Regex;
use synapsed_substrates::{
    BasicSource, Source, Subject, Substrate,
    types::{Name, SubjectType, SubstratesResult},
};
use tokio::sync::RwLock;

/// Parser for agent definition markdown files with observability
pub struct AgentMarkdownParser {
    tool_aliases: HashMap<String, Vec<String>>,
    capability_patterns: Vec<CapabilityPattern>,
    // Substrates observability
    parse_events: Arc<BasicSource<ParseEvent>>,
    parse_metrics: Arc<RwLock<Vec<ParseMetric>>>,
}

/// Pattern for inferring capabilities from content
#[derive(Debug, Clone)]
pub struct CapabilityPattern {
    pub pattern: Regex,
    pub capability: String,
}

/// Parsed agent definition from markdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedAgentDefinition {
    pub name: String,
    pub description: String,
    pub role: Option<String>,
    pub tools: Vec<String>,
    pub capabilities: Vec<String>,
    pub instructions: Vec<String>,
    pub constraints: Vec<String>,
    pub examples: Vec<Example>,
    pub metadata: HashMap<String, String>,
}

/// Example usage in the agent definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    pub input: String,
    pub output: String,
    pub explanation: Option<String>,
}

/// Parse event for observability
#[derive(Debug, Clone)]
pub struct ParseEvent {
    pub event_type: ParseEventType,
    pub agent_name: String,
    pub details: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Type of parse event
#[derive(Debug, Clone)]
pub enum ParseEventType {
    Started,
    ToolsExtracted,
    CapabilitiesInferred,
    Completed,
    Failed,
}

/// Parse metric for tracking
#[derive(Debug, Clone)]
pub struct ParseMetric {
    pub agent_name: String,
    pub tools_found: usize,
    pub capabilities_inferred: usize,
    pub parse_time_ms: u64,
}

impl AgentMarkdownParser {
    pub fn new() -> Self {
        // Create Substrates observability components
        let event_subject = Subject::new(
            Name::from("agent-parser-events"),
            SubjectType::Source
        );
        let parse_events = Arc::new(BasicSource::new(event_subject));
        
        Self {
            tool_aliases: Self::create_tool_aliases(),
            capability_patterns: Self::create_capability_patterns(),
            parse_events,
            parse_metrics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Parse a markdown file into an agent definition with observability
    pub async fn parse_file(&self, path: &Path) -> Result<SubAgentDefinition> {
        let start = chrono::Utc::now();
        let agent_name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Emit parse started event
        self.emit_parse_event(ParseEventType::Started, &agent_name, "Starting parse").await;
        
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| {
                let _ = self.emit_parse_event(ParseEventType::Failed, &agent_name, &e.to_string());
                IntentError::Other(anyhow::anyhow!("Failed to read file: {}", e))
            })?;
        
        let parsed = self.parse_markdown(&content)?;
        
        // Emit tools extracted event
        self.emit_parse_event(
            ParseEventType::ToolsExtracted,
            &agent_name,
            &format!("Found {} tools", parsed.tools.len())
        ).await;
        
        // Emit capabilities inferred event
        self.emit_parse_event(
            ParseEventType::CapabilitiesInferred,
            &agent_name,
            &format!("Inferred {} capabilities", parsed.capabilities.len())
        ).await;
        
        let result = self.convert_to_agent_definition(parsed.clone(), Some(path.to_path_buf()));
        
        // Record metrics
        let duration = chrono::Utc::now().signed_duration_since(start);
        let metric = ParseMetric {
            agent_name: agent_name.clone(),
            tools_found: parsed.tools.len(),
            capabilities_inferred: parsed.capabilities.len(),
            parse_time_ms: duration.num_milliseconds() as u64,
        };
        
        let mut metrics = self.parse_metrics.write().await;
        metrics.push(metric);
        
        // Emit completion event
        self.emit_parse_event(ParseEventType::Completed, &agent_name, "Parse completed").await;
        
        Ok(result)
    }
    
    /// Emit a parse event through Substrates
    async fn emit_parse_event(&self, event_type: ParseEventType, agent_name: &str, details: &str) {
        let event = ParseEvent {
            event_type,
            agent_name: agent_name.to_string(),
            details: details.to_string(),
            timestamp: chrono::Utc::now(),
        };
        
        // Emit through the source (would need proper Subject implementation)
        // For now, just log it
        tracing::info!("Parse event: {:?}", event);
    }

    /// Parse markdown content
    pub fn parse_markdown(&self, content: &str) -> Result<ParsedAgentDefinition> {
        let mut parsed = ParsedAgentDefinition {
            name: String::new(),
            description: String::new(),
            role: None,
            tools: Vec::new(),
            capabilities: Vec::new(),
            instructions: Vec::new(),
            constraints: Vec::new(),
            examples: Vec::new(),
            metadata: HashMap::new(),
        };

        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // Parse headers
            if line.starts_with("# ") {
                // Main title is the agent name
                parsed.name = line[2..].trim().to_string();
            } else if line.starts_with("## ") {
                let section = line[3..].trim().to_lowercase();
                i += 1;
                
                match section.as_str() {
                    "description" | "overview" => {
                        parsed.description = self.parse_paragraph(&lines, &mut i);
                    },
                    "role" => {
                        parsed.role = Some(self.parse_paragraph(&lines, &mut i));
                    },
                    "tools" => {
                        parsed.tools = self.parse_list(&lines, &mut i);
                        // Expand tool aliases
                        parsed.tools = self.expand_tool_aliases(&parsed.tools);
                    },
                    "capabilities" | "abilities" => {
                        parsed.capabilities = self.parse_list(&lines, &mut i);
                    },
                    "instructions" | "guidelines" => {
                        parsed.instructions = self.parse_list(&lines, &mut i);
                    },
                    "constraints" | "restrictions" | "limitations" => {
                        parsed.constraints = self.parse_list(&lines, &mut i);
                    },
                    "examples" | "usage" => {
                        parsed.examples = self.parse_examples(&lines, &mut i);
                    },
                    _ => {
                        // Store as metadata
                        let content = self.parse_paragraph(&lines, &mut i);
                        parsed.metadata.insert(section, content);
                    }
                }
                continue;
            }

            i += 1;
        }

        // Infer capabilities from content if not explicitly provided
        if parsed.capabilities.is_empty() {
            parsed.capabilities = self.infer_capabilities(&content);
        }

        // Infer tools from instructions if needed
        if parsed.tools.is_empty() {
            parsed.tools = self.infer_tools_from_content(&content);
        }

        Ok(parsed)
    }

    /// Parse a paragraph of text
    fn parse_paragraph(&self, lines: &[&str], i: &mut usize) -> String {
        let mut paragraph = String::new();
        
        while *i < lines.len() {
            let line = lines[*i];
            
            // Stop at next header or empty line after content
            if line.starts_with("#") || (paragraph.len() > 0 && line.trim().is_empty()) {
                break;
            }
            
            if !line.trim().is_empty() {
                if !paragraph.is_empty() {
                    paragraph.push(' ');
                }
                paragraph.push_str(line.trim());
            }
            
            *i += 1;
        }
        
        paragraph
    }

    /// Parse a list (bullet points or numbered)
    fn parse_list(&self, lines: &[&str], i: &mut usize) -> Vec<String> {
        let mut items = Vec::new();
        
        while *i < lines.len() {
            let line = lines[*i].trim();
            
            // Stop at next header
            if line.starts_with("#") {
                break;
            }
            
            // Parse bullet points or numbered lists
            if line.starts_with("- ") || line.starts_with("* ") {
                items.push(line[2..].trim().to_string());
            } else if let Some(pos) = line.find(". ") {
                if line[..pos].parse::<i32>().is_ok() {
                    items.push(line[pos + 2..].trim().to_string());
                }
            } else if line.is_empty() && !items.is_empty() {
                // Empty line after list items means end of list
                break;
            }
            
            *i += 1;
        }
        
        items
    }

    /// Parse examples section
    fn parse_examples(&self, lines: &[&str], i: &mut usize) -> Vec<Example> {
        let mut examples = Vec::new();
        let mut current_example: Option<Example> = None;
        let mut in_code_block = false;
        let mut code_type = String::new();
        
        while *i < lines.len() {
            let line = lines[*i];
            
            if line.starts_with("#") {
                break;
            }
            
            if line.starts_with("```") {
                in_code_block = !in_code_block;
                if in_code_block {
                    code_type = line[3..].trim().to_string();
                } else {
                    code_type.clear();
                }
            } else if in_code_block {
                if let Some(ref mut ex) = current_example {
                    if code_type == "input" {
                        if !ex.input.is_empty() {
                            ex.input.push('\n');
                        }
                        ex.input.push_str(line);
                    } else if code_type == "output" {
                        if !ex.output.is_empty() {
                            ex.output.push('\n');
                        }
                        ex.output.push_str(line);
                    }
                }
            } else if line.starts_with("### ") {
                // New example
                if let Some(ex) = current_example.take() {
                    examples.push(ex);
                }
                current_example = Some(Example {
                    input: String::new(),
                    output: String::new(),
                    explanation: Some(line[4..].trim().to_string()),
                });
            }
            
            *i += 1;
        }
        
        if let Some(ex) = current_example {
            examples.push(ex);
        }
        
        examples
    }

    /// Expand tool aliases (e.g., "file_operations" -> ["read_file", "write_file"])
    fn expand_tool_aliases(&self, tools: &[String]) -> Vec<String> {
        let mut expanded = Vec::new();
        
        for tool in tools {
            if let Some(aliases) = self.tool_aliases.get(tool) {
                expanded.extend(aliases.clone());
            } else {
                expanded.push(tool.clone());
            }
        }
        
        expanded.sort();
        expanded.dedup();
        expanded
    }

    /// Infer capabilities from content
    fn infer_capabilities(&self, content: &str) -> Vec<String> {
        let mut capabilities = Vec::new();
        
        for pattern in &self.capability_patterns {
            if pattern.pattern.is_match(content) {
                capabilities.push(pattern.capability.clone());
            }
        }
        
        capabilities.sort();
        capabilities.dedup();
        capabilities
    }

    /// Infer tools from content mentions
    fn infer_tools_from_content(&self, content: &str) -> Vec<String> {
        let mut tools = Vec::new();
        
        // Common tool mentions
        let tool_keywords = vec![
            ("read", "read_file"),
            ("write", "write_file"),
            ("execute", "run_command"),
            ("search", "web_search"),
            ("git", "git_ops"),
            ("test", "test_runner"),
            ("debug", "debug_shell"),
            ("parse", "ast_parser"),
            ("api", "http_request"),
            ("database", "sql_query"),
        ];
        
        let content_lower = content.to_lowercase();
        for (keyword, tool) in tool_keywords {
            if content_lower.contains(keyword) {
                tools.push(tool.to_string());
            }
        }
        
        tools.sort();
        tools.dedup();
        tools
    }

    /// Convert parsed definition to SubAgentDefinition
    fn convert_to_agent_definition(
        &self,
        parsed: ParsedAgentDefinition,
        source_file: Option<PathBuf>,
    ) -> SubAgentDefinition {
        // Combine instructions and constraints
        let mut custom_instructions = String::new();
        
        if !parsed.instructions.is_empty() {
            custom_instructions.push_str("Instructions:\n");
            for instruction in &parsed.instructions {
                custom_instructions.push_str(&format!("- {}\n", instruction));
            }
        }
        
        if !parsed.constraints.is_empty() {
            custom_instructions.push_str("\nConstraints:\n");
            for constraint in &parsed.constraints {
                custom_instructions.push_str(&format!("- {}\n", constraint));
            }
        }
        
        SubAgentDefinition {
            name: parsed.name,
            description: parsed.description,
            tools: parsed.tools,
            capabilities: parsed.capabilities,
            custom_instructions: if custom_instructions.is_empty() {
                None
            } else {
                Some(custom_instructions)
            },
            source_file,
        }
    }

    /// Create tool aliases mapping
    fn create_tool_aliases() -> HashMap<String, Vec<String>> {
        let mut aliases = HashMap::new();
        
        aliases.insert(
            "file_operations".to_string(),
            vec!["read_file".to_string(), "write_file".to_string(), "str_replace".to_string()],
        );
        
        aliases.insert(
            "code_tools".to_string(),
            vec!["ast_parser".to_string(), "debug_shell".to_string(), "test_runner".to_string()],
        );
        
        aliases.insert(
            "network_tools".to_string(),
            vec!["web_search".to_string(), "http_request".to_string()],
        );
        
        aliases.insert(
            "development_tools".to_string(),
            vec!["git_ops".to_string(), "run_command".to_string(), "test_runner".to_string()],
        );
        
        aliases
    }

    /// Create capability inference patterns
    fn create_capability_patterns() -> Vec<CapabilityPattern> {
        vec![
            CapabilityPattern {
                pattern: Regex::new(r"(?i)(analyz|analy[sz]e|review)").unwrap(),
                capability: "analysis".to_string(),
            },
            CapabilityPattern {
                pattern: Regex::new(r"(?i)(test|verify|validate)").unwrap(),
                capability: "testing".to_string(),
            },
            CapabilityPattern {
                pattern: Regex::new(r"(?i)(debug|troubleshoot|fix)").unwrap(),
                capability: "debugging".to_string(),
            },
            CapabilityPattern {
                pattern: Regex::new(r"(?i)(deploy|release|publish)").unwrap(),
                capability: "deployment".to_string(),
            },
            CapabilityPattern {
                pattern: Regex::new(r"(?i)(design|architect|plan)").unwrap(),
                capability: "design".to_string(),
            },
            CapabilityPattern {
                pattern: Regex::new(r"(?i)(security|vulnerabilit|threat)").unwrap(),
                capability: "security".to_string(),
            },
            CapabilityPattern {
                pattern: Regex::new(r"(?i)(document|write|report)").unwrap(),
                capability: "documentation".to_string(),
            },
            CapabilityPattern {
                pattern: Regex::new(r"(?i)(refactor|optimize|improve)").unwrap(),
                capability: "optimization".to_string(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_markdown() {
        let parser = AgentMarkdownParser::new();
        
        let markdown = r#"
# Code Reviewer

## Description
Reviews code for quality and security issues.

## Tools
- read_file
- ast_parser
- web_search

## Capabilities
- code_analysis
- security_scanning

## Instructions
- Focus on security vulnerabilities
- Check for code style violations
- Suggest performance improvements
"#;
        
        let parsed = parser.parse_markdown(markdown).unwrap();
        
        assert_eq!(parsed.name, "Code Reviewer");
        assert_eq!(parsed.description, "Reviews code for quality and security issues.");
        assert_eq!(parsed.tools.len(), 3);
        assert!(parsed.tools.contains(&"read_file".to_string()));
        assert_eq!(parsed.capabilities.len(), 2);
        assert!(parsed.capabilities.contains(&"code_analysis".to_string()));
        assert_eq!(parsed.instructions.len(), 3);
    }

    #[test]
    fn test_tool_alias_expansion() {
        let parser = AgentMarkdownParser::new();
        
        let markdown = r#"
# Developer Assistant

## Tools
- file_operations
- git_ops
"#;
        
        let parsed = parser.parse_markdown(markdown).unwrap();
        
        // file_operations should expand to read_file, write_file, str_replace
        assert!(parsed.tools.contains(&"read_file".to_string()));
        assert!(parsed.tools.contains(&"write_file".to_string()));
        assert!(parsed.tools.contains(&"str_replace".to_string()));
        assert!(parsed.tools.contains(&"git_ops".to_string()));
    }

    #[test]
    fn test_capability_inference() {
        let parser = AgentMarkdownParser::new();
        
        let markdown = r#"
# Security Analyst

This agent analyzes code for security vulnerabilities and helps debug issues.
It can test applications and document findings.
"#;
        
        let parsed = parser.parse_markdown(markdown).unwrap();
        
        // Should infer capabilities from content
        assert!(parsed.capabilities.contains(&"analysis".to_string()));
        assert!(parsed.capabilities.contains(&"security".to_string()));
        assert!(parsed.capabilities.contains(&"debugging".to_string()));
        assert!(parsed.capabilities.contains(&"testing".to_string()));
        assert!(parsed.capabilities.contains(&"documentation".to_string()));
    }

    #[test]
    fn test_tool_inference() {
        let parser = AgentMarkdownParser::new();
        
        let markdown = r#"
# Data Processor

This agent reads files, executes Python scripts, and searches the web for data.
"#;
        
        let parsed = parser.parse_markdown(markdown).unwrap();
        
        // Should infer tools from content
        assert!(parsed.tools.contains(&"read_file".to_string()));
        assert!(parsed.tools.contains(&"run_command".to_string()));
        assert!(parsed.tools.contains(&"web_search".to_string()));
    }
}