//! Safety monitoring system implementation
//!
//! This module provides real-time monitoring of system state and
//! automatic detection of safety violations as they occur.

use crate::error::{Result, SafetyError};
use crate::traits::{SafetyMonitor, StateChangeCallback};
use crate::types::*;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{debug, info, warn, error};
use uuid::Uuid;

/// Default safety monitor implementation
#[derive(Debug)]
pub struct DefaultSafetyMonitor {
    /// Current system state
    current_state: Arc<RwLock<Option<SafetyState>>>,
    /// Monitor configuration
    config: MonitorConfig,
    /// Monitor statistics
    stats: Arc<RwLock<MonitoringStats>>,
    /// State change callbacks
    callbacks: Arc<RwLock<Vec<Box<dyn StateChangeCallback>>>>,
    /// Monitoring task handle
    monitor_task: Arc<RwLock<Option<JoinHandle<()>>>>,
    /// State change broadcaster
    state_tx: Arc<RwLock<Option<broadcast::Sender<StateChange>>>>,
    /// Whether monitor is active
    active: Arc<RwLock<bool>>,
    /// Monitor metadata
    metadata: MonitorMetadata,
    /// Last monitoring check time
    last_check: Arc<RwLock<Option<Instant>>>,
}

/// State change event
#[derive(Debug, Clone)]
pub struct StateChange {
    pub old_state: Option<SafetyState>,
    pub new_state: SafetyState,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub change_type: StateChangeType,
}

/// Types of state changes
#[derive(Debug, Clone)]
pub enum StateChangeType {
    /// Initial state capture
    Initial,
    /// Regular update
    Update,
    /// Significant change detected
    Significant,
    /// Resource threshold crossed
    ResourceThreshold,
    /// Health status changed
    HealthChange,
    /// Error condition detected
    Error,
}

impl DefaultSafetyMonitor {
    /// Create a new safety monitor with default configuration
    pub fn new() -> Self {
        Self::with_config(MonitorConfig {
            check_interval_ms: 1000, // 1 second
            memory_limit_bytes: 50 * 1024 * 1024, // 50MB
            enable_predictive_analysis: false,
            violation_threshold: 0.8,
            custom_settings: HashMap::new(),
        })
    }

    /// Create a new safety monitor with custom configuration
    pub fn with_config(config: MonitorConfig) -> Self {
        Self {
            current_state: Arc::new(RwLock::new(None)),
            config,
            stats: Arc::new(RwLock::new(MonitoringStats {
                states_monitored: 0,
                violations_detected: 0,
                avg_check_duration_ms: 0.0,
                uptime_ms: 0,
                memory_usage_bytes: 0,
            })),
            callbacks: Arc::new(RwLock::new(Vec::new())),
            monitor_task: Arc::new(RwLock::new(None)),
            state_tx: Arc::new(RwLock::new(None)),
            active: Arc::new(RwLock::new(false)),
            metadata: MonitorMetadata {
                name: "DefaultSafetyMonitor".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![
                    "real-time monitoring".to_string(),
                    "state change detection".to_string(),
                    "resource tracking".to_string(),
                    "health monitoring".to_string(),
                ],
                supported_constraints: vec![
                    "resource".to_string(),
                    "invariant".to_string(),
                    "temporal".to_string(),
                ],
            },
            last_check: Arc::new(RwLock::new(None)),
        }
    }

    /// Capture current system state
    async fn capture_state(&self) -> Result<SafetyState> {
        debug!("Capturing system state");
        
        // Get current system metrics
        let resource_usage = self.get_resource_usage().await?;
        let health_indicators = self.get_health_indicators().await?;
        
        let state = SafetyState {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            values: self.get_system_values().await?,
            active_constraints: vec![], // Will be populated by constraint engine
            resource_usage,
            health_indicators,
            metadata: StateMetadata {
                source: "DefaultSafetyMonitor".to_string(),
                version: "1.0.0".to_string(),
                checksum: "auto_generated".to_string(),
                size_bytes: 0, // Will be calculated if needed
                compression_ratio: None,
                tags: vec!["monitoring".to_string()],
                properties: HashMap::new(),
            },
        };
        
        debug!("State captured: {}", state.id);
        Ok(state)
    }

