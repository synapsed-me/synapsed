#!/usr/bin/env node

/**
 * Synapsed Init - Initialize a project with intent verification
 * Sets up Claude Code integration and generates CLAUDE.md
 */

const fs = require('fs').promises;
const path = require('path');
const os = require('os');
// Simple argument parser for compatibility
const program = {
  opts: () => {
    const args = process.argv.slice(2);
    return {
      skipClaude: args.includes('--skip-claude'),
      force: args.includes('--force'),
      help: args.includes('--help') || args.includes('-h')
    };
  },
  parse: () => {}
};
// Simple color functions for compatibility
const chalk = {
  cyan: (str) => `\x1b[36m${str}\x1b[0m`,
  yellow: (str) => `\x1b[33m${str}\x1b[0m`,
  green: (str) => `\x1b[32m${str}\x1b[0m`,
  red: (str) => `\x1b[31m${str}\x1b[0m`,
  gray: (str) => `\x1b[90m${str}\x1b[0m`,
  white: (str) => `\x1b[37m${str}\x1b[0m`
};
const { execSync } = require('child_process');

async function detectClaudeCode() {
  const possiblePaths = [
    path.join(os.homedir(), 'Library', 'Application Support', 'Claude', 'claude_desktop_config.json'),
    path.join(os.homedir(), '.claude', 'claude_desktop_config.json'),
    path.join(os.homedir(), '.config', 'claude', 'claude_desktop_config.json')
  ];
  
  for (const configPath of possiblePaths) {
    try {
      await fs.access(configPath);
      return configPath;
    } catch {}
  }
  
  return null;
}

async function setupClaudeCode(configPath) {
  try {
    const configContent = await fs.readFile(configPath, 'utf8');
    const config = JSON.parse(configContent);
    
    if (!config.mcpServers) {
      config.mcpServers = {};
    }
    
    // Add synapsed-intent server configuration
    config.mcpServers['synapsed-intent'] = {
      type: 'stdio',
      command: 'npx',
      args: ['-y', '@synapsed/intent-sdk'],
      env: {
        SYNAPSED_STORAGE_PATH: path.join(os.homedir(), '.synapsed', 'intents.db'),
        SYNAPSED_SUBSTRATES_LOG: path.join(os.homedir(), '.synapsed', 'substrates.log'),
        DEBUG: 'false'
      }
    };
    
    await fs.writeFile(configPath, JSON.stringify(config, null, 2));
    return true;
  } catch (error) {
    console.error(chalk.red('Failed to update Claude Code configuration:'), error.message);
    return false;
  }
}

async function generateClaudeMd(projectPath) {
  const template = `# CLAUDE.md

This file provides guidance to Claude Code when working with this project using the Synapsed Intent Verification System.

## Project Overview

This project uses the Synapsed Intent SDK to ensure verifiable AI agent execution. All agent actions must be declared as intents before execution and verified after completion.

## Available MCP Tools

The following tools are available through the synapsed-intent MCP server:

### intent_declare
Declare an intent before executing any significant action:
\`\`\`javascript
{
  "goal": "Refactor authentication module",
  "description": "Improve code organization and add tests",
  "steps": [
    {"name": "Analyze current structure", "action": "Read and understand existing code"},
    {"name": "Create new structure", "action": "Reorganize into modular components"},
    {"name": "Add tests", "action": "Write comprehensive unit tests"}
  ],
  "success_criteria": [
    "All tests pass",
    "Code coverage > 80%",
    "No breaking changes"
  ]
}
\`\`\`

### intent_verify
Verify that an intent was successfully completed:
\`\`\`javascript
{
  "intent_id": "uuid-from-declare",
  "evidence": {
    "tests_passed": true,
    "coverage": 85,
    "files_modified": ["auth.js", "auth.test.js"]
  }
}
\`\`\`

### intent_status
Check the status of a declared intent:
\`\`\`javascript
{
  "intent_id": "uuid-from-declare"
}
\`\`\`

### agent_spawn
Spawn specialized agents for complex tasks:
\`\`\`javascript
{
  "name": "Code Analyzer",
  "capabilities": ["static-analysis", "complexity-metrics", "dependency-mapping"]
}
\`\`\`

### context_inject
Pass context to sub-agents to prevent context escape:
\`\`\`javascript
{
  "agent_id": "agent-uuid",
  "context": {
    "parent_intent": "refactor-auth",
    "constraints": ["preserve-api", "maintain-backwards-compatibility"],
    "allowed_paths": ["src/auth/**"]
  }
}
\`\`\`

## Best Practices

1. **Always Declare Intents**: Before making any significant changes, declare your intent with clear goals and success criteria.

2. **Verify Execution**: After completing tasks, always verify with evidence that the intent was successfully executed.

3. **Use Specialized Agents**: For complex tasks, spawn specialized agents with specific capabilities.

4. **Inject Context**: When using sub-agents, always inject context to prevent them from escaping their intended scope.

5. **Check Status**: Regularly check intent status to ensure all declared intents are being tracked.

## Project-Specific Guidelines

### Directory Structure
- Source code: \`src/\`
- Tests: \`tests/\` or \`__tests__/\`
- Documentation: \`docs/\`

### Testing Requirements
- All new code must have tests
- Maintain minimum 80% code coverage
- Run tests before declaring intent as verified

### Code Style
- Follow existing patterns in the codebase
- Use consistent naming conventions
- Document complex logic with comments

## Verification Checklist

Before marking any intent as verified, ensure:
- [ ] All declared steps were completed
- [ ] Success criteria are met
- [ ] Tests pass (if applicable)
- [ ] Code compiles without errors
- [ ] No unintended side effects
- [ ] Documentation updated (if needed)

## Storage Locations

- Intent Database: \`~/.synapsed/intents.db\`
- Event Logs: \`~/.synapsed/substrates.log\`
- Project Config: \`.synapsed/config.json\`

## Troubleshooting

If the MCP server is not responding:
1. Check that the server is running: \`ps aux | grep synapsed-mcp\`
2. Review logs: \`cat ~/.synapsed/substrates.log\`
3. Restart Claude Code
4. Re-run: \`npx @synapsed/intent-sdk init\`

---
Generated by Synapsed Intent SDK v0.1.0
`;

  const claudeMdPath = path.join(projectPath, 'CLAUDE.md');
  await fs.writeFile(claudeMdPath, template);
  return claudeMdPath;
}

