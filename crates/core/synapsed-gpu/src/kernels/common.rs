//! Common utility GPU kernels.

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::{Device, KernelSource, Result};

/// Common utility GPU kernel implementations.
#[derive(Debug)]
pub struct CommonKernels {
    device: Device,
    kernel_sources: Arc<RwLock<HashMap<String, KernelSource>>>,
}

impl CommonKernels {
    /// Create new common kernel implementations.
    pub async fn new(device: Device) -> Result<Self> {
        info!("Initializing common GPU kernels for device: {}", device.info().id);

        let mut kernel_sources = HashMap::new();
        
        // Add common utility kernel sources
        kernel_sources.insert("memset".to_string(), Self::memset_kernel_source());
        kernel_sources.insert("memcpy".to_string(), Self::memcpy_kernel_source());
        kernel_sources.insert("vector_add".to_string(), Self::vector_add_kernel_source());
        kernel_sources.insert("vector_mul".to_string(), Self::vector_mul_kernel_source());
        kernel_sources.insert("matrix_mul".to_string(), Self::matrix_mul_kernel_source());
        kernel_sources.insert("reduction_sum".to_string(), Self::reduction_sum_kernel_source());
        kernel_sources.insert("parallel_sort".to_string(), Self::parallel_sort_kernel_source());
        kernel_sources.insert("histogram".to_string(), Self::histogram_kernel_source());
        kernel_sources.insert("transpose".to_string(), Self::transpose_kernel_source());
        kernel_sources.insert("prefix_sum".to_string(), Self::prefix_sum_kernel_source());

        Ok(Self {
            device,
            kernel_sources: Arc::new(RwLock::new(kernel_sources)),
        })
    }

    /// Get available kernel sources.
    pub async fn kernel_sources(&self) -> HashMap<String, KernelSource> {
        self.kernel_sources.read().await.clone()
    }

    // Kernel source implementations

