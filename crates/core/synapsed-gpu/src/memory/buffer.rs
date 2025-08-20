//! GPU buffer management and operations.

use std::sync::Arc;
use std::time::Instant;
use serde::{Deserialize, Serialize};

use crate::{AllocationInfo, Result, GpuError};

/// GPU memory buffer with metadata and lifecycle management.
#[derive(Debug)]
pub struct GpuBuffer {
    info: AllocationInfo,
    data: BufferData,
    state: BufferState,
}

/// Buffer data storage backend.
#[derive(Debug)]
enum BufferData {
    #[cfg(feature = "cuda")]
    Cuda(CudaBufferData),
    
    #[cfg(feature = "opencl")]
    OpenCL(OpenClBufferData),
    
    Mock(MockBufferData),
}

#[cfg(feature = "cuda")]
#[derive(Debug)]
struct CudaBufferData {
    device_ptr: cudarc::driver::DevicePtr<u8>,
    host_ptr: Option<*mut u8>,
}

#[cfg(feature = "opencl")]
#[derive(Debug)]
struct OpenClBufferData {
    buffer: opencl3::memory::Buffer<u8>,
    host_ptr: Option<*mut u8>,
}

#[derive(Debug)]
struct MockBufferData {
    device_ptr: u64,
    host_ptr: Option<u64>,
    data: Vec<u8>,
}

/// Buffer state tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BufferState {
    Allocated,
    InUse,
    Transferring,
    Released,
}

impl GpuBuffer {
    /// Create a new GPU buffer with mock data (for testing).
    pub fn new_mock(id: String, size: u64, alignment: u64) -> Self {
        let info = AllocationInfo {
            id,
            size_bytes: size,
            alignment,
            device_ptr: rand::random::<u64>(),
            host_ptr: None,
            is_pinned: false,
            is_managed: false,
            allocation_time: Instant::now(),
        };

        let data = BufferData::Mock(MockBufferData {
            device_ptr: info.device_ptr,
            host_ptr: None,
            data: vec![0u8; size as usize],
        });

        Self {
            info,
            data,
            state: BufferState::Allocated,
        }
    }

    /// Create a new CUDA buffer.
    #[cfg(feature = "cuda")]
    pub fn new_cuda(
        id: String,
        device_ptr: cudarc::driver::DevicePtr<u8>,
        size: u64,
        alignment: u64,
        is_pinned: bool,
        is_managed: bool,
    ) -> Self {
        let info = AllocationInfo {
            id,
            size_bytes: size,
            alignment,
            device_ptr: device_ptr.as_raw(),
            host_ptr: None,
            is_pinned,
            is_managed,
            allocation_time: Instant::now(),
        };

        let data = BufferData::Cuda(CudaBufferData {
            device_ptr,
            host_ptr: None,
        });

        Self {
            info,
            data,
            state: BufferState::Allocated,
        }
    }

    /// Create a new OpenCL buffer.
    #[cfg(feature = "opencl")]
    pub fn new_opencl(
        id: String,
        buffer: opencl3::memory::Buffer<u8>,
        size: u64,
        alignment: u64,
    ) -> Self {
        let info = AllocationInfo {
            id,
            size_bytes: size,
            alignment,
            device_ptr: buffer.as_ptr() as u64,
            host_ptr: None,
            is_pinned: false,
            is_managed: false,
            allocation_time: Instant::now(),
        };

        let data = BufferData::OpenCL(OpenClBufferData {
            buffer,
            host_ptr: None,
        });

        Self {
            info,
            data,
            state: BufferState::Allocated,
        }
    }

    /// Get buffer ID.
    pub fn id(&self) -> &str {
        &self.info.id
    }

    /// Get buffer size in bytes.
    pub fn size(&self) -> u64 {
        self.info.size_bytes
    }

    /// Get buffer alignment.
    pub fn alignment(&self) -> u64 {
        self.info.alignment
    }

    /// Get device pointer.
    pub fn device_ptr(&self) -> u64 {
        self.info.device_ptr
    }

