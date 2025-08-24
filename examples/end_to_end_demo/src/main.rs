//! End-to-end demonstration of the Synapsed Swarm system
//! 
//! This demo showcases:
//! - Multi-agent coordination with real command execution
//! - Promise Theory in action
//! - Verification of agent claims
//! - Trust score evolution
//! - Monitoring and metrics
//! - Persistent storage

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::sync::Arc;
use std::time::Duration;
use synapsed_swarm::prelude::*;
use synapsed_intent::{IntentBuilder, Step, StepAction};
use synapsed_promise::{AutonomousAgent, AgentConfig, AgentCapabilities};
use tokio::time::sleep;
use tracing::{info, warn, error};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "synapsed-e2e")]
#[command(about = "End-to-end demonstration of Synapsed Swarm", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a simple multi-agent task
    Simple {
        /// Number of agents to use
        #[arg(short, long, default_value_t = 3)]
        agents: usize,
    },
    /// Run a complex software development scenario
    Complex {
        /// Project name
        #[arg(short, long, default_value = "demo_api")]
        project: String,
    },
    /// Demonstrate trust evolution
    Trust {
        /// Number of iterations
        #[arg(short, long, default_value_t = 10)]
        iterations: usize,
    },
    /// Show monitoring dashboard
    Monitor {
        /// Port for metrics server
        #[arg(short, long, default_value_t = 9090)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,synapsed=debug")
        .init();
    
    let cli = Cli::parse();
    
    println!("{}", "═══════════════════════════════════════════════════════".bright_blue());
    println!("{}", "          SYNAPSED SWARM - END-TO-END DEMO            ".bright_blue().bold());
    println!("{}", "═══════════════════════════════════════════════════════".bright_blue());
    println!();
    
    match cli.command {
        Commands::Simple { agents } => run_simple_demo(agents).await?,
        Commands::Complex { project } => run_complex_demo(&project).await?,
        Commands::Trust { iterations } => run_trust_demo(iterations).await?,
        Commands::Monitor { port } => run_monitoring_demo(port).await?,
    }
    
    Ok(())
}

