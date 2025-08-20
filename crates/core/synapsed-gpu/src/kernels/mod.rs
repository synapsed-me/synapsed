//! GPU kernel management and execution for cryptographic operations.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::{Device, DeviceContext, GpuBuffer, Result, GpuError};

pub mod crypto;
pub mod kyber;
pub mod common;
pub mod compiler;

pub use crypto::CryptoKernels;
pub use kyber::KyberKernels;
pub use common::CommonKernels;
pub use compiler::{KernelCompiler, KernelSource};

/// GPU kernel manager for compiling and executing kernels.
#[derive(Debug)]
pub struct KernelManager {
    device: Device,
    backend: KernelBackend,
    compiled_kernels: Arc<RwLock<HashMap<String, CompiledKernel>>>,
    crypto_kernels: Arc<CryptoKernels>,
    kyber_kernels: Arc<KyberKernels>,
    common_kernels: Arc<CommonKernels>,
}

/// Backend-specific kernel management.
#[derive(Debug)]
enum KernelBackend {
    #[cfg(feature = "cuda")]
    Cuda(CudaKernelBackend),
    
    #[cfg(feature = "opencl")]
    OpenCL(OpenClKernelBackend),
    
    Mock(MockKernelBackend),
}

#[cfg(feature = "cuda")]
#[derive(Debug)]
struct CudaKernelBackend {
    device: Arc<cudarc::driver::CudaDevice>,
    stream: cudarc::driver::CudaStream,
}

#[cfg(feature = "opencl")]
#[derive(Debug)]
struct OpenClKernelBackend {
    context: opencl3::context::Context,
    queue: opencl3::command_queue::CommandQueue,
    program_cache: Arc<RwLock<HashMap<String, opencl3::program::Program>>>,
}

#[derive(Debug)]
struct MockKernelBackend {
    kernels: Arc<RwLock<HashMap<String, MockKernel>>>,
}

#[derive(Debug, Clone)]
struct MockKernel {
    name: String,
    source: String,
    execution_time_ms: u64,
}

/// Compiled kernel ready for execution.
#[derive(Debug)]
pub struct CompiledKernel {
    pub name: String,
    pub source_hash: String,
    pub backend_kernel: BackendKernel,
    pub work_group_size: Option<(u32, u32, u32)>,
    pub compile_time: std::time::Instant,
}

#[derive(Debug)]
pub enum BackendKernel {
    #[cfg(feature = "cuda")]
    Cuda(cudarc::driver::CudaFunction),
    
    #[cfg(feature = "opencl")]
    OpenCL(opencl3::kernel::Kernel),
    
    Mock(String),
}

/// Kernel execution parameters.
#[derive(Debug, Clone)]
pub struct KernelParams {
    /// Global work size (total number of work items).
    pub global_work_size: (u32, u32, u32),
    
    /// Local work size (work group size).
    pub local_work_size: Option<(u32, u32, u32)>,
    
    /// Kernel arguments (buffer references and scalar values).
    pub args: Vec<KernelArg>,
    
    /// Shared memory size in bytes.
    pub shared_memory_bytes: u32,
}

/// Kernel argument types.
#[derive(Debug, Clone)]
pub enum KernelArg {
    Buffer(String), // Buffer ID
    Scalar(ScalarValue),
}

#[derive(Debug, Clone)]
pub enum ScalarValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl KernelManager {
    /// Create a new kernel manager for the specified device.
    pub async fn new(device: Device) -> Result<Self> {
        info!("Creating kernel manager for device: {}", device.info().id);

        let backend = match device.context() {
            #[cfg(feature = "cuda")]
            DeviceContext::Cuda(cuda_context) => {
                KernelBackend::Cuda(CudaKernelBackend {
                    device: cuda_context.device,
                    stream: cuda_context.stream,
                })
            }
            
            #[cfg(feature = "opencl")]
            DeviceContext::OpenCL(opencl_context) => {
                KernelBackend::OpenCL(OpenClKernelBackend {
                    context: opencl_context.context,
                    queue: opencl_context.queue,
                    program_cache: Arc::new(RwLock::new(HashMap::new())),
                })
            }
            
            DeviceContext::Mock => {
                KernelBackend::Mock(MockKernelBackend {
                    kernels: Arc::new(RwLock::new(HashMap::new())),
                })
            }
        };

        let crypto_kernels = Arc::new(CryptoKernels::new(device.clone()).await?);
        let kyber_kernels = Arc::new(KyberKernels::new(device.clone()).await?);
        let common_kernels = Arc::new(CommonKernels::new(device.clone()).await?);

        Ok(Self {
            device,
            backend,
            compiled_kernels: Arc::new(RwLock::new(HashMap::new())),
            crypto_kernels,
            kyber_kernels,
            common_kernels,
        })
    }

