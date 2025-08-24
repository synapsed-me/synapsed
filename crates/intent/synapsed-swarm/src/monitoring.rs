//! Comprehensive monitoring and metrics system for synapsed-swarm
//!
//! This module provides real-time metrics collection, Prometheus integration,
//! alerting capabilities, and health checks for swarm operations.

use crate::{
    types::{AgentId, SwarmEvent, SwarmMetrics, TaskResult, AgentStatus},
    trust::{TrustScore, TrustUpdate},
    error::{SwarmError, SwarmResult},
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use metrics::{counter, gauge, histogram, register_counter, register_gauge, register_histogram};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::time::{Duration, Instant};
use tokio::{
    sync::{broadcast, RwLock},
    time::interval,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[cfg(feature = "monitoring")]
use {
    hyper::{Body, Request, Response, StatusCode, Server},
    hyper::service::{make_service_fn, service_fn},
    metrics_exporter_prometheus::PrometheusBuilder,
    tower::ServiceBuilder,
    tower_http::{cors::CorsLayer, trace::TraceLayer},
};

/// Configuration for the monitoring system
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            prometheus_port: 9090,
            health_check_port: 8080,
            collection_interval: Duration::from_secs(5),
            max_events: 10000,
            alert_thresholds: AlertThresholds::default(),
            enable_dashboard: true,
        }
    }
}

/// Alert threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Minimum trust score before alert
    pub min_trust_score: f64,
    /// Maximum task failure rate before alert
    pub max_failure_rate: f64,
    /// Maximum task execution time before alert (seconds)
    pub max_execution_time: u64,
    /// Minimum promise fulfillment rate before alert
    pub min_promise_fulfillment_rate: f64,
    /// Maximum verification failure rate before alert
    pub max_verification_failure_rate: f64,
    /// Minimum agent availability before alert
    pub min_agent_availability: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            min_trust_score: 0.3,
            max_failure_rate: 0.2,
            max_execution_time: 300, // 5 minutes
            min_promise_fulfillment_rate: 0.8,
            max_verification_failure_rate: 0.1,
            min_agent_availability: 0.7,
        }
    }
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Alert information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub agent_id: Option<AgentId>,
    pub timestamp: DateTime<Utc>,
    pub resolved: bool,
    pub resolution_time: Option<DateTime<Utc>>,
}

/// Real-time metrics for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub timestamp: DateTime<Utc>,
    pub swarm_metrics: SwarmMetrics,
    pub agent_metrics: HashMap<AgentId, AgentMetrics>,
    pub recent_events: Vec<SwarmEvent>,
    pub active_alerts: Vec<Alert>,
    pub performance_trends: PerformanceTrends,
}

/// Per-agent metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub agent_id: AgentId,
    pub status: AgentStatus,
    pub trust_score: TrustScore,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub avg_execution_time_ms: f64,
    pub promises_made: u64,
    pub promises_fulfilled: u64,
    pub last_activity: DateTime<Utc>,
    pub cpu_usage: Option<f64>,
    pub memory_usage: Option<u64>,
}

/// Performance trend data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrends {
    pub task_completion_rate: VecDeque<(DateTime<Utc>, f64)>,
    pub trust_score_trend: VecDeque<(DateTime<Utc>, f64)>,
    pub verification_success_rate: VecDeque<(DateTime<Utc>, f64)>,
    pub promise_fulfillment_rate: VecDeque<(DateTime<Utc>, f64)>,
}

/// Health status of the swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub overall_status: HealthLevel,
    pub components: HashMap<String, ComponentHealth>,
    pub timestamp: DateTime<Utc>,
    pub uptime: Duration,
}

/// Component health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: HealthLevel,
    pub message: String,
    pub last_check: DateTime<Utc>,
}

/// Health level enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthLevel {
    Healthy,
    Degraded,
    Unhealthy,
    Critical,
}

/// Main metrics collector for swarm operations
pub struct MetricsCollector {
    config: MonitoringConfig,
    start_time: Instant,
    
    // Event storage
    events: Arc<RwLock<VecDeque<SwarmEvent>>>,
    
    // Agent metrics tracking
    agent_metrics: Arc<DashMap<AgentId, AgentMetrics>>,
    
