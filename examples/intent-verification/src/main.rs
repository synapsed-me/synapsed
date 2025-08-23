//! Example demonstrating complete intent-promise-verify integration
//!
//! This example shows how the synapsed-intent system prevents AI agents
//! from escaping context and making false claims through:
//! 1. Hierarchical intent declaration
//! 2. Promise-based cooperation
//! 3. Multi-strategy verification
//! 4. Context boundary enforcement

use synapsed_intent::{
    IntentBuilder, ContextBuilder, VerifiedIntent,
    StepAction, DelegationSpec, VerificationRequirement,
    VerificationType, VerificationStrategy, RecoveryStrategy,
    Condition, ConditionType, Priority,
};
// Note: In production, synapsed-promise would be used for agent cooperation
// This example shows the integration patterns without creating cyclic dependencies
use synapsed_verify::{
    CommandVerifier, CommandVerifierConfig,
    FileSystemVerifier, NetworkVerifier,
};
use synapsed_substrates::{BasicCircuit, BasicChannel, Subject};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use serde_json::json;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    info!("Starting Intent-Promise-Verify Integration Example");
    
    // Demonstrate the complete flow
    demonstrate_verified_execution().await?;
    demonstrate_context_escaping_prevention().await?;
    demonstrate_false_claim_detection().await?;
    demonstrate_promise_based_delegation().await?;
    
    info!("Example completed successfully");
    Ok(())
}

/// Demonstrates basic verified execution with command verification
async fn demonstrate_verified_execution() -> anyhow::Result<()> {
    info!("\n=== Demonstrating Verified Execution ===");
    
    // Create a context with specific bounds
    let context = ContextBuilder::new()
        .creator("main-agent")
        .purpose("data-processing")
        .allow_commands(vec![
            "echo".to_string(),
            "cat".to_string(),
            "grep".to_string(),
            "wc".to_string(),
        ])
        .allow_paths(vec!["/tmp".to_string(), "/var/tmp".to_string()])
        .max_memory(100 * 1024 * 1024) // 100MB limit
        .max_cpu_time(30) // 30 seconds limit
        .variable("input_file", json!("/tmp/data.txt"))
        .variable("output_file", json!("/tmp/results.txt"))
        .build()
        .await;
    
    // Build an intent with verification requirements
    let intent = IntentBuilder::new("Process and verify data")
        .description("Demonstrates command execution with verification")
        .priority(Priority::High)
        
        // Step 1: Create test data
        .verified_step(
            "create_data",
            StepAction::Command("echo 'test data\nmore data\nfinal data' > /tmp/data.txt".to_string()),
            VerificationRequirement {
                verification_type: VerificationType::FileSystem,
                expected: json!({ 
                    "file": "/tmp/data.txt",
                    "exists": true 
                }),
                mandatory: true,
                strategy: VerificationStrategy::Single,
            }
        )
        
        // Step 2: Process data with verification
        .verified_step(
            "process_data",
            StepAction::Command("grep 'data' /tmp/data.txt | wc -l > /tmp/results.txt".to_string()),
            VerificationRequirement {
                verification_type: VerificationType::Command,
                expected: json!({ "exit_code": 0 }),
                mandatory: true,
                strategy: VerificationStrategy::Single,
            }
        )
        
        // Step 3: Verify results
        .verified_step(
            "verify_results",
            StepAction::Command("cat /tmp/results.txt".to_string()),
            VerificationRequirement {
                verification_type: VerificationType::Command,
                expected: json!({ "exit_code": 0 }),
                mandatory: true,
                strategy: VerificationStrategy::Single,
            }
        )
        .build();
    
    // Create verified intent with recovery strategy
    let verified = VerifiedIntent::new(intent, context.bounds().clone())
        .with_recovery_strategy(RecoveryStrategy::Retry {
            max_attempts: 3,
            delay_ms: 1000,
        })
        .with_file_rollback("/tmp");
    
    // Execute with full verification
    let result = verified.execute(&context).await?;
    
    // Report results
    info!("Execution completed: success={}", result.success);
    info!("Steps executed: {}", result.step_results.len());
    
    let metrics = verified.metrics().await;
    info!("Metrics: {:?}", metrics);
    
    if result.success {
        info!("✅ All commands executed and verified successfully");
    } else {
        warn!("⚠️ Some steps failed verification");
    }
    
    Ok(())
}

