//! Monitors API - direct port of Java Serventis Monitors interface
//!
//! The Monitors API is used to monitor the operational condition of a subject.
//! It emits assessments of a subject's operational condition along with statistical certainty.

use crate::{async_trait, Arc, Composer, Pipe, Subject, Substrate};
use serde::{Deserialize, Serialize};
use std::fmt;
use synapsed_substrates::types::SubstratesResult;

/// The Monitors interface - entry point into the Serventis Monitors API
/// Direct port of Java Serventis Monitors interface
pub trait Monitors: Composer<Arc<dyn Monitor>, Box<dyn Status>> + Send + Sync {}

/// Monitor interface for emitting observer assessments of operational condition
/// Direct port of Java Serventis Monitor interface
#[async_trait]
pub trait Monitor: Pipe<Box<dyn Status>> + Send + Sync {
    /// Emit a Status with condition and confidence
    async fn status(&mut self, condition: Condition, confidence: Confidence) -> SubstratesResult<()> {
        let status = BasicStatus::new(condition, confidence);
        self.emit(Box::new(status)).await
    }
}

/// Status interface representing assessed operational condition of a subject
/// Direct port of Java Serventis Status interface
pub trait Status: Send + Sync {
    /// Returns the operational condition classification
    fn condition(&self) -> Condition;
    
    /// Returns the statistical certainty of the classification
    fn confidence(&self) -> Confidence;
}

/// Condition enum representing operational condition of a subject
/// Direct port of Java Serventis Condition enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Condition {
    /// Subject is stabilizing towards reliable operation
    Converging,
    /// Subject is operating within expected parameters
    Stable,
    /// Subject is destabilizing with increasing variations
    Diverging,
    /// Subject exhibits unpredictable behavior
    Erratic,
    /// Subject is partially operational with reduced performance
    Degraded,
    /// Subject is unreliable with predominantly failed operations
    Defective,
    /// Subject is entirely non-operational
    Down,
}

impl Condition {
    /// Check if this condition indicates a healthy state
    pub fn is_healthy(&self) -> bool {
        matches!(self, Condition::Converging | Condition::Stable)
    }
    
    /// Check if this condition indicates an unhealthy state
    pub fn is_unhealthy(&self) -> bool {
        matches!(
            self,
            Condition::Degraded | Condition::Defective | Condition::Down
        )
    }
    
    /// Check if this condition indicates an unstable state
    pub fn is_unstable(&self) -> bool {
        matches!(self, Condition::Diverging | Condition::Erratic)
    }
    
    /// Get a severity score (0-6, where 6 is most severe)
    pub fn severity(&self) -> u8 {
        match self {
            Condition::Stable => 0,
            Condition::Converging => 1,
            Condition::Diverging => 2,
            Condition::Erratic => 3,
            Condition::Degraded => 4,
            Condition::Defective => 5,
            Condition::Down => 6,
        }
    }
}

/// Confidence enum representing statistical certainty of condition classification
/// Direct port of Java Serventis Confidence enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Confidence {
    /// Preliminary assessment based on initial patterns
    Tentative,
    /// Established assessment with strong evidence
    Measured,
    /// Definitive assessment with unambiguous evidence
    Confirmed,
}

impl Confidence {
    /// Get a confidence score (0-2, where 2 is highest confidence)
    pub fn score(&self) -> u8 {
        match self {
            Confidence::Tentative => 0,
            Confidence::Measured => 1,
            Confidence::Confirmed => 2,
        }
    }
    
    /// Check if this confidence level is high enough for action
    pub fn is_actionable(&self) -> bool {
        matches!(self, Confidence::Measured | Confidence::Confirmed)
    }
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let desc = match self {
            Condition::Converging => "Converging - stabilizing towards reliable operation",
            Condition::Stable => "Stable - operating within expected parameters",
            Condition::Diverging => "Diverging - destabilizing with increasing variations",
            Condition::Erratic => "Erratic - exhibiting unpredictable behavior",
            Condition::Degraded => "Degraded - partially operational with reduced performance",
            Condition::Defective => "Defective - unreliable with predominantly failed operations",
            Condition::Down => "Down - entirely non-operational",
        };
        write!(f, "{}", desc)
    }
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let desc = match self {
            Confidence::Tentative => "Tentative - preliminary assessment",
            Confidence::Measured => "Measured - established assessment with strong evidence",
            Confidence::Confirmed => "Confirmed - definitive assessment with unambiguous evidence",
        };
        write!(f, "{}", desc)
    }
}

