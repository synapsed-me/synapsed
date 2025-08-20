//! Utility functions for cryptographic operations
//!
//! This module provides common utility functions used throughout
//! the library, with a focus on constant-time operations.

use subtle::{Choice, ConditionallySelectable, ConstantTimeEq};
use zeroize::Zeroize;
use crate::error::{Error, Result};

/// Constant-time comparison of byte slices
#[inline]
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).unwrap_u8() == 1
}

/// Constant-time conditional selection of bytes
#[inline]
pub fn ct_select(a: &[u8], b: &[u8], choice: Choice, out: &mut [u8]) {
    debug_assert_eq!(a.len(), b.len());
    debug_assert_eq!(a.len(), out.len());
    
    for ((out_byte, &a_byte), &b_byte) in out.iter_mut().zip(a.iter()).zip(b.iter()) {
        *out_byte = u8::conditional_select(&a_byte, &b_byte, choice);
    }
}

/// Convert a little-endian byte array to u16
#[inline]
pub fn bytes_to_u16_le(bytes: &[u8]) -> u16 {
    debug_assert!(bytes.len() >= 2);
    u16::from_le_bytes([bytes[0], bytes[1]])
}

/// Convert a u16 to little-endian bytes
#[inline]
pub fn u16_to_bytes_le(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}