/// Demonstrates how context boundaries prevent unauthorized operations
async fn demonstrate_context_escaping_prevention() -> anyhow::Result<()> {
    info!("\n=== Demonstrating Context Escaping Prevention ===");
    
    // Create a restrictive context
    let context = ContextBuilder::new()
        .creator("restricted-agent")
        .purpose("limited-scope")
        .allow_commands(vec!["echo".to_string(), "ls".to_string()])
        .allow_paths(vec!["/tmp".to_string()])
        .allow_endpoints(vec!["https://api.example.com".to_string()])
        .build()
        .await;
    
    // Build an intent that tries to escape context
    let intent = IntentBuilder::new("Attempt context escape")
        .description("This intent will be blocked by context bounds")
        
        // Allowed operation
        .step("allowed_op", StepAction::Command("echo 'This is allowed'".to_string()))
        
        // Attempt to use forbidden command (will be blocked)
        .step("forbidden_command", StepAction::Command("rm -rf /important/file".to_string()))
        
        // Attempt to access forbidden path (will be blocked)
        .step("forbidden_path", StepAction::Command("cat /etc/passwd".to_string()))
        
        // Attempt to access forbidden network (will be blocked)
        .step("forbidden_network", StepAction::Function(
            "http_request".to_string(),
            vec![json!("https://evil.com"), json!("GET")]
        ))
        .build();
    
    let verified = VerifiedIntent::new(intent, context.bounds().clone())
        .with_recovery_strategy(RecoveryStrategy::Skip); // Skip failed steps
    
    let result = verified.execute(&context).await?;
    
    // Check violations
    let violations = verified.get_violations().await;
    info!("Context violations detected: {}", violations.len());
    
    for violation in violations {
        warn!("❌ Violation: {:?} - {}", violation.violation_type, violation.details);
    }
    
    if violations.len() > 0 {
        info!("✅ Context boundaries successfully prevented unauthorized operations");
    }
    
    Ok(())
}

/// Demonstrates detection of false claims through verification
async fn demonstrate_false_claim_detection() -> anyhow::Result<()> {
    info!("\n=== Demonstrating False Claim Detection ===");
    
    let context = ContextBuilder::new()
        .creator("verifier-agent")
        .purpose("claim-verification")
        .allow_commands(vec!["echo".to_string(), "touch".to_string(), "ls".to_string()])
        .allow_paths(vec!["/tmp".to_string()])
        .build()
        .await;
    
    // Create an intent that makes claims about its actions
    let intent = IntentBuilder::new("Verify agent claims")
        .description("Ensures agents actually do what they claim")
        
        // Step with postcondition that must be verified
        .step("create_file", StepAction::Command("touch /tmp/verified_file.txt".to_string()))
        .ensures(Condition {
            condition_type: ConditionType::FileExists,
            expected: json!("/tmp/verified_file.txt"),
            critical: true,
            description: Some("File must exist after creation".to_string()),
        })
        
        // Step that claims to create multiple files (but might fail)
        .verified_step(
            "create_multiple",
            StepAction::Command("echo 'Creating files...'".to_string()), // Doesn't actually create files!
            VerificationRequirement {
                verification_type: VerificationType::FileSystem,
                expected: json!({
                    "files": ["/tmp/file1.txt", "/tmp/file2.txt"],
                    "all_exist": true
                }),
                mandatory: true,
                strategy: VerificationStrategy::All, // All verifiers must agree
            }
        )
        .build();
    
    let verified = VerifiedIntent::new(intent, context.bounds().clone());
    let result = verified.execute(&context).await?;
    
    // Check which claims were verified
    for (i, step_result) in result.step_results.iter().enumerate() {
        if let Some(verification) = &step_result.verification {
            if verification.passed {
                info!("✅ Step {} claim verified", i + 1);
            } else {
                warn!("❌ Step {} false claim detected!", i + 1);
            }
        }
    }
    
    info!("False claim detection completed");
    Ok(())
}