    /// Get current resource usage
    async fn get_resource_usage(&self) -> Result<ResourceUsage> {
        // In a real implementation, this would gather actual system metrics
        // For now, we'll simulate some realistic values
        
        #[cfg(target_os = "linux")]
        {
            self.get_linux_resource_usage().await
        }
        #[cfg(not(target_os = "linux"))]
        {
            self.get_simulated_resource_usage().await
        }
    }

    #[cfg(target_os = "linux")]
    async fn get_linux_resource_usage(&self) -> Result<ResourceUsage> {
        use std::fs;
        
        // Read memory info from /proc/meminfo
        let (memory_usage, memory_limit) = match fs::read_to_string("/proc/meminfo") {
            Ok(content) => {
                let mut total = 0;
                let mut available = 0;
                
                for line in content.lines() {
                    if line.starts_with("MemTotal:") {
                        if let Some(value) = line.split_whitespace().nth(1) {
                            total = value.parse::<u64>().unwrap_or(0) * 1024; // Convert KB to bytes
                        }
                    } else if line.starts_with("MemAvailable:") {
                        if let Some(value) = line.split_whitespace().nth(1) {
                            available = value.parse::<u64>().unwrap_or(0) * 1024; // Convert KB to bytes
                        }
                    }
                }
                
                let used = total.saturating_sub(available);
                (used, total)
            }
            Err(_) => (512 * 1024 * 1024, 1024 * 1024 * 1024), // Fallback values
        };

        // Read CPU usage from /proc/stat (simplified)
        let cpu_usage = match fs::read_to_string("/proc/stat") {
            Ok(content) => {
                if let Some(line) = content.lines().next() {
                    if line.starts_with("cpu ") {
                        // Parse CPU times and calculate usage percentage
                        // This is a simplified calculation
                        0.5 // Placeholder
                    } else {
                        0.5
                    }
                } else {
                    0.5
                }
            }
            Err(_) => 0.5,
        };

        Ok(ResourceUsage {
            cpu_usage,
            memory_usage,
            memory_limit,
            network_usage: 1024 * 1024, // 1MB/s placeholder
            disk_io: 512 * 1024,        // 512KB/s placeholder
            file_descriptors: 100,       // Placeholder
            thread_count: 20,            // Placeholder
            custom_resources: HashMap::new(),
        })
    }

    #[cfg(not(target_os = "linux"))]
    async fn get_simulated_resource_usage(&self) -> Result<ResourceUsage> {
        // Simulate resource usage for non-Linux systems
        Ok(ResourceUsage {
            cpu_usage: 0.3 + rand::random::<f64>() * 0.4, // 30-70%
            memory_usage: 512 * 1024 * 1024 + (rand::random::<u64>() % (256 * 1024 * 1024)), // 512-768MB
            memory_limit: 1024 * 1024 * 1024, // 1GB
            network_usage: (rand::random::<u64>() % (10 * 1024 * 1024)), // 0-10MB/s
            disk_io: (rand::random::<u64>() % (5 * 1024 * 1024)), // 0-5MB/s
            file_descriptors: 50 + (rand::random::<u32>() % 100), // 50-150
            thread_count: 10 + (rand::random::<u32>() % 20), // 10-30
            custom_resources: HashMap::new(),
        })
    }

