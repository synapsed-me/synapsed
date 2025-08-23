# Dynamic Agent System Documentation

## Overview

The Dynamic Agent System in synapsed-intent enables secure, flexible creation and management of AI sub-agents with user-defined capabilities while maintaining strict security boundaries. This system specifically addresses Claude's ability to define sub-agents through markdown files and the Claude UI.

## Core Components

### 1. Agent Markdown Parser

The `AgentMarkdownParser` reads Claude's markdown agent definition files and extracts structured information:

```rust
let parser = AgentMarkdownParser::new();
let agent_def = parser.parse_file("agents/code_reviewer.md").await?;
```

Features:
- Parses standard markdown sections (Description, Tools, Capabilities, Instructions)
- Expands tool aliases (e.g., "file_operations" → ["read_file", "write_file"])
- Infers capabilities from content when not explicitly stated
- Extracts examples and constraints

### 2. Dynamic Context Generator

The `DynamicContextGenerator` creates secure execution contexts for user-defined agents based on:
- Agent tool requirements
- User trust levels
- Risk assessment
- Security zones

```rust
let generator = DynamicContextGenerator::new();
let context = generator.generate_context_from_agent_definition(
    &agent_def,
    user_trust_level
).await?;
```

### 3. Capability Inference Engine

The `CapabilityInferenceEngine` understands tool relationships and infers agent capabilities:

```rust
let engine = CapabilityInferenceEngine::new();
let inferred = engine.infer_capabilities(&agent_def);
```

Features:
- Tool relationship graph (requires, complements, substitutes)
- Inference rules for capability detection
- Learning from successful patterns
- Tool recommendations for desired capabilities

### 4. Tool Registry

Manages available tools with detailed security profiles:

```rust
pub struct ToolSecurityProfile {
    pub tool_name: String,
    pub required_commands: Vec<String>,
    pub required_paths: Vec<String>,
    pub required_endpoints: Vec<String>,
    pub risk_level: RiskLevel,
    pub verification_requirements: Vec<VerificationRequirement>,
    pub resource_requirements: ResourceRequirements,
}
```

Standard Claude tools included:
- `read_file` - Low risk file reading
- `write_file` - Medium risk file writing  
- `run_command` - High risk command execution
- `web_search` - Low risk web searches
- `str_replace` - Medium risk file modification
- `debug_shell` - High risk debugging
- `ast_parser` - Low risk code analysis
- `test_runner` - Medium risk test execution
- `git_ops` - Medium risk git operations

### 5. Tool Discovery System

The `ToolDiscoverySystem` monitors agent execution and discovers new tools dynamically:

```rust
let discovery = ToolDiscoverySystem::new(tool_registry);
let decision = discovery.handle_tool_attempt(
    agent_id,
    "new_tool",
    command,
    args,
    task_context,
    agent_trust
).await?;
```

Features:
- Automatic tool discovery during execution
- Risk assessment for unknown tools
- Policy-based approval/denial
- Usage statistics tracking
- Tool suggestions for capabilities

### 6. Agent Profiling System

The `AgentProfilingSystem` builds behavioral profiles from execution patterns:

```rust
let profiling = AgentProfilingSystem::new();
let profile = profiling.profile_agent(&agent_def).await?;
```

Features:
- Tracks declared vs. actually used tools
- Identifies execution patterns
- Detects anomalies in behavior
- Provides adaptation suggestions
- Trust score management

### 7. Permission Negotiation

Allows agents to request additional permissions with justification:

```rust
let request = PermissionRequest {
    requested_permissions: RequestedPermissions {
        additional_commands: vec!["npm".to_string()],
        additional_paths: vec!["/workspace/node_modules".to_string()],
        increased_memory_mb: Some(200),
        // ...
    },
    justification: "Need to run Node.js tests".to_string(),
    // ...
};

let response = negotiator.request_permissions(request, &context).await?;
```

### 8. Security Zones

Workspace isolation with different security levels:

