# Synapsed Integration Guide

## Overview

This guide documents how to integrate Synapsed's intent verification and Promise Theory implementation with AI agents, particularly Claude Code, while maintaining model-agnostic architecture.

## Core Principles

### 1. Promise Theory Implementation
Our implementation follows Mark Burgess's Promise Theory with these key principles:

- **Autonomy**: Agents are causally independent and cannot be coerced
- **Voluntary Cooperation**: All cooperation is voluntary through explicit acceptance
- **Semantic Spacetime**: Promises have spatial, temporal, and semantic contexts
- **Promise Chemistry**: Promises can compose, decompose, catalyze, or inhibit each other

### 2. Intent Verification
The hierarchical intent system ensures verifiable execution:

- **Pre-declaration**: Agents declare intentions before acting
- **Multi-strategy Verification**: Command, filesystem, API, and composite verification
- **Cryptographic Proofs**: Tamper-proof evidence of execution
- **Context Boundaries**: Strict enforcement of operational limits

## Claude Code Integration

### Hooks Configuration
The `.claude/hooks.json` file captures agent interactions:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "name": "intent-declaration",
        "matcher": "Write|Edit|MultiEdit",
        "hooks": [{
          "type": "command",
          "command": "cargo run --bin synapsed-mcp -- intent declare"
        }]
      }
    ],
    "PostToolUse": [
      {
        "name": "intent-verification",
        "matcher": ".*",
        "hooks": [{
          "type": "command",
          "command": "cargo run --bin synapsed-mcp -- intent verify"
        }]
      }
    ],
    "Task": [
      {
        "name": "sub-agent-context",
        "hooks": [{
          "type": "command",
          "command": "cargo run --bin synapsed-mcp -- agent spawn"
        }]
      }
    ]
  }
}
```

### MCP Server Tools
The synapsed-mcp server provides these tools:

- `intent_declare` - Declare an intent before execution
- `intent_verify` - Verify execution against declaration
- `context_inject` - Pass context to sub-agents
- `trust_check` - Query agent trust levels
- `promise_make` - Create a new promise
- `promise_accept` - Accept an imposition as a promise

## Agent Communication (FIPA ACL)

### Performatives
We support standard FIPA ACL performatives for agent communication:

```rust
// Assertives
Inform, InformIf, Confirm, Disconfirm

// Directives  
Request, QueryIf, Subscribe

// Commissives
Agree, Refuse

// Expressives
Failure, NotUnderstood

// Declarations
AcceptProposal, RejectProposal, Propose
```

### Message Structure
```rust
let message = ACLMessageBuilder::new()
    .performative(Performative::Request)
    .sender(agent_id)
    .receiver(target_agent)
    .content(MessageContent::Promise(promise_body))
    .protocol(InteractionProtocol::RequestResponse)
    .build()?;
```

### Conversation Management
Multi-turn dialogues with state tracking:

```rust
let conv_id = manager.start_conversation(
    initiator,
    participants,
    InteractionProtocol::ContractNet
);

manager.add_message(&conv_id, message)?;
manager.update_phase(&conv_id, ConversationPhase::Negotiating)?;
```

## Voluntary Cooperation Protocol

### Willingness Evaluation
Before accepting any imposition or making a promise:

```rust
let willingness = evaluator.evaluate_promise_willingness(
    agent_id,
    &promise_type,
    &body,
    &context
).await?;

match willingness {
    Willingness::Willing { confidence } => {
        // Proceed with promise
    },
    Willingness::Conditional { conditions, confidence } => {
        // Negotiate conditions
    },
    Willingness::Unwilling { reason } => {
        // Refuse politely
    },
    Willingness::Uncertain { missing_info } => {
        // Request more information
    }
}
```

### Causal Independence Verification
Ensure agents maintain autonomy:

```rust
let verifier = CausalIndependenceVerifier;
let is_independent = verifier.verify_independence(
    agent_id,
    &other_agents
).await?;
```

## Observability Integration

### Substrates Event Circuits
All agent actions emit events through Substrates:

```rust
// Create event circuit
let circuit = BasicCircuit::new();
let channel = BasicChannel::new(Name::from("intent.events"));

