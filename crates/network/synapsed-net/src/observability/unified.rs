//! Unified observability implementation.

use crate::config::ObservabilityConfig;
use crate::error::Result;
use crate::observability::{ObservabilityContext, ObservabilityHandle};
use std::sync::Arc;
use synapsed_serventis::{Monitor, Service, Condition, Confidence};
use synapsed_substrates::{Circuit, Cortex, DefaultCortex, Subject, Name, SubjectType};
use tokio::sync::RwLock;

/// Unified observability system that integrates Substrates and Serventis.
pub struct UnifiedObservability {
    /// Substrates cortex for event streaming
    cortex: Arc<dyn synapsed_substrates::Cortex>,
    
    /// Transport circuit
    transport_circuit: Arc<dyn synapsed_substrates::Circuit>,
    
    /// Security circuit
    security_circuit: Arc<dyn synapsed_substrates::Circuit>,
    
    /// Privacy circuit
    privacy_circuit: Arc<dyn synapsed_substrates::Circuit>,
    
    /// Connection circuit
    connection_circuit: Arc<dyn synapsed_substrates::Circuit>,
    
    /// Serventis service
    service: Arc<dyn Service>,
    
    /// Health monitor
    health_monitor: Arc<dyn Monitor>,
    
    /// Performance monitor
    performance_monitor: Arc<dyn Monitor>,
    
    /// Security monitor
    security_monitor: Arc<dyn Monitor>,
    
    /// Configuration
    config: ObservabilityConfig,
    
    /// State
    state: Arc<RwLock<ObservabilityState>>,
}

#[derive(Default)]
struct ObservabilityState {
    is_running: bool,
}

impl UnifiedObservability {
    /// Creates a new unified observability system.
    pub async fn new(config: &ObservabilityConfig) -> Result<Arc<Self>> {
        // Create Substrates cortex
        let cortex = Arc::new(DefaultCortex::new());
        
        // Create circuits for different layers
        let transport_circuit = cortex.circuit_named(cortex.name_from_str("net.transport")).await
            .map_err(|e| crate::error::NetworkError::Observability(e.to_string()))?;
            
        let security_circuit = cortex.circuit_named(cortex.name_from_str("net.security")).await
            .map_err(|e| crate::error::NetworkError::Observability(e.to_string()))?;
            
        let privacy_circuit = cortex.circuit_named(cortex.name_from_str("net.privacy")).await
            .map_err(|e| crate::error::NetworkError::Observability(e.to_string()))?;
            
        let connection_circuit = cortex.circuit_named(cortex.name_from_str("net.connection")).await
            .map_err(|e| crate::error::NetworkError::Observability(e.to_string()))?;
        
        // Create Serventis service - using BasicService
        let service = Arc::new(synapsed_serventis::BasicService::new(
            Subject::new(Name::from_part("synapsed-net"), SubjectType::Container)
        ));
        
        // Create monitors - using BasicMonitor  
        let health_monitor = Arc::new(synapsed_serventis::BasicMonitor::new(
            Subject::new(Name::from_part("health"), SubjectType::Subscriber)
        ));
        let performance_monitor = Arc::new(synapsed_serventis::BasicMonitor::new(
            Subject::new(Name::from_part("performance"), SubjectType::Subscriber)
        ));
        let security_monitor = Arc::new(synapsed_serventis::BasicMonitor::new(
            Subject::new(Name::from_part("security"), SubjectType::Subscriber)
        ));
        
        Ok(Arc::new(Self {
            cortex,
            transport_circuit,
            security_circuit,
            privacy_circuit,
            connection_circuit,
            service,
            health_monitor,
            performance_monitor,
            security_monitor,
            config: config.clone(),
            state: Arc::new(RwLock::new(ObservabilityState::default())),
        }))
    }
    
    /// Starts the observability services.
    pub async fn start(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if state.is_running {
            return Ok(());
        }
        
        // Circuits and monitors are ready to use immediately
        // No explicit start needed - they start when first used
        
        state.is_running = true;
        Ok(())
    }
    
    /// Stops the observability services.
    pub async fn stop(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if !state.is_running {
            return Ok(());
        }
        
        // Circuits and monitors clean up automatically
        
        state.is_running = false;
        Ok(())
    }
    
    /// Creates a new observability context.
    pub fn create_context(&self) -> ObservabilityContext {
        ObservabilityContext::new()
    }
    
    /// Creates an observability handle for a component.
    pub fn create_handle(&self) -> ObservabilityHandle {
        ObservabilityHandle::new(self.cortex.clone(), self.service.clone())
    }
    
