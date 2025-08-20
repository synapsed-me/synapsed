//! Dictionary management for compression algorithms

use crate::compression::engine::{CompressionResult, CompressionError};
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Compression dictionary with metadata
#[derive(Debug, Clone)]
pub struct Dictionary {
    pub id: String,
    pub data: Bytes,
    pub created_at: u64,
    pub usage_count: u64,
    pub effectiveness_score: f32,
    pub size: usize,
}

impl Dictionary {
    pub fn new(id: String, data: Bytes) -> Self {
        let size = data.len();
        Self {
            id,
            data,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            usage_count: 0,
            effectiveness_score: 0.0,
            size,
        }
    }
    
    pub fn update_effectiveness(&mut self, compression_improvement: f32) {
        let count = self.usage_count as f32;
        let new_count = count + 1.0;
        
        // Update moving average of effectiveness
        self.effectiveness_score = (self.effectiveness_score * count + compression_improvement) / new_count;
        self.usage_count += 1;
    }
    
    pub fn age_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(self.created_at)
    }
}

/// Dictionary training configuration
#[derive(Debug, Clone)]
pub struct DictionaryConfig {
    pub max_dictionaries: usize,
    pub max_dictionary_size: usize,
    pub min_training_samples: usize,
    pub max_age_seconds: u64,
    pub min_effectiveness_score: f32,
}

impl Default for DictionaryConfig {
    fn default() -> Self {
        Self {
            max_dictionaries: 10,
            max_dictionary_size: 64 * 1024, // 64KB
            min_training_samples: 100,
            max_age_seconds: 24 * 60 * 60, // 24 hours
            min_effectiveness_score: 0.1,
        }
    }
}

/// Dictionary manager for training and managing compression dictionaries
#[derive(Debug)]
pub struct DictionaryManager {
    config: DictionaryConfig,
    dictionaries: Arc<RwLock<HashMap<String, Dictionary>>>,
    training_samples: Arc<RwLock<Vec<Bytes>>>,
}

