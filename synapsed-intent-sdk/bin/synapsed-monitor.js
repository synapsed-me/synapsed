#!/usr/bin/env node

/**
 * Synapsed Monitor - Web-based dashboard for intent verification
 */

const http = require('http');
const fs = require('fs').promises;
const path = require('path');
const os = require('os');
const { program } = require('commander');
// Simple color functions for compatibility
const chalk = {
  cyan: (str) => `\x1b[36m${str}\x1b[0m`,
  green: (str) => `\x1b[32m${str}\x1b[0m`,
  gray: (str) => `\x1b[90m${str}\x1b[0m`
};
const Database = require('better-sqlite3');

class MonitorServer {
  constructor(port = 8080) {
    this.port = port;
    this.setupDatabase();
  }

  setupDatabase() {
    const storagePath = process.env.SYNAPSED_STORAGE_PATH || 
      path.join(os.homedir(), '.synapsed', 'intents.db');
    
    this.db = new Database(storagePath, { readonly: true });
  }

  async handleRequest(req, res) {
    const url = new URL(req.url, `http://localhost:${this.port}`);
    
    // Set CORS headers
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
    
    if (req.method === 'OPTIONS') {
      res.writeHead(200);
      res.end();
      return;
    }
    
    switch (url.pathname) {
      case '/':
        await this.serveDashboard(res);
        break;
      
      case '/api/intents':
        this.serveIntents(res);
        break;
      
      case '/api/agents':
        this.serveAgents(res);
        break;
      
      case '/api/verifications':
        this.serveVerifications(res);
        break;
      
      case '/api/events':
        await this.serveEvents(res);
        break;
      
      default:
        res.writeHead(404);
        res.end('Not Found');
    }
  }

