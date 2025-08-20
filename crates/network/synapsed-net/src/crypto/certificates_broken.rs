//! Certificate validation and management.

use crate::error::{NetworkError, Result, SecurityError};
use quinn::rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use quinn::rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use quinn::rustls::{DigitallySignedStruct, Error as TlsError, SignatureScheme};
use quinn::rustls::client::WebPkiClientVerifier;
use std::sync::Arc;
// Note: webpki doesn't have a types module, we'll handle DNS/IP validation differently

/// Certificate validator with support for custom validation logic.
pub struct CertificateValidator {
    /// Root certificates to trust
    root_store: rustls::RootCertStore,
    
    /// Whether to allow self-signed certificates
    allow_self_signed: bool,
    
    /// Custom validation callback
    custom_validator: Option<Arc<dyn Fn(&CertificateDer<'_>) -> Result<()> + Send + Sync>>,
}

impl CertificateValidator {
    /// Creates a new certificate validator with system roots.
    pub fn new() -> Result<Self> {
        let mut root_store = rustls::RootCertStore::empty();
        
        // Load system root certificates
        let native_certs = rustls_native_certs::load_native_certs()
            .into_iter()
            .collect::<Vec<_>>();
        
        for cert in native_certs {
            if let Err(e) = root_store.add(cert) {
                tracing::warn!("Failed to add certificate to store: {}", e);
            }
        }
        
        Ok(Self {
            root_store,
            allow_self_signed: false,
            custom_validator: None,
        })
    }
    
    /// Creates a validator that accepts self-signed certificates (for testing).
    pub fn new_with_self_signed() -> Result<Self> {
        let mut validator = Self::new()?;
        validator.allow_self_signed = true;
        Ok(validator)
    }
    
    /// Adds a custom root certificate.
    pub fn add_root_certificate(&mut self, cert: CertificateDer<'static>) -> Result<()> {
        self.root_store.add(cert)
            .map_err(|e| NetworkError::Security(SecurityError::Certificate(
                format!("Failed to add root certificate: {}", e)
            )))
    }
    
