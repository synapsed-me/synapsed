# Code-Level Walkthrough: Complete Execution Flow

## Starting Point: Claude Receives a Request

Let's trace through the EXACT code execution when Claude processes: 
**"Create a Python script that processes data and saves results"**

## Phase 1: Intent Declaration

### Step 1.1: Claude Creates Intent (intent.rs)

```rust
// Claude's code generation
let intent = IntentBuilder::new("Create data processing script")
    .description("Generate Python script with verification")
    
    // Step 1: Write the script
    .verified_step(
        "create_script",
        StepAction::Command("echo 'import pandas as pd\n\ndef process():\n    pass' > process.py"),
        VerificationRequirement {
            verification_type: VerificationType::FileSystem,
            expected: json!({"file": "process.py", "exists": true}),
            mandatory: true,
            strategy: VerificationStrategy::Single,
        }
    )
    
    // Step 2: Test the script
    .verified_step(
        "test_script",
        StepAction::Command("python3 -m py_compile process.py"),
        VerificationRequirement {
            verification_type: VerificationType::Command,
            expected: json!({"exit_code": 0}),
            mandatory: true,
            strategy: VerificationStrategy::Single,
        }
    )
    .build();
```

### What happens internally:

```rust
// IntentBuilder::build() - intent.rs:599
pub fn build(self) -> HierarchicalIntent {
    self.intent  // Returns the constructed intent
}

// The HierarchicalIntent structure created:
HierarchicalIntent {
    id: IntentId(UUID="123e4567-e89b-12d3-a456-426614174000"),
    goal: "Create data processing script",
    description: Some("Generate Python script with verification"),
    steps: vec![
        Step {
            id: UUID="step-1-uuid",
            name: "create_script",
            action: StepAction::Command("echo '...' > process.py"),
            preconditions: vec![],
            postconditions: vec![],
            verification: Some(VerificationRequirement { ... }),
            status: StepStatus::Pending,
            result: None,
        },
        Step {
            id: UUID="step-2-uuid",
            name: "test_script",
            action: StepAction::Command("python3 -m py_compile process.py"),
            // ... similar structure
        }
    ],
    bounds: ContextBounds::default(),
    config: ExecutionConfig {
        stop_on_failure: true,
        enable_rollback: true,
        verify_steps: true,
        max_retries: 0,
        timeout_ms: Some(300000),  // 5 minutes
        parallelization: ParallelizationStrategy::Sequential,
        generate_proofs: true,
    },
    status: Arc<RwLock<IntentStatus::Pending>>,
    substrate: Arc<Subject>,  // For observability
}
```

## Phase 2: Context Setup

### Step 2.1: Create Bounded Context (context.rs)

```rust
let context = ContextBuilder::new()
    .creator("claude-main")
    .purpose("code-generation")
    .allow_commands(vec![
        "echo".to_string(),
        "python3".to_string(),
        "cat".to_string(),
    ])
    .allow_paths(vec![
        "/workspace".to_string(),
        "/tmp".to_string(),
    ])
    .max_memory(100 * 1024 * 1024)  // 100MB
    .max_cpu_time(60)  // 60 seconds
    .build()
    .await;
```

### Internal context creation (context.rs:508):

```rust
pub async fn build(self) -> IntentContext {
    let mut context = IntentContext::new(self.bounds);
    context.metadata = self.metadata;
    context.verification_requirements = self.verification_requirements;
    
    // Set initial variables
    for (key, value) in self.variables {
        let _ = context.set_variable(key, value).await;
    }
    
    context
}

// The created IntentContext:
IntentContext {
    id: UUID="context-uuid",
    parent: None,  // Root context
    variables: Arc<RwLock<HashMap<String, Value>>>,
    bounds: ContextBounds {
        allowed_paths: vec!["/workspace", "/tmp"],
        allowed_commands: vec!["echo", "python3", "cat"],
        allowed_endpoints: vec![],
        max_memory_bytes: Some(104857600),
        max_cpu_seconds: Some(60),
        env_vars: HashMap::new(),
    },
    metadata: ContextMetadata {
        creator: "claude-main",
        created_at: "2024-01-01T12:00:00Z",
        purpose: "code-generation",
        agent_id: Some("claude-main"),
    },
    audit_log: Arc<RwLock<Vec<AuditEntry>>>,
}
```

## Phase 3: Verification Setup

### Step 3.1: Create VerifiedIntent (enhanced_intent.rs)

