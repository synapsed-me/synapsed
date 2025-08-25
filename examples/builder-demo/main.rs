//! Example demonstrating how to use the Synapsed Builder system
//! 
//! This shows how Claude (or any user) can compose applications
//! by assembling pre-built modules without writing implementation code.

use synapsed_builder::prelude::*;
use synapsed_builder::{
    builder::{StorageBackend, ObservabilityLevel, NetworkType},
    recipe::RecipeManager,
    templates::Templates,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🏗️  Synapsed Builder Demo\n");
    
    // Example 1: Use a pre-built template
    demo_template_usage().await?;
    
    // Example 2: Build custom composition with DSL
    demo_custom_builder().await?;
    
    // Example 3: Load and use a recipe
    demo_recipe_usage().await?;
    
    // Example 4: Natural language to composition (what Claude would do)
    demo_natural_language_composition().await?;
    
    Ok(())
}

/// Demonstrate using a pre-built template
async fn demo_template_usage() -> anyhow::Result<()> {
    println!("📦 Example 1: Using a Template\n");
    
    // List available templates
    println!("Available templates:");
    for template_info in Templates::list() {
        println!("  • {} - {}", template_info.name, template_info.description);
    }
    
    // Select and build the verified AI agent template
    println!("\n🎯 Building 'verified-ai-agent' template...");
    
    let app = Templates::verified_ai_agent()
        .configure("synapsed-storage", serde_json::json!({
            "path": "./demo/agent.db"
        }))
        .env("RUST_LOG", "debug")
        .build()?;
    
    println!("✅ Application built successfully!");
    println!("   Name: {}", app.name);
    println!("   Components: {:?}", app.components);
    
    // Save to directory
    app.save("./build/verified-agent").await?;
    println!("📁 Saved to ./build/verified-agent/");
    
    Ok(())
}

/// Demonstrate custom builder DSL
async fn demo_custom_builder() -> anyhow::Result<()> {
    println!("\n📦 Example 2: Custom Builder DSL\n");
    
    println!("🔨 Building custom application...");
    
    let app = SynapsedBuilder::new("my-custom-app")
        .description("Custom application with specific requirements")
        
        // Add capabilities by high-level requirements
        .add_intent_verification()
        .add_storage(StorageBackend::Sqlite)
        .add_observability(ObservabilityLevel::Full)
        .add_network(NetworkType::P2P)
        
        // Add specific connections
        .connect(
            "synapsed-net", "peer_connected",
            "synapsed-intent", "context_updated"
        )
        
        // Configure components
        .configure("synapsed-net", serde_json::json!({
            "bootstrap_nodes": [
                "/ip4/127.0.0.1/tcp/4001",
                "/ip4/127.0.0.1/tcp/4002"
            ],
            "max_peers": 50
        }))
        
        // Set environment
        .env("RUST_LOG", "info")
        .env("P2P_LISTEN_ADDR", "0.0.0.0:4000")
        
        // Validate and build
        .build()?;
    
    println!("✅ Custom application built!");
    println!("   Components: {} total", app.components.len());
    
    // Show generated Cargo.toml
    println!("\n📄 Generated Cargo.toml preview:");
    let cargo_toml = app.generate_cargo_toml();
    println!("{}", &cargo_toml[..cargo_toml.len().min(500)]);
    println!("...");
    
    Ok(())
}

/// Demonstrate loading and using recipes
async fn demo_recipe_usage() -> anyhow::Result<()> {
    println!("\n📦 Example 3: Using Recipes\n");
    
    let mut recipe_manager = RecipeManager::new();
    
    // Load recipe from YAML file
    let yaml_content = std::fs::read_to_string("../../tools/recipes/verified-ai-agent.yaml")
        .unwrap_or_else(|_| {
            // Fallback if file doesn't exist
            include_str!("../../tools/recipes/verified-ai-agent.yaml").to_string()
        });
    
    let recipe_name = recipe_manager.load_yaml(&yaml_content)?;
    println!("📋 Loaded recipe: {}", recipe_name);
    
    // Get and validate the recipe
    let recipe = recipe_manager.get(&recipe_name).unwrap();
    println!("   Description: {}", recipe.description);
    println!("   Components: {} total", recipe.components.len());
    println!("   Connections: {} defined", recipe.connections.len());
    
    // Build from recipe
    let app = SynapsedBuilder::from_recipe(recipe.clone())
        .build()?;
    
    println!("✅ Built application from recipe!");
    
    Ok(())
}