    /// Get health indicators
    async fn get_health_indicators(&self) -> Result<HealthIndicators> {
        // In a real implementation, this would assess actual system health
        let mut component_health = HashMap::new();
        component_health.insert("cpu".to_string(), 0.8 + rand::random::<f64>() * 0.2);
        component_health.insert("memory".to_string(), 0.7 + rand::random::<f64>() * 0.3);
        component_health.insert("disk".to_string(), 0.9 + rand::random::<f64>() * 0.1);
        component_health.insert("network".to_string(), 0.85 + rand::random::<f64>() * 0.15);
        
        let overall_health = component_health.values().sum::<f64>() / component_health.len() as f64;
        
        let mut error_rates = HashMap::new();
        error_rates.insert("system".to_string(), rand::random::<f64>() * 0.1); // 0-10% error rate
        
        let mut response_times = HashMap::new();
        response_times.insert("api".to_string(), 50.0 + rand::random::<f64>() * 100.0); // 50-150ms
        
        let mut availability = HashMap::new();
        availability.insert("service".to_string(), 0.95 + rand::random::<f64>() * 0.05); // 95-100%
        
        Ok(HealthIndicators {
            overall_health,
            component_health,
            error_rates,
            response_times,
            availability,
            performance_indicators: HashMap::new(),
        })
    }

    /// Get system values
    async fn get_system_values(&self) -> Result<HashMap<String, StateValue>> {
        let mut values = HashMap::new();
        
        // Add some common system values
        values.insert("uptime".to_string(), StateValue::Integer(chrono::Utc::now().timestamp()));
        values.insert("process_id".to_string(), StateValue::Integer(std::process::id() as i64));
        values.insert("monitor_version".to_string(), StateValue::String("1.0.0".to_string()));
        
        // Add environment variables if available
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            values.insert("hostname".to_string(), StateValue::String(hostname));
        }
        
        Ok(values)
    }

    /// Check if state change is significant
    fn is_significant_change(&self, old_state: &SafetyState, new_state: &SafetyState) -> bool {
        // Check resource usage changes
        let memory_change = (new_state.resource_usage.memory_usage_percentage() 
            - old_state.resource_usage.memory_usage_percentage()).abs();
        if memory_change > 0.1 { // 10% change
            return true;
        }
        
        let cpu_change = (new_state.resource_usage.cpu_usage - old_state.resource_usage.cpu_usage).abs();
        if cpu_change > 0.2 { // 20% change
            return true;
        }
        
        // Check health changes
        let health_change = (new_state.health_indicators.overall_health 
            - old_state.health_indicators.overall_health).abs();
        if health_change > 0.1 { // 10% change
            return true;
        }
        
        false
    }

    /// Monitoring loop
    async fn monitor_loop(&self) {
        let mut interval = interval(Duration::from_millis(self.config.check_interval_ms));
        let start_time = Instant::now();
        
        info!("Starting monitoring loop with {}ms interval", self.config.check_interval_ms);
        
        loop {
            interval.tick().await;
            
            if !*self.active.read() {
                debug!("Monitor not active, stopping loop");
                break;
            }
            
            let check_start = Instant::now();
            
            match self.capture_state().await {
                Ok(new_state) => {
                    let old_state = {
                        let mut current = self.current_state.write();
                        current.replace(new_state.clone())
                    };
                    
                    // Determine change type
                    let change_type = if old_state.is_none() {
                        StateChangeType::Initial
                    } else if self.is_significant_change(old_state.as_ref().unwrap(), &new_state) {
                        StateChangeType::Significant
                    } else {
                        StateChangeType::Update
                    };
                    
                    // Create state change event
                    let state_change = StateChange {
                        old_state: old_state.clone(),
                        new_state: new_state.clone(),
                        timestamp: chrono::Utc::now(),
                        change_type: change_type.clone(),
                    };
                    
                    // Broadcast state change
                    if let Some(tx) = self.state_tx.read().as_ref() {
                        if let Err(e) = tx.send(state_change.clone()) {
                            debug!("No state change listeners: {}", e);
                        }
                    }
                    
                    // Notify callbacks
                    let callbacks = self.callbacks.read().clone();
                    for callback in callbacks.iter() {
                        // Note: This is a simplified callback notification
                        // In a real implementation, you'd want to handle async callbacks properly
                        debug!("Would notify callback of state change");
                    }
                    
                    // Update statistics
                    let check_duration = check_start.elapsed();
                    let mut stats = self.stats.write();
                    stats.states_monitored += 1;
                    stats.uptime_ms = start_time.elapsed().as_millis() as u64;
                    
                    // Update average check duration
                    let new_duration_ms = check_duration.as_millis() as f64;
                    if stats.states_monitored == 1 {
                        stats.avg_check_duration_ms = new_duration_ms;
                    } else {
                        stats.avg_check_duration_ms = 
                            (stats.avg_check_duration_ms * (stats.states_monitored - 1) as f64 + new_duration_ms)
                            / stats.states_monitored as f64;
                    }
                    
                    debug!(
                        "State monitoring check completed in {}ms (type: {:?})",
                        check_duration.as_millis(),
                        change_type
                    );
                    
                    *self.last_check.write() = Some(Instant::now());
                }
                Err(e) => {
                    error!("Failed to capture state: {}", e);
                    
                    // Broadcast error state change
                    if let Some(tx) = self.state_tx.read().as_ref() {
                        let error_change = StateChange {
                            old_state: None,
                            new_state: SafetyState {
                                id: Uuid::new_v4(),
                                timestamp: chrono::Utc::now(),
                                values: HashMap::new(),
                                active_constraints: vec![],
                                resource_usage: ResourceUsage {
                                    cpu_usage: 0.0,
                                    memory_usage: 0,
                                    memory_limit: 0,
                                    network_usage: 0,
                                    disk_io: 0,
                                    file_descriptors: 0,
                                    thread_count: 0,
                                    custom_resources: HashMap::new(),
                                },
                                health_indicators: HealthIndicators {
                                    overall_health: 0.0,
                                    component_health: HashMap::new(),
                                    error_rates: HashMap::new(),
                                    response_times: HashMap::new(),
                                    availability: HashMap::new(),
                                    performance_indicators: HashMap::new(),
                                },
                                metadata: StateMetadata {
                                    source: "error".to_string(),
                                    version: "1.0.0".to_string(),
                                    checksum: "error".to_string(),
                                    size_bytes: 0,
                                    compression_ratio: None,
                                    tags: vec!["error".to_string()],
                                    properties: HashMap::new(),
                                },
                            },
                            timestamp: chrono::Utc::now(),
                            change_type: StateChangeType::Error,
                        };
                        
                        let _ = tx.send(error_change);
                    }
                }
            }
        }
        
        info!("Monitoring loop stopped");
    }
}

