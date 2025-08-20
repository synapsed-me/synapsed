//! CPU fallback for general compute operations.

use rayon::prelude::*;
use tracing::{debug, info};

use crate::{Result, GpuError};

/// CPU fallback for general compute operations.
#[derive(Debug)]
pub struct ComputeFallback;

impl ComputeFallback {
    pub fn new() -> Self {
        info!("Creating general compute CPU fallback processor");
        Self
    }

    /// Parallel vector addition.
    pub fn vector_add(&self, a: &[f32], b: &[f32]) -> Result<Vec<f32>> {
        if a.len() != b.len() {
            return Err(GpuError::FallbackError {
                message: "Vector lengths must match".to_string(),
            });
        }

        let result = a.par_iter()
            .zip(b.par_iter())
            .map(|(x, y)| x + y)
            .collect();

        Ok(result)
    }

    /// Parallel matrix multiplication.
    pub fn matrix_multiply(&self, a: &[f32], b: &[f32], m: usize, n: usize, k: usize) -> Result<Vec<f32>> {
        if a.len() != m * n || b.len() != n * k {
            return Err(GpuError::FallbackError {
                message: "Matrix dimension mismatch".to_string(),
            });
        }

        let mut result = vec![0.0f32; m * k];

        result.par_chunks_mut(k)
            .enumerate()
            .for_each(|(i, row)| {
                for j in 0..k {
                    let mut sum = 0.0;
                    for l in 0..n {
                        sum += a[i * n + l] * b[l * k + j];
                    }
                    row[j] = sum;
                }
            });

        Ok(result)
    }
}