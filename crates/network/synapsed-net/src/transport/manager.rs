//! Transport manager for protocol selection and connection management.

use crate::error::{NetworkError, Result, TransportError};
use crate::observability::UnifiedObservability;
use crate::transport::{
    traits::{Transport, TransportFeature, TransportPriority, TransportRequirements},
    Connection, ObservableTransport, TransportType,
};
use crate::types::PeerInfo;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Manages multiple transport protocols and selects the best one for each connection.
pub struct TransportManager {
    /// Available transports
    transports: Arc<RwLock<HashMap<TransportType, Arc<dyn Transport + Send + Sync>>>>,
    
    /// Transport metrics for intelligent selection
    transport_metrics: Arc<DashMap<TransportType, TransportMetrics>>,
    
    /// Connection history for peer-specific transport selection
    peer_history: Arc<DashMap<String, PeerTransportHistory>>,
    
    /// Default transport type
    default_transport: TransportType,
    
    /// Transport selection strategy
    selection_strategy: SelectionStrategy,
    
    /// Observability integration
    observability: Option<Arc<UnifiedObservability>>,
}

/// Transport performance metrics.
#[derive(Debug, Clone)]
struct TransportMetrics {
    /// Total connection attempts
    attempts: u64,
    
    /// Successful connections
    successes: u64,
    
    /// Average connection time
    avg_connection_time: Duration,
    
    /// Last failure time
    last_failure: Option<Instant>,
    
    /// Current active connections
    active_connections: u32,
}

impl Default for TransportMetrics {
    fn default() -> Self {
        Self {
            attempts: 0,
            successes: 0,
            avg_connection_time: Duration::from_secs(0),
            last_failure: None,
            active_connections: 0,
        }
    }
}

/// Per-peer transport history.
#[derive(Debug, Clone)]
struct PeerTransportHistory {
    /// Successful transports for this peer
    successful_transports: Vec<TransportType>,
    
    /// Failed transports for this peer
    failed_transports: Vec<TransportType>,
    
    /// Last successful transport
    last_successful: Option<TransportType>,
    
    /// Last connection time
    last_connection: Option<Instant>,
}

/// Transport selection strategy.
#[derive(Clone)]
pub enum SelectionStrategy {
    /// Always use the highest priority transport
    Priority,
    
    /// Use historical success rates
    Adaptive,
    
    /// Round-robin between available transports
    RoundRobin,
    
    /// Use specific transport based on requirements
    RequirementsBased,
    
    /// Custom selection function
    Custom(Arc<dyn Fn(&TransportRequirements, &HashMap<TransportType, TransportScore>) -> Option<TransportType> + Send + Sync>),
}

impl std::fmt::Debug for SelectionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Priority => write!(f, "Priority"),
            Self::Adaptive => write!(f, "Adaptive"),
            Self::RoundRobin => write!(f, "RoundRobin"),
            Self::RequirementsBased => write!(f, "RequirementsBased"),
            Self::Custom(_) => write!(f, "Custom(<function>)"),
        }
    }
}

impl PartialEq for SelectionStrategy {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Priority, Self::Priority) => true,
            (Self::Adaptive, Self::Adaptive) => true,
            (Self::RoundRobin, Self::RoundRobin) => true,
            (Self::RequirementsBased, Self::RequirementsBased) => true,
            (Self::Custom(_), Self::Custom(_)) => false, // Functions can't be compared
            _ => false,
        }
    }
}

/// Transport score for selection algorithm.
#[derive(Debug, Clone)]
struct TransportScore {
    /// Base priority
    priority: TransportPriority,
    
    /// Success rate (0.0 - 1.0)
    success_rate: f64,
    
    /// Average connection time
    avg_connection_time: Duration,
    
    /// Feature match score (0.0 - 1.0)
    feature_score: f64,
    
    /// Current load (0.0 - 1.0)
    load_factor: f64,
    
    /// Composite score
    total_score: f64,
}

impl TransportManager {
    /// Creates a new transport manager.
    pub fn new(default_transport: TransportType) -> Self {
        Self {
            transports: Arc::new(RwLock::new(HashMap::new())),
            transport_metrics: Arc::new(DashMap::new()),
            peer_history: Arc::new(DashMap::new()),
            default_transport,
            selection_strategy: SelectionStrategy::Adaptive,
            observability: None,
        }
    }
    
    /// Creates a new transport manager with observability.
    pub fn with_observability(
        default_transport: TransportType,
        observability: Arc<UnifiedObservability>,
    ) -> Self {
        let mut manager = Self::new(default_transport);
        manager.observability = Some(observability);
        manager
    }
    