| Zone | Security Level | Allowed Operations | Max Agents |
|------|---------------|-------------------|------------|
| Sandbox | Highest | Read, Write only | 10 |
| Development | Moderate | Read, Write, Execute, Create | 5 |
| Staging | Low | Read, Write, Execute, Network | 3 |
| Production | Minimal | Read only | 2 |

## Risk-Based Permission Model

### Risk Levels

1. **Minimal** - Read-only operations on non-sensitive data
2. **Low** - Basic file operations in workspace
3. **Medium** - File modifications, limited execution
4. **High** - Command execution, debugging
5. **Critical** - System modifications, sudo access

### Trust-Based Scaling

| Trust Level | Risk Tolerance | Available Tools | Resource Limits |
|------------|----------------|-----------------|-----------------|
| 0.0-0.3 | Minimal | Read-only | 10MB RAM, 10s CPU |
| 0.3-0.6 | Low-Medium | File ops, analysis | 100MB RAM, 60s CPU |
| 0.6-0.9 | High | Execution, testing | 500MB RAM, 300s CPU |
| 0.9-1.0 | Critical | All tools | 1GB RAM, 600s CPU |

## Enhanced Agent Discovery and Integration

### Automatic Capability Discovery

The system automatically discovers agent capabilities through multiple mechanisms:

1. **Markdown Parsing** - Reads Claude's agent definition files
2. **Tool Inference** - Infers capabilities from tool combinations
3. **Pattern Learning** - Learns from successful execution patterns
4. **Behavioral Profiling** - Adapts based on actual usage

### How It Works

```rust
// 1. Parse agent definition from markdown
let parser = AgentMarkdownParser::new();
let parsed = parser.parse_file("agents/code_reviewer.md").await?;

// 2. Infer capabilities from tools
let inference_engine = CapabilityInferenceEngine::new();
let capabilities = inference_engine.infer_capabilities(&agent_def);
// Returns: ["code_analysis", "security_scanning", "documentation"]

// 3. Discover new tools during execution
let discovery = ToolDiscoverySystem::new(tool_registry);
discovery.handle_tool_attempt(
    agent_id, "eslint", "eslint --fix", &["src/*.js"],
    "linting code", agent_trust
).await?;

// 4. Build behavioral profile
let profiling = AgentProfilingSystem::new();
profiling.record_tool_usage(agent_id, "eslint", true, 1500);
let suggestions = profiling.get_adaptation_suggestions(agent_id).await;
// Returns: ["Add eslint to declared tools", "Remove unused ast_parser"]
```

## User-Defined Agent Integration

### Agent Definition Format

```markdown
# Code Reviewer

## Description
Reviews code for quality and security issues.

## Tools
- file_operations  # Expands to: read_file, write_file, str_replace
- ast_parser
- web_search

## Capabilities
- code_analysis
- security_scanning

## Instructions
- Focus on security vulnerabilities
- Check for code style violations
- Suggest performance improvements

## Examples
### Input
```python
def process(data):
    eval(data)  # Security issue!
```
### Output
Security vulnerability detected: eval() on user input
```

### Context Generation Process

1. **Parse Agent Definition** - Extract tools, capabilities, instructions
2. **Risk Assessment** - Analyze requested tools and permissions
3. **Trust Evaluation** - Check user's trust score
4. **Zone Assignment** - Place agent in appropriate security zone
5. **Context Creation** - Generate bounded execution context
6. **Verification Setup** - Configure verification requirements

## Multi-Agent Collaboration

### Team Composition Example

```rust
let team = vec![
    SubAgentDefinition {
        name: "system_architect",
        tools: vec!["read_file", "write_file"],
        capabilities: vec!["design"],
        // ...
    },
    SubAgentDefinition {
        name: "analyst",
        tools: vec!["read_file", "web_search"],
        capabilities: vec!["analysis"],
        // ...
    },
    SubAgentDefinition {
        name: "coder",
        tools: vec!["read_file", "write_file", "run_command"],
        capabilities: vec!["coding"],
        // ...
    },
    SubAgentDefinition {
        name: "tester",
        tools: vec!["test_runner", "debug_shell"],
        capabilities: vec!["testing"],
        // ...
    },
    SubAgentDefinition {
        name: "cicd_expert",
        tools: vec!["git_ops", "run_command"],
        capabilities: vec!["deployment"],
        // ...
    },
];
```

