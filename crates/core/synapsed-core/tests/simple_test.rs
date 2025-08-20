//! Simple test to verify the framework compiles

use synapsed_core::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Test basic error creation
        let error = SynapsedError::config("test error");
        assert!(error.to_string().contains("test error"));

        // Test result type
        let result: SynapsedResult<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);

        // Test version info
        assert!(!VERSION.is_empty());
        assert_eq!(NAME, "synapsed-core");
    }

    #[test]
    fn test_error_classification() {
        let network_error = SynapsedError::network("connection failed");
        assert!(network_error.is_retryable());

        let validation_error = SynapsedError::invalid_input("bad input");
        assert!(validation_error.is_client_error());
        assert!(!validation_error.is_retryable());
    }

    #[test]
    fn test_error_creation_methods() {
        let config_err = SynapsedError::config("test config error");
        assert!(config_err.to_string().contains("Configuration error"));
        assert!(config_err.to_string().contains("test config error"));

        let network_err = SynapsedError::network("connection failed");
        assert!(network_err.to_string().contains("Network error"));
        assert!(network_err.is_retryable());

        let app_err = SynapsedError::application("process failed", "user_id=123");
        assert!(app_err.to_string().contains("process failed"));
        assert!(app_err.to_string().contains("user_id=123"));
    }

    #[test]
    fn test_error_conversions() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let synapsed_err: SynapsedError = io_err.into();
        assert!(matches!(synapsed_err, SynapsedError::Internal(_)));

        let json_err: serde_json::Error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let synapsed_err: SynapsedError = json_err.into();
        assert!(matches!(synapsed_err, SynapsedError::Serialization(_)));
    }

    #[test]
    fn test_network_functionality() {
        use synapsed_core::network::{NetworkMessage, NetworkStats};
        
        // Test network message creation
        let payload = b"test payload".to_vec();
        let message = NetworkMessage::new("test.message", payload.clone())
            .with_header("content-type", "application/octet-stream")
            .with_header("version", "1.0");
        
        assert_eq!(message.message_type, "test.message");
        assert_eq!(message.payload, payload);
        assert_eq!(message.payload_size(), payload.len());
        assert_eq!(message.get_header("version"), Some("1.0"));
        
        // Test network stats
        let mut stats = NetworkStats::new();
        stats.record_bytes_sent(1000);
        stats.record_bytes_received(2000);
        stats.record_message_sent();
        stats.update_uptime(60);
        
        assert_eq!(stats.bytes_sent, 1000);
        assert_eq!(stats.bytes_received, 2000);
        assert_eq!(stats.messages_sent, 1);
        assert_eq!(stats.throughput(), 50.0); // (1000 + 2000) / 60
    }

    #[cfg(feature = "config")]
    #[test]
    fn test_config_features() {
        use synapsed_core::config::{ConfigValue, EnvConfigSource, ConfigSource};
        
        let string_val = ConfigValue::String("test".to_string());
        assert_eq!(string_val.as_string().unwrap(), "test");
        
        let env_source = EnvConfigSource::new("TEST");
        assert_eq!(env_source.source_name(), "environment");
    }

    #[test]
    fn test_network_addresses() {
        use synapsed_core::network::NetworkAddress;
        use std::str::FromStr;
        
        let addr = NetworkAddress::from_str("127.0.0.1:8080").unwrap();
        assert_eq!(addr.protocol(), "tcp");
        
        let peer_addr = NetworkAddress::PeerId("peer123".to_string());
        assert_eq!(peer_addr.protocol(), "p2p");
    }

    #[test]
    fn test_trait_functionality() {
        use synapsed_core::traits::{Validatable, HealthLevel};
        
        struct TestValidator(bool);
        impl Validatable for TestValidator {
            fn validate(&self) -> SynapsedResult<()> {
                if self.0 { Ok(()) } else { Err(SynapsedError::invalid_input("test")) }
            }
        }
        
        let valid = TestValidator(true);
        assert!(valid.is_valid());
        
        let invalid = TestValidator(false);
        assert!(!invalid.is_valid());
        
        // Test enum serialization
        let level = HealthLevel::Healthy;
        let json = serde_json::to_string(&level).unwrap();
        let deserialized: HealthLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(level, deserialized);
    }
}