    /// Initialize the transport manager and all transports.
    pub async fn initialize(&self) -> Result<()> {
        // Initialize default transports based on configuration
        // This is a placeholder implementation
        info!("Transport manager initialized with default transport: {:?}", self.default_transport);
        Ok(())
    }
    
    /// Shutdown the transport manager and all transports.
    pub async fn shutdown(&self) -> Result<()> {
        // Shutdown all active transports
        // This is a placeholder implementation
        info!("Transport manager shutting down");
        Ok(())
    }
    
    /// Sets the transport selection strategy.
    pub fn set_selection_strategy(&mut self, strategy: SelectionStrategy) {
        self.selection_strategy = strategy;
    }
    
    /// Registers a transport.
    pub async fn register(&self, transport_type: TransportType, transport: Arc<dyn Transport + Send + Sync>) {
        // Wrap with observability if available
        let transport = if let Some(obs) = &self.observability {
            Arc::new(ObservableTransport::new(
                transport,
                obs.clone(),
                transport_type,
            )) as Arc<dyn Transport + Send + Sync>
        } else {
            transport
        };
        
        let mut transports = self.transports.write().await;
        info!("Registered transport: {:?}", transport_type);
        transports.insert(transport_type, transport);
        
        // Initialize metrics
        self.transport_metrics.insert(transport_type, TransportMetrics::default());
    }
    
    /// Connects to a peer using the best available transport.
    pub async fn connect(&self, peer: &PeerInfo) -> Result<Connection> {
        let requirements = self.infer_requirements(peer);
        let transport_type = self.select_transport(&requirements, peer).await?;
        
        let start_time = Instant::now();
        
        // Update metrics
        if let Some(mut metrics) = self.transport_metrics.get_mut(&transport_type) {
            metrics.attempts += 1;
        }
        
        // Attempt connection
        let transports = self.transports.read().await;
        let transport = transports.get(&transport_type)
            .ok_or_else(|| NetworkError::Transport(
                TransportError::NotAvailable(format!("Transport {:?} not available", transport_type))
            ))?;
        
        debug!("Connecting to {} using {:?} transport", peer.id, transport_type);
        
        match transport.connect(peer).await {
            Ok(connection) => {
                let connection_time = start_time.elapsed();
                
                // Update success metrics
                if let Some(mut metrics) = self.transport_metrics.get_mut(&transport_type) {
                    metrics.successes += 1;
                    metrics.active_connections += 1;
                    
                    // Update average connection time
                    let total_time = metrics.avg_connection_time.as_millis() as u64 * (metrics.successes - 1)
                        + connection_time.as_millis() as u64;
                    metrics.avg_connection_time = Duration::from_millis(total_time / metrics.successes);
                }
                
                // Update peer history
                self.update_peer_history(peer, transport_type, true);
                
                Ok(connection)
            }
            Err(e) => {
                // Update failure metrics
                if let Some(mut metrics) = self.transport_metrics.get_mut(&transport_type) {
                    metrics.last_failure = Some(Instant::now());
                }
                
                // Update peer history
                self.update_peer_history(peer, transport_type, false);
                
                // Try fallback transports
                if let Ok(fallback_connection) = self.try_fallback_transports(peer, transport_type).await {
                    Ok(fallback_connection)
                } else {
                    Err(e)
                }
            }
        }
    }
    