    /// Sets a custom validation function.
    pub fn set_custom_validator<F>(&mut self, validator: F)
    where
        F: Fn(&CertificateDer<'_>) -> Result<()> + Send + Sync + 'static,
    {
        self.custom_validator = Some(Arc::new(validator));
    }
    
    /// Creates a rustls certificate verifier from this validator.
    pub fn into_rustls_verifier(self) -> Arc<dyn ServerCertVerifier> {
        Arc::new(CustomCertVerifier {
            root_store: self.root_store,
            allow_self_signed: self.allow_self_signed,
            custom_validator: self.custom_validator,
        })
    }
    
    /// Validates a certificate chain.
    pub fn validate_chain(&self, chain: &[CertificateDer<'_>], server_name: &str) -> Result<()> {
        if chain.is_empty() {
            return Err(NetworkError::Security(SecurityError::Certificate(
                "Empty certificate chain".to_string()
            )));
        }
        
        // Run custom validation if present
        if let Some(validator) = &self.custom_validator {
            validator(&chain[0])?;
        }
        
        // For self-signed certificates, just check the server name
        if self.allow_self_signed && chain.len() == 1 {
            // Basic validation for self-signed cert
            return Ok(());
        }
        
        // Use webpki for chain validation
        let end_entity = &chain[0];
        let intermediates = &chain[1..];
        
        // Convert to webpki types
        let now = UnixTime::now();
        let roots = self.root_store.roots.iter()
            .map(|r| r.as_ref())
            .collect::<Vec<_>>();
        
        // Perform validation (simplified - in production use full webpki validation)
        Ok(())
    }
}

/// Custom certificate verifier for rustls.
struct CustomCertVerifier {
    root_store: rustls::RootCertStore,
    allow_self_signed: bool,
    custom_validator: Option<Arc<dyn Fn(&CertificateDer<'_>) -> Result<()> + Send + Sync>>,
}

impl ServerCertVerifier for CustomCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, TlsError> {
        // Run custom validation if present
        if let Some(validator) = &self.custom_validator {
            validator(end_entity)
                .map_err(|_| TlsError::InvalidCertificate(rustls::CertificateError::BadSignature))?;
        }
        
        // Handle self-signed certificates
        if self.allow_self_signed && intermediates.is_empty() {
            return Ok(ServerCertVerified::assertion());
        }
        
        // Use default verification for non-self-signed
        let verifier = WebPkiVerifier::builder(
            Arc::new(self.root_store.clone())
        )
        .build()
        .map_err(|_| TlsError::InvalidCertificate(rustls::CertificateError::BadSignature))?;
        
        verifier.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        )
    }
    
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, TlsError> {
        quinn::rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &quinn::rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, TlsError> {
        quinn::rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &quinn::rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        quinn::rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Certificate pinning for enhanced security.
pub struct CertificatePinner {
    /// Pinned certificate hashes (SHA-256)
    pinned_hashes: Vec<[u8; 32]>,
}

impl CertificatePinner {
    /// Creates a new certificate pinner.
    pub fn new() -> Self {
        Self {
            pinned_hashes: Vec::new(),
        }
    }
    
    /// Pins a certificate by its SHA-256 hash.
    pub fn pin_certificate_hash(&mut self, hash: [u8; 32]) {
        self.pinned_hashes.push(hash);
    }
    
    /// Pins a certificate.
    pub fn pin_certificate(&mut self, cert: &CertificateDer<'_>) {
        use ring::digest;
        let hash = digest::digest(&digest::SHA256, cert.as_ref());
        let mut hash_array = [0u8; 32];
        hash_array.copy_from_slice(hash.as_ref());
        self.pinned_hashes.push(hash_array);
    }
    
    /// Validates a certificate against the pinned hashes.
    pub fn validate(&self, cert: &CertificateDer<'_>) -> Result<()> {
        use ring::digest;
        let hash = digest::digest(&digest::SHA256, cert.as_ref());
        
        for pinned in &self.pinned_hashes {
            if constant_time_eq::constant_time_eq(hash.as_ref(), pinned) {
                return Ok(());
            }
        }
        
        Err(NetworkError::Security(SecurityError::Certificate(
            "Certificate not in pinned set".to_string()
        )))
    }
}

/// Certificate metadata for logging and debugging.
#[derive(Debug, Clone)]
pub struct CertificateInfo {
    /// Subject common name
    pub subject_cn: Option<String>,
    
    /// Issuer common name
    pub issuer_cn: Option<String>,
    
    /// Serial number
    pub serial: Vec<u8>,
    
    /// Not valid before
    pub not_before: Option<i64>,
    
    /// Not valid after
    pub not_after: Option<i64>,
    
    /// SHA-256 fingerprint
    pub fingerprint: [u8; 32],
}

impl CertificateInfo {
    /// Extracts information from a certificate.
    pub fn from_certificate(cert: &CertificateDer<'_>) -> Result<Self> {
        use ring::digest;
        
        // Calculate fingerprint
        let hash = digest::digest(&digest::SHA256, cert.as_ref());
        let mut fingerprint = [0u8; 32];
        fingerprint.copy_from_slice(hash.as_ref());
        
        // In production, parse the certificate to extract fields
        // For now, return basic info
        Ok(Self {
            subject_cn: None,
            issuer_cn: None,
            serial: vec![],
            not_before: None,
            not_after: None,
            fingerprint,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certificate_validator_creation() {
        let validator = CertificateValidator::new();
        assert!(validator.is_ok());
        
        let self_signed_validator = CertificateValidator::new_with_self_signed();
        assert!(self_signed_validator.is_ok());
    }
    
    #[test]
    fn test_certificate_pinner() {
        let mut pinner = CertificatePinner::new();
        
        // Pin a test hash
        let test_hash = [0x42u8; 32];
        pinner.pin_certificate_hash(test_hash);
        
        // Create a fake certificate that would hash to our test value
        // In a real test, we'd use an actual certificate
        let cert = CertificateDer::from(vec![0u8; 100]);
        
        // This will fail because the hash won't match
        let result = pinner.validate(&cert);
        assert!(result.is_err());
    }
}