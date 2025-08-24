# Swarm Monitoring System

This document describes the comprehensive monitoring and metrics system for the synapsed-swarm crate.

## Overview

The monitoring system provides real-time metrics collection, Prometheus integration, alerting capabilities, and health checks for swarm operations. It tracks key performance indicators including agent performance, trust scores, promise fulfillment, and verification success rates.

## Features

### 🔍 Metrics Collection
- **Agent Performance**: Task success rate, execution time, resource utilization
- **Trust Management**: Trust score distributions and changes over time
- **Promise System**: Promise fulfillment rates and violations
- **Verification**: Verification success rates and failure patterns  
- **Swarm Coordination**: Overall swarm efficiency and health

### 📊 Prometheus Integration
- Standard Prometheus metrics format
- Custom swarm-specific metrics
- Histogram data for latency analysis
- Counter metrics for events
- Gauge metrics for current state

### 🚨 Alerting System
- Configurable alert thresholds
- Multiple severity levels (Info, Warning, Critical, Emergency)
- Real-time alert broadcasting
- Alert resolution tracking

### 🏥 Health Monitoring
- Overall swarm health status
- Component-level health checks
- Uptime tracking
- Degradation detection

### 📈 Real-time Dashboard
- Live metrics updates
- Performance trends
- Agent details
- Event history

## Quick Start

### Basic Setup

```rust
use synapsed_swarm::monitoring::{MetricsCollector, MonitoringConfig};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create monitoring configuration
    let config = MonitoringConfig {
        prometheus_port: 9090,
        health_check_port: 8080,
        collection_interval: Duration::from_secs(5),
        max_events: 10000,
        enable_dashboard: true,
        ..Default::default()
    };
    
    // Create metrics collector
    let collector = Arc::new(MetricsCollector::new(config));
    
    // Start monitoring system
    collector.start().await?;
    
    Ok(())
}
```

### Recording Events

```rust
use synapsed_swarm::types::{SwarmEvent, AgentId, AgentRole};
use chrono::Utc;
use uuid::Uuid;

// Record agent joining swarm
let agent_id = Uuid::new_v4();
let event = SwarmEvent::AgentJoined {
    agent_id,
    role: AgentRole::Worker,
    timestamp: Utc::now(),
};
collector.record_event(event).await;

// Record task completion
let task_event = SwarmEvent::TaskCompleted {
    task_id: Uuid::new_v4(),
    agent_id,
    success: true,
    timestamp: Utc::now(),
};
collector.record_event(task_event).await;
```

### Setting up Alerts

```rust
// Subscribe to alerts
let mut alert_receiver = collector.subscribe_alerts();

tokio::spawn(async move {
    while let Ok(alert) = alert_receiver.recv().await {
        match alert.severity {
            AlertSeverity::Critical => {
                eprintln!("CRITICAL ALERT: {}", alert.description);
                // Send notification, page ops team, etc.
            }
            AlertSeverity::Warning => {
                println!("Warning: {}", alert.description);
            }
            _ => {}
        }
    }
});
```

## Configuration

### MonitoringConfig

```rust
pub struct MonitoringConfig {
    /// Prometheus metrics endpoint port
    pub prometheus_port: u16,
    /// Health check endpoint port  
    pub health_check_port: u16,
    /// Metrics collection interval
    pub collection_interval: Duration,
    /// Maximum events to keep in memory
    pub max_events: usize,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
    /// Enable real-time dashboard
    pub enable_dashboard: bool,
}
```

### Alert Thresholds

```rust
pub struct AlertThresholds {
    /// Minimum trust score before alert (default: 0.3)
    pub min_trust_score: f64,
    /// Maximum task failure rate before alert (default: 0.2)
    pub max_failure_rate: f64,
    /// Maximum task execution time before alert in seconds (default: 300)
    pub max_execution_time: u64,
    /// Minimum promise fulfillment rate before alert (default: 0.8)
    pub min_promise_fulfillment_rate: f64,
    /// Maximum verification failure rate before alert (default: 0.1)
    pub max_verification_failure_rate: f64,
    /// Minimum agent availability before alert (default: 0.7)
    pub min_agent_availability: f64,
}
```

## Metrics Reference

### Prometheus Metrics

#### Counters
- `swarm_tasks_total` - Total number of tasks assigned
- `swarm_tasks_success_total` - Total number of successful tasks
- `swarm_tasks_failed_total` - Total number of failed tasks
- `swarm_promises_total` - Total number of promises made
- `swarm_promises_fulfilled_total` - Total number of promises fulfilled
- `swarm_promises_broken_total` - Total number of promises broken
- `swarm_verifications_total` - Total number of verifications performed
- `swarm_verifications_success_total` - Total number of successful verifications

#### Gauges
- `swarm_agents_active` - Number of active agents
- `swarm_trust_score_avg` - Average trust score across all agents
- `swarm_task_success_rate` - Task success rate (0.0 to 1.0)
- `swarm_promise_fulfillment_rate` - Promise fulfillment rate (0.0 to 1.0)
- `swarm_verification_success_rate` - Verification success rate (0.0 to 1.0)

