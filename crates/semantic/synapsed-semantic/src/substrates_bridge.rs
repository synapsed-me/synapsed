//! Bridge between Substrates events and semantic stories
//! 
//! This module converts Substrates event circuits into story fragments,
//! enabling the observable event layer to feed the semantic narrative layer.

use crate::{
    story::{Story, StoryEvent, StoryEventType, StoryFragment},
    SemanticCoords, SemanticPosition,
};
// Note: In production, these would come from synapsed_substrates
// For now, using simplified types to avoid circular dependencies
use uuid::Uuid;
use chrono::{DateTime, Utc};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;
use serde::{Deserialize, Serialize};

/// Event from Substrates circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub source_id: Uuid,
    pub name: String,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub data: Value,
}

/// Stream of events
pub struct EventStream {
    events: Vec<Event>,
    position: usize,
}

impl EventStream {
    pub async fn next(&mut self) -> Option<Event> {
        if self.position < self.events.len() {
            let event = self.events[self.position].clone();
            self.position += 1;
            Some(event)
        } else {
            None
        }
    }
}

/// Converts Substrates events into story events
pub struct SubstratesStoryBridge {
    /// Current story being recorded
    current_story: Arc<RwLock<Option<Story>>>,
    
    /// Event buffer for batching
    event_buffer: Arc<RwLock<Vec<StoryEvent>>>,
    
    /// Semantic position tracker
    position_tracker: Arc<RwLock<SemanticPositionTracker>>,
    
    /// Event type mappings
    event_mappings: EventTypeMapper,
}

impl SubstratesStoryBridge {
    /// Create a new bridge
    pub fn new() -> Self {
        Self {
            current_story: Arc::new(RwLock::new(None)),
            event_buffer: Arc::new(RwLock::new(Vec::new())),
            position_tracker: Arc::new(RwLock::new(SemanticPositionTracker::new())),
            event_mappings: EventTypeMapper::default(),
        }
    }
    
    /// Convert a Substrates event to a story event
    pub async fn convert_event(&self, event: &Event) -> StoryEvent {
        let event_type = self.event_mappings.map_event_type(&event.name);
        let position = self.position_tracker.read().await.current_position();
        
        StoryEvent {
            id: Uuid::new_v4(),
            agent_id: event.source_id,
            event_type,
            description: event.description.clone(),
            timestamp: event.timestamp,
            position: position.coords,
            data: event.data.clone(),
        }
    }
    
    /// Process an event stream into story fragments
    pub async fn process_stream(&self, mut stream: EventStream) -> Vec<StoryFragment> {
        let mut fragments = Vec::new();
        let mut sequence = 0;
        
        while let Some(event) = stream.next().await {
            let story_event = self.convert_event(&event).await;
            let position = self.position_tracker.read().await.current_position();
            
            let fragment = StoryFragment {
                id: Uuid::new_v4(),
                content: format_event_as_narrative(&story_event),
                sequence,
                position: position.coords,
                connects_to: Vec::new(), // Will be populated by narrative builder
            };
            
            fragments.push(fragment);
            sequence += 1;
            
            // Update position based on event
            self.position_tracker.write().await
                .update_from_event(&story_event);
        }
        
        fragments
    }
    
    /// Start recording a new story
    pub async fn begin_story(&self, intent: crate::traits::Intent) {
        let story = Story::begin(intent);
        *self.current_story.write().await = Some(story);
        self.event_buffer.write().await.clear();
    }
    
    /// Add event to current story
    pub async fn record_event(&self, event: StoryEvent) {
        if let Some(story) = self.current_story.write().await.as_mut() {
            story.record_event(event.clone());
        }
        
        self.event_buffer.write().await.push(event);
    }
    
    /// Complete current story with verification
    pub async fn complete_story(&self, verification: synapsed_verify::VerificationResult) -> Option<Story> {
        let mut story_guard = self.current_story.write().await;
        if let Some(mut story) = story_guard.take() {
            story.complete(verification);
            Some(story)
        } else {
            None
        }
    }
}

