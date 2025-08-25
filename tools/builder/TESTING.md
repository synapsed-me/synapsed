# Synapsed Builder Testing Guide

Comprehensive testing approach for the Synapsed Builder system, including unit tests, integration tests, property-based tests, and performance benchmarks.

## Testing Philosophy

The builder system uses multiple testing strategies:
1. **Unit Tests** - Test individual components in isolation
2. **Integration Tests** - Test end-to-end workflows
3. **Property-Based Tests** - Verify invariants hold for all inputs
4. **Snapshot Tests** - Ensure generated code remains consistent
5. **Fuzzing** - Find edge cases and security issues
6. **Benchmarks** - Monitor performance characteristics

## Running Tests

### All Tests
```bash
# Run all tests
cargo test --all-features

# Run with output
cargo test --all-features -- --nocapture

# Run specific test
cargo test test_builder_produces_valid_apps
```

### Test Categories

```bash
# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test integration_tests

# Property tests
cargo test --test property_tests

# Benchmarks
cargo bench
```

## Unit Tests

Located in `src/tests/` with separate files per module:

### Registry Tests (`registry_tests.rs`)
Tests component registration and capability discovery:
```rust
#[test]
fn test_register_and_retrieve_component() {
    let mut registry = ComponentRegistry::new();
    let component = create_test_component("test-comp");
    
    registry.register(component.clone());
    let retrieved = registry.get("test-comp");
    
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "test-comp");
}
```

### Builder Tests (`builder_tests.rs`)
Tests the builder DSL and application construction:
```rust
#[test]
fn test_builder_with_all_features() {
    let result = SynapsedBuilder::new("full-app")
        .add_intent_verification()
        .add_consensus()
        .add_storage(StorageBackend::Postgres)
        .add_observability(ObservabilityLevel::Full)
        .build();
    
    assert!(result.is_ok());
    let app = result.unwrap();
    assert!(app.components.contains(&"synapsed-intent".to_string()));
}
```

### Recipe Tests (`recipe_tests.rs`)
Tests recipe loading and validation:
```rust
#[test]
fn test_parse_yaml_recipe() {
    let yaml = include_str!("../../recipes/verified-ai-agent.yaml");
    let recipe: Recipe = serde_yaml::from_str(yaml).unwrap();
    
    assert_eq!(recipe.name, "verified-ai-agent");
    assert!(!recipe.components.is_empty());
}
```

## Integration Tests

Located in `tests/integration_tests.rs`:

### Workflow Tests
Test complete workflows from template to deployment:
```rust
#[tokio::test]
async fn test_template_to_deployment_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test-app");
    
    // Build from template
    let app = Templates::verified_ai_agent()
        .configure("synapsed-storage", json!({"path": "./test.db"}))
        .build()
        .expect("Failed to build");
    
    // Save to directory
    app.save(&output_path).await.expect("Failed to save");
    
    // Verify files exist
    assert!(output_path.join("Cargo.toml").exists());
    assert!(output_path.join("src/main.rs").exists());
}
```

### Concurrent Build Tests
Ensure thread safety:
```rust
#[tokio::test]
async fn test_concurrent_builds() {
    let handles: Vec<_> = (0..10).map(|i| {
        tokio::spawn(async move {
            SynapsedBuilder::new(&format!("app-{}", i))
                .add_intent_verification()
                .build()
        })
    }).collect();
    
    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }
}
```

## Property-Based Tests

Located in `tests/property_tests.rs`:

### Invariant Testing
Verify system invariants hold for all inputs:
```rust
proptest! {
    #[test]
    fn registry_consistency(
        components in prop::collection::vec(component_strategy(), 0..50)
    ) {
        let mut registry = ComponentRegistry::new();
        
        for component in &components {
            registry.register(component.clone());
        }
        
        // All registered components can be retrieved
        for component in &components {
            prop_assert!(registry.get(&component.name).is_some());
        }
    }
}
```

### Composition Properties
```rust
proptest! {
    #[test]
    fn builder_produces_valid_apps(
        name in "[a-z][a-z0-9-]{0,30}",
        storage in storage_backend_strategy(),
        observability in observability_level_strategy()
    ) {
        let result = SynapsedBuilder::new(&name)
            .add_storage(storage)
            .add_observability(observability)
            .build();
        
        if let Ok(app) = result {
            // Core is always included
            prop_assert!(app.components.contains(&"synapsed-core".to_string()));
            // No empty components
            prop_assert!(!app.components.is_empty());
        }
    }
}
```

## Snapshot Testing

Test that generated code remains consistent:

```rust
#[test]
fn test_generated_cargo_toml_snapshot() {
    let app = create_test_app();
    let cargo_toml = app.generate_cargo_toml();
    
    // Compare with snapshot
    insta::assert_snapshot!(cargo_toml);
}

#[test]
fn test_generated_main_rs_snapshot() {
    let app = create_test_app();
    let main_rs = app.generate_main_rs();
    
    insta::assert_snapshot!(main_rs);
}
```

Update snapshots:
```bash
cargo insta review
```

## Fuzzing Tests

Using cargo-fuzz for security testing:

```rust
// fuzz/fuzz_targets/builder_fuzz.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = SynapsedBuilder::new(s)
            .add_intent_verification()
            .build();
    }
});
```

Run fuzzing:
```bash
cargo +nightly fuzz run builder_fuzz
```

## Performance Benchmarks

