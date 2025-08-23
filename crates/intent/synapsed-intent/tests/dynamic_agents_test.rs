//! Integration tests for the dynamic agent system

use synapsed_intent::{
    dynamic_agents::{
        DynamicContextGenerator, SubAgentDefinition, RiskLevel, SecurityLevel,
        WorkspaceZones, Zone, Operation,
    },
    tool_registry::{ToolRegistry, CustomTool, ToolImplementation},
    permission_negotiation::{
        PermissionNegotiator, PermissionRequest, RequestedPermissions, Priority,
        Decision, EvaluationContext, ResourceUsage,
    },
    Result,
};
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;
use chrono::Utc;

#[tokio::test]
async fn test_dynamic_context_generation() {
    let generator = DynamicContextGenerator::new();
    
    // Define a code reviewer agent
    let agent_def = SubAgentDefinition {
        name: "code_reviewer".to_string(),
        description: "Reviews code for quality and security".to_string(),
        tools: vec!["read_file".to_string(), "ast_parser".to_string()],
        capabilities: vec!["code_analysis".to_string()],
        custom_instructions: Some("Focus on security vulnerabilities".to_string()),
        source_file: None,
    };
    
    // Generate context with medium trust
    let context_bounds = generator
        .generate_context_from_agent_definition(&agent_def, 0.6)
        .await
        .unwrap();
    
    // Verify appropriate restrictions
    assert!(context_bounds.allowed_commands.contains(&"cat".to_string()));
    assert!(context_bounds.allowed_paths.iter().any(|p| p.contains("workspace")));
    assert!(context_bounds.max_memory_bytes.is_some());
    assert!(context_bounds.max_cpu_seconds.is_some());
}

#[tokio::test]
async fn test_risk_based_context_restrictions() {
    let generator = DynamicContextGenerator::new();
    
    // High-risk agent with debug capabilities
    let high_risk_agent = SubAgentDefinition {
        name: "debugger".to_string(),
        description: "Debug and fix runtime issues".to_string(),
        tools: vec!["debug_shell".to_string(), "run_command".to_string()],
        capabilities: vec!["execution".to_string()],
        custom_instructions: None,
        source_file: None,
    };
    
    // Low trust user
    let low_trust_context = generator
        .generate_context_from_agent_definition(&high_risk_agent, 0.3)
        .await
        .unwrap();
    
    // Should have very restrictive bounds
    assert!(low_trust_context.max_memory_bytes.unwrap() <= 10 * 1024 * 1024);
    assert!(low_trust_context.max_cpu_seconds.unwrap() <= 10);
    assert!(low_trust_context.allowed_endpoints.is_empty());
    
    // High trust user
    let high_trust_context = generator
        .generate_context_from_agent_definition(&high_risk_agent, 0.9)
        .await
        .unwrap();
    
    // Should have more relaxed bounds
    assert!(high_trust_context.max_memory_bytes.unwrap() > low_trust_context.max_memory_bytes.unwrap());
    assert!(high_trust_context.max_cpu_seconds.unwrap() > low_trust_context.max_cpu_seconds.unwrap());
}