/// Maps Substrates event types to story event types
#[derive(Debug, Clone)]
pub struct EventTypeMapper {
    mappings: std::collections::HashMap<String, StoryEventType>,
}

impl EventTypeMapper {
    /// Map an event name to a story event type
    pub fn map_event_type(&self, event_name: &str) -> StoryEventType {
        self.mappings.get(event_name)
            .cloned()
            .unwrap_or_else(|| {
                // Infer from event name patterns
                match event_name {
                    name if name.contains("intent") => StoryEventType::IntentDeclared,
                    name if name.contains("promise") && name.contains("made") => StoryEventType::PromiseMade,
                    name if name.contains("promise") && name.contains("accept") => StoryEventType::PromiseAccepted,
                    name if name.contains("promise") && name.contains("reject") => StoryEventType::PromiseRejected,
                    name if name.contains("execute") || name.contains("start") => StoryEventType::ExecutionStarted,
                    name if name.contains("invoke") || name.contains("call") => StoryEventType::ModuleInvoked,
                    name if name.contains("transform") || name.contains("process") => StoryEventType::DataTransformed,
                    name if name.contains("error") || name.contains("fail") => StoryEventType::ErrorOccurred,
                    name if name.contains("complete") || name.contains("finish") => StoryEventType::ExecutionCompleted,
                    name if name.contains("verif") => StoryEventType::VerificationPerformed,
                    _ => StoryEventType::ModuleInvoked,
                }
            })
    }
    
    /// Add custom mapping
    pub fn add_mapping(&mut self, event_name: String, story_type: StoryEventType) {
        self.mappings.insert(event_name, story_type);
    }
}

impl Default for EventTypeMapper {
    fn default() -> Self {
        let mut mappings = std::collections::HashMap::new();
        
        // Standard mappings
        mappings.insert("intent.declared".to_string(), StoryEventType::IntentDeclared);
        mappings.insert("promise.made".to_string(), StoryEventType::PromiseMade);
        mappings.insert("promise.accepted".to_string(), StoryEventType::PromiseAccepted);
        mappings.insert("promise.rejected".to_string(), StoryEventType::PromiseRejected);
        mappings.insert("execution.started".to_string(), StoryEventType::ExecutionStarted);
        mappings.insert("module.invoked".to_string(), StoryEventType::ModuleInvoked);
        mappings.insert("data.transformed".to_string(), StoryEventType::DataTransformed);
        mappings.insert("error.occurred".to_string(), StoryEventType::ErrorOccurred);
        mappings.insert("execution.completed".to_string(), StoryEventType::ExecutionCompleted);
        mappings.insert("verification.performed".to_string(), StoryEventType::VerificationPerformed);
        
        Self { mappings }
    }
}

/// Tracks semantic position based on events
#[derive(Debug, Clone)]
pub struct SemanticPositionTracker {
    current_position: SemanticPosition,
    position_history: Vec<SemanticPosition>,
}

impl SemanticPositionTracker {
    pub fn new() -> Self {
        Self {
            current_position: SemanticPosition::new(
                SemanticCoords::default(),
                "initialization".to_string(),
                vec!["startup".to_string()],
            ),
            position_history: Vec::new(),
        }
    }
    
    pub fn current_position(&self) -> SemanticPosition {
        self.current_position.clone()
    }
    
