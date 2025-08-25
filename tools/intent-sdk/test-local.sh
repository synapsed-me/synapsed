#!/bin/bash

# Test script for local SDK testing without npm install

echo "🧪 Testing Synapsed Intent SDK locally..."
echo ""

# Set up paths
SDK_DIR="$(cd "$(dirname "$0")" && pwd)"
export NODE_PATH="$SDK_DIR/node_modules:$NODE_PATH"

# Test 1: Check MCP server can start
echo "1️⃣ Testing MCP server..."
timeout 2s node "$SDK_DIR/bin/synapsed-mcp.js" < /dev/null
if [ $? -eq 124 ]; then
  echo "✅ MCP server starts correctly (timed out as expected)"
else
  echo "❌ MCP server failed to start"
fi

# Test 2: Check init script
echo ""
echo "2️⃣ Testing init script..."
node "$SDK_DIR/bin/synapsed-init.js" --help
if [ $? -eq 0 ]; then
  echo "✅ Init script works"
else
  echo "❌ Init script failed"
fi

# Test 3: Create test project
echo ""
echo "3️⃣ Creating test project..."
TEST_DIR="/tmp/synapsed-test-$$"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

# Mock better-sqlite3 if not installed
if ! node -e "require('better-sqlite3')" 2>/dev/null; then
  echo "⚠️  better-sqlite3 not installed, using mock..."
  mkdir -p node_modules
  cat > node_modules/better-sqlite3.js << 'EOF'
// Mock better-sqlite3 for testing
class Database {
  constructor() {
    this.data = {};
  }
  prepare(sql) {
    return {
      run: () => ({ lastInsertRowid: 1, changes: 1 }),
      get: () => null,
      all: () => []
    };
  }
  exec() {}
  close() {}
}
module.exports = Database;
EOF
fi

# Run init
node "$SDK_DIR/bin/synapsed-init.js" --skip-claude
if [ $? -eq 0 ]; then
  echo "✅ Project initialization succeeded"
else
  echo "❌ Project initialization failed"
fi

# Check if CLAUDE.md was created
if [ -f "CLAUDE.md" ]; then
  echo "✅ CLAUDE.md created"
else
  echo "❌ CLAUDE.md not created"
fi

# Clean up
rm -rf "$TEST_DIR"

echo ""
echo "📊 Test Summary:"
echo "SDK is ready for local testing!"
echo ""
echo "To test with Claude Code:"
echo "1. Run: node $SDK_DIR/bin/synapsed-init.js"
echo "2. Restart Claude Code"
echo "3. Use intent_declare in Claude Code"
echo ""
echo "To view monitor:"
echo "node $SDK_DIR/bin/synapsed-monitor.js"