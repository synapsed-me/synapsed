//! Memory architecture for agent intent and context management
//! 
//! Inspired by SAFLA's hybrid memory system, this module provides
//! multiple memory types for efficient storage and retrieval of
//! agent experiences, knowledge, and active context.

use crate::{IntentId, IntentStatus};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque, BTreeMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

/// Memory item with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: Uuid,
    pub content: MemoryContent,
    pub timestamp: DateTime<Utc>,
    pub access_count: usize,
    pub importance_score: f64,
    pub decay_rate: f64,
    pub associations: Vec<Uuid>,
}

/// Content stored in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryContent {
    /// Intent execution memory
    Intent {
        intent_id: IntentId,
        goal: String,
        status: IntentStatus,
        result: Option<serde_json::Value>,
    },
    /// Episodic event memory
    Episode {
        event_type: String,
        context: HashMap<String, serde_json::Value>,
        outcomes: Vec<String>,
    },
    /// Semantic knowledge
    Knowledge {
        concept: String,
        facts: Vec<String>,
        relationships: HashMap<String, Vec<String>>,
    },
    /// Vector embedding
    Embedding {
        vector: Vec<f32>,
        dimensions: usize,
        model: String,
    },
}

/// Vector memory for similarity search
pub struct VectorMemory {
    embeddings: Arc<RwLock<HashMap<Uuid, (Vec<f32>, MemoryItem)>>>,
    dimension: usize,
    max_items: usize,
}

impl VectorMemory {
    pub fn new(dimension: usize, max_items: usize) -> Self {
        Self {
            embeddings: Arc::new(RwLock::new(HashMap::new())),
            dimension,
            max_items,
        }
    }
    
    /// Store an embedding with associated memory item
    pub async fn store(&self, embedding: Vec<f32>, item: MemoryItem) -> Result<Uuid, String> {
        if embedding.len() != self.dimension {
            return Err(format!("Embedding dimension mismatch: expected {}, got {}", 
                self.dimension, embedding.len()));
        }
        
        let mut embeddings = self.embeddings.write().await;
        
        // Evict oldest if at capacity
        if embeddings.len() >= self.max_items {
            if let Some(oldest_id) = self.find_oldest(&embeddings).await {
                embeddings.remove(&oldest_id);
            }
        }
        
        let id = item.id;
        embeddings.insert(id, (embedding, item));
        Ok(id)
    }
    
    /// Find k nearest neighbors
    pub async fn search(&self, query: &[f32], k: usize) -> Vec<(Uuid, f64, MemoryItem)> {
        if query.len() != self.dimension {
            return Vec::new();
        }
        
        let embeddings = self.embeddings.read().await;
        let mut scores: Vec<(Uuid, f64, MemoryItem)> = Vec::new();
        
        for (id, (embedding, item)) in embeddings.iter() {
            let similarity = self.cosine_similarity(query, embedding);
            scores.push((*id, similarity, item.clone()));
        }
        
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);
        scores
    }
    
    /// Calculate cosine similarity between two vectors
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f64 {
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        
        (dot_product / (norm_a * norm_b)) as f64
    }
    
    async fn find_oldest(&self, embeddings: &HashMap<Uuid, (Vec<f32>, MemoryItem)>) -> Option<Uuid> {
        embeddings.iter()
            .min_by_key(|(_, (_, item))| item.timestamp)
            .map(|(id, _)| *id)
    }
}