    /// Compile a kernel from source code.
    pub async fn compile_kernel(&self, name: &str, source: &KernelSource) -> Result<()> {
        info!("Compiling kernel: {}", name);

        let compiled = match &self.backend {
            #[cfg(feature = "cuda")]
            KernelBackend::Cuda(cuda) => {
                self.compile_cuda_kernel(cuda, name, source).await?
            }
            
            #[cfg(feature = "opencl")]
            KernelBackend::OpenCL(opencl) => {
                self.compile_opencl_kernel(opencl, name, source).await?
            }
            
            KernelBackend::Mock(mock) => {
                self.compile_mock_kernel(mock, name, source).await?
            }
        };

        let mut kernels = self.compiled_kernels.write().await;
        kernels.insert(name.to_string(), compiled);

        info!("Successfully compiled kernel: {}", name);
        Ok(())
    }

    /// Execute a compiled kernel with the given parameters.
    pub async fn execute_kernel(
        &self,
        name: &str,
        params: KernelParams,
        buffers: &HashMap<String, Arc<GpuBuffer>>,
    ) -> Result<KernelExecutionResult> {
        debug!("Executing kernel: {} with {} args", name, params.args.len());

        let kernels = self.compiled_kernels.read().await;
        let kernel = kernels.get(name)
            .ok_or_else(|| GpuError::kernel(format!("Kernel '{}' not found", name)))?;

        let start_time = std::time::Instant::now();

        let result = match &self.backend {
            #[cfg(feature = "cuda")]
            KernelBackend::Cuda(cuda) => {
                self.execute_cuda_kernel(cuda, kernel, params, buffers).await?
            }
            
            #[cfg(feature = "opencl")]
            KernelBackend::OpenCL(opencl) => {
                self.execute_opencl_kernel(opencl, kernel, params, buffers).await?
            }
            
            KernelBackend::Mock(mock) => {
                self.execute_mock_kernel(mock, kernel, params, buffers).await?
            }
        };

        let execution_time = start_time.elapsed();
        debug!("Kernel {} executed in {:?}", name, execution_time);

        Ok(KernelExecutionResult {
            kernel_name: name.to_string(),
            execution_time,
            work_items_executed: result.work_items_executed,
            memory_transferred: result.memory_transferred,
        })
    }

    /// Get crypto kernel implementations.
    pub fn crypto_kernels(&self) -> &CryptoKernels {
        &self.crypto_kernels
    }

    /// Get Kyber768 kernel implementations.
    pub fn kyber_kernels(&self) -> &KyberKernels {
        &self.kyber_kernels
    }

    /// Get common utility kernels.
    pub fn common_kernels(&self) -> &CommonKernels {
        &self.common_kernels
    }

    /// List all compiled kernels.
    pub async fn list_kernels(&self) -> Vec<String> {
        self.compiled_kernels.read().await.keys().cloned().collect()
    }

    /// Get kernel compilation information.
    pub async fn kernel_info(&self, name: &str) -> Option<KernelInfo> {
        let kernels = self.compiled_kernels.read().await;
        kernels.get(name).map(|kernel| KernelInfo {
            name: kernel.name.clone(),
            source_hash: kernel.source_hash.clone(),
            work_group_size: kernel.work_group_size,
            compile_time: kernel.compile_time,
            age: kernel.compile_time.elapsed(),
        })
    }

    // Backend-specific compilation methods

    #[cfg(feature = "cuda")]
    async fn compile_cuda_kernel(
        &self,
        cuda: &CudaKernelBackend,
        name: &str,
        source: &KernelSource,
    ) -> Result<CompiledKernel> {
        let ptx_source = match source {
            KernelSource::Cuda(ptx) => ptx.clone(),
            KernelSource::OpenCL(_) => {
                return Err(GpuError::kernel("Cannot compile OpenCL source for CUDA"));
            }
            KernelSource::Generic(code) => {
                // Convert generic code to PTX (simplified)
                format!("// Generated PTX from generic code\n{}", code)
            }
        };

        // In a real implementation, this would compile PTX to a CUDA function
        // For now, create a mock compiled kernel
        let source_hash = format!("{:x}", md5::compute(ptx_source.as_bytes()));
        
        Ok(CompiledKernel {
            name: name.to_string(),
            source_hash,
            backend_kernel: BackendKernel::Mock(name.to_string()),
            work_group_size: Some((256, 1, 1)), // Default CUDA block size
            compile_time: std::time::Instant::now(),
        })
    }

