#!/usr/bin/env node

/**
 * Synapsed Builder SDK - MCP Server for Claude Code
 * 
 * Provides no-code application composition capabilities through MCP tools.
 */

const readline = require('readline');
const path = require('path');
const fs = require('fs').promises;

class BuilderMCPServer {
  constructor() {
    this.components = new Map();
    this.recipes = new Map();
    this.templates = new Map();
    this.initializeComponents();
    this.initializeTemplates();
  }

  initializeComponents() {
    // Component registry
    const components = [
      { name: 'synapsed-core', capabilities: ['runtime', 'memory', 'traits'] },
      { name: 'synapsed-intent', capabilities: ['intent', 'planning', 'verification'] },
      { name: 'synapsed-verify', capabilities: ['verification', 'proof', 'claims'] },
      { name: 'synapsed-consensus', capabilities: ['consensus', 'byzantine', 'ordering'] },
      { name: 'synapsed-net', capabilities: ['p2p', 'networking', 'transport'] },
      { name: 'synapsed-storage', capabilities: ['storage', 'persistence', 'database'] },
      { name: 'synapsed-crdt', capabilities: ['crdt', 'replication', 'conflict-free'] },
      { name: 'synapsed-crypto', capabilities: ['crypto', 'quantum-safe', 'encryption'] },
      { name: 'synapsed-substrates', capabilities: ['observability', 'events', 'monitoring'] },
      { name: 'synapsed-payments', capabilities: ['payments', 'transactions', 'financial'] }
    ];
    
    components.forEach(comp => {
      this.components.set(comp.name, comp);
    });
  }

  initializeTemplates() {
    this.templates.set('verified-ai-agent', {
      name: 'verified-ai-agent',
      description: 'AI agent with intent verification',
      components: ['synapsed-core', 'synapsed-intent', 'synapsed-verify', 'synapsed-substrates'],
      connections: [
        { from: 'synapsed-intent', event: 'intent_declared', to: 'synapsed-verify', handler: 'verify_intent' }
      ]
    });

    this.templates.set('distributed-consensus', {
      name: 'distributed-consensus', 
      description: 'Distributed system with consensus',
      components: ['synapsed-core', 'synapsed-consensus', 'synapsed-net', 'synapsed-crdt'],
      connections: [
        { from: 'synapsed-net', event: 'message_received', to: 'synapsed-consensus', handler: 'process_message' }
      ]
    });

    this.templates.set('observable-microservice', {
      name: 'observable-microservice',
      description: 'Microservice with full observability',
      components: ['synapsed-core', 'synapsed-substrates', 'synapsed-storage'],
      connections: []
    });
  }

  async handleRequest(request) {
    const { jsonrpc, id, method, params } = request;

    try {
      let result;

      switch (method) {
        case 'initialize':
          result = {
            protocolVersion: '2024-11-05',
            capabilities: {
              tools: {}
            },
            serverInfo: {
              name: 'synapsed-builder-sdk',
              version: '1.0.0'
            }
          };
          break;

        case 'initialized':
          result = {};
          break;

        case 'tools/list':
          result = {
            tools: [
              {
                name: 'compose_app',
                description: 'Compose an application from Synapsed components',
                inputSchema: {
                  type: 'object',
                  properties: {
                    name: { type: 'string', description: 'Application name' },
                    description: { type: 'string', description: 'Application description' },
                    components: { 
                      type: 'array', 
                      items: { type: 'string' },
                      description: 'List of component names'
                    },
                    connections: {
                      type: 'array',
                      description: 'Component connections',
                      items: {
                        type: 'object',
                        properties: {
                          from: { type: 'string' },
                          event: { type: 'string' },
                          to: { type: 'string' },
                          handler: { type: 'string' }
                        }
                      }
                    }
                  },
                  required: ['name', 'components']
                }
              },
              {
                name: 'find_components',
                description: 'Find components by capability',
                inputSchema: {
                  type: 'object',
                  properties: {
                    capabilities: {
                      type: 'array',
                      items: { type: 'string' },
                      description: 'Required capabilities'
                    }
                  },
                  required: ['capabilities']
                }
              },
              {
                name: 'list_templates',
                description: 'List available application templates',
                inputSchema: {
                  type: 'object',
                  properties: {}
                }
              },
              {
                name: 'use_template',
                description: 'Build application from template',
                inputSchema: {
                  type: 'object',
                  properties: {
                    template: { type: 'string', description: 'Template name' },
                    name: { type: 'string', description: 'Application name' },
                    config: { type: 'object', description: 'Configuration overrides' }
                  },
                  required: ['template', 'name']
                }
              },
              {
                name: 'validate_composition',
                description: 'Validate application composition',
                inputSchema: {
                  type: 'object',
                  properties: {
                    components: { type: 'array', items: { type: 'string' } },
                    connections: { type: 'array' }
                  },
                  required: ['components']
                }
              },
              {
                name: 'generate_code',
                description: 'Generate application code',
                inputSchema: {
                  type: 'object',
                  properties: {
                    app: { type: 'object', description: 'Application definition' },
                    language: { type: 'string', enum: ['rust', 'typescript'], default: 'rust' }
                  },
                  required: ['app']
                }
              },
              {
                name: 'save_recipe',
                description: 'Save composition as reusable recipe',
                inputSchema: {
                  type: 'object',
                  properties: {
                    name: { type: 'string' },
                    description: { type: 'string' },
                    components: { type: 'array', items: { type: 'string' } },
                    connections: { type: 'array' }
                  },
                  required: ['name', 'components']
                }
              }
            ]
          };
          break;

        case 'tools/call':
          const { name: toolName, arguments: args } = params;
          result = await this.handleToolCall(toolName, args);
          break;

        default:
          throw new Error(`Unknown method: ${method}`);
      }

      return {
        jsonrpc: '2.0',
        id,
        result
      };
    } catch (error) {
      return {
        jsonrpc: '2.0',
        id,
        error: {
          code: -32603,
          message: error.message
        }
      };
    }
  }

