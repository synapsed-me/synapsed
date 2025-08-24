# Synapsed Monitor

Human-centric monitoring system with real-time visualization and narrative insights.

## Overview

`synapsed-monitor` provides comprehensive monitoring capabilities for the Synapsed ecosystem, focusing on human-readable insights rather than raw metrics. It combines real-time data aggregation with narrative generation to help operators understand system behavior at a glance.

## Features

### Core Capabilities
- **Real-time Monitoring**: WebSocket-based live updates
- **Narrative Generation**: Automatic conversion of events to human-readable stories  
- **Multi-View System**: Agent, health, and task-specific views
- **Pattern Detection**: Automatic anomaly and pattern recognition
- **REST API**: Comprehensive API for data access
- **Web Dashboard**: Interactive HTML5 visualization

### Monitoring Components
- **Event Aggregator**: Collects and correlates events from all sources
- **Metric Collector**: Gathers performance and health metrics
- **Narrative Engine**: Converts technical events into readable insights
- **Pattern Detector**: Identifies trends and anomalies
- **WebSocket Server**: Real-time event streaming
- **REST API Server**: HTTP endpoints for queries and control

## Implementation Status

### Core Features
- ✅ Event aggregation from Substrates
- ✅ Metric collection system
- ✅ WebSocket server for real-time updates
- ✅ REST API with multiple endpoints
- ✅ Basic web viewer (HTML)
- 🚧 Advanced narrative generation
- 🚧 Pattern detection algorithms
- 📋 Machine learning anomaly detection

### Views and Visualizations
- ✅ Agent activity view
- ✅ System health dashboard
- ✅ Task progress tracking
- 🚧 Intent verification view
- 🚧 Network topology visualization
- 📋 Custom dashboard builder

### Integration
- ✅ Substrates event integration
- ✅ Serventis service monitoring
- 🚧 External metrics (Prometheus)
- 📋 Cloud provider integration

## Architecture

```
┌─────────────────────────────────────┐
│         Event Sources               │
│  (Substrates, Serventis, Agents)    │
└────────────┬────────────────────────┘
             │
      ┌──────▼──────┐
      │  Aggregator │
      └──────┬──────┘
             │
    ┌────────┴────────┐
    │                 │
┌───▼───┐       ┌─────▼─────┐
│Metrics│       │  Narrator  │
└───┬───┘       └─────┬─────┘
    │                 │
    └────────┬────────┘
             │
      ┌──────▼──────┐
      │   Storage   │
      └──────┬──────┘
             │
    ┌────────┴────────┐
    │                 │
┌───▼───┐       ┌─────▼─────┐
│  API  │       │ WebSocket │
└───────┘       └───────────┘
    │                 │
    └────────┬────────┘
             │
      ┌──────▼──────┐
      │  Dashboard  │
      └─────────────┘
```

## Usage

### Starting the Monitor Server
```bash
# Run the monitor server
cargo run --bin synapsed-monitor

# With custom configuration
MONITOR_PORT=9090 cargo run --bin synapsed-monitor
```

### Programmatic Usage
```rust
use synapsed_monitor::{Monitor, CollectorConfig};

// Create monitor instance
let config = CollectorConfig::default()
    .with_aggregation_interval(Duration::seconds(5))
    .with_retention_period(Duration::hours(24));

let monitor = Monitor::new(config);

// Start monitoring
monitor.start().await?;

// Subscribe to events
let mut events = monitor.subscribe_events();
while let Some(event) = events.next().await {
    println!("Event: {:?}", event);
}
```

### REST API Endpoints

- `GET /api/health` - System health status
- `GET /api/agents` - Active agent information
- `GET /api/tasks` - Task execution status
- `GET /api/metrics` - Performance metrics
- `GET /api/events?from=<timestamp>` - Event history
- `GET /api/narrative` - Human-readable system narrative
- `WebSocket /ws` - Real-time event stream

### Web Dashboard

Access the dashboard at `http://localhost:8080` (default port).

Features:
- Real-time event feed
- Agent activity tracking
- System health indicators
- Task progress visualization
- Intent verification status

## Configuration

```toml
[dependencies.synapsed-monitor]
version = "0.1.0"
features = ["websocket", "api", "narrator"]
```

### Environment Variables

- `MONITOR_PORT` - HTTP server port (default: 8080)
- `MONITOR_WS_PORT` - WebSocket port (default: 8081)
- `MONITOR_RETENTION_HOURS` - Event retention period (default: 24)
- `MONITOR_AGGREGATION_INTERVAL_MS` - Aggregation interval (default: 5000)

## Examples

### Subscribe to Specific Events
```rust
use synapsed_monitor::{EventFilter, EventType};

let filter = EventFilter::new()
    .with_types(vec![EventType::AgentSpawned, EventType::TaskCompleted])
    .with_severity_min(Severity::Info);

let events = monitor.subscribe_filtered(filter);
```

### Generate Narrative
```rust
use synapsed_monitor::narrator::NarrativeGenerator;

let narrator = NarrativeGenerator::new();
let narrative = narrator.generate_narrative(
    events,
    Duration::minutes(5)
).await?;

println!("System Story: {}", narrative);
```

## Testing

```bash
# Run all tests
cargo test -p synapsed-monitor

# Run with example data
cargo run --example monitor-demo
```

## Performance Considerations

1. **Event Buffer Size**: Default 10,000 events in memory
2. **Aggregation Window**: 5 seconds for metric rollups
3. **WebSocket Connections**: Max 100 concurrent connections
4. **Storage Retention**: 24 hours of event history by default

## License

Licensed under Apache 2.0 or MIT at your option.