    fn memset_kernel_source() -> KernelSource {
        KernelSource::Generic(r#"
// Memory Set Kernel - Fill buffer with value
__kernel void memset(
    __global uchar* buffer,
    uchar value,
    ulong size
) {
    ulong gid = get_global_id(0);
    if (gid >= size) return;
    
    buffer[gid] = value;
}
"#.to_string())
    }

    fn memcpy_kernel_source() -> KernelSource {
        KernelSource::Generic(r#"
// Memory Copy Kernel - Copy data between buffers
__kernel void memcpy(
    __global const uchar* src,
    __global uchar* dst,
    ulong size
) {
    ulong gid = get_global_id(0);
    if (gid >= size) return;
    
    dst[gid] = src[gid];
}
"#.to_string())
    }

    fn vector_add_kernel_source() -> KernelSource {
        KernelSource::Generic(r#"
// Vector Addition Kernel
__kernel void vector_add(
    __global const float* a,
    __global const float* b,
    __global float* c,
    uint size
) {
    uint gid = get_global_id(0);
    if (gid >= size) return;
    
    c[gid] = a[gid] + b[gid];
}
"#.to_string())
    }

    fn vector_mul_kernel_source() -> KernelSource {
        KernelSource::Generic(r#"
// Vector Multiplication Kernel (Element-wise)
__kernel void vector_mul(
    __global const float* a,
    __global const float* b,
    __global float* c,
    uint size
) {
    uint gid = get_global_id(0);
    if (gid >= size) return;
    
    c[gid] = a[gid] * b[gid];
}
"#.to_string())
    }

    fn matrix_mul_kernel_source() -> KernelSource {
        KernelSource::Generic(r#"
// Matrix Multiplication Kernel (Optimized with local memory)
__kernel void matrix_mul(
    __global const float* A,
    __global const float* B,
    __global float* C,
    uint M,  // Rows of A
    uint N,  // Cols of A, Rows of B
    uint K   // Cols of B
) {
    __local float As[16][16];
    __local float Bs[16][16];
    
    uint bx = get_group_id(0);
    uint by = get_group_id(1);
    uint tx = get_local_id(0);
    uint ty = get_local_id(1);
    
    uint aBegin = N * 16 * by;
    uint aEnd = aBegin + N - 1;
    uint aStep = 16;
    uint bBegin = 16 * bx;
    uint bStep = 16 * K;
    
    float Csub = 0.0f;
    
    for (uint a = aBegin, b = bBegin; a <= aEnd; a += aStep, b += bStep) {
        // Load matrices into local memory
        if (a/N + ty < M && (a%N) + tx < N) {
            As[ty][tx] = A[a + N * ty + tx];
        } else {
            As[ty][tx] = 0.0f;
        }
        
        if (b/K + ty < N && b%K + tx < K) {
            Bs[ty][tx] = B[b + K * ty + tx];
        } else {
            Bs[ty][tx] = 0.0f;
        }
        
        barrier(CLK_LOCAL_MEM_FENCE);
        
        // Compute partial dot product
        for (uint k = 0; k < 16; k++) {
            Csub += As[ty][k] * Bs[k][tx];
        }
        
        barrier(CLK_LOCAL_MEM_FENCE);
    }
    
    // Write result
    uint c = K * 16 * by + 16 * bx;
    if (by * 16 + ty < M && bx * 16 + tx < K) {
        C[c + K * ty + tx] = Csub;
    }
}
"#.to_string())
    }

    fn reduction_sum_kernel_source() -> KernelSource {
        KernelSource::Generic(r#"
// Parallel Reduction Sum Kernel
__kernel void reduction_sum(
    __global const float* input,
    __global float* output,
    __local float* local_sum,
    uint size
) {
    uint gid = get_global_id(0);
    uint lid = get_local_id(0);
    uint group_size = get_local_size(0);
    
    // Load data into local memory
    if (gid < size) {
        local_sum[lid] = input[gid];
    } else {
        local_sum[lid] = 0.0f;
    }
    
    barrier(CLK_LOCAL_MEM_FENCE);
    
    // Parallel reduction in local memory
    for (uint stride = group_size / 2; stride > 0; stride /= 2) {
        if (lid < stride) {
            local_sum[lid] += local_sum[lid + stride];
        }
        barrier(CLK_LOCAL_MEM_FENCE);
    }
    
    // Write group result
    if (lid == 0) {
        output[get_group_id(0)] = local_sum[0];
    }
}
"#.to_string())
    }

    fn parallel_sort_kernel_source() -> KernelSource {
        KernelSource::Generic(r#"
// Bitonic Sort Kernel (Power of 2 sizes)
__kernel void parallel_sort(
    __global uint* data,
    uint stage,
    uint pass,
    uint dir  // 1 for ascending, 0 for descending
) {
    uint gid = get_global_id(0);
    uint size = get_global_size(0);
    
    uint distance = 1 << (stage - pass);
    uint block_size = distance << 1;
    uint left_id = (gid % block_size) + (gid / block_size) * block_size;
    
    uint right_id = left_id + distance;
    
    if (right_id < size) {
        uint left_val = data[left_id];
        uint right_val = data[right_id];
        
        uint direction = dir ^ ((left_id / (1 << stage)) & 1);
        
        if ((left_val > right_val) == direction) {
            data[left_id] = right_val;
            data[right_id] = left_val;
        }
    }
}
"#.to_string())
    }

    fn histogram_kernel_source() -> KernelSource {
        KernelSource::Generic(r#"
// Histogram Calculation Kernel
__kernel void histogram(
    __global const uchar* data,
    __global uint* histogram,
    __local uint* local_hist,
    uint size
) {
    uint gid = get_global_id(0);
    uint lid = get_local_id(0);
    uint group_size = get_local_size(0);
    
    // Initialize local histogram
    for (uint i = lid; i < 256; i += group_size) {
        local_hist[i] = 0;
    }
    
    barrier(CLK_LOCAL_MEM_FENCE);
    
    // Accumulate local histogram
    if (gid < size) {
        atomic_inc(&local_hist[data[gid]]);
    }
    
    barrier(CLK_LOCAL_MEM_FENCE);
    
    // Merge with global histogram
    for (uint i = lid; i < 256; i += group_size) {
        if (local_hist[i] > 0) {
            atomic_add(&histogram[i], local_hist[i]);
        }
    }
}
"#.to_string())
    }

    fn transpose_kernel_source() -> KernelSource {
        KernelSource::Generic(r#"
// Matrix Transpose Kernel (Optimized with local memory)
__kernel void transpose(
    __global const float* input,
    __global float* output,
    uint rows,
    uint cols
) {
    __local float tile[16][17]; // 17 to avoid bank conflicts
    
    uint x = get_group_id(0) * 16 + get_local_id(0);
    uint y = get_group_id(1) * 16 + get_local_id(1);
    
    // Load tile into local memory
    if (x < cols && y < rows) {
        tile[get_local_id(1)][get_local_id(0)] = input[y * cols + x];
    }
    
    barrier(CLK_LOCAL_MEM_FENCE);
    
    // Transpose coordinates
    x = get_group_id(1) * 16 + get_local_id(0);
    y = get_group_id(0) * 16 + get_local_id(1);
    
    // Write transposed tile
    if (x < rows && y < cols) {
        output[y * rows + x] = tile[get_local_id(0)][get_local_id(1)];
    }
}
"#.to_string())
    }

    fn prefix_sum_kernel_source() -> KernelSource {
        KernelSource::Generic(r#"
// Parallel Prefix Sum (Scan) Kernel
__kernel void prefix_sum(
    __global const uint* input,
    __global uint* output,
    __local uint* temp,
    uint size
) {
    uint gid = get_global_id(0);
    uint lid = get_local_id(0);
    uint group_size = get_local_size(0);
    
    // Load input into local memory
    if (gid < size) {
        temp[lid] = input[gid];
    } else {
        temp[lid] = 0;
    }
    
    barrier(CLK_LOCAL_MEM_FENCE);
    
    // Up-sweep phase
    for (uint d = group_size >> 1; d > 0; d >>= 1) {
        if (lid < d) {
            uint ai = (lid + 1) * (group_size / d) - 1;
            uint bi = ai + (group_size / d / 2);
            temp[bi] += temp[ai];
        }
        barrier(CLK_LOCAL_MEM_FENCE);
    }
    
    // Clear the last element
    if (lid == 0) {
        temp[group_size - 1] = 0;
    }
    
    barrier(CLK_LOCAL_MEM_FENCE);
    
    // Down-sweep phase
    for (uint d = 1; d < group_size; d <<= 1) {
        if (lid < d) {
            uint ai = (lid + 1) * (group_size / d) - 1;
            uint bi = ai + (group_size / d / 2);
            uint t = temp[ai];
            temp[ai] = temp[bi];
            temp[bi] += t;
        }
        barrier(CLK_LOCAL_MEM_FENCE);
    }
    
    // Write output
    if (gid < size) {
        output[gid] = temp[lid];
    }
}
"#.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceManager, DeviceConfig};

    async fn create_test_common_kernels() -> Result<CommonKernels> {
        let device_config = DeviceConfig::default();
        let device_manager = DeviceManager::new(device_config).await?;
        let device = device_manager.select_best_device().await?;
        
        CommonKernels::new(device).await
    }

    #[tokio::test]
    async fn test_common_kernels_creation() {
        let common_kernels = create_test_common_kernels().await.unwrap();
        
        let sources = common_kernels.kernel_sources().await;
        assert!(sources.len() >= 10); // Should have all utility kernels
        assert!(sources.contains_key("memset"));
        assert!(sources.contains_key("memcpy"));
        assert!(sources.contains_key("vector_add"));
        assert!(sources.contains_key("matrix_mul"));
    }

    #[tokio::test]
    async fn test_memory_kernels() {
        let common_kernels = create_test_common_kernels().await.unwrap();
        let sources = common_kernels.kernel_sources().await;
        
        let memset_source = sources.get("memset").unwrap();
        match memset_source {
            KernelSource::Generic(code) => {
                assert!(code.contains("memset"));
                assert!(code.contains("buffer[gid] = value"));
            }
            _ => panic!("Expected generic kernel source"),
        }
        
        let memcpy_source = sources.get("memcpy").unwrap();
        match memcpy_source {
            KernelSource::Generic(code) => {
                assert!(code.contains("memcpy"));
                assert!(code.contains("dst[gid] = src[gid]"));
            }
            _ => panic!("Expected generic kernel source"),
        }
    }

    #[tokio::test]
    async fn test_vector_kernels() {
        let common_kernels = create_test_common_kernels().await.unwrap();
        let sources = common_kernels.kernel_sources().await;
        
        let vector_add_source = sources.get("vector_add").unwrap();
        match vector_add_source {
            KernelSource::Generic(code) => {
                assert!(code.contains("vector_add"));
                assert!(code.contains("c[gid] = a[gid] + b[gid]"));
            }
            _ => panic!("Expected generic kernel source"),
        }
        
        let vector_mul_source = sources.get("vector_mul").unwrap();
        match vector_mul_source {
            KernelSource::Generic(code) => {
                assert!(code.contains("vector_mul"));
                assert!(code.contains("c[gid] = a[gid] * b[gid]"));
            }
            _ => panic!("Expected generic kernel source"),
        }
    }

    #[tokio::test]
    async fn test_matrix_operations() {
        let common_kernels = create_test_common_kernels().await.unwrap();
        let sources = common_kernels.kernel_sources().await;
        
        let matrix_mul_source = sources.get("matrix_mul").unwrap();
        match matrix_mul_source {
            KernelSource::Generic(code) => {
                assert!(code.contains("matrix_mul"));
                assert!(code.contains("__local float As"));
                assert!(code.contains("__local float Bs"));
                assert!(code.contains("barrier(CLK_LOCAL_MEM_FENCE)"));
            }
            _ => panic!("Expected generic kernel source"),
        }
        
        let transpose_source = sources.get("transpose").unwrap();
        match transpose_source {
            KernelSource::Generic(code) => {
                assert!(code.contains("transpose"));
                assert!(code.contains("__local float tile"));
            }
            _ => panic!("Expected generic kernel source"),
        }
    }

    #[tokio::test]
    async fn test_reduction_kernels() {
        let common_kernels = create_test_common_kernels().await.unwrap();
        let sources = common_kernels.kernel_sources().await;
        
        let reduction_source = sources.get("reduction_sum").unwrap();
        match reduction_source {
            KernelSource::Generic(code) => {
                assert!(code.contains("reduction_sum"));
                assert!(code.contains("__local float* local_sum"));
                assert!(code.contains("stride /= 2"));
            }
            _ => panic!("Expected generic kernel source"),
        }
        
        let prefix_sum_source = sources.get("prefix_sum").unwrap();
        match prefix_sum_source {
            KernelSource::Generic(code) => {
                assert!(code.contains("prefix_sum"));
                assert!(code.contains("Up-sweep"));
                assert!(code.contains("Down-sweep"));
            }
            _ => panic!("Expected generic kernel source"),
        }
    }

    #[tokio::test]
    async fn test_sorting_kernels() {
        let common_kernels = create_test_common_kernels().await.unwrap();
        let sources = common_kernels.kernel_sources().await;
        
        let parallel_sort_source = sources.get("parallel_sort").unwrap();
        match parallel_sort_source {
            KernelSource::Generic(code) => {
                assert!(code.contains("parallel_sort"));
                assert!(code.contains("Bitonic"));
                assert!(code.contains("distance"));
            }
            _ => panic!("Expected generic kernel source"),
        }
    }

    #[tokio::test]
    async fn test_histogram_kernel() {
        let common_kernels = create_test_common_kernels().await.unwrap();
        let sources = common_kernels.kernel_sources().await;
        
        let histogram_source = sources.get("histogram").unwrap();
        match histogram_source {
            KernelSource::Generic(code) => {
                assert!(code.contains("histogram"));
                assert!(code.contains("__local uint* local_hist"));
                assert!(code.contains("atomic_inc"));
                assert!(code.contains("atomic_add"));
            }
            _ => panic!("Expected generic kernel source"),
        }
    }

    #[tokio::test]
    async fn test_all_kernels_have_valid_structure() {
        let common_kernels = create_test_common_kernels().await.unwrap();
        let sources = common_kernels.kernel_sources().await;
        
        for (name, source) in sources {
            match source {
                KernelSource::Generic(code) => {
                    assert!(!code.is_empty(), "Kernel {} has empty source", name);
                    assert!(code.contains("__kernel"), "Kernel {} missing __kernel directive", name);
                    assert!(code.contains("get_global_id") || code.contains("get_local_id"), 
                           "Kernel {} missing thread ID logic", name);
                }
                _ => {}
            }
        }
    }
}