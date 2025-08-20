//! Traffic obfuscation for hiding network patterns.

use crate::error::{NetworkError, PrivacyError, Result};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use rand::Rng;

/// Obfuscation methods available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObfuscationMethod {
    /// No obfuscation
    None,
    /// Traffic padding
    Padding,
    /// Traffic shaping
    Shaping,
    /// Dummy traffic injection
    DummyTraffic,
    /// Packet size normalization
    SizeNormalization,
    /// Timing obfuscation
    TimingObfuscation,
}

/// Traffic obfuscation state.
#[derive(Debug, Clone)]
pub struct ObfuscationState {
    /// Current obfuscation method
    method: ObfuscationMethod,
    /// Padding parameters
    padding_params: PaddingParams,
    /// Last activity timestamp
    last_activity: SystemTime,
    /// Bytes sent without padding
    real_bytes_sent: u64,
    /// Total bytes sent (including padding)
    total_bytes_sent: u64,
}

/// Parameters for traffic padding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaddingParams {
    /// Minimum packet size
    pub min_packet_size: usize,
    /// Maximum packet size
    pub max_packet_size: usize,
    /// Padding distribution
    pub distribution: PaddingDistribution,
    /// Padding rate (packets per second)
    pub padding_rate: f64,
}

/// Padding distribution types.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PaddingDistribution {
    /// Uniform distribution
    Uniform,
    /// Exponential distribution
    Exponential,
    /// Normal distribution
    Normal,
    /// Custom distribution
    Custom,
}

impl Default for PaddingParams {
    fn default() -> Self {
        Self {
            min_packet_size: 256,
            max_packet_size: 1500,
            distribution: PaddingDistribution::Uniform,
            padding_rate: 1.0,
        }
    }
}

impl ObfuscationState {
    /// Creates a new obfuscation state.
    pub fn new(method: ObfuscationMethod) -> Self {
        Self {
            method,
            padding_params: PaddingParams::default(),
            last_activity: SystemTime::now(),
            real_bytes_sent: 0,
            total_bytes_sent: 0,
        }
    }
    
    /// Applies obfuscation to outgoing data.
    pub fn obfuscate_outgoing(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        match self.method {
            ObfuscationMethod::None => Ok(data.to_vec()),
            ObfuscationMethod::Padding => self.apply_padding(data),
            ObfuscationMethod::Shaping => self.apply_shaping(data),
            ObfuscationMethod::DummyTraffic => self.inject_dummy_traffic(data),
            ObfuscationMethod::SizeNormalization => self.normalize_size(data),
            ObfuscationMethod::TimingObfuscation => self.obfuscate_timing(data),
        }
    }
    
    /// Removes obfuscation from incoming data.
    pub fn deobfuscate_incoming(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        match self.method {
            ObfuscationMethod::None => Ok(data.to_vec()),
            ObfuscationMethod::Padding => self.remove_padding(data),
            ObfuscationMethod::Shaping => self.remove_shaping(data),
            ObfuscationMethod::DummyTraffic => self.filter_dummy_traffic(data),
            ObfuscationMethod::SizeNormalization => self.denormalize_size(data),
            ObfuscationMethod::TimingObfuscation => self.remove_timing_obfuscation(data),
        }
    }
    
    /// Applies padding to data.
    fn apply_padding(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let target_size = self.calculate_padded_size(data.len());
        
        if target_size <= data.len() {
            return Ok(data.to_vec());
        }
        
        let padding_size = target_size - data.len();
        let mut padded = Vec::with_capacity(target_size);
        
        // Add original data
        padded.extend_from_slice(data);
        
        // Add padding header (4 bytes for original length)
        let original_len = data.len() as u32;
        padded.extend_from_slice(&original_len.to_be_bytes());
        
        // Add random padding
        let mut rng = rand::thread_rng();
        for _ in 0..(padding_size - 4) {
            padded.push(rng.gen());
        }
        
        self.real_bytes_sent += data.len() as u64;
        self.total_bytes_sent += padded.len() as u64;
        
        Ok(padded)
    }
    
    /// Removes padding from data.
    fn remove_padding(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 4 {
            return Err(NetworkError::Privacy(PrivacyError::AnonymizationFailed(
                "Data too small to contain padding header".to_string()
            )));
        }
        
        // Extract original length from padding header
        let header_start = data.len() - 4;
        let original_len_bytes = &data[header_start..];
        let original_len = u32::from_be_bytes([
            original_len_bytes[0],
            original_len_bytes[1],
            original_len_bytes[2],
            original_len_bytes[3],
        ]) as usize;
        
        if original_len > header_start {
            return Err(NetworkError::Privacy(PrivacyError::AnonymizationFailed(
                "Invalid padding header".to_string()
            )));
        }
        
        Ok(data[..original_len].to_vec())
    }
    
    /// Applies traffic shaping.
    fn apply_shaping(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        // For now, just return the data as-is
        // In production, this would implement traffic shaping algorithms
        Ok(data.to_vec())
    }
    
