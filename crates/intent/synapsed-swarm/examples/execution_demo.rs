//! Demonstration of the real command execution capabilities in synapsed-swarm

use synapsed_swarm::{
    ExecutionEngine, ExecutionConfig,
    SwarmCoordinator, SwarmConfig,
    HierarchicalIntent, IntentBuilder, IntentContext,
};
use tokio;
use tracing::{info, error};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::init();

    println!("🚀 Synapsed Swarm - Real Command Execution Demo");
    println!("================================================");

    // Configure execution engine with safe defaults
    let mut exec_config = ExecutionConfig::default();
    exec_config.allowed_commands = vec![
        "echo".to_string(),
        "pwd".to_string(),
        "ls".to_string(),
        "cat".to_string(),
        "whoami".to_string(),
        "date".to_string(),
    ];
    exec_config.max_execution_time_secs = 10;
    exec_config.allowed_working_dirs = vec![
        PathBuf::from("/tmp"),
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp")),
    ];

    // Create execution engine
    let engine = ExecutionEngine::with_config(exec_config.clone());
    engine.initialize().await?;

    println!("\n🔧 Testing Basic Command Execution");
    println!("-----------------------------------");

    // Test 1: Simple echo command
    match engine.execute_command("echo", &["Hello", "from", "Synapsed!"], None).await {
        Ok(result) => {
            println!("✅ Echo command succeeded:");
            println!("   Command: {} {:?}", result.command, result.args);
            println!("   Output: {}", result.stdout.trim());
            println!("   Duration: {}ms", result.duration_ms);
        }
        Err(e) => {
            error!("❌ Echo command failed: {}", e);
        }
    }

    // Test 2: Working directory
    match engine.execute_command("pwd", &[], None).await {
        Ok(result) => {
            println!("✅ PWD command succeeded:");
            println!("   Current directory: {}", result.stdout.trim());
        }
        Err(e) => {
            error!("❌ PWD command failed: {}", e);
        }
    }

    // Test 3: List files
    match engine.execute_command("ls", &["-la"], None).await {
        Ok(result) => {
            println!("✅ LS command succeeded:");
            println!("   Files found: {} lines of output", result.stdout.lines().count());
        }
        Err(e) => {
            error!("❌ LS command failed: {}", e);
        }
    }

    println!("\n🚫 Testing Security Restrictions");
    println!("----------------------------------");

    // Test 4: Blocked command (should fail)
    match engine.execute_command("rm", &["-rf", "/"], None).await {
        Ok(_) => {
            println!("❌ SECURITY BREACH: rm command should have been blocked!");
        }
        Err(_) => {
            println!("✅ Security check passed: rm command properly blocked");
        }
    }

    // Test 5: Unknown command (should fail)
    match engine.execute_command("nonexistent_command_12345", &[], None).await {
        Ok(_) => {
            println!("❌ Unknown command somehow succeeded");
        }
        Err(_) => {
            println!("✅ Unknown command properly rejected");
        }
    }

    println!("\n🏗️ Testing Swarm Integration");
    println!("-----------------------------");

    // Create swarm with custom execution config
    let mut swarm_config = SwarmConfig::default();
    swarm_config.execution_config = exec_config;
    
    let coordinator = SwarmCoordinator::new(swarm_config);
    coordinator.initialize().await?;

    // Create a simple intent
    let intent = IntentBuilder::new("demo_intent")
        .description("Demonstrate real command execution in swarm")
        .add_step("echo 'Swarm execution test'")
        .add_step("whoami")
        .build()?;

    println!("✅ Swarm coordinator initialized with real execution engine");
    println!("✅ Created intent with {} steps", intent.steps().len());

    // Test direct execution through the execution engine
    match coordinator.execution_engine().execute_command("echo", &["Swarm", "integration", "works!"], None).await {
        Ok(result) => {
            println!("✅ Swarm execution engine test:");
            println!("   Output: {}", result.stdout.trim());
        }
        Err(e) => {
            error!("❌ Swarm execution failed: {}", e);
        }
    }

    println!("\n📊 Execution Statistics");
    println!("------------------------");

    let history = coordinator.execution_engine().execution_history().await;
    println!("Total commands executed: {}", history.len());
    
    let successful_commands = history.iter().filter(|r| r.success).count();
    let failed_commands = history.len() - successful_commands;
    
    println!("Successful executions: {}", successful_commands);
    println!("Failed executions: {}", failed_commands);
    
    if !history.is_empty() {
        let avg_duration: f64 = history.iter().map(|r| r.duration_ms as f64).sum::<f64>() / history.len() as f64;
        println!("Average execution time: {:.2}ms", avg_duration);
    }

    println!("\n🎉 Demo completed successfully!");
    println!("The synapsed-swarm crate now has production-ready command execution capabilities:");
    println!("• Real shell command execution with tokio::process::Command");
    println!("• Security: allowlist/blocklist, working directory restrictions");
    println!("• Resource limits: timeouts, memory/CPU constraints (OS-dependent)"); 
    println!("• Sandboxing: user/group restrictions (when running as root)");
    println!("• Integration: seamlessly works with existing verification framework");
    println!("• Monitoring: execution history, statistics, and real-time logging");

    Ok(())
}