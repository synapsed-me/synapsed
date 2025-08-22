//! Intent Verification Example
//! 
//! This example demonstrates how to use the synapsed-intent and synapsed-verify
//! crates to create verifiable AI agent intentions and verify their execution.

use anyhow::Result;
use synapsed_intent::{
    HierarchicalIntent, IntentBuilder, StepAction, VerificationRequirement,
    Priority, IntentConfig,
};
use synapsed_verify::{
    VerificationStrategy, CommandVerifier, FileSystemVerifier, 
    CompositeVerifier, VerificationOutcome,
};
use synapsed_substrates::{
    BasicCircuit, BasicChannel, Subject, Emission,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, error};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    info!("Starting Intent Verification Example");

    // Create a circuit for observability
    let circuit = Arc::new(BasicCircuit::new("intent-verification"));
    
    // Example 1: Simple command execution intent
    simple_command_intent().await?;
    
    // Example 2: File operation intent with verification
    file_operation_intent().await?;
    
    // Example 3: Complex hierarchical intent
    hierarchical_intent_example().await?;
    
    // Example 4: Intent with observability integration
    observable_intent_example(circuit).await?;
    
    info!("All examples completed successfully!");
    Ok(())
}

/// Example 1: Simple command execution intent
async fn simple_command_intent() -> Result<()> {
    info!("=== Example 1: Simple Command Execution Intent ===");
    
    // Create an intent to list files
    let intent = IntentBuilder::new("List project files")
        .description("List all Rust files in the project")
        .priority(Priority::Normal)
        .step("list_files", StepAction::Command {
            command: "ls".to_string(),
            args: vec!["-la".to_string()],
        })
        .verified_step(
            "count_files",
            StepAction::Command {
                command: "find".to_string(),
                args: vec![".".to_string(), "-name".to_string(), "*.rs".to_string()],
            },
            VerificationRequirement::CommandOutput {
                expected_pattern: Some(r"\.rs$".to_string()),
            }
        )
        .build();
    
    // Plan the execution
    let plan = intent.plan().await?;
    info!("Execution plan created with {} steps", plan.steps.len());
    
    // Create a command verifier
    let verifier = CommandVerifier::new();
    
    // Simulate command execution and verification
    let command = "ls -la";
    let output = "file1.rs\nfile2.rs\nCargo.toml";
    
    let verification = verifier.verify_output(command, output)?;
    
    match verification.passed {
        true => info!("✓ Command verification passed"),
        false => error!("✗ Command verification failed"),
    }
    
    Ok(())
}

/// Example 2: File operation intent with verification
async fn file_operation_intent() -> Result<()> {
    info!("=== Example 2: File Operation Intent ===");
    
    let intent = IntentBuilder::new("Create and verify configuration file")
        .description("Create a config file and verify its existence")
        .step("create_config", StepAction::Custom(
            serde_json::json!({
                "action": "create_file",
                "path": "/tmp/config.json",
                "content": {
                    "version": "1.0",
                    "enabled": true
                }
            })
        ))
        .verified_step(
            "verify_config",
            StepAction::Custom(
                serde_json::json!({
                    "action": "verify_file",
                    "path": "/tmp/config.json"
                })
            ),
            VerificationRequirement::FileExists {
                path: "/tmp/config.json".to_string(),
                check_content: true,
            }
        )
        .build();
    
    // Create a file system verifier
    let mut fs_verifier = FileSystemVerifier::new();
    fs_verifier.expect_file("/tmp/config.json");
    
    // Simulate file creation
    debug!("Simulating file creation at /tmp/config.json");
    
    // Verify file existence
    let files = vec!["/tmp/config.json".to_string()];
    let verification = fs_verifier.verify_files(&files)?;
    
    info!("File verification: {}", 
        if verification.passed { "✓ Passed" } else { "✗ Failed" }
    );
    
    Ok(())
}

