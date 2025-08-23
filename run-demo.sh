#!/bin/bash

# Run script for the Synapsed live demo
# Demonstrates multi-agent system building a REST API with real-time monitoring

set -e

echo "Starting Synapsed Live Demo"
echo "==========================="
echo ""

# Check if built
if [ ! -f "target/release/live-demo" ]; then
    echo "Demo not built. Running build script..."
    ./build-demo.sh
fi

# Start the monitor server in background
echo "Starting monitor server on http://localhost:8080..."
cd examples/live-demo
cargo run --release --bin monitor-server &
MONITOR_PID=$!

# Wait for monitor to start
sleep 2

# Start the main demo
echo ""
echo "Starting multi-agent system..."
echo "Watch the agents build a REST API in real-time!"
echo ""
echo "Monitor Dashboard: http://localhost:8080"
echo "WebSocket Events:  ws://localhost:8080/ws"
echo ""

cargo run --release --bin live-demo

# Cleanup on exit
trap "kill $MONITOR_PID 2>/dev/null" EXIT

echo ""
echo "Demo completed!"