#### Histograms
- `swarm_task_duration_seconds` - Task execution duration distribution
- `swarm_verification_duration_seconds` - Verification duration distribution

#### Per-Agent Metrics
- `agent_trust_score{agent_id}` - Trust score for specific agent
- `agent_tasks_completed{agent_id}` - Tasks completed by agent

### Dashboard API

#### Endpoints

- `GET /health` - Overall swarm health status
- `GET /metrics/dashboard` - Real-time dashboard data
- `GET /metrics` - Prometheus formatted metrics

#### Dashboard Data Format

```json
{
  "timestamp": "2024-08-24T10:30:00Z",
  "swarm_metrics": {
    "total_agents": 5,
    "active_agents": 4,
    "tasks_assigned": 100,
    "tasks_succeeded": 92,
    "tasks_failed": 8,
    "avg_trust_score": 0.85
  },
  "agent_metrics": {
    "agent-uuid-1": {
      "status": "Ready",
      "trust_score": { "value": 0.9, "confidence": 0.85 },
      "tasks_completed": 25,
      "tasks_failed": 2
    }
  },
  "active_alerts": [...],
  "performance_trends": {...}
}
```

## Alert Types

### Trust Score Alerts
- **Low Trust Score**: Triggered when agent trust falls below threshold
- **Rapid Trust Decline**: Alerts on sudden trust drops

### Performance Alerts  
- **High Task Failure Rate**: Agent failing too many tasks
- **Slow Task Execution**: Tasks taking longer than expected
- **Agent Unavailability**: Agent not responding

### Promise System Alerts
- **Promise Violation**: Agent broke a promise
- **Low Fulfillment Rate**: Agent not meeting promise commitments

### System Health Alerts
- **Swarm Degradation**: Overall swarm performance declining
- **Resource Exhaustion**: System resources running low

## Integration Examples

### With Grafana

1. Configure Prometheus to scrape metrics from port 9090
2. Import swarm dashboard template
3. Set up alert rules in Grafana
4. Configure notification channels

### With Custom Monitoring

```rust
use synapsed_swarm::monitoring::DashboardProvider;

let dashboard = DashboardProvider::new(collector);

// Get current metrics
let metrics = dashboard.get_dashboard_data().await;

// Get performance trends for last hour
let trends = dashboard.get_performance_trends(Duration::from_secs(3600)).await;

// Get specific agent details
if let Some(agent_metrics) = dashboard.get_agent_details(agent_id).await {
    println!("Agent trust: {}", agent_metrics.trust_score.value);
}
```

### Custom Alert Handlers

```rust
use synapsed_swarm::monitoring::{Alert, AlertSeverity};

async fn handle_alert(alert: Alert) {
    match alert.severity {
        AlertSeverity::Critical => {
            // Send to incident management system
            send_to_incident_system(&alert).await;
            // Page on-call engineer
            page_engineer(&alert).await;
        }
        AlertSeverity::Warning => {
            // Log to monitoring system
            log_warning(&alert).await;
            // Send Slack notification
            send_slack_notification(&alert).await;
        }
        _ => {
            // Standard logging
            log::info!("Alert: {}", alert.description);
        }
    }
}
```

## Performance Considerations

### Memory Usage
- Events are stored in a circular buffer (configurable size)
- Default limit is 10,000 events
- Trends data is automatically pruned to 1,000 data points

### CPU Impact
- Metrics collection runs on separate tokio tasks
- Configurable collection interval (default: 5 seconds)
- Atomic counters for high-frequency events

### Network Impact
- Prometheus scraping is pull-based
- Dashboard API uses efficient JSON serialization
- Alert broadcasting uses tokio broadcast channels

## Troubleshooting

### Common Issues

1. **Metrics not appearing in Prometheus**
   - Check that port 9090 is accessible
   - Verify Prometheus configuration
   - Check for firewall blocking

2. **Alerts not firing**
   - Verify alert thresholds are set correctly
   - Check that events are being recorded
   - Ensure alert subscription is active

3. **High memory usage**
   - Reduce `max_events` in configuration
   - Increase pruning frequency
   - Check for event recording loops

### Debug Logging

```rust
// Enable debug logging for monitoring
tracing_subscriber::fmt()
    .with_env_filter("synapsed_swarm::monitoring=debug")
    .init();
```

## Best Practices

1. **Configure appropriate thresholds** - Set alert thresholds based on your swarm's normal operating parameters
2. **Monitor the monitors** - Set up alerting on monitoring system health
3. **Use dashboard for troubleshooting** - Real-time dashboard is invaluable for debugging issues
4. **Regular metric review** - Periodically review metrics to identify trends
5. **Test alerting** - Regularly test alert delivery to ensure proper functioning

## Examples

See the `examples/monitoring_demo.rs` file for a complete working example of the monitoring system in action.

## Contributing

When adding new metrics or alerts:

1. Update the metrics collector to record the new data
2. Add corresponding Prometheus metrics
3. Update dashboard data structures
4. Add appropriate alert thresholds
5. Update this documentation
6. Add tests for new functionality