#[tokio::test]
async fn test_custom_tool_registration() {
    let registry = ToolRegistry::new();
    
    // Create a custom linting tool
    let custom_tool = CustomTool {
        name: "custom_linter".to_string(),
        description: "Custom code linter".to_string(),
        implementation: ToolImplementation::Command {
            command: "eslint".to_string(),
            args: vec!["--fix".to_string()],
            env: HashMap::new(),
        },
        security_profile: synapsed_intent::dynamic_agents::ToolSecurityProfile {
            tool_name: "custom_linter".to_string(),
            required_commands: vec!["eslint".to_string()],
            required_paths: vec!["${workspace}/**".to_string()],
            required_endpoints: vec![],
            risk_level: RiskLevel::Low,
            verification_requirements: vec![],
            resource_requirements: Default::default(),
        },
        created_by: "test_user".to_string(),
        created_at: Utc::now(),
        approved: true,
        approval_reason: Some("Safe linting tool".to_string()),
    };
    
    // Register with sufficient trust
    let result = registry.register_custom_tool(custom_tool.clone(), 0.6).await;
    assert!(result.is_ok());
    
    // Verify tool is available
    let profile = registry.get_tool_profile("custom_linter").await;
    assert!(profile.is_some());
    
    // Try to register high-risk tool with low trust
    let dangerous_tool = CustomTool {
        name: "dangerous_tool".to_string(),
        description: "Dangerous tool".to_string(),
        implementation: ToolImplementation::Command {
            command: "rm".to_string(),
            args: vec!["-rf".to_string()],
            env: HashMap::new(),
        },
        security_profile: synapsed_intent::dynamic_agents::ToolSecurityProfile {
            tool_name: "dangerous_tool".to_string(),
            required_commands: vec!["rm".to_string()],
            required_paths: vec!["/**".to_string()],
            required_endpoints: vec![],
            risk_level: RiskLevel::Critical,
            verification_requirements: vec![],
            resource_requirements: Default::default(),
        },
        created_by: "test_user".to_string(),
        created_at: Utc::now(),
        approved: false,
        approval_reason: None,
    };
    
    // Should fail with low trust
    let result = registry.register_custom_tool(dangerous_tool, 0.4).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_permission_negotiation() {
    let (tx, mut rx) = mpsc::channel(10);
    let negotiator = PermissionNegotiator::new(tx);
    
    // Create a permission request
    let request = PermissionRequest {
        request_id: Uuid::new_v4(),
        agent_id: "test_agent".to_string(),
        requested_permissions: RequestedPermissions {
            additional_commands: vec!["npm".to_string(), "node".to_string()],
            additional_paths: vec!["/workspace/node_modules".to_string()],
            additional_endpoints: vec![],
            increased_memory_mb: Some(200),
            increased_cpu_seconds: Some(120),
            network_access: false,
            spawn_processes: true,
        },
        justification: "Need to run Node.js tests".to_string(),
        context: HashMap::new(),
        duration: Some(chrono::Duration::minutes(30)),
        priority: Priority::Normal,
        timestamp: Utc::now(),
    };
    
    // Create evaluation context with high trust
    let context = EvaluationContext {
        agent_trust_score: 0.85,
        current_risk_level: RiskLevel::Medium,
        security_zone: SecurityLevel::Development,
        recent_violations: vec![],
        resource_usage: ResourceUsage {
            memory_used_mb: 100,
            cpu_percent: 25.0,
            disk_io_kbps: 100,
            network_io_kbps: 0,
        },
        parent_context: None,
    };
    
    // Request permissions
    let response = negotiator.request_permissions(request.clone(), &context).await.unwrap();
    
    // Should be approved with high trust
    assert_eq!(response.decision, Decision::Approved);
    assert!(response.granted_permissions.is_some());
    
    // Check notification was sent
    if let Some(notification) = rx.recv().await {
        match notification {
            synapsed_intent::permission_negotiation::PermissionNotification::RequestReceived(req) => {
                assert_eq!(req.request_id, request.request_id);
            },
            _ => panic!("Unexpected notification type"),
        }
    }
}

#[tokio::test]
async fn test_permission_denial_with_alternatives() {
    let (tx, _rx) = mpsc::channel(10);
    let negotiator = PermissionNegotiator::new(tx);
    
    // Request dangerous permissions with low trust
    let request = PermissionRequest {
        request_id: Uuid::new_v4(),
        agent_id: "untrusted_agent".to_string(),
        requested_permissions: RequestedPermissions {
            additional_commands: vec!["sudo".to_string(), "rm".to_string()],
            additional_paths: vec!["/etc".to_string(), "/sys".to_string()],
            additional_endpoints: vec![],
            increased_memory_mb: Some(1000),
            increased_cpu_seconds: Some(600),
            network_access: true,
            spawn_processes: true,
        },
        justification: "System maintenance".to_string(),
        context: HashMap::new(),
        duration: None,
        priority: Priority::Low,
        timestamp: Utc::now(),
    };
    
    // Low trust context
    let context = EvaluationContext {
        agent_trust_score: 0.2,
        current_risk_level: RiskLevel::High,
        security_zone: SecurityLevel::Sandbox,
        recent_violations: vec!["Attempted unauthorized access".to_string()],
        resource_usage: ResourceUsage {
            memory_used_mb: 500,
            cpu_percent: 60.0,
            disk_io_kbps: 1000,
            network_io_kbps: 500,
        },
        parent_context: None,
    };
    
    // Request should be denied
    let response = negotiator.request_permissions(request, &context).await.unwrap();
    
    assert_eq!(response.decision, Decision::Denied);
    assert!(response.granted_permissions.is_none());
    assert!(!response.alternatives.is_empty());
    
    // Check that alternatives are safer
    for alt in &response.alternatives {
        assert!(alt.modified_permissions.additional_commands.is_empty() || 
                !alt.modified_permissions.additional_commands.contains(&"sudo".to_string()));
    }
}

#[tokio::test]
async fn test_workspace_zones() {
    let mut zones = WorkspaceZones::create_default();
    
    // Assign agent to sandbox
    let result = zones.assign_agent_to_zone("agent1", "sandbox");
    assert!(result.is_ok());
    
    // Verify zone assignment
    let sandbox = zones.zones.get("sandbox").unwrap();
    assert!(sandbox.current_agents.contains(&"agent1".to_string()));
    
    // Try to exceed zone capacity
    for i in 2..=10 {
        let _ = zones.assign_agent_to_zone(&format!("agent{}", i), "sandbox");
    }
    
    // This should fail (sandbox max is 10)
    let result = zones.assign_agent_to_zone("agent11", "sandbox");
    assert!(result.is_err());
    
    // Production zone should be most restrictive
    let prod_zone = zones.zones.get("production").unwrap();
    assert_eq!(prod_zone.allowed_operations, vec![Operation::Read]);
    assert_eq!(prod_zone.max_agents, 2);
}

#[tokio::test]
async fn test_multi_agent_collaboration() {
    let generator = DynamicContextGenerator::new();
    
    // Define a team of agents for building an application
    let agents = vec![
        SubAgentDefinition {
            name: "system_architect".to_string(),
            description: "Designs system architecture".to_string(),
            tools: vec!["read_file".to_string(), "write_file".to_string()],
            capabilities: vec!["design".to_string()],
            custom_instructions: Some("Create scalable architectures".to_string()),
            source_file: None,
        },
        SubAgentDefinition {
            name: "analyst".to_string(),
            description: "Analyzes requirements".to_string(),
            tools: vec!["read_file".to_string(), "web_search".to_string()],
            capabilities: vec!["analysis".to_string()],
            custom_instructions: Some("Focus on user needs".to_string()),
            source_file: None,
        },
        SubAgentDefinition {
            name: "coder".to_string(),
            description: "Implements features".to_string(),
            tools: vec!["read_file".to_string(), "write_file".to_string(), "run_command".to_string()],
            capabilities: vec!["coding".to_string()],
            custom_instructions: Some("Write clean, tested code".to_string()),
            source_file: None,
        },
        SubAgentDefinition {
            name: "tester".to_string(),
            description: "Tests the application".to_string(),
            tools: vec!["test_runner".to_string(), "debug_shell".to_string()],
            capabilities: vec!["testing".to_string()],
            custom_instructions: Some("Ensure comprehensive test coverage".to_string()),
            source_file: None,
        },
        SubAgentDefinition {
            name: "cicd_expert".to_string(),
            description: "Sets up CI/CD pipelines".to_string(),
            tools: vec!["git_ops".to_string(), "run_command".to_string()],
            capabilities: vec!["deployment".to_string()],
            custom_instructions: Some("Automate everything".to_string()),
            source_file: None,
        },
    ];
    
    // Generate contexts for each agent with varying trust levels
    let trust_levels = vec![0.9, 0.8, 0.7, 0.6, 0.8];
    
    for (agent, trust) in agents.iter().zip(trust_levels.iter()) {
        let context = generator
            .generate_context_from_agent_definition(agent, *trust)
            .await
            .unwrap();
        
        // Verify each agent gets appropriate permissions
        match agent.name.as_str() {
            "system_architect" => {
                assert!(context.allowed_commands.contains(&"echo".to_string()));
                assert!(context.allowed_paths.iter().any(|p| p.contains("workspace")));
            },
            "analyst" => {
                assert!(context.allowed_endpoints.len() > 0);
            },
            "coder" => {
                assert!(context.allowed_commands.len() > 2);
            },
            "tester" => {
                assert!(context.allowed_commands.iter().any(|c| c.contains("test") || c.contains("pytest")));
            },
            "cicd_expert" => {
                assert!(context.allowed_commands.contains(&"git".to_string()));
            },
            _ => {},
        }
    }
}

#[tokio::test]
async fn test_tool_permission_checking() {
    let registry = ToolRegistry::new();
    
    // Test trust-based tool access
    assert!(!registry.is_tool_allowed("debug_shell", 0.2).await);
    assert!(registry.is_tool_allowed("debug_shell", 0.8).await);
    
    assert!(registry.is_tool_allowed("read_file", 0.4).await);
    assert!(registry.is_tool_allowed("read_file", 0.9).await);
}

#[tokio::test]
async fn test_composite_permissions() {
    let registry = ToolRegistry::new();
    
    // Calculate permissions for multiple tools
    let tools = vec![
        "read_file".to_string(),
        "write_file".to_string(),
        "run_command".to_string(),
    ];
    
    let composite = registry.calculate_composite_permissions(&tools).await.unwrap();
    
    // Should have combined permissions
    assert!(!composite.filesystem.read_paths.is_empty());
    assert!(!composite.filesystem.write_paths.is_empty());
    assert!(composite.process.can_spawn);
}