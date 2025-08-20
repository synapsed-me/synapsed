//! Multi-factor authentication support
//! 
//! Provides:
//! - TOTP (Time-based One-Time Password)
//! - HOTP (HMAC-based One-Time Password)
//! - Backup codes
//! - SMS/Email verification (interfaces)

use crate::{Error, Result};
use sha3::{Sha1, Digest};
use base64;

#[cfg(not(feature = "std"))]
// String and Vec are available in std prelude

/// MFA method types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfaMethod {
    /// Time-based OTP
    Totp,
    /// Counter-based OTP
    Hotp,
    /// SMS verification
    Sms,
    /// Email verification
    Email,
    /// Backup codes
    BackupCode,
}

/// MFA provider trait
pub trait MfaProvider: Send + Sync {
    /// Generate MFA secret
    fn generate_secret(&self) -> Result<MfaSecret>;
    
    /// Verify MFA code
    fn verify_code(&self, secret: &MfaSecret, code: &str) -> Result<bool>;
    
    /// Get MFA method type
    fn method(&self) -> MfaMethod;
}

/// MFA secret data
#[derive(Debug, Clone)]
pub struct MfaSecret {
    /// Secret key
    pub secret: Vec<u8>,
    /// Recovery codes (if applicable)
    pub recovery_codes: Option<Vec<String>>,
    /// Method-specific data
    pub metadata: Option<serde_json::Value>,
}

/// TOTP (Time-based One-Time Password) provider
pub struct TotpProvider {
    /// Time step in seconds (usually 30)
    pub time_step: u64,
    /// Number of digits in OTP (usually 6)
    pub digits: u32,
    /// Issuer name
    pub issuer: String,
}

impl Default for TotpProvider {
    fn default() -> Self {
        Self {
            time_step: 30,
            digits: 6,
            issuer: "Synapsed".to_string(),
        }
    }
}

impl TotpProvider {
    /// Generate provisioning URI for QR code
    pub fn provisioning_uri(&self, secret: &[u8], account: &str) -> String {
        let secret_b32 = base32::encode(base32::Alphabet::RFC4648 { padding: false }, secret);
        format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}&digits={}&period={}",
            self.issuer, account, secret_b32, self.issuer, self.digits, self.time_step
        )
    }
    
    /// Generate TOTP code
    pub fn generate_code(&self, secret: &[u8], time: u64) -> Result<String> {
        let counter = time / self.time_step;
        self.generate_hotp(secret, counter)
    }
    
    /// Generate HOTP code (used internally)
    fn generate_hotp(&self, secret: &[u8], counter: u64) -> Result<String> {
        // Convert counter to bytes
        let counter_bytes = counter.to_be_bytes();
        
        // HMAC-SHA1
        let hash = hmac_sha1(secret, &counter_bytes);
        
        // Dynamic truncation
        let offset = (hash[19] & 0x0f) as usize;
        let truncated = u32::from_be_bytes([
            hash[offset] & 0x7f,
            hash[offset + 1],
            hash[offset + 2],
            hash[offset + 3],
        ]);
        
        // Generate code
        let code = truncated % 10u32.pow(self.digits);
        Ok(format!("{:0width$}", code, width = self.digits as usize))
    }
}

impl MfaProvider for TotpProvider {
    fn generate_secret(&self) -> Result<MfaSecret> {
        // Generate random secret (160 bits for compatibility)
        let mut secret = vec![0u8; 20];
        use rand_core::{RngCore, OsRng};
        OsRng.fill_bytes(&mut secret);
        
        // Generate recovery codes
        let mut recovery_codes = Vec::new();
        for _ in 0..10 {
            let mut code = vec![0u8; 4];
            OsRng.fill_bytes(&mut code);
            let code_num = u32::from_be_bytes([code[0], code[1], code[2], code[3]]) % 100000000;
            recovery_codes.push(format!("{:08}", code_num));
        }
        
        Ok(MfaSecret {
            secret,
            recovery_codes: Some(recovery_codes),
            metadata: Some(serde_json::json!({
                "time_step": self.time_step,
                "digits": self.digits,
                "issuer": self.issuer,
            })),
        })
    }
    
