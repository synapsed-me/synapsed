# Claude Swarm Coordination Guide

This guide explains how to use the Synapsed Swarm system for coordinating multiple AI agents with verification.

## Overview

The swarm system enables you to:
- Coordinate multiple agents to work on complex tasks
- Make and verify promises about agent behavior
- Build trust through proven performance
- Verify all agent claims against reality
- Prevent context escape through boundaries

## Available MCP Tools

### 🎯 `swarm_coordinate`
Coordinate multiple agents to achieve a goal.

**Parameters:**
- `goal` (string, required): What you want the swarm to achieve
- `agent_count` (number, optional): Number of agents to coordinate (default: 3)
- `min_trust_score` (number, optional): Minimum trust for agents (0.0-1.0, default: 0.3)
- `require_verification` (boolean, optional): Verify all executions (default: true)
- `context` (object, optional): Context variables to pass

**Example:**
```json
{
  "goal": "Build a REST API with tests",
  "agent_count": 3,
  "require_verification": true,
  "context": {
    "language": "rust",
    "framework": "actix-web"
  }
}
```

### 🤝 `promise_make`
Make a voluntary promise about what you will do.

**Parameters:**
- `agent_id` (string, required): Your agent ID
- `promise_body` (string, required): What you promise to do
- `scope` (array, optional): Who the promise applies to
- `promise_type` (string, optional): "offer", "use", or "delegate"
- `conditions` (array, optional): Conditions for the promise

**Example:**
```json
{
  "agent_id": "claude_123",
  "promise_body": "I will write comprehensive tests for all endpoints",
  "promise_type": "offer",
  "conditions": ["access to test framework", "API spec available"]
}
```

### ✅ `promise_verify`
Verify whether a promise was fulfilled.

**Parameters:**
- `promise_id` (string, required): ID of the promise
- `agent_id` (string, required): Agent who made the promise
- `evidence` (object, required): Evidence of fulfillment
- `fulfilled` (boolean, required): Whether promise was fulfilled

**Example:**
```json
{
  "promise_id": "promise_456",
  "agent_id": "claude_123",
  "fulfilled": true,
  "evidence": {
    "tests_written": 15,
    "coverage": "95%",
    "all_passing": true
  }
}
```

### 📋 `intent_delegate`
Delegate an intent to a sub-agent with verification.

**Parameters:**
- `intent_id` (string, required): Intent to delegate
- `target` (string, optional): Target agent or "swarm"
- `require_verification` (boolean, optional): Require verification (default: true)
- `min_trust` (number, optional): Minimum trust required
- `context` (object, optional): Context to inject

**Example:**
```json
{
  "intent_id": "intent_789",
  "target": "code_reviewer",
  "require_verification": true,
  "min_trust": 0.7,
  "context": {
    "review_type": "security",
    "severity": "high"
  }
}
```

### 👤 `agent_register`
Register yourself or another agent with the swarm.

**Parameters:**
- `name` (string, required): Agent name
- `capabilities` (array, required): What the agent can do
- `tools` (array, optional): Available tools
- `initial_trust` (number, optional): Starting trust score
- `role` (string, optional): "worker", "verifier", "coordinator", "observer"

**Example:**
```json
{
  "name": "claude_coder",
  "capabilities": ["code_generation", "refactoring", "testing"],
  "tools": ["read_file", "write_file", "execute_command"],
  "role": "worker",
  "initial_trust": 0.6
}
```

### 🔍 `trust_query`
Query the current trust score of an agent.

**Parameters:**
- `agent_id` (string, required): Agent to query

**Example:**
```json
{
  "agent_id": "claude_123"
}
```

### 📈 `trust_update`
Update trust score based on performance.

**Parameters:**
- `agent_id` (string, required): Agent to update
- `success` (boolean, required): Task succeeded or failed
- `verified` (boolean, optional): Was the task verified
- `reason` (string, optional): Reason for update

**Example:**
```json
{
  "agent_id": "claude_123",
  "success": true,
  "verified": true,
  "reason": "Successfully completed code review with verification"
}
```

## Usage Patterns

### Pattern 1: Simple Task Delegation
```python
# 1. Coordinate swarm for a goal
swarm_coordinate({
  "goal": "Implement user authentication",
  "agent_count": 2
})

# 2. System automatically:
#    - Finds suitable agents
#    - Negotiates promises
#    - Delegates tasks
#    - Verifies execution
#    - Updates trust scores
```

### Pattern 2: Promise-Based Workflow
```python
# 1. Register as an agent
agent_register({
  "name": "claude_tester",
  "capabilities": ["testing", "test_generation"]
})

# 2. Make a promise
promise_make({
  "agent_id": "claude_tester_id",
  "promise_body": "Write unit tests with 90% coverage"
})

# 3. Execute the work
# ... perform testing ...

# 4. Verify the promise
promise_verify({
  "promise_id": "promise_id",
  "agent_id": "claude_tester_id",
  "fulfilled": true,
  "evidence": {"coverage": "92%", "tests": 25}
})
```