    #[cfg(feature = "opencl")]
    async fn compile_opencl_kernel(
        &self,
        opencl: &OpenClKernelBackend,
        name: &str,
        source: &KernelSource,
    ) -> Result<CompiledKernel> {
        let cl_source = match source {
            KernelSource::OpenCL(cl) => cl.clone(),
            KernelSource::Cuda(_) => {
                return Err(GpuError::kernel("Cannot compile CUDA source for OpenCL"));
            }
            KernelSource::Generic(code) => {
                // Convert generic code to OpenCL C (simplified)
                format!("// Generated OpenCL C from generic code\n{}", code)
            }
        };

        // In a real implementation, this would compile OpenCL C to a kernel
        let source_hash = format!("{:x}", md5::compute(cl_source.as_bytes()));
        
        Ok(CompiledKernel {
            name: name.to_string(),
            source_hash,
            backend_kernel: BackendKernel::Mock(name.to_string()),
            work_group_size: Some((256, 1, 1)), // Default OpenCL work group size
            compile_time: std::time::Instant::now(),
        })
    }

    async fn compile_mock_kernel(
        &self,
        mock: &MockKernelBackend,
        name: &str,
        source: &KernelSource,
    ) -> Result<CompiledKernel> {
        let source_code = match source {
            KernelSource::Cuda(ptx) => ptx.clone(),
            KernelSource::OpenCL(cl) => cl.clone(),
            KernelSource::Generic(code) => code.clone(),
        };

        let source_hash = format!("{:x}", md5::compute(source_code.as_bytes()));
        
        let mock_kernel = MockKernel {
            name: name.to_string(),
            source: source_code,
            execution_time_ms: 1, // 1ms mock execution time
        };

        let mut kernels = mock.kernels.write().await;
        kernels.insert(name.to_string(), mock_kernel);

        Ok(CompiledKernel {
            name: name.to_string(),
            source_hash,
            backend_kernel: BackendKernel::Mock(name.to_string()),
            work_group_size: Some((256, 1, 1)),
            compile_time: std::time::Instant::now(),
        })
    }

    // Backend-specific execution methods

    #[cfg(feature = "cuda")]
    async fn execute_cuda_kernel(
        &self,
        cuda: &CudaKernelBackend,
        kernel: &CompiledKernel,
        params: KernelParams,
        buffers: &HashMap<String, Arc<GpuBuffer>>,
    ) -> Result<ExecutionResult> {
        // Mock CUDA execution
        cuda.device.synchronize()?;
        
        let work_items = params.global_work_size.0 as u64 
            * params.global_work_size.1 as u64 
            * params.global_work_size.2 as u64;
        
        Ok(ExecutionResult {
            work_items_executed: work_items,
            memory_transferred: 0, // Would track actual memory transfers
        })
    }

    #[cfg(feature = "opencl")]
    async fn execute_opencl_kernel(
        &self,
        opencl: &OpenClKernelBackend,
        kernel: &CompiledKernel,
        params: KernelParams,
        buffers: &HashMap<String, Arc<GpuBuffer>>,
    ) -> Result<ExecutionResult> {
        // Mock OpenCL execution
        opencl.queue.finish()?;
        
        let work_items = params.global_work_size.0 as u64 
            * params.global_work_size.1 as u64 
            * params.global_work_size.2 as u64;
        
        Ok(ExecutionResult {
            work_items_executed: work_items,
            memory_transferred: 0, // Would track actual memory transfers
        })
    }

    async fn execute_mock_kernel(
        &self,
        mock: &MockKernelBackend,
        kernel: &CompiledKernel,
        params: KernelParams,
        buffers: &HashMap<String, Arc<GpuBuffer>>,
    ) -> Result<ExecutionResult> {
        let kernels = mock.kernels.read().await;
        if let Some(mock_kernel) = kernels.get(&kernel.name) {
            // Simulate execution time
            tokio::time::sleep(tokio::time::Duration::from_millis(mock_kernel.execution_time_ms)).await;
        }

        let work_items = params.global_work_size.0 as u64 
            * params.global_work_size.1 as u64 
            * params.global_work_size.2 as u64;

        Ok(ExecutionResult {
            work_items_executed: work_items,
            memory_transferred: 0,
        })
    }
}

/// Kernel execution result.
#[derive(Debug, Clone)]
pub struct KernelExecutionResult {
    pub kernel_name: String,
    pub execution_time: std::time::Duration,
    pub work_items_executed: u64,
    pub memory_transferred: u64,
}

