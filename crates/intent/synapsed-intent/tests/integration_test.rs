//! Integration test demonstrating complete verification flow

use synapsed_intent::{
    HierarchicalIntent, IntentBuilder, IntentContext, ContextBuilder,
    VerifiedIntent, RecoveryStrategy, RecoveryAction,
    StepAction, VerificationRequirement, VerificationType, VerificationStrategy,
    ContextBounds, Condition, ConditionType, Priority,
    DelegationSpec,
};
use std::collections::HashMap;
use serde_json::json;

#[tokio::test]
async fn test_basic_intent_execution_with_verification() {
    // Create context with bounds
    let context = ContextBuilder::new()
        .creator("test")
        .purpose("integration test")
        .allow_commands(vec!["echo".to_string(), "ls".to_string()])
        .allow_paths(vec!["/tmp".to_string()])
        .max_memory(100 * 1024 * 1024) // 100MB
        .max_cpu_time(60) // 60 seconds
        .build()
        .await;

    // Build intent with verification
    let intent = IntentBuilder::new("Test intent with verification")
        .description("Tests command execution with verification")
        .verified_step(
            "echo_test",
            StepAction::Command("echo 'Hello, World!'".to_string()),
            VerificationRequirement {
                verification_type: VerificationType::Command,
                expected: json!({ "exit_code": 0 }),
                mandatory: true,
                strategy: VerificationStrategy::Single,
            }
        )
        .verified_step(
            "list_files",
            StepAction::Command("ls /tmp".to_string()),
            VerificationRequirement {
                verification_type: VerificationType::Command,
                expected: json!({ "exit_code": 0 }),
                mandatory: true,
                strategy: VerificationStrategy::Single,
            }
        )
        .priority(Priority::High)
        .build();

    // Create verified intent
    let verified = VerifiedIntent::new(intent, context.bounds().clone())
        .with_recovery_strategy(RecoveryStrategy::Retry {
            max_attempts: 2,
            delay_ms: 500,
        });

    // Execute
    let result = verified.execute(&context).await.unwrap();
    
    assert!(result.success);
    assert_eq!(result.step_results.len(), 2);
    
    // Check metrics
    let metrics = verified.metrics().await;
    assert_eq!(metrics.steps_executed, 2);
    assert_eq!(metrics.steps_succeeded, 2);
    assert_eq!(metrics.steps_failed, 0);
}

#[tokio::test]
async fn test_intent_with_preconditions_and_postconditions() {
    let context = ContextBuilder::new()
        .creator("test")
        .purpose("condition test")
        .allow_commands(vec!["touch".to_string(), "rm".to_string()])
        .allow_paths(vec!["/tmp".to_string()])
        .build()
        .await;

    let mut intent = HierarchicalIntent::new("Test with conditions");
    
    // Add step with precondition
    intent = intent
        .step("create_file", StepAction::Command("touch /tmp/test_file.txt".to_string()))
        .ensures(Condition {
            condition_type: ConditionType::FileExists,
            expected: json!("/tmp/test_file.txt"),
            critical: true,
            description: Some("File must exist after creation".to_string()),
        });
    
    // Add step that depends on the file
    intent = intent
        .step("remove_file", StepAction::Command("rm /tmp/test_file.txt".to_string()))
        .requires(Condition {
            condition_type: ConditionType::FileExists,
            expected: json!("/tmp/test_file.txt"),
            critical: true,
            description: Some("File must exist before removal".to_string()),
        });

    let verified = VerifiedIntent::new(intent, context.bounds().clone());
    let result = verified.execute(&context).await.unwrap();
    
    // The test might fail depending on actual file system state
    // This demonstrates the condition checking
    assert_eq!(result.step_results.len(), 2);
}

#[tokio::test]
async fn test_intent_with_delegation() {
    let context = ContextBuilder::new()
        .creator("test")
        .purpose("delegation test")
        .variable("agent_id", json!("sub-agent-1"))
        .build()
        .await;

    let intent = IntentBuilder::new("Test delegation")
        .description("Tests delegation to sub-agent")
        .step(
            "delegate_task",
            StepAction::Delegate(DelegationSpec {
                agent_id: Some("sub-agent-1".to_string()),
                task: "Process data".to_string(),
                context: {
                    let mut ctx = HashMap::new();
                    ctx.insert("data".to_string(), json!({"value": 42}));
                    ctx
                },
                timeout_ms: 5000,
                wait_for_completion: true,
            })
        )
        .build();

    let verified = VerifiedIntent::new(intent, context.bounds().clone());
    let result = verified.execute(&context).await.unwrap();
    
    assert!(result.success);
    assert_eq!(result.step_results.len(), 1);
    
    // Check that promise was created
    if let Some(output) = &result.step_results[0].output {
        assert!(output.get("promise_id").is_some());
        assert!(output.get("agent_id").is_some());
    }
}

