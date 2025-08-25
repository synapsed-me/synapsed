# Synapsed Intent SDK

🧠 **Intent Verification System for Claude Code** - Ensure verifiable AI agent execution with hierarchical intent declaration and multi-agent verification.

## Quick Start

Install and initialize the SDK in your project:

```bash
npx @synapsed/intent-sdk init
```

This will:
1. ✅ Set up Claude Code MCP integration
2. 📝 Generate a `CLAUDE.md` file with project-specific guidance
3. 💾 Initialize the intent database
4. 🚀 Configure the MCP server for immediate use

## Features

- **Intent Declaration**: AI agents declare intentions before taking actions
- **Multi-Agent Verification**: Spawn specialized agents to verify execution
- **Context Injection**: Prevent context escape in sub-agents
- **Real-time Monitoring**: Web dashboard for viewing intents and verifications
- **Substrates Integration**: Event-driven observability with detailed logging
- **Zero Configuration**: Works immediately with `npx`, no compilation needed

## How It Works

The SDK provides an MCP (Model Context Protocol) server that integrates with Claude Code, allowing Claude to:

1. **Declare intents** before making changes
2. **Spawn specialized agents** for complex tasks
3. **Verify execution** with evidence
4. **Track all actions** in a persistent database

## Available Commands

### Initialize a Project
```bash
npx @synapsed/intent-sdk init
```

### Start the Monitoring Dashboard
```bash
npx @synapsed/intent-sdk monitor
```
Then open http://localhost:8080 in your browser.

### Run the MCP Server Manually
```bash
npx @synapsed/intent-sdk
```
(Usually started automatically by Claude Code)

## MCP Tools Available in Claude Code

Once installed, Claude Code has access to these tools:

### `intent_declare`
Declare an intent before executing actions:
```json
{
  "goal": "Refactor authentication module",
  "description": "Improve code organization and add tests",
  "steps": [
    {"name": "Analyze", "action": "Review existing code"},
    {"name": "Refactor", "action": "Reorganize into modules"},
    {"name": "Test", "action": "Add unit tests"}
  ],
  "success_criteria": ["All tests pass", "Coverage > 80%"]
}
```

### `intent_verify`
Verify that an intent was completed:
```json
{
  "intent_id": "uuid-from-declare",
  "evidence": {
    "tests_passed": true,
    "coverage": 85,
    "files_modified": ["auth.js", "auth.test.js"]
  }
}
```

### `agent_spawn`
Create specialized agents for tasks:
```json
{
  "name": "Code Analyzer",
  "capabilities": ["static-analysis", "dependency-mapping"]
}
```

### `context_inject`
Pass context to prevent agent escape:
```json
{
  "agent_id": "agent-uuid",
  "context": {
    "constraints": ["preserve-api"],
    "allowed_paths": ["src/auth/**"]
  }
}
```

## Project Structure

After initialization, your project will have:

```
your-project/
├── CLAUDE.md           # AI guidance file (generated)
└── ~/.synapsed/        # Global SDK data
    ├── intents.db      # Intent storage
    ├── substrates.log  # Event logs
    └── config.json     # SDK configuration
```

## Manual Claude Code Configuration

If automatic configuration doesn't work, add this to your Claude Code settings:

```bash
claude mcp add synapsed-intent -- npx -y @synapsed/intent-sdk
```

Or edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "synapsed-intent": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@synapsed/intent-sdk"],
      "env": {
        "SYNAPSED_STORAGE_PATH": "${HOME}/.synapsed/intents.db",
        "DEBUG": "false"
      }
    }
  }
}
```

## Environment Variables

- `SYNAPSED_STORAGE_PATH`: Database location (default: `~/.synapsed/intents.db`)
- `SYNAPSED_SUBSTRATES_LOG`: Event log location (default: `~/.synapsed/substrates.log`)
- `DEBUG`: Enable debug logging (default: `false`)

## Development

### Local Development
```bash
# Clone the repository
git clone https://github.com/synapsed-me/synapsed
cd synapsed/synapsed-intent-sdk

# Install dependencies
npm install

# Test locally
node bin/synapsed-init.js
```

### Testing with Claude Code
1. Run `npm link` in the SDK directory
2. In your test project: `npm link @synapsed/intent-sdk`
3. Initialize: `npx synapsed-init`

## Architecture

The SDK consists of:

1. **MCP Server** (`lib/mcp-server.js`): JSON-RPC server for Claude Code
2. **Init Script** (`bin/synapsed-init.js`): Project initialization
3. **Monitor** (`bin/synapsed-monitor.js`): Web dashboard
4. **Database**: SQLite for intent and verification storage
5. **Event Log**: Substrates-compatible event stream

## Troubleshooting

### MCP Server Not Responding
1. Check if running: `ps aux | grep synapsed-mcp`
2. View logs: `cat ~/.synapsed/substrates.log`
3. Restart Claude Code
4. Re-initialize: `npx @synapsed/intent-sdk init`

### Database Issues
```bash
# Check database
sqlite3 ~/.synapsed/intents.db "SELECT * FROM intents;"

# Reset database
rm ~/.synapsed/intents.db
npx @synapsed/intent-sdk init
```

## Contributing

We welcome contributions! Please see the main [Synapsed repository](https://github.com/synapsed-me/synapsed) for guidelines.

## License

MIT © Synapsed Team

---

Built with 🧠 by the Synapsed team to ensure verifiable AI agent execution.