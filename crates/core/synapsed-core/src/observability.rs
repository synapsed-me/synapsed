//! Humainary Observability - A paradigm shift from traditional monitoring.
//!
//! This module implements the Humainary observability paradigm, which focuses on
//! context-aware, reactive observation patterns rather than traditional metrics
//! and monitoring approaches.
//!
//! ## Key Concepts
//!
//! - **Context-Aware Observation**: Understanding the full context of what's happening
//! - **Reactive Patterns**: Responding to changes as they occur
//! - **Behavioral Intelligence**: Learning from patterns of behavior
//! - **Self-Describing Systems**: Components that can explain their own state
//! - **Emergent Understanding**: Insights that emerge from system interactions

use crate::SynapsedResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Context represents the full situational awareness of an observable entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationContext {
    /// Unique context identifier
    pub id: Uuid,
    /// The observer's identity
    pub observer: String,
    /// The subject being observed
    pub subject: String,
    /// Current environmental conditions
    pub environment: EnvironmentalConditions,
    /// Behavioral patterns detected
    pub patterns: Vec<BehavioralPattern>,
    /// Current intentions or goals
    pub intentions: Vec<Intention>,
    /// Relationships to other entities
    pub relationships: Vec<Relationship>,
    /// Temporal context
    pub temporal: TemporalContext,
    /// Spatial context (if applicable)
    pub spatial: Option<SpatialContext>,
}

/// Environmental conditions affecting the observation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalConditions {
    /// System load characteristics
    pub load_characteristics: LoadCharacteristics,
    /// Resource availability
    pub resource_availability: ResourceAvailability,
    /// Network conditions
    pub network_conditions: NetworkConditions,
    /// Security posture
    pub security_posture: SecurityPosture,
    /// External influences
    pub external_influences: Vec<ExternalInfluence>,
}

/// Load characteristics describe how the system is being utilized
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadCharacteristics {
    /// Current workload intensity
    pub intensity: f64, // 0.0 to 1.0
    /// Workload distribution pattern
    pub distribution_pattern: DistributionPattern,
    /// Variability in load
    pub variability: f64,
    /// Predictability of load patterns
    pub predictability: f64,
}

/// Patterns of workload distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistributionPattern {
    /// Uniform distribution
    Uniform,
    /// Burst patterns
    Bursty,
    /// Periodic patterns
    Periodic,
    /// Random distribution
    Random,
    /// Emergent patterns
    Emergent,
}

/// Resource availability in the environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAvailability {
    /// Computational resources
    pub computational: ResourceStatus,
    /// Memory resources
    pub memory: ResourceStatus,
    /// Network bandwidth
    pub network: ResourceStatus,
    /// Storage capacity
    pub storage: ResourceStatus,
    /// Custom resources
    pub custom: HashMap<String, ResourceStatus>,
}

/// Status of a particular resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStatus {
    /// Current availability (0.0 to 1.0)
    pub availability: f64,
    /// Quality of the resource
    pub quality: f64,
    /// Reliability of the resource
    pub reliability: f64,
    /// Trend direction
    pub trend: TrendDirection,
}

/// Direction of resource trends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    /// Resource improving
    Improving,
    /// Resource stable
    Stable,
    /// Resource degrading
    Degrading,
    /// Resource oscillating
    Oscillating,
    /// Trend unknown
    Unknown,
}

/// Network conditions in the environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConditions {
    /// Connectivity quality
    pub connectivity_quality: f64,
    /// Latency characteristics
    pub latency_characteristics: LatencyCharacteristics,
    /// Bandwidth availability
    pub bandwidth_availability: f64,
    /// Network stability
    pub stability: f64,
    /// Peer connectivity
    pub peer_connectivity: Vec<PeerConnection>,
}

/// Latency characteristics of the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyCharacteristics {
    /// Average latency
    pub average: f64,
    /// Latency variance
    pub variance: f64,
    /// Peak latency
    pub peak: f64,
    /// Latency pattern
    pub pattern: LatencyPattern,
}

/// Patterns of network latency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LatencyPattern {
    /// Consistent latency
    Consistent,
    /// Variable latency
    Variable,
    /// Spike patterns
    Spiky,
    /// Gradually changing
    Gradual,
}

/// Peer connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConnection {
    /// Peer identifier
    pub peer_id: String,
    /// Connection quality
    pub quality: f64,
    /// Connection type
    pub connection_type: String,
    /// Relationship to this peer
    pub relationship: String,
}

/// Security posture of the environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPosture {
    /// Trust level
    pub trust_level: f64,
    /// Threat assessment
    pub threat_assessment: ThreatAssessment,
    /// Authentication state
    pub authentication_state: AuthenticationState,
    /// Privacy level
    pub privacy_level: f64,
}

