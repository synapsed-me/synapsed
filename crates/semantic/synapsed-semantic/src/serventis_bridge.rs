//! Bridge between Serventis health monitoring and story health metrics
//!
//! This module maps service-level health indicators to narrative coherence,
//! detecting when stories are breaking down and need intervention.

use crate::{
    story::{Story, StoryOutcome, Narrative, NarrativeArc},
    trust::TrustScore,
    SemanticCoords,
};
// Simplified types for Serventis integration
// In production, these would come from synapsed_serventis

use serde::{Deserialize, Serialize};

/// Health probe types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeType {
    Liveness,
    Readiness,
    Performance,
    Custom(u32),
}

/// Probe result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub probe_type: ProbeType,
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

impl ProbeResult {
    pub fn is_healthy(&self) -> bool {
        self.success
    }
}

/// Serventis health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServentisHealth {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};

/// Maps Serventis health probes to story health
pub struct ServentisStoryHealth {
    /// Health thresholds for story coherence
    thresholds: HealthThresholds,
    
    /// Current story health metrics
    metrics: StoryHealthMetrics,
    
    /// Health history for trend analysis
    history: Vec<HealthSnapshot>,
}

impl ServentisStoryHealth {
    pub fn new() -> Self {
        Self {
            thresholds: HealthThresholds::default(),
            metrics: StoryHealthMetrics::default(),
            history: Vec::new(),
        }
    }
    
    /// Evaluate story health from probe results
    pub fn evaluate_story_health(&mut self, story: &Story, probes: &[ProbeResult]) -> StoryHealth {
        let coherence = self.calculate_coherence(story, probes);
        let trust_factor = self.calculate_trust_factor(story);
        let completion_rate = self.calculate_completion_rate(story);
        let error_rate = self.calculate_error_rate(probes);
        
        // Update metrics
        self.metrics.coherence_score = coherence;
        self.metrics.trust_score = trust_factor;
        self.metrics.completion_rate = completion_rate;
        self.metrics.error_rate = error_rate;
        
        // Record snapshot
        self.history.push(HealthSnapshot {
            timestamp: Utc::now(),
            metrics: self.metrics.clone(),
        });
        
        // Determine overall health
        self.determine_health_status()
    }
    