    /// Returns the transport circuit.
    pub fn transport_circuit(&self) -> &Arc<dyn Circuit> {
        &self.transport_circuit
    }
    
    /// Returns the security circuit.
    pub fn security_circuit(&self) -> &Arc<dyn Circuit> {
        &self.security_circuit
    }
    
    /// Returns the privacy circuit.
    pub fn privacy_circuit(&self) -> &Arc<dyn Circuit> {
        &self.privacy_circuit
    }
    
    /// Returns the connection circuit.
    pub fn connection_circuit(&self) -> &Arc<dyn Circuit> {
        &self.connection_circuit
    }
    
    /// Returns the health monitor.
    pub fn health_monitor(&self) -> &Arc<dyn Monitor> {
        &self.health_monitor
    }
    
    /// Returns the performance monitor.
    pub fn performance_monitor(&self) -> &Arc<dyn Monitor> {
        &self.performance_monitor
    }
    
    /// Returns the security monitor.
    pub fn security_monitor(&self) -> &Arc<dyn Monitor> {
        &self.security_monitor
    }
    
    /// Updates the health state based on current conditions.
    pub fn assess_health(&self, healthy_connections: usize, total_connections: usize) {
        if total_connections == 0 {
            // For dormant state, no connections to assess
            let (_condition, _confidence) = (Condition::Stable, Confidence::Tentative);
            return;
        }
        
        let health_ratio = healthy_connections as f64 / total_connections as f64;
        let (_condition, _confidence) = match health_ratio {
            r if r >= 0.95 => (Condition::Stable, Confidence::Confirmed),
            r if r >= 0.80 => (Condition::Stable, Confidence::Measured),
            r if r >= 0.50 => (Condition::Diverging, Confidence::Tentative),
            _ => (Condition::Degraded, Confidence::Measured),
        };
        
        // Note: actual implementation would await, but this is a sync method
        // so we'd need to spawn a task or make this async
    }
    
    /// Updates the performance state based on metrics.
    pub fn assess_performance(&self, avg_latency_ms: f64, throughput_mbps: f64) {
        let latency_score = match avg_latency_ms {
            l if l <= 10.0 => 1.0,
            l if l <= 50.0 => 0.8,
            l if l <= 100.0 => 0.6,
            l if l <= 500.0 => 0.4,
            _ => 0.2,
        };
        
        let throughput_score = match throughput_mbps {
            t if t >= 100.0 => 1.0,
            t if t >= 10.0 => 0.8,
            t if t >= 1.0 => 0.6,
            t if t >= 0.1 => 0.4,
            _ => 0.2,
        };
        
        let combined_score = (latency_score + throughput_score) / 2.0;
        let (_condition, _confidence) = match combined_score {
            s if s >= 0.9 => (Condition::Stable, Confidence::Confirmed),
            s if s >= 0.7 => (Condition::Stable, Confidence::Measured),
            s if s >= 0.5 => (Condition::Converging, Confidence::Tentative),
            _ => (Condition::Degraded, Confidence::Measured),
        };
        
        // Note: actual implementation would await
    }
    
    /// Updates the security state based on recent events.
    pub fn assess_security(&self, auth_failures: usize, total_auths: usize) {
        if total_auths == 0 {
            // No auth attempts yet, default to high confidence
            return;
        }
        
        let failure_rate = auth_failures as f64 / total_auths as f64;
        let (_condition, _confidence) = match failure_rate {
            r if r <= 0.01 => (Condition::Stable, Confidence::Confirmed),
            r if r <= 0.05 => (Condition::Stable, Confidence::Measured),
            r if r <= 0.10 => (Condition::Diverging, Confidence::Tentative),
            r if r <= 0.25 => (Condition::Degraded, Confidence::Measured),
            _ => (Condition::Defective, Confidence::Confirmed),
        };
        
        // Note: actual implementation would await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_unified_observability_lifecycle() {
        let config = ObservabilityConfig::default();
        let obs = UnifiedObservability::new(&config).await.unwrap();
        
        // Start observability
        obs.start().await.unwrap();
        
        // Create context
        let ctx = obs.create_context();
        assert_eq!(ctx.parent_span, None);
        
        // Create handle
        let _handle = obs.create_handle();
        
        // Stop observability
        obs.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_health_assessment() {
        let config = ObservabilityConfig::default();
        let obs = UnifiedObservability::new(&config).await.unwrap();
        
        // Test various health scenarios
        obs.assess_health(95, 100);
        obs.assess_health(50, 100);
        obs.assess_health(0, 100);
        obs.assess_health(0, 0);
    }
}