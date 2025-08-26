//! Story recording infrastructure for capturing execution narratives
//!
//! This module provides the core infrastructure for recording stories
//! as they happen, managing story lifecycle, and preparing them for storage.

use crate::{
    story::{Story, StoryEvent, StoryEventType, StoryContext, Narrative, NarrativeArc},
    substrates_bridge::{SubstratesStoryBridge, Event},
    serventis_bridge::{ServentisStoryHealth, StoryHealth},
    verification_gate::{VerificationGate, GateTicket, VerifiedStory},
    traits::Intent,
    SemanticResult, SemanticError,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// The main story recorder that manages story lifecycle
pub struct StoryRecorder {
    /// Active stories being recorded
    active_stories: Arc<RwLock<HashMap<Uuid, ActiveStory>>>,
    
    /// Completed stories buffer
    completed_buffer: Arc<RwLock<Vec<Story>>>,
    
    /// Substrates bridge for event conversion
    substrates_bridge: Arc<SubstratesStoryBridge>,
    
    /// Serventis bridge for health monitoring
    serventis_health: Arc<ServentisStoryHealth>,
    
    /// Verification gate
    verification_gate: Arc<VerificationGate>,
    
    /// Event channel for async recording
    event_sender: mpsc::Sender<RecordingEvent>,
    event_receiver: Arc<RwLock<mpsc::Receiver<RecordingEvent>>>,
    
    /// Configuration
    config: RecorderConfig,
    
    /// Recording statistics
    stats: Arc<RwLock<RecordingStats>>,
}

impl StoryRecorder {
    /// Create a new story recorder
    pub fn new(config: RecorderConfig) -> Self {
        let (tx, rx) = mpsc::channel(config.event_buffer_size);
        
        Self {
            active_stories: Arc::new(RwLock::new(HashMap::new())),
            completed_buffer: Arc::new(RwLock::new(Vec::new())),
            substrates_bridge: Arc::new(SubstratesStoryBridge::new()),
            serventis_health: Arc::new(ServentisStoryHealth::new()),
            verification_gate: Arc::new(VerificationGate::new(Default::default())),
            event_sender: tx,
            event_receiver: Arc::new(RwLock::new(rx)),
            config,
            stats: Arc::new(RwLock::new(RecordingStats::default())),
        }
    }
    
    /// Start recording a new story
    pub async fn begin_story(&self, intent: Intent) -> SemanticResult<StoryHandle> {
        let story_id = Uuid::new_v4();
        let story = Story::begin(intent);
        
        let active = ActiveStory {
            story,
            handle: StoryHandle { id: story_id },
            started_at: Utc::now(),
            last_event: Utc::now(),
            event_count: 0,
            parent_story: None,
        };
        
        self.active_stories.write().await.insert(story_id, active);
        
        // Update stats
        self.stats.write().await.stories_started += 1;
        
        // Send recording started event
        let _ = self.event_sender.send(RecordingEvent::StoryStarted {
            story_id,
            timestamp: Utc::now(),
        }).await;
        
        Ok(StoryHandle { id: story_id })
    }
    
    /// Record an event in an active story
    pub async fn record_event(
        &self,
        handle: &StoryHandle,
        event: StoryEvent,
    ) -> SemanticResult<()> {
        let mut stories = self.active_stories.write().await;
        
        let active = stories.get_mut(&handle.id)
            .ok_or_else(|| SemanticError::Other("Story not found".to_string()))?;
        
        active.story.record_event(event.clone());
        active.event_count += 1;
        active.last_event = Utc::now();
        
        // Check if this is a verification event
        if matches!(event.event_type, StoryEventType::VerificationPerformed) {
            // Trigger verification gate check
            let _ = self.event_sender.send(RecordingEvent::VerificationTriggered {
                story_id: handle.id,
                timestamp: Utc::now(),
            }).await;
        }
        
        // Update stats
        self.stats.write().await.events_recorded += 1;
        
        Ok(())
    }
    
    /// Complete a story and submit for verification
    pub async fn complete_story(
        &self,
        handle: &StoryHandle,
        verification: synapsed_verify::VerificationResult,
    ) -> SemanticResult<GateTicket> {
        let mut stories = self.active_stories.write().await;
        
        let mut active = stories.remove(&handle.id)
            .ok_or_else(|| SemanticError::Other("Story not found".to_string()))?;
        
        // Complete the story
        active.story.complete(verification);
        
        // Submit to verification gate
        let ticket = self.verification_gate.submit_story(active.story.clone()).await?;
        
        // Update stats
        self.stats.write().await.stories_completed += 1;
        
        // Send completion event
        let _ = self.event_sender.send(RecordingEvent::StoryCompleted {
            story_id: handle.id,
            ticket: ticket.clone(),
            timestamp: Utc::now(),
        }).await;
        
        Ok(ticket)
    }
    
    /// Get verification result
    pub async fn get_verified_story(
        &self,
        ticket: &GateTicket,
    ) -> SemanticResult<VerifiedStory> {
        self.verification_gate.verify(ticket).await
    }
    
    /// Record a Substrates event
    pub async fn record_substrates_event(&self, event: Event) -> SemanticResult<()> {
        // Convert to story event
        let story_event = self.substrates_bridge.convert_event(&event).await;
        
        // Find active story for this event (by source_id or context)
        // For now, broadcast to all active stories
        let stories = self.active_stories.read().await;
        for (_, active) in stories.iter() {
            // In production, would match event to specific story
            let _ = self.record_event(&active.handle, story_event.clone()).await;
        }
        
        Ok(())
    }
    
    /// Create a narrative from related stories
    pub async fn create_narrative(
        &self,
        title: String,
        story_ids: Vec<Uuid>,
    ) -> SemanticResult<Narrative> {
        let completed = self.completed_buffer.read().await;
        
        let stories: Vec<Story> = completed.iter()
            .filter(|s| story_ids.contains(&s.id))
            .cloned()
            .collect();
        
        if stories.is_empty() {
            return Err(SemanticError::Other("No stories found for narrative".to_string()));
        }
        
        let protagonists = stories.iter()
            .flat_map(|s| s.participants())
            .collect::<Vec<_>>();
        
        let narrative = Narrative {
            id: Uuid::new_v4(),
            title,
            stories,
            theme: self.detect_theme(&protagonists),
            arc: self.detect_arc(&story_ids),
            protagonists,
        };
        
        Ok(narrative)
    }
    
    /// Start the background event processor
    pub async fn start_processor(&self) {
        let receiver = self.event_receiver.clone();
        let recorder = self.clone();
        
        tokio::spawn(async move {
            let mut rx = receiver.write().await;
            
            while let Some(event) = rx.recv().await {
                recorder.process_recording_event(event).await;
            }
        });
    }
    
    /// Process recording events
    async fn process_recording_event(&self, event: RecordingEvent) {
        match event {
            RecordingEvent::StoryStarted { story_id, .. } => {
                tracing::info!("Story started: {}", story_id);
            }
            RecordingEvent::StoryCompleted { story_id, ticket, .. } => {
                tracing::info!("Story completed: {}, ticket: {:?}", story_id, ticket);
                
                // Try to verify automatically
                if self.config.auto_verify {
                    if let Ok(verified) = self.verification_gate.verify(&ticket).await {
                        // Add to completed buffer
                        self.completed_buffer.write().await.push(verified.story);
                        self.stats.write().await.stories_verified += 1;
                    }
                }
            }
            RecordingEvent::VerificationTriggered { story_id, .. } => {
                tracing::debug!("Verification triggered for story: {}", story_id);
            }
            RecordingEvent::HealthCheck { story_id, health, .. } => {
                tracing::debug!("Health check for story {}: {:?}", story_id, health);
            }
        }
    }
    
    /// Detect narrative theme from protagonists
    fn detect_theme(&self, protagonists: &[Uuid]) -> String {
        // Simple theme detection - in production would analyze story content
        if protagonists.len() > 3 {
            "Collaboration".to_string()
        } else if protagonists.len() == 1 {
            "Solo Journey".to_string()
        } else {
            "Partnership".to_string()
        }
    }
    
    /// Detect narrative arc type
    fn detect_arc(&self, story_ids: &[Uuid]) -> NarrativeArc {
        // Simple arc detection - in production would analyze story relationships
        match story_ids.len() {
            1 => NarrativeArc::Linear,
            2..=3 => NarrativeArc::Converging,
            4..=6 => NarrativeArc::Branching,
            _ => NarrativeArc::Emergent,
        }
    }
    
    /// Get recording statistics
    pub async fn get_stats(&self) -> RecordingStats {
        self.stats.read().await.clone()
    }
    
    /// Flush completed stories to storage
    pub async fn flush_completed(&self) -> Vec<Story> {
        let mut buffer = self.completed_buffer.write().await;
        let stories = buffer.clone();
        buffer.clear();
        stories
    }
    
    /// Get active story count
    pub async fn active_count(&self) -> usize {
        self.active_stories.read().await.len()
    }
}

impl Clone for StoryRecorder {
    fn clone(&self) -> Self {
        Self {
            active_stories: self.active_stories.clone(),
            completed_buffer: self.completed_buffer.clone(),
            substrates_bridge: self.substrates_bridge.clone(),
            serventis_health: self.serventis_health.clone(),
            verification_gate: self.verification_gate.clone(),
            event_sender: self.event_sender.clone(),
            event_receiver: self.event_receiver.clone(),
            config: self.config.clone(),
            stats: self.stats.clone(),
        }
    }
}

/// Handle for an active story
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StoryHandle {
    pub id: Uuid,
}

/// Active story being recorded
#[derive(Debug, Clone)]
struct ActiveStory {
    story: Story,
    handle: StoryHandle,
    started_at: DateTime<Utc>,
    last_event: DateTime<Utc>,
    event_count: usize,
    parent_story: Option<Uuid>,
}

/// Events in the recording system
#[derive(Debug, Clone)]
enum RecordingEvent {
    StoryStarted {
        story_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    StoryCompleted {
        story_id: Uuid,
        ticket: GateTicket,
        timestamp: DateTime<Utc>,
    },
    VerificationTriggered {
        story_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    HealthCheck {
        story_id: Uuid,
        health: StoryHealth,
        timestamp: DateTime<Utc>,
    },
}

/// Configuration for the recorder
#[derive(Debug, Clone)]
pub struct RecorderConfig {
    /// Maximum active stories
    pub max_active_stories: usize,
    
    /// Event buffer size
    pub event_buffer_size: usize,
    
    /// Auto-verify on completion
    pub auto_verify: bool,
    
    /// Flush interval in seconds
    pub flush_interval_secs: u64,
    
    /// Enable health monitoring
    pub enable_health_monitoring: bool,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            max_active_stories: 1000,
            event_buffer_size: 10000,
            auto_verify: true,
            flush_interval_secs: 60,
            enable_health_monitoring: true,
        }
    }
}

/// Recording statistics
#[derive(Debug, Clone, Default)]
pub struct RecordingStats {
    pub stories_started: u64,
    pub stories_completed: u64,
    pub stories_verified: u64,
    pub stories_failed: u64,
    pub events_recorded: u64,
    pub narratives_created: u64,
}

/// Story storage trait for persistence
#[async_trait::async_trait]
pub trait StoryStorage: Send + Sync {
    /// Store a verified story
    async fn store(&self, story: VerifiedStory) -> SemanticResult<()>;
    
    /// Retrieve a story by ID
    async fn get(&self, id: Uuid) -> SemanticResult<Option<Story>>;
    
    /// Query stories by criteria
    async fn query(&self, query: StorageQuery) -> SemanticResult<Vec<Story>>;
    
    /// Delete old stories
    async fn cleanup(&self, older_than: DateTime<Utc>) -> SemanticResult<usize>;
}

/// Query for finding stories in storage
#[derive(Debug, Clone)]
pub struct StorageQuery {
    /// Search in goal/description
    pub text_search: Option<String>,
    
    /// Filter by time range
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    
    /// Filter by outcome
    pub outcome_filter: Option<StoryOutcomeFilter>,
    
    /// Limit results
    pub limit: Option<usize>,
    
    /// Semantic similarity search
    pub semantic_coords: Option<crate::SemanticCoords>,
    pub semantic_radius: Option<f64>,
}

/// Filter for story outcomes
#[derive(Debug, Clone)]
pub enum StoryOutcomeFilter {
    SuccessOnly,
    FailureOnly,
    PartialOnly,
    Any,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_story_recording() {
        let recorder = StoryRecorder::new(RecorderConfig::default());
        
        let intent = Intent::new("Test recording");
        let handle = recorder.begin_story(intent).await.unwrap();
        
        assert_eq!(recorder.active_count().await, 1);
        
        // Record some events
        let event = StoryEvent {
            id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            event_type: StoryEventType::ExecutionStarted,
            description: "Started execution".to_string(),
            timestamp: Utc::now(),
            position: Default::default(),
            data: serde_json::json!({}),
        };
        
        recorder.record_event(&handle, event).await.unwrap();
        
        let stats = recorder.get_stats().await;
        assert_eq!(stats.events_recorded, 1);
    }
}