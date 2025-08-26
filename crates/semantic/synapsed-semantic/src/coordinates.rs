//! Semantic coordinate system for positioning agents in spacetime

use serde::{Deserialize, Serialize};
use nalgebra::{Vector4, distance};

/// Coordinates in semantic spacetime
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SemanticCoords {
    /// Intent dimension - how purposeful/goal-oriented
    pub intent: f64,
    
    /// Promise dimension - how reliable/trustworthy
    pub promise: f64,
    
    /// Context dimension - how context-dependent
    pub context: f64,
    
    /// Expression dimension - how it manifests intentions
    pub expression: f64,
}

impl SemanticCoords {
    /// Create new semantic coordinates
    pub fn new(intent: f64, promise: f64, context: f64, expression: f64) -> Self {
        Self {
            intent: intent.clamp(0.0, 1.0),
            promise: promise.clamp(0.0, 1.0),
            context: context.clamp(0.0, 1.0),
            expression: expression.clamp(0.0, 1.0),
        }
    }
    
    /// Calculate Euclidean distance to another point
    pub fn distance_to(&self, other: &Self) -> f64 {
        let v1 = Vector4::new(self.intent, self.promise, self.context, self.expression);
        let v2 = Vector4::new(other.intent, other.promise, other.context, other.expression);
        distance(&v1, &v2)
    }
    
    /// Calculate semantic similarity (inverse of distance)
    pub fn similarity_to(&self, other: &Self) -> f64 {
        let max_distance = 2.0_f64.sqrt(); // Max distance in 4D unit hypercube
        1.0 - (self.distance_to(other) / max_distance)
    }
    
    /// Move towards another point by a factor
    pub fn move_towards(&mut self, target: &Self, factor: f64) {
        let factor = factor.clamp(0.0, 1.0);
        self.intent += (target.intent - self.intent) * factor;
        self.promise += (target.promise - self.promise) * factor;
        self.context += (target.context - self.context) * factor;
        self.expression += (target.expression - self.expression) * factor;
        
        // Ensure coordinates stay in valid range
        self.intent = self.intent.clamp(0.0, 1.0);
        self.promise = self.promise.clamp(0.0, 1.0);
        self.context = self.context.clamp(0.0, 1.0);
        self.expression = self.expression.clamp(0.0, 1.0);
    }
    
    /// Get the center of mass between multiple coordinates
    pub fn center_of_mass(coords: &[Self]) -> Self {
        if coords.is_empty() {
            return Self::new(0.5, 0.5, 0.5, 0.5);
        }
        
        let sum = coords.iter().fold(
            (0.0, 0.0, 0.0, 0.0),
            |acc, c| (
                acc.0 + c.intent,
                acc.1 + c.promise,
                acc.2 + c.context,
                acc.3 + c.expression,
            )
        );
        
        let count = coords.len() as f64;
        Self::new(
            sum.0 / count,
            sum.1 / count,
            sum.2 / count,
            sum.3 / count,
        )
    }
}

impl Default for SemanticCoords {
    fn default() -> Self {
        Self::new(0.5, 0.5, 0.5, 0.5)
    }
}

/// A position in semantic spacetime with additional context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticPosition {
    /// The coordinates in semantic space
    pub coords: SemanticCoords,
    
    /// Current narrative chapter/context
    pub chapter: String,
    
    /// Active themes at this position
    pub themes: Vec<String>,
    
    /// Timestamp of this position
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Confidence in this position (0-1)
    pub confidence: f64,
}

impl SemanticPosition {
    /// Create a new semantic position
    pub fn new(coords: SemanticCoords, chapter: String, themes: Vec<String>) -> Self {
        Self {
            coords,
            chapter,
            themes,
            timestamp: chrono::Utc::now(),
            confidence: 1.0,
        }
    }
    
    /// Calculate contextual distance (considers themes and chapter)
    pub fn contextual_distance(&self, other: &Self) -> f64 {
        let coord_distance = self.coords.distance_to(&other.coords);
        
        // Chapter difference adds distance
        let chapter_distance = if self.chapter == other.chapter {
            0.0
        } else {
            0.2
        };
        
        // Theme overlap reduces distance
        let common_themes = self.themes.iter()
            .filter(|t| other.themes.contains(t))
            .count();
        let total_themes = (self.themes.len() + other.themes.len()) as f64 / 2.0;
        let theme_similarity = if total_themes > 0.0 {
            common_themes as f64 / total_themes
        } else {
            0.0
        };
        
        coord_distance + chapter_distance - (theme_similarity * 0.1)
    }
}

/// Semantic distance calculator with different metrics
#[derive(Debug, Clone, Copy)]
pub enum SemanticDistance {
    /// Euclidean distance in semantic space
    Euclidean,
    /// Manhattan distance (sum of absolute differences)
    Manhattan,
    /// Cosine similarity based distance
    Cosine,
    /// Custom weighted distance
    Weighted { intent: f64, promise: f64, context: f64, expression: f64 },
}

impl SemanticDistance {
    /// Calculate distance between two positions using the selected metric
    pub fn calculate(&self, from: &SemanticCoords, to: &SemanticCoords) -> f64 {
        match self {
            Self::Euclidean => from.distance_to(to),
            
            Self::Manhattan => {
                (from.intent - to.intent).abs() +
                (from.promise - to.promise).abs() +
                (from.context - to.context).abs() +
                (from.expression - to.expression).abs()
            }
            
            Self::Cosine => {
                let v1 = Vector4::new(from.intent, from.promise, from.context, from.expression);
                let v2 = Vector4::new(to.intent, to.promise, to.context, to.expression);
                
                let dot_product = v1.dot(&v2);
                let magnitude1 = v1.magnitude();
                let magnitude2 = v2.magnitude();
                
                if magnitude1 == 0.0 || magnitude2 == 0.0 {
                    1.0
                } else {
                    1.0 - (dot_product / (magnitude1 * magnitude2))
                }
            }
            
            Self::Weighted { intent, promise, context, expression } => {
                let di = (from.intent - to.intent).abs() * intent;
                let dp = (from.promise - to.promise).abs() * promise;
                let dc = (from.context - to.context).abs() * context;
                let de = (from.expression - to.expression).abs() * expression;
                
                (di * di + dp * dp + dc * dc + de * de).sqrt()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_semantic_coords_distance() {
        let c1 = SemanticCoords::new(0.0, 0.0, 0.0, 0.0);
        let c2 = SemanticCoords::new(1.0, 1.0, 1.0, 1.0);
        
        let distance = c1.distance_to(&c2);
        assert!((distance - 2.0).abs() < 0.001); // sqrt(4) = 2
    }
    
    #[test]
    fn test_semantic_similarity() {
        let c1 = SemanticCoords::new(0.5, 0.5, 0.5, 0.5);
        let c2 = SemanticCoords::new(0.5, 0.5, 0.5, 0.5);
        
        assert_eq!(c1.similarity_to(&c2), 1.0); // Identical points
    }
    
    #[test]
    fn test_move_towards() {
        let mut c1 = SemanticCoords::new(0.0, 0.0, 0.0, 0.0);
        let c2 = SemanticCoords::new(1.0, 1.0, 1.0, 1.0);
        
        c1.move_towards(&c2, 0.5);
        
        assert!((c1.intent - 0.5).abs() < 0.001);
        assert!((c1.promise - 0.5).abs() < 0.001);
        assert!((c1.context - 0.5).abs() < 0.001);
        assert!((c1.expression - 0.5).abs() < 0.001);
    }
}