    // Alert management
    alerts: Arc<RwLock<Vec<Alert>>>,
    alert_sender: broadcast::Sender<Alert>,
    
    // Performance trends
    trends: Arc<RwLock<PerformanceTrends>>,
    
    // Atomic counters for high-frequency metrics
    task_counter: AtomicU64,
    success_counter: AtomicU64,
    failure_counter: AtomicU64,
    promise_counter: AtomicU64,
    fulfillment_counter: AtomicU64,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(config: MonitoringConfig) -> Self {
        // Register Prometheus metrics
        register_counter!("swarm_tasks_total", "Total number of tasks assigned");
        register_counter!("swarm_tasks_success_total", "Total number of successful tasks");
        register_counter!("swarm_tasks_failed_total", "Total number of failed tasks");
        register_counter!("swarm_promises_total", "Total number of promises made");
        register_counter!("swarm_promises_fulfilled_total", "Total number of promises fulfilled");
        register_counter!("swarm_promises_broken_total", "Total number of promises broken");
        register_counter!("swarm_verifications_total", "Total number of verifications performed");
        register_counter!("swarm_verifications_success_total", "Total number of successful verifications");
        
        register_gauge!("swarm_agents_active", "Number of active agents");
        register_gauge!("swarm_trust_score_avg", "Average trust score across all agents");
        register_gauge!("swarm_task_success_rate", "Task success rate");
        register_gauge!("swarm_promise_fulfillment_rate", "Promise fulfillment rate");
        register_gauge!("swarm_verification_success_rate", "Verification success rate");
        
        register_histogram!("swarm_task_duration_seconds", "Task execution duration");
        register_histogram!("swarm_verification_duration_seconds", "Verification duration");

        let (alert_sender, _) = broadcast::channel(1000);

        Self {
            config,
            start_time: Instant::now(),
            events: Arc::new(RwLock::new(VecDeque::new())),
            agent_metrics: Arc::new(DashMap::new()),
            alerts: Arc::new(RwLock::new(Vec::new())),
            alert_sender,
            trends: Arc::new(RwLock::new(PerformanceTrends {
                task_completion_rate: VecDeque::new(),
                trust_score_trend: VecDeque::new(),
                verification_success_rate: VecDeque::new(),
                promise_fulfillment_rate: VecDeque::new(),
            })),
            task_counter: AtomicU64::new(0),
            success_counter: AtomicU64::new(0),
            failure_counter: AtomicU64::new(0),
            promise_counter: AtomicU64::new(0),
            fulfillment_counter: AtomicU64::new(0),
        }
    }

    /// Record a swarm event
    pub async fn record_event(&self, event: SwarmEvent) {
        // Update atomic counters and Prometheus metrics
        self.update_metrics_from_event(&event).await;
        
        // Store event
        {
            let mut events = self.events.write().await;
            events.push_back(event.clone());
            if events.len() > self.config.max_events {
                events.pop_front();
            }
        }
        
        // Check for alerts
        self.check_alerts(&event).await;
        
        debug!("Recorded swarm event: {:?}", event);
    }