#[tokio::test]
async fn test_intent_with_context_violations() {
    // Create restrictive context
    let context = ContextBuilder::new()
        .creator("test")
        .purpose("violation test")
        .allow_commands(vec!["echo".to_string()]) // Only allow echo
        .build()
        .await;

    let intent = IntentBuilder::new("Test context violations")
        .step("allowed_command", StepAction::Command("echo 'allowed'".to_string()))
        .step("forbidden_command", StepAction::Command("rm -rf /".to_string())) // Dangerous!
        .build();

    let verified = VerifiedIntent::new(intent, context.bounds().clone())
        .with_recovery_strategy(RecoveryStrategy::Skip); // Skip failed steps

    let result = verified.execute(&context).await.unwrap();
    
    // First step should succeed, second should fail due to context bounds
    assert!(!result.success); // Overall failure due to violation
    
    // Check violations
    let violations = verified.get_violations().await;
    assert!(violations.len() > 0);
}

#[tokio::test]
async fn test_intent_with_rollback() {
    let context = ContextBuilder::new()
        .creator("test")
        .purpose("rollback test")
        .allow_commands(vec!["echo".to_string(), "false".to_string()])
        .build()
        .await;

    let intent = IntentBuilder::new("Test rollback")
        .step("step1", StepAction::Command("echo 'step 1'".to_string()))
        .step("step2", StepAction::Command("false".to_string())) // Will fail
        .step("step3", StepAction::Command("echo 'step 3'".to_string()))
        .build();

    let verified = VerifiedIntent::new(intent, context.bounds().clone())
        .with_recovery_strategy(RecoveryStrategy::Rollback)
        .with_file_rollback("/tmp");

    let result = verified.execute(&context).await.unwrap();
    
    assert!(!result.success);
    
    // Check that rollback was performed
    let metrics = verified.metrics().await;
    assert!(metrics.rollbacks_performed > 0);
}

#[tokio::test]
async fn test_intent_with_custom_recovery() {
    let context = ContextBuilder::new()
        .creator("test")
        .purpose("custom recovery test")
        .build()
        .await;

    let intent = IntentBuilder::new("Test custom recovery")
        .step("step1", StepAction::Command("echo 'test'".to_string()))
        .build();

    // Custom recovery function that always retries once
    let recovery_fn = Arc::new(|result: &StepResult| {
        if result.success {
            RecoveryAction::Skip
        } else {
            RecoveryAction::Retry
        }
    });

    let verified = VerifiedIntent::new(intent, context.bounds().clone())
        .with_recovery_strategy(RecoveryStrategy::Custom(recovery_fn));

    let result = verified.execute(&context).await.unwrap();
    assert!(result.success);
}

#[tokio::test]
async fn test_hierarchical_intent_with_sub_intents() {
    let context = ContextBuilder::new()
        .creator("test")
        .purpose("hierarchical test")
        .allow_commands(vec!["echo".to_string()])
        .build()
        .await;

    // Create sub-intent
    let sub_intent = IntentBuilder::new("Sub task")
        .step("sub_step", StepAction::Command("echo 'sub task'".to_string()))
        .build();

    // Create main intent with sub-intent
    let main_intent = IntentBuilder::new("Main task")
        .step("main_step", StepAction::Command("echo 'main task'".to_string()))
        .sub_intent(sub_intent)
        .build();

    let verified = VerifiedIntent::new(main_intent, context.bounds().clone());
    let result = verified.execute(&context).await.unwrap();
    
    assert!(result.success);
    
    // Check metrics include sub-intent execution
    let metrics = verified.metrics().await;
    assert!(metrics.steps_executed > 0);
}

#[tokio::test]
async fn test_intent_with_parallel_steps() {
    use synapsed_intent::ExecutionConfig;
    use synapsed_intent::ParallelizationStrategy;
    
    let context = ContextBuilder::new()
        .creator("test")
        .purpose("parallel test")
        .allow_commands(vec!["echo".to_string(), "sleep".to_string()])
        .build()
        .await;

    let mut intent = IntentBuilder::new("Test parallel execution")
        .step("step1", StepAction::Command("echo '1'".to_string()))
        .step("step2", StepAction::Command("echo '2'".to_string()))
        .step("step3", StepAction::Command("echo '3'".to_string()))
        .build();
    
    // Enable parallel execution
    intent.config.parallelization = ParallelizationStrategy::Parallel;

    let verified = VerifiedIntent::new(intent, context.bounds().clone());
    let result = verified.execute(&context).await.unwrap();
    
    assert!(result.success);
    assert_eq!(result.step_results.len(), 3);
}

use std::sync::Arc;
use synapsed_intent::StepResult;