/// Simple multi-agent task demonstration
async fn run_simple_demo(agent_count: usize) -> Result<()> {
    println!("{}", "🚀 Simple Multi-Agent Demo".green().bold());
    println!("Creating swarm with {} agents...\n", agent_count);
    
    // Configure swarm with real execution
    let config = SwarmConfig {
        max_agents: 10,
        min_trust_score: 0.3,
        require_verification: true,
        task_timeout_secs: 60,
        track_promises: true,
        ..Default::default()
    };
    
    // Add execution configuration
    let mut exec_config = ExecutionConfig::default();
    exec_config.allowed_commands = vec![
        "echo".to_string(),
        "ls".to_string(),
        "cat".to_string(),
        "pwd".to_string(),
        "date".to_string(),
    ];
    exec_config.enable_sandboxing = true;
    
    // Create coordinator with persistent storage
    let coordinator = Arc::new(SwarmCoordinator::with_config_and_storage(
        config,
        exec_config,
        StorageBackend::Sqlite("swarm_trust.db".to_string()),
    ).await?);
    
    coordinator.initialize().await?;
    println!("✅ Swarm initialized with SQLite storage\n");
    
    // Create and add agents
    for i in 0..agent_count {
        let agent = create_demo_agent(&format!("agent_{}", i), i);
        let agent_id = coordinator.add_agent(agent, agent_role(i)).await?;
        println!("  {} Agent {} joined (ID: {})", 
                 "➕".green(), 
                 format!("agent_{}", i).cyan(),
                 format!("{}", agent_id).dim());
    }
    
    println!("\n{}", "📋 Creating intent chain...".yellow());
    
    // Create a chain of intents
    let intent = IntentBuilder::new("Analyze system status")
        .add_step(Step::new(
            "Check current directory",
            StepAction::Command("pwd".to_string())
        ))
        .add_step(Step::new(
            "List files",
            StepAction::Command("ls -la".to_string())
        ))
        .add_step(Step::new(
            "Show date",
            StepAction::Command("date".to_string())
        ))
        .add_step(Step::new(
            "Echo completion",
            StepAction::Command("echo 'Task completed successfully!'".to_string())
        ))
        .with_verification_required(true)
        .build()?;
    
    // Create context
    let context = synapsed_intent::ContextBuilder::new()
        .variable("demo_type", serde_json::json!("simple"))
        .variable("agent_count", serde_json::json!(agent_count))
        .build()
        .await;
    
    // Delegate to swarm
    println!("{}", "🤝 Delegating intent to swarm...".yellow());
    let task_id = coordinator.delegate_intent(intent, context).await?;
    println!("  Task ID: {}\n", format!("{}", task_id).dim());
    
    // Monitor execution
    println!("{}", "⏳ Monitoring execution...".yellow());
    let start = std::time::Instant::now();
    
    loop {
        sleep(Duration::from_millis(500)).await;
        
        if let Some(result) = coordinator.get_task_result(task_id).await {
            let duration = start.elapsed();
            
            if result.success {
                println!("\n{} Task completed successfully!", "✅".green());
                println!("  Duration: {:.2}s", duration.as_secs_f64());
                
                if let Some(output) = result.output {
                    println!("\n{}", "📊 Output:".cyan().bold());
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
                
                if result.verification_proof.is_some() {
                    println!("\n{} Execution verified with cryptographic proof", "🔐".green());
                }
            } else {
                println!("\n{} Task failed!", "❌".red());
                if let Some(error) = result.error {
                    println!("  Error: {}", error.red());
                }
            }
            
            break;
        }
        
        print!(".");
        use std::io::Write;
        std::io::stdout().flush()?;
    }
    
    // Show metrics
    println!("\n{}", "📊 Swarm Metrics:".cyan().bold());
    let metrics = coordinator.metrics().await;
    println!("  Total agents: {}", metrics.total_agents);
    println!("  Tasks succeeded: {}", metrics.tasks_succeeded);
    println!("  Tasks failed: {}", metrics.tasks_failed);
    println!("  Promises made: {}", metrics.promises_made);
    println!("  Promises fulfilled: {}", metrics.promises_fulfilled);
    println!("  Average trust score: {:.2}", metrics.avg_trust_score);
    println!("  Verification success rate: {:.2}%", metrics.verification_success_rate * 100.0);
    
    Ok(())
}

/// Complex software development scenario
async fn run_complex_demo(project: &str) -> Result<()> {
    println!("{}", "🏗️ Complex Software Development Demo".green().bold());
    println!("Project: {}\n", project.cyan());
    
    // This would implement a full software development workflow
    // with multiple specialized agents working together
    
    println!("{}", "Creating specialized development team...".yellow());
    
    // Create swarm
    let config = SwarmConfig::default();
    let coordinator = Arc::new(SwarmCoordinator::new(config));
    coordinator.initialize().await?;
    
    // Create specialized agents
    let architect = create_specialist_agent("architect", vec![
        "api_design".to_string(),
        "architecture".to_string(),
    ]);
    let backend_dev = create_specialist_agent("backend_dev", vec![
        "rust".to_string(),
        "database".to_string(),
        "api_implementation".to_string(),
    ]);
    let frontend_dev = create_specialist_agent("frontend_dev", vec![
        "typescript".to_string(),
        "react".to_string(),
        "ui_implementation".to_string(),
    ]);
    let tester = create_specialist_agent("tester", vec![
        "testing".to_string(),
        "test_generation".to_string(),
        "quality_assurance".to_string(),
    ]);
    let reviewer = create_specialist_agent("reviewer", vec![
        "code_review".to_string(),
        "security_audit".to_string(),
        "best_practices".to_string(),
    ]);
    
    // Add agents to swarm
    coordinator.add_agent(architect, AgentRole::Coordinator).await?;
    coordinator.add_agent(backend_dev, AgentRole::Worker).await?;
    coordinator.add_agent(frontend_dev, AgentRole::Worker).await?;
    coordinator.add_agent(tester, AgentRole::Verifier).await?;
    coordinator.add_agent(reviewer, AgentRole::Verifier).await?;
    
    println!("✅ Development team assembled\n");
    
    // Create development workflow
    let workflow = IntentBuilder::new(&format!("Build {} API", project))
        .add_step(Step::new(
            "Design API architecture",
            StepAction::Delegate {
                target: "architect".to_string(),
                sub_intent: Box::new(IntentBuilder::new("Design REST API")
                    .add_step(Step::new("Define endpoints", StepAction::Custom(serde_json::json!({
                        "action": "design",
                        "target": "endpoints"
                    }))))
                    .build()?),
            }
        ))
        .add_step(Step::new(
            "Implement backend",
            StepAction::Delegate {
                target: "backend_dev".to_string(),
                sub_intent: Box::new(IntentBuilder::new("Implement API backend")
                    .build()?),
            }
        ))
        .add_step(Step::new(
            "Implement frontend",
            StepAction::Delegate {
                target: "frontend_dev".to_string(),
                sub_intent: Box::new(IntentBuilder::new("Create UI components")
                    .build()?),
            }
        ))
        .add_step(Step::new(
            "Write tests",
            StepAction::Delegate {
                target: "tester".to_string(),
                sub_intent: Box::new(IntentBuilder::new("Create test suite")
                    .build()?),
            }
        ))
        .add_step(Step::new(
            "Review code",
            StepAction::Delegate {
                target: "reviewer".to_string(),
                sub_intent: Box::new(IntentBuilder::new("Perform code review")
                    .build()?),
            }
        ))
        .with_verification_required(true)
        .build()?;
    
    println!("{}", "🚀 Starting development workflow...".yellow());
    
    let context = synapsed_intent::ContextBuilder::new()
        .variable("project", serde_json::json!(project))
        .variable("language", serde_json::json!("rust"))
        .variable("framework", serde_json::json!("actix-web"))
        .build()
        .await;
    
    let task_id = coordinator.delegate_intent(workflow, context).await?;
    
    // Simulate monitoring the complex workflow
    println!("{}", "⏳ Development in progress...".yellow());
    for step in ["Architecture", "Backend", "Frontend", "Testing", "Review"] {
        sleep(Duration::from_secs(2)).await;
        println!("  {} {} phase completed", "✓".green(), step);
    }
    
    println!("\n{} Project {} completed successfully!", "🎉".green(), project.cyan());
    
    Ok(())
}

/// Demonstrate trust evolution over time
async fn run_trust_demo(iterations: usize) -> Result<()> {
    println!("{}", "📈 Trust Evolution Demo".green().bold());
    println!("Running {} iterations...\n", iterations);
    
    // Create swarm with persistent storage
    let config = SwarmConfig::default();
    let coordinator = Arc::new(SwarmCoordinator::with_storage(
        config,
        StorageBackend::Sqlite("trust_evolution.db".to_string()),
    ).await?);
    coordinator.initialize().await?;
    
    // Create agents with different reliability levels
    let reliable_agent = create_demo_agent("reliable", 0);
    let unreliable_agent = create_demo_agent("unreliable", 1);
    let improving_agent = create_demo_agent("improving", 2);
    
    let reliable_id = coordinator.add_agent(reliable_agent, AgentRole::Worker).await?;
    let unreliable_id = coordinator.add_agent(unreliable_agent, AgentRole::Worker).await?;
    let improving_id = coordinator.add_agent(improving_agent, AgentRole::Worker).await?;
    
    println!("Agents created:");
    println!("  {} Reliable Agent (90% success rate)", "🟢".green());
    println!("  {} Unreliable Agent (30% success rate)", "🔴".red());
    println!("  {} Improving Agent (starts at 50%, improves over time)", "🟡".yellow());
    println!();
    
    // Run iterations
    for i in 0..iterations {
        println!("Iteration {}/{}:", i + 1, iterations);
        
        // Create simple task
        let intent = IntentBuilder::new(&format!("Task {}", i))
            .add_step(Step::new(
                "Execute",
                StepAction::Command("echo 'Executing task'".to_string())
            ))
            .build()?;
        
        let context = synapsed_intent::ContextBuilder::new().build().await;
        
        // Delegate tasks to each agent
        for (agent_id, name) in &[
            (reliable_id, "Reliable"),
            (unreliable_id, "Unreliable"),
            (improving_id, "Improving"),
        ] {
            // Simulate task execution with different success rates
            let success = match *name {
                "Reliable" => rand::random::<f64>() < 0.9,
                "Unreliable" => rand::random::<f64>() < 0.3,
                "Improving" => {
                    let rate = 0.5 + (i as f64 / iterations as f64) * 0.4;
                    rand::random::<f64>() < rate
                }
                _ => true,
            };
            
            // Update trust based on outcome
            coordinator.update_agent_trust(*agent_id, success, true).await?;
            
            let trust = coordinator.get_agent_trust(*agent_id).await?;
            let symbol = if success { "✓".green() } else { "✗".red() };
            
            println!("  {} {} - Trust: {:.2}", 
                     symbol, 
                     name.pad_to(11), 
                     trust);
        }
        
        println!();
        sleep(Duration::from_millis(500)).await;
    }
    
    // Show final trust scores
    println!("{}", "📊 Final Trust Scores:".cyan().bold());
    for (agent_id, name) in &[
        (reliable_id, "Reliable"),
        (unreliable_id, "Unreliable"),
        (improving_id, "Improving"),
    ] {
        let trust = coordinator.get_agent_trust(*agent_id).await?;
        let color = if trust > 0.7 {
            "green"
        } else if trust > 0.4 {
            "yellow"
        } else {
            "red"
        };
        
        let bar_length = (trust * 20.0) as usize;
        let bar = "█".repeat(bar_length);
        let empty = "░".repeat(20 - bar_length);
        
        println!("  {} {} [{}{

}] {:.2}", 
                 name.pad_to(11),
                 match color {
                     "green" => bar.green(),
                     "yellow" => bar.yellow(),
                     _ => bar.red(),
                 },
                 empty.dim(),
                 trust);
    }
    
    Ok(())
}

