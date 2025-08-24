#!/usr/bin/env node

/**
 * Synapsed MCP Server wrapper for Claude Code integration
 * Uses Node.js implementation for cross-platform compatibility
 */

const MCPServer = require('../lib/mcp-server');

// Start the server
const server = new MCPServer();
server.start();