/// Basic implementation of Status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasicStatus {
    condition: Condition,
    confidence: Confidence,
}

impl BasicStatus {
    pub fn new(condition: Condition, confidence: Confidence) -> Self {
        Self {
            condition,
            confidence,
        }
    }
}

impl Status for BasicStatus {
    fn condition(&self) -> Condition {
        self.condition
    }
    
    fn confidence(&self) -> Confidence {
        self.confidence
    }
}

impl fmt::Display for BasicStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Status[condition={:?}, confidence={:?}]",
            self.condition, self.confidence
        )
    }
}

/// A wrapper type for status handlers that implements Debug
struct StatusHandler {
    handler: Arc<dyn Fn(Condition, Confidence) + Send + Sync>,
}

impl StatusHandler {
    fn new<F>(handler: F) -> Self
    where
        F: Fn(Condition, Confidence) + Send + Sync + 'static,
    {
        Self {
            handler: Arc::new(handler),
        }
    }
    
    fn handle(&self, condition: Condition, confidence: Confidence) {
        (self.handler)(condition, confidence);
    }
}

impl fmt::Debug for StatusHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StatusHandler")
            .field("handler", &"<function>")
            .finish()
    }
}

/// Basic implementation of Monitor
#[derive(Debug)]
pub struct BasicMonitor {
    subject: Subject,
    status_handler: Option<StatusHandler>,
}

impl BasicMonitor {
    pub fn new(subject: Subject) -> Self {
        Self {
            subject,
            status_handler: None,
        }
    }
    
    pub fn with_handler<F>(subject: Subject, handler: F) -> Self
    where
        F: Fn(Condition, Confidence) + Send + Sync + 'static,
    {
        Self {
            subject,
            status_handler: Some(StatusHandler::new(handler)),
        }
    }
    
    /// Emit a status assessment
    pub async fn assess(&mut self, condition: Condition, confidence: Confidence) -> SubstratesResult<()> {
        if let Some(handler) = &self.status_handler {
            handler.handle(condition, confidence);
        }
        self.status(condition, confidence).await
    }
}

impl Substrate for BasicMonitor {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

#[async_trait]
impl Pipe<Box<dyn Status>> for BasicMonitor {
    async fn emit(&mut self, emission: Box<dyn Status>) -> SubstratesResult<()> {
        if let Some(handler) = &self.status_handler {
            handler.handle(emission.condition(), emission.confidence());
        }
        Ok(())
    }
}

#[async_trait]
impl Monitor for BasicMonitor {}

/// Utility for creating assessments based on metrics
pub struct ConditionAssessor {
    thresholds: AssessmentThresholds,
}

#[derive(Debug, Clone)]
pub struct AssessmentThresholds {
    /// Error rate threshold for degraded condition (0.0-1.0)
    pub degraded_error_rate: f64,
    /// Error rate threshold for defective condition (0.0-1.0)
    pub defective_error_rate: f64,
    /// Response time multiplier for degraded condition
    pub degraded_latency_multiplier: f64,
    /// Response time multiplier for defective condition
    pub defective_latency_multiplier: f64,
    /// Minimum samples needed for measured confidence
    pub measured_sample_count: usize,
    /// Minimum samples needed for confirmed confidence
    pub confirmed_sample_count: usize,
}

impl Default for AssessmentThresholds {
    fn default() -> Self {
        Self {
            degraded_error_rate: 0.05,    // 5% error rate
            defective_error_rate: 0.25,   // 25% error rate
            degraded_latency_multiplier: 2.0,
            defective_latency_multiplier: 5.0,
            measured_sample_count: 100,
            confirmed_sample_count: 1000,
        }
    }
}

impl ConditionAssessor {
    pub fn new(thresholds: AssessmentThresholds) -> Self {
        Self { thresholds }
    }
    
    pub fn default() -> Self {
        Self::new(AssessmentThresholds::default())
    }
    
