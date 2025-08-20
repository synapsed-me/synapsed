//! Utility functions and helpers for the Synapsed ecosystem.
//!
//! This module provides common utility functions that can be used across
//! all Synapsed components.

use crate::{SynapsedError, SynapsedResult};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Time-related utilities
pub mod time {
    use super::{SystemTime, UNIX_EPOCH, Duration};

    /// Get current timestamp as milliseconds since Unix epoch
    #[must_use] pub fn current_timestamp_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Get current timestamp as seconds since Unix epoch
    #[must_use] pub fn current_timestamp_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Convert milliseconds to Duration
    #[must_use] pub fn millis_to_duration(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// Convert seconds to Duration
    #[must_use] pub fn secs_to_duration(secs: u64) -> Duration {
        Duration::from_secs(secs)
    }

    /// Format duration as human-readable string
    #[must_use] pub fn format_duration(duration: Duration) -> String {
        let total_secs = duration.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        let millis = duration.subsec_millis();

        if hours > 0 {
            format!("{hours}h {minutes}m {seconds}s")
        } else if minutes > 0 {
            format!("{minutes}m {seconds}s")
        } else if seconds > 0 {
            format!("{seconds}.{millis:03}s")
        } else {
            format!("{}ms", duration.as_millis())
        }
    }

    /// Check if a timestamp is expired given a TTL
    #[must_use] pub fn is_expired(timestamp_millis: u64, ttl_millis: u64) -> bool {
        let now = current_timestamp_millis();
        now > timestamp_millis + ttl_millis
    }

    /// Get time until expiration (returns None if already expired)
    #[must_use] pub fn time_until_expiration(timestamp_millis: u64, ttl_millis: u64) -> Option<Duration> {
        let now = current_timestamp_millis();
        let expiry = timestamp_millis + ttl_millis;
        
        if now >= expiry {
            None
        } else {
            Some(Duration::from_millis(expiry - now))
        }
    }
}

/// String utilities
pub mod string {
    use super::{Hash, Hasher};

    /// Check if a string is empty or contains only whitespace
    #[must_use] pub fn is_blank(s: &str) -> bool {
        s.trim().is_empty()
    }

    /// Truncate a string to a maximum length, adding ellipsis if truncated
    #[must_use] pub fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else if max_len <= 3 {
            "...".to_string()
        } else {
            format!("{}...", &s[..max_len - 3])
        }
    }

    /// Convert string to title case
    #[must_use] pub fn to_title_case(s: &str) -> String {
        s.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Convert string to `snake_case`
    #[must_use] pub fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        let mut prev_was_upper = false;
        
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() {
                if i > 0 && !prev_was_upper {
                    result.push('_');
                }
                result.push(c.to_lowercase().next().unwrap());
                prev_was_upper = true;
            } else {
                result.push(c);
                prev_was_upper = false;
            }
        }
        
        result
    }

    /// Convert string to kebab-case
    #[must_use] pub fn to_kebab_case(s: &str) -> String {
        to_snake_case(s).replace('_', "-")
    }

    /// Generate a random alphanumeric string of given length
    #[must_use] pub fn random_alphanumeric(length: usize) -> String {
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        let timestamp = crate::utils::time::current_timestamp_millis();
        timestamp.hash(&mut hasher);
        
        let hash = hasher.finish();
        let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        
        (0..length)
            .map(|i| {
                let idx = ((hash >> (i % 8 * 8)) as usize) % chars.len();
                chars.chars().nth(idx).unwrap()
            })
            .collect()
    }

    /// Sanitize a string for use as filename
    #[must_use] pub fn sanitize_filename(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                c if c.is_control() => '_',
                c => c,
            })
            .collect()
    }
}

/// Collection utilities
pub mod collections {
    use super::{Hash, HashMap};

    /// Merge two `HashMaps`, with values from the second map taking precedence
    #[must_use] pub fn merge_hashmaps<K, V>(mut base: HashMap<K, V>, overlay: HashMap<K, V>) -> HashMap<K, V>
    where
        K: Eq + Hash,
    {
        for (key, value) in overlay {
            base.insert(key, value);
        }
        base
    }