    /// Update metrics based on event
    async fn update_metrics_from_event(&self, event: &SwarmEvent) {
        match event {
            SwarmEvent::TaskAssigned { agent_id, .. } => {
                self.task_counter.fetch_add(1, Ordering::Relaxed);
                counter!("swarm_tasks_total").increment(1);
                self.update_agent_metrics(*agent_id, |metrics| {
                    metrics.last_activity = Utc::now();
                }).await;
            },
            
            SwarmEvent::TaskCompleted { agent_id, success, .. } => {
                if *success {
                    self.success_counter.fetch_add(1, Ordering::Relaxed);
                    counter!("swarm_tasks_success_total").increment(1);
                } else {
                    self.failure_counter.fetch_add(1, Ordering::Relaxed);
                    counter!("swarm_tasks_failed_total").increment(1);
                }
                
                self.update_agent_metrics(*agent_id, |metrics| {
                    if *success {
                        metrics.tasks_completed += 1;
                    } else {
                        metrics.tasks_failed += 1;
                    }
                    metrics.last_activity = Utc::now();
                }).await;
            },
            
            SwarmEvent::PromiseMade { agent_id, .. } => {
                self.promise_counter.fetch_add(1, Ordering::Relaxed);
                counter!("swarm_promises_total").increment(1);
                self.update_agent_metrics(*agent_id, |metrics| {
                    metrics.promises_made += 1;
                    metrics.last_activity = Utc::now();
                }).await;
            },
            
            SwarmEvent::PromiseFulfilled { agent_id, .. } => {
                self.fulfillment_counter.fetch_add(1, Ordering::Relaxed);
                counter!("swarm_promises_fulfilled_total").increment(1);
                self.update_agent_metrics(*agent_id, |metrics| {
                    metrics.promises_fulfilled += 1;
                    metrics.last_activity = Utc::now();
                }).await;
            },
            
            SwarmEvent::PromiseBroken { .. } => {
                counter!("swarm_promises_broken_total").increment(1);
            },
            
            SwarmEvent::VerificationCompleted { verified, .. } => {
                counter!("swarm_verifications_total").increment(1);
                if *verified {
                    counter!("swarm_verifications_success_total").increment(1);
                }
            },
            
            SwarmEvent::TrustUpdated { agent_id, new_score, .. } => {
                self.update_agent_metrics(*agent_id, |metrics| {
                    metrics.trust_score.value = *new_score;
                    metrics.last_activity = Utc::now();
                }).await;
            },
            
            _ => {}
        }
        
        // Update gauge metrics
        self.update_gauge_metrics().await;
    }

    /// Update gauge metrics
    async fn update_gauge_metrics(&self) {
        let active_agents = self.agent_metrics.len() as f64;
        gauge!("swarm_agents_active").set(active_agents);
        
        let total_tasks = self.task_counter.load(Ordering::Relaxed) as f64;
        let successful_tasks = self.success_counter.load(Ordering::Relaxed) as f64;
        let success_rate = if total_tasks > 0.0 { successful_tasks / total_tasks } else { 0.0 };
        gauge!("swarm_task_success_rate").set(success_rate);
        
        let total_promises = self.promise_counter.load(Ordering::Relaxed) as f64;
        let fulfilled_promises = self.fulfillment_counter.load(Ordering::Relaxed) as f64;
        let fulfillment_rate = if total_promises > 0.0 { fulfilled_promises / total_promises } else { 0.0 };
        gauge!("swarm_promise_fulfillment_rate").set(fulfillment_rate);
        
        // Calculate average trust score
        let trust_scores: Vec<f64> = self.agent_metrics
            .iter()
            .map(|entry| entry.trust_score.value)
            .collect();
        
        let avg_trust = if trust_scores.is_empty() { 
            0.0 
        } else { 
            trust_scores.iter().sum::<f64>() / trust_scores.len() as f64 
        };
        gauge!("swarm_trust_score_avg").set(avg_trust);
    }

    /// Update agent metrics
    async fn update_agent_metrics<F>(&self, agent_id: AgentId, update_fn: F) 
    where 
        F: FnOnce(&mut AgentMetrics),
    {
        let mut entry = self.agent_metrics.entry(agent_id).or_insert_with(|| {
            AgentMetrics {
                agent_id,
                status: AgentStatus::Ready,
                trust_score: TrustScore::default(),
                tasks_completed: 0,
                tasks_failed: 0,
                avg_execution_time_ms: 0.0,
                promises_made: 0,
                promises_fulfilled: 0,
                last_activity: Utc::now(),
                cpu_usage: None,
                memory_usage: None,
            }
        });
        
        update_fn(&mut entry);
    }

    /// Record task result with detailed metrics
    pub async fn record_task_result(&self, result: &TaskResult) {
        // Record execution time
        histogram!("swarm_task_duration_seconds")
            .record(result.duration_ms as f64 / 1000.0);
        
        // Update agent metrics
        self.update_agent_metrics(result.agent_id, |metrics| {
            let total_tasks = metrics.tasks_completed + metrics.tasks_failed;
            if total_tasks > 0 {
                metrics.avg_execution_time_ms = (metrics.avg_execution_time_ms * total_tasks as f64 + result.duration_ms as f64) / (total_tasks + 1) as f64;
            } else {
                metrics.avg_execution_time_ms = result.duration_ms as f64;
            }
        }).await;
        
        // Check for performance alerts
        if result.duration_ms > self.config.alert_thresholds.max_execution_time * 1000 {
            self.trigger_alert(Alert {
                id: Uuid::new_v4(),
                severity: AlertSeverity::Warning,
                title: "Task Execution Time Alert".to_string(),
                description: format!("Task {} took {}ms to execute, exceeding threshold", 
                    result.task_id, result.duration_ms),
                agent_id: Some(result.agent_id),
                timestamp: Utc::now(),
                resolved: false,
                resolution_time: None,
            }).await;
        }
    }