  async handleToolCall(toolName, args) {
    switch (toolName) {
      case 'compose_app':
        return this.composeApp(args);
      case 'find_components':
        return this.findComponents(args);
      case 'list_templates':
        return this.listTemplates();
      case 'use_template':
        return this.useTemplate(args);
      case 'validate_composition':
        return this.validateComposition(args);
      case 'generate_code':
        return this.generateCode(args);
      case 'save_recipe':
        return this.saveRecipe(args);
      default:
        throw new Error(`Unknown tool: ${toolName}`);
    }
  }

  async composeApp(args) {
    const { name, description, components, connections = [] } = args;
    
    // Validate components exist
    const missing = components.filter(c => !this.components.has(c));
    if (missing.length > 0) {
      throw new Error(`Unknown components: ${missing.join(', ')}`);
    }

    // Resolve dependencies
    const resolved = this.resolveDependencies(components);

    // Create application definition
    const app = {
      name,
      description: description || `Application composed from ${components.length} components`,
      components: resolved,
      connections,
      config: {},
      env: {}
    };

    return {
      content: [
        {
          type: 'text',
          text: `✅ Application "${name}" composed successfully!\n\n` +
                `Components (${resolved.length}):\n` +
                resolved.map(c => `  • ${c}`).join('\n') + '\n\n' +
                `Connections: ${connections.length}\n\n` +
                `Next steps:\n` +
                `1. Validate with 'validate_composition'\n` +
                `2. Generate code with 'generate_code'\n` +
                `3. Save as recipe with 'save_recipe'`
        }
      ],
      app
    };
  }

  findComponents(args) {
    const { capabilities } = args;
    const matches = [];

    for (const [name, comp] of this.components) {
      const hasAll = capabilities.every(cap => 
        comp.capabilities.includes(cap)
      );
      if (hasAll) {
        matches.push({
          name,
          capabilities: comp.capabilities
        });
      }
    }

    return {
      content: [
        {
          type: 'text',
          text: matches.length > 0 
            ? `Found ${matches.length} components:\n` +
              matches.map(m => `  • ${m.name}: ${m.capabilities.join(', ')}`).join('\n')
            : `No components found with all capabilities: ${capabilities.join(', ')}`
        }
      ],
      components: matches
    };
  }

  listTemplates() {
    const templates = Array.from(this.templates.values());
    
    return {
      content: [
        {
          type: 'text',
          text: `Available templates (${templates.length}):\n\n` +
                templates.map(t => 
                  `📦 ${t.name}\n` +
                  `   ${t.description}\n` +
                  `   Components: ${t.components.join(', ')}`
                ).join('\n\n')
        }
      ],
      templates
    };
  }

  async useTemplate(args) {
    const { template, name, config = {} } = args;
    
    const tmpl = this.templates.get(template);
    if (!tmpl) {
      throw new Error(`Template not found: ${template}`);
    }

    const app = {
      name,
      description: `${tmpl.description} (from template: ${template})`,
      components: [...tmpl.components],
      connections: [...tmpl.connections],
      config,
      env: {}
    };

    return {
      content: [
        {
          type: 'text',
          text: `✅ Created "${name}" from template "${template}"\n\n` +
                `Components: ${app.components.join(', ')}\n` +
                `Connections: ${app.connections.length}`
        }
      ],
      app
    };
  }