impl DictionaryManager {
    pub fn new(config: DictionaryConfig) -> Self {
        Self {
            config,
            dictionaries: Arc::new(RwLock::new(HashMap::new())),
            training_samples: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Add training sample for dictionary creation
    pub fn add_training_sample(&self, data: Bytes) -> CompressionResult<()> {
        let mut samples = self.training_samples.write().map_err(|_| {
            CompressionError::DictionaryError {
                reason: "Failed to acquire training samples lock".to_string(),
            }
        })?;
        
        samples.push(data);
        
        // Limit number of samples to prevent memory bloat
        if samples.len() > self.config.min_training_samples * 2 {
            samples.remove(0);
        }
        
        Ok(())
    }
    
    /// Train a new dictionary from collected samples
    pub fn train_dictionary(&self, id: String) -> CompressionResult<()> {
        // Implementation will be added in GREEN phase
        todo!("Dictionary training not implemented yet")
    }
    
    /// Get dictionary by ID
    pub fn get_dictionary(&self, id: &str) -> Option<Dictionary> {
        self.dictionaries.read().ok()?.get(id).cloned()
    }
    
    /// List all available dictionaries
    pub fn list_dictionaries(&self) -> Vec<String> {
        self.dictionaries
            .read()
            .map(|dicts| dicts.keys().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Remove old or ineffective dictionaries
    pub fn cleanup_dictionaries(&self) -> CompressionResult<usize> {
        // Implementation will be added in GREEN phase
        todo!("Dictionary cleanup not implemented yet")
    }
    
    /// Update dictionary effectiveness based on compression results
    pub fn update_dictionary_effectiveness(&self, id: &str, improvement: f32) -> CompressionResult<()> {
        let mut dictionaries = self.dictionaries.write().map_err(|_| {
            CompressionError::DictionaryError {
                reason: "Failed to acquire dictionaries lock".to_string(),
            }
        })?;
        
        if let Some(dictionary) = dictionaries.get_mut(id) {
            dictionary.update_effectiveness(improvement);
            Ok(())
        } else {
            Err(CompressionError::DictionaryError {
                reason: format!("Dictionary not found: {}", id),
            })
        }
    }
    
    /// Get the best dictionary for given data characteristics
    pub fn select_best_dictionary(&self, data: &[u8]) -> Option<String> {
        let dictionaries = self.dictionaries.read().ok()?;
        
        if dictionaries.is_empty() {
            return None;
        }
        
        // Simple heuristic: select dictionary with highest effectiveness score
        // In a real implementation, you'd analyze data characteristics
        let mut best_dict = None;
        let mut best_score = 0.0;
        
        for (id, dict) in dictionaries.iter() {
            if dict.usage_count > 0 && dict.effectiveness_score > best_score {
                best_score = dict.effectiveness_score;
                best_dict = Some(id.clone());
            }
        }
        
        best_dict
    }
    
    /// Get total memory usage of all dictionaries
    pub fn memory_usage(&self) -> usize {
        self.dictionaries
            .read()
            .map(|dicts| dicts.values().map(|d| d.size).sum())
            .unwrap_or(0)
    }
    
    /// Export dictionary for external use
    pub fn export_dictionary(&self, id: &str) -> CompressionResult<Bytes> {
        let dictionaries = self.dictionaries.read().map_err(|_| {
            CompressionError::DictionaryError {
                reason: "Failed to acquire dictionaries lock".to_string(),
            }
        })?;
        
        dictionaries.get(id)
            .map(|dict| dict.data.clone())
            .ok_or_else(|| CompressionError::DictionaryError {
                reason: format!("Dictionary not found: {}", id),
            })
    }
    
    /// Import dictionary from external source
    pub fn import_dictionary(&self, id: String, data: Bytes) -> CompressionResult<()> {
        if data.len() > self.config.max_dictionary_size {
            return Err(CompressionError::DictionaryError {
                reason: format!(
                    "Dictionary too large: {} > {}",
                    data.len(),
                    self.config.max_dictionary_size
                ),
            });
        }
        
        let dictionary = Dictionary::new(id.clone(), data);
        
        let mut dictionaries = self.dictionaries.write().map_err(|_| {
            CompressionError::DictionaryError {
                reason: "Failed to acquire dictionaries lock".to_string(),
            }
        })?;
        
        dictionaries.insert(id, dictionary);
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary_creation() {
        let data = Bytes::from("test dictionary data");
        let dict = Dictionary::new("test_dict".to_string(), data.clone());
        
        assert_eq!(dict.id, "test_dict");
        assert_eq!(dict.data, data);
        assert_eq!(dict.usage_count, 0);
        assert_eq!(dict.effectiveness_score, 0.0);
        assert_eq!(dict.size, data.len());
        assert!(dict.created_at > 0);
    }

    #[test]
    fn test_dictionary_effectiveness_update() {
        let data = Bytes::from("test dictionary data");
        let mut dict = Dictionary::new("test_dict".to_string(), data);
        
        dict.update_effectiveness(0.5);
        assert_eq!(dict.usage_count, 1);
        assert_eq!(dict.effectiveness_score, 0.5);
        
        dict.update_effectiveness(0.3);
        assert_eq!(dict.usage_count, 2);
        assert_eq!(dict.effectiveness_score, 0.4); // (0.5 + 0.3) / 2
    }

    #[test]
    fn test_dictionary_age() {
        let data = Bytes::from("test dictionary data");
        let dict = Dictionary::new("test_dict".to_string(), data);
        
        // Age should be very small for newly created dictionary
        let age = dict.age_seconds();
        assert!(age < 5); // Should be less than 5 seconds old
    }

    #[test]
    fn test_dictionary_config_default() {
        let config = DictionaryConfig::default();
        assert_eq!(config.max_dictionaries, 10);
        assert_eq!(config.max_dictionary_size, 64 * 1024);
        assert_eq!(config.min_training_samples, 100);
        assert_eq!(config.max_age_seconds, 24 * 60 * 60);
        assert_eq!(config.min_effectiveness_score, 0.1);
    }

    #[test]
    fn test_dictionary_manager_creation() {
        let config = DictionaryConfig::default();
        let manager = DictionaryManager::new(config);
        
        // Should be able to create manager without issues
        assert_eq!(manager.config.max_dictionaries, 10);
    }

    #[test]
    #[should_panic(expected = "Dictionary training sample addition not implemented yet")]
    fn test_add_training_sample_not_implemented() {
        let manager = DictionaryManager::new(DictionaryConfig::default());
        let data = Bytes::from("training sample");
        let _result = manager.add_training_sample(data);
    }

    #[test]
    #[should_panic(expected = "Dictionary training not implemented yet")]
    fn test_train_dictionary_not_implemented() {
        let manager = DictionaryManager::new(DictionaryConfig::default());
        let _result = manager.train_dictionary("test_dict".to_string());
    }

    #[test]
    #[should_panic(expected = "Dictionary retrieval not implemented yet")]
    fn test_get_dictionary_not_implemented() {
        let manager = DictionaryManager::new(DictionaryConfig::default());
        let _result = manager.get_dictionary("test_dict");
    }

    #[test]
    #[should_panic(expected = "Dictionary listing not implemented yet")]
    fn test_list_dictionaries_not_implemented() {
        let manager = DictionaryManager::new(DictionaryConfig::default());
        let _result = manager.list_dictionaries();
    }

    #[test]
    #[should_panic(expected = "Dictionary cleanup not implemented yet")]
    fn test_cleanup_dictionaries_not_implemented() {
        let manager = DictionaryManager::new(DictionaryConfig::default());
        let _result = manager.cleanup_dictionaries();
    }

    #[test]
    #[should_panic(expected = "Dictionary effectiveness update not implemented yet")]
    fn test_update_effectiveness_not_implemented() {
        let manager = DictionaryManager::new(DictionaryConfig::default());
        let _result = manager.update_dictionary_effectiveness("test_dict", 0.5);
    }

    #[test]
    #[should_panic(expected = "Dictionary selection not implemented yet")]
    fn test_select_best_dictionary_not_implemented() {
        let manager = DictionaryManager::new(DictionaryConfig::default());
        let _result = manager.select_best_dictionary(b"test data");
    }

    #[test]
    #[should_panic(expected = "Memory usage calculation not implemented yet")]
    fn test_memory_usage_not_implemented() {
        let manager = DictionaryManager::new(DictionaryConfig::default());
        let _result = manager.memory_usage();
    }

    #[test]
    #[should_panic(expected = "Dictionary export not implemented yet")]
    fn test_export_dictionary_not_implemented() {
        let manager = DictionaryManager::new(DictionaryConfig::default());
        let _result = manager.export_dictionary("test_dict");
    }

    #[test]
    #[should_panic(expected = "Dictionary import not implemented yet")]
    fn test_import_dictionary_not_implemented() {
        let manager = DictionaryManager::new(DictionaryConfig::default());
        let data = Bytes::from("dictionary data");
        let _result = manager.import_dictionary("test_dict".to_string(), data);
    }
}