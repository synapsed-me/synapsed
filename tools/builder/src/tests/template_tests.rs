//! Unit tests for Templates
//!
//! Intent: Test pre-built application templates
//! Verification: All templates are valid and can be instantiated

use crate::templates::{Templates, TemplateInfo};
use crate::builder::{StorageBackend, ObservabilityLevel, NetworkType};

#[test]
fn test_template_list() {
    let templates = Templates::list();
    
    // Should have at least the core templates
    assert!(templates.len() >= 10);
    
    // Check for specific templates
    let template_names: Vec<String> = templates.iter().map(|t| t.name.clone()).collect();
    assert!(template_names.contains(&"verified-ai-agent".to_string()));
    assert!(template_names.contains(&"payment-system".to_string()));
    assert!(template_names.contains(&"swarm-coordinator".to_string()));
    assert!(template_names.contains(&"observable-service".to_string()));
    assert!(template_names.contains(&"edge-compute".to_string()));
    assert!(template_names.contains(&"secure-vault".to_string()));
    assert!(template_names.contains(&"mcp-server".to_string()));
    assert!(template_names.contains(&"neural-pipeline".to_string()));
    assert!(template_names.contains(&"dev-sandbox".to_string()));
    assert!(template_names.contains(&"minimal".to_string()));
}

#[test]
fn test_template_get() {
    // Test getting existing templates
    assert!(Templates::get("verified-ai-agent").is_some());
    assert!(Templates::get("payment-system").is_some());
    assert!(Templates::get("minimal").is_some());
    
    // Test getting non-existent template
    assert!(Templates::get("non-existent").is_none());
}

#[test]
fn test_verified_ai_agent_template() {
    let builder = Templates::verified_ai_agent();
    
    assert_eq!(builder.config.name, "verified-ai-agent");
    assert!(builder.config.description.contains("AI agent"));
    assert!(builder.components.contains(&"synapsed-intent".to_string()));
    assert!(builder.components.contains(&"synapsed-verify".to_string()));
    assert!(builder.components.contains(&"synapsed-storage".to_string()));
    assert!(builder.configurations.contains_key("synapsed-intent"));
    assert!(builder.environment.contains_key("RUST_LOG"));
}

#[test]
fn test_payment_system_template() {
    let builder = Templates::payment_system();
    
    assert_eq!(builder.config.name, "payment-system");
    assert!(builder.components.contains(&"synapsed-payments".to_string()));
    assert!(builder.components.contains(&"synapsed-identity".to_string()));
    assert!(builder.components.contains(&"synapsed-crypto".to_string()));
    assert!(builder.configurations.contains_key("synapsed-payments"));
    assert!(builder.configurations.contains_key("synapsed-identity"));
}

#[test]
fn test_swarm_coordinator_template() {
    let builder = Templates::swarm_coordinator();
    
    assert_eq!(builder.config.name, "swarm-coordinator");
    assert!(builder.components.contains(&"synapsed-swarm".to_string()));
    assert!(builder.components.contains(&"synapsed-consensus".to_string()));
    assert!(builder.components.contains(&"synapsed-crdt".to_string()));
    assert!(builder.connections.len() >= 2);
    assert!(builder.configurations.contains_key("synapsed-consensus"));
}

#[test]
fn test_observable_service_template() {
    let builder = Templates::observable_service();
    
    assert_eq!(builder.config.name, "observable-service");
    assert!(builder.components.contains(&"synapsed-core".to_string()));
    assert!(builder.components.contains(&"synapsed-substrates".to_string()));
    assert!(builder.components.contains(&"synapsed-serventis".to_string()));
    assert!(builder.components.contains(&"synapsed-monitor".to_string()));
    assert!(builder.configurations.contains_key("synapsed-substrates"));
    assert!(builder.configurations.contains_key("synapsed-monitor"));
}

#[test]
fn test_edge_compute_template() {
    let builder = Templates::edge_compute();
    
    assert_eq!(builder.config.name, "edge-compute");
    assert!(builder.components.contains(&"synapsed-wasm".to_string()));
    assert!(builder.components.contains(&"synapsed-gpu".to_string()));
    assert!(builder.configurations.contains_key("synapsed-wasm"));
    assert!(builder.configurations.contains_key("synapsed-gpu"));
}

#[test]
fn test_secure_vault_template() {
    let builder = Templates::secure_vault();
    
    assert_eq!(builder.config.name, "secure-vault");
    assert!(builder.components.contains(&"synapsed-storage".to_string()));
    assert!(builder.components.contains(&"synapsed-crypto".to_string()));
    assert!(builder.components.contains(&"synapsed-identity".to_string()));
    assert!(builder.components.contains(&"synapsed-safety".to_string()));
    assert!(builder.configurations.contains_key("synapsed-crypto"));
    assert!(builder.configurations.contains_key("synapsed-storage"));
}