    /// Removes traffic shaping.
    fn remove_shaping(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
    
    /// Injects dummy traffic.
    fn inject_dummy_traffic(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        // For now, just return the data as-is
        // In production, this would inject dummy packets
        Ok(data.to_vec())
    }
    
    /// Filters out dummy traffic.
    fn filter_dummy_traffic(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
    
    /// Normalizes packet size.
    fn normalize_size(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let target_size = self.padding_params.max_packet_size;
        
        if data.len() >= target_size {
            return Ok(data.to_vec());
        }
        
        let mut normalized = Vec::with_capacity(target_size);
        normalized.extend_from_slice(data);
        
        // Add length header
        let original_len = data.len() as u32;
        normalized.extend_from_slice(&original_len.to_be_bytes());
        
        // Pad to target size
        let padding_needed = target_size - normalized.len();
        normalized.resize(target_size, 0);
        
        // Fill padding with random data
        let mut rng = rand::thread_rng();
        for i in (target_size - padding_needed)..target_size {
            normalized[i] = rng.gen();
        }
        
        Ok(normalized)
    }
    
    /// Denormalizes packet size.
    fn denormalize_size(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 4 {
            return Err(NetworkError::Privacy(PrivacyError::AnonymizationFailed(
                "Data too small for denormalization".to_string()
            )));
        }
        
        // Find the length header (last 4 bytes of original data)
        let mut original_len = None;
        for i in 0..(data.len() - 3) {
            let len_bytes = &data[i..i + 4];
            let len = u32::from_be_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;
            
            if len == i && len <= data.len() {
                original_len = Some(len);
                break;
            }
        }
        
        match original_len {
            Some(len) => Ok(data[..len].to_vec()),
            None => Err(NetworkError::Privacy(PrivacyError::AnonymizationFailed(
                "Could not find valid length header".to_string()
            ))),
        }
    }
    
    /// Applies timing obfuscation.
    fn obfuscate_timing(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        // Add a small random delay (simulated by modifying the timestamp)
        self.last_activity = SystemTime::now();
        Ok(data.to_vec())
    }
    
    /// Removes timing obfuscation.
    fn remove_timing_obfuscation(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
    
    /// Calculates the target size for padding.
    fn calculate_padded_size(&self, original_size: usize) -> usize {
        match self.padding_params.distribution {
            PaddingDistribution::Uniform => {
                let min = self.padding_params.min_packet_size;
                let max = self.padding_params.max_packet_size;
                
                if original_size >= max {
                    return original_size;
                }
                
                let mut rng = rand::thread_rng();
                rng.gen_range(original_size.max(min)..=max)
            }
            PaddingDistribution::Exponential => {
                // Simple exponential distribution approximation
                let base = original_size.max(self.padding_params.min_packet_size);
                let multiplier = 1.0 + rand::thread_rng().gen::<f64>() * 0.5; // 1.0 to 1.5x
                (base as f64 * multiplier).min(self.padding_params.max_packet_size as f64) as usize
            }
            PaddingDistribution::Normal => {
                // Simple normal distribution approximation
                let mean = (self.padding_params.min_packet_size + self.padding_params.max_packet_size) / 2;
                let variance = (self.padding_params.max_packet_size - self.padding_params.min_packet_size) / 4;
                
                let mut rng = rand::thread_rng();
                let offset: i32 = rng.gen_range(-(variance as i32)..=(variance as i32));
                ((mean as i32 + offset).max(original_size as i32).max(self.padding_params.min_packet_size as i32)
                    .min(self.padding_params.max_packet_size as i32)) as usize
            }
            PaddingDistribution::Custom => {
                // Default to uniform for now
                self.calculate_padded_size(original_size)
            }
        }
    }
    
    /// Gets obfuscation statistics.
    pub fn get_stats(&self) -> ObfuscationStats {
        let overhead = if self.real_bytes_sent > 0 {
            (self.total_bytes_sent - self.real_bytes_sent) as f64 / self.real_bytes_sent as f64
        } else {
            0.0
        };
        
        ObfuscationStats {
            method: self.method,
            real_bytes_sent: self.real_bytes_sent,
            total_bytes_sent: self.total_bytes_sent,
            overhead_ratio: overhead,
            last_activity: self.last_activity,
        }
    }
    
    /// Updates padding parameters.
    pub fn update_padding_params(&mut self, params: PaddingParams) {
        self.padding_params = params;
    }
}

/// Obfuscation statistics.
#[derive(Debug, Clone, Serialize)]
pub struct ObfuscationStats {
    /// Current obfuscation method
    pub method: ObfuscationMethod,
    /// Real bytes sent (without obfuscation)
    pub real_bytes_sent: u64,
    /// Total bytes sent (with obfuscation)
    pub total_bytes_sent: u64,
    /// Overhead ratio (padding/real data)
    pub overhead_ratio: f64,
    /// Last activity timestamp
    pub last_activity: SystemTime,
}

impl Default for ObfuscationState {
    fn default() -> Self {
        Self::new(ObfuscationMethod::Padding)
    }
}