/// Demonstrate how Claude would compose based on natural language
async fn demo_natural_language_composition() -> anyhow::Result<()> {
    println!("\n📦 Example 4: Natural Language Composition (Claude-style)\n");
    
    // Simulate Claude receiving a natural language request
    let user_request = "I need a secure system for processing payments with full monitoring \
                        and the ability to handle distributed consensus";
    
    println!("👤 User request: \"{}\"", user_request);
    println!("\n🤖 Claude's interpretation:");
    
    // Claude would parse this and identify needed capabilities
    let identified_needs = vec![
        "payment processing",
        "security/encryption",
        "monitoring/observability",
        "distributed consensus"
    ];
    
    println!("   Identified needs:");
    for need in &identified_needs {
        println!("     • {}", need);
    }
    
    // Claude would then compose the application
    println!("\n🔧 Composing application...");
    
    let app = SynapsedBuilder::new("payment-consensus-system")
        .description("Secure payment system with consensus and monitoring")
        
        // Add components based on identified needs
        .add_payments()                           // payment processing
        .add_component("synapsed-consensus")      // distributed consensus
        .add_component("synapsed-crdt")           // conflict-free replicated data
        .add_observability(ObservabilityLevel::Full)  // monitoring
        
        // Claude would know to add these connections
        .connect(
            "synapsed-payments", "transaction_created",
            "synapsed-consensus", "propose"
        )
        .connect(
            "synapsed-consensus", "committed",
            "synapsed-crdt", "merge"
        )
        
        // Configuration based on requirements
        .configure("synapsed-consensus", serde_json::json!({
            "consensus_type": "hotstuff",
            "committee_size": 5,
            "block_time_ms": 1000
        }))
        .configure("synapsed-payments", serde_json::json!({
            "supported_currencies": ["USD", "EUR", "BTC"],
            "risk_threshold": 80
        }))
        
        .build()?;
    
    println!("✅ Application composed successfully!");
    println!("\n📊 Composition summary:");
    println!("   Total components: {}", app.components.len());
    println!("   Components:");
    for component in &app.components {
        println!("     • {}", component);
    }
    
    // Claude would explain what was built
    println!("\n💬 Claude's explanation:");
    println!("   I've composed a payment processing system with the following features:");
    println!("   • Secure payment handling with multiple currency support");
    println!("   • Identity management and cryptographic security");
    println!("   • Distributed consensus using HotStuff algorithm");
    println!("   • Conflict-free replicated data types for consistency");
    println!("   • Full observability with monitoring dashboard");
    println!("   • All components are connected and configured appropriately");
    
    Ok(())
}

/// Helper function to simulate Claude's capability matching
fn match_requirement_to_components(requirement: &str) -> Vec<String> {
    match requirement {
        r if r.contains("payment") => vec!["synapsed-payments", "synapsed-identity", "synapsed-crypto"],
        r if r.contains("security") || r.contains("encryption") => vec!["synapsed-crypto", "synapsed-safety"],
        r if r.contains("monitoring") || r.contains("observability") => vec!["synapsed-substrates", "synapsed-monitor"],
        r if r.contains("consensus") => vec!["synapsed-consensus", "synapsed-net"],
        r if r.contains("storage") => vec!["synapsed-storage"],
        r if r.contains("ai") || r.contains("agent") => vec!["synapsed-intent", "synapsed-verify"],
        _ => vec!["synapsed-core"],
    }.into_iter().map(String::from).collect()
}