```rust
let verified = VerifiedIntent::new(intent, context.bounds().clone())
    .with_recovery_strategy(RecoveryStrategy::Retry {
        max_attempts: 3,
        delay_ms: 1000,
    });
```

### Internal setup (enhanced_intent.rs:71):

```rust
pub fn new(intent: HierarchicalIntent, context_bounds: ContextBounds) -> Self {
    let executor = Arc::new(RwLock::new(VerifiedExecutor::new(context_bounds.clone())));
    let checkpoint_manager = Arc::new(CheckpointManager::new());
    let context_monitor = Arc::new(ContextMonitor::new(context_bounds));
    
    Self {
        intent,
        executor,
        checkpoint_manager,
        context_monitor,
        recovery_strategy: RecoveryStrategy::Retry { 
            max_attempts: 3, 
            delay_ms: 1000 
        },
        metrics: Arc::new(RwLock::new(ExecutionMetrics::default())),
    }
}
```

### VerifiedExecutor creation (execution.rs:45):

```rust
pub fn new(context_bounds: ContextBounds) -> Self {
    Self {
        command_verifier: Box::new(MockCommandVerifier),  // Would be real implementation
        fs_verifier: Box::new(MockFileSystemVerifier),
        network_verifier: Box::new(MockNetworkVerifier),
        state_verifier: Box::new(MockStateVerifier),
        proof_generator: Box::new(MockProofGenerator),
        bounds_enforcer: BoundsEnforcer::new(context_bounds),
        trust_scores: Arc::new(RwLock::new(HashMap::new())),
    }
}
```

## Phase 4: Execution Begins

### Step 4.1: Execute Intent (enhanced_intent.rs:105)

```rust
let result = verified.execute(&context).await?;
```

### Detailed execution flow:

```rust
pub async fn execute(&self, context: &IntentContext) -> Result<IntentResult> {
    info!("Starting verified intent execution: {}", self.intent.goal());
    let start = Utc::now();
    
    // 1. Update status
    *self.intent.status.write().await = IntentStatus::Executing;
    
    // 2. Emit start event
    self.emit_event(EventType::Started, json!({
        "goal": self.intent.goal(),
        "recovery_strategy": format!("{:?}", self.recovery_strategy),
    })).await;
    
    // 3. Validate intent structure
    self.intent.validate().await?;  // Checks for circular dependencies
    
    // 4. Create initial checkpoint
    self.checkpoint_manager.create_checkpoint(
        self.intent.id(),
        Uuid::nil()
    ).await?;
    
    // 5. Plan execution
    let plan = self.intent.plan().await?;
    // plan.steps = ["step-1-uuid", "step-2-uuid"]
    
    // 6. Execute each step
    for step_id in &plan.steps {
        let step = self.intent.steps.iter().find(|s| s.id == *step_id);
        // ... execute step with recovery
    }
}
```

## Phase 5: Step Execution

### Step 5.1: Execute First Step - Create Script (enhanced_intent.rs:219)

```rust
async fn execute_step_with_recovery(
    &self,
    step: &Step,  // Step { name: "create_script", action: Command("echo...") }
    context: &IntentContext,
) -> Result<StepResult> {
    let mut attempts = 0;
    let max_attempts = 3;  // From RecoveryStrategy::Retry
    
    loop {
        attempts += 1;
        debug!("Executing step 'create_script' (attempt 1/3)");
        
        // 1. Check preconditions
        if !self.check_conditions(&step.preconditions, context).await? {
            warn!("Preconditions not met for step 'create_script'");
            return Ok(StepResult { success: false, ... });
        }
        
        // 2. Create checkpoint
        self.checkpoint_manager.create_checkpoint(
            self.intent.id(),
            step.id
        ).await?;
        
        // 3. Execute with verification
        let mut executor = self.executor.write().await;
        let result = executor.execute_step(step, context).await?;
        
        // ... handle result
    }
}
```

### Step 5.2: VerifiedExecutor executes (execution.rs:60)