    /// Calculate story coherence from probes
    fn calculate_coherence(&self, story: &Story, probes: &[ProbeResult]) -> f64 {
        let mut coherence = 1.0;
        
        // Check if promises align with execution
        let promise_count = story.promises.len();
        let execution_count = story.execution.len();
        
        if promise_count > 0 {
            let promise_fulfillment = execution_count as f64 / promise_count as f64;
            coherence *= promise_fulfillment.min(1.0);
        }
        
        // Check probe health impacts coherence
        for probe in probes {
            match probe.probe_type {
                ProbeType::Liveness => {
                    if !probe.is_healthy() {
                        coherence *= 0.5; // Major impact if not alive
                    }
                }
                ProbeType::Readiness => {
                    if !probe.is_healthy() {
                        coherence *= 0.8; // Moderate impact if not ready
                    }
                }
                ProbeType::Performance => {
                    if let Some(latency) = probe.latency_ms {
                        if latency > self.thresholds.max_latency_ms {
                            coherence *= 0.9; // Minor impact for slow performance
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Check semantic drift
        if let Some(drift) = self.calculate_semantic_drift(&story.path.positions) {
            coherence *= (1.0 - drift).max(0.3);
        }
        
        coherence
    }
    
    /// Calculate trust factor from story
    fn calculate_trust_factor(&self, story: &Story) -> f64 {
        if story.trust_updates.is_empty() {
            return 0.5; // Neutral trust
        }
        
        let total_delta: f64 = story.trust_updates.iter()
            .map(|update| update.delta)
            .sum();
        
        let average_delta = total_delta / story.trust_updates.len() as f64;
        
        // Convert delta to trust score (0-1)
        (0.5 + average_delta).clamp(0.0, 1.0)
    }
    
    /// Calculate completion rate
    fn calculate_completion_rate(&self, story: &Story) -> f64 {
        match &story.verification {
            StoryOutcome::Success { .. } => 1.0,
            StoryOutcome::Partial { completed, failed, .. } => {
                let total = completed.len() + failed.len();
                if total > 0 {
                    completed.len() as f64 / total as f64
                } else {
                    0.0
                }
            }
            StoryOutcome::Failure { .. } => 0.0,
            StoryOutcome::Pending => 0.5, // In progress
        }
    }
    
    /// Calculate error rate from probes
    fn calculate_error_rate(&self, probes: &[ProbeResult]) -> f64 {
        if probes.is_empty() {
            return 0.0;
        }
        
        let error_count = probes.iter()
            .filter(|p| p.error.is_some())
            .count();
        
        error_count as f64 / probes.len() as f64
    }
    
    /// Calculate semantic drift in story path
    fn calculate_semantic_drift(&self, positions: &[SemanticCoords]) -> Option<f64> {
        if positions.len() < 2 {
            return None;
        }
        
        let mut total_drift = 0.0;
        let mut expected_position = positions[0];
        
        for (i, actual_position) in positions.iter().enumerate().skip(1) {
            // Expected position moves gradually
            expected_position.intent += 0.1;
            expected_position.expression += 0.1;
            
            // Calculate drift as distance from expected
            let drift = expected_position.distance_to(actual_position);
            total_drift += drift;
            
            // Update expected for next iteration
            expected_position = *actual_position;
        }
        
        Some(total_drift / (positions.len() - 1) as f64)
    }
    
    /// Determine overall health status
    fn determine_health_status(&self) -> StoryHealth {
        let metrics = &self.metrics;
        
        if metrics.coherence_score < self.thresholds.critical_coherence {
            StoryHealth::Critical {
                reason: "Story coherence critically low".to_string(),
                recovery_action: RecoveryAction::RestartStory,
            }
        } else if metrics.error_rate > self.thresholds.max_error_rate {
            StoryHealth::Unhealthy {
                reason: "High error rate detected".to_string(),
                recovery_action: RecoveryAction::RetryFailedSteps,
            }
        } else if metrics.coherence_score < self.thresholds.min_coherence {
            StoryHealth::Degraded {
                reason: "Story coherence below threshold".to_string(),
                recovery_action: RecoveryAction::RealignNarrative,
            }
        } else {
            StoryHealth::Healthy {
                confidence: metrics.coherence_score * metrics.trust_score,
            }
        }
    }
    
    /// Analyze narrative health
    pub fn analyze_narrative(&self, narrative: &Narrative) -> NarrativeHealth {
        let mut story_healths = Vec::new();
        
        for story in &narrative.stories {
            let health = self.evaluate_story_from_outcome(&story.verification);
            story_healths.push(health);
        }
        
        let healthy_count = story_healths.iter()
            .filter(|h| matches!(h, StoryHealth::Healthy { .. }))
            .count();
        
        let health_ratio = healthy_count as f64 / story_healths.len() as f64;
        
        NarrativeHealth {
            arc_integrity: self.check_arc_integrity(&narrative.arc),
            story_health_ratio: health_ratio,
            protagonist_trust: self.calculate_protagonist_trust(narrative),
            overall_status: if health_ratio > 0.8 {
                "Narrative progressing well".to_string()
            } else if health_ratio > 0.5 {
                "Narrative experiencing friction".to_string()
            } else {
                "Narrative breaking down".to_string()
            },
        }
    }
    
    /// Evaluate story health from outcome alone
    fn evaluate_story_from_outcome(&self, outcome: &StoryOutcome) -> StoryHealth {
        match outcome {
            StoryOutcome::Success { confidence, .. } => {
                StoryHealth::Healthy { confidence: *confidence }
            }
            StoryOutcome::Partial { confidence, .. } => {
                StoryHealth::Degraded {
                    reason: "Partial success".to_string(),
                    recovery_action: RecoveryAction::CompletePartialTasks,
                }
            }
            StoryOutcome::Failure { reason, .. } => {
                StoryHealth::Unhealthy {
                    reason: reason.clone(),
                    recovery_action: RecoveryAction::RetryFailedSteps,
                }
            }
            StoryOutcome::Pending => {
                StoryHealth::Unknown
            }
        }
    }
    
    /// Check narrative arc integrity
    fn check_arc_integrity(&self, arc: &NarrativeArc) -> f64 {
        match arc {
            NarrativeArc::Linear => 1.0,        // Simple, stable
            NarrativeArc::Cyclical => 0.9,      // Repeating, predictable
            NarrativeArc::Branching => 0.7,     // Complex but manageable
            NarrativeArc::Converging => 0.8,    // Coming together
            NarrativeArc::Emergent => 0.5,      // Unpredictable
        }
    }
    
    /// Calculate protagonist trust scores
    fn calculate_protagonist_trust(&self, narrative: &Narrative) -> f64 {
        // Average trust across all protagonists
        if narrative.protagonists.is_empty() {
            return 0.5;
        }
        
        // This would integrate with trust network
        // For now, return a reasonable default
        0.7
    }
}

/// Story health status
#[derive(Debug, Clone)]
pub enum StoryHealth {
    /// Story is progressing well
    Healthy { confidence: f64 },
    
    /// Story has minor issues
    Degraded { reason: String, recovery_action: RecoveryAction },
    
    /// Story has major issues
    Unhealthy { reason: String, recovery_action: RecoveryAction },
    
    /// Story is failing critically
    Critical { reason: String, recovery_action: RecoveryAction },
    
    /// Health unknown (no data)
    Unknown,
}

/// Recovery actions for unhealthy stories
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Continue monitoring
    Monitor,
    
    /// Retry failed steps
    RetryFailedSteps,
    
    /// Complete partial tasks
    CompletePartialTasks,
    
    /// Realign narrative path
    RealignNarrative,
    
    /// Restart the story
    RestartStory,
    
    /// Escalate to human
    EscalateToHuman,
}

/// Story health metrics
#[derive(Debug, Clone)]
pub struct StoryHealthMetrics {
    /// Narrative coherence (0-1)
    pub coherence_score: f64,
    
    /// Trust score (0-1)
    pub trust_score: f64,
    
    /// Task completion rate (0-1)
    pub completion_rate: f64,
    
    /// Error rate (0-1)
    pub error_rate: f64,
    
    /// Semantic drift
    pub semantic_drift: Option<f64>,
    
    /// Performance score
    pub performance_score: f64,
}

impl Default for StoryHealthMetrics {
    fn default() -> Self {
        Self {
            coherence_score: 1.0,
            trust_score: 0.5,
            completion_rate: 0.0,
            error_rate: 0.0,
            semantic_drift: None,
            performance_score: 1.0,
        }
    }
}

/// Health thresholds
#[derive(Debug, Clone)]
pub struct HealthThresholds {
    /// Minimum acceptable coherence
    pub min_coherence: f64,
    
    /// Critical coherence level
    pub critical_coherence: f64,
    
    /// Maximum acceptable error rate
    pub max_error_rate: f64,
    
    /// Maximum acceptable latency
    pub max_latency_ms: u64,
    
    /// Maximum semantic drift
    pub max_drift: f64,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            min_coherence: 0.7,
            critical_coherence: 0.4,
            max_error_rate: 0.2,
            max_latency_ms: 1000,
            max_drift: 0.3,
        }
    }
}

/// Health snapshot for history
#[derive(Debug, Clone)]
pub struct HealthSnapshot {
    pub timestamp: DateTime<Utc>,
    pub metrics: StoryHealthMetrics,
}

/// Narrative health analysis
#[derive(Debug, Clone)]
pub struct NarrativeHealth {
    /// Arc integrity (0-1)
    pub arc_integrity: f64,
    
    /// Ratio of healthy stories
    pub story_health_ratio: f64,
    
    /// Average protagonist trust
    pub protagonist_trust: f64,
    
    /// Overall status description
    pub overall_status: String,
}

/// Convert Serventis health to story health
impl From<ServentisHealth> for StoryHealth {
    fn from(health: ServentisHealth) -> Self {
        match health {
            ServentisHealth::Healthy => StoryHealth::Healthy { confidence: 1.0 },
            ServentisHealth::Degraded(reason) => StoryHealth::Degraded {
                reason,
                recovery_action: RecoveryAction::Monitor,
            },
            ServentisHealth::Unhealthy(reason) => StoryHealth::Unhealthy {
                reason,
                recovery_action: RecoveryAction::RetryFailedSteps,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_story_health_evaluation() {
        let mut health = ServentisStoryHealth::new();
        let story = Story::begin(crate::traits::Intent::new("test"));
        let probes = vec![];
        
        let status = health.evaluate_story_health(&story, &probes);
        assert!(matches!(status, StoryHealth::Healthy { .. } | StoryHealth::Unknown));
    }
}