#[test]
fn test_mcp_server_template() {
    let builder = Templates::mcp_server();
    
    assert_eq!(builder.config.name, "mcp-server");
    assert!(builder.config.description.contains("Model Context Protocol"));
    assert!(builder.components.contains(&"synapsed-mcp".to_string()));
    assert!(builder.components.contains(&"synapsed-intent".to_string()));
    assert!(builder.components.contains(&"synapsed-verify".to_string()));
    assert!(builder.configurations.contains_key("synapsed-mcp"));
    assert!(builder.environment.contains_key("SYNAPSED_MCP_PORT"));
}

#[test]
fn test_neural_pipeline_template() {
    let builder = Templates::neural_pipeline();
    
    assert_eq!(builder.config.name, "neural-pipeline");
    assert!(builder.components.contains(&"synapsed-neural-core".to_string()));
    assert!(builder.components.contains(&"synapsed-gpu".to_string()));
    assert!(builder.configurations.contains_key("synapsed-neural-core"));
}

#[test]
fn test_dev_sandbox_template() {
    let builder = Templates::dev_sandbox();
    
    assert_eq!(builder.config.name, "dev-sandbox");
    assert!(builder.components.contains(&"synapsed-core".to_string()));
    assert!(builder.components.contains(&"synapsed-intent".to_string()));
    assert!(builder.components.contains(&"synapsed-verify".to_string()));
    assert!(!builder.validations_enabled); // Validations should be skipped
    assert_eq!(builder.environment["RUST_LOG"], "debug");
    assert_eq!(builder.environment["RUST_BACKTRACE"], "1");
}

#[test]
fn test_minimal_template() {
    let builder = Templates::minimal();
    
    assert_eq!(builder.config.name, "minimal-app");
    assert!(builder.components.contains(&"synapsed-core".to_string()));
    assert!(builder.components.contains(&"synapsed-storage".to_string()));
    assert_eq!(builder.components.len(), 2); // Should only have these two
}

#[test]
fn test_template_info() {
    let info = TemplateInfo {
        name: "test-template".to_string(),
        description: "Test template".to_string(),
        components: vec!["comp1".to_string(), "comp2".to_string()],
        tags: vec!["test".to_string(), "example".to_string()],
    };
    
    assert_eq!(info.name, "test-template");
    assert_eq!(info.components.len(), 2);
    assert_eq!(info.tags.len(), 2);
}

#[test]
fn test_template_tag_matching() {
    let info = TemplateInfo {
        name: "test".to_string(),
        description: "Test".to_string(),
        components: vec![],
        tags: vec!["ai".to_string(), "verification".to_string(), "observable".to_string()],
    };
    
    // Should match if any tag matches
    assert!(info.matches_tags(&["ai".to_string()]));
    assert!(info.matches_tags(&["verification".to_string()]));
    assert!(info.matches_tags(&["observable".to_string()]));
    assert!(info.matches_tags(&["other".to_string(), "ai".to_string()]));
    
    // Should not match if no tags match
    assert!(!info.matches_tags(&["payment".to_string()]));
    assert!(!info.matches_tags(&["network".to_string(), "storage".to_string()]));
}

#[test]
fn test_find_templates_by_tag() {
    let templates = Templates::list();
    
    // Find AI-related templates
    let ai_templates: Vec<_> = templates.iter()
        .filter(|t| t.matches_tags(&["ai".to_string()]))
        .collect();
    assert!(ai_templates.len() >= 3); // At least verified-ai-agent, mcp-server, neural-pipeline
    
    // Find security-related templates
    let security_templates: Vec<_> = templates.iter()
        .filter(|t| t.matches_tags(&["security".to_string()]))
        .collect();
    assert!(security_templates.len() >= 2); // At least payment-system, secure-vault
    
    // Find minimal templates
    let minimal_templates: Vec<_> = templates.iter()
        .filter(|t| t.matches_tags(&["minimal".to_string()]))
        .collect();
    assert!(minimal_templates.len() >= 1); // At least minimal template
}

#[test]
fn test_template_configurations() {
    // Test that templates have proper configurations
    let ai_agent = Templates::verified_ai_agent();
    let config = &ai_agent.configurations["synapsed-intent"];
    assert_eq!(config["max_depth"], 5);
    assert_eq!(config["timeout_ms"], 30000);
    assert_eq!(config["strict_mode"], true);
    
    let payment = Templates::payment_system();
    let payment_config = &payment.configurations["synapsed-payments"];
    assert!(payment_config["supported_currencies"].is_array());
    assert_eq!(payment_config["risk_threshold"], 70);
    
    let swarm = Templates::swarm_coordinator();
    let consensus_config = &swarm.configurations["synapsed-consensus"];
    assert_eq!(consensus_config["consensus_type"], "hotstuff");
    assert_eq!(consensus_config["committee_size"], 7);
}

#[test]
fn test_template_completeness() {
    // Ensure all templates have required fields
    for template_info in Templates::list() {
        assert!(!template_info.name.is_empty());
        assert!(!template_info.description.is_empty());
        assert!(!template_info.components.is_empty());
        assert!(!template_info.tags.is_empty());
        
        // Get the actual template and verify it can be created
        if let Some(template) = Templates::get(&template_info.name) {
            assert_eq!(template.config.name, template_info.name);
            assert!(!template.config.description.is_empty());
            assert!(!template.components.is_empty());
        }
    }
}