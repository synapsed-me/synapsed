#!/bin/bash

# Run script for the Synapsed live demo
# Demonstrates multi-agent system building a REST API with real-time monitoring

set -e

echo "🚀 Starting Synapsed Live Demo"
echo "================================"
echo ""
echo "This demo shows:"
echo "  • Multi-agent cooperation using Promise Theory"
echo "  • Intent declaration and verification"
echo "  • Real-time observability with Substrates"
echo "  • Safety mechanisms and self-healing"
echo ""

# Clean up any previous data
echo "🧹 Cleaning up previous data..."
rm -f /tmp/synapsed-intents.json
rm -f /tmp/synapsed-intents.db
rm -rf /tmp/todo-api

# Check if built
if [ ! -f "examples/live-demo/target/release/live-demo" ]; then
    echo "📦 Building demo (this may take a minute)..."
    cd examples/live-demo
    cargo build --release --bin live-demo --bin monitor-server
    cd ../..
fi

# Start the monitor server in background
echo ""
echo "📊 Starting monitor server on http://localhost:8080..."
cd examples/live-demo
cargo run --release --bin monitor-server > monitor.log 2>&1 &
MONITOR_PID=$!
cd ../..

# Wait for monitor to start
sleep 2

# Set the storage path for persistent intents
export SYNAPSED_INTENT_STORAGE_PATH=/tmp/synapsed-intents.json

# Start the main demo
echo ""
echo "🎯 Starting multi-agent system..."
echo ""
echo "Services available:"
echo "  📊 Monitor Dashboard: http://localhost:8080"
echo "  🔍 Intent Viewer:     http://localhost:8080/viewer"
echo "  📡 WebSocket Events:  ws://localhost:8080/ws"
echo "  📁 Stored Intents:    http://localhost:8080/api/intents/stored"
echo ""
echo "Press Ctrl+C to stop all services"
echo "================================"
echo ""

# Cleanup on exit
trap "echo ''; echo '🛑 Stopping services...'; kill $MONITOR_PID 2>/dev/null; echo '✅ Demo stopped'; exit" INT TERM

# Run the demo
cd examples/live-demo
cargo run --release --bin live-demo

echo ""
echo "✨ Demo completed successfully!"