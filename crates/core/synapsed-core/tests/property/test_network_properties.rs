//! Property-based tests for network functionality

use proptest::prelude::*;
use crate::utils::generators::*;

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn test_network_address_display_format(addr in arb_network_address()) {
            let display_str = addr.to_string();
            prop_assert!(display_str.contains("://"));
            prop_assert!(!display_str.is_empty());
            prop_assert!(display_str.len() > 5);
        }

        #[test]
        fn test_network_message_consistency(msg in arb_network_message()) {
            prop_assert!(!msg.id.is_nil());
            prop_assert!(!msg.message_type.is_empty());
            prop_assert_eq!(msg.payload_size(), msg.payload.len());
        }
    }
}