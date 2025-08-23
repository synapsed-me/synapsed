//! DID resolver implementation
//! 
//! This module provides W3C DID Core v1.0 compliant resolution with:
//! - Universal resolver interface
//! - Caching and performance optimization
//! - Error handling and metadata
//! - Support for multiple DID methods

use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{Result, Error};
use super::{Did, DidDocument, DidMethod, DidKey, DidWeb, DidResolutionOptions, DidResolutionMetadata};

/// Universal DID resolver
pub struct DidResolver {
    /// Registered DID methods
    methods: HashMap<String, Box<dyn DidMethod + Send + Sync>>,
    /// Resolution cache
    cache: ResolutionCache,
    /// Resolver configuration
    config: ResolverConfig,
}

impl DidResolver {
    /// Create a new DID resolver
    pub fn new(config: ResolverConfig) -> Self {
        let mut resolver = Self {
            methods: HashMap::new(),
            cache: ResolutionCache::new(config.cache_size, config.cache_ttl),
            config,
        };

        // Register default methods
        resolver.register_method("key", Box::new(DidKey::new()));
        resolver.register_method("web", Box::new(DidWeb::new()));

        resolver
    }

    /// Register a DID method
    pub fn register_method(&mut self, method_name: &str, method: Box<dyn DidMethod + Send + Sync>) {
        self.methods.insert(method_name.to_string(), method);
    }

    /// Resolve a DID to its document
    pub async fn resolve(&mut self, did: &Did, options: DidResolutionOptions) -> Result<ResolutionResult> {
        // Check cache first
        let cache_key = self.create_cache_key(did, &options);
        if let Some(cached_result) = self.cache.get(&cache_key) {
            if !cached_result.is_expired() {
                return Ok(cached_result.result.clone());
            }
        }

        // Find appropriate method
        let method = self.methods.get(&did.method)
            .ok_or_else(|| Error::DidResolutionError(format!("Unsupported DID method: {}", did.method)))?;

        let start_time = Instant::now();
        
        // Attempt resolution
        let result = match method.resolve(did) {
            Ok(Some(document)) => {
                // Validate document
                document.validate()?;
                
                ResolutionResult {
                    document: Some(document),
                    metadata: DidResolutionMetadata {
                        content_type: Some("application/did+json".to_string()),
                        error: None,
                        additional: HashMap::new(),
                    },
                    document_metadata: Some(super::DidMetadata::default()),
                }
            }
            Ok(None) => {
                ResolutionResult {
                    document: None,
                    metadata: DidResolutionMetadata {
                        content_type: None,
                        error: Some("notFound".to_string()),
                        additional: HashMap::new(),
                    },
                    document_metadata: None,
                }
            }
            Err(e) => {
                ResolutionResult {
                    document: None,
                    metadata: DidResolutionMetadata {
                        content_type: None,
                        error: Some("internalError".to_string()),
                        additional: {
                            let mut map = HashMap::new();
                            map.insert("error_details".to_string(), serde_json::Value::String(e.to_string()));
                            map
                        },
                    },
                    document_metadata: None,
                }
            }
        };

        let resolution_time = start_time.elapsed();

        // Cache result if successful
        if result.document.is_some() {
            self.cache.insert(cache_key, CachedResult {
                result: result.clone(),
                cached_at: Instant::now(),
                resolution_time,
            });
        }

        Ok(result)
    }

    /// Create cache key
    fn create_cache_key(&self, did: &Did, options: &DidResolutionOptions) -> String {
        use sha3::{Sha3_256, Digest};
        
        let mut hasher = Sha3_256::new();
        hasher.update(did.to_string().as_bytes());
        
        // Include options in cache key
        if let Some(ref accept) = options.accept {
            hasher.update(accept.as_bytes());
        }
        if let Some(ref service) = options.service {
            hasher.update(service.as_bytes());
        }
        if let Some(ref relative_ref) = options.relative_ref {
            hasher.update(relative_ref.as_bytes());
        }
        
        format!("{:x}", hasher.finalize())
    }