#[derive(Debug, Clone)]
struct ExecutionResult {
    work_items_executed: u64,
    memory_transferred: u64,
}

/// Kernel information for monitoring.
#[derive(Debug, Clone)]
pub struct KernelInfo {
    pub name: String,
    pub source_hash: String,
    pub work_group_size: Option<(u32, u32, u32)>,
    pub compile_time: std::time::Instant,
    pub age: std::time::Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceManager, DeviceConfig};

    async fn create_test_kernel_manager() -> Result<KernelManager> {
        let device_config = DeviceConfig::default();
        let device_manager = DeviceManager::new(device_config).await?;
        let device = device_manager.select_best_device().await?;
        
        KernelManager::new(device).await
    }

    #[tokio::test]
    async fn test_kernel_manager_creation() {
        let manager = create_test_kernel_manager().await.unwrap();
        let kernels = manager.list_kernels().await;
        assert_eq!(kernels.len(), 0); // No kernels compiled initially
    }

    #[tokio::test]
    async fn test_kernel_compilation() {
        let manager = create_test_kernel_manager().await.unwrap();
        
        let source = KernelSource::Generic(
            "__kernel void test_kernel(__global float* data) { int id = get_global_id(0); data[id] = id; }"
            .to_string()
        );
        
        manager.compile_kernel("test_kernel", &source).await.unwrap();
        
        let kernels = manager.list_kernels().await;
        assert_eq!(kernels.len(), 1);
        assert!(kernels.contains(&"test_kernel".to_string()));
    }

    #[tokio::test]
    async fn test_kernel_info() {
        let manager = create_test_kernel_manager().await.unwrap();
        
        let source = KernelSource::Generic("test source".to_string());
        manager.compile_kernel("info_test", &source).await.unwrap();
        
        let info = manager.kernel_info("info_test").await;
        assert!(info.is_some());
        
        let info = info.unwrap();
        assert_eq!(info.name, "info_test");
        assert!(!info.source_hash.is_empty());
        assert!(info.work_group_size.is_some());
    }

    #[tokio::test]
    async fn test_kernel_execution() {
        let manager = create_test_kernel_manager().await.unwrap();
        
        let source = KernelSource::Generic("test kernel".to_string());
        manager.compile_kernel("exec_test", &source).await.unwrap();
        
        let params = KernelParams {
            global_work_size: (1024, 1, 1),
            local_work_size: Some((256, 1, 1)),
            args: vec![],
            shared_memory_bytes: 0,
        };
        
        let buffers = HashMap::new();
        let result = manager.execute_kernel("exec_test", params, &buffers).await.unwrap();
        
        assert_eq!(result.kernel_name, "exec_test");
        assert_eq!(result.work_items_executed, 1024);
        assert!(result.execution_time.as_millis() >= 0);
    }

    #[tokio::test]
    async fn test_kernel_args() {
        let manager = create_test_kernel_manager().await.unwrap();
        
        let source = KernelSource::Generic("test kernel with args".to_string());
        manager.compile_kernel("args_test", &source).await.unwrap();
        
        let params = KernelParams {
            global_work_size: (256, 1, 1),
            local_work_size: None,
            args: vec![
                KernelArg::Buffer("input_buffer".to_string()),
                KernelArg::Buffer("output_buffer".to_string()),
                KernelArg::Scalar(ScalarValue::U32(42)),
                KernelArg::Scalar(ScalarValue::F32(3.14)),
            ],
            shared_memory_bytes: 1024,
        };
        
        let buffers = HashMap::new();
        let result = manager.execute_kernel("args_test", params, &buffers).await.unwrap();
        
        assert_eq!(result.work_items_executed, 256);
    }

    #[tokio::test]
    async fn test_specialized_kernels() {
        let manager = create_test_kernel_manager().await.unwrap();
        
        // Test that specialized kernel modules are available
        let _crypto = manager.crypto_kernels();
        let _kyber = manager.kyber_kernels();
        let _common = manager.common_kernels();
        
        // These should not panic and should be properly initialized
    }

    #[tokio::test]
    async fn test_kernel_not_found() {
        let manager = create_test_kernel_manager().await.unwrap();
        
        let params = KernelParams {
            global_work_size: (1, 1, 1),
            local_work_size: None,
            args: vec![],
            shared_memory_bytes: 0,
        };
        
        let buffers = HashMap::new();
        let result = manager.execute_kernel("nonexistent", params, &buffers).await;
        
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("not found"));
        }
    }
}