### Agent Communication (Planned)

Agents will communicate through:
- Shared workspace zones
- Message passing protocols
- Event subscriptions
- Promise contracts

## Custom Tool Registration

Users can register custom tools with security profiles:

```rust
let custom_tool = CustomTool {
    name: "custom_linter",
    implementation: ToolImplementation::Command {
        command: "eslint",
        args: vec!["--fix"],
        env: HashMap::new(),
    },
    security_profile: ToolSecurityProfile {
        required_commands: vec!["eslint"],
        required_paths: vec!["${workspace}/**"],
        risk_level: RiskLevel::Low,
        // ...
    },
    // ...
};

registry.register_custom_tool(custom_tool, user_trust_level).await?;
```

## Permission Negotiation Protocol

### Request Flow

1. **Agent Request** - Agent requests additional permissions
2. **Policy Evaluation** - Multiple policies evaluate request:
   - Trust-based policy
   - Risk-based policy
   - Resource-based policy
3. **Decision Aggregation** - Weighted decision based on all policies
4. **Response Generation** - Approved, Partially Approved, Denied, or Escalated
5. **Alternative Suggestions** - If denied, suggest safer alternatives

### Auto-Approval Patterns

```rust
AutoApprovePattern {
    name: "read_only_workspace",
    command_pattern: Some("cat"),
    path_pattern: Some("/workspace"),
    max_risk_level: RiskLevel::Low,
    min_trust_score: 0.5,
}
```

### Auto-Deny Patterns

```rust
AutoDenyPattern {
    name: "system_modification",
    command_pattern: Some("sudo"),
    reason: "System modification commands are never auto-approved",
}
```

## Security Features

### Context Boundaries

- **Path restrictions** - Limit filesystem access
- **Command whitelisting** - Only allowed commands can execute
- **Network isolation** - Control external connections
- **Resource limits** - CPU, memory, disk I/O caps
- **Time limits** - Maximum execution duration

### Verification Requirements

Each tool operation requires verification:
- **Command verification** - Confirm execution success
- **File system verification** - Check file states
- **Network verification** - Validate API responses
- **State verification** - Ensure consistency

### Audit Logging

All permission negotiations are logged:
```rust
NegotiationAuditEntry {
    timestamp: DateTime<Utc>,
    request_id: Uuid,
    agent_id: String,
    decision: Decision,
    policy_decisions: Vec<String>,
    final_reason: String,
}
```

## Usage Examples

### Basic Agent Creation

```rust
// Define agent from Claude markdown
let agent_def = SubAgentDefinition {
    name: "data_analyst",
    description: "Analyzes data and generates reports",
    tools: vec!["read_file", "web_search"],
    capabilities: vec!["analysis", "reporting"],
    custom_instructions: Some("Focus on data accuracy"),
    source_file: Some(PathBuf::from("agents/data_analyst.md")),
};

// Generate secure context
let context = generator
    .generate_context_from_agent_definition(&agent_def, 0.7)
    .await?;
```

### Permission Request

```rust
// Agent needs more resources
let request = PermissionRequest {
    requested_permissions: RequestedPermissions {
        increased_memory_mb: Some(500),
        additional_commands: vec!["python3"],
        // ...
    },
    justification: "Large dataset analysis requires more memory",
    priority: Priority::High,
    // ...
};

// Negotiate permissions
let response = negotiator.request_permissions(request, &eval_context).await?;

match response.decision {
    Decision::Approved => {
        // Use granted permissions
        let granted = response.granted_permissions.unwrap();
        // ...
    },
    Decision::PartiallyApproved => {
        // Work with reduced permissions
        // ...
    },
    Decision::Denied => {
        // Try alternatives
        for alt in response.alternatives {
            // ...
        }
    },
    _ => {}
}
```

