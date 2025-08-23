#!/bin/bash
# Test script to verify MCP flow

echo "🚀 Testing MCP Agent Spawning Flow"
echo "=================================="

# Clean up any previous test data
rm -f /tmp/synapsed-intents.json

# Start the MCP server in the background
echo "1. Starting MCP server..."
SYNAPSED_INTENT_STORAGE_PATH=/tmp/synapsed-intents.json \
cargo run --bin synapsed-mcp 2>&1 | tee mcp-server.log &
MCP_PID=$!

sleep 3

# Test declaring an intent via JSON-RPC
echo ""
echo "2. Declaring test intent via MCP..."
echo '{"jsonrpc":"2.0","method":"intent/declare","params":{"agent_id":"test-agent","description":"Test intent","metadata":null},"id":"test-1"}' | \
nc localhost 8000 || echo "(Note: Direct RPC test skipped - MCP uses stdio)"

# Run a simple agent-runner test
echo ""
echo "3. Testing agent-runner directly..."
cd examples/live-demo
cargo run --bin agent-runner -- \
  --agent-type architect \
  --workspace /tmp/test-workspace \
  --intent-id test-intent-123 &
AGENT_PID=$!

# Wait for agent to complete
sleep 5

# Check if intent storage file was created
echo ""
echo "4. Checking intent storage..."
if [ -f /tmp/synapsed-intents.json ]; then
    echo "✅ Intent storage file exists"
    echo "Contents:"
    cat /tmp/synapsed-intents.json | jq '.' 2>/dev/null || cat /tmp/synapsed-intents.json
else
    echo "⚠️ No intent storage file found"
fi

# Clean up
echo ""
echo "5. Cleaning up..."
kill $MCP_PID 2>/dev/null
kill $AGENT_PID 2>/dev/null

echo ""
echo "✅ Test complete!"