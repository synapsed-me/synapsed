//! MCP Server Example
//! 
//! This example demonstrates how to run the Synapsed MCP (Model Context Protocol)
//! server to provide intent verification tools to Claude and other AI agents.

use anyhow::Result;
use synapsed_mcp::{McpServer, ServerConfig};
use tracing::{info, debug};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,synapsed_mcp=debug".to_string())
        )
        .init();

    info!("Starting Synapsed MCP Server Example");
    
    // Configure the server
    let config = ServerConfig {
        name: "synapsed-intent-verifier".to_string(),
        version: "1.0.0".to_string(),
        description: "Intent verification and context management for AI agents".to_string(),
        host: "127.0.0.1".to_string(),
        port: 3000,
        enable_stdio: false,  // Use HTTP transport for this example
        enable_verification: true,
        enable_promises: true,
        enable_context_injection: true,
        max_context_size: 1024 * 1024, // 1MB
        trust_threshold: 0.7,
    };
    
    info!("Server configuration:");
    info!("  Name: {}", config.name);
    info!("  Version: {}", config.version);
    info!("  Address: {}:{}", config.host, config.port);
    info!("  Features:");
    info!("    - Verification: {}", config.enable_verification);
    info!("    - Promises: {}", config.enable_promises);
    info!("    - Context Injection: {}", config.enable_context_injection);
    
    // Create and start the server
    let server = McpServer::new(config)?;
    
    // Register available tools
    info!("\nAvailable MCP Tools:");
    info!("  Intent Management:");
    info!("    - intent_declare: Declare an intent before acting");
    info!("    - intent_status: Get current intent status");
    info!("    - intent_complete: Mark intent as completed");
    
    info!("  Verification:");
    info!("    - verify_command: Verify command execution");
    info!("    - verify_file: Verify file operations");
    info!("    - verify_api: Verify API responses");
    
    info!("  Promise Theory:");
    info!("    - promise_make: Make a promise to another agent");
    info!("    - promise_accept: Accept a promise from another");
    info!("    - promise_fulfill: Mark promise as fulfilled");
    info!("    - trust_check: Check trust level of an agent");
    
    info!("  Context Management:");
    info!("    - context_inject: Inject context for sub-agents");
    info!("    - context_get: Retrieve current context");
    info!("    - context_validate: Validate context boundaries");
    
    // Example of how Claude would interact with the server
    demonstrate_tool_usage().await?;
    
    // Start the server
    let addr: SocketAddr = format!("{}:{}", server.config().host, server.config().port)
        .parse()?;
    
    info!("\n🚀 MCP Server starting on http://{}", addr);
    info!("Claude can connect to this server to use intent verification tools");
    
    // Run the server
    server.run().await?;
    
    Ok(())
}

/// Demonstrate how the MCP tools would be used
async fn demonstrate_tool_usage() -> Result<()> {
    info!("\n=== Demonstration of MCP Tool Usage ===");
    
    // Example 1: Intent declaration flow
    info!("\n1. Intent Declaration Flow:");
    info!("   Claude → intent_declare('Process user data')");
    info!("   Server → Returns intent_id: 'intent_123'");
    info!("   Claude → Executes data processing");
    info!("   Claude → verify_command('python process.py', output)");
    info!("   Server → Verification passed ✓");
    info!("   Claude → intent_complete('intent_123')");
    
    // Example 2: Promise-based cooperation
    info!("\n2. Promise-Based Cooperation:");
    info!("   Claude → promise_make('Complete analysis in 5s', 'claude', 'user')");
    info!("   Server → Promise created: 'promise_456'");
    info!("   Claude → Performs analysis");
    info!("   Claude → promise_fulfill('promise_456')");
    info!("   Server → Trust score updated: 0.85");
    
    // Example 3: Context injection for sub-agents
    info!("\n3. Context Injection for Sub-Agents:");
    info!("   Claude → context_inject({{");
    info!("     'parent_intent': 'intent_123',");
    info!("     'allowed_operations': ['read', 'process'],");
    info!("     'max_tokens': 1000,");
    info!("     'verification_required': true");
    info!("   }})");
    info!("   Server → Context injected: 'context_789'");
    info!("   SubAgent → context_get('context_789')");
    info!("   SubAgent → Operates within boundaries");
    
    Ok(())
}

/// Example of a custom MCP tool handler
/// This would be registered with the server in a real implementation
#[allow(dead_code)]
async fn handle_custom_tool(tool_name: &str, args: serde_json::Value) -> Result<serde_json::Value> {
    match tool_name {
        "intent_declare" => {
            let goal = args["goal"].as_str().unwrap_or("Unknown goal");
            debug!("Declaring intent: {}", goal);
            
            Ok(serde_json::json!({
                "intent_id": "intent_example_123",
                "goal": goal,
                "status": "declared",
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }))
        },
        
        "verify_command" => {
            let command = args["command"].as_str().unwrap_or("");
            let output = args["output"].as_str().unwrap_or("");
            
            debug!("Verifying command: {}", command);
            
            Ok(serde_json::json!({
                "verified": true,
                "command": command,
                "output_matches": output.len() > 0,
                "confidence": 0.95,
            }))
        },
        
        "trust_check" => {
            let agent = args["agent"].as_str().unwrap_or("unknown");
            
            Ok(serde_json::json!({
                "agent": agent,
                "trust_score": 0.75,
                "total_promises": 10,
                "fulfilled": 8,
                "violated": 2,
            }))
        },
        
        _ => {
            Ok(serde_json::json!({
                "error": format!("Unknown tool: {}", tool_name)
            }))
        }
    }
}