### Pattern 3: Verified Delegation
```python
# 1. Create an intent
intent_declare({
  "goal": "Refactor authentication module",
  "steps": [...],
  "success_criteria": ["All tests pass", "No security issues"]
})

# 2. Delegate with verification
intent_delegate({
  "intent_id": "intent_id",
  "require_verification": true,
  "min_trust": 0.6
})

# 3. System verifies:
#    - Commands actually executed
#    - Files actually changed
#    - Tests actually pass
#    - Security scan clean
```

## Trust Model

Agents build trust through successful task completion:

| Action | Trust Impact | Condition |
|--------|-------------|-----------|
| Task Success (Verified) | +0.05 | Task completed and verified |
| Task Success (Unverified) | +0.02 | Task completed, not verified |
| Task Failure | -0.10 | Task failed |
| Promise Fulfilled | +0.05 | Promise kept |
| Promise Broken | -0.10 | Promise broken |

Trust scores determine:
- **0.0-0.3**: Cannot perform tasks
- **0.3-0.5**: Basic tasks only
- **0.5-0.7**: Standard tasks
- **0.7-0.9**: Critical tasks
- **0.9-1.0**: Full autonomy

## Context Boundaries

When delegating to sub-agents, the system enforces boundaries:

```json
{
  "filesystem_access": "workspace",  // none, read_only, workspace, full
  "allowed_endpoints": ["api.github.com"],
  "max_memory_mb": 512,
  "max_execution_secs": 300,
  "can_delegate": false,
  "can_call_external": false
}
```

## Verification

All agent claims are verified against reality:

1. **Command Verification**: Commands actually executed with expected output
2. **File Verification**: Files actually created/modified
3. **Network Verification**: API calls actually made
4. **State Verification**: System state actually changed

## Best Practices

1. **Always make promises** before taking actions
2. **Start with low trust** and build up through success
3. **Require verification** for critical tasks
4. **Use context injection** to prevent sub-agent escape
5. **Monitor trust scores** to identify reliable agents
6. **Delegate to specialists** based on capabilities
7. **Verify all claims** against external reality

## Example: Multi-Agent Code Review

```python
# 1. Coordinate a code review swarm
result = swarm_coordinate({
  "goal": "Review and improve authentication module",
  "agent_count": 3,
  "context": {
    "module": "src/auth",
    "focus": ["security", "performance", "tests"]
  }
})

# 2. Agents automatically:
#    - Analyzer: Reviews code structure
#    - Security: Checks for vulnerabilities  
#    - Tester: Writes missing tests

# 3. Each agent:
#    - Makes promises about what they'll do
#    - Executes with verification
#    - Updates trust based on success

# 4. Get results
if result.success:
    print(f"Review complete by {result.agent_count} agents")
    print(f"Issues found: {result.issues}")
    print(f"Improvements made: {result.improvements}")
```

## Troubleshooting

### Low Trust Scores
- Agents start with default trust (0.5)
- Build trust through successful verified tasks
- Failed tasks reduce trust significantly

### Verification Failures
- Ensure agents have necessary permissions
- Check that expected outcomes are realistic
- Verify context boundaries aren't too restrictive

### Promise Conflicts
- Agents won't make promises they can't keep
- Check agent capabilities match requirements
- Ensure willingness conditions are met

## Security Considerations

1. **Never trust without verification** - All claims must be proven
2. **Context injection is mandatory** - Prevents sub-agent escape
3. **Trust boundaries are enforced** - Limits damage from rogue agents
4. **Cryptographic proofs** - All verifications generate proofs
5. **Audit trail** - Complete history of all agent actions

## Advanced Features

### Consensus Verification
Multiple agents verify critical operations:
```python
config.require_consensus = true
config.consensus_verifiers = 3
```

### Adaptive Trust
Trust scores decay over time and adapt to patterns:
```python
trust_manager.apply_time_decay(0.01)  # 1% daily decay
trust_manager.apply_peer_feedback(agent_id, feedback, peer_trust)
```

### Promise Chemistry
Promises can interact:
- **Compose**: Combine smaller promises
- **Catalyze**: Enable other promises
- **Inhibit**: Prevent conflicting promises

## Summary

The swarm system provides reliable multi-agent coordination through:
- **Voluntary cooperation** (Promise Theory)
- **Hierarchical planning** (Intent System)
- **Reality verification** (Verification Framework)
- **Reputation tracking** (Trust Model)

This ensures AI agents work together effectively while preventing context escape and false claims.