/// Assessment of threats in the environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatAssessment {
    /// Overall threat level
    pub overall_level: f64,
    /// Specific threats detected
    pub detected_threats: Vec<DetectedThreat>,
    /// Confidence in assessment
    pub confidence: f64,
}

/// A detected threat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedThreat {
    /// Threat type
    pub threat_type: String,
    /// Severity level
    pub severity: f64,
    /// Likelihood of occurrence
    pub likelihood: f64,
    /// Potential impact
    pub potential_impact: String,
}

/// Authentication state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationState {
    /// Fully authenticated
    Authenticated,
    /// Partially authenticated
    PartiallyAuthenticated,
    /// Anonymous
    Anonymous,
    /// Authentication pending
    Pending,
    /// Authentication failed
    Failed,
}

/// External influences on the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalInfluence {
    /// Source of influence
    pub source: String,
    /// Type of influence
    pub influence_type: String,
    /// Strength of influence
    pub strength: f64,
    /// Direction of influence
    pub direction: InfluenceDirection,
}

/// Direction of external influence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InfluenceDirection {
    /// Positive influence
    Positive,
    /// Negative influence
    Negative,
    /// Neutral influence
    Neutral,
    /// Mixed influence
    Mixed,
}

/// Behavioral patterns observed in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralPattern {
    /// Pattern identifier
    pub id: Uuid,
    /// Pattern name
    pub name: String,
    /// Pattern description
    pub description: String,
    /// Pattern confidence
    pub confidence: f64,
    /// Pattern frequency
    pub frequency: PatternFrequency,
    /// Pattern triggers
    pub triggers: Vec<PatternTrigger>,
    /// Pattern outcomes
    pub outcomes: Vec<PatternOutcome>,
    /// Pattern evolution
    pub evolution: PatternEvolution,
}

/// Frequency of pattern occurrence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternFrequency {
    /// Continuous pattern
    Continuous,
    /// Regular intervals
    Regular,
    /// Irregular occurrence
    Irregular,
    /// One-time occurrence
    OneTime,
    /// Seasonal pattern
    Seasonal,
}

/// Trigger that activates a pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternTrigger {
    /// Trigger condition
    pub condition: String,
    /// Trigger strength
    pub strength: f64,
    /// Trigger timing
    pub timing: String,
}

/// Outcome of a pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternOutcome {
    /// Outcome description
    pub description: String,
    /// Outcome probability
    pub probability: f64,
    /// Outcome impact
    pub impact: f64,
}

/// Evolution of a pattern over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEvolution {
    /// How the pattern is changing
    pub change_direction: ChangeDirection,
    /// Rate of change
    pub change_rate: f64,
    /// Stability of the pattern
    pub stability: f64,
    /// Predictability of evolution
    pub predictability: f64,
}

/// Direction of pattern change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeDirection {
    /// Pattern strengthening
    Strengthening,
    /// Pattern weakening
    Weakening,
    /// Pattern stable
    Stable,
    /// Pattern oscillating
    Oscillating,
    /// Pattern transforming
    Transforming,
}

/// Intentions represent goals or purposes driving behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intention {
    /// Intention identifier
    pub id: Uuid,
    /// Goal description
    pub goal: String,
    /// Priority level
    pub priority: f64,
    /// Progress toward goal
    pub progress: f64,
    /// Constraints affecting goal
    pub constraints: Vec<Constraint>,
    /// Resources required
    pub resource_requirements: ResourceRequirements,
}

/// Constraint affecting an intention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Constraint type
    pub constraint_type: String,
    /// Constraint severity
    pub severity: f64,
    /// Constraint description
    pub description: String,
}

/// Resource requirements for an intention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// Computational requirements
    pub computational: f64,
    /// Memory requirements
    pub memory: f64,
    /// Network requirements
    pub network: f64,
    /// Time requirements
    pub time: f64,
    /// Custom requirements
    pub custom: HashMap<String, f64>,
}

/// Relationship between entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Related entity identifier
    pub entity_id: String,
    /// Relationship type
    pub relationship_type: RelationshipType,
    /// Relationship strength
    pub strength: f64,
    /// Relationship direction
    pub direction: RelationshipDirection,
    /// Relationship stability
    pub stability: f64,
}

/// Types of relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    /// Dependency relationship
    Dependency,
    /// Collaboration relationship
    Collaboration,
    /// Competition relationship
    Competition,
    /// Communication relationship
    Communication,
    /// Hierarchical relationship
    Hierarchical,
    /// Peer relationship
    Peer,
    /// Custom relationship
    Custom(String),
}