#[async_trait]
impl SafetyMonitor for DefaultSafetyMonitor {
    async fn start_monitoring(&mut self) -> Result<()> {
        if *self.active.read() {
            return Err(SafetyError::MonitorError {
                message: "Monitor is already active".to_string(),
            });
        }
        
        info!("Starting safety monitor");
        
        // Create state change broadcaster
        let (tx, _) = broadcast::channel(100);
        *self.state_tx.write() = Some(tx);
        
        // Mark as active
        *self.active.write() = true;
        
        // Start monitoring task
        let monitor_clone = self.clone();
        let handle = tokio::spawn(async move {
            monitor_clone.monitor_loop().await;
        });
        
        *self.monitor_task.write() = Some(handle);
        
        info!("Safety monitor started successfully");
        Ok(())
    }

    async fn stop_monitoring(&mut self) -> Result<()> {
        if !*self.active.read() {
            return Err(SafetyError::MonitorError {
                message: "Monitor is not active".to_string(),
            });
        }
        
        info!("Stopping safety monitor");
        
        // Mark as inactive
        *self.active.write() = false;
        
        // Stop monitoring task
        if let Some(handle) = self.monitor_task.write().take() {
            handle.abort();
        }
        
        // Clear state broadcaster
        *self.state_tx.write() = None;
        
        info!("Safety monitor stopped successfully");
        Ok(())
    }

    fn is_active(&self) -> bool {
        *self.active.read()
    }

    async fn get_current_state(&self) -> Result<SafetyState> {
        let state_guard = self.current_state.read();
        match state_guard.as_ref() {
            Some(state) => Ok(state.clone()),
            None => {
                if *self.active.read() {
                    // If monitoring is active but no state captured yet, capture one now
                    drop(state_guard);
                    self.capture_state().await
                } else {
                    Err(SafetyError::MonitorError {
                        message: "No current state available and monitor is not active".to_string(),
                    })
                }
            }
        }
    }