    /// Selects the best transport based on strategy and requirements.
    async fn select_transport(
        &self,
        requirements: &TransportRequirements,
        peer: &PeerInfo,
    ) -> Result<TransportType> {
        let transports = self.transports.read().await;
        
        // Check peer history first
        if let Some(history) = self.peer_history.get(&peer.id.to_string()) {
            if let Some(last_successful) = history.last_successful {
                if transports.contains_key(&last_successful) {
                    debug!("Using previously successful transport {:?} for peer {}", last_successful, peer.id);
                    return Ok(last_successful);
                }
            }
        }
        
        // Calculate scores for each transport
        let mut scores: HashMap<TransportType, TransportScore> = HashMap::new();
        
        for (transport_type, transport) in transports.iter() {
            let score = self.calculate_transport_score(
                *transport_type,
                transport.as_ref(),
                requirements,
            );
            scores.insert(*transport_type, score);
        }
        
        // Select based on strategy
        match &self.selection_strategy {
            SelectionStrategy::Priority => {
                scores.iter()
                    .max_by_key(|(_, score)| score.priority)
                    .map(|(transport_type, _)| *transport_type)
                    .ok_or_else(|| NetworkError::Transport(
                        TransportError::NotAvailable("No transports available".to_string())
                    ))
            }
            SelectionStrategy::Adaptive => {
                scores.iter()
                    .max_by(|(_, a), (_, b)| {
                        a.total_score.partial_cmp(&b.total_score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(transport_type, _)| *transport_type)
                    .ok_or_else(|| NetworkError::Transport(
                        TransportError::NotAvailable("No transports available".to_string())
                    ))
            }
            SelectionStrategy::RoundRobin => {
                // Simple round-robin based on attempts
                let min_attempts = self.transport_metrics.iter()
                    .filter(|entry| transports.contains_key(entry.key()))
                    .min_by_key(|entry| entry.value().attempts)
                    .map(|entry| *entry.key());
                
                min_attempts.ok_or_else(|| NetworkError::Transport(
                    TransportError::NotAvailable("No transports available".to_string())
                ))
            }
            SelectionStrategy::RequirementsBased => {
                // Select transport that best matches requirements
                scores.iter()
                    .filter(|(_, score)| score.feature_score > 0.8)
                    .max_by(|(_, a), (_, b)| {
                        a.total_score.partial_cmp(&b.total_score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(transport_type, _)| *transport_type)
                    .ok_or_else(|| NetworkError::Transport(
                        TransportError::NotAvailable("No transport matches requirements".to_string())
                    ))
            }
            SelectionStrategy::Custom(selector) => {
                selector(requirements, &scores)
                    .ok_or_else(|| NetworkError::Transport(
                        TransportError::NotAvailable("Custom selector returned no transport".to_string())
                    ))
            }
        }
    }
    
    /// Calculates a score for a transport.
    fn calculate_transport_score(
        &self,
        transport_type: TransportType,
        transport: &dyn Transport,
        requirements: &TransportRequirements,
    ) -> TransportScore {
        let metrics = self.transport_metrics.get(&transport_type)
            .map(|m| m.clone())
            .unwrap_or_default();
        
        // Calculate success rate
        let success_rate = if metrics.attempts > 0 {
            metrics.successes as f64 / metrics.attempts as f64
        } else {
            0.5 // Default for untested transports
        };
        
        // Calculate feature match score
        let mut feature_matches = 0;
        let mut total_features = 0;
        
        // Check required features
        for feature in &requirements.required_features {
            total_features += 2; // Weight required features more heavily
            if transport.supports_feature(*feature) {
                feature_matches += 2;
            }
        }
        
        // Check preferred features
        for feature in &requirements.preferred_features {
            total_features += 1;
            if transport.supports_feature(*feature) {
                feature_matches += 1;
            }
        }
        
        // Check anonymity requirement
        if requirements.require_anonymity {
            total_features += 2;
            if transport.supports_feature(TransportFeature::Anonymity) {
                feature_matches += 2;
            }
        }
        
        // Check post-quantum requirement
        if requirements.require_post_quantum {
            total_features += 2;
            if transport.supports_feature(TransportFeature::PostQuantum) {
                feature_matches += 2;
            }
        }
        
        let feature_score = if total_features > 0 {
            feature_matches as f64 / total_features as f64
        } else {
            1.0
        };
        
        // Calculate load factor (inverse of utilization)
        let load_factor = 1.0 - (metrics.active_connections as f64 / 100.0).min(1.0);
        
        // Calculate total score
        let priority_weight = match transport.priority() {
            TransportPriority::Required => 1.0,
            TransportPriority::Preferred => 0.8,
            TransportPriority::High => 0.6,
            TransportPriority::Medium => 0.4,
            TransportPriority::Low => 0.2,
            TransportPriority::Fallback => 0.1,
        };
        
        let total_score = priority_weight * 0.3
            + success_rate * 0.3
            + feature_score * 0.2
            + load_factor * 0.1
            + (1.0 / (1.0 + metrics.avg_connection_time.as_secs_f64())) * 0.1;
        
        TransportScore {
            priority: transport.priority(),
            success_rate,
            avg_connection_time: metrics.avg_connection_time,
            feature_score,
            load_factor,
            total_score,
        }
    }
    
    /// Infers transport requirements from peer information.
    fn infer_requirements(&self, peer: &PeerInfo) -> TransportRequirements {
        let mut requirements = TransportRequirements::default();
        
        // Check peer capabilities
        for capability in &peer.capabilities {
            match capability.as_str() {
                "webrtc" => requirements.preferred_features.push(TransportFeature::NATTraversal),
                "quic" => requirements.preferred_features.push(TransportFeature::ZeroRTT),
                "multistream" => requirements.preferred_features.push(TransportFeature::Multistream),
                "low-latency" => requirements.max_latency_ms = Some(50),
                "high-bandwidth" => requirements.min_bandwidth_mbps = Some(10.0), // 10 Mbps
                "anonymous" => requirements.require_anonymity = true,
                "post-quantum" => requirements.require_post_quantum = true,
                _ => {}
            }
        }
        
        // Check address format
        if peer.address.starts_with("ws://") || peer.address.starts_with("wss://") {
            // WebSocket address suggests browser peer
            requirements.preferred_features.push(TransportFeature::NATTraversal);
        }
        
        requirements
    }
    
    /// Updates peer transport history.
    fn update_peer_history(&self, peer: &PeerInfo, transport_type: TransportType, success: bool) {
        let mut history = self.peer_history.entry(peer.id.to_string())
            .or_insert_with(|| PeerTransportHistory {
                successful_transports: Vec::new(),
                failed_transports: Vec::new(),
                last_successful: None,
                last_connection: None,
            });
        
        if success {
            if !history.successful_transports.contains(&transport_type) {
                history.successful_transports.push(transport_type);
            }
            history.last_successful = Some(transport_type);
            history.last_connection = Some(Instant::now());
        } else {
            if !history.failed_transports.contains(&transport_type) {
                history.failed_transports.push(transport_type);
            }
        }
    }
    
    /// Tries fallback transports after primary failure.
    async fn try_fallback_transports(
        &self,
        peer: &PeerInfo,
        failed_transport: TransportType,
    ) -> Result<Connection> {
        let transports = self.transports.read().await;
        
        // Get all transports sorted by priority
        let mut fallback_transports: Vec<_> = transports.iter()
            .filter(|(t, _)| **t != failed_transport)
            .collect();
        
        fallback_transports.sort_by_key(|(_, transport)| std::cmp::Reverse(transport.priority()));
        
        for (transport_type, transport) in fallback_transports {
            warn!("Trying fallback transport {:?} for peer {}", transport_type, peer.id);
            
            if let Ok(connection) = transport.connect(peer).await {
                // Update metrics for successful fallback
                if let Some(mut metrics) = self.transport_metrics.get_mut(transport_type) {
                    metrics.attempts += 1;
                    metrics.successes += 1;
                    metrics.active_connections += 1;
                }
                
                self.update_peer_history(peer, *transport_type, true);
                return Ok(connection);
            }
        }
        
        Err(NetworkError::Transport(
            TransportError::AllTransportsFailed("All transports failed".to_string())
        ))
    }
    
    /// Lists available transports.
    pub async fn list_transports(&self) -> Vec<TransportType> {
        self.transports.read().await.keys().cloned().collect()
    }
    
    /// Gets transport statistics.
    pub fn get_transport_stats(&self) -> HashMap<TransportType, TransportStats> {
        self.transport_metrics.iter()
            .map(|entry| {
                let transport_type = *entry.key();
                let metrics = entry.value();
                
                let stats = TransportStats {
                    attempts: metrics.attempts,
                    successes: metrics.successes,
                    success_rate: if metrics.attempts > 0 {
                        metrics.successes as f64 / metrics.attempts as f64
                    } else {
                        0.0
                    },
                    avg_connection_time: metrics.avg_connection_time,
                    active_connections: metrics.active_connections,
                    last_failure: metrics.last_failure,
                };
                
                (transport_type, stats)
            })
            .collect()
    }
}

/// Public transport statistics.
#[derive(Debug, Clone)]
pub struct TransportStats {
    pub attempts: u64,
    pub successes: u64,
    pub success_rate: f64,
    pub avg_connection_time: Duration,
    pub active_connections: u32,
    pub last_failure: Option<Instant>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::memory::MemoryTransport;
    
    #[tokio::test]
    async fn test_transport_registration() {
        let manager = TransportManager::new(TransportType::Memory);
        let transport = Arc::new(MemoryTransport::new());
        
        manager.register(TransportType::Memory, transport).await;
        
        let transports = manager.list_transports().await;
        assert!(transports.contains(&TransportType::Memory));
    }
    
    #[test]
    fn test_transport_score_calculation() {
        let manager = TransportManager::new(TransportType::Tcp);
        let requirements = TransportRequirements {
            required_features: vec![TransportFeature::ZeroRTT],
            preferred_features: vec![TransportFeature::Multistream],
            ..Default::default()
        };
        
        // Test score calculation logic
        let metrics = TransportMetrics {
            attempts: 100,
            successes: 90,
            avg_connection_time: Duration::from_millis(50),
            last_failure: None,
            active_connections: 10,
        };
        
        assert!(metrics.successes as f64 / metrics.attempts as f64 > 0.8);
    }
}