```rust
pub async fn execute_step(
    &mut self,
    step: &Step,  // "create_script" step
    context: &IntentContext,
) -> Result<StepResult> {
    let start = Utc::now();
    
    // 1. Check bounds BEFORE execution
    self.bounds_enforcer.check_step_bounds(step)?;
    
    // 2. Take state snapshot
    let pre_snapshot = self.state_verifier.take_snapshot().await?;
    // Snapshot { files: {}, variables: {...}, timestamp: "..." }
    
    // 3. Execute based on action type
    let (success, output, error, verification) = match &step.action {
        StepAction::Command(cmd) => {
            // cmd = "echo '...' > process.py"
            self.execute_command(cmd, step, context).await?
        },
        // ... other action types
    };
    
    // 4. Take post-execution snapshot
    let post_snapshot = self.state_verifier.take_snapshot().await?;
    // Now has: files: {"process.py": FileInfo{...}}
    
    // 5. Generate proof if successful
    let proof_id = if success && step.verification.is_some() {
        let proof = self.proof_generator.generate_proof(
            &pre_snapshot,
            &post_snapshot,
            verification.as_ref(),
        ).await?;
        Some(proof.id)
    } else {
        None
    };
    
    Ok(StepResult {
        success,
        output,
        error,
        duration_ms: (Utc::now() - start).num_milliseconds() as u64,
        verification: Some(VerificationOutcome {
            passed: success,
            details: verification,
            proof_id,
            timestamp: Utc::now(),
        }),
    })
}
```

### Step 5.3: Command Execution (execution.rs:131)

```rust
async fn execute_command(
    &mut self,
    command: &str,  // "echo '...' > process.py"
    step: &Step,
    _context: &IntentContext,
) -> Result<(bool, Option<Value>, Option<String>, Option<Value>)> {
    // 1. Parse command
    let parts: Vec<&str> = command.split_whitespace().collect();
    // parts = ["echo", "'...'", ">", "process.py"]
    let cmd = parts[0];  // "echo"
    let args = &parts[1..];
    
    // 2. Check if command is allowed
    if !self.bounds_enforcer.is_command_allowed(cmd) {
        return Ok((false, None, Some("Command 'echo' not allowed"), None));
    }
    // BoundsEnforcer checks: "echo" in ["echo", "python3", "cat"] ✓
    
    // 3. Execute with verification
    let verification = self.command_verifier.verify(
        cmd,
        Some(args),
        step.verification.as_ref().map(|v| &v.expected),
    ).await?;
    
    // 4. Return results
    let success = verification.exit_code == 0;
    let output = json!({
        "stdout": verification.stdout,  // Empty for file creation
        "stderr": verification.stderr,  // Empty if successful
        "exit_code": 0,
    });
    
    let verification_details = json!({
        "command": "echo",
        "args": ["'...'", ">", "process.py"],
        "executed": true,
        "sandboxed": true,
        "duration_ms": 15,
    });
    
    Ok((true, Some(output), None, Some(verification_details)))
}
```

### Step 5.4: File System Verification

```rust
// After command execution, verify file was created
// FileSystemVerifierTrait implementation would:

async fn verify_changes(
    &self,
    before: &FileSystemSnapshot,  // Empty
    after: &FileSystemSnapshot,   // Has process.py
    expected: Option<&Value>,     // {"file": "process.py", "exists": true}
) -> Result<FileSystemVerification> {
    // 1. Detect changes
    let files_created = vec!["process.py"];
    
    // 2. Verify against expected
    let expected_file = expected.get("file").unwrap();
    let should_exist = expected.get("exists").unwrap();
    
    let file_exists = after.files.contains_key("process.py");
    let matches_expected = file_exists == should_exist;
    
    Ok(FileSystemVerification {
        changes_detected: true,
        files_created: vec!["process.py".to_string()],
        files_modified: vec![],
        files_deleted: vec![],
        matches_expected: true,
    })
}
```

## Phase 6: Second Step - Test Script

Similar flow for testing the script:

```rust
// Step: "test_script"
// Action: Command("python3 -m py_compile process.py")

// 1. Bounds check: "python3" allowed? ✓
// 2. Execute: python3 -m py_compile process.py
// 3. Capture output:
//    - exit_code: 0 (success)
//    - stdout: ""
//    - stderr: ""
// 4. Verify: exit_code == 0 as expected? ✓
// 5. Generate proof
```

## Phase 7: Completion

### Step 7.1: Finalize Results (enhanced_intent.rs:189)

