//! GPU kernel compilation and optimization.

use std::collections::HashMap;
use tracing::{debug, info, warn};
use serde::{Deserialize, Serialize};

use crate::{DeviceType, Result, GpuError};

/// Kernel source code representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KernelSource {
    /// CUDA PTX assembly source.
    Cuda(String),
    
    /// OpenCL C source code.
    OpenCL(String),
    
    /// Generic kernel source that can be adapted to different backends.
    Generic(String),
}

/// Kernel compiler for different GPU backends.
#[derive(Debug)]
pub struct KernelCompiler {
    device_type: DeviceType,
    optimization_level: OptimizationLevel,
    compiler_options: CompilerOptions,
}

/// Kernel compilation optimization levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    /// No optimization (fastest compilation).
    None,
    
    /// Basic optimizations.
    Basic,
    
    /// Aggressive optimizations (slower compilation, faster execution).
    Aggressive,
    
    /// Maximum optimizations including experimental features.
    Maximum,
}

/// Compiler configuration options.
#[derive(Debug, Clone)]
pub struct CompilerOptions {
    /// Enable debug information in compiled kernels.
    pub debug_info: bool,
    
    /// Enable profiling hooks in kernels.
    pub profiling: bool,
    
    /// Maximum register usage per thread.
    pub max_registers: Option<u32>,
    
    /// Target compute capability (CUDA only).
    pub compute_capability: Option<(u32, u32)>,
    
    /// Enable math optimizations.
    pub fast_math: bool,
    
    /// Use native math functions where possible.
    pub native_math: bool,
    
    /// Additional compiler flags.
    pub extra_flags: Vec<String>,
}

/// Kernel compilation result.
#[derive(Debug, Clone)]
pub struct CompilationResult {
    /// Compiled binary code.
    pub binary: Vec<u8>,
    
    /// Compilation log and warnings.
    pub log: String,
    
    /// Compilation time in milliseconds.
    pub compile_time_ms: u64,
    
    /// Source code hash for caching.
    pub source_hash: String,
    
    /// Kernel metadata.
    pub metadata: KernelMetadata,
}

/// Metadata about compiled kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelMetadata {
    /// Kernel name.
    pub name: String,
    
    /// Number of registers used per thread.
    pub register_count: u32,
    
    /// Shared memory usage in bytes.
    pub shared_memory_bytes: u32,
    
    /// Maximum threads per block.
    pub max_threads_per_block: u32,
    
    /// Recommended block size.
    pub recommended_block_size: (u32, u32, u32),
    
    /// Estimated occupancy.
    pub occupancy: f32,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            debug_info: false,
            profiling: false,
            max_registers: None,
            compute_capability: None,
            fast_math: true,
            native_math: false,
            extra_flags: Vec::new(),
        }
    }
}

impl KernelCompiler {
    /// Create a new kernel compiler for the specified device type.
    pub fn new(device_type: DeviceType) -> Self {
        Self {
            device_type,
            optimization_level: OptimizationLevel::Basic,
            compiler_options: CompilerOptions::default(),
        }
    }

    /// Create compiler with custom options.
    pub fn with_options(device_type: DeviceType, options: CompilerOptions) -> Self {
        Self {
            device_type,
            optimization_level: OptimizationLevel::Basic,
            compiler_options: options,
        }
    }

    /// Set optimization level.
    pub fn set_optimization_level(&mut self, level: OptimizationLevel) {
        self.optimization_level = level;
    }

    /// Compile kernel source to binary.
    pub async fn compile(&self, name: &str, source: &KernelSource) -> Result<CompilationResult> {
        let start_time = std::time::Instant::now();
        
        info!("Compiling kernel '{}' for {:?}", name, self.device_type);
        
        let result = match self.device_type {
            DeviceType::Cuda => self.compile_cuda(name, source).await?,
            DeviceType::OpenCL => self.compile_opencl(name, source).await?,
            DeviceType::Auto => {
                // Try CUDA first, fall back to OpenCL
                match self.compile_cuda(name, source).await {
                    Ok(result) => result,
                    Err(_) => self.compile_opencl(name, source).await?,
                }
            }
        };
        
        let compile_time = start_time.elapsed().as_millis() as u64;
        info!("Kernel '{}' compiled in {}ms", name, compile_time);
        
        Ok(CompilationResult {
            binary: result.binary,
            log: result.log,
            compile_time_ms: compile_time,
            source_hash: self.compute_source_hash(source),
            metadata: result.metadata,
        })
    }