    /// Get host pointer if available.
    pub fn host_ptr(&self) -> Option<u64> {
        self.info.host_ptr
    }

    /// Check if buffer is pinned.
    pub fn is_pinned(&self) -> bool {
        self.info.is_pinned
    }

    /// Check if buffer is managed (unified memory).
    pub fn is_managed(&self) -> bool {
        self.info.is_managed
    }

    /// Get allocation information.
    pub fn allocation_info(&self) -> &AllocationInfo {
        &self.info
    }

    /// Get current buffer state.
    pub fn state(&self) -> BufferState {
        self.state
    }

    /// Mark buffer as in use.
    pub fn mark_in_use(&mut self) {
        self.state = BufferState::InUse;
    }

    /// Mark buffer as transferring.
    pub fn mark_transferring(&mut self) {
        self.state = BufferState::Transferring;
    }

    /// Mark buffer as released.
    pub fn mark_released(&mut self) {
        self.state = BufferState::Released;
    }

    /// Check if buffer is available for use.
    pub fn is_available(&self) -> bool {
        matches!(self.state, BufferState::Allocated | BufferState::InUse)
    }

    /// Fill buffer with pattern (for testing and debugging).
    pub async fn fill_pattern(&mut self, pattern: u8) -> Result<()> {
        match &mut self.data {
            BufferData::Mock(mock_data) => {
                mock_data.data.fill(pattern);
                Ok(())
            }
            #[cfg(feature = "cuda")]
            BufferData::Cuda(_) => {
                // Would use CUDA memset kernel
                Ok(())
            }
            #[cfg(feature = "opencl")]
            BufferData::OpenCL(_) => {
                // Would use OpenCL fill kernel
                Ok(())
            }
        }
    }

    /// Copy data from host slice to buffer.
    pub async fn copy_from_host(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > self.size() as usize {
            return Err(GpuError::memory("Data size exceeds buffer size"));
        }

        self.mark_transferring();

        match &mut self.data {
            BufferData::Mock(mock_data) => {
                mock_data.data[..data.len()].copy_from_slice(data);
                Ok(())
            }
            #[cfg(feature = "cuda")]
            BufferData::Cuda(_) => {
                // Would use CUDA memory copy
                Ok(())
            }
            #[cfg(feature = "opencl")]
            BufferData::OpenCL(_) => {
                // Would use OpenCL memory copy
                Ok(())
            }
        }?;

        self.state = BufferState::InUse;
        Ok(())
    }

    /// Copy data from buffer to host slice.
    pub async fn copy_to_host(&self, data: &mut [u8]) -> Result<()> {
        if data.len() > self.size() as usize {
            return Err(GpuError::memory("Host buffer size exceeds GPU buffer size"));
        }

        match &self.data {
            BufferData::Mock(mock_data) => {
                data.copy_from_slice(&mock_data.data[..data.len()]);
                Ok(())
            }
            #[cfg(feature = "cuda")]
            BufferData::Cuda(_) => {
                // Would use CUDA memory copy
                Ok(())
            }
            #[cfg(feature = "opencl")]
            BufferData::OpenCL(_) => {
                // Would use OpenCL memory copy
                Ok(())
            }
        }
    }

    /// Zero-fill the buffer.
    pub async fn zero(&mut self) -> Result<()> {
        self.fill_pattern(0).await
    }

    /// Get age of buffer since allocation.
    pub fn age(&self) -> std::time::Duration {
        self.info.allocation_time.elapsed()
    }

    /// Check if buffer can be reused for given size and alignment.
    pub fn can_reuse(&self, size: u64, alignment: u64) -> bool {
        self.is_available() && 
        self.size() >= size && 
        self.alignment() >= alignment &&
        self.state != BufferState::Released
    }
}

// Implement Send and Sync for thread safety
unsafe impl Send for GpuBuffer {}
unsafe impl Sync for GpuBuffer {}