    /// Group items by a key function
    pub fn group_by<T, K, F>(items: Vec<T>, key_fn: F) -> HashMap<K, Vec<T>>
    where
        K: Eq + Hash,
        F: Fn(&T) -> K,
    {
        let mut groups: HashMap<K, Vec<T>> = HashMap::new();
        
        for item in items {
            let key = key_fn(&item);
            groups.entry(key).or_default().push(item);
        }
        
        groups
    }

    /// Count items by a key function
    pub fn count_by<T, K, F>(items: &[T], key_fn: F) -> HashMap<K, usize>
    where
        K: Eq + Hash,
        F: Fn(&T) -> K,
    {
        let mut counts: HashMap<K, usize> = HashMap::new();
        
        for item in items {
            let key = key_fn(item);
            *counts.entry(key).or_insert(0) += 1;
        }
        
        counts
    }

    /// Find duplicates in a collection
    pub fn find_duplicates<T, K, F>(items: &[T], key_fn: F) -> Vec<K>
    where
        T: Clone,
        K: Eq + Hash + Clone,
        F: Fn(&T) -> K,
    {
        let counts = count_by(items, key_fn);
        counts
            .into_iter()
            .filter_map(|(key, count)| if count > 1 { Some(key) } else { None })
            .collect()
    }

    /// Remove duplicates from a vector while preserving order
    #[must_use] pub fn dedup_preserve_order<T>(items: Vec<T>) -> Vec<T>
    where
        T: Eq + Hash + Clone,
    {
        let mut seen = std::collections::HashSet::new();
        items
            .into_iter()
            .filter(|item| seen.insert(item.clone()))
            .collect()
    }

    /// Partition a collection into two based on a predicate
    pub fn partition<T, F>(items: Vec<T>, predicate: F) -> (Vec<T>, Vec<T>)
    where
        F: Fn(&T) -> bool,
    {
        let mut true_items = Vec::new();
        let mut false_items = Vec::new();
        
        for item in items {
            if predicate(&item) {
                true_items.push(item);
            } else {
                false_items.push(item);
            }
        }
        
        (true_items, false_items)
    }
}

/// Math utilities
pub mod math {

    /// Calculate percentage of a value
    #[must_use] pub fn percentage(value: f64, total: f64) -> f64 {
        if total == 0.0 {
            0.0
        } else {
            (value / total) * 100.0
        }
    }

    /// Clamp a value between min and max
    pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    /// Linear interpolation between two values
    #[must_use] pub fn lerp(start: f64, end: f64, t: f64) -> f64 {
        start + (end - start) * clamp(t, 0.0, 1.0)
    }

    /// Calculate moving average
    #[must_use] pub fn moving_average(values: &[f64], window_size: usize) -> Vec<f64> {
        if values.is_empty() || window_size == 0 {
            return Vec::new();
        }

        let window_size = window_size.min(values.len());
        let mut result = Vec::new();

        for i in 0..=values.len() - window_size {
            let window = &values[i..i + window_size];
            let avg = window.iter().sum::<f64>() / window.len() as f64;
            result.push(avg);
        }

        result
    }

    /// Calculate standard deviation
    #[must_use] pub fn standard_deviation(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / values.len() as f64;
        
        variance.sqrt()
    }

    /// Round to specified number of decimal places
    #[must_use] pub fn round_to_decimal_places(value: f64, places: u32) -> f64 {
        let multiplier = 10_f64.powi(places as i32);
        (value * multiplier).round() / multiplier
    }
}

/// Validation utilities
pub mod validation {
    use super::{SynapsedResult, SynapsedError};

    /// Validate email address format (basic validation)
    #[must_use] pub fn is_valid_email(email: &str) -> bool {
        email.contains('@') && email.contains('.') && !email.starts_with('@') && !email.ends_with('@')
    }

    /// Validate URL format (basic validation)
    #[must_use] pub fn is_valid_url(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    /// Validate UUID format
    #[must_use] pub fn is_valid_uuid(uuid: &str) -> bool {
        uuid::Uuid::parse_str(uuid).is_ok()
    }

    /// Validate that a string is not empty after trimming
    pub fn is_not_empty(s: &str) -> SynapsedResult<()> {
        if s.trim().is_empty() {
            Err(SynapsedError::invalid_input("String cannot be empty"))
        } else {
            Ok(())
        }
    }

    /// Validate that a number is within a range
    pub fn is_in_range<T: PartialOrd>(value: T, min: T, max: T) -> SynapsedResult<()> {
        if value >= min && value <= max {
            Ok(())
        } else {
            Err(SynapsedError::invalid_input("Value is out of range"))
        }
    }

    /// Validate that a collection has a minimum number of items
    pub fn has_min_items<T>(items: &[T], min_count: usize) -> SynapsedResult<()> {
        if items.len() >= min_count {
            Ok(())
        } else {
            Err(SynapsedError::invalid_input(format!(
                "Collection must have at least {} items, got {}",
                min_count,
                items.len()
            )))
        }
    }

    /// Validate that a collection has a maximum number of items
    pub fn has_max_items<T>(items: &[T], max_count: usize) -> SynapsedResult<()> {
        if items.len() <= max_count {
            Ok(())
        } else {
            Err(SynapsedError::invalid_input(format!(
                "Collection must have at most {} items, got {}",
                max_count,
                items.len()
            )))
        }
    }
}

