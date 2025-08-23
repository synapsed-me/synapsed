#!/bin/bash
# Run the complete live demo with monitoring

echo "🚀 Starting Synapsed Live Demo with Full Monitoring"
echo "===================================================="

# Clean up any previous data
echo "🧹 Cleaning up previous data..."
rm -f /tmp/synapsed-intents.json
rm -f /tmp/synapsed-intents.db
rm -rf /tmp/todo-api

# Start the monitor server in the background
echo ""
echo "📊 Starting Monitor Server..."
cd /workspaces/synapsed/examples/live-demo
cargo run --bin monitor-server > monitor.log 2>&1 &
MONITOR_PID=$!
echo "Monitor PID: $MONITOR_PID"

# Give monitor time to start
sleep 3

# Run the main live demo
echo ""
echo "🎯 Starting Live Demo (with MCP server)..."
echo "This will:"
echo "  1. Start MCP server"
echo "  2. Declare intents through MCP"
echo "  3. Spawn agents to build TODO API"
echo "  4. Store intents persistently"
echo ""

# Set the storage path so monitor can find it
export SYNAPSED_INTENT_STORAGE_PATH=/tmp/synapsed-intents.json

# Run the demo
cargo run --bin live-demo 2>&1 | tee live-demo.log &
DEMO_PID=$!

echo ""
echo "=========================================="
echo "🌐 Services are starting up..."
echo ""
echo "📊 Monitor Dashboard: http://localhost:8080"
echo "📁 Monitor API: http://localhost:8080/api"
echo "👁️ View Stored Intents: http://localhost:8080/api/intents/stored"
echo "🔍 Intent Viewer: http://localhost:8080/viewer"
echo ""
echo "⏳ Waiting for demo to run..."
echo "=========================================="

# Function to check stored intents
check_intents() {
    echo ""
    echo "📋 Checking stored intents via API..."
    curl -s http://localhost:8080/api/intents/stored | jq '.' 2>/dev/null || echo "Waiting for API..."
}

# Wait a bit for things to start
sleep 5

# Check intents periodically
for i in {1..10}; do
    check_intents
    sleep 10
done

echo ""
echo "=========================================="
echo "Press Ctrl+C to stop all services"
echo "=========================================="

# Wait for user to stop
trap "echo '🛑 Stopping services...'; kill $MONITOR_PID $DEMO_PID 2>/dev/null; exit" INT
wait