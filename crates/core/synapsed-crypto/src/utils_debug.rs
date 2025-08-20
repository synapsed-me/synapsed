//! Debug version of utils.rs with tracing for compression bug

use subtle::{Choice, ConditionallySelectable, ConstantTimeEq};
use zeroize::Zeroize;
use crate::error::{Error, Result};

// Same utility functions as before...
#[inline]
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).unwrap_u8() == 1
}

// ... (other unchanged functions) ...

/// Compress and encode a polynomial with debug output
pub fn compress_poly_debug(coeffs: &[i16], d: usize, bytes: &mut [u8]) -> Result<()> {
    println!("[DEBUG] compress_poly called with d={}, coeffs.len()={}", d, coeffs.len());
    
    match d {
        10 => {
            compress_poly_10bit_debug(coeffs, bytes);
            Ok(())
        }
        _ => {
            // Call regular versions for other compressions
            match d {
                4 => compress_poly_4bit(coeffs, bytes),
                5 => compress_poly_5bit(coeffs, bytes),
                11 => compress_poly_11bit(coeffs, bytes),
                _ => return Err(Error::UnsupportedCompression),
            }
            Ok(())
        }
    }
}

/// Decompress and decode a polynomial with debug output
pub fn decompress_poly_debug(bytes: &[u8], d: usize, coeffs: &mut [i16]) -> Result<()> {
    println!("[DEBUG] decompress_poly called with d={}, bytes.len()={}", d, bytes.len());
    
    match d {
        10 => {
            decompress_poly_10bit_debug(bytes, coeffs);
            Ok(())
        }
        _ => {
            // Call regular versions for other decompressions
            match d {
                4 => decompress_poly_4bit(bytes, coeffs),
                5 => decompress_poly_5bit(bytes, coeffs),
                11 => decompress_poly_11bit(bytes, coeffs),
                _ => return Err(Error::UnsupportedCompression),
            }
            Ok(())
        }
    }
}

fn compress_poly_10bit_debug(coeffs: &[i16], bytes: &mut [u8]) {
    println!("[DEBUG] compress_poly_10bit_debug: Processing {} coefficients", coeffs.len());
    
    // Show first few coefficients
    if coeffs.len() >= 4 {
        println!("[DEBUG] First 4 coefficients: {:?}", &coeffs[0..4]);
    }
    
    // 10-bit compression: 4 coefficients -> 5 bytes
    for (i, chunk) in coeffs.chunks(4).enumerate() {
        let mut bits = [0u8; 5];
        
        if i == 0 {
            println!("[DEBUG] Processing first chunk: {:?}", chunk);
        }
        
        for j in 0..4.min(chunk.len()) {
            let coeff = if chunk[j] < 0 {
                (chunk[j] + 3329) as u32
            } else {
                chunk[j] as u32
            };
            
            // THE COMPRESSION FORMULA WITH ROUNDING CONSTANT 1664
            let compressed = ((coeff.wrapping_mul(1024).wrapping_add(1664)) / 3329) & 0x3FF;
            
            if i == 0 && j == 0 {
                println!("[DEBUG] First coefficient compression:");
                println!("  Original: {}", chunk[j]);
                println!("  Adjusted (if negative): {}", coeff);
                println!("  Formula: ({} * 1024 + 1664) / 3329", coeff);
                println!("  Compressed: {}", compressed);
            }
            
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

fn decompress_poly_10bit_debug(bytes: &[u8], coeffs: &mut [i16]) {
    println!("[DEBUG] decompress_poly_10bit_debug: Processing {} bytes", bytes.len());
    
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
            
            // THE BUG: DECOMPRESSION FORMULA WITH WRONG ROUNDING CONSTANT 512
            coeffs[i * 4 + j] = ((compressed * 3329 + 512) / 1024) as i16;
            
            if i == 0 && j == 0 {
                println!("[DEBUG] First coefficient decompression:");
                println!("  Compressed: {}", compressed);
                println!("  Formula (BUGGY): ({} * 3329 + 512) / 1024", compressed);
                println!("  Decompressed: {}", coeffs[i * 4 + j]);
                println!("  CORRECT formula would be: ({} * 3329 + 1664) / 1024", compressed);
                println!("  CORRECT result would be: {}", ((compressed * 3329 + 1664) / 1024));
            }
        }
    }
    
    // Show first few decompressed coefficients
    if coeffs.len() >= 4 {
        println!("[DEBUG] First 4 decompressed coefficients: {:?}", &coeffs[0..4]);
    }
}

// Include the regular compression functions
fn compress_poly_4bit(coeffs: &[i16], bytes: &mut [u8]) {
    let required_bytes = (coeffs.len() + 1) / 2;
    debug_assert!(bytes.len() >= required_bytes, "Insufficient output buffer size");
    
    for (i, chunk) in coeffs.chunks(2).enumerate() {
        if i >= bytes.len() {
            break;
        }
        let coeff0 = if chunk[0] < 0 {
            (chunk[0] + 3329) as u32
        } else {
            chunk[0] as u32
        };
        let c0 = ((coeff0.wrapping_mul(16).wrapping_add(1664)) / 3329) as u8;
        let c1 = if chunk.len() > 1 {
            let coeff1 = if chunk[1] < 0 {
                (chunk[1] + 3329) as u32
            } else {
                chunk[1] as u32
            };
            ((coeff1.wrapping_mul(16).wrapping_add(1664)) / 3329) as u8
        } else {
            0
        };
        bytes[i] = (c1 << 4) | (c0 & 0x0F);
    }
}

fn decompress_poly_4bit(bytes: &[u8], coeffs: &mut [i16]) {
    let max_coeffs = bytes.len() * 2;
    debug_assert!(coeffs.len() >= max_coeffs.min(256), "Insufficient coefficient buffer size");
    
    for (i, &byte) in bytes.iter().enumerate() {
        let c0 = (byte & 0x0F) as u32;
        let c1 = (byte >> 4) as u32;
        
        if 2*i < coeffs.len() {
            coeffs[2*i] = ((c0 * 3329 + 8) / 16) as i16;
        }
        if 2*i + 1 < coeffs.len() {
            coeffs[2*i + 1] = ((c1 * 3329 + 8) / 16) as i16;
        }
    }
}

fn compress_poly_5bit(coeffs: &[i16], bytes: &mut [u8]) {
    for (i, chunk) in coeffs.chunks(8).enumerate() {
        let mut bits = [0u8; 5];
        
        for j in 0..8.min(chunk.len()) {
            let coeff = if chunk[j] < 0 {
                (chunk[j] + 3329) as u32
            } else {
                chunk[j] as u32
            };
            let compressed = ((coeff.wrapping_mul(32).wrapping_add(1664)) / 3329) & 0x1F;
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
            
            coeffs[i * 8 + j] = ((compressed * 3329 + 16) / 32) as i16;
        }
    }
}

fn compress_poly_11bit(coeffs: &[i16], bytes: &mut [u8]) {
    for (i, chunk) in coeffs.chunks(8).enumerate() {
        let mut bits = [0u8; 11];
        
        for j in 0..8.min(chunk.len()) {
            let coeff = if chunk[j] < 0 {
                (chunk[j] + 3329) as u32
            } else {
                chunk[j] as u32
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
            
            coeffs[i * 8 + j] = ((compressed * 3329 + 1024) / 2048) as i16;
        }
    }
}