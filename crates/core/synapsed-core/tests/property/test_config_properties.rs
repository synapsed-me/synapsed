//! Property-based tests for configuration handling

use proptest::prelude::*;
use crate::utils::generators::*;

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn test_config_value_roundtrip_serialization(config in arb_config_value()) {
            // Test that ConfigValue can be serialized and deserialized
            let json = serde_json::to_string(&config).unwrap();
            let deserialized: synapsed_core::config::ConfigValue = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(config, deserialized);
        }

        #[test]
        fn test_config_value_clone_equality(config in arb_config_value()) {
            let cloned = config.clone();
            prop_assert_eq!(config, cloned);
        }
    }
}