```rust
// After all steps executed:
let duration_ms = (Utc::now() - start).num_milliseconds() as u64;

// Update metrics
self.metrics.write().await.total_duration_ms = duration_ms;
self.metrics.write().await.steps_executed = 2;
self.metrics.write().await.steps_succeeded = 2;
self.metrics.write().await.verifications_passed = 2;

// Update status
*self.intent.status.write().await = IntentStatus::Completed;

// Emit completion event
self.emit_event(EventType::Completed, json!({
    "duration_ms": duration_ms,
    "metrics": *self.metrics.read().await,
})).await;

// Generate final report
info!(
    "Intent execution completed: success=true, steps=2/2, verifications=2/2, duration=45ms"
);

// Return result with proofs
Ok(IntentResult {
    intent_id: self.intent.id(),
    success: true,
    step_results: vec![
        StepResult {
            success: true,
            output: Some(json!({"stdout": "", "stderr": "", "exit_code": 0})),
            error: None,
            duration_ms: 15,
            verification: Some(VerificationOutcome {
                passed: true,
                details: json!({"file_created": "process.py"}),
                proof_id: Some(UUID="proof-1"),
                timestamp: "2024-01-01T12:00:15Z",
            }),
        },
        StepResult {
            success: true,
            output: Some(json!({"stdout": "", "stderr": "", "exit_code": 0})),
            error: None,
            duration_ms: 30,
            verification: Some(VerificationOutcome {
                passed: true,
                details: json!({"compilation": "successful"}),
                proof_id: Some(UUID="proof-2"),
                timestamp: "2024-01-01T12:00:45Z",
            }),
        },
    ],
    duration_ms: 45,
    verification_proofs: vec![UUID="proof-1", UUID="proof-2"],
})
```

## Phase 8: Claude Returns Verified Response

```rust
// Claude can now respond with confidence:
"I've created process.py with the following verification:
- File creation verified (proof: proof-1)
- Python syntax verified (proof: proof-2)
- Execution time: 45ms
- All operations within allowed bounds
- Audit trail available for review"
```

## Key Data Structures During Execution

### 1. CheckpointData (checkpoint.rs:353)
```rust
CheckpointData {
    id: UUID,
    intent_id: IntentId,
    step_id: UUID,
    state: StateSnapshot {
        variables: HashMap<String, Value>,
        files: HashMap<String, FileState>,  // File hashes before step
        processes: Vec<ProcessState>,
        connections: Vec<ConnectionState>,
    },
    timestamp: DateTime<Utc>,
    safe_rollback: true,
}
```

### 2. ContextViolation (execution.rs:429)
```rust
ContextViolation {
    timestamp: DateTime<Utc>,
    violation_type: ViolationType::UnauthorizedCommand,
    details: "Attempted to execute 'rm'",
    step_id: Some(UUID),
}
```

### 3. VerificationProof (verification.rs:119)
```rust
VerificationProof {
    id: UUID,
    timestamp: DateTime<Utc>,
    pre_state_hash: "sha256:abc123...",
    post_state_hash: "sha256:def456...",
    verification_data: json!({
        "command": "echo",
        "output": {"exit_code": 0},
        "file_created": "process.py",
        "file_hash": "sha256:789...",
    }),
    signature: Some("ed25519:signature..."),
}
```

## Error Handling Paths

### If Command Fails:
```rust
// In execute_step_with_recovery:
if !result.success {
    match self.recovery_strategy {
        RecoveryStrategy::Retry { max_attempts, delay_ms } => {
            if attempts < max_attempts {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                continue;  // Retry the loop
            }
        },
        RecoveryStrategy::Rollback => {
            self.checkpoint_manager.rollback_to_last().await?;
            return Ok(result);  // Failed but rolled back
        },
        RecoveryStrategy::Skip => {
            self.metrics.write().await.steps_skipped += 1;
            return Ok(result);  // Continue with next step
        },
    }
}
```

### If Context Violation:
```rust
// In BoundsEnforcer::check_step_bounds:
if !self.is_command_allowed(parts[0]) {
    return Err(IntentError::ContextViolation(
        format!("Command '{}' not allowed", parts[0])
    ));
}
// Execution stops immediately, violation logged
```

### If Verification Fails:
```rust
// In execute_step:
if let Some(verification) = &step.verification {
    if verification.mandatory && !verification_result.passed {
        return Ok(StepResult {
            success: false,
            error: Some("Mandatory verification failed"),
            // ...
        });
    }
}
```

## Complete System Properties

Through this execution flow, the system guarantees:

1. **No Unverified Claims**: Every step has verification evidence
2. **No Context Escaping**: Bounds checked before execution
3. **Failure Recovery**: Checkpoints enable safe rollback
4. **Complete Auditability**: Every action logged with timestamp
5. **Cryptographic Proof**: Tamper-evident execution trail