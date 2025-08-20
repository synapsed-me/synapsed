//! Buffer pool for efficient memory management

use std::sync::Arc;
use parking_lot::Mutex;
use bytes::BytesMut;

/// Buffer pool for reusing byte buffers
pub struct BufferPool {
    pool: Arc<Mutex<Vec<BytesMut>>>,
    buffer_size: usize,
    max_pool_size: usize,
}

impl BufferPool {
    /// Create a new buffer pool
    pub fn new(buffer_size: usize, max_pool_size: usize) -> Self {
        Self {
            pool: Arc::new(Mutex::new(Vec::with_capacity(max_pool_size))),
            buffer_size,
            max_pool_size,
        }
    }

    /// Get a buffer from the pool or create a new one
    pub fn get(&self) -> BytesMut {
        let mut pool = self.pool.lock();
        pool.pop().unwrap_or_else(|| BytesMut::with_capacity(self.buffer_size))
    }

    /// Return a buffer to the pool
    pub fn put(&self, mut buffer: BytesMut) {
        buffer.clear();
        
        let mut pool = self.pool.lock();
        if pool.len() < self.max_pool_size {
            pool.push(buffer);
        }
        // Otherwise, let the buffer be dropped
    }

    /// Get the current pool size
    pub fn size(&self) -> usize {
        self.pool.lock().len()
    }

    /// Clear the pool
    pub fn clear(&self) {
        self.pool.lock().clear();
    }
}

impl Default for BufferPool {
    fn default() -> Self {
        Self::new(4096, 100) // 4KB buffers, max 100 in pool
    }
}

/// Global buffer pool for the storage system
static GLOBAL_POOL: once_cell::sync::Lazy<BufferPool> = 
    once_cell::sync::Lazy::new(|| BufferPool::default());

/// Get a buffer from the global pool
pub fn get_buffer() -> BytesMut {
    GLOBAL_POOL.get()
}

/// Return a buffer to the global pool
pub fn return_buffer(buffer: BytesMut) {
    GLOBAL_POOL.put(buffer);
}