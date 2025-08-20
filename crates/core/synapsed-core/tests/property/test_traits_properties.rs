//! Property-based tests for traits

use proptest::prelude::*;
use crate::utils::generators::*;

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn test_observable_state_serialization(state in arb_observable_state()) {
            let json = serde_json::to_string(&state).unwrap();
            let deserialized = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(state, deserialized);
        }

        #[test]
        fn test_health_level_consistency(level in arb_health_level()) {
            let cloned = level.clone();
            prop_assert_eq!(level.clone(), cloned);
            
            let debug_str = format!("{:?}", level);
            prop_assert!(!debug_str.is_empty());
        }
    }
}