// Emit intent event
channel.emit(IntentEvent::Started {
    intent_id,
    agent_id,
    timestamp: Utc::now(),
})?;
```

### Subject-Based Modeling
Agents are modeled as subjects, not resources:

```rust
let subject = Subject::new(
    Name::from(format!("agent.{}", agent_id)),
    SubjectType::Source
);
```

## Usage Examples

### Basic Intent Declaration and Verification

```rust
// 1. Declare intent
let intent = HierarchicalIntent::new(
    "Update configuration file",
    vec![
        Step::new("Read current config", StepAction::Command("cat config.toml")),
        Step::new("Modify values", StepAction::Function("update_config")),
        Step::new("Write new config", StepAction::Command("echo ... > config.toml")),
    ]
);

// 2. Execute with verification
let executor = VerifiedExecutor::new();
let result = executor.execute_with_verification(&intent).await?;

// 3. Generate proof
let proof = ProofGenerator::generate(&intent, &result)?;
```

### Promise-Based Cooperation

```rust
// Agent A makes a promise
let promise_body = PromiseBody {
    content: "Will provide data processing service".to_string(),
    constraints: vec![],
    qos: Some(QualityOfService {
        availability: 0.99,
        response_time_ms: Some(100),
        throughput: Some(1000),
        reliability: 0.999,
    }),
    metadata: HashMap::new(),
};

let promise = agent_a.make_promise(
    PromiseType::Offer,
    PromiseScope::Universal,
    promise_body
).await?;

// Agent B evaluates and accepts
let willingness = agent_b.evaluate_imposition_willingness(
    &imposition,
    trust_score
).await?;

if matches!(willingness, Willingness::Willing { .. }) {
    let accepted_promise = agent_b.accept_imposition(imposition).await?;
}
```

### Multi-Agent Communication

```rust
// Start contract net protocol
let conv_id = conversation_manager.start_conversation(
    initiator,
    participants,
    InteractionProtocol::ContractNet
);

// Send call for proposals
let cfp = ACLMessageBuilder::new()
    .performative(Performative::CallForProposal)
    .sender(initiator)
    .receivers(participants)
    .content(MessageContent::Text("Need data processing service"))
    .conversation(conv_id.clone())
    .build()?;

// Participants respond with proposals
for participant in participants {
    let proposal = ACLMessageBuilder::new()
        .performative(Performative::Propose)
        .sender(participant)
        .receiver(initiator)
        .content(MessageContent::Proposal(proposal_content))
        .conversation(conv_id.clone())
        .build()?;
}
```

## Best Practices

### 1. Always Verify Autonomy
- Check causal independence before accepting impositions
- Ensure voluntary cooperation at every step
- Never coerce other agents

### 2. Intent Before Action
- Declare intentions before any significant action
- Include verification requirements in intent declaration
- Generate cryptographic proofs of execution

### 3. Trust-Based Cooperation
- Build trust through successful promise fulfillment
- Start with low-risk promises to establish trust
- Use trust scores to evaluate impositions

### 4. Observable by Design
- Emit events for all significant state changes
- Use Substrates circuits for event propagation
- Maintain audit logs with cryptographic signatures

### 5. Graceful Degradation
- Handle promise failures gracefully
- Provide alternative strategies when promises break
- Maintain system stability despite individual failures

## Security Considerations

### Context Boundaries
- Enforce strict boundaries on agent operations
- Use security zones (sandbox, development, staging, production)
- Implement permission negotiation for elevated access

### Verification Strategies
- Always verify external claims
- Use multiple verification strategies
- Generate tamper-proof evidence

### Behavioral Profiling
- Monitor agent behavior for anomalies
- Track promise fulfillment rates
- Detect and respond to trust violations

## Future Enhancements

### Phase 1 (Completed)
- ✅ Claude Code hooks configuration
- ✅ Voluntary cooperation protocol
- ✅ FIPA ACL performatives

### Phase 2 (In Progress)
- ⏳ Basic memory architecture
- ⏳ Permission negotiation system
- ⏳ Behavioral profiling

### Phase 3 (Planned)
- Quantum-resistant cryptography
- DAG-based consensus
- Swarm coordination

### Phase 4 (Future)
- Ephemeral neural networks
- Meta-cognitive engine
- Advanced forecasting models

## References

- [Promise Theory](https://markburgess.org/promises.html) by Mark Burgess
- [FIPA ACL Specification](http://www.fipa.org/repository/aclspecs.html)
- [Humainary Substrates](https://substrates.io/)
- [Claude Code Documentation](https://docs.anthropic.com/en/docs/claude-code)

## Support

For questions or issues:
- GitHub Issues: https://github.com/synapsed/synapsed/issues
- Documentation: https://synapsed.io/docs