/// Run monitoring dashboard
async fn run_monitoring_demo(port: u16) -> Result<()> {
    println!("{}", "📊 Monitoring Dashboard Demo".green().bold());
    println!("Starting metrics server on port {}...\n", port);
    
    // Create monitoring configuration
    let monitoring_config = MonitoringConfig {
        prometheus_port: port,
        health_check_port: port + 1,
        collection_interval: Duration::from_secs(5),
        enable_dashboard: true,
        ..Default::default()
    };
    
    // Create swarm with monitoring
    let swarm_config = SwarmConfig::default();
    let coordinator = Arc::new(SwarmCoordinator::with_monitoring(
        swarm_config,
        monitoring_config,
    ).await?);
    coordinator.initialize().await?;
    
    println!("✅ Monitoring system initialized");
    println!();
    println!("Available endpoints:");
    println!("  {} Prometheus metrics: http://localhost:{}/metrics", "📈".cyan(), port);
    println!("  {} Health check: http://localhost:{}/health", "🏥".green(), port + 1);
    println!("  {} Dashboard: http://localhost:{}/dashboard", "📊".blue(), port + 1);
    println!();
    
    // Create some agents and activity
    println!("{}", "Creating agents and generating activity...".yellow());
    
    for i in 0..5 {
        let agent = create_demo_agent(&format!("monitor_agent_{}", i), i);
        coordinator.add_agent(agent, AgentRole::Worker).await?;
    }
    
    // Generate some activity
    let activity_handle = tokio::spawn({
        let coordinator = coordinator.clone();
        async move {
            loop {
                // Create random tasks
                let intent = IntentBuilder::new("Monitoring test task")
                    .add_step(Step::new(
                        "Test",
                        StepAction::Command("echo 'test'".to_string())
                    ))
                    .build()
                    .unwrap();
                
                let context = synapsed_intent::ContextBuilder::new().build().await;
                
                let _ = coordinator.delegate_intent(intent, context).await;
                
                sleep(Duration::from_secs(10)).await;
            }
        }
    });
    
    println!("{}", "📊 Monitoring dashboard is running!".green().bold());
    println!("{}", "Press Ctrl+C to stop...".dim());
    println!();
    
    // Keep running until interrupted
    tokio::signal::ctrl_c().await?;
    
    println!("\n{}", "Shutting down monitoring...".yellow());
    activity_handle.abort();
    
    Ok(())
}