    pub fn update_from_event(&mut self, event: &StoryEvent) {
        // Save current position to history
        self.position_history.push(self.current_position.clone());
        
        // Update position based on event type
        let mut new_coords = self.current_position.coords;
        
        match event.event_type {
            StoryEventType::IntentDeclared => {
                new_coords.intent += 0.1;
                self.current_position.chapter = "intent".to_string();
                self.current_position.themes = vec!["planning".to_string()];
            }
            StoryEventType::PromiseMade | StoryEventType::PromiseAccepted => {
                new_coords.promise += 0.1;
                self.current_position.chapter = "negotiation".to_string();
                self.current_position.themes.push("cooperation".to_string());
            }
            StoryEventType::ExecutionStarted | StoryEventType::ModuleInvoked => {
                new_coords.expression += 0.1;
                self.current_position.chapter = "execution".to_string();
                self.current_position.themes.push("action".to_string());
            }
            StoryEventType::VerificationPerformed => {
                new_coords.context += 0.1;
                self.current_position.chapter = "verification".to_string();
                self.current_position.themes.push("validation".to_string());
            }
            StoryEventType::ErrorOccurred => {
                // Errors move us away in all dimensions
                new_coords.intent -= 0.05;
                new_coords.promise -= 0.05;
                new_coords.expression -= 0.05;
                self.current_position.themes.push("recovery".to_string());
            }
            _ => {
                // Minor adjustments for other events
                new_coords.context += 0.02;
            }
        }
        
        // Ensure coordinates stay in valid range
        new_coords.intent = new_coords.intent.clamp(0.0, 1.0);
        new_coords.promise = new_coords.promise.clamp(0.0, 1.0);
        new_coords.context = new_coords.context.clamp(0.0, 1.0);
        new_coords.expression = new_coords.expression.clamp(0.0, 1.0);
        
        self.current_position.coords = new_coords;
        self.current_position.timestamp = Utc::now();
    }
}

/// Format a story event as narrative text
fn format_event_as_narrative(event: &StoryEvent) -> String {
    match event.event_type {
        StoryEventType::IntentDeclared => {
            format!("The journey begins with a declaration of intent: {}", event.description)
        }
        StoryEventType::PromiseMade => {
            format!("A promise is made: {}", event.description)
        }
        StoryEventType::PromiseAccepted => {
            format!("The promise is accepted, cooperation begins: {}", event.description)
        }
        StoryEventType::PromiseRejected => {
            format!("The promise is rejected: {}", event.description)
        }
        StoryEventType::ExecutionStarted => {
            format!("Execution commences: {}", event.description)
        }
        StoryEventType::ModuleInvoked => {
            format!("A module springs to life: {}", event.description)
        }
        StoryEventType::DataTransformed => {
            format!("Data flows and transforms: {}", event.description)
        }
        StoryEventType::ErrorOccurred => {
            format!("An obstacle appears: {}", event.description)
        }
        StoryEventType::ExecutionCompleted => {
            format!("The execution concludes: {}", event.description)
        }
        StoryEventType::VerificationPerformed => {
            format!("Truth is verified: {}", event.description)
        }
    }
}

// Subscriber trait would be implemented when integrating with actual Substrates
// For now, we provide a manual receive method
impl SubstratesStoryBridge {
    pub async fn receive_event(&self, event: Event) {
        let story_event = self.convert_event(&event).await;
        self.record_event(story_event).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_event_conversion() {
        let bridge = SubstratesStoryBridge::new();
        
        let substrate_event = Event {
            id: Uuid::new_v4(),
            source_id: Uuid::new_v4(),
            name: "promise.made".to_string(),
            description: "Agent promises to execute task".to_string(),
            timestamp: Utc::now(),
            data: serde_json::json!({"task": "test"}),
        };
        
        let story_event = bridge.convert_event(&substrate_event).await;
        assert_eq!(story_event.event_type, StoryEventType::PromiseMade);
    }
    
    #[test]
    fn test_event_type_mapping() {
        let mapper = EventTypeMapper::default();
        
        assert_eq!(mapper.map_event_type("intent.declared"), StoryEventType::IntentDeclared);
        assert_eq!(mapper.map_event_type("promise.made"), StoryEventType::PromiseMade);
        assert_eq!(mapper.map_event_type("unknown.event"), StoryEventType::ModuleInvoked);
    }
}