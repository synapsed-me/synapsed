//! Test suite for synapsed-builder
//! 
//! Intent: Create comprehensive unit tests for all builder components
//! Goal: Ensure each component works correctly in isolation
//! Success Criteria:
//!   - All public APIs have tests
//!   - Code coverage > 80%
//!   - All edge cases covered

#[cfg(test)]
mod registry_tests;

#[cfg(test)]
mod recipe_tests;

#[cfg(test)]
mod builder_tests;

#[cfg(test)]
mod composer_tests;

#[cfg(test)]
mod validator_tests;

#[cfg(test)]
mod template_tests;