/// Convert a little-endian byte array to u32
#[inline]
pub fn bytes_to_u32_le(bytes: &[u8]) -> u32 {
    debug_assert!(bytes.len() >= 4);
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Convert a u32 to little-endian bytes
#[inline]
pub fn u32_to_bytes_le(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

/// Encode polynomial coefficients to bytes (Kyber-specific 12-bit packing)
pub fn encode_poly(coeffs: &[i16], bytes: &mut [u8]) {
    // For Kyber, we pack 256 coefficients into 384 bytes (12 bits per coefficient)
    if coeffs.len() == 256 && bytes.len() == 384 {
        // Pack two coefficients into 3 bytes
        for i in 0..128 {
            let c0 = coeffs[2*i] as u16;
            let c1 = coeffs[2*i + 1] as u16;
            
            bytes[3*i] = c0 as u8;
            bytes[3*i + 1] = ((c0 >> 8) | (c1 << 4)) as u8;
            bytes[3*i + 2] = (c1 >> 4) as u8;
        }
    } else {
        // Fallback to simple 16-bit encoding
        debug_assert_eq!(coeffs.len() * 2, bytes.len());
        
        for (i, &coeff) in coeffs.iter().enumerate() {
            let b = u16_to_bytes_le(coeff as u16);
            bytes[2*i..2*i + 2].copy_from_slice(&b);
        }
    }
}

/// Decode bytes to polynomial coefficients (Kyber-specific 12-bit unpacking)
pub fn decode_poly(bytes: &[u8], coeffs: &mut [i16]) {
    // For Kyber, we unpack 384 bytes into 256 coefficients (12 bits per coefficient)
    if bytes.len() == 384 && coeffs.len() == 256 {
        // Unpack 3 bytes into two coefficients
        for i in 0..128 {
            let b0 = bytes[3*i] as u16;
            let b1 = bytes[3*i + 1] as u16;
            let b2 = bytes[3*i + 2] as u16;
            
            coeffs[2*i] = (b0 | ((b1 & 0x0F) << 8)) as i16;
            coeffs[2*i + 1] = ((b1 >> 4) | (b2 << 4)) as i16;
        }
    } else {
        // Fallback to simple 16-bit decoding
        debug_assert_eq!(bytes.len(), coeffs.len() * 2);
        
        for (i, coeff) in coeffs.iter_mut().enumerate() {
            *coeff = bytes_to_u16_le(&bytes[2*i..2*i+2]) as i16;
        }
    }
}

/// Compress and encode a polynomial
pub fn compress_poly(coeffs: &[i16], d: usize, bytes: &mut [u8]) -> Result<()> {
    // Implementation depends on compression parameter d
    // This is a placeholder for the actual compression logic
    match d {
        4 => {
            compress_poly_4bit(coeffs, bytes);
            Ok(())
        }
        5 => {
            compress_poly_5bit(coeffs, bytes);
            Ok(())
        }
        10 => {
            compress_poly_10bit(coeffs, bytes);
            Ok(())
        }
        11 => {
            compress_poly_11bit(coeffs, bytes);
            Ok(())
        }
        _ => Err(Error::UnsupportedCompression),
    }
}

/// Decompress and decode a polynomial
pub fn decompress_poly(bytes: &[u8], d: usize, coeffs: &mut [i16]) -> Result<()> {
    match d {
        4 => {
            decompress_poly_4bit(bytes, coeffs);
            Ok(())
        }
        5 => {
            decompress_poly_5bit(bytes, coeffs);
            Ok(())
        }
        10 => {
            decompress_poly_10bit(bytes, coeffs);
            Ok(())
        }
        11 => {
            decompress_poly_11bit(bytes, coeffs);
            Ok(())
        }
        _ => Err(Error::UnsupportedCompression),
    }
}

/// 4-bit compression for Kyber (used in Kyber512)
/// Compresses coefficients from Zq to 4 bits using the formula:
/// compress(x) = round((2^4/q) * x) mod 2^4
fn compress_poly_4bit(coeffs: &[i16], bytes: &mut [u8]) {
    // Ensure we have enough space (2 coefficients per byte)
    let required_bytes = coeffs.len().div_ceil(2);
    debug_assert!(bytes.len() >= required_bytes, "Insufficient output buffer size");
    
    // Clear the output buffer
    bytes[..required_bytes].fill(0);
    
    for (i, chunk) in coeffs.chunks(2).enumerate() {
        if i >= bytes.len() {
            break; // Prevent buffer overflow
        }
        
        
        // Compress first coefficient
        // Formula: compress(x) = round((16/3329) * x) mod 16
        // To avoid floating point, we use: round((16 * x) / 3329)
        // Which is equivalent to: ((16 * x + 3329/2) / 3329) & 0x0F
        let c0 = {
            // Ensure coefficient is in range [0, q)
            let mut x = chunk[0] as i32;
            if x < 0 {
                x = ((x % 3329) + 3329) % 3329;
            } else {
                x %= 3329;
            }
            // Compress: round((16 * x) / 3329)
            ((16u32 * (x as u32) + 1665) / 3329) & 0x0F
        };
        
        // Compress second coefficient (if exists)
        let c1 = if chunk.len() > 1 {
            let mut x = chunk[1] as i32;
            if x < 0 {
                x = ((x % 3329) + 3329) % 3329;
            } else {
                x %= 3329;
            }
            ((16u32 * (x as u32) + 1665) / 3329) & 0x0F
        } else {
            0
        };
        
        
        // Pack two 4-bit values into one byte
        // c0 goes in low nibble, c1 goes in high nibble
        bytes[i] = ((c1 as u8) << 4) | (c0 as u8);
    }
}

/// 4-bit decompression for Kyber (used in Kyber512)
/// Decompresses 4-bit values back to Zq using the formula:
/// decompress(x) = round((q/2^4) * x)
fn decompress_poly_4bit(bytes: &[u8], coeffs: &mut [i16]) {
    // Ensure we have enough coefficients space
    let max_coeffs = bytes.len() * 2;
    debug_assert!(coeffs.len() >= max_coeffs.min(256), "Insufficient coefficient buffer size");
    
    // Clear the output buffer to ensure clean state
    coeffs.fill(0);
    
    
    for (i, &byte) in bytes.iter().enumerate() {
        // Extract two 4-bit values
        let c0 = (byte & 0x0F) as u32;
        let c1 = (byte >> 4) as u32;
        
        
        // Decompress first coefficient
        // Formula: decompress(x) = round((3329/16) * x)
        // To avoid floating point: round((3329 * x) / 16)
        // Which is: (3329 * x + 8) / 16
        if 2*i < coeffs.len() {
            coeffs[2*i] = ((3329u32 * c0 + 8) / 16) as i16;
        }
        
        // Decompress second coefficient
        if 2*i + 1 < coeffs.len() {
            coeffs[2*i + 1] = ((3329u32 * c1 + 8) / 16) as i16;
        }
    }
}

fn compress_poly_5bit(coeffs: &[i16], bytes: &mut [u8]) {
    // 5-bit compression: 8 coefficients -> 5 bytes
    for (i, chunk) in coeffs.chunks(8).enumerate() {
        let mut bits = [0u8; 5];
        
        // Compress each coefficient to 5 bits
        for (j, &coeff_val) in chunk.iter().enumerate().take(8) {
            // Handle negative coefficients properly
            let coeff = if coeff_val < 0 {
                (coeff_val + 3329) as u32
            } else {
                coeff_val as u32
            };
            let compressed = ((coeff.wrapping_mul(32).wrapping_add(1664)) / 3329) & 0x1F;
            // Pack into the 5-byte output
            let bit_offset = j * 5;
            let byte_offset = bit_offset / 8;
            let bit_shift = bit_offset % 8;
            
            bits[byte_offset] |= (compressed << bit_shift) as u8;
            if byte_offset + 1 < 5 && bit_shift > 3 {
                bits[byte_offset + 1] |= (compressed >> (8 - bit_shift)) as u8;
            }
        }
        
        bytes[i*5..(i+1)*5].copy_from_slice(&bits);
    }
}

fn decompress_poly_5bit(bytes: &[u8], coeffs: &mut [i16]) {
    // 5-bit decompression: 5 bytes -> 8 coefficients
    for (i, chunk) in bytes.chunks(5).enumerate() {
        for j in 0..8 {
            if i * 8 + j >= coeffs.len() {
                break;
            }
            
            let bit_offset = j * 5;
            let byte_offset = bit_offset / 8;
            let bit_shift = bit_offset % 8;
            
            let mut compressed = (chunk[byte_offset] >> bit_shift) as u32;
            if byte_offset + 1 < 5 && bit_shift > 3 {
                compressed |= (chunk[byte_offset + 1] << (8 - bit_shift)) as u32;
            }
            compressed &= 0x1F;
            
            // Decompress from 5 bits to full coefficient
            coeffs[i * 8 + j] = ((compressed * 3329 + 16) / 32) as i16;
        }
    }
}

fn compress_poly_10bit(coeffs: &[i16], bytes: &mut [u8]) {
    // 10-bit compression: 4 coefficients -> 5 bytes
    for (i, chunk) in coeffs.chunks(4).enumerate() {
        let mut bits = [0u8; 5];
        
        for (j, &coeff_val) in chunk.iter().enumerate().take(4) {
            // Handle negative coefficients properly
            let coeff = if coeff_val < 0 {
                (coeff_val + 3329) as u32
            } else {
                coeff_val as u32
            };
            let compressed = ((coeff.wrapping_mul(1024).wrapping_add(1664)) / 3329) & 0x3FF;
            let bit_offset = j * 10;
            let byte_offset = bit_offset / 8;
            let bit_shift = bit_offset % 8;
            
            bits[byte_offset] |= (compressed << bit_shift) as u8;
            if byte_offset + 1 < 5 {
                bits[byte_offset + 1] |= (compressed >> (8 - bit_shift)) as u8;
                if byte_offset + 2 < 5 && bit_shift > 6 {
                    bits[byte_offset + 2] |= (compressed >> (16 - bit_shift)) as u8;
                }
            }
        }
        
        bytes[i*5..(i+1)*5].copy_from_slice(&bits);
    }
}

fn decompress_poly_10bit(bytes: &[u8], coeffs: &mut [i16]) {
    // 10-bit decompression: 5 bytes -> 4 coefficients
    for (i, chunk) in bytes.chunks(5).enumerate() {
        for j in 0..4 {
            if i * 4 + j >= coeffs.len() {
                break;
            }
            
            let bit_offset = j * 10;
            let byte_offset = bit_offset / 8;
            let bit_shift = bit_offset % 8;
            
            let mut compressed = (chunk[byte_offset] >> bit_shift) as u32;
            if byte_offset + 1 < 5 {
                compressed |= (chunk[byte_offset + 1] as u32) << (8 - bit_shift);
                if byte_offset + 2 < 5 && bit_shift > 6 {
                    compressed |= (chunk[byte_offset + 2] as u32) << (16 - bit_shift);
                }
            }
            compressed &= 0x3FF;
            
            // Decompress from 10 bits to full coefficient
            coeffs[i * 4 + j] = ((compressed * 3329 + 1664) / 1024) as i16;
        }
    }
}

fn compress_poly_11bit(coeffs: &[i16], bytes: &mut [u8]) {
    // 11-bit compression: 8 coefficients -> 11 bytes
    for (i, chunk) in coeffs.chunks(8).enumerate() {
        let mut bits = [0u8; 11];
        
        for (j, &coeff_val) in chunk.iter().enumerate().take(8) {
            // Handle negative coefficients properly  
            let coeff = if coeff_val < 0 {
                (coeff_val + 3329) as u32
            } else {
                coeff_val as u32
            };
            let compressed = ((coeff.wrapping_mul(2048).wrapping_add(1664)) / 3329) & 0x7FF;
            let bit_offset = j * 11;
            let byte_offset = bit_offset / 8;
            let bit_shift = bit_offset % 8;
            
            bits[byte_offset] |= (compressed << bit_shift) as u8;
            if byte_offset + 1 < 11 {
                bits[byte_offset + 1] |= (compressed >> (8 - bit_shift)) as u8;
                if byte_offset + 2 < 11 && bit_shift > 5 {
                    bits[byte_offset + 2] |= (compressed >> (16 - bit_shift)) as u8;
                }
            }
        }
        
        bytes[i*11..(i+1)*11].copy_from_slice(&bits);
    }
}

fn decompress_poly_11bit(bytes: &[u8], coeffs: &mut [i16]) {
    // 11-bit decompression: 11 bytes -> 8 coefficients
    for (i, chunk) in bytes.chunks(11).enumerate() {
        for j in 0..8 {
            if i * 8 + j >= coeffs.len() {
                break;
            }
            
            let bit_offset = j * 11;
            let byte_offset = bit_offset / 8;
            let bit_shift = bit_offset % 8;
            
            let mut compressed = (chunk[byte_offset] >> bit_shift) as u32;
            if byte_offset + 1 < 11 {
                compressed |= (chunk[byte_offset + 1] as u32) << (8 - bit_shift);
                if byte_offset + 2 < 11 && bit_shift > 5 {
                    compressed |= (chunk[byte_offset + 2] as u32) << (16 - bit_shift);
                }
            }
            compressed &= 0x7FF;
            
            // Decompress from 11 bits to full coefficient
            coeffs[i * 8 + j] = ((compressed * 3329 + 1024) / 2048) as i16;
        }
    }
}

/// Compress a polynomial vector
pub fn compress_polyvec(polyvec: &[[i16; 256]], d: usize) -> Result<Vec<u8>> {
    let k = polyvec.len();
    let compressed_size = k * d * 256 / 8;
    let mut compressed = vec![0u8; compressed_size];
    
    for (i, poly) in polyvec.iter().enumerate().take(k) {
        let offset = i * d * 256 / 8;
        compress_poly(poly, d, &mut compressed[offset..offset + d * 256 / 8])?;
    }
    
    Ok(compressed)
}

/// Decompress a polynomial vector
pub fn decompress_polyvec(bytes: &[u8], d: usize, polyvec: &mut [[i16; 256]]) -> Result<()> {
    let k = polyvec.len();
    let expected_size = k * d * 256 / 8;
    
    if bytes.len() != expected_size {
        return Err(Error::InvalidSize);
    }
    
    for (i, poly) in polyvec.iter_mut().enumerate().take(k) {
        let offset = i * d * 256 / 8;
        decompress_poly(&bytes[offset..offset + d * 256 / 8], d, poly)?;
    }
    
    Ok(())
}

/// Securely clear sensitive data
pub fn secure_zero<T: Zeroize>(data: &mut T) {
    data.zeroize();
}

/// Barrett reduction for Kyber modulus q = 3329
#[inline]
pub fn barrett_reduce(a: i16) -> i16 {
    const V: i32 = 20159; // floor(2^26/q + 1/2)
    let t = (V * a as i32 + (1 << 25)) >> 26;
    a - t as i16 * 3329
}

/// Montgomery reduction
#[inline]
pub fn montgomery_reduce(a: i32) -> i16 {
    const QINV: i32 = 62209; // q^{-1} mod 2^16
    const Q: i32 = 3329;
    
    let t = a.wrapping_mul(QINV) as i16 as i32;
    ((a.wrapping_sub(t.wrapping_mul(Q))) >> 16) as i16
}

/// Barrett reduction for Dilithium modulus q = 8380417
#[inline]
pub fn barrett_reduce_dilithium(a: i32) -> i32 {
    const Q: i32 = 8380417; // 2^23 - 2^13 + 1
    const M: i64 = 549755813889; // ceil(2^43/Q)
    
    let t = ((M.wrapping_mul(a as i64)) >> 43) as i32;
    a.wrapping_sub(t.wrapping_mul(Q))
}

/// Montgomery reduction for Dilithium
#[inline]
pub fn montgomery_reduce_dilithium(a: i64) -> i32 {
    const QINV: u32 = 4236238847; // -q^{-1} mod 2^32
    const Q: i32 = 8380417;
    
    let t = (a as i32).wrapping_mul(QINV as i32);
    ((a.wrapping_sub(t as i64 * Q as i64)) >> 32) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ct_eq() {
        let a = [1u8, 2, 3, 4];
        let b = [1u8, 2, 3, 4];
        let c = [1u8, 2, 3, 5];
        
        assert!(ct_eq(&a, &b));
        assert!(!ct_eq(&a, &c));
    }
    
    #[test]
    fn test_barrett_reduce() {
        // Test that barrett_reduce is idempotent for values in range
        for i in -1664..=1664 {
            let reduced = barrett_reduce(i);
            assert!((-1664..=1664).contains(&reduced));
            assert_eq!(barrett_reduce(reduced), reduced);
        }
    }
    
    #[test]
    fn test_compress_decompress_4bit() {
        // Test compression and decompression roundtrip
        let mut coeffs = vec![0i16; 256];
        let mut compressed = vec![0u8; 128]; // 256 coeffs * 4 bits / 8 = 128 bytes
        let mut decompressed = vec![0i16; 256];
        
        // Test with various coefficient values
        // Note: Due to compression loss, we won't get exact values back
        // but they should be close (within the compression error bound)
        for (i, coeff) in coeffs.iter_mut().enumerate().take(256) {
            // Test with values throughout the range [0, q)
            *coeff = ((i * 13) % 3329) as i16;
        }
        
        compress_poly_4bit(&coeffs, &mut compressed);
        
        decompress_poly_4bit(&compressed, &mut decompressed);
        
        // Check that decompressed values are close to original
        // The maximum error for 4-bit compression is q/(2^5) = 3329/32 ≈ 104
        // However, due to wraparound at the boundaries, some values may have larger errors
        for i in 0..256 {
            let orig = coeffs[i];
            let decomp = decompressed[i];
            
            // Calculate the error, accounting for wraparound
            // When a value compresses to 16 (wraps to 0), it decompresses to ~0 instead of ~3329
            let diff = if orig > 3200 && decomp < 200 {
                // This is likely a wraparound case
                // The actual error is much smaller when considering modular arithmetic
                ((orig - decomp + 3329) % 3329).min((decomp - orig + 3329) % 3329)
            } else {
                (orig - decomp).abs()
            };
            
            assert!(diff <= 210, "Coefficient {i} differs by {diff}: {orig} vs {decomp}");
        }
    }
    
    #[test]
    fn test_compress_4bit_edge_cases() {
        // Test edge cases for 4-bit compression
        let mut bytes = vec![0u8; 2];
        
        // Test with q-1
        let coeffs = vec![3328i16, 0];
        compress_poly_4bit(&coeffs, &mut bytes);
        // 3328 * 16 / 3329 = 15.995... rounds to 16, but 16 & 0x0F = 0
        assert_eq!(bytes[0] & 0x0F, 0); // Should compress to 0 (16 mod 16)
        
        // Test with 0
        let coeffs = vec![0i16, 0];
        compress_poly_4bit(&coeffs, &mut bytes);
        assert_eq!(bytes[0], 0); // Should compress to 0
        
        // Test with negative values (should be handled by modular reduction)
        let coeffs = vec![-1i16, -1665];
        compress_poly_4bit(&coeffs, &mut bytes);
        // -1 mod 3329 = 3328, which compresses to 0 (as shown above)
        assert_eq!(bytes[0] & 0x0F, 0);
        // -1665 mod 3329 = 1664, which compresses to (1664 * 16 + 1665) / 3329 = 8
        assert_eq!(bytes[0] >> 4, 8);
    }
    
    #[test]
    fn test_decompress_4bit_all_values() {
        // Test decompression for all possible 4-bit values
        let mut coeffs = vec![0i16; 32];
        let mut bytes = vec![0u8; 16];
        
        // Set up bytes with all possible 4-bit values (0-15)
        for (i, byte) in bytes.iter_mut().enumerate().take(16) {
            *byte = ((i as u8) << 4) | (i as u8);
        }
        
        decompress_poly_4bit(&bytes, &mut coeffs);
        
        // Check decompressed values
        for i in 0..16 {
            let expected = ((3329u32 * i as u32 + 8) / 16) as i16;
            assert_eq!(coeffs[2*i], expected);
            assert_eq!(coeffs[2*i + 1], expected);
        }
    }
}