/// Direction of relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipDirection {
    /// Bidirectional relationship
    Bidirectional,
    /// Unidirectional outgoing
    Outgoing,
    /// Unidirectional incoming
    Incoming,
}

/// Temporal context of observations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalContext {
    /// Current time
    pub current_time: chrono::DateTime<chrono::Utc>,
    /// Time since last significant event
    pub time_since_last_event: chrono::Duration,
    /// Temporal patterns
    pub patterns: Vec<TemporalPattern>,
    /// Time horizon for predictions
    pub prediction_horizon: chrono::Duration,
}

/// Temporal patterns in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalPattern {
    /// Pattern cycle duration
    pub cycle_duration: chrono::Duration,
    /// Pattern phase
    pub phase: f64, // 0.0 to 1.0
    /// Pattern amplitude
    pub amplitude: f64,
    /// Pattern description
    pub description: String,
}

/// Spatial context for location-aware observations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialContext {
    /// Current location
    pub location: Location,
    /// Nearby entities
    pub nearby_entities: Vec<NearbyEntity>,
    /// Spatial patterns
    pub patterns: Vec<SpatialPattern>,
}

/// Location representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// Location type
    pub location_type: LocationType,
    /// Coordinates (if applicable)
    pub coordinates: Option<Coordinates>,
    /// Named location
    pub name: Option<String>,
}

/// Types of locations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocationType {
    /// Geographic location
    Geographic,
    /// Network location
    Network,
    /// Logical location
    Logical,
    /// Virtual location
    Virtual,
}

/// Coordinate system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coordinates {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
    /// Z coordinate (optional)
    pub z: Option<f64>,
}

/// Nearby entity in spatial context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbyEntity {
    /// Entity identifier
    pub entity_id: String,
    /// Distance to entity
    pub distance: f64,
    /// Relationship to entity
    pub relationship: Option<String>,
}

/// Spatial patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialPattern {
    /// Pattern description
    pub description: String,
    /// Pattern scale
    pub scale: f64,
    /// Pattern density
    pub density: f64,
}

/// Trait for context-aware observability
#[async_trait]
pub trait ContextAwareObservable: Send + Sync {
    /// Get the current observation context
    async fn observation_context(&self) -> SynapsedResult<ObservationContext>;

    /// Update the observation context
    async fn update_context(&mut self, context: ObservationContext) -> SynapsedResult<()>;

    /// React to context changes
    async fn react_to_context(&mut self, context: &ObservationContext) -> SynapsedResult<Vec<ContextReaction>>;

    /// Learn from observed patterns
    async fn learn_from_patterns(&mut self, patterns: Vec<BehavioralPattern>) -> SynapsedResult<()>;

    /// Predict future states based on context
    async fn predict_future_states(&self, horizon: chrono::Duration) -> SynapsedResult<Vec<FutureState>>;
}

/// Reaction to context changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextReaction {
    /// Reaction type
    pub reaction_type: ReactionType,
    /// Reaction description
    pub description: String,
    /// Reaction priority
    pub priority: f64,
    /// Actions to take
    pub actions: Vec<ContextAction>,
}

/// Types of reactions to context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReactionType {
    /// Adaptive reaction
    Adaptive,
    /// Defensive reaction
    Defensive,
    /// Proactive reaction
    Proactive,
    /// Corrective reaction
    Corrective,
    /// Optimizing reaction
    Optimizing,
}

/// Action to take in response to context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAction {
    /// Action type
    pub action_type: String,
    /// Action parameters
    pub parameters: HashMap<String, String>,
    /// Action urgency
    pub urgency: f64,
}

/// Predicted future state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FutureState {
    /// Time of prediction
    pub time: chrono::DateTime<chrono::Utc>,
    /// Predicted context
    pub predicted_context: ObservationContext,
    /// Prediction confidence
    pub confidence: f64,
    /// Key factors in prediction
    pub key_factors: Vec<String>,
}

/// Reactive observer that responds to changes
#[async_trait]
pub trait ReactiveObserver: Send + Sync {
    /// Subscribe to context changes
    async fn subscribe_to_context_changes(&mut self) -> SynapsedResult<()>;

    /// Handle context change events
    async fn handle_context_change(&mut self, context: ObservationContext) -> SynapsedResult<()>;

    /// Get observer configuration
    fn get_observer_config(&self) -> ObserverConfig;
}