    /// Record trust update
    pub async fn record_trust_update(&self, update: &TrustUpdate) {
        // Check for trust score alerts
        if update.current.value < self.config.alert_thresholds.min_trust_score {
            let severity = if update.current.value < 0.1 {
                AlertSeverity::Critical
            } else {
                AlertSeverity::Warning
            };
            
            self.trigger_alert(Alert {
                id: Uuid::new_v4(),
                severity,
                title: "Low Trust Score Alert".to_string(),
                description: format!("Agent {} trust score dropped to {:.3}", 
                    update.agent_id, update.current.value),
                agent_id: Some(update.agent_id),
                timestamp: Utc::now(),
                resolved: false,
                resolution_time: None,
            }).await;
        }
        
        // Update trends
        {
            let mut trends = self.trends.write().await;
            trends.trust_score_trend.push_back((Utc::now(), update.current.value));
            if trends.trust_score_trend.len() > 1000 {
                trends.trust_score_trend.pop_front();
            }
        }
    }

    /// Check for alerts based on event
    async fn check_alerts(&self, event: &SwarmEvent) {
        match event {
            SwarmEvent::TaskCompleted { success: false, agent_id, .. } => {
                let metrics = self.agent_metrics.get(agent_id);
                if let Some(metrics) = metrics {
                    let total_tasks = metrics.tasks_completed + metrics.tasks_failed;
                    let failure_rate = metrics.tasks_failed as f64 / total_tasks as f64;
                    
                    if failure_rate > self.config.alert_thresholds.max_failure_rate && total_tasks >= 10 {
                        self.trigger_alert(Alert {
                            id: Uuid::new_v4(),
                            severity: AlertSeverity::Warning,
                            title: "High Task Failure Rate".to_string(),
                            description: format!("Agent {} has failure rate of {:.1}%", 
                                agent_id, failure_rate * 100.0),
                            agent_id: Some(*agent_id),
                            timestamp: Utc::now(),
                            resolved: false,
                            resolution_time: None,
                        }).await;
                    }
                }
            },
            
            SwarmEvent::PromiseBroken { agent_id, reason, .. } => {
                self.trigger_alert(Alert {
                    id: Uuid::new_v4(),
                    severity: AlertSeverity::Warning,
                    title: "Promise Violation".to_string(),
                    description: format!("Agent {} broke promise: {}", agent_id, reason),
                    agent_id: Some(*agent_id),
                    timestamp: Utc::now(),
                    resolved: false,
                    resolution_time: None,
                }).await;
            },
            
            _ => {}
        }
    }

    /// Trigger an alert
    async fn trigger_alert(&self, alert: Alert) {
        {
            let mut alerts = self.alerts.write().await;
            alerts.push(alert.clone());
        }
        
        // Send alert through broadcast channel
        if let Err(e) = self.alert_sender.send(alert.clone()) {
            warn!("Failed to broadcast alert: {}", e);
        }
        
        // Log alert based on severity
        match alert.severity {
            AlertSeverity::Info => info!("{}: {}", alert.title, alert.description),
            AlertSeverity::Warning => warn!("{}: {}", alert.title, alert.description),
            AlertSeverity::Critical => error!("{}: {}", alert.title, alert.description),
            AlertSeverity::Emergency => error!("EMERGENCY - {}: {}", alert.title, alert.description),
        }
    }

