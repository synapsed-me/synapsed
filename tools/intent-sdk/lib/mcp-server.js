#!/usr/bin/env node

/**
 * Synapsed Intent MCP Server - Node.js implementation
 * Provides intent verification tools for Claude Code
 */

const readline = require('readline');
const fs = require('fs').promises;
const path = require('path');
const crypto = require('crypto');
const os = require('os');

// SQLite database setup
const Database = require('better-sqlite3');

class SynapsedMCPServer {
  constructor() {
    this.setupStorage();
    this.setupDatabase();
    this.setupLogging();
  }

  setupStorage() {
    const homeDir = os.homedir();
    this.storagePath = process.env.SYNAPSED_STORAGE_PATH || 
      path.join(homeDir, '.synapsed', 'intents.db');
    this.logPath = process.env.SYNAPSED_SUBSTRATES_LOG || 
      path.join(homeDir, '.synapsed', 'substrates.log');
    
    // Ensure directories exist
    const storageDir = path.dirname(this.storagePath);
    fs.mkdir(storageDir, { recursive: true }).catch(() => {});
  }

  setupDatabase() {
    this.db = new Database(this.storagePath);
    
    // Create tables
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS intents (
        id TEXT PRIMARY KEY,
        goal TEXT NOT NULL,
        description TEXT,
        status TEXT NOT NULL,
        created_at TEXT NOT NULL,
        verified INTEGER DEFAULT 0,
        verification_count INTEGER DEFAULT 0,
        steps TEXT,
        success_criteria TEXT
      )
    `);
    
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS verifications (
        id TEXT PRIMARY KEY,
        intent_id TEXT NOT NULL,
        evidence TEXT NOT NULL,
        timestamp TEXT NOT NULL,
        agent_id TEXT,
        FOREIGN KEY (intent_id) REFERENCES intents(id)
      )
    `);
    
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS agents (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        capabilities TEXT,
        trust_score REAL DEFAULT 0.5,
        created_at TEXT NOT NULL
      )
    `);
  }

  setupLogging() {
    this.log = (level, message, data = {}) => {
      const logEntry = {
        timestamp: new Date().toISOString(),
        level,
        message,
        ...data
      };
      
      // Write to substrates log
      fs.appendFile(this.logPath, JSON.stringify(logEntry) + '\n').catch(() => {});
      
      // Also log to stderr for debugging
      if (process.env.DEBUG) {
        console.error(`[${level}] ${message}`, data);
      }
    };
  }

  async handleRequest(request) {
    const { method, params, id } = request;
    
    switch (method) {
      case 'initialize':
        return this.handleInitialize(id);
      
      case 'tools/list':
        return this.handleToolsList(id);
      
      case 'tools/call':
        return this.handleToolCall(params, id);
      
      default:
        return this.errorResponse(id, -32601, `Method not found: ${method}`);
    }
  }

  handleInitialize(id) {
    return {
      jsonrpc: '2.0',
      id,
      result: {
        protocolVersion: '2024-11-05',
        serverInfo: {
          name: 'synapsed-intent',
          version: '0.1.0'
        },
        capabilities: {
          tools: {
            available: this.getToolDefinitions()
          }
        }
      }
    };
  }

  handleToolsList(id) {
    return {
      jsonrpc: '2.0',
      id,
      result: {
        tools: this.getToolDefinitions().map(tool => ({
          name: tool.name,
          description: tool.description
        }))
      }
    };
  }

  async handleToolCall(params, id) {
    const { name, arguments: args } = params;
    
    try {
      let result;
      
      switch (name) {
        case 'intent_declare':
          result = this.declareIntent(args);
          break;
        
        case 'intent_verify':
          result = this.verifyIntent(args);
          break;
        
        case 'intent_status':
          result = this.getIntentStatus(args);
          break;
        
        case 'intent_list':
          result = this.listIntents(args);
          break;
        
        case 'agent_spawn':
          result = this.spawnAgent(args);
          break;
        
        case 'context_inject':
          result = this.injectContext(args);
          break;
        
        default:
          throw new Error(`Unknown tool: ${name}`);
      }
      
      return {
        jsonrpc: '2.0',
        id,
        result: {
          content: [
            {
              type: 'text',
              text: JSON.stringify(result, null, 2)
            }
          ]
        }
      };
    } catch (error) {
      return this.errorResponse(id, -32603, error.message);
    }
  }

  declareIntent(args) {
    const intentId = crypto.randomUUID();
    const timestamp = new Date().toISOString();
    
    const stmt = this.db.prepare(`
      INSERT INTO intents (id, goal, description, status, created_at, steps, success_criteria)
      VALUES (?, ?, ?, 'declared', ?, ?, ?)
    `);
    
    stmt.run(
      intentId,
      args.goal,
      args.description || '',
      timestamp,
      JSON.stringify(args.steps || []),
      JSON.stringify(args.success_criteria || [])
    );
    
    this.log('info', 'Intent declared', { intentId, goal: args.goal });
    
    // Emit Substrates event
    this.emitEvent('intent.declared', intentId, {
      goal: args.goal,
      steps: (args.steps || []).length,
      criteria: (args.success_criteria || []).length
    });
    
    return {
      intent_id: intentId,
      status: 'declared',
      goal: args.goal,
      steps: (args.steps || []).length,
      timestamp
    };
  }

  verifyIntent(args) {
    const verificationId = crypto.randomUUID();
    const timestamp = new Date().toISOString();
    
    // Store verification
    const stmt = this.db.prepare(`
      INSERT INTO verifications (id, intent_id, evidence, timestamp, agent_id)
      VALUES (?, ?, ?, ?, ?)
    `);
    
    stmt.run(
      verificationId,
      args.intent_id,
      JSON.stringify(args.evidence),
      timestamp,
      args.agent_id || null
    );
    
    // Update intent status
    const updateStmt = this.db.prepare(`
      UPDATE intents 
      SET verified = 1, 
          verification_count = verification_count + 1,
          status = 'verified'
      WHERE id = ?
    `);
    
    updateStmt.run(args.intent_id);
    
    this.log('info', 'Intent verified', { intentId: args.intent_id, verificationId });
    
    // Emit Substrates event
    this.emitEvent('intent.verified', args.intent_id, {
      verification_id: verificationId,
      agent_id: args.agent_id
    });
    
    return {
      verification_id: verificationId,
      intent_id: args.intent_id,
      status: 'verified',
      timestamp
    };
  }

  getIntentStatus(args) {
    const stmt = this.db.prepare(`
      SELECT goal, description, status, created_at, verified, verification_count, steps, success_criteria
      FROM intents WHERE id = ?
    `);
    
    const row = stmt.get(args.intent_id);
    
    if (!row) {
      throw new Error('Intent not found');
    }
    
    return {
      intent_id: args.intent_id,
      goal: row.goal,
      description: row.description,
      status: row.status,
      created_at: row.created_at,
      verified: row.verified === 1,
      verification_count: row.verification_count,
      steps: JSON.parse(row.steps || '[]'),
      success_criteria: JSON.parse(row.success_criteria || '[]')
    };
  }

  listIntents(args) {
    const limit = args?.limit || 10;
    const stmt = this.db.prepare(`
      SELECT id, goal, status, created_at, verified, verification_count
      FROM intents
      ORDER BY created_at DESC
      LIMIT ?
    `);
    
    const rows = stmt.all(limit);
    
    return {
      intents: rows.map(row => ({
        intent_id: row.id,
        goal: row.goal,
        status: row.status,
        created_at: row.created_at,
        verified: row.verified === 1,
        verification_count: row.verification_count
      }))
    };
  }

  spawnAgent(args) {
    const agentId = crypto.randomUUID();
    const timestamp = new Date().toISOString();
    
    const stmt = this.db.prepare(`
      INSERT INTO agents (id, name, capabilities, trust_score, created_at)
      VALUES (?, ?, ?, ?, ?)
    `);
    
    stmt.run(
      agentId,
      args.name,
      JSON.stringify(args.capabilities || []),
      args.trust_score || 0.5,
      timestamp
    );
    
    this.log('info', 'Agent spawned', { agentId, name: args.name });
    
    // Emit Substrates event
    this.emitEvent('agent.spawned', agentId, {
      name: args.name,
      capabilities: args.capabilities
    });
    
    return {
      agent_id: agentId,
      name: args.name,
      capabilities: args.capabilities || [],
      trust_score: args.trust_score || 0.5,
      timestamp
    };
  }

  injectContext(args) {
    // This would pass context to sub-agents
    // For now, we just log it
    this.log('info', 'Context injected', {
      agent_id: args.agent_id,
      context_size: JSON.stringify(args.context).length
    });
    
    return {
      success: true,
      agent_id: args.agent_id,
      context_hash: crypto.createHash('sha256')
        .update(JSON.stringify(args.context))
        .digest('hex')
        .substring(0, 8)
    };
  }

  emitEvent(eventType, subject, data) {
    const event = {
      timestamp: new Date().toISOString(),
      event_type: eventType,
      subject,
      data
    };
    
    fs.appendFile(this.logPath, JSON.stringify(event) + '\n').catch(() => {});
  }

  getToolDefinitions() {
    return [
      {
        name: 'intent_declare',
        description: 'Declare an intent with goals and verification criteria',
        inputSchema: {
          type: 'object',
          properties: {
            goal: {
              type: 'string',
              description: 'The goal of the intent'
            },
            description: {
              type: 'string',
              description: 'Detailed description of the intent'
            },
            steps: {
              type: 'array',
              items: {
                type: 'object',
                properties: {
                  name: { type: 'string' },
                  action: { type: 'string' }
                },
                required: ['name', 'action']
              }
            },
            success_criteria: {
              type: 'array',
              items: { type: 'string' }
            }
          },
          required: ['goal']
        }
      },
      {
        name: 'intent_verify',
        description: 'Verify an intent with evidence',
        inputSchema: {
          type: 'object',
          properties: {
            intent_id: {
              type: 'string',
              description: 'ID of the intent to verify'
            },
            evidence: {
              type: 'object',
              description: 'Evidence of intent completion'
            },
            agent_id: {
              type: 'string',
              description: 'ID of the verifying agent'
            }
          },
          required: ['intent_id', 'evidence']
        }
      },
      {
        name: 'intent_status',
        description: 'Get status of an intent',
        inputSchema: {
          type: 'object',
          properties: {
            intent_id: {
              type: 'string',
              description: 'ID of the intent'
            }
          },
          required: ['intent_id']
        }
      },
      {
        name: 'intent_list',
        description: 'List recent intents',
        inputSchema: {
          type: 'object',
          properties: {
            limit: {
              type: 'number',
              description: 'Maximum number of intents to return'
            }
          }
        }
      },
      {
        name: 'agent_spawn',
        description: 'Spawn a specialized agent for a task',
        inputSchema: {
          type: 'object',
          properties: {
            name: {
              type: 'string',
              description: 'Name of the agent'
            },
            capabilities: {
              type: 'array',
              items: { type: 'string' },
              description: 'List of agent capabilities'
            },
            trust_score: {
              type: 'number',
              description: 'Initial trust score (0-1)'
            }
          },
          required: ['name']
        }
      },
      {
        name: 'context_inject',
        description: 'Inject context into a sub-agent',
        inputSchema: {
          type: 'object',
          properties: {
            agent_id: {
              type: 'string',
              description: 'ID of the agent'
            },
            context: {
              type: 'object',
              description: 'Context to inject'
            }
          },
          required: ['agent_id', 'context']
        }
      }
    ];
  }

  errorResponse(id, code, message) {
    return {
      jsonrpc: '2.0',
      id,
      error: {
        code,
        message
      }
    };
  }

  async start() {
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
      terminal: false
    });

    this.log('info', 'Synapsed MCP Server started', {
      storagePath: this.storagePath,
      logPath: this.logPath
    });

    rl.on('line', async (line) => {
      if (!line.trim()) return;
      
      try {
        const request = JSON.parse(line);
        const response = await this.handleRequest(request);
        console.log(JSON.stringify(response));
      } catch (error) {
        console.log(JSON.stringify(this.errorResponse(null, -32700, 'Parse error')));
      }
    });

    rl.on('close', () => {
      this.log('info', 'MCP Server shutting down');
      this.db.close();
      process.exit(0);
    });
  }
}

// Start server if run directly
if (require.main === module) {
  const server = new SynapsedMCPServer();
  server.start();
}

module.exports = SynapsedMCPServer;