/// Configuration for observers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverConfig {
    /// Observer sensitivity to changes
    pub sensitivity: f64,
    /// Reaction threshold
    pub reaction_threshold: f64,
    /// Learning rate
    pub learning_rate: f64,
    /// Pattern recognition enabled
    pub pattern_recognition: bool,
    /// Predictive analysis enabled
    pub predictive_analysis: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observation_context_creation() {
        let context = ObservationContext {
            id: Uuid::new_v4(),
            observer: "test_observer".to_string(),
            subject: "test_subject".to_string(),
            environment: EnvironmentalConditions {
                load_characteristics: LoadCharacteristics {
                    intensity: 0.5,
                    distribution_pattern: DistributionPattern::Uniform,
                    variability: 0.2,
                    predictability: 0.8,
                },
                resource_availability: ResourceAvailability {
                    computational: ResourceStatus {
                        availability: 0.7,
                        quality: 0.9,
                        reliability: 0.8,
                        trend: TrendDirection::Stable,
                    },
                    memory: ResourceStatus {
                        availability: 0.6,
                        quality: 0.8,
                        reliability: 0.9,
                        trend: TrendDirection::Improving,
                    },
                    network: ResourceStatus {
                        availability: 0.8,
                        quality: 0.7,
                        reliability: 0.7,
                        trend: TrendDirection::Oscillating,
                    },
                    storage: ResourceStatus {
                        availability: 0.9,
                        quality: 0.9,
                        reliability: 0.9,
                        trend: TrendDirection::Stable,
                    },
                    custom: HashMap::new(),
                },
                network_conditions: NetworkConditions {
                    connectivity_quality: 0.8,
                    latency_characteristics: LatencyCharacteristics {
                        average: 50.0,
                        variance: 10.0,
                        peak: 100.0,
                        pattern: LatencyPattern::Consistent,
                    },
                    bandwidth_availability: 0.9,
                    stability: 0.8,
                    peer_connectivity: Vec::new(),
                },
                security_posture: SecurityPosture {
                    trust_level: 0.8,
                    threat_assessment: ThreatAssessment {
                        overall_level: 0.2,
                        detected_threats: Vec::new(),
                        confidence: 0.9,
                    },
                    authentication_state: AuthenticationState::Authenticated,
                    privacy_level: 0.9,
                },
                external_influences: Vec::new(),
            },
            patterns: Vec::new(),
            intentions: Vec::new(),
            relationships: Vec::new(),
            temporal: TemporalContext {
                current_time: chrono::Utc::now(),
                time_since_last_event: chrono::Duration::seconds(30),
                patterns: Vec::new(),
                prediction_horizon: chrono::Duration::minutes(5),
            },
            spatial: None,
        };

        assert_eq!(context.observer, "test_observer");
        assert_eq!(context.subject, "test_subject");
        assert_eq!(context.environment.load_characteristics.intensity, 0.5);
    }

    #[test]
    fn test_behavioral_pattern() {
        let pattern = BehavioralPattern {
            id: Uuid::new_v4(),
            name: "test_pattern".to_string(),
            description: "A test pattern".to_string(),
            confidence: 0.8,
            frequency: PatternFrequency::Regular,
            triggers: vec![PatternTrigger {
                condition: "load > 0.8".to_string(),
                strength: 0.9,
                timing: "immediate".to_string(),
            }],
            outcomes: vec![PatternOutcome {
                description: "System adaptation".to_string(),
                probability: 0.7,
                impact: 0.5,
            }],
            evolution: PatternEvolution {
                change_direction: ChangeDirection::Strengthening,
                change_rate: 0.1,
                stability: 0.8,
                predictability: 0.7,
            },
        };

        assert_eq!(pattern.name, "test_pattern");
        assert_eq!(pattern.confidence, 0.8);
        assert!(matches!(pattern.frequency, PatternFrequency::Regular));
    }

    #[test]
    fn test_relationship() {
        let relationship = Relationship {
            entity_id: "entity_123".to_string(),
            relationship_type: RelationshipType::Collaboration,
            strength: 0.8,
            direction: RelationshipDirection::Bidirectional,
            stability: 0.9,
        };

        assert_eq!(relationship.entity_id, "entity_123");
        assert!(matches!(relationship.relationship_type, RelationshipType::Collaboration));
        assert_eq!(relationship.strength, 0.8);
    }

    #[test]
    fn test_intention() {
        let intention = Intention {
            id: Uuid::new_v4(),
            goal: "Optimize performance".to_string(),
            priority: 0.9,
            progress: 0.5,
            constraints: vec![Constraint {
                constraint_type: "resource".to_string(),
                severity: 0.6,
                description: "Limited memory".to_string(),
            }],
            resource_requirements: ResourceRequirements {
                computational: 0.7,
                memory: 0.8,
                network: 0.3,
                time: 0.5,
                custom: HashMap::new(),
            },
        };

        assert_eq!(intention.goal, "Optimize performance");
        assert_eq!(intention.priority, 0.9);
        assert_eq!(intention.progress, 0.5);
    }
}