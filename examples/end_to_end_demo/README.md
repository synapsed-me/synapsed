# Synapsed End-to-End Demo

Comprehensive demonstration of the Synapsed Swarm system showcasing real-world multi-agent coordination with verification.

## Features Demonstrated

- 🤝 **Multi-agent coordination** with Promise Theory
- ⚡ **Real command execution** with sandboxing
- ✅ **Verification** of agent claims
- 📈 **Trust evolution** over time
- 💾 **Persistent storage** for trust scores
- 📊 **Monitoring & metrics** with Prometheus integration
- 🔐 **Cryptographic proofs** of execution

## Prerequisites

```bash
# Build the entire workspace first
cd /workspaces/synapsed
cargo build --all

# Install the demo
cd examples/end_to_end_demo
cargo build
```

## Usage

The demo provides several commands to showcase different aspects:

### 1. Simple Multi-Agent Demo

Demonstrates basic swarm coordination with real command execution:

```bash
cargo run -- simple --agents 3
```

This will:
- Create a swarm with 3 agents
- Execute real shell commands (pwd, ls, date, echo)
- Verify execution results
- Show trust scores and metrics

### 2. Complex Development Scenario

Simulates a software development team working together:

```bash
cargo run -- complex --project my_api
```

This demonstrates:
- Specialized agents (architect, developer, tester, reviewer)
- Hierarchical task delegation
- Promise-based cooperation
- Complex workflow coordination

### 3. Trust Evolution Demo

Shows how trust scores evolve based on agent performance:

```bash
cargo run -- trust --iterations 20
```

Features:
- Agents with different reliability levels
- Trust score changes based on success/failure
- Persistent storage of trust scores
- Visual representation of trust evolution

### 4. Monitoring Dashboard

Runs a live monitoring dashboard:

```bash
cargo run -- monitor --port 9090
```

Provides:
- Prometheus metrics endpoint: `http://localhost:9090/metrics`
- Health check API: `http://localhost:9091/health`
- Dashboard data: `http://localhost:9091/dashboard`

## Example Output

### Simple Demo
```
🚀 Simple Multi-Agent Demo
Creating swarm with 3 agents...

✅ Swarm initialized with SQLite storage

  ➕ Agent agent_0 joined (ID: 123e4567-e89b-12d3-a456-426614174000)
  ➕ Agent agent_1 joined (ID: 234e5678-e89b-12d3-a456-426614174001)
  ➕ Agent agent_2 joined (ID: 345e6789-e89b-12d3-a456-426614174002)

📋 Creating intent chain...
🤝 Delegating intent to swarm...
  Task ID: 456e789a-e89b-12d3-a456-426614174003

⏳ Monitoring execution...
.....
✅ Task completed successfully!
  Duration: 2.34s

📊 Output:
{
  "steps": [
    {
      "name": "Check current directory",
      "output": "/workspaces/synapsed",
      "success": true
    },
    {
      "name": "List files",
      "output": "Cargo.toml\nCargo.lock\nREADME.md\n...",
      "success": true
    }
  ]
}

🔐 Execution verified with cryptographic proof

📊 Swarm Metrics:
  Total agents: 3
  Tasks succeeded: 1
  Tasks failed: 0
  Promises made: 3
  Promises fulfilled: 3
  Average trust score: 0.52
  Verification success rate: 100.00%
```

### Trust Evolution
```
📈 Trust Evolution Demo
Running 20 iterations...

Agents created:
  🟢 Reliable Agent (90% success rate)
  🔴 Unreliable Agent (30% success rate)
  🟡 Improving Agent (starts at 50%, improves over time)

Iteration 1/20:
  ✓ Reliable    - Trust: 0.51
  ✗ Unreliable  - Trust: 0.48
  ✓ Improving   - Trust: 0.51

...

📊 Final Trust Scores:
  Reliable    [████████████████░░░░] 0.82
  Unreliable  [██████░░░░░░░░░░░░░░] 0.31
  Improving   [██████████████░░░░░░] 0.71
```

## Architecture

The demo showcases the complete Synapsed architecture:

```
┌─────────────────────────────────────┐
│         Swarm Coordinator           │
├─────────────────────────────────────┤
│  Intent  │  Promise  │ Verification │
│  System  │  Theory   │  Framework   │
├─────────────────────────────────────┤
│      Execution Engine (Real)        │
├─────────────────────────────────────┤
│      Persistent Storage (SQLite)    │
├─────────────────────────────────────┤
│      Monitoring & Metrics           │
├─────────────────────────────────────┤
│   Agent₁    Agent₂    Agent₃        │
└─────────────────────────────────────┘
```

## Key Technologies

- **Promise Theory**: Agents make voluntary promises
- **Intent System**: Hierarchical task planning
- **Verification**: All claims verified against reality
- **Trust Management**: Reputation-based coordination
- **Command Execution**: Sandboxed shell commands
- **Persistent Storage**: SQLite for trust scores
- **Monitoring**: Prometheus-compatible metrics

## Security Features

- Command allowlisting/blocklisting
- Execution timeouts
- Resource limits
- Sandboxing support
- Path traversal protection
- Environment variable filtering

## Configuration

The demo can be configured through environment variables:

```bash
# Set log level
RUST_LOG=debug cargo run -- simple

# Use different storage backend
STORAGE_PATH=/tmp/swarm.db cargo run -- simple

# Configure execution timeout
EXEC_TIMEOUT=10 cargo run -- simple
```

## Integration with Grafana

To visualize metrics in Grafana:

1. Start the monitoring demo:
   ```bash
   cargo run -- monitor
   ```

2. Add Prometheus data source in Grafana:
   - URL: `http://localhost:9090`

3. Import the dashboard (see `dashboards/swarm.json`)

## Troubleshooting

### Permission Denied
If commands fail with permission errors, ensure the demo has execute permissions:
```bash
chmod +x target/debug/e2e_demo
```

### Port Already in Use
If the monitoring port is in use, specify a different port:
```bash
cargo run -- monitor --port 9999
```

### Database Locked
If you see database lock errors, ensure no other instance is running:
```bash
pkill e2e_demo
rm *.db-wal *.db-shm
```

## Next Steps

After running the demo, you can:

1. **Modify agent behaviors** in `src/main.rs`
2. **Add new command types** to the execution engine
3. **Implement custom verification strategies**
4. **Create your own swarm scenarios**
5. **Integrate with your applications**

## License

MIT OR Apache-2.0