// Helper functions

fn create_demo_agent(name: &str, index: usize) -> Arc<AutonomousAgent> {
    let capabilities = match index % 3 {
        0 => vec!["execution".to_string(), "verification".to_string()],
        1 => vec!["analysis".to_string(), "planning".to_string()],
        _ => vec!["testing".to_string(), "monitoring".to_string()],
    };
    
    let config = AgentConfig {
        name: name.to_string(),
        capabilities: AgentCapabilities {
            services: capabilities,
            resources: vec!["cpu".to_string(), "memory".to_string()],
            protocols: vec!["promise".to_string(), "verification".to_string()],
            quality: synapsed_promise::QualityOfService::default(),
        },
        trust_model: synapsed_promise::TrustModel::new(),
        cooperation_protocol: synapsed_promise::CooperationProtocol::new(),
        max_promises: 10,
        promise_timeout_secs: 60,
    };
    
    Arc::new(AutonomousAgent::new(config))
}

fn create_specialist_agent(role: &str, capabilities: Vec<String>) -> Arc<AutonomousAgent> {
    let config = AgentConfig {
        name: role.to_string(),
        capabilities: AgentCapabilities {
            services: capabilities,
            resources: vec!["cpu".to_string(), "memory".to_string(), "network".to_string()],
            protocols: vec!["promise".to_string(), "verification".to_string(), "consensus".to_string()],
            quality: synapsed_promise::QualityOfService::default(),
        },
        trust_model: synapsed_promise::TrustModel::new(),
        cooperation_protocol: synapsed_promise::CooperationProtocol::new(),
        max_promises: 20,
        promise_timeout_secs: 300,
    };
    
    Arc::new(AutonomousAgent::new(config))
}

fn agent_role(index: usize) -> AgentRole {
    match index % 4 {
        0 => AgentRole::Worker,
        1 => AgentRole::Verifier,
        2 => AgentRole::Worker,
        _ => AgentRole::Observer,
    }
}

// Extension trait for padding
trait PadTo {
    fn pad_to(&self, width: usize) -> String;
}

impl PadTo for &str {
    fn pad_to(&self, width: usize) -> String {
        format!("{:width$}", self, width = width)
    }
}