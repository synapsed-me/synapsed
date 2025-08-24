//! Test for monitoring module compilation and basic functionality

use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use chrono::Utc;

// Only test the monitoring module if it compiles without dependencies errors
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitoring_config_creation() {
        use synapsed_swarm::monitoring::{MonitoringConfig, AlertThresholds};
        
        let config = MonitoringConfig {
            prometheus_port: 9090,
            health_check_port: 8080,
            collection_interval: Duration::from_secs(5),
            max_events: 1000,
            enable_dashboard: true,
            alert_thresholds: AlertThresholds::default(),
        };
        
        assert_eq!(config.prometheus_port, 9090);
        assert_eq!(config.health_check_port, 8080);
        assert!(config.enable_dashboard);
    }

    #[tokio::test] 
    async fn test_alert_creation() {
        use synapsed_swarm::monitoring::{Alert, AlertSeverity};
        
        let alert = Alert {
            id: Uuid::new_v4(),
            severity: AlertSeverity::Warning,
            title: "Test Alert".to_string(),
            description: "This is a test alert".to_string(),
            agent_id: Some(Uuid::new_v4()),
            timestamp: Utc::now(),
            resolved: false,
            resolution_time: None,
        };
        
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert_eq!(alert.title, "Test Alert");
        assert!(!alert.resolved);
    }

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        use synapsed_swarm::monitoring::{MetricsCollector, MonitoringConfig};
        
        let config = MonitoringConfig::default();
        let collector = Arc::new(MetricsCollector::new(config));
        
        // Test that we can create dashboard metrics without panicking
        let dashboard_metrics = collector.get_dashboard_metrics().await;
        assert_eq!(dashboard_metrics.swarm_metrics.total_agents, 0);
        assert_eq!(dashboard_metrics.swarm_metrics.tasks_assigned, 0);
    }

    #[tokio::test]
    async fn test_health_status_creation() {
        use synapsed_swarm::monitoring::{MetricsCollector, MonitoringConfig, HealthLevel};
        
        let config = MonitoringConfig::default();
        let collector = Arc::new(MetricsCollector::new(config));
        
        let health = collector.get_health_status().await;
        assert!(matches!(health.overall_status, HealthLevel::Healthy | HealthLevel::Degraded));
        assert!(health.components.contains_key("agents"));
        assert!(health.components.contains_key("tasks"));
    }
}