/// File system utilities
pub mod fs {
    use super::{SynapsedResult, SynapsedError};
    use std::path::{Path, PathBuf};

    /// Ensure a directory exists, creating it if necessary
    pub fn ensure_dir_exists<P: AsRef<Path>>(path: P) -> SynapsedResult<()> {
        let path = path.as_ref();
        if !path.exists() {
            std::fs::create_dir_all(path)
                .map_err(|e| SynapsedError::internal(format!("Failed to create directory: {e}")))?;
        }
        Ok(())
    }

    /// Get file extension from path
    pub fn get_file_extension<P: AsRef<Path>>(path: P) -> Option<String> {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_lowercase)
    }

    /// Get filename without extension
    pub fn get_filename_without_extension<P: AsRef<Path>>(path: P) -> Option<String> {
        path.as_ref()
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(std::string::ToString::to_string)
    }

    /// Join multiple path components
    pub fn join_paths<P: AsRef<Path>>(base: P, components: &[&str]) -> PathBuf {
        let mut path = base.as_ref().to_path_buf();
        for component in components {
            path.push(component);
        }
        path
    }

    /// Check if a path is safe (doesn't contain directory traversal)
    pub fn is_safe_path<P: AsRef<Path>>(path: P) -> bool {
        let path_str = path.as_ref().to_string_lossy();
        !path_str.contains("..") && !path_str.starts_with('/')
    }

    /// Get file size in bytes
    pub fn get_file_size<P: AsRef<Path>>(path: P) -> SynapsedResult<u64> {
        let metadata = std::fs::metadata(path)
            .map_err(|e| SynapsedError::internal(format!("Failed to get file metadata: {e}")))?;
        Ok(metadata.len())
    }
}

/// Retry utilities
pub mod retry {
    use super::Duration;
    use std::future::Future;

    /// Retry configuration
    #[derive(Debug, Clone)]
    pub struct RetryConfig {
        /// Maximum number of attempts
        pub max_attempts: usize,
        /// Base delay between attempts
        pub base_delay: Duration,
        /// Maximum delay between attempts
        pub max_delay: Duration,
        /// Backoff multiplier
        pub backoff_multiplier: f64,
        /// Whether to use jitter
        pub use_jitter: bool,
    }

    impl RetryConfig {
        /// Create a new retry configuration with defaults
        #[must_use] pub fn new() -> Self {
            Self {
                max_attempts: 3,
                base_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(60),
                backoff_multiplier: 2.0,
                use_jitter: true,
            }
        }