async function initializeStorage() {
  const synapsedDir = path.join(os.homedir(), '.synapsed');
  
  try {
    await fs.mkdir(synapsedDir, { recursive: true });
    
    // Create initial config
    const configPath = path.join(synapsedDir, 'config.json');
    const config = {
      version: '0.1.0',
      initialized: new Date().toISOString(),
      projects: []
    };
    
    try {
      const existing = await fs.readFile(configPath, 'utf8');
      const existingConfig = JSON.parse(existing);
      config.projects = existingConfig.projects || [];
    } catch {}
    
    // Add current project
    const currentProject = {
      path: process.cwd(),
      name: path.basename(process.cwd()),
      initialized: new Date().toISOString()
    };
    
    if (!config.projects.find(p => p.path === currentProject.path)) {
      config.projects.push(currentProject);
    }
    
    await fs.writeFile(configPath, JSON.stringify(config, null, 2));
    
    return synapsedDir;
  } catch (error) {
    console.error(chalk.red('Failed to initialize storage:'), error.message);
    throw error;
  }
}

async function main() {
  // Parse arguments
  program.parse();

  const options = program.opts();
  
  if (options.help) {
    console.log('Usage: synapsed-init [options]\n');
    console.log('Initialize Synapsed Intent verification for your project\n');
    console.log('Options:');
    console.log('  --skip-claude  Skip Claude Code configuration');
    console.log('  --force        Overwrite existing configuration');
    console.log('  -h, --help     display help for command');
    process.exit(0);
  }
  
  console.log(chalk.cyan('\n🚀 Synapsed Intent SDK Initialization\n'));
  
  try {
    // Step 1: Initialize storage
    console.log(chalk.yellow('📁 Initializing storage...'));
    const storageDir = await initializeStorage();
    console.log(chalk.green('✓ Storage initialized at:'), storageDir);
    
    // Step 2: Generate CLAUDE.md
    console.log(chalk.yellow('📝 Generating CLAUDE.md...'));
    const claudeMdPath = await generateClaudeMd(process.cwd());
    console.log(chalk.green('✓ CLAUDE.md created at:'), claudeMdPath);
    
    // Step 3: Configure Claude Code
    if (!options.skipClaude) {
      console.log(chalk.yellow('🔧 Configuring Claude Code...'));
      const claudeConfigPath = await detectClaudeCode();
      
      if (claudeConfigPath) {
        const success = await setupClaudeCode(claudeConfigPath);
        if (success) {
          console.log(chalk.green('✓ Claude Code configured successfully'));
        } else {
          console.log(chalk.yellow('⚠ Could not update Claude Code config automatically'));
          console.log(chalk.cyan('\nManual configuration:'));
          console.log('Add the following to your Claude Code settings:');
          console.log(chalk.gray('---'));
          const template = await fs.readFile(
            path.join(__dirname, '..', 'templates', 'claude-config.json'),
            'utf8'
          );
          console.log(template);
          console.log(chalk.gray('---'));
        }
      } else {
        console.log(chalk.yellow('⚠ Claude Code configuration not found'));
        console.log(chalk.cyan('To configure manually, run:'));
        console.log(chalk.white('  claude mcp add synapsed-intent -- npx -y @synapsed/intent-sdk'));
      }
    }
    
    // Step 4: Test the installation
    console.log(chalk.yellow('\n🧪 Testing installation...'));
    try {
      // Check if we can require the MCP server
      require('../lib/mcp-server');
      console.log(chalk.green('✓ MCP server module loaded successfully'));
    } catch (error) {
      console.log(chalk.red('✗ Failed to load MCP server:'), error.message);
    }
    
    // Success message
    console.log(chalk.green('\n✨ Initialization complete!\n'));
    console.log(chalk.cyan('Next steps:'));
    console.log('1. Restart Claude Code to load the MCP server');
    console.log('2. Use intent_declare before making changes');
    console.log('3. Use intent_verify after completing tasks');
    console.log('4. Run', chalk.white('npx @synapsed/intent-sdk monitor'), 'to view the dashboard\n');
    
  } catch (error) {
    console.error(chalk.red('\n❌ Initialization failed:'), error.message);
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}