//! Pattern detection for identifying meaningful sequences in events

use crate::collector::CollectedEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pattern detector for identifying meaningful event sequences
pub struct PatternDetector {
    patterns: Vec<Box<dyn Pattern>>,
    history: Vec<CollectedEvent>,
    max_history: usize,
}

impl PatternDetector {
    pub fn new() -> Self {
        Self {
            patterns: Self::create_patterns(),
            history: Vec::new(),
            max_history: 1000,
        }
    }
    
    /// Add event to history and detect patterns
    pub fn process_event(&mut self, event: CollectedEvent) -> Option<DetectedPattern> {
        self.history.push(event.clone());
        
        // Keep history bounded
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
        
        // Check all patterns
        for pattern in &self.patterns {
            if let Some(detected) = pattern.detect(&self.history) {
                return Some(detected);
            }
        }
        
        None
    }
    
    fn create_patterns() -> Vec<Box<dyn Pattern>> {
        vec![
            Box::new(StartStopPattern),
            Box::new(ErrorRecoveryPattern),
            Box::new(ProgressionPattern),
            Box::new(PerformanceDegradationPattern),
        ]
    }
}

/// Detected pattern with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub confidence: f32,
    pub description: String,
    pub events: Vec<String>,
}

/// Types of patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    TaskLifecycle,
    ErrorRecovery,
    NormalProgression,
    PerformanceDegradation,
    ResourceContention,
    SecurityAnomaly,
}

/// Trait for pattern detection
trait Pattern: Send + Sync {
    fn detect(&self, history: &[CollectedEvent]) -> Option<DetectedPattern>;
}

/// Detects start-stop patterns
struct StartStopPattern;
impl Pattern for StartStopPattern {
    fn detect(&self, history: &[CollectedEvent]) -> Option<DetectedPattern> {
        if history.len() < 2 {
            return None;
        }
        
        // Look for start signal followed by stop
        let recent = &history[history.len().saturating_sub(10)..];
        
        let has_start = recent.iter().any(|e| {
            matches!(e, CollectedEvent::ServentisSignal { signal, .. } if signal.contains("Start"))
        });
        
        let has_stop = recent.iter().any(|e| {
            matches!(e, CollectedEvent::ServentisSignal { signal, .. } if signal.contains("Stop"))
        });
        
        if has_start && has_stop {
            Some(DetectedPattern {
                pattern_type: PatternType::TaskLifecycle,
                confidence: 0.9,
                description: "Task completed its lifecycle".to_string(),
                events: vec!["start".to_string(), "stop".to_string()],
            })
        } else {
            None
        }
    }
}

/// Detects error recovery patterns
struct ErrorRecoveryPattern;
impl Pattern for ErrorRecoveryPattern {
    fn detect(&self, history: &[CollectedEvent]) -> Option<DetectedPattern> {
        let recent = &history[history.len().saturating_sub(5)..];
        
        let mut saw_error = false;
        let mut saw_recovery = false;
        
        for event in recent {
            if let CollectedEvent::ServentisSignal { signal, .. } = event {
                if signal.contains("Fail") {
                    saw_error = true;
                } else if saw_error && signal.contains("Success") {
                    saw_recovery = true;
                }
            }
        }
        
        if saw_error && saw_recovery {
            Some(DetectedPattern {
                pattern_type: PatternType::ErrorRecovery,
                confidence: 0.85,
                description: "System recovered from error".to_string(),
                events: vec!["error".to_string(), "recovery".to_string()],
            })
        } else {
            None
        }
    }
}

/// Detects normal progression
struct ProgressionPattern;
impl Pattern for ProgressionPattern {
    fn detect(&self, history: &[CollectedEvent]) -> Option<DetectedPattern> {
        let recent = &history[history.len().saturating_sub(20)..];
        
        // Count successful operations
        let success_count = recent.iter().filter(|e| {
            matches!(e, CollectedEvent::ServentisObservation { outcome, .. } if outcome == "Success")
        }).count();
        
        if success_count > 10 {
            Some(DetectedPattern {
                pattern_type: PatternType::NormalProgression,
                confidence: 0.95,
                description: "Consistent successful operations".to_string(),
                events: vec![format!("{} successes", success_count)],
            })
        } else {
            None
        }
    }
}

/// Detects performance degradation
struct PerformanceDegradationPattern;
impl Pattern for PerformanceDegradationPattern {
    fn detect(&self, history: &[CollectedEvent]) -> Option<DetectedPattern> {
        let recent = &history[history.len().saturating_sub(10)..];
        
        // Look for increasing queue depths or response times
        let mut queue_depths = Vec::new();
        
        for event in recent {
            if let CollectedEvent::SubstratesMetric { metric_type, value, .. } = event {
                if metric_type == "queue_depth" {
                    queue_depths.push(*value);
                }
            }
        }
        
        // Check if queue depth is increasing
        if queue_depths.len() >= 3 {
            let increasing = queue_depths.windows(2)
                .all(|w| w[1] >= w[0]);
            
            if increasing && queue_depths.last().unwrap_or(&0.0) > &10.0 {
                return Some(DetectedPattern {
                    pattern_type: PatternType::PerformanceDegradation,
                    confidence: 0.75,
                    description: "Queue depth increasing, possible congestion".to_string(),
                    events: vec![format!("queue_depth: {:.0}", queue_depths.last().unwrap())],
                });
            }
        }
        
        None
    }
}