/// Example 3: Complex hierarchical intent
async fn hierarchical_intent_example() -> Result<()> {
    info!("=== Example 3: Hierarchical Intent ===");
    
    // Create parent intent
    let mut parent_intent = HierarchicalIntent::new("Deploy application")
        .with_description("Complete application deployment process");
    
    // Create sub-intents
    let build_intent = IntentBuilder::new("Build application")
        .step("compile", StepAction::Command {
            command: "cargo".to_string(),
            args: vec!["build".to_string(), "--release".to_string()],
        })
        .step("run_tests", StepAction::Command {
            command: "cargo".to_string(),
            args: vec!["test".to_string()],
        })
        .build();
    
    let deploy_intent = IntentBuilder::new("Deploy to server")
        .step("package", StepAction::Custom(
            serde_json::json!({
                "action": "create_deployment_package",
                "format": "tar.gz"
            })
        ))
        .step("upload", StepAction::Custom(
            serde_json::json!({
                "action": "upload_to_server",
                "server": "production",
                "path": "/opt/app"
            })
        ))
        .build();
    
    // Add sub-intents to parent
    parent_intent = parent_intent.sub_intent(build_intent);
    parent_intent = parent_intent.sub_intent(deploy_intent);
    
    // Validate the intent structure
    parent_intent.validate().await?;
    info!("✓ Hierarchical intent structure validated");
    
    // Create composite verifier for multiple verification strategies
    let mut composite = CompositeVerifier::new();
    
    // Add command verification result
    composite.add_result(VerificationOutcome {
        passed: true,
        details: serde_json::json!({
            "step": "compile",
            "output": "Build successful"
        }),
        proof_id: Some(Uuid::new_v4()),
        timestamp: chrono::Utc::now(),
    });
    
    // Add file verification result
    composite.add_result(VerificationOutcome {
        passed: true,
        details: serde_json::json!({
            "step": "package",
            "file": "app.tar.gz",
            "size": 1024000
        }),
        proof_id: Some(Uuid::new_v4()),
        timestamp: chrono::Utc::now(),
    });
    
    let overall = composite.get_overall_result();
    info!("Overall verification: {} (confidence: {:.2})", 
        if overall.passed { "✓ Passed" } else { "✗ Failed" },
        composite.confidence_score()
    );
    
    Ok(())
}

/// Example 4: Intent with observability integration
async fn observable_intent_example(circuit: Arc<BasicCircuit>) -> Result<()> {
    info!("=== Example 4: Observable Intent ===");
    
    // Create a channel for intent events
    let subject = Subject::new("intent", "verification");
    let channel: Arc<dyn synapsed_substrates::Channel<String>> = 
        Arc::new(BasicChannel::new(subject.clone()));
    circuit.add_channel(channel.clone());
    
    // Create intent with observability
    let intent = IntentBuilder::new("Observable data processing")
        .description("Process data with full observability")
        .step("fetch_data", StepAction::Custom(
            serde_json::json!({
                "action": "fetch",
                "source": "api",
                "endpoint": "/data"
            })
        ))
        .step("transform_data", StepAction::Function {
            name: "transform_json".to_string(),
            args: vec!["input.json".to_string(), "output.json".to_string()],
        })
        .step("validate_output", StepAction::Custom(
            serde_json::json!({
                "action": "validate",
                "schema": "output_schema.json"
            })
        ))
        .build();
    
    // Emit intent creation event
    let pipe = channel.create_pipe("intent_events");
    pipe.emit(Emission::new(
        format!("Intent created: {}", intent.id().0),
        subject.clone()
    ));
    
    // Simulate step execution with events
    for (i, step_name) in ["fetch_data", "transform_data", "validate_output"].iter().enumerate() {
        debug!("Executing step: {}", step_name);
        
        // Emit step start event
        pipe.emit(Emission::new(
            format!("Step {} started: {}", i + 1, step_name),
            subject.clone()
        ));
        
        // Simulate processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Emit step completion event
        pipe.emit(Emission::new(
            format!("Step {} completed: {}", i + 1, step_name),
            subject.clone()
        ));
    }
    
    info!("✓ Observable intent execution completed");
    
    // Get circuit statistics
    let stats = circuit.get_statistics();
    info!("Circuit statistics: {} channels active", stats.channels_count);
    
    Ok(())
}

/// Helper function to create a mock verification strategy
fn create_mock_verifier() -> Arc<dyn VerificationStrategy> {
    struct MockVerifier;
    
    impl VerificationStrategy for MockVerifier {
        fn verify(&self, _data: &serde_json::Value) -> Result<VerificationOutcome> {
            Ok(VerificationOutcome {
                passed: true,
                details: serde_json::json!({"mock": true}),
                proof_id: Some(Uuid::new_v4()),
                timestamp: chrono::Utc::now(),
            })
        }
        
        fn strategy_type(&self) -> String {
            "mock".to_string()
        }
    }
    
    Arc::new(MockVerifier)
}