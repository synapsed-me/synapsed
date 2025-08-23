#!/bin/bash

# Run Synapsed demo with monitoring

echo "🚀 Starting Synapsed Demo with Monitoring"
echo "=========================================="
echo ""

# Build everything first
echo "📦 Building components..."
cargo build -p synapsed-monitor --bin monitor-server 2>/dev/null
cargo build -p working-demo 2>/dev/null

# Start monitor server in background
echo "🖥️  Starting monitor server on http://localhost:8080..."
cargo run -p synapsed-monitor --bin monitor-server 2>/dev/null &
MONITOR_PID=$!

# Give monitor time to start
sleep 2

echo ""
echo "📊 Monitor endpoints available:"
echo "   • Health: http://localhost:8080/health"
echo "   • Tasks: http://localhost:8080/api/tasks"
echo "   • Agents: http://localhost:8080/api/agents"
echo "   • Events: http://localhost:8080/api/events"
echo "   • WebSocket: ws://localhost:8080/ws"
echo ""

# Run the demo
echo "🎯 Running working demo..."
echo "============================"
echo ""
cd examples/working-demo && cargo run

# Cleanup
echo ""
echo "🛑 Stopping monitor server..."
kill $MONITOR_PID 2>/dev/null

echo "✅ Demo complete!"