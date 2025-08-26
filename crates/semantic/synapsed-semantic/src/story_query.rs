//! Story query interface for AI interaction
//!
//! This module provides a natural language interface for AI agents to
//! discover capabilities and learn from stories rather than documentation.

use crate::{
    story::{Story, StoryEvent, StoryOutcome, Narrative},
    navigation::SemanticNavigator,
    trust::TrustNetwork,
    SemanticCoords, SemanticResult, SemanticError,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Story query engine for AI agents
pub struct StoryQueryEngine {
    /// Story database (in-memory for now)
    stories: Arc<RwLock<Vec<Story>>>,
    
    /// Semantic navigator for finding related stories
    navigator: Arc<RwLock<SemanticNavigator>>,
    
    /// Trust network for ranking results
    trust_network: Arc<RwLock<TrustNetwork>>,
    
    /// Query cache for performance
    cache: Arc<RwLock<QueryCache>>,
    
    /// Configuration
    config: QueryConfig,
}

impl StoryQueryEngine {
    /// Create a new query engine
    pub fn new(config: QueryConfig) -> Self {
        Self {
            stories: Arc::new(RwLock::new(Vec::new())),
            navigator: Arc::new(RwLock::new(SemanticNavigator::new())),
            trust_network: Arc::new(RwLock::new(TrustNetwork::new())),
            cache: Arc::new(RwLock::new(QueryCache::new(config.cache_size))),
            config,
        }
    }
    
    /// Add stories to the engine
    pub async fn add_stories(&self, stories: Vec<Story>) {
        let mut store = self.stories.write().await;
        let mut navigator = self.navigator.write().await;
        
        for story in stories {
            // Add to navigator for semantic search
            if let Some(last_pos) = story.path.positions.last() {
                navigator.add_agent(
                    story.id,
                    *last_pos,
                    story.intent.goal.clone(),
                );
            }
            
            store.push(story);
        }
    }
    
    /// Query stories using natural language
    pub async fn query_natural(&self, query: &str) -> SemanticResult<QueryResponse> {
        // Check cache first
        if let Some(cached) = self.cache.read().await.get(query) {
            return Ok(cached);
        }
        
        // Parse query intent
        let intent = self.parse_query_intent(query);
        
        // Find relevant stories
        let stories = match intent {
            QueryIntent::HowTo(task) => self.find_how_to_stories(&task).await?,
            QueryIntent::WhatHappened(context) => self.find_what_happened(&context).await?,
            QueryIntent::WhoCan(capability) => self.find_who_can(&capability).await?,
            QueryIntent::WhatWentWrong(error) => self.find_failures(&error).await?,
            QueryIntent::ShowPattern(pattern) => self.find_patterns(&pattern).await?,
        };
        
        // Build response
        let response = self.build_response(stories, query).await;
        
        // Cache result
        self.cache.write().await.put(query.to_string(), response.clone());
        
        Ok(response)
    }
    
    /// Find stories by semantic similarity
    pub async fn query_semantic(
        &self,
        coords: SemanticCoords,
        radius: f64,
    ) -> SemanticResult<Vec<Story>> {
        let navigator = self.navigator.read().await;
        let nearest = navigator.find_nearest(coords, self.config.max_results);
        
        let stories = self.stories.read().await;
        let mut results = Vec::new();
        
        for (id, distance) in nearest {
            if distance <= radius {
                if let Some(story) = stories.iter().find(|s| s.id == id) {
                    results.push(story.clone());
                }
            }
        }
        
        Ok(results)
    }
    
    /// Find stories about how to do something
    async fn find_how_to_stories(&self, task: &str) -> SemanticResult<Vec<Story>> {
        let stories = self.stories.read().await;
        
        let mut matches: Vec<(Story, f64)> = stories.iter()
            .filter_map(|story| {
                // Score based on goal similarity
                let score = self.text_similarity(&story.intent.goal, task);
                
                // Only include successful stories for "how to"
                if story.is_successful() && score > self.config.similarity_threshold {
                    Some((story.clone(), score))
                } else {
                    None
                }
            })
            .collect();
        
        // Sort by score
        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        Ok(matches.into_iter()
            .take(self.config.max_results)
            .map(|(story, _)| story)
            .collect())
    }
    
    /// Find stories about what happened in a context
    async fn find_what_happened(&self, context: &str) -> SemanticResult<Vec<Story>> {
        let stories = self.stories.read().await;
        
        let matches: Vec<Story> = stories.iter()
            .filter(|story| {
                // Check if context matches
                story.context.chapter.contains(context) ||
                story.context.themes.iter().any(|t| t.contains(context))
            })
            .cloned()
            .collect();
        
        Ok(matches)
    }
    
    /// Find agents who can do something
    async fn find_who_can(&self, capability: &str) -> SemanticResult<Vec<Story>> {
        let stories = self.stories.read().await;
        let trust = self.trust_network.read().await;
        
        // Find stories demonstrating the capability
        let capable_agents: Vec<uuid::Uuid> = stories.iter()
            .filter(|story| {
                story.is_successful() &&
                (story.intent.goal.contains(capability) ||
                 story.execution.iter().any(|e| e.description.contains(capability)))
            })
            .flat_map(|story| story.participants())
            .collect();
        
        // Get stories from trusted agents
        let mut results = Vec::new();
        for agent_id in capable_agents {
            let reputation = trust.get_reputation(agent_id);
            if reputation > self.config.min_trust_score {
                // Get their successful stories
                for story in stories.iter() {
                    if story.participants().contains(&agent_id) && story.is_successful() {
                        results.push(story.clone());
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Find stories about failures
    async fn find_failures(&self, error: &str) -> SemanticResult<Vec<Story>> {
        let stories = self.stories.read().await;
        
        let failures: Vec<Story> = stories.iter()
            .filter(|story| {
                matches!(story.verification, StoryOutcome::Failure { .. }) &&
                story.execution.iter().any(|e| {
                    matches!(e.event_type, crate::story::StoryEventType::ErrorOccurred) ||
                    e.description.contains(error)
                })
            })
            .cloned()
            .collect();
        
        Ok(failures)
    }
    
    /// Find patterns in stories
    async fn find_patterns(&self, pattern: &str) -> SemanticResult<Vec<Story>> {
        let stories = self.stories.read().await;
        
        // Simple pattern matching - in production would use more sophisticated analysis
        let patterns: Vec<Story> = stories.iter()
            .filter(|story| {
                // Check if events follow a pattern
                let event_pattern = story.execution.iter()
                    .map(|e| format!("{:?}", e.event_type))
                    .collect::<Vec<_>>()
                    .join("->");
                
                event_pattern.contains(pattern)
            })
            .cloned()
            .collect();
        
        Ok(patterns)
    }
    
    /// Parse natural language query into intent
    fn parse_query_intent(&self, query: &str) -> QueryIntent {
        let query_lower = query.to_lowercase();
        
        if query_lower.starts_with("how to") || query_lower.starts_with("how do") {
            QueryIntent::HowTo(query.to_string())
        } else if query_lower.starts_with("what happened") {
            QueryIntent::WhatHappened(query.to_string())
        } else if query_lower.starts_with("who can") || query_lower.starts_with("which agent") {
            QueryIntent::WhoCan(query.to_string())
        } else if query_lower.contains("error") || query_lower.contains("fail") {
            QueryIntent::WhatWentWrong(query.to_string())
        } else {
            QueryIntent::ShowPattern(query.to_string())
        }
    }
    
    /// Build a response from stories
    async fn build_response(&self, stories: Vec<Story>, query: &str) -> QueryResponse {
        QueryResponse {
            query: query.to_string(),
            story_count: stories.len(),
            stories: stories.clone(),
            summary: self.generate_summary(&stories),
            suggestions: self.generate_suggestions(&stories),
            confidence: self.calculate_confidence(&stories),
        }
    }
    
    /// Generate a summary of stories
    fn generate_summary(&self, stories: &[Story]) -> String {
        if stories.is_empty() {
            return "No stories found matching your query.".to_string();
        }
        
        let success_count = stories.iter().filter(|s| s.is_successful()).count();
        let total = stories.len();
        
        format!(
            "Found {} stories ({} successful, {} failed) demonstrating the requested pattern.",
            total,
            success_count,
            total - success_count
        )
    }
    
    /// Generate follow-up suggestions
    fn generate_suggestions(&self, stories: &[Story]) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        if stories.is_empty() {
            suggestions.push("Try a different query or broader search terms".to_string());
        } else {
            // Suggest exploring related themes
            let themes: Vec<String> = stories.iter()
                .flat_map(|s| s.context.themes.clone())
                .collect();
            
            if !themes.is_empty() {
                suggestions.push(format!("Explore related themes: {}", themes.join(", ")));
            }
            
            // Suggest looking at specific agents
            let agents: Vec<uuid::Uuid> = stories.iter()
                .flat_map(|s| s.participants())
                .collect();
            
            if agents.len() > 1 {
                suggestions.push("Query specific agent experiences for more detail".to_string());
            }
        }
        
        suggestions
    }
    
    /// Calculate confidence in results
    fn calculate_confidence(&self, stories: &[Story]) -> f64 {
        if stories.is_empty() {
            return 0.0;
        }
        
        let success_rate = stories.iter()
            .filter(|s| s.is_successful())
            .count() as f64 / stories.len() as f64;
        
        // Confidence based on success rate and count
        let count_factor = (stories.len() as f64 / 10.0).min(1.0);
        success_rate * count_factor
    }
    
    /// Simple text similarity (Levenshtein-like)
    fn text_similarity(&self, text1: &str, text2: &str) -> f64 {
        let words1: Vec<&str> = text1.split_whitespace().collect();
        let words2: Vec<&str> = text2.split_whitespace().collect();
        
        let mut matches = 0;
        for word1 in &words1 {
            if words2.iter().any(|w| w.contains(word1) || word1.contains(w)) {
                matches += 1;
            }
        }
        
        if words1.is_empty() || words2.is_empty() {
            0.0
        } else {
            matches as f64 / words1.len().max(words2.len()) as f64
        }
    }
}

/// Query intent types
#[derive(Debug, Clone)]
enum QueryIntent {
    HowTo(String),
    WhatHappened(String),
    WhoCan(String),
    WhatWentWrong(String),
    ShowPattern(String),
}

/// Response to a story query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    /// Original query
    pub query: String,
    
    /// Number of stories found
    pub story_count: usize,
    
    /// The stories themselves
    pub stories: Vec<Story>,
    
    /// Human-readable summary
    pub summary: String,
    
    /// Suggested follow-up queries
    pub suggestions: Vec<String>,
    
    /// Confidence in the results
    pub confidence: f64,
}

/// Cache for query results
struct QueryCache {
    entries: HashMap<String, QueryResponse>,
    max_size: usize,
}

impl QueryCache {
    fn new(max_size: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_size,
        }
    }
    
    fn get(&self, key: &str) -> Option<QueryResponse> {
        self.entries.get(key).cloned()
    }
    
    fn put(&mut self, key: String, response: QueryResponse) {
        if self.entries.len() >= self.max_size {
            // Simple eviction - remove first entry
            if let Some(first_key) = self.entries.keys().next().cloned() {
                self.entries.remove(&first_key);
            }
        }
        self.entries.insert(key, response);
    }
}

/// Configuration for the query engine
#[derive(Debug, Clone)]
pub struct QueryConfig {
    /// Maximum results to return
    pub max_results: usize,
    
    /// Cache size
    pub cache_size: usize,
    
    /// Minimum similarity threshold
    pub similarity_threshold: f64,
    
    /// Minimum trust score for agents
    pub min_trust_score: f64,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            cache_size: 100,
            similarity_threshold: 0.3,
            min_trust_score: 0.5,
        }
    }
}

/// Interface for AI agents to discover capabilities
pub struct AIDiscoveryInterface {
    query_engine: Arc<StoryQueryEngine>,
}

impl AIDiscoveryInterface {
    pub fn new(query_engine: Arc<StoryQueryEngine>) -> Self {
        Self { query_engine }
    }
    
    /// Ask "What can you do?"
    pub async fn discover_capabilities(&self) -> QueryResponse {
        self.query_engine.query_natural("show all successful patterns").await
            .unwrap_or_else(|_| QueryResponse {
                query: "discover capabilities".to_string(),
                story_count: 0,
                stories: Vec::new(),
                summary: "No capabilities discovered yet".to_string(),
                suggestions: vec!["Start by declaring an intent".to_string()],
                confidence: 0.0,
            })
    }
    
    /// Ask "How do I X?"
    pub async fn learn_how_to(&self, task: &str) -> QueryResponse {
        let query = format!("how to {}", task);
        self.query_engine.query_natural(&query).await
            .unwrap_or_else(|_| QueryResponse {
                query: query.clone(),
                story_count: 0,
                stories: Vec::new(),
                summary: format!("No examples found for '{}'", task),
                suggestions: vec!["Try a simpler task".to_string()],
                confidence: 0.0,
            })
    }
    
    /// Ask "What went wrong?"
    pub async fn diagnose_failure(&self, context: &str) -> QueryResponse {
        let query = format!("what went wrong with {}", context);
        self.query_engine.query_natural(&query).await
            .unwrap_or_else(|_| QueryResponse {
                query: query.clone(),
                story_count: 0,
                stories: Vec::new(),
                summary: "No failures found in that context".to_string(),
                suggestions: vec!["Check the logs".to_string()],
                confidence: 0.0,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Intent;
    
    #[tokio::test]
    async fn test_story_query() {
        let engine = StoryQueryEngine::new(QueryConfig::default());
        
        // Add a test story
        let story = Story::begin(Intent::new("test task"));
        engine.add_stories(vec![story]).await;
        
        // Query for it
        let response = engine.query_natural("how to test").await.unwrap();
        assert!(response.story_count > 0);
    }
}