/// Demonstrates promise-based delegation between agents
async fn demonstrate_promise_based_delegation() -> anyhow::Result<()> {
    info!("\n=== Demonstrating Promise-Based Delegation ===");
    
    // In production, we would create agents using synapsed-promise
    // For this example, we'll simulate agent creation
    info!("Creating main and sub agents (simulated)");
    
    // Create context for delegation
    let context = ContextBuilder::new()
        .creator("main-agent")
        .purpose("distributed-processing")
        .allow_commands(vec!["echo".to_string(), "python3".to_string()])
        .variable("trust_model", json!({}))
        .build()
        .await;
    
    // Build intent with delegation
    let intent = IntentBuilder::new("Coordinate distributed task")
        .description("Delegates work to sub-agents with promises")
        
        // Step 1: Prepare task
        .step("prepare", StepAction::Command("echo 'Preparing task for delegation'".to_string()))
        
        // Step 2: Delegate to sub-agent with promise
        .verified_step(
            "delegate_processing",
            StepAction::Delegate(DelegationSpec {
                agent_id: Some("sub-agent-1".to_string()),
                task: "Process dataset and compute statistics".to_string(),
                context: {
                    let mut ctx = HashMap::new();
                    ctx.insert("dataset".to_string(), json!({
                        "path": "/tmp/data.csv",
                        "format": "csv",
                        "size": 1000
                    }));
                    ctx.insert("requirements".to_string(), json!({
                        "compute_mean": true,
                        "compute_std": true,
                        "generate_report": true
                    }));
                    ctx
                },
                timeout_ms: 10000,
                wait_for_completion: true,
            }),
            VerificationRequirement {
                verification_type: VerificationType::Custom,
                expected: json!({
                    "promise_fulfilled": true,
                    "trust_maintained": true
                }),
                mandatory: true,
                strategy: VerificationStrategy::Consensus(2), // Multiple verifiers
            }
        )
        
        // Step 3: Verify delegation results
        .step("verify_results", StepAction::Function(
            "verify_delegation".to_string(),
            vec![json!("sub-agent-1"), json!("processing_task")]
        ))
        .build();
    
    // Create verified intent
    let verified = VerifiedIntent::new(intent, context.bounds().clone());
    
    // Execute with promise tracking
    info!("Starting delegation with promise tracking...");
    let result = verified.execute(&context).await?;
    
    // Check promise fulfillment
    if result.success {
        info!("✅ Delegation completed successfully with promise fulfillment");
        
        // In a real scenario, we would check the trust model update
        info!("Trust model updated based on promise fulfillment");
    } else {
        warn!("⚠️ Delegation failed or promise not fulfilled");
    }
    
    // Report on the delegation
    for step_result in &result.step_results {
        if let Some(output) = &step_result.output {
            if let Some(promise_id) = output.get("promise_id") {
                info!("Promise ID: {}", promise_id);
            }
            if let Some(trust_score) = output.get("trust_score") {
                info!("Updated trust score: {}", trust_score);
            }
        }
    }
    
    Ok(())
}

/// Example of a complete AI agent task with full verification
async fn example_ai_agent_task() -> anyhow::Result<()> {
    info!("\n=== Complete AI Agent Task Example ===");
    
    // This demonstrates how Claude sub-agents would be constrained
    let context = ContextBuilder::new()
        .creator("claude-main")
        .purpose("code-generation-task")
        .allow_commands(vec![
            "git".to_string(),
            "cargo".to_string(),
            "rustc".to_string(),
            "echo".to_string(),
        ])
        .allow_paths(vec![
            "/workspace".to_string(),
            "/tmp".to_string(),
        ])
        .allow_endpoints(vec![
            "https://api.github.com".to_string(),
            "https://crates.io".to_string(),
        ])
        .max_memory(512 * 1024 * 1024) // 512MB
        .max_cpu_time(300) // 5 minutes
        .variable("project_path", json!("/workspace/project"))
        .variable("requirements", json!({
            "language": "rust",
            "framework": "tokio",
            "must_compile": true,
            "must_pass_tests": true
        }))
        .build()
        .await;
    
    let intent = IntentBuilder::new("Generate and verify Rust code")
        .description("AI agent generates code with full verification")
        .priority(Priority::Critical)
        
        // Declare what we're going to do
        .step("declare_intent", StepAction::Function(
            "declare_to_user".to_string(),
            vec![json!("I will generate a Rust async function with error handling")]
        ))
        
        // Generate code (simulated)
        .step("generate_code", StepAction::Command(
            "echo 'async fn process() -> Result<()> { Ok(()) }' > /tmp/generated.rs".to_string()
        ))
        
        // Verify the code compiles
        .verified_step(
            "verify_compilation",
            StepAction::Command("rustc --edition 2021 --crate-type lib /tmp/generated.rs".to_string()),
            VerificationRequirement {
                verification_type: VerificationType::Command,
                expected: json!({ "exit_code": 0 }),
                mandatory: true,
                strategy: VerificationStrategy::All,
            }
        )
        
        // Verify code quality
        .verified_step(
            "verify_quality",
            StepAction::Command("cargo clippy --all-targets -- -D warnings".to_string()),
            VerificationRequirement {
                verification_type: VerificationType::Command,
                expected: json!({ "exit_code": 0 }),
                mandatory: false, // Warning only
                strategy: VerificationStrategy::Single,
            }
        )
        .build();
    
    let verified = VerifiedIntent::new(intent, context.bounds().clone())
        .with_recovery_strategy(RecoveryStrategy::Retry {
            max_attempts: 2,
            delay_ms: 2000,
        });
    
    let result = verified.execute(&context).await?;
    
    if result.success {
        info!("✅ AI agent successfully generated and verified code");
        info!("All claims about code generation were verified");
    } else {
        error!("❌ AI agent failed to generate valid code or made false claims");
    }
    
    Ok(())
}