  validateComposition(args) {
    const { components, connections = [] } = args;
    const errors = [];
    const warnings = [];

    // Check component dependencies
    components.forEach(comp => {
      if (comp === 'synapsed-consensus' && !components.includes('synapsed-net')) {
        errors.push(`${comp} requires synapsed-net`);
      }
      if (comp === 'synapsed-verify' && !components.includes('synapsed-intent')) {
        warnings.push(`${comp} works best with synapsed-intent`);
      }
    });

    // Check connections
    connections.forEach(conn => {
      if (!components.includes(conn.from)) {
        errors.push(`Connection from unknown component: ${conn.from}`);
      }
      if (!components.includes(conn.to)) {
        errors.push(`Connection to unknown component: ${conn.to}`);
      }
    });

    const valid = errors.length === 0;

    return {
      content: [
        {
          type: 'text',
          text: valid 
            ? `✅ Composition is valid!\n` +
              (warnings.length > 0 ? `\nWarnings:\n${warnings.map(w => `  ⚠️  ${w}`).join('\n')}` : '')
            : `❌ Composition has errors:\n${errors.map(e => `  • ${e}`).join('\n')}` +
              (warnings.length > 0 ? `\n\nWarnings:\n${warnings.map(w => `  ⚠️  ${w}`).join('\n')}` : '')
        }
      ],
      valid,
      errors,
      warnings
    };
  }

  generateCode(args) {
    const { app, language = 'rust' } = args;
    
    if (language === 'rust') {
      const cargoToml = this.generateCargoToml(app);
      const mainRs = this.generateMainRs(app);
      
      return {
        content: [
          {
            type: 'text',
            text: `Generated Rust code for "${app.name}":\n\n` +
                  `📄 Cargo.toml:\n${'```toml'}\n${cargoToml}\n${'```'}\n\n` +
                  `📄 src/main.rs:\n${'```rust'}\n${mainRs}\n${'```'}`
          }
        ],
        files: {
          'Cargo.toml': cargoToml,
          'src/main.rs': mainRs
        }
      };
    } else {
      throw new Error(`Language not supported yet: ${language}`);
    }
  }

  generateCargoToml(app) {
    return `[package]
name = "${app.name}"
version = "0.1.0"
edition = "2021"

[dependencies]
${app.components.map(c => `${c} = { version = "*" }`).join('\n')}
tokio = { version = "1", features = ["full"] }
anyhow = "1.0"
serde_json = "1.0"`;
  }

  generateMainRs(app) {
    return `use anyhow::Result;
${app.components.map(c => `use ${c.replace('-', '_')}::prelude::*;`).join('\n')}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting ${app.name}...");
    
    // Initialize components
${app.components.map(c => `    let ${c.replace('-', '_')} = ${c.replace('-', '_')}::initialize().await?;`).join('\n')}
    
    // Setup connections
${app.connections.map(conn => 
    `    ${conn.from.replace('-', '_')}.connect("${conn.event}", ${conn.to.replace('-', '_')}.handler("${conn.handler}"));`
).join('\n')}
    
    // Start application
    println!("${app.name} is running!");
    tokio::signal::ctrl_c().await?;
    
    Ok(())
}`;
  }

  async saveRecipe(args) {
    const { name, description, components, connections = [] } = args;
    
    const recipe = {
      name,
      description,
      version: '1.0.0',
      components,
      connections,
      config: {},
      created: new Date().toISOString()
    };

    this.recipes.set(name, recipe);

    // Save to file
    const recipePath = path.join(process.env.HOME, '.synapsed', 'builder', 'recipes', `${name}.json`);
    await fs.mkdir(path.dirname(recipePath), { recursive: true });
    await fs.writeFile(recipePath, JSON.stringify(recipe, null, 2));

    return {
      content: [
        {
          type: 'text',
          text: `✅ Recipe "${name}" saved successfully!\n\n` +
                `Location: ${recipePath}\n` +
                `Components: ${components.length}\n` +
                `Connections: ${connections.length}`
        }
      ],
      recipe
    };
  }

  resolveDependencies(components) {
    const resolved = new Set(components);
    
    // Add required dependencies
    if (components.includes('synapsed-consensus')) {
      resolved.add('synapsed-net');
    }
    if (components.includes('synapsed-verify')) {
      resolved.add('synapsed-intent');
    }
    
    // Always include core
    resolved.add('synapsed-core');
    
    return Array.from(resolved);
  }

  async run() {
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
      terminal: false
    });

    process.stderr.write('Builder MCP Server started\n');

    for await (const line of rl) {
      try {
        const request = JSON.parse(line);
        const response = await this.handleRequest(request);
        console.log(JSON.stringify(response));
      } catch (error) {
        console.error(JSON.stringify({
          jsonrpc: '2.0',
          error: {
            code: -32700,
            message: 'Parse error',
            data: error.message
          }
        }));
      }
    }
  }
}

// Run the server
if (require.main === module) {
  const server = new BuilderMCPServer();
  server.run().catch(error => {
    process.stderr.write(`Fatal error: ${error.message}\n`);
    process.exit(1);
  });
}