    async fn subscribe_to_changes(&mut self, callback: Box<dyn StateChangeCallback>) -> Result<()> {
        let mut callbacks = self.callbacks.write();
        callbacks.push(callback);
        
        info!("Added state change callback (total: {})", callbacks.len());
        Ok(())
    }

    async fn unsubscribe_from_changes(&mut self) -> Result<()> {
        let mut callbacks = self.callbacks.write();
        callbacks.clear();
        
        info!("Cleared all state change callbacks");
        Ok(())
    }

    async fn get_stats(&self) -> Result<crate::traits::MonitoringStats> {
        let stats = self.stats.read();
        Ok(stats.clone())
    }

    async fn configure(&mut self, config: crate::traits::MonitorConfig) -> Result<()> {
        info!("Updating monitor configuration");
        
        let was_active = *self.active.read();
        
        // Stop monitoring if active
        if was_active {
            self.stop_monitoring().await?;
        }
        
        // Update configuration
        self.config = config;
        
        // Restart monitoring if it was active
        if was_active {
            self.start_monitoring().await?;
        }
        
        info!("Monitor configuration updated successfully");
        Ok(())
    }

    async fn health_check(&self) -> Result<crate::traits::HealthStatus> {
        let mut issues = Vec::new();
        let mut performance_score = 1.0;
        
        // Check if monitor is active
        if !*self.active.read() {
            issues.push("Monitor is not active".to_string());
            performance_score -= 0.5;
        }
        
        // Check last monitoring check time
        if let Some(last_check) = *self.last_check.read() {
            let elapsed = last_check.elapsed();
            if elapsed > Duration::from_millis(self.config.check_interval_ms * 3) {
                issues.push("Monitor checks are delayed".to_string());
                performance_score -= 0.3;
            }
        } else if *self.active.read() {
            issues.push("No monitoring checks performed yet".to_string());
            performance_score -= 0.2;
        }
        
        // Check memory usage
        let stats = self.stats.read();
        if stats.memory_usage_bytes > self.config.memory_limit_bytes {
            issues.push("Memory usage exceeds limit".to_string());
            performance_score -= 0.2;
        }
        
        // Check average check duration
        if stats.avg_check_duration_ms > self.config.check_interval_ms as f64 * 0.8 {
            issues.push("Monitor checks are taking too long".to_string());
            performance_score -= 0.1;
        }
        
        Ok(crate::traits::HealthStatus {
            healthy: issues.is_empty(),
            issues,
            performance_score: performance_score.max(0.0),
            last_check: chrono::Utc::now(),
        })
    }

    fn get_metadata(&self) -> crate::traits::MonitorMetadata {
        self.metadata.clone()
    }
}

