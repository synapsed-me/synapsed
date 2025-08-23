//! Observable memory backend implementation

use async_trait::async_trait;
use bytes::Bytes;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::{
    error::{Result, StorageError},
    traits::Storage,
    observable::{StorageEvent, EventType},
};

/// Observable memory storage backend
pub struct ObservableMemoryStorage {
    data: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
    event_sender: broadcast::Sender<StorageEvent>,
}

impl ObservableMemoryStorage {
    /// Create a new observable memory storage instance
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(1024);
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
        }
    }

    /// Subscribe to storage events
    pub fn subscribe(&self) -> broadcast::Receiver<StorageEvent> {
        self.event_sender.subscribe()
    }

    /// Notify all observers of an event
    fn notify_observers(&self, event: StorageEvent) {
        // Ignore errors - if no receivers are listening, that's fine
        let _ = self.event_sender.send(event);
    }
}

impl Default for ObservableMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Storage for ObservableMemoryStorage {
    type Error = StorageError;

    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>> {
        let data = self.data.read();
        let result = data.get(key).map(|v| Bytes::copy_from_slice(v));
        
        self.notify_observers(StorageEvent {
            event_type: EventType::Get,
            key: Some(key.to_vec()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            metadata: None,
        });
        
        Ok(result)
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let mut data = self.data.write();
        data.insert(key.to_vec(), value.to_vec());
        
        self.notify_observers(StorageEvent {
            event_type: EventType::Put,
            key: Some(key.to_vec()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            metadata: None,
        });
        
        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
        let mut data = self.data.write();
        let _existed = data.remove(key).is_some();
        
        self.notify_observers(StorageEvent {
            event_type: EventType::Delete,
            key: Some(key.to_vec()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            metadata: None,
        });
        
        Ok(())
    }
    
    async fn list(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>> {
        let data = self.data.read();
        let keys: Vec<Vec<u8>> = data
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(keys)
    }
}