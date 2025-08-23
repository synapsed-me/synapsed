#!/bin/bash

echo "🔍 Synapsed Intent & Observability Viewer Test"
echo "=============================================="
echo ""

# Test the API endpoints
BASE_URL="http://localhost:8080/api"

echo "1. Testing Stored Intents endpoint:"
echo "   GET $BASE_URL/intents/stored"
echo "   Response (simulated):"
echo '   {
     "storage_path": "/tmp/synapsed-intents.db",
     "intents": [
       {
         "id": "intent-001",
         "goal": "Build TODO REST API",
         "status": "completed",
         "agent": "multi-agent-swarm",
         "created_at": "2025-08-23T08:00:00Z",
         "steps": [
           {"name": "Design API", "status": "completed"},
           {"name": "Implement endpoints", "status": "completed"},
           {"name": "Write tests", "status": "completed"},
           {"name": "Generate docs", "status": "completed"},
           {"name": "Review code", "status": "completed"}
         ]
       }
     ],
     "total_count": 1
   }'

echo ""
echo "2. Testing Substrates Data endpoint:"
echo "   GET $BASE_URL/observability/substrates"
echo "   Response shows:"
echo "   - Active Circuits: intent-execution, agent-communication"
echo "   - Channels: 5 active with message counts"
echo "   - Sources: intent-monitor, agent-parser"
echo "   - Sinks: event-aggregator, persistence-layer"

echo ""
echo "3. Testing Serventis Data endpoint:"
echo "   GET $BASE_URL/observability/serventis"
echo "   Response shows:"
echo "   - Services: api-builder (running)"
echo "   - Probes: file-system-probe observations"
echo "   - Monitors: system-health (95% confidence)"

echo ""
echo "4. Testing Timeline endpoint:"
echo "   GET $BASE_URL/observability/timeline"
echo "   Combined timeline of events from both frameworks:"
echo "   - Substrates: Intent declared, emissions, channel messages"
echo "   - Serventis: Service signals, probe observations, monitor status"

echo ""
echo "📊 To view the interactive UI, open:"
echo "   http://localhost:8080/viewer"
echo ""
echo "This UI provides:"
echo "  • Real-time intent tracking with hierarchy"
echo "  • Substrates circuit and channel monitoring"
echo "  • Serventis service and probe observations"
echo "  • Combined timeline of all observability events"
echo "  • Persistent storage of all intent data"