  async serveDashboard(res) {
    const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Synapsed Intent Monitor</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      color: #333;
      min-height: 100vh;
      padding: 20px;
    }
    .container {
      max-width: 1400px;
      margin: 0 auto;
    }
    header {
      background: white;
      border-radius: 12px;
      padding: 30px;
      margin-bottom: 30px;
      box-shadow: 0 10px 30px rgba(0,0,0,0.1);
    }
    h1 {
      color: #764ba2;
      margin-bottom: 10px;
      display: flex;
      align-items: center;
      gap: 10px;
    }
    .subtitle {
      color: #666;
      font-size: 14px;
    }
    .stats {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
      gap: 20px;
      margin-bottom: 30px;
    }
    .stat-card {
      background: white;
      border-radius: 12px;
      padding: 20px;
      box-shadow: 0 5px 15px rgba(0,0,0,0.08);
      text-align: center;
    }
    .stat-value {
      font-size: 36px;
      font-weight: bold;
      color: #667eea;
      margin-bottom: 5px;
    }
    .stat-label {
      color: #666;
      font-size: 14px;
      text-transform: uppercase;
      letter-spacing: 1px;
    }
    .section {
      background: white;
      border-radius: 12px;
      padding: 25px;
      margin-bottom: 25px;
      box-shadow: 0 5px 15px rgba(0,0,0,0.08);
    }
    .section-title {
      font-size: 20px;
      color: #333;
      margin-bottom: 20px;
      padding-bottom: 10px;
      border-bottom: 2px solid #f0f0f0;
    }
    table {
      width: 100%;
      border-collapse: collapse;
    }
    th {
      text-align: left;
      padding: 12px;
      background: #f8f9fa;
      color: #666;
      font-weight: 600;
      font-size: 13px;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }
    td {
      padding: 12px;
      border-top: 1px solid #f0f0f0;
      font-size: 14px;
    }
    tr:hover {
      background: #f8f9fa;
    }
    .status {
      display: inline-block;
      padding: 4px 12px;
      border-radius: 20px;
      font-size: 12px;
      font-weight: 600;
      text-transform: uppercase;
    }
    .status-declared {
      background: #fff3cd;
      color: #856404;
    }
    .status-verified {
      background: #d4edda;
      color: #155724;
    }
    .status-failed {
      background: #f8d7da;
      color: #721c24;
    }
    .agent-capabilities {
      display: flex;
      gap: 5px;
      flex-wrap: wrap;
    }
    .capability {
      background: #e9ecef;
      padding: 3px 8px;
      border-radius: 12px;
      font-size: 12px;
      color: #495057;
    }
    .trust-bar {
      width: 100px;
      height: 8px;
      background: #e9ecef;
      border-radius: 4px;
      overflow: hidden;
      display: inline-block;
      vertical-align: middle;
    }
    .trust-fill {
      height: 100%;
      background: linear-gradient(90deg, #667eea, #764ba2);
      transition: width 0.3s ease;
    }
    .refresh-btn {
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      color: white;
      border: none;
      padding: 10px 20px;
      border-radius: 8px;
      cursor: pointer;
      font-size: 14px;
      font-weight: 600;
      transition: transform 0.2s;
    }
    .refresh-btn:hover {
      transform: translateY(-2px);
    }
    .empty-state {
      text-align: center;
      padding: 40px;
      color: #999;
    }
    .timeline {
      position: relative;
      padding-left: 30px;
    }
    .timeline-item {
      position: relative;
      padding-bottom: 20px;
      border-left: 2px solid #e9ecef;
      padding-left: 20px;
    }
    .timeline-item:before {
      content: '';
      position: absolute;
      left: -7px;
      top: 0;
      width: 12px;
      height: 12px;
      border-radius: 50%;
      background: #667eea;
      border: 2px solid white;
    }
    .timeline-time {
      font-size: 12px;
      color: #999;
      margin-bottom: 5px;
    }
    .timeline-content {
      background: #f8f9fa;
      padding: 10px;
      border-radius: 8px;
      font-size: 14px;
    }
  </style>
</head>
<body>
  <div class="container">
    <header>
      <h1>
        <span>🧠</span>
        Synapsed Intent Monitor
      </h1>
      <div class="subtitle">Real-time monitoring of AI agent intent verification</div>
    </header>
    
    <div class="stats" id="stats">
      <div class="stat-card">
        <div class="stat-value" id="total-intents">0</div>
        <div class="stat-label">Total Intents</div>
      </div>
      <div class="stat-card">
        <div class="stat-value" id="verified-intents">0</div>
        <div class="stat-label">Verified</div>
      </div>
      <div class="stat-card">
        <div class="stat-value" id="active-agents">0</div>
        <div class="stat-label">Active Agents</div>
      </div>
      <div class="stat-card">
        <div class="stat-value" id="success-rate">0%</div>
        <div class="stat-label">Success Rate</div>
      </div>
    </div>
    
    <div class="section">
      <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
        <h2 class="section-title">Recent Intents</h2>
        <button class="refresh-btn" onclick="refresh()">↻ Refresh</button>
      </div>
      <div id="intents-table"></div>
    </div>
    
    <div class="section">
      <h2 class="section-title">Active Agents</h2>
      <div id="agents-table"></div>
    </div>
    
    <div class="section">
      <h2 class="section-title">Event Timeline</h2>
      <div id="timeline" class="timeline"></div>
    </div>
  </div>
  
  <script>
    async function fetchData() {
      try {
        const [intents, agents, verifications, events] = await Promise.all([
          fetch('/api/intents').then(r => r.json()),
          fetch('/api/agents').then(r => r.json()),
          fetch('/api/verifications').then(r => r.json()),
          fetch('/api/events').then(r => r.json())
        ]);
        
        updateStats(intents, agents);
        updateIntentsTable(intents);
        updateAgentsTable(agents);
        updateTimeline(events);
      } catch (error) {
        console.error('Failed to fetch data:', error);
      }
    }
    
    function updateStats(intents, agents) {
      const total = intents.length;
      const verified = intents.filter(i => i.verified).length;
      const successRate = total > 0 ? Math.round((verified / total) * 100) : 0;
      
      document.getElementById('total-intents').textContent = total;
      document.getElementById('verified-intents').textContent = verified;
      document.getElementById('active-agents').textContent = agents.length;
      document.getElementById('success-rate').textContent = successRate + '%';
    }
    
    function updateIntentsTable(intents) {
      const container = document.getElementById('intents-table');
      
      if (intents.length === 0) {
        container.innerHTML = '<div class="empty-state">No intents declared yet</div>';
        return;
      }
      
      const html = \`
        <table>
          <thead>
            <tr>
              <th>Goal</th>
              <th>Status</th>
              <th>Verifications</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            \${intents.map(intent => \`
              <tr>
                <td>\${intent.goal}</td>
                <td><span class="status status-\${intent.status}">\${intent.status}</span></td>
                <td>\${intent.verification_count}</td>
                <td>\${new Date(intent.created_at).toLocaleString()}</td>
              </tr>
            \`).join('')}
          </tbody>
        </table>
      \`;
      
      container.innerHTML = html;
    }
    
    function updateAgentsTable(agents) {
      const container = document.getElementById('agents-table');
      
      if (agents.length === 0) {
        container.innerHTML = '<div class="empty-state">No agents spawned yet</div>';
        return;
      }
      
      const html = \`
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Capabilities</th>
              <th>Trust Score</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            \${agents.map(agent => {
              const capabilities = JSON.parse(agent.capabilities || '[]');
              const trustPercent = Math.round(agent.trust_score * 100);
              return \`
                <tr>
                  <td>\${agent.name}</td>
                  <td>
                    <div class="agent-capabilities">
                      \${capabilities.map(cap => \`<span class="capability">\${cap}</span>\`).join('')}
                    </div>
                  </td>
                  <td>
                    <div class="trust-bar">
                      <div class="trust-fill" style="width: \${trustPercent}%"></div>
                    </div>
                    \${agent.trust_score.toFixed(2)}
                  </td>
                  <td>\${new Date(agent.created_at).toLocaleString()}</td>
                </tr>
              \`;
            }).join('')}
          </tbody>
        </table>
      \`;
      
      container.innerHTML = html;
    }
    
    function updateTimeline(events) {
      const container = document.getElementById('timeline');
      
      if (events.length === 0) {
        container.innerHTML = '<div class="empty-state">No events recorded yet</div>';
        return;
      }
      
      const html = events.slice(0, 10).map(event => \`
        <div class="timeline-item">
          <div class="timeline-time">\${new Date(event.timestamp).toLocaleTimeString()}</div>
          <div class="timeline-content">
            <strong>\${event.event_type}</strong>: \${event.subject.substring(0, 8)}...
            \${event.data ? \`<br><small>\${JSON.stringify(event.data).substring(0, 100)}...</small>\` : ''}
          </div>
        </div>
      \`).join('');
      
      container.innerHTML = html;
    }
    
    function refresh() {
      fetchData();
    }
    
    // Initial load
    fetchData();
    
    // Auto-refresh every 5 seconds
    setInterval(fetchData, 5000);
  </script>
</body>
</html>`;
    
    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end(html);
  }

  serveIntents(res) {
    try {
      const stmt = this.db.prepare(`
        SELECT id, goal, description, status, created_at, verified, verification_count
        FROM intents
        ORDER BY created_at DESC
        LIMIT 50
      `);
      
      const intents = stmt.all();
      
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(intents));
    } catch (error) {
      res.writeHead(500, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: error.message }));
    }
  }

  serveAgents(res) {
    try {
      const stmt = this.db.prepare(`
        SELECT id, name, capabilities, trust_score, created_at
        FROM agents
        ORDER BY created_at DESC
        LIMIT 50
      `);
      
      const agents = stmt.all();
      
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(agents));
    } catch (error) {
      // Table might not exist yet
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify([]));
    }
  }

  serveVerifications(res) {
    try {
      const stmt = this.db.prepare(`
        SELECT id, intent_id, evidence, timestamp
        FROM verifications
        ORDER BY timestamp DESC
        LIMIT 50
      `);
      
      const verifications = stmt.all();
      
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(verifications));
    } catch (error) {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify([]));
    }
  }

  async serveEvents(res) {
    try {
      const logPath = process.env.SYNAPSED_SUBSTRATES_LOG || 
        path.join(os.homedir(), '.synapsed', 'substrates.log');
      
      const content = await fs.readFile(logPath, 'utf8');
      const lines = content.split('\n').filter(line => line.trim());
      const events = lines.slice(-50).reverse().map(line => {
        try {
          return JSON.parse(line);
        } catch {
          return null;
        }
      }).filter(Boolean);
      
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(events));
    } catch (error) {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify([]));
    }
  }

  start() {
    const server = http.createServer((req, res) => this.handleRequest(req, res));
    
    server.listen(this.port, () => {
      console.log(chalk.cyan('\\n🖥️  Synapsed Intent Monitor\\n'));
      console.log(chalk.green(\`✓ Dashboard running at: http://localhost:\${this.port}\`));
      console.log(chalk.gray('\\nPress Ctrl+C to stop\\n'));
    });
  }
}

program
  .name('synapsed-monitor')
  .description('Web-based monitoring dashboard for Synapsed Intent verification')
  .option('-p, --port <port>', 'port to run the server on', '8080')
  .parse();

const options = program.opts();
const monitor = new MonitorServer(parseInt(options.port));
monitor.start();