    fn verify_code(&self, secret: &MfaSecret, code: &str) -> Result<bool> {
        let now = chrono::Utc::now().timestamp() as u64;
        
        // Check current and adjacent time windows for clock skew
        for window in -1..=1 {
            let time = (now as i64 + window * self.time_step as i64) as u64;
            let expected = self.generate_code(&secret.secret, time)?;
            if constant_time_eq(expected.as_bytes(), code.as_bytes()) {
                return Ok(true);
            }
        }
        
        // Check recovery codes
        if let Some(recovery_codes) = &secret.recovery_codes {
            for recovery_code in recovery_codes {
                if constant_time_eq(recovery_code.as_bytes(), code.as_bytes()) {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    fn method(&self) -> MfaMethod {
        MfaMethod::Totp
    }
}

/// HOTP (HMAC-based One-Time Password) provider
pub struct HotpProvider {
    /// Number of digits in OTP
    pub digits: u32,
}

impl Default for HotpProvider {
    fn default() -> Self {
        Self { digits: 6 }
    }
}

impl MfaProvider for HotpProvider {
    fn generate_secret(&self) -> Result<MfaSecret> {
        // Generate random secret
        let mut secret = vec![0u8; 20];
        use rand_core::{RngCore, OsRng};
        OsRng.fill_bytes(&mut secret);
        
        Ok(MfaSecret {
            secret,
            recovery_codes: None,
            metadata: Some(serde_json::json!({
                "counter": 0u64,
                "digits": self.digits,
            })),
        })
    }
    
    fn verify_code(&self, secret: &MfaSecret, code: &str) -> Result<bool> {
        // Get current counter from metadata
        let counter = secret.metadata
            .as_ref()
            .and_then(|m| m.get("counter"))
            .and_then(|c| c.as_u64())
            .unwrap_or(0);
        
        // Check current and next few counters
        for i in 0..10 {
            let expected = generate_hotp(&secret.secret, counter + i, self.digits)?;
            if constant_time_eq(expected.as_bytes(), code.as_bytes()) {
                // Note: In a real implementation, we would update the counter
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    fn method(&self) -> MfaMethod {
        MfaMethod::Hotp
    }
}

/// Simple HMAC-SHA1 implementation
fn hmac_sha1(key: &[u8], data: &[u8]) -> Vec<u8> {
    use sha3::Sha1;
    
    let mut ipad = vec![0x36u8; 64];
    let mut opad = vec![0x5cu8; 64];
    
    // XOR key with pads
    for (i, &k) in key.iter().enumerate().take(64) {
        ipad[i] ^= k;
        opad[i] ^= k;
    }
    
    // Inner hash
    let mut inner = Sha1::new();
    inner.update(&ipad);
    inner.update(data);
    let inner_hash = inner.finalize();
    
    // Outer hash
    let mut outer = Sha1::new();
    outer.update(&opad);
    outer.update(&inner_hash);
    outer.finalize().to_vec()
}

/// Generate HOTP code
fn generate_hotp(secret: &[u8], counter: u64, digits: u32) -> Result<String> {
    let counter_bytes = counter.to_be_bytes();
    let hash = hmac_sha1(secret, &counter_bytes);
    
    let offset = (hash[19] & 0x0f) as usize;
    let truncated = u32::from_be_bytes([
        hash[offset] & 0x7f,
        hash[offset + 1],
        hash[offset + 2],
        hash[offset + 3],
    ]);
    
    let code = truncated % 10u32.pow(digits);
    Ok(format!("{:0width$}", code, width = digits as usize))
}

/// Constant-time equality comparison
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    
    result == 0
}

// Base32 encoding module (simplified)
mod base32 {
    pub struct Alphabet {
        pub padding: bool,
    }
    
    pub const RFC4648: Alphabet = Alphabet { padding: true };
    
    pub fn encode(_alphabet: Alphabet, data: &[u8]) -> String {
        // Simplified base32 encoding
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
        let mut result = String::new();
        
        for chunk in data.chunks(5) {
            let mut buffer = [0u8; 5];
            buffer[..chunk.len()].copy_from_slice(chunk);
            
            let b1 = buffer[0] >> 3;
            let b2 = ((buffer[0] & 0x07) << 2) | (buffer[1] >> 6);
            let b3 = (buffer[1] & 0x3e) >> 1;
            let b4 = ((buffer[1] & 0x01) << 4) | (buffer[2] >> 4);
            let b5 = ((buffer[2] & 0x0f) << 1) | (buffer[3] >> 7);
            let b6 = (buffer[3] & 0x7c) >> 2;
            let b7 = ((buffer[3] & 0x03) << 3) | (buffer[4] >> 5);
            let b8 = buffer[4] & 0x1f;
            
            result.push(CHARS[b1 as usize] as char);
            result.push(CHARS[b2 as usize] as char);
            if chunk.len() > 1 { result.push(CHARS[b3 as usize] as char); }
            if chunk.len() > 1 { result.push(CHARS[b4 as usize] as char); }
            if chunk.len() > 2 { result.push(CHARS[b5 as usize] as char); }
            if chunk.len() > 3 { result.push(CHARS[b6 as usize] as char); }
            if chunk.len() > 3 { result.push(CHARS[b7 as usize] as char); }
            if chunk.len() > 4 { result.push(CHARS[b8 as usize] as char); }
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_totp_generation() {
        let provider = TotpProvider::default();
        let secret = provider.generate_secret().unwrap();
        
        // Generate code
        let time = 1234567890;
        let code = provider.generate_code(&secret.secret, time).unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_numeric()));
    }
    
    #[test]
    fn test_totp_verification() {
        let provider = TotpProvider::default();
        let secret = provider.generate_secret().unwrap();
        
        // Generate and verify code
        let now = chrono::Utc::now().timestamp() as u64;
        let code = provider.generate_code(&secret.secret, now).unwrap();
        assert!(provider.verify_code(&secret, &code).unwrap());
        
        // Wrong code should fail
        assert!(!provider.verify_code(&secret, "000000").unwrap());
    }
    
    #[test]
    fn test_provisioning_uri() {
        let provider = TotpProvider::default();
        let secret = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]; // "Hello"
        let uri = provider.provisioning_uri(&secret, "user@example.com");
        
        assert!(uri.starts_with("otpauth://totp/"));
        assert!(uri.contains("secret="));
        assert!(uri.contains("issuer=Synapsed"));
        assert!(uri.contains("digits=6"));
        assert!(uri.contains("period=30"));
    }
}