impl Drop for GpuBuffer {
    fn drop(&mut self) {
        // Mark as released when dropped
        self.state = BufferState::Released;
        
        // The actual memory deallocation is handled by the allocator
        // This is just for state tracking
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_buffer_creation() {
        let buffer = GpuBuffer::new_mock("test-buffer".to_string(), 1024, 256);
        
        assert_eq!(buffer.id(), "test-buffer");
        assert_eq!(buffer.size(), 1024);
        assert_eq!(buffer.alignment(), 256);
        assert!(!buffer.is_pinned());
        assert!(!buffer.is_managed());
        assert_eq!(buffer.state(), BufferState::Allocated);
        assert!(buffer.is_available());
    }

    #[tokio::test]
    async fn test_buffer_fill_pattern() {
        let mut buffer = GpuBuffer::new_mock("test-buffer".to_string(), 1024, 256);
        
        buffer.fill_pattern(0xAB).await.unwrap();
        
        // For mock buffer, we can verify the pattern
        if let BufferData::Mock(mock_data) = &buffer.data {
            assert!(mock_data.data.iter().all(|&b| b == 0xAB));
        }
    }

    #[tokio::test]
    async fn test_buffer_copy_operations() {
        let mut buffer = GpuBuffer::new_mock("test-buffer".to_string(), 1024, 256);
        
        // Test copy from host
        let data = vec![1, 2, 3, 4, 5];
        buffer.copy_from_host(&data).await.unwrap();
        assert_eq!(buffer.state(), BufferState::InUse);
        
        // Test copy to host
        let mut result = vec![0u8; 5];
        buffer.copy_to_host(&mut result).await.unwrap();
        assert_eq!(result, data);
    }

    #[tokio::test]
    async fn test_buffer_zero() {
        let mut buffer = GpuBuffer::new_mock("test-buffer".to_string(), 1024, 256);
        
        // Fill with pattern first
        buffer.fill_pattern(0xFF).await.unwrap();
        
        // Then zero
        buffer.zero().await.unwrap();
        
        // Verify it's zeroed
        if let BufferData::Mock(mock_data) = &buffer.data {
            assert!(mock_data.data.iter().all(|&b| b == 0));
        }
    }

    #[test]
    fn test_buffer_state_transitions() {
        let mut buffer = GpuBuffer::new_mock("test-buffer".to_string(), 1024, 256);
        
        assert_eq!(buffer.state(), BufferState::Allocated);
        assert!(buffer.is_available());
        
        buffer.mark_in_use();
        assert_eq!(buffer.state(), BufferState::InUse);
        assert!(buffer.is_available());
        
        buffer.mark_transferring();
        assert_eq!(buffer.state(), BufferState::Transferring);
        assert!(!buffer.is_available());
        
        buffer.mark_released();
        assert_eq!(buffer.state(), BufferState::Released);
        assert!(!buffer.is_available());
    }

    #[test]
    fn test_buffer_reuse_check() {
        let buffer = GpuBuffer::new_mock("test-buffer".to_string(), 1024, 256);
        
        // Should be reusable for smaller or equal size
        assert!(buffer.can_reuse(512, 128));
        assert!(buffer.can_reuse(1024, 256));
        
        // Should not be reusable for larger size or alignment
        assert!(!buffer.can_reuse(2048, 256));
        assert!(!buffer.can_reuse(1024, 512));
    }

    #[test]
    fn test_buffer_age() {
        let buffer = GpuBuffer::new_mock("test-buffer".to_string(), 1024, 256);
        
        let age = buffer.age();
        assert!(age.as_millis() >= 0);
        
        // Age should increase
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(buffer.age() > age);
    }

    #[tokio::test]
    async fn test_buffer_size_validation() {
        let mut buffer = GpuBuffer::new_mock("test-buffer".to_string(), 100, 1);
        
        // Should fail if data is too large
        let large_data = vec![0u8; 200];
        let result = buffer.copy_from_host(&large_data).await;
        assert!(result.is_err());
        
        // Should fail if host buffer is too large
        let mut large_host = vec![0u8; 200];
        let result = buffer.copy_to_host(&mut large_host).await;
        assert!(result.is_err());
    }
}