        /// Set maximum attempts
        #[must_use] pub fn with_max_attempts(mut self, max_attempts: usize) -> Self {
            self.max_attempts = max_attempts;
            self
        }

        /// Set base delay
        #[must_use] pub fn with_base_delay(mut self, base_delay: Duration) -> Self {
            self.base_delay = base_delay;
            self
        }

        /// Calculate delay for attempt number
        #[must_use] pub fn calculate_delay(&self, attempt: usize) -> Duration {
            let mut delay = Duration::from_millis(
                (self.base_delay.as_millis() as f64 
                    * self.backoff_multiplier.powi(attempt as i32)) as u64
            );

            if delay > self.max_delay {
                delay = self.max_delay;
            }

            if self.use_jitter {
                // Add up to 10% jitter
                let jitter_ms = (delay.as_millis() as f64 * 0.1) as u64;
                let jitter = Duration::from_millis(jitter_ms);
                delay + jitter
            } else {
                delay
            }
        }
    }

    impl Default for RetryConfig {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Execute a function with retry logic
    pub async fn retry_async<F, Fut, T, E>(
        config: RetryConfig,
        mut operation: F,
    ) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        let mut attempt = 0;
        let mut last_error = None;

        while attempt < config.max_attempts {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    last_error = Some(error);
                    attempt += 1;

                    if attempt < config.max_attempts {
                        let delay = config.calculate_delay(attempt - 1);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }
}

/// Rate limiting utilities
pub mod rate_limit {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::SystemTime;

    /// Simple token bucket rate limiter
    #[derive(Debug)]
    pub struct TokenBucket {
        capacity: u32,
        tokens: Arc<Mutex<f64>>,
        refill_rate: f64, // tokens per second
        last_refill: Arc<Mutex<SystemTime>>,
    }

    impl TokenBucket {
        /// Create a new token bucket
        #[must_use] pub fn new(capacity: u32, refill_rate: f64) -> Self {
            Self {
                capacity,
                tokens: Arc::new(Mutex::new(f64::from(capacity))),
                refill_rate,
                last_refill: Arc::new(Mutex::new(SystemTime::now())),
            }
        }

        /// Try to consume tokens from the bucket
        #[must_use] pub fn try_consume(&self, tokens: u32) -> bool {
            self.refill();
            
            let mut current_tokens = self.tokens.lock().unwrap();
            if *current_tokens >= f64::from(tokens) {
                *current_tokens -= f64::from(tokens);
                true
            } else {
                false
            }
        }

        /// Refill tokens based on elapsed time
        fn refill(&self) {
            let now = SystemTime::now();
            let mut last_refill = self.last_refill.lock().unwrap();
            
            if let Ok(elapsed) = now.duration_since(*last_refill) {
                let tokens_to_add = elapsed.as_secs_f64() * self.refill_rate;
                
                let mut current_tokens = self.tokens.lock().unwrap();
                *current_tokens = (*current_tokens + tokens_to_add).min(f64::from(self.capacity));
                
                *last_refill = now;
            }
        }

        /// Get current token count
        #[must_use] pub fn current_tokens(&self) -> u32 {
            self.refill();
            let tokens = self.tokens.lock().unwrap();
            *tokens as u32
        }
    }

    /// Rate limiter with per-key buckets
    #[derive(Debug)]
    pub struct KeyedRateLimiter {
        buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
        capacity: u32,
        refill_rate: f64,
    }

    impl KeyedRateLimiter {
        /// Create a new keyed rate limiter
        #[must_use] pub fn new(capacity: u32, refill_rate: f64) -> Self {
            Self {
                buckets: Arc::new(Mutex::new(HashMap::new())),
                capacity,
                refill_rate,
            }
        }

        /// Try to consume tokens for a specific key
        #[must_use] pub fn try_consume(&self, key: &str, tokens: u32) -> bool {
            let mut buckets = self.buckets.lock().unwrap();
            let bucket = buckets
                .entry(key.to_string())
                .or_insert_with(|| TokenBucket::new(self.capacity, self.refill_rate));
            
            bucket.try_consume(tokens)
        }

        /// Clean up expired buckets (call periodically)
        pub fn cleanup(&self) {
            // In a real implementation, you would remove buckets that haven't been used recently
            // For now, this is a placeholder
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_utils() {
        let timestamp = time::current_timestamp_millis();
        assert!(timestamp > 0);

        let duration = time::millis_to_duration(1500);
        assert_eq!(duration, Duration::from_millis(1500));

        let formatted = time::format_duration(Duration::from_secs(3661));
        assert_eq!(formatted, "1h 1m 1s");

        assert!(!time::is_expired(timestamp, 1000));
    }

    #[test]
    fn test_string_utils() {
        assert!(string::is_blank("  "));
        assert!(!string::is_blank("hello"));

        assert_eq!(string::truncate("hello world", 5), "he...");
        assert_eq!(string::truncate("hi", 5), "hi");

        assert_eq!(string::to_title_case("hello world"), "Hello World");
        assert_eq!(string::to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(string::to_kebab_case("HelloWorld"), "hello-world");

        let random = string::random_alphanumeric(10);
        assert_eq!(random.len(), 10);

        assert_eq!(string::sanitize_filename("file/name?.txt"), "file_name_.txt");
    }

    #[test]
    fn test_collection_utils() {
        let mut map1 = HashMap::new();
        map1.insert("a", 1);
        map1.insert("b", 2);

        let mut map2 = HashMap::new();
        map2.insert("b", 3);
        map2.insert("c", 4);

        let merged = collections::merge_hashmaps(map1, map2);
        assert_eq!(merged.get("b"), Some(&3));
        assert_eq!(merged.get("c"), Some(&4));

        let items = vec!["apple", "banana", "apple", "cherry"];
        let groups = collections::group_by(items, |s| s.len());
        assert_eq!(groups.get(&5).unwrap().len(), 2); // apple, apple
        assert_eq!(groups.get(&6).unwrap().len(), 2); // banana, cherry

        let counts = collections::count_by(&["a", "b", "a", "c"], |&s| s);
        assert_eq!(counts.get("a"), Some(&2));

        let duplicates = collections::find_duplicates(&[1, 2, 3, 2, 4], |&x| x);
        assert_eq!(duplicates, vec![2]);

        let deduped = collections::dedup_preserve_order(vec![1, 2, 3, 2, 4]);
        assert_eq!(deduped, vec![1, 2, 3, 4]);

        let (evens, odds) = collections::partition(vec![1, 2, 3, 4, 5], |&x| x % 2 == 0);
        assert_eq!(evens, vec![2, 4]);
        assert_eq!(odds, vec![1, 3, 5]);
    }

    #[test]
    fn test_math_utils() {
        assert_eq!(math::percentage(25.0, 100.0), 25.0);
        assert_eq!(math::percentage(10.0, 0.0), 0.0);

        assert_eq!(math::clamp(5, 1, 10), 5);
        assert_eq!(math::clamp(-1, 1, 10), 1);
        assert_eq!(math::clamp(15, 1, 10), 10);

        assert_eq!(math::lerp(0.0, 10.0, 0.5), 5.0);

        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let moving_avg = math::moving_average(&values, 3);
        assert_eq!(moving_avg, vec![2.0, 3.0, 4.0]);

        let std_dev = math::standard_deviation(&values);
        assert!(std_dev > 1.4 && std_dev < 1.6);

        assert_eq!(math::round_to_decimal_places(3.14159, 2), 3.14);
    }

    #[test]
    fn test_validation_utils() {
        assert!(validation::is_valid_email("test@example.com"));
        assert!(!validation::is_valid_email("invalid-email"));

        assert!(validation::is_valid_url("https://example.com"));
        assert!(!validation::is_valid_url("not-a-url"));

        assert!(validation::is_not_empty("hello").is_ok());
        assert!(validation::is_not_empty("  ").is_err());

        assert!(validation::is_in_range(5, 1, 10).is_ok());
        assert!(validation::is_in_range(15, 1, 10).is_err());

        assert!(validation::has_min_items(&[1, 2, 3], 2).is_ok());
        assert!(validation::has_min_items(&[1], 2).is_err());

        assert!(validation::has_max_items(&[1, 2], 3).is_ok());
        assert!(validation::has_max_items(&[1, 2, 3, 4], 3).is_err());
    }

    #[test]
    fn test_retry_config() {
        let config = retry::RetryConfig::new()
            .with_max_attempts(5)
            .with_base_delay(Duration::from_millis(100));

        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.base_delay, Duration::from_millis(100));

        let delay = config.calculate_delay(1);
        assert!(delay >= Duration::from_millis(100));
    }

    #[test]
    fn test_token_bucket() {
        let bucket = rate_limit::TokenBucket::new(10, 1.0);
        
        assert!(bucket.try_consume(5));
        assert!(bucket.try_consume(3));
        assert!(!bucket.try_consume(5)); // Should fail, not enough tokens
        
        assert!(bucket.current_tokens() < 10);
    }

    #[test]
    fn test_keyed_rate_limiter() {
        let limiter = rate_limit::KeyedRateLimiter::new(10, 1.0);
        
        assert!(limiter.try_consume("user1", 5));
        assert!(limiter.try_consume("user2", 5));
        assert!(limiter.try_consume("user1", 3));
        assert!(!limiter.try_consume("user1", 5)); // Should fail for user1
        assert!(limiter.try_consume("user2", 3)); // Should succeed for user2
    }
}