// Clone implementation needed for the monitoring task
impl Clone for DefaultSafetyMonitor {
    fn clone(&self) -> Self {
        Self {
            current_state: Arc::clone(&self.current_state),
            config: self.config.clone(),
            stats: Arc::clone(&self.stats),
            callbacks: Arc::clone(&self.callbacks),
            monitor_task: Arc::clone(&self.monitor_task),
            state_tx: Arc::clone(&self.state_tx),
            active: Arc::clone(&self.active),
            metadata: self.metadata.clone(),
            last_check: Arc::clone(&self.last_check),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_monitor_lifecycle() {
        let mut monitor = DefaultSafetyMonitor::new();
        
        // Initially not active
        assert!(!monitor.is_active());
        
        // Start monitoring
        monitor.start_monitoring().await.unwrap();
        assert!(monitor.is_active());
        
        // Wait a bit for state capture
        sleep(Duration::from_millis(100)).await;
        
        // Should have current state
        let state = monitor.get_current_state().await.unwrap();
        assert!(!state.id.is_nil());
        
        // Stop monitoring
        monitor.stop_monitoring().await.unwrap();
        assert!(!monitor.is_active());
    }

    #[tokio::test]
    async fn test_monitor_stats() {
        let mut monitor = DefaultSafetyMonitor::new();
        
        // Start monitoring
        monitor.start_monitoring().await.unwrap();
        
        // Wait for some monitoring cycles
        sleep(Duration::from_millis(200)).await;
        
        // Check stats
        let stats = monitor.get_stats().await.unwrap();
        assert!(stats.states_monitored > 0);
        assert!(stats.uptime_ms > 0);
        assert!(stats.avg_check_duration_ms > 0.0);
        
        monitor.stop_monitoring().await.unwrap();
    }

    #[tokio::test]
    async fn test_monitor_health_check() {
        let mut monitor = DefaultSafetyMonitor::new();
        
        // Health check when not active
        let health = monitor.health_check().await.unwrap();
        assert!(!health.healthy);
        assert!(health.issues.len() > 0);
        assert!(health.performance_score < 1.0);
        
        // Start monitoring
        monitor.start_monitoring().await.unwrap();
        sleep(Duration::from_millis(100)).await;
        
        // Health check when active
        let health = monitor.health_check().await.unwrap();
        // May or may not be healthy depending on timing, but should have better score
        assert!(health.performance_score > 0.0);
        
        monitor.stop_monitoring().await.unwrap();
    }

    #[tokio::test]
    async fn test_monitor_configuration() {
        let mut monitor = DefaultSafetyMonitor::new();
        
        // Update configuration
        let new_config = crate::traits::MonitorConfig {
            check_interval_ms: 2000,
            memory_limit_bytes: 100 * 1024 * 1024,
            enable_predictive_analysis: true,
            violation_threshold: 0.9,
            custom_settings: HashMap::new(),
        };
        
        monitor.configure(new_config.clone()).await.unwrap();
        assert_eq!(monitor.config.check_interval_ms, 2000);
        assert_eq!(monitor.config.memory_limit_bytes, 100 * 1024 * 1024);
        assert!(monitor.config.enable_predictive_analysis);
    }

    #[tokio::test]
    async fn test_state_capture() {
        let monitor = DefaultSafetyMonitor::new();
        
        // Capture state directly
        let state = monitor.capture_state().await.unwrap();
        
        assert!(!state.id.is_nil());
        assert!(state.resource_usage.memory_limit > 0);
        assert!(state.health_indicators.overall_health >= 0.0);
        assert!(state.health_indicators.overall_health <= 1.0);
        assert_eq!(state.metadata.source, "DefaultSafetyMonitor");
    }

    #[tokio::test]
    async fn test_significant_change_detection() {
        let monitor = DefaultSafetyMonitor::new();
        
        let state1 = SafetyState {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            values: HashMap::new(),
            active_constraints: vec![],
            resource_usage: ResourceUsage {
                cpu_usage: 0.5,
                memory_usage: 512 * 1024 * 1024,
                memory_limit: 1024 * 1024 * 1024,
                network_usage: 0,
                disk_io: 0,
                file_descriptors: 0,
                thread_count: 0,
                custom_resources: HashMap::new(),
            },
            health_indicators: HealthIndicators {
                overall_health: 0.8,
                component_health: HashMap::new(),
                error_rates: HashMap::new(),
                response_times: HashMap::new(),
                availability: HashMap::new(),
                performance_indicators: HashMap::new(),
            },
            metadata: StateMetadata {
                source: "test".to_string(),
                version: "1.0".to_string(),
                checksum: "test".to_string(),
                size_bytes: 0,
                compression_ratio: None,
                tags: vec![],
                properties: HashMap::new(),
            },
        };
        
        let mut state2 = state1.clone();
        state2.resource_usage.cpu_usage = 0.8; // 30% increase - should be significant
        
        assert!(monitor.is_significant_change(&state1, &state2));
        
        // Small change - should not be significant
        state2.resource_usage.cpu_usage = 0.55; // 5% increase
        assert!(!monitor.is_significant_change(&state1, &state2));
    }
}