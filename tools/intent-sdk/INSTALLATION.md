# Synapsed Intent SDK Installation Guide

## 🚀 Quick Installation (via NPX)

For end users who want to use the SDK in their projects:

```bash
# One-command installation
npx @synapsed/intent-sdk init

# This will:
# 1. Set up Claude Code MCP integration
# 2. Generate CLAUDE.md in your project
# 3. Initialize the intent database
# 4. Configure everything automatically
```

## 📦 Publishing to NPM (for maintainers)

To publish this SDK so others can use it via `npx`:

```bash
# From the synapsed-intent-sdk directory
cd /workspaces/synapsed/synapsed-intent-sdk

# Login to npm (if not already)
npm login

# Publish the package
npm publish --access public
```

## 🧪 Local Testing (without publishing)

### Test the SDK locally:

```bash
# From any project directory
node /workspaces/synapsed/synapsed-intent-sdk/bin/synapsed-init.js --skip-claude

# Start the monitor
node /workspaces/synapsed/synapsed-intent-sdk/bin/synapsed-monitor.js

# Run the MCP server manually
node /workspaces/synapsed/synapsed-intent-sdk/bin/synapsed-mcp.js
```

### Configure Claude Code manually:

1. **Option A: Using Claude CLI**
   ```bash
   claude mcp add synapsed-intent -- node /workspaces/synapsed/synapsed-intent-sdk/bin/synapsed-mcp.js
   ```

2. **Option B: Edit config file directly**
   
   Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:
   ```json
   {
     "mcpServers": {
       "synapsed-intent": {
         "type": "stdio",
         "command": "node",
         "args": ["/workspaces/synapsed/synapsed-intent-sdk/bin/synapsed-mcp.js"],
         "env": {
           "SYNAPSED_STORAGE_PATH": "${HOME}/.synapsed/intents.db"
         }
       }
     }
   }
   ```

## 🎯 Using with Claude Code

Once configured, Claude Code will have access to these tools:

### Declare an intent before making changes:
```
Use the intent_declare tool to declare my intention to refactor the authentication module with the goal of improving code organization and adding comprehensive tests.
```

### Verify completion:
```
Use the intent_verify tool to confirm that the refactoring is complete with evidence that all tests pass and coverage is above 80%.
```

### Spawn specialized agents:
```
Use the agent_spawn tool to create a Code Analyzer agent with capabilities for static analysis and dependency mapping.
```

## 📊 Monitoring

View the real-time dashboard:
```bash
# If installed via npx
npx @synapsed/intent-sdk monitor

# If testing locally
node /workspaces/synapsed/synapsed-intent-sdk/bin/synapsed-monitor.js
```

Then open http://localhost:8080 in your browser.

## 🔍 Verification

Check that everything is working:

```bash
# View stored intents
sqlite3 ~/.synapsed/intents.db "SELECT * FROM intents;"

# View event logs
cat ~/.synapsed/substrates.log | tail -20

# Check if MCP server is running
ps aux | grep synapsed-mcp
```

## 📝 Generated Files

After initialization, you'll have:

- **`CLAUDE.md`** - Project-specific guidance for Claude Code
- **`~/.synapsed/intents.db`** - SQLite database with intents
- **`~/.synapsed/substrates.log`** - Event log file
- **`~/.synapsed/config.json`** - SDK configuration

## 🛠️ Troubleshooting

### Issue: "better-sqlite3 not found"
```bash
# Install in SDK directory
cd /workspaces/synapsed/synapsed-intent-sdk
npm install better-sqlite3
```

### Issue: "Claude Code doesn't see the tools"
1. Restart Claude Code after configuration
2. Check the config file was updated correctly
3. Try the manual configuration method

### Issue: "Permission denied"
```bash
# Make scripts executable
chmod +x /workspaces/synapsed/synapsed-intent-sdk/bin/*.js
```

## 🏗️ Architecture

The SDK provides:
- **MCP Server**: Node.js implementation of the Model Context Protocol
- **Intent Storage**: SQLite database for persistent storage
- **Event Logging**: Substrates-compatible event stream
- **Web Dashboard**: Real-time monitoring interface
- **Claude Integration**: Automatic MCP server configuration

## 🚢 Deployment Checklist

Before publishing to npm:

- [ ] Update version in `package.json`
- [ ] Test all commands locally
- [ ] Verify Claude Code integration works
- [ ] Update README with any new features
- [ ] Tag release in git
- [ ] Run `npm publish --access public`

## 📜 License

MIT © Synapsed Team