    /// Get resolver statistics
    pub fn get_stats(&self) -> ResolverStats {
        ResolverStats {
            cache_size: self.cache.len(),
            cache_hit_rate: self.cache.hit_rate(),
            supported_methods: self.methods.keys().cloned().collect(),
        }
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for DidResolver {
    fn default() -> Self {
        Self::new(ResolverConfig::default())
    }
}

/// Resolution cache for performance optimization
pub struct ResolutionCache {
    cache: HashMap<String, CachedResult>,
    max_size: usize,
    ttl: Duration,
    hits: u64,
    misses: u64,
}

impl ResolutionCache {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            ttl,
            hits: 0,
            misses: 0,
        }
    }

    pub fn get(&mut self, key: &str) -> Option<&CachedResult> {
        // Check if key exists and is not expired
        let should_remove = self.cache.get(key).map(|r| r.is_expired()).unwrap_or(false);
        
        if should_remove {
            self.cache.remove(key);
            self.misses += 1;
            return None;
        }
        
        if let Some(result) = self.cache.get(key) {
            self.hits += 1;
            Some(result)
        } else {
            self.misses += 1;
            None
        }
    }

    pub fn insert(&mut self, key: String, result: CachedResult) {
        // Evict oldest entries if cache is full
        if self.cache.len() >= self.max_size {
            self.evict_oldest();
        }
        self.cache.insert(key, result);
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.hits = 0;
        self.misses = 0;
    }

    fn evict_oldest(&mut self) {
        if let Some((key, _)) = self.cache.iter()
            .min_by_key(|(_, result)| result.cached_at)
            .map(|(k, v)| (k.clone(), v.cached_at)) {
            self.cache.remove(&key);
        }
    }
}

/// Cached resolution result
#[derive(Clone)]
pub struct CachedResult {
    pub result: ResolutionResult,
    pub cached_at: Instant,
    pub resolution_time: Duration,
}

impl CachedResult {
    pub fn is_expired(&self) -> bool {
        // For now, use a simple TTL approach
        // In practice, might want to check document metadata for expiration
        self.cached_at.elapsed() > Duration::from_secs(300) // 5 minutes default
    }
}

/// DID resolution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    /// The resolved DID document
    #[serde(rename = "didDocument")]
    pub document: Option<DidDocument>,
    
    /// Resolution metadata
    #[serde(rename = "didResolutionMetadata")]
    pub metadata: DidResolutionMetadata,
    
    /// Document metadata
    #[serde(rename = "didDocumentMetadata")]
    pub document_metadata: Option<super::DidMetadata>,
}

/// Resolver configuration
#[derive(Debug, Clone)]
pub struct ResolverConfig {
    /// Maximum cache size
    pub cache_size: usize,
    
    /// Cache TTL
    pub cache_ttl: Duration,
    
    /// HTTP timeout for web-based resolution
    pub http_timeout: Duration,
    
    /// Enable/disable caching
    pub caching_enabled: bool,
    
    /// Custom HTTP headers for resolution
    pub custom_headers: HashMap<String, String>,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            cache_size: 1000,
            cache_ttl: Duration::from_secs(300), // 5 minutes
            http_timeout: Duration::from_secs(30),
            caching_enabled: true,
            custom_headers: HashMap::new(),
        }
    }
}

/// Resolver statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolverStats {
    pub cache_size: usize,
    pub cache_hit_rate: f64,
    pub supported_methods: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_creation() {
        let resolver = DidResolver::default();
        let stats = resolver.get_stats();
        
        assert!(stats.supported_methods.contains(&"key".to_string()));
        assert!(stats.supported_methods.contains(&"web".to_string()));
    }

    #[tokio::test]
    async fn test_did_key_resolution() {
        let mut resolver = DidResolver::default();
        let did = Did::parse("did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK").unwrap();
        
        let result = resolver.resolve(&did, DidResolutionOptions::default()).await.unwrap();
        
        assert!(result.document.is_some());
        assert!(result.metadata.error.is_none());
        
        let document = result.document.unwrap();
        assert_eq!(document.id, did);
        assert!(!document.verification_method.is_empty());
    }
}