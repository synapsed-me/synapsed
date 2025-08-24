//! Demonstration of the swarm monitoring system
//!
//! This example shows how to:
//! - Set up metrics collection
//! - Start Prometheus exporter
//! - Generate sample metrics
//! - Trigger alerts
//! - View dashboard data

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;
use chrono::Utc;

use synapsed_swarm::{
    monitoring::{MetricsCollector, MonitoringConfig, PrometheusExporter, DashboardProvider, AlertSeverity},
    types::{SwarmEvent, AgentId, AgentRole, AgentStatus, TaskResult},
    trust::TrustScore,
    SwarmResult,
};

#[tokio::main]
async fn main() -> SwarmResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🔍 Starting Swarm Monitoring Demo");
    
    // Create monitoring configuration
    let config = MonitoringConfig {
        prometheus_port: 9090,
        health_check_port: 8080,
        collection_interval: Duration::from_secs(2),
        max_events: 1000,
        enable_dashboard: true,
        ..Default::default()
    };
    
    // Create metrics collector
    let collector = Arc::new(MetricsCollector::new(config));
    
    // Start monitoring system
    collector.start().await?;
    
    // Create dashboard provider and Prometheus exporter
    let dashboard = DashboardProvider::new(Arc::clone(&collector));
    let prometheus_exporter = PrometheusExporter::new(Arc::clone(&collector));
    
    println!("📊 Monitoring system started");
    println!("   - Prometheus metrics: http://localhost:9090/metrics");
    println!("   - Health checks: http://localhost:8080/health");
    println!("   - Dashboard data: http://localhost:8080/metrics/dashboard");
    
    // Subscribe to alerts
    let mut alert_receiver = collector.subscribe_alerts();
    tokio::spawn(async move {
        while let Ok(alert) = alert_receiver.recv().await {
            match alert.severity {
                AlertSeverity::Info => println!("ℹ️  {}: {}", alert.title, alert.description),
                AlertSeverity::Warning => println!("⚠️  {}: {}", alert.title, alert.description),
                AlertSeverity::Critical => println!("🔴 {}: {}", alert.title, alert.description),
                AlertSeverity::Emergency => println!("🚨 EMERGENCY - {}: {}", alert.title, alert.description),
            }
        }
    });
    
    // Simulate swarm activity
    println!("\n🤖 Simulating swarm activity...");
    
    // Create some agents
    let agents = vec![
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    ];
    
    // Simulate agents joining
    for &agent_id in &agents {
        let event = SwarmEvent::AgentJoined {
            agent_id,
            role: AgentRole::Worker,
            timestamp: Utc::now(),
        };
        collector.record_event(event).await;
    }
    
    sleep(Duration::from_secs(1)).await;
    
    // Simulate task assignments and completions
    for i in 0..10 {
        let agent_id = agents[i % agents.len()];
        let task_id = Uuid::new_v4();
        
        // Task assignment
        let assignment_event = SwarmEvent::TaskAssigned {
            task_id,
            agent_id,
            timestamp: Utc::now(),
        };
        collector.record_event(assignment_event).await;
        
        // Simulate task execution
        sleep(Duration::from_millis(100 + i as u64 * 50)).await;
        
        // Task completion (90% success rate)
        let success = i % 10 != 0; // Fail every 10th task
        let completion_event = SwarmEvent::TaskCompleted {
            task_id,
            agent_id,
            success,
            timestamp: Utc::now(),
        };
        collector.record_event(completion_event).await;
        
        // Record detailed task result
        let task_result = TaskResult {
            task_id,
            agent_id,
            success,
            output: if success { Some(serde_json::json!({"result": "completed"})) } else { None },
            error: if !success { Some("Simulated failure".to_string()) } else { None },
            verification_proof: None,
            duration_ms: 100 + i as u64 * 50,
            completed_at: Utc::now(),
        };
        collector.record_task_result(&task_result).await;
        
        // Simulate promises
        if i % 3 == 0 {
            let promise_event = SwarmEvent::PromiseMade {
                agent_id,
                promise_id: Uuid::new_v4(),
                timestamp: Utc::now(),
            };
            collector.record_event(promise_event).await;
            
            // Most promises are fulfilled
            if i % 7 != 0 {
                let fulfillment_event = SwarmEvent::PromiseFulfilled {
                    agent_id,
                    promise_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                };
                collector.record_event(fulfillment_event).await;
            } else {
                // Occasionally break a promise to trigger alert
                let broken_event = SwarmEvent::PromiseBroken {
                    agent_id,
                    promise_id: Uuid::new_v4(),
                    reason: "Resource unavailable".to_string(),
                    timestamp: Utc::now(),
                };
                collector.record_event(broken_event).await;
            }
        }
        
        // Simulate trust updates
        if i % 4 == 0 {
            let trust_value = if success { 0.8 + (i as f64 * 0.01) } else { 0.3 - (i as f64 * 0.01) };
            let trust_event = SwarmEvent::TrustUpdated {
                agent_id,
                old_score: 0.5,
                new_score: trust_value.clamp(0.0, 1.0),
                timestamp: Utc::now(),
            };
            collector.record_event(trust_event).await;
        }
        
        // Simulate verifications
        let verification_event = SwarmEvent::VerificationCompleted {
            task_id,
            verified: success && i % 5 != 0, // Some successful tasks are not verified
            timestamp: Utc::now(),
        };
        collector.record_event(verification_event).await;
    }
    
    println!("📈 Generated sample metrics and events");
    
    // Wait for metrics to be processed
    sleep(Duration::from_secs(2)).await;
    
    // Display dashboard data
    println!("\n📊 Current Dashboard Metrics:");
    let dashboard_metrics = dashboard.get_dashboard_data().await;
    println!("   Total Agents: {}", dashboard_metrics.swarm_metrics.total_agents);
    println!("   Active Agents: {}", dashboard_metrics.swarm_metrics.active_agents);
    println!("   Tasks Assigned: {}", dashboard_metrics.swarm_metrics.tasks_assigned);
    println!("   Tasks Succeeded: {}", dashboard_metrics.swarm_metrics.tasks_succeeded);
    println!("   Tasks Failed: {}", dashboard_metrics.swarm_metrics.tasks_failed);
    println!("   Promises Made: {}", dashboard_metrics.swarm_metrics.promises_made);
    println!("   Promises Fulfilled: {}", dashboard_metrics.swarm_metrics.promises_fulfilled);
    println!("   Average Trust Score: {:.3}", dashboard_metrics.swarm_metrics.avg_trust_score);
    println!("   Active Alerts: {}", dashboard_metrics.active_alerts.len());
    
    if !dashboard_metrics.active_alerts.is_empty() {
        println!("\n🚨 Active Alerts:");
        for alert in &dashboard_metrics.active_alerts {
            println!("   [{:?}] {}: {}", alert.severity, alert.title, alert.description);
        }
    }
    
    // Display agent details
    println!("\n👥 Agent Details:");
    for (agent_id, metrics) in &dashboard_metrics.agent_metrics {
        println!("   Agent {}: Trust={:.3}, Tasks={}/{}, Promises={}/{}",
            agent_id,
            metrics.trust_score.value,
            metrics.tasks_completed,
            metrics.tasks_completed + metrics.tasks_failed,
            metrics.promises_fulfilled,
            metrics.promises_made
        );
    }
    
    // Display health status
    println!("\n🏥 Health Status:");
    let health = collector.get_health_status().await;
    println!("   Overall Status: {:?}", health.overall_status);
    println!("   Uptime: {:?}", health.uptime);
    for (component, status) in &health.components {
        println!("   {}: {:?} - {}", component, status.status, status.message);
    }
    
    // Generate some Prometheus metrics
    println!("\n📊 Sample Prometheus Metrics:");
    let prometheus_metrics = prometheus_exporter.get_metrics().await;
    for line in prometheus_metrics.lines().take(20) {
        if !line.starts_with('#') {
            println!("   {}", line);
        }
    }
    
    // Simulate some high-frequency events to show alerting
    println!("\n🔔 Testing alert system with failures...");
    for i in 0..5 {
        let agent_id = agents[0];
        let task_id = Uuid::new_v4();
        
        // All tasks fail to trigger failure rate alert
        let completion_event = SwarmEvent::TaskCompleted {
            task_id,
            agent_id,
            success: false,
            timestamp: Utc::now(),
        };
        collector.record_event(completion_event).await;
        
        // Record slow task result to trigger execution time alert
        let task_result = TaskResult {
            task_id,
            agent_id,
            success: false,
            output: None,
            error: Some("Timeout".to_string()),
            verification_proof: None,
            duration_ms: 350_000, // 5.8 minutes - exceeds threshold
            completed_at: Utc::now(),
        };
        collector.record_task_result(&task_result).await;
        
        sleep(Duration::from_millis(500)).await;
    }
    
    // Update trust to very low value to trigger trust alert
    let low_trust_event = SwarmEvent::TrustUpdated {
        agent_id: agents[0],
        old_score: 0.5,
        new_score: 0.15, // Below threshold
        timestamp: Utc::now(),
    };
    collector.record_event(low_trust_event).await;
    
    sleep(Duration::from_secs(1)).await;
    
    // Show final metrics
    let final_metrics = dashboard.get_dashboard_data().await;
    println!("\n📊 Final Metrics:");
    println!("   Success Rate: {:.1}%", 
        (final_metrics.swarm_metrics.tasks_succeeded as f64 / final_metrics.swarm_metrics.tasks_assigned as f64) * 100.0);
    println!("   Promise Fulfillment Rate: {:.1}%",
        (final_metrics.swarm_metrics.promises_fulfilled as f64 / final_metrics.swarm_metrics.promises_made as f64) * 100.0);
    println!("   Total Active Alerts: {}", final_metrics.active_alerts.len());
    
    println!("\n✅ Monitoring demo completed!");
    println!("   Keep the demo running to view metrics at:");
    println!("   - http://localhost:9090/metrics (Prometheus)");
    println!("   - http://localhost:8080/health (Health Check)");
    println!("   - http://localhost:8080/metrics/dashboard (Dashboard JSON)");
    
    // Keep running to allow external metric collection
    println!("\n⏳ Press Ctrl+C to exit...");
    loop {
        sleep(Duration::from_secs(10)).await;
        
        // Periodic health check
        let health = collector.get_health_status().await;
        println!("🔍 Health check - Overall: {:?}, Components: {}", 
            health.overall_status, health.components.len());
    }
}