### Multi-Agent Deployment

```rust
// Deploy team of agents
for agent in team {
    // Generate context based on role
    let trust = match agent.name.as_str() {
        "system_architect" => 0.9,
        "analyst" => 0.8,
        "coder" => 0.7,
        "tester" => 0.6,
        "cicd_expert" => 0.8,
        _ => 0.5,
    };
    
    let context = generator
        .generate_context_from_agent_definition(&agent, trust)
        .await?;
    
    // Assign to appropriate zone
    let zone = determine_zone(&agent);
    zones.assign_agent_to_zone(&agent.name, &zone)?;
    
    // Launch agent with context
    // ...
}
```

## Intelligence Features

### Tool Relationship Understanding

The system maintains a knowledge graph of tool relationships:

```rust
// Tool relationship types
enum ToolRelationship {
    Requires,      // ast_parser requires read_file
    Complements,   // git_ops complements run_command
    Substitutes,   // curl can substitute wget
    Enhances,      // debug_shell enhances test_runner
    Conflicts,     // Different linters might conflict
}

// Example: Finding complementary tools
let recommendations = engine.recommend_tools_for_capability("deployment");
// Returns: [("git_ops", 0.9), ("run_command", 0.85), ("test_runner", 0.7)]
```

### Behavioral Learning

The system learns from agent execution patterns:

```rust
// Learn successful pattern
engine.learn_pattern(
    vec!["read_file", "ast_parser", "write_file"],
    vec!["code_refactoring"],
    success: true,
    execution_time_ms: 2500
);

// Future agents benefit from learned patterns
let inferred = engine.infer_capabilities(&new_agent);
// Includes "code_refactoring" based on tool combination
```

### Anomaly Detection

Identifies unusual agent behavior:

```rust
let anomalies = profiling.detect_anomalies(
    agent_id,
    current_tools,
    resource_usage
).await;

// Detected anomalies:
// - UnusualToolUsage: Using undeclared tool "sudo"
// - ResourceSpike: CPU usage 200% above baseline
// - SuspiciousSequence: "read_file->run_command->delete"
```

### Adaptive Permissions

Permissions adapt based on agent behavior:

```rust
// Good behavior increases trust
if task_successful && no_violations {
    trust_score = profiling.update_trust_score(
        agent_id,
        TrustEventType::Increase,
        current_trust,
        "Successful task completion"
    ).await;
    // trust_score: 0.75 -> 0.80
}

// Trust affects future permissions
if trust_score > 0.8 {
    // Auto-approve low-risk tool discoveries
    // Grant higher resource limits
    // Allow more parallel operations
}
```

## Best Practices

1. **Start with minimal permissions** - Agents should request only what they need
2. **Use zones for isolation** - Keep agents in appropriate security zones
3. **Monitor resource usage** - Track and limit resource consumption
4. **Regular permission review** - Revoke unused permissions
5. **Implement verification** - Always verify agent actions
6. **Use auto-patterns wisely** - Configure auto-approve/deny for common cases
7. **Maintain audit logs** - Keep detailed logs of all permission changes
8. **Test in sandbox first** - New agents should start in sandbox zone
9. **Trust incrementally** - Build trust through successful operations
10. **Plan for failures** - Have rollback strategies for failed operations

## Future Enhancements

- **Inter-agent communication protocol** - Direct agent-to-agent messaging
- **Distributed execution** - Agents across multiple nodes
- **Advanced scheduling** - Priority-based agent scheduling
- **Machine learning integration** - Learn optimal permission patterns
- **Visual monitoring** - Real-time agent activity dashboard
- **Template library** - Pre-configured agent templates
- **Compliance frameworks** - Built-in compliance checking
- **Performance optimization** - GPU acceleration for certain operations