Located in `benches/`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_builder(c: &mut Criterion) {
    c.bench_function("build simple app", |b| {
        b.iter(|| {
            SynapsedBuilder::new("test")
                .add_intent_verification()
                .build()
        })
    });
    
    c.bench_function("build complex app", |b| {
        b.iter(|| {
            SynapsedBuilder::new("test")
                .add_intent_verification()
                .add_consensus()
                .add_storage(StorageBackend::Postgres)
                .add_observability(ObservabilityLevel::Full)
                .add_payments()
                .build()
        })
    });
}

criterion_group!(benches, benchmark_builder);
criterion_main!(benches);
```

Run benchmarks:
```bash
cargo bench

# Save baseline
cargo bench -- --save-baseline main

# Compare with baseline
cargo bench -- --baseline main
```

## Test Fixtures

Common test utilities in `tests/common/`:

```rust
// tests/common/mod.rs
pub fn create_test_component(name: &str) -> Component {
    Component {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![Capability::Core],
        description: "Test component".to_string(),
        dependencies: vec![],
    }
}

pub fn create_test_app() -> Application {
    SynapsedBuilder::new("test-app")
        .add_intent_verification()
        .build()
        .unwrap()
}

pub fn create_test_recipe() -> Recipe {
    Recipe {
        name: "test-recipe".to_string(),
        description: "Test recipe".to_string(),
        version: "1.0.0".to_string(),
        components: vec!["synapsed-core".to_string()],
        connections: vec![],
        config: HashMap::new(),
    }
}
```

## Mocking

Using mockall for component mocking:

```rust
#[cfg(test)]
mod tests {
    use mockall::*;
    
    #[automock]
    trait ComponentRegistry {
        fn get(&self, name: &str) -> Option<Component>;
        fn register(&mut self, component: Component);
    }
    
    #[test]
    fn test_with_mock_registry() {
        let mut mock = MockComponentRegistry::new();
        mock.expect_get()
            .with(eq("test"))
            .returning(|_| Some(create_test_component("test")));
        
        assert!(mock.get("test").is_some());
    }
}
```

## Test Coverage

Generate coverage reports:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --out Html --output-dir coverage

# With specific features
cargo tarpaulin --features full --out Lcov
```

## CI/CD Integration

GitHub Actions workflow (`.github/workflows/test.yml`):

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Run tests
        run: cargo test --all-features
        
      - name: Run property tests
        run: cargo test --test property_tests
        
      - name: Check formatting
        run: cargo fmt -- --check
        
      - name: Run clippy
        run: cargo clippy -- -D warnings
        
      - name: Generate coverage
        run: cargo tarpaulin --out Lcov
        
      - name: Upload coverage
        uses: codecov/codecov-action@v2
```

## Test Organization

```
tests/
├── common/
│   ├── mod.rs           # Shared test utilities
│   ├── fixtures.rs      # Test fixtures
│   └── helpers.rs       # Helper functions
├── integration_tests.rs # End-to-end tests
├── property_tests.rs    # Property-based tests
└── snapshot_tests.rs    # Snapshot tests

src/tests/
├── mod.rs              # Test module declaration
├── registry_tests.rs   # Registry unit tests
├── recipe_tests.rs     # Recipe unit tests
├── builder_tests.rs    # Builder unit tests
├── composer_tests.rs   # Composer unit tests
├── validator_tests.rs  # Validator unit tests
└── template_tests.rs   # Template unit tests

benches/
├── builder_bench.rs    # Builder benchmarks
├── registry_bench.rs   # Registry benchmarks
└── recipe_bench.rs     # Recipe benchmarks

fuzz/
└── fuzz_targets/
    ├── builder_fuzz.rs  # Builder fuzzing
    └── recipe_fuzz.rs   # Recipe fuzzing
```

## Writing Good Tests

### Test Naming
Use descriptive names that explain what is being tested:
```rust
#[test]
fn test_builder_adds_core_component_automatically() { }

#[test]
fn test_registry_finds_components_by_capability() { }

#[test]
fn test_recipe_validation_catches_missing_components() { }
```

### Test Independence
Each test should be independent:
```rust
#[test]
fn test_independent_1() {
    let registry = ComponentRegistry::new(); // Fresh instance
    // Test logic
}

#[test]
fn test_independent_2() {
    let registry = ComponentRegistry::new(); // Fresh instance
    // Test logic
}
```

### Assertion Messages
Provide helpful assertion messages:
```rust
assert!(
    app.components.contains(&"synapsed-core".to_string()),
    "Core component should always be included, but found: {:?}",
    app.components
);
```

### Test Data Builders
Use builders for complex test data:
```rust
struct TestAppBuilder {
    name: String,
    components: Vec<String>,
}

impl TestAppBuilder {
    fn new() -> Self {
        Self {
            name: "test-app".to_string(),
            components: vec![],
        }
    }
    
    fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }
    
    fn with_component(mut self, component: &str) -> Self {
        self.components.push(component.to_string());
        self
    }
    
    fn build(self) -> Application {
        // Build application
    }
}
```

## Debugging Tests

### Enable logging in tests
```rust
#[test]
fn test_with_logging() {
    env_logger::init();
    // Test logic with log output
}
```

### Use debug assertions
```rust
#[test]
fn test_with_debug_info() {
    let app = build_test_app();
    dbg!(&app.components);
    assert!(!app.components.is_empty());
}
```

### Run single test with output
```bash
cargo test test_name -- --nocapture
```

## Test Maintenance

1. **Keep tests fast** - Mock external dependencies
2. **Update tests with code** - Tests should evolve with implementation
3. **Remove redundant tests** - Don't test the same thing multiple ways
4. **Document complex tests** - Explain non-obvious test logic
5. **Review test failures** - Don't ignore intermittent failures
6. **Maintain test coverage** - Aim for >80% coverage
7. **Test error paths** - Don't just test happy paths