    /// Assess condition based on error rate and sample count
    pub fn assess_by_error_rate(&self, error_rate: f64, sample_count: usize) -> (Condition, Confidence) {
        let condition = if error_rate >= self.thresholds.defective_error_rate {
            Condition::Defective
        } else if error_rate >= self.thresholds.degraded_error_rate {
            Condition::Degraded
        } else {
            Condition::Stable
        };
        
        let confidence = self.assess_confidence(sample_count);
        
        (condition, confidence)
    }
    
    /// Assess condition based on latency multiplier
    pub fn assess_by_latency(&self, latency_multiplier: f64, sample_count: usize) -> (Condition, Confidence) {
        let condition = if latency_multiplier >= self.thresholds.defective_latency_multiplier {
            Condition::Defective
        } else if latency_multiplier >= self.thresholds.degraded_latency_multiplier {
            Condition::Degraded
        } else {
            Condition::Stable
        };
        
        let confidence = self.assess_confidence(sample_count);
        
        (condition, confidence)
    }
    
    /// Assess confidence based on sample count
    pub fn assess_confidence(&self, sample_count: usize) -> Confidence {
        if sample_count >= self.thresholds.confirmed_sample_count {
            Confidence::Confirmed
        } else if sample_count >= self.thresholds.measured_sample_count {
            Confidence::Measured
        } else {
            Confidence::Tentative
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use synapsed_substrates::types::{Name, SubjectType};
    
    #[tokio::test]
    async fn test_monitor_status() {
        let subject = Subject::new(Name::from_part("test-monitor"), SubjectType::Source);
        let mut monitor = BasicMonitor::new(subject);
        
        // Test status emission
        monitor.status(Condition::Stable, Confidence::Measured).await.unwrap();
        monitor.assess(Condition::Degraded, Confidence::Tentative).await.unwrap();
    }
    
    #[test]
    fn test_condition_properties() {
        assert!(Condition::Stable.is_healthy());
        assert!(Condition::Converging.is_healthy());
        assert!(!Condition::Degraded.is_healthy());
        
        assert!(Condition::Degraded.is_unhealthy());
        assert!(Condition::Defective.is_unhealthy());
        assert!(Condition::Down.is_unhealthy());
        
        assert!(Condition::Diverging.is_unstable());
        assert!(Condition::Erratic.is_unstable());
        
        assert_eq!(Condition::Stable.severity(), 0);
        assert_eq!(Condition::Down.severity(), 6);
    }
    
    #[test]
    fn test_confidence_properties() {
        assert!(!Confidence::Tentative.is_actionable());
        assert!(Confidence::Measured.is_actionable());
        assert!(Confidence::Confirmed.is_actionable());
        
        assert_eq!(Confidence::Tentative.score(), 0);
        assert_eq!(Confidence::Confirmed.score(), 2);
    }
    
    #[test]
    fn test_condition_assessor() {
        let assessor = ConditionAssessor::default();
        
        // Test error rate assessment
        let (condition, confidence) = assessor.assess_by_error_rate(0.02, 500);
        assert_eq!(condition, Condition::Stable);
        assert_eq!(confidence, Confidence::Measured);
        
        let (condition, confidence) = assessor.assess_by_error_rate(0.10, 1500);
        assert_eq!(condition, Condition::Degraded);
        assert_eq!(confidence, Confidence::Confirmed);
        
        let (condition, confidence) = assessor.assess_by_error_rate(0.30, 50);
        assert_eq!(condition, Condition::Defective);
        assert_eq!(confidence, Confidence::Tentative);
        
        // Test latency assessment
        let (condition, confidence) = assessor.assess_by_latency(1.5, 200);
        assert_eq!(condition, Condition::Stable);
        assert_eq!(confidence, Confidence::Measured);
        
        let (condition, confidence) = assessor.assess_by_latency(3.0, 100);
        assert_eq!(condition, Condition::Degraded);
        assert_eq!(confidence, Confidence::Measured);
    }
    
    #[test]
    fn test_basic_status() {
        let status = BasicStatus::new(Condition::Stable, Confidence::Confirmed);
        assert_eq!(status.condition(), Condition::Stable);
        assert_eq!(status.confidence(), Confidence::Confirmed);
        
        let status_str = status.to_string();
        assert!(status_str.contains("Stable"));
        assert!(status_str.contains("Confirmed"));
    }
}