    /// Get current dashboard metrics
    pub async fn get_dashboard_metrics(&self) -> DashboardMetrics {
        let events = self.events.read().await;
        let alerts = self.alerts.read().await;
        let trends = self.trends.read().await;
        
        let agent_metrics: HashMap<AgentId, AgentMetrics> = self.agent_metrics
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect();
        
        let swarm_metrics = SwarmMetrics {
            total_agents: agent_metrics.len(),
            active_agents: agent_metrics.values()
                .filter(|m| matches!(m.status, AgentStatus::Ready | AgentStatus::Busy | AgentStatus::Cooperating))
                .count(),
            tasks_assigned: self.task_counter.load(Ordering::Relaxed) as usize,
            tasks_succeeded: self.success_counter.load(Ordering::Relaxed) as usize,
            tasks_failed: self.failure_counter.load(Ordering::Relaxed) as usize,
            promises_made: self.promise_counter.load(Ordering::Relaxed) as usize,
            promises_fulfilled: self.fulfillment_counter.load(Ordering::Relaxed) as usize,
            promises_broken: 0, // Would need separate counter
            avg_task_duration_ms: agent_metrics.values()
                .map(|m| m.avg_execution_time_ms)
                .sum::<f64>() / agent_metrics.len().max(1) as f64,
            avg_trust_score: agent_metrics.values()
                .map(|m| m.trust_score.value)
                .sum::<f64>() / agent_metrics.len().max(1) as f64,
            verification_success_rate: 0.95, // Would calculate from verification events
        };
        
        DashboardMetrics {
            timestamp: Utc::now(),
            swarm_metrics,
            agent_metrics,
            recent_events: events.iter().rev().take(50).cloned().collect(),
            active_alerts: alerts.iter()
                .filter(|a| !a.resolved)
                .cloned()
                .collect(),
            performance_trends: trends.clone(),
        }
    }

    /// Get health status
    pub async fn get_health_status(&self) -> HealthStatus {
        let mut components = HashMap::new();
        
        // Check agent health
        let active_agents = self.agent_metrics.len();
        let agent_status = if active_agents > 0 {
            HealthLevel::Healthy
        } else {
            HealthLevel::Degraded
        };
        
        components.insert("agents".to_string(), ComponentHealth {
            status: agent_status,
            message: format!("{} active agents", active_agents),
            last_check: Utc::now(),
        });
        
        // Check task success rate
        let total_tasks = self.task_counter.load(Ordering::Relaxed);
        let success_tasks = self.success_counter.load(Ordering::Relaxed);
        let success_rate = if total_tasks > 0 { 
            success_tasks as f64 / total_tasks as f64 
        } else { 
            1.0 
        };
        
        let task_status = if success_rate >= 0.9 {
            HealthLevel::Healthy
        } else if success_rate >= 0.7 {
            HealthLevel::Degraded
        } else {
            HealthLevel::Unhealthy
        };
        
        components.insert("tasks".to_string(), ComponentHealth {
            status: task_status,
            message: format!("{:.1}% success rate", success_rate * 100.0),
            last_check: Utc::now(),
        });
        
        // Determine overall status
        let overall_status = components.values()
            .map(|c| c.status)
            .min_by(|a, b| {
                use HealthLevel::*;
                let order = |h| match h {
                    Healthy => 0,
                    Degraded => 1,
                    Unhealthy => 2,
                    Critical => 3,
                };
                order(*a).cmp(&order(*b))
            })
            .unwrap_or(HealthLevel::Healthy);
        
        HealthStatus {
            overall_status,
            components,
            timestamp: Utc::now(),
            uptime: self.start_time.elapsed(),
        }
    }

    /// Subscribe to alerts
    pub fn subscribe_alerts(&self) -> broadcast::Receiver<Alert> {
        self.alert_sender.subscribe()
    }

    /// Start the monitoring system
    pub async fn start(&self) -> SwarmResult<()> {
        info!("Starting swarm monitoring system");
        
        // Start periodic metrics collection
        let collector = Arc::new(self);
        let metrics_collector = Arc::clone(&collector);
        let collection_interval = self.config.collection_interval;
        
        tokio::spawn(async move {
            let mut interval = interval(collection_interval);
            loop {
                interval.tick().await;
                metrics_collector.collect_periodic_metrics().await;
            }
        });
        
        #[cfg(feature = "monitoring")]
        {
            // Start Prometheus exporter
            if let Err(e) = self.start_prometheus_exporter().await {
                error!("Failed to start Prometheus exporter: {}", e);
            }
            
            // Start health check server
            if let Err(e) = self.start_health_server().await {
                error!("Failed to start health check server: {}", e);
            }
        }
        
        info!("Swarm monitoring system started successfully");
        Ok(())
    }