/// Episodic memory for sequential experiences
pub struct EpisodicMemory {
    episodes: Arc<RwLock<VecDeque<Episode>>>,
    max_episodes: usize,
    temporal_index: Arc<RwLock<BTreeMap<DateTime<Utc>, Vec<Uuid>>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub duration: Duration,
    pub events: Vec<Event>,
    pub outcome: EpisodeOutcome,
    pub importance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub data: serde_json::Value,
    pub agent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EpisodeOutcome {
    Success,
    Failure(String),
    Partial(f64), // 0.0 to 1.0
    Unknown,
}

impl EpisodicMemory {
    pub fn new(max_episodes: usize) -> Self {
        Self {
            episodes: Arc::new(RwLock::new(VecDeque::new())),
            max_episodes,
            temporal_index: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
    
    /// Add a new episode
    pub async fn add_episode(&self, episode: Episode) -> Result<(), String> {
        let mut episodes = self.episodes.write().await;
        
        // Evict oldest if at capacity
        if episodes.len() >= self.max_episodes {
            if let Some(old_episode) = episodes.pop_front() {
                // Remove from temporal index
                let mut index = self.temporal_index.write().await;
                if let Some(ids) = index.get_mut(&old_episode.timestamp) {
                    ids.retain(|id| *id != old_episode.id);
                    if ids.is_empty() {
                        index.remove(&old_episode.timestamp);
                    }
                }
            }
        }
        
        // Add to temporal index
        let mut index = self.temporal_index.write().await;
        index.entry(episode.timestamp)
            .or_insert_with(Vec::new)
            .push(episode.id);
        
        episodes.push_back(episode);
        Ok(())
    }
    
    /// Retrieve episodes within a time range
    pub async fn get_episodes_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<Episode> {
        let episodes = self.episodes.read().await;
        episodes.iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .cloned()
            .collect()
    }
    
    /// Find similar episodes based on event patterns
    pub async fn find_similar_episodes(&self, episode: &Episode, max_results: usize) -> Vec<Episode> {
        let episodes = self.episodes.read().await;
        let mut similarities: Vec<(Episode, f64)> = Vec::new();
        
        for other in episodes.iter() {
            if other.id != episode.id {
                let similarity = self.calculate_episode_similarity(episode, other);
                similarities.push((other.clone(), similarity));
            }
        }
        
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similarities.truncate(max_results);
        similarities.into_iter().map(|(e, _)| e).collect()
    }
    
    fn calculate_episode_similarity(&self, a: &Episode, b: &Episode) -> f64 {
        // Simple similarity based on event type overlap
        let a_types: std::collections::HashSet<_> = a.events.iter().map(|e| &e.event_type).collect();
        let b_types: std::collections::HashSet<_> = b.events.iter().map(|e| &e.event_type).collect();
        
        let intersection = a_types.intersection(&b_types).count();
        let union = a_types.union(&b_types).count();
        
        if union == 0 {
            return 0.0;
        }
        
        intersection as f64 / union as f64
    }
}

/// Semantic memory for knowledge representation
pub struct SemanticMemory {
    concepts: Arc<RwLock<HashMap<String, Concept>>>,
    relationships: Arc<RwLock<HashMap<(String, String), Relationship>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub name: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub access_count: usize,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from_concept: String,
    pub to_concept: String,
    pub relation_type: String,
    pub strength: f64,
    pub evidence: Vec<String>,
}

impl SemanticMemory {
    pub fn new() -> Self {
        Self {
            concepts: Arc::new(RwLock::new(HashMap::new())),
            relationships: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add or update a concept
    pub async fn add_concept(&self, concept: Concept) -> Result<(), String> {
        let mut concepts = self.concepts.write().await;
        concepts.insert(concept.name.clone(), concept);
        Ok(())
    }
    
    /// Add a relationship between concepts
    pub async fn add_relationship(&self, relationship: Relationship) -> Result<(), String> {
        let key = (relationship.from_concept.clone(), relationship.to_concept.clone());
        let mut relationships = self.relationships.write().await;
        relationships.insert(key, relationship);
        Ok(())
    }
    
    /// Get related concepts
    pub async fn get_related_concepts(&self, concept_name: &str) -> Vec<(String, Relationship)> {
        let relationships = self.relationships.read().await;
        relationships.iter()
            .filter(|((from, _), _)| from == concept_name)
            .map(|((_, to), rel)| (to.clone(), rel.clone()))
            .collect()
    }
    
    /// Infer new relationships through transitivity
    pub async fn infer_relationships(&self) -> Vec<Relationship> {
        let relationships = self.relationships.read().await;
        let mut inferred = Vec::new();
        
        // Simple transitivity: if A->B and B->C, infer A->C
        for ((a, b), rel1) in relationships.iter() {
            for ((b2, c), rel2) in relationships.iter() {
                if b == b2 && a != c {
                    // Check if relationship already exists
                    if !relationships.contains_key(&(a.clone(), c.clone())) {
                        inferred.push(Relationship {
                            from_concept: a.clone(),
                            to_concept: c.clone(),
                            relation_type: format!("{}_through_{}", rel1.relation_type, rel2.relation_type),
                            strength: rel1.strength * rel2.strength * 0.8, // Decay factor
                            evidence: vec![format!("Inferred from {} -> {} -> {}", a, b, c)],
                        });
                    }
                }
            }
        }
        
        inferred
    }
}

/// Working memory for active context
pub struct WorkingMemory {
    active_items: Arc<RwLock<VecDeque<WorkingMemoryItem>>>,
    capacity: usize,
    attention_weights: Arc<RwLock<HashMap<Uuid, f64>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryItem {
    pub id: Uuid,
    pub content: serde_json::Value,
    pub priority: f64,
    pub activation: f64,
    pub last_accessed: DateTime<Utc>,
}

impl WorkingMemory {
    pub fn new(capacity: usize) -> Self {
        Self {
            active_items: Arc::new(RwLock::new(VecDeque::new())),
            capacity,
            attention_weights: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add item to working memory
    pub async fn add(&self, item: WorkingMemoryItem) -> Result<(), String> {
        let mut items = self.active_items.write().await;
        
        // Remove if already exists
        items.retain(|i| i.id != item.id);
        
        // Evict lowest priority if at capacity
        if items.len() >= self.capacity {
            let min_idx = items.iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.priority.partial_cmp(&b.priority).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(idx, _)| idx);
            
            if let Some(idx) = min_idx {
                items.remove(idx);
            }
        }
        
        items.push_back(item.clone());
        
        // Update attention weight
        let mut weights = self.attention_weights.write().await;
        weights.insert(item.id, item.priority);
        
        Ok(())
    }
    
    /// Get most relevant items based on attention
    pub async fn get_focused_items(&self, n: usize) -> Vec<WorkingMemoryItem> {
        let items = self.active_items.read().await;
        let mut sorted_items: Vec<_> = items.iter().cloned().collect();
        sorted_items.sort_by(|a, b| {
            b.activation.partial_cmp(&a.activation).unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted_items.truncate(n);
        sorted_items
    }
    
    /// Update activation based on usage
    pub async fn update_activation(&self, id: Uuid, delta: f64) -> Result<(), String> {
        let mut items = self.active_items.write().await;
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.activation = (item.activation + delta).min(1.0).max(0.0);
            item.last_accessed = Utc::now();
            Ok(())
        } else {
            Err("Item not found in working memory".to_string())
        }
    }
    
    /// Decay all activations over time
    pub async fn decay_activations(&self, decay_rate: f64) {
        let mut items = self.active_items.write().await;
        for item in items.iter_mut() {
            item.activation *= (1.0 - decay_rate);
        }
        
        // Remove items with very low activation
        items.retain(|i| i.activation > 0.01);
    }
}

/// Hybrid memory system combining all memory types
pub struct HybridMemory {
    pub vector: VectorMemory,
    pub episodic: EpisodicMemory,
    pub semantic: SemanticMemory,
    pub working: WorkingMemory,
}

impl HybridMemory {
    pub fn new() -> Self {
        Self {
            vector: VectorMemory::new(768, 10000),     // 768-dim embeddings, 10k max
            episodic: EpisodicMemory::new(1000),       // 1000 episodes max
            semantic: SemanticMemory::new(),
            working: WorkingMemory::new(50),           // 50 items in working memory
        }
    }
    
    /// Consolidate memories from working to long-term storage
    pub async fn consolidate(&self) -> Result<(), String> {
        let working_items = self.working.get_focused_items(10).await;
        
        for item in working_items {
            // Convert working memory to episodic if it represents an event
            if let Ok(event_data) = serde_json::from_value::<Event>(item.content.clone()) {
                let episode = Episode {
                    id: Uuid::new_v4(),
                    timestamp: event_data.timestamp,
                    duration: Duration::seconds(0),
                    events: vec![event_data],
                    outcome: EpisodeOutcome::Unknown,
                    importance: item.priority,
                };
                self.episodic.add_episode(episode).await?;
            }
            
            // Update semantic memory if it contains knowledge
            if let Ok(knowledge) = serde_json::from_value::<HashMap<String, serde_json::Value>>(item.content.clone()) {
                if let Some(concept_name) = knowledge.get("concept").and_then(|v| v.as_str()) {
                    let concept = Concept {
                        name: concept_name.to_string(),
                        attributes: knowledge,
                        created_at: Utc::now(),
                        access_count: 1,
                        confidence: item.activation,
                    };
                    self.semantic.add_concept(concept).await?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Query across all memory types
    pub async fn query(&self, query: &str) -> QueryResult {
        // This would integrate with actual embedding models
        // For now, return a placeholder result
        QueryResult {
            vector_results: vec![],
            episodic_results: vec![],
            semantic_results: vec![],
            working_results: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub vector_results: Vec<(Uuid, f64, MemoryItem)>,
    pub episodic_results: Vec<Episode>,
    pub semantic_results: Vec<Concept>,
    pub working_results: Vec<WorkingMemoryItem>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_vector_memory() {
        let memory = VectorMemory::new(3, 100);
        let embedding = vec![0.1, 0.2, 0.3];
        let item = MemoryItem {
            id: Uuid::new_v4(),
            content: MemoryContent::Knowledge {
                concept: "test".to_string(),
                facts: vec!["fact1".to_string()],
                relationships: HashMap::new(),
            },
            timestamp: Utc::now(),
            access_count: 0,
            importance_score: 1.0,
            decay_rate: 0.1,
            associations: vec![],
        };
        
        let id = memory.store(embedding.clone(), item).await.unwrap();
        let results = memory.search(&embedding, 1).await;
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, id);
        assert!((results[0].1 - 1.0).abs() < 0.001); // Perfect match
    }
    
    #[tokio::test]
    async fn test_episodic_memory() {
        let memory = EpisodicMemory::new(10);
        let episode = Episode {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            duration: Duration::minutes(5),
            events: vec![
                Event {
                    timestamp: Utc::now(),
                    event_type: "test_event".to_string(),
                    data: serde_json::json!({"key": "value"}),
                    agent_id: None,
                }
            ],
            outcome: EpisodeOutcome::Success,
            importance: 0.8,
        };
        
        memory.add_episode(episode.clone()).await.unwrap();
        let similar = memory.find_similar_episodes(&episode, 5).await;
        
        assert_eq!(similar.len(), 0); // No other episodes to compare
    }
}