    /// Optimize kernel source before compilation.
    pub fn optimize_source(&self, source: &KernelSource) -> Result<KernelSource> {
        match source {
            KernelSource::Generic(code) => {
                let optimized = self.apply_generic_optimizations(code)?;
                Ok(KernelSource::Generic(optimized))
            }
            KernelSource::Cuda(ptx) => {
                let optimized = self.apply_cuda_optimizations(ptx)?;
                Ok(KernelSource::Cuda(optimized))
            }
            KernelSource::OpenCL(cl) => {
                let optimized = self.apply_opencl_optimizations(cl)?;
                Ok(KernelSource::OpenCL(optimized))
            }
        }
    }

    /// Convert generic kernel source to backend-specific source.
    pub fn convert_generic_source(&self, source: &str) -> Result<KernelSource> {
        match self.device_type {
            DeviceType::Cuda => {
                let cuda_source = self.generic_to_cuda(source)?;
                Ok(KernelSource::Cuda(cuda_source))
            }
            DeviceType::OpenCL => {
                let opencl_source = self.generic_to_opencl(source)?;
                Ok(KernelSource::OpenCL(opencl_source))
            }
            DeviceType::Auto => {
                // Default to OpenCL for generic conversion
                let opencl_source = self.generic_to_opencl(source)?;
                Ok(KernelSource::OpenCL(opencl_source))
            }
        }
    }

    /// Analyze kernel for optimization opportunities.
    pub fn analyze_kernel(&self, source: &KernelSource) -> KernelAnalysis {
        let source_code = match source {
            KernelSource::Cuda(code) => code,
            KernelSource::OpenCL(code) => code,
            KernelSource::Generic(code) => code,
        };

        let mut analysis = KernelAnalysis::default();
        
        // Analyze memory access patterns
        if source_code.contains("__global") {
            analysis.global_memory_accesses = source_code.matches("__global").count() as u32;
        }
        
        if source_code.contains("__local") || source_code.contains("__shared") {
            analysis.uses_shared_memory = true;
        }
        
        // Analyze control flow
        analysis.has_divergent_branches = source_code.contains("if") && source_code.contains("get_global_id");
        analysis.has_loops = source_code.contains("for") || source_code.contains("while");
        
        // Analyze synchronization
        analysis.uses_barriers = source_code.contains("barrier") || source_code.contains("__syncthreads");
        analysis.uses_atomics = source_code.contains("atomic_") || source_code.contains("atomicAdd");
        
        // Estimate complexity
        let lines = source_code.lines().count();
        analysis.estimated_complexity = if lines < 50 {
            ComplexityLevel::Low
        } else if lines < 200 {
            ComplexityLevel::Medium
        } else {
            ComplexityLevel::High
        };
        
        analysis
    }

    // Backend-specific compilation methods

    async fn compile_cuda(&self, name: &str, source: &KernelSource) -> Result<CompilationResult> {
        let cuda_source = match source {
            KernelSource::Cuda(ptx) => ptx.clone(),
            KernelSource::Generic(code) => self.generic_to_cuda(code)?,
            KernelSource::OpenCL(_) => {
                return Err(GpuError::kernel("Cannot compile OpenCL source for CUDA"));
            }
        };

        // Mock CUDA compilation
        debug!("Compiling CUDA kernel: {}", name);
        
        let metadata = KernelMetadata {
            name: name.to_string(),
            register_count: 32,
            shared_memory_bytes: 0,
            max_threads_per_block: 1024,
            recommended_block_size: (256, 1, 1),
            occupancy: 0.75,
        };

        Ok(CompilationResult {
            binary: cuda_source.into_bytes(),
            log: "CUDA compilation successful".to_string(),
            compile_time_ms: 0,
            source_hash: String::new(),
            metadata,
        })
    }

    async fn compile_opencl(&self, name: &str, source: &KernelSource) -> Result<CompilationResult> {
        let opencl_source = match source {
            KernelSource::OpenCL(cl) => cl.clone(),
            KernelSource::Generic(code) => self.generic_to_opencl(code)?,
            KernelSource::Cuda(_) => {
                return Err(GpuError::kernel("Cannot compile CUDA source for OpenCL"));
            }
        };

        // Mock OpenCL compilation
        debug!("Compiling OpenCL kernel: {}", name);
        
        let metadata = KernelMetadata {
            name: name.to_string(),
            register_count: 24,
            shared_memory_bytes: 0,
            max_threads_per_block: 256,
            recommended_block_size: (64, 1, 1),
            occupancy: 0.8,
        };

        Ok(CompilationResult {
            binary: opencl_source.into_bytes(),
            log: "OpenCL compilation successful".to_string(),
            compile_time_ms: 0,
            source_hash: String::new(),
            metadata,
        })
    }

    // Optimization methods

    fn apply_generic_optimizations(&self, source: &str) -> Result<String> {
        let mut optimized = source.to_string();
        
        match self.optimization_level {
            OptimizationLevel::None => {},
            OptimizationLevel::Basic => {
                optimized = self.apply_basic_optimizations(&optimized);
            }
            OptimizationLevel::Aggressive => {
                optimized = self.apply_basic_optimizations(&optimized);
                optimized = self.apply_aggressive_optimizations(&optimized);
            }
            OptimizationLevel::Maximum => {
                optimized = self.apply_basic_optimizations(&optimized);
                optimized = self.apply_aggressive_optimizations(&optimized);
                optimized = self.apply_experimental_optimizations(&optimized);
            }
        }
        
        Ok(optimized)
    }

    fn apply_cuda_optimizations(&self, source: &str) -> Result<String> {
        let mut optimized = source.to_string();
        
        // CUDA-specific optimizations
        if self.compiler_options.fast_math {
            optimized.insert_str(0, "#pragma CUDA_FAST_MATH\n");
        }
        
        Ok(optimized)
    }

    fn apply_opencl_optimizations(&self, source: &str) -> Result<String> {
        let mut optimized = source.to_string();
        
        // OpenCL-specific optimizations
        if self.compiler_options.fast_math {
            optimized.insert_str(0, "#pragma OPENCL EXTENSION cl_khr_fp16 : enable\n");
        }
        
        Ok(optimized)
    }

    fn apply_basic_optimizations(&self, source: &str) -> String {
        // Basic optimizations like loop unrolling hints
        source.replace("for (int i = 0; i < 4; i++)", "#pragma unroll\nfor (int i = 0; i < 4; i++)")
    }

    fn apply_aggressive_optimizations(&self, source: &str) -> String {
        // More aggressive optimizations
        let mut optimized = source.to_string();
        
        // Add vectorization hints
        if !optimized.contains("#pragma vectorize") {
            optimized = optimized.replace("for (", "#pragma vectorize\nfor (");
        }
        
        optimized
    }

    fn apply_experimental_optimizations(&self, source: &str) -> String {
        // Experimental optimizations
        source.to_string()
    }

    // Source conversion methods

    fn generic_to_cuda(&self, source: &str) -> Result<String> {
        let mut cuda_source = source.to_string();
        
        // Convert OpenCL keywords to CUDA
        cuda_source = cuda_source.replace("__kernel", "__global__");
        cuda_source = cuda_source.replace("__global", "__device__");
        cuda_source = cuda_source.replace("__local", "__shared__");
        cuda_source = cuda_source.replace("get_global_id(0)", "blockIdx.x * blockDim.x + threadIdx.x");
        cuda_source = cuda_source.replace("get_local_id(0)", "threadIdx.x");
        cuda_source = cuda_source.replace("get_group_id(0)", "blockIdx.x");
        cuda_source = cuda_source.replace("barrier(CLK_LOCAL_MEM_FENCE)", "__syncthreads()");
        
        Ok(cuda_source)
    }

    fn generic_to_opencl(&self, source: &str) -> Result<String> {
        // Generic source is already close to OpenCL
        Ok(source.to_string())
    }

    fn compute_source_hash(&self, source: &KernelSource) -> String {
        let source_str = match source {
            KernelSource::Cuda(code) => code,
            KernelSource::OpenCL(code) => code,
            KernelSource::Generic(code) => code,
        };
        
        format!("{:x}", md5::compute(source_str.as_bytes()))
    }
}

/// Kernel analysis results.
#[derive(Debug, Clone, Default)]
pub struct KernelAnalysis {
    pub global_memory_accesses: u32,
    pub uses_shared_memory: bool,
    pub has_divergent_branches: bool,
    pub has_loops: bool,
    pub uses_barriers: bool,
    pub uses_atomics: bool,
    pub estimated_complexity: ComplexityLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexityLevel {
    Low,
    Medium,
    High,
}

impl Default for ComplexityLevel {
    fn default() -> Self {
        ComplexityLevel::Low
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compiler_creation() {
        let compiler = KernelCompiler::new(DeviceType::OpenCL);
        assert_eq!(compiler.device_type, DeviceType::OpenCL);
        assert_eq!(compiler.optimization_level, OptimizationLevel::Basic);
    }

    #[tokio::test]
    async fn test_source_conversion() {
        let compiler = KernelCompiler::new(DeviceType::Cuda);
        
        let generic_source = r#"
        __kernel void test(__global float* data) {
            int id = get_global_id(0);
            data[id] = id;
        }
        "#;
        
        let cuda_source = compiler.generic_to_cuda(generic_source).unwrap();
        assert!(cuda_source.contains("__global__"));
        assert!(cuda_source.contains("blockIdx.x * blockDim.x + threadIdx.x"));
    }

    #[tokio::test]
    async fn test_kernel_analysis() {
        let compiler = KernelCompiler::new(DeviceType::OpenCL);
        
        let source = KernelSource::Generic(r#"
        __kernel void test(__global float* data, __local float* temp) {
            int id = get_global_id(0);
            if (id > 0) {
                for (int i = 0; i < 10; i++) {
                    temp[id] += data[id + i];
                }
                barrier(CLK_LOCAL_MEM_FENCE);
                atomic_add(&data[0], temp[id]);
            }
        }
        "#.to_string());
        
        let analysis = compiler.analyze_kernel(&source);
        
        assert!(analysis.global_memory_accesses > 0);
        assert!(analysis.uses_shared_memory);
        assert!(analysis.has_divergent_branches);
        assert!(analysis.has_loops);
        assert!(analysis.uses_barriers);
        assert!(analysis.uses_atomics);
    }

    #[tokio::test]
    async fn test_optimization_levels() {
        let mut compiler = KernelCompiler::new(DeviceType::OpenCL);
        
        let source = "for (int i = 0; i < 4; i++) { data[i] = i; }";
        
        compiler.set_optimization_level(OptimizationLevel::None);
        let none_opt = compiler.apply_generic_optimizations(source).unwrap();
        assert_eq!(none_opt, source);
        
        compiler.set_optimization_level(OptimizationLevel::Basic);
        let basic_opt = compiler.apply_generic_optimizations(source).unwrap();
        assert!(basic_opt.contains("#pragma unroll"));
    }

    #[tokio::test]
    async fn test_compilation() {
        let compiler = KernelCompiler::new(DeviceType::OpenCL);
        
        let source = KernelSource::Generic(r#"
        __kernel void simple_add(__global float* a, __global float* b, __global float* c) {
            int id = get_global_id(0);
            c[id] = a[id] + b[id];
        }
        "#.to_string());
        
        let result = compiler.compile("simple_add", &source).await.unwrap();
        
        assert!(!result.binary.is_empty());
        assert!(!result.log.is_empty());
        assert!(!result.source_hash.is_empty());
        assert_eq!(result.metadata.name, "simple_add");
    }

    #[tokio::test]
    async fn test_compiler_options() {
        let mut options = CompilerOptions::default();
        options.debug_info = true;
        options.profiling = true;
        options.fast_math = false;
        
        let compiler = KernelCompiler::with_options(DeviceType::Cuda, options.clone());
        
        assert!(compiler.compiler_options.debug_info);
        assert!(compiler.compiler_options.profiling);
        assert!(!compiler.compiler_options.fast_math);
    }

    #[test]
    fn test_source_hash_consistency() {
        let compiler = KernelCompiler::new(DeviceType::OpenCL);
        
        let source1 = KernelSource::Generic("test code".to_string());
        let source2 = KernelSource::Generic("test code".to_string());
        let source3 = KernelSource::Generic("different code".to_string());
        
        let hash1 = compiler.compute_source_hash(&source1);
        let hash2 = compiler.compute_source_hash(&source2);
        let hash3 = compiler.compute_source_hash(&source3);
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}