    /// Collect periodic metrics
    async fn collect_periodic_metrics(&self) {
        // Update performance trends
        {
            let mut trends = self.trends.write().await;
            let now = Utc::now();
            
            // Task completion rate
            let total_tasks = self.task_counter.load(Ordering::Relaxed);
            let success_tasks = self.success_counter.load(Ordering::Relaxed);
            let completion_rate = if total_tasks > 0 { 
                success_tasks as f64 / total_tasks as f64 
            } else { 
                0.0 
            };
            trends.task_completion_rate.push_back((now, completion_rate));
            if trends.task_completion_rate.len() > 1000 {
                trends.task_completion_rate.pop_front();
            }
            
            // Average trust score
            let trust_scores: Vec<f64> = self.agent_metrics
                .iter()
                .map(|entry| entry.trust_score.value)
                .collect();
            let avg_trust = if trust_scores.is_empty() { 
                0.0 
            } else { 
                trust_scores.iter().sum::<f64>() / trust_scores.len() as f64 
            };
            trends.trust_score_trend.push_back((now, avg_trust));
            if trends.trust_score_trend.len() > 1000 {
                trends.trust_score_trend.pop_front();
            }
        }
        
        debug!("Collected periodic metrics");
    }
}

#[cfg(feature = "monitoring")]
impl MetricsCollector {
    /// Start Prometheus metrics exporter
    async fn start_prometheus_exporter(&self) -> SwarmResult<()> {
        use std::net::SocketAddr;
        
        let builder = PrometheusBuilder::new();
        let handle = builder.install_recorder()
            .map_err(|e| SwarmError::MonitoringError(format!("Failed to install Prometheus recorder: {}", e)))?;
        
        let addr: SocketAddr = ([0, 0, 0, 0], self.config.prometheus_port).into();
        
        let make_svc = make_service_fn(move |_conn| {
            let handle = handle.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                    let handle = handle.clone();
                    async move {
                        if req.uri().path() == "/metrics" {
                            let metrics = handle.render();
                            Ok(Response::new(Body::from(metrics)))
                        } else {
                            let mut not_found = Response::new(Body::from("Not Found"));
                            *not_found.status_mut() = StatusCode::NOT_FOUND;
                            Ok(not_found)
                        }
                    }
                }))
            }
        });
        
        let server = Server::bind(&addr).serve(make_svc);
        
        tokio::spawn(async move {
            if let Err(e) = server.await {
                error!("Prometheus server error: {}", e);
            }
        });
        
        info!("Prometheus metrics server started on {}", addr);
        Ok(())
    }

    /// Start health check server
    async fn start_health_server(&self) -> SwarmResult<()> {
        use std::net::SocketAddr;
        
        let collector = Arc::new(self);
        let addr: SocketAddr = ([0, 0, 0, 0], self.config.health_check_port).into();
        
        let make_svc = make_service_fn(move |_conn| {
            let collector = Arc::clone(&collector);
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                    let collector = Arc::clone(&collector);
                    async move {
                        match req.uri().path() {
                            "/health" => {
                                let health = collector.get_health_status().await;
                                let json = serde_json::to_string(&health).unwrap();
                                Ok(Response::new(Body::from(json)))
                            },
                            "/metrics/dashboard" => {
                                let metrics = collector.get_dashboard_metrics().await;
                                let json = serde_json::to_string(&metrics).unwrap();
                                Ok(Response::new(Body::from(json)))
                            },
                            _ => {
                                let mut not_found = Response::new(Body::from("Not Found"));
                                *not_found.status_mut() = StatusCode::NOT_FOUND;
                                Ok(not_found)
                            }
                        }
                    }
                }))
            }
        });
        
        let service = ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::permissive())
            .service_fn(make_svc);
        
        let server = Server::bind(&addr).serve(make_svc);
        
        tokio::spawn(async move {
            if let Err(e) = server.await {
                error!("Health server error: {}", e);
            }
        });
        
        info!("Health check server started on {}", addr);
        Ok(())
    }
}

/// Prometheus exporter for swarm metrics
pub struct PrometheusExporter {
    collector: Arc<MetricsCollector>,
}

impl PrometheusExporter {
    /// Create a new Prometheus exporter
    pub fn new(collector: Arc<MetricsCollector>) -> Self {
        Self { collector }
    }

    /// Get Prometheus formatted metrics
    pub async fn get_metrics(&self) -> String {
        let dashboard_metrics = self.collector.get_dashboard_metrics().await;
        
        // Format metrics in Prometheus format
        let mut output = String::new();
        
        // Swarm-level metrics
        output.push_str(&format!("# HELP swarm_agents_total Total number of agents\n"));
        output.push_str(&format!("# TYPE swarm_agents_total gauge\n"));
        output.push_str(&format!("swarm_agents_total {}\n", dashboard_metrics.swarm_metrics.total_agents));
        
        output.push_str(&format!("# HELP swarm_agents_active Number of active agents\n"));
        output.push_str(&format!("# TYPE swarm_agents_active gauge\n"));
        output.push_str(&format!("swarm_agents_active {}\n", dashboard_metrics.swarm_metrics.active_agents));
        
        output.push_str(&format!("# HELP swarm_tasks_success_rate Task success rate\n"));
        output.push_str(&format!("# TYPE swarm_tasks_success_rate gauge\n"));
        let success_rate = if dashboard_metrics.swarm_metrics.tasks_assigned > 0 {
            dashboard_metrics.swarm_metrics.tasks_succeeded as f64 / dashboard_metrics.swarm_metrics.tasks_assigned as f64
        } else {
            0.0
        };
        output.push_str(&format!("swarm_tasks_success_rate {}\n", success_rate));
        
        // Per-agent metrics
        for (agent_id, metrics) in dashboard_metrics.agent_metrics {
            output.push_str(&format!("# HELP agent_trust_score Trust score for agent\n"));
            output.push_str(&format!("# TYPE agent_trust_score gauge\n"));
            output.push_str(&format!("agent_trust_score{{agent_id=\"{}\"}} {}\n", 
                agent_id, metrics.trust_score.value));
            
            output.push_str(&format!("# HELP agent_tasks_completed Tasks completed by agent\n"));
            output.push_str(&format!("# TYPE agent_tasks_completed counter\n"));
            output.push_str(&format!("agent_tasks_completed{{agent_id=\"{}\"}} {}\n", 
                agent_id, metrics.tasks_completed));
        }
        
        output
    }
}

/// Real-time dashboard data provider
pub struct DashboardProvider {
    collector: Arc<MetricsCollector>,
}

impl DashboardProvider {
    /// Create a new dashboard provider
    pub fn new(collector: Arc<MetricsCollector>) -> Self {
        Self { collector }
    }

    /// Get real-time dashboard data
    pub async fn get_dashboard_data(&self) -> DashboardMetrics {
        self.collector.get_dashboard_metrics().await
    }

    /// Get agent details
    pub async fn get_agent_details(&self, agent_id: AgentId) -> Option<AgentMetrics> {
        self.collector.agent_metrics.get(&agent_id).map(|entry| entry.clone())
    }

    /// Get performance trends
    pub async fn get_performance_trends(&self, duration: Duration) -> PerformanceTrends {
        let trends = self.collector.trends.read().await;
        let cutoff = Utc::now() - chrono::Duration::from_std(duration).unwrap_or_default();
        
        PerformanceTrends {
            task_completion_rate: trends.task_completion_rate.iter()
                .filter(|(timestamp, _)| *timestamp > cutoff)
                .cloned()
                .collect(),
            trust_score_trend: trends.trust_score_trend.iter()
                .filter(|(timestamp, _)| *timestamp > cutoff)
                .cloned()
                .collect(),
            verification_success_rate: trends.verification_success_rate.iter()
                .filter(|(timestamp, _)| *timestamp > cutoff)
                .cloned()
                .collect(),
            promise_fulfillment_rate: trends.promise_fulfillment_rate.iter()
                .filter(|(timestamp, _)| *timestamp > cutoff)
                .cloned()
                .collect(),
        }
    }

    /// Subscribe to real-time updates
    pub fn subscribe_to_updates(&self) -> broadcast::Receiver<Alert> {
        self.collector.subscribe_alerts()
    }
}