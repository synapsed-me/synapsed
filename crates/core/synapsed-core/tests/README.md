# Synapsed Core Test Framework

This directory contains comprehensive tests for the `synapsed-core` crate, following TDD principles and best practices for Rust testing.

## Test Structure

### Unit Tests (`unit/`)
- **Purpose**: Test individual modules and functions in isolation
- **Scope**: Single function or small group of related functions
- **Mocking**: Uses `mockall` for trait implementations
- **Coverage**: High coverage of edge cases and error conditions

### Integration Tests (`integration/`)
- **Purpose**: Test how different modules work together
- **Scope**: Cross-module interactions and complete workflows
- **Environment**: Real implementations with test configurations
- **Scenarios**: End-to-end scenarios and complex interactions

### Property-Based Tests (`property/`)
- **Purpose**: Test properties that should always hold true
- **Framework**: Uses `proptest` for generating test inputs
- **Focus**: Invariants, round-trip properties, and mathematical properties
- **Coverage**: Large input space exploration

### Benchmark Tests (`benchmarks/`)
- **Purpose**: Performance testing and regression detection
- **Framework**: Uses `criterion` for statistical benchmarking
- **Metrics**: Throughput, latency, memory usage
- **Baseline**: Performance regression detection

## Test Categories

### Core Functionality Tests
- **Error handling**: All error variants and conversions
- **Configuration**: Loading, validation, merging from multiple sources
- **Traits**: All trait implementations with mock objects
- **Network abstractions**: Address parsing, connection management
- **Observability**: Health checks, metrics, status reporting

### Edge Case Testing
- **Boundary conditions**: Empty inputs, maximum values, null cases
- **Error scenarios**: Network failures, invalid configs, timeouts
- **Concurrent operations**: Thread safety, race conditions
- **Resource constraints**: Memory limits, connection limits

### Security Testing
- **Input validation**: Malicious inputs, injection attempts
- **Authentication**: Invalid credentials, expired tokens
- **Authorization**: Permission boundaries, privilege escalation
- **Cryptographic**: Key validation, signature verification

## Test Utilities

### Mock Implementations (`utils/mocks.rs`)
- **Observable**: Mock observable components
- **Configurable**: Mock configurable services
- **NetworkConnection**: Mock network connections
- **NetworkListener**: Mock network listeners

### Test Fixtures (`fixtures/`)
- **Config files**: TOML, JSON test configurations
- **Network data**: Sample messages, addresses
- **Error scenarios**: Pre-defined error conditions
- **Performance data**: Baseline performance metrics

### Helper Functions (`utils/helpers.rs`)
- **Setup/teardown**: Test environment management
- **Assertions**: Custom assertions for complex types
- **Generators**: Test data generation utilities
- **Matchers**: Custom matching functions

## Running Tests

### All Tests
```bash
cargo test
```

### Unit Tests Only
```bash
cargo test --test unit
```

### Integration Tests Only
```bash
cargo test --test integration
```

### Property-Based Tests
```bash
cargo test --test property
```

### Benchmarks
```bash
cargo bench
```

### With Coverage
```bash
cargo tarpaulin --out Html
```

## Test Guidelines

### Writing Unit Tests
1. **Single responsibility**: One test per behavior
2. **Descriptive names**: `test_should_return_error_when_invalid_input`
3. **AAA pattern**: Arrange, Act, Assert
4. **Mock dependencies**: Use `mockall` for external dependencies
5. **Test errors**: Always test error conditions

### Writing Integration Tests
1. **Real components**: Use actual implementations where possible
2. **Test configurations**: Use minimal test-specific configs
3. **Cleanup**: Ensure proper cleanup after tests
4. **Independent**: Tests should not depend on each other
5. **Realistic scenarios**: Test real-world usage patterns

### Writing Property Tests
1. **Clear properties**: Define what should always be true
2. **Good generators**: Create meaningful input generators
3. **Shrinking**: Ensure failures shrink to minimal cases
4. **Performance**: Keep property tests reasonably fast
5. **Edge cases**: Include edge cases in generators

### Performance Testing
1. **Baseline establishment**: Establish performance baselines
2. **Regression detection**: Detect performance regressions
3. **Statistical significance**: Use proper statistical methods
4. **Resource monitoring**: Monitor memory and CPU usage
5. **Real-world data**: Use realistic data sizes and patterns

## Test Configuration

### Environment Variables
- `SYNAPSED_TEST_LOG_LEVEL`: Set logging level for tests
- `SYNAPSED_TEST_PARALLEL`: Control parallel test execution
- `SYNAPSED_TEST_TIMEOUT`: Set test timeout values

### Feature Flags
- `testing`: Enables additional test utilities
- `test-utils`: Includes test helper functions
- `mock-implementations`: Provides mock trait implementations

## CI/CD Integration

### GitHub Actions
- **Unit tests**: Run on every PR and push
- **Integration tests**: Run on release branches
- **Property tests**: Run nightly with extended time
- **Benchmarks**: Run on performance-critical changes
- **Coverage**: Generate and upload coverage reports

### Quality Gates
- **Minimum coverage**: 80% line coverage required
- **All tests pass**: No failing tests allowed
- **No clippy warnings**: Clean code quality
- **Documentation tests**: All doc examples work
- **Benchmark regression**: Performance within acceptable bounds

## Test Data Management

### Fixtures
- **Version control**: Small test data in version control
- **Generation**: Large test data generated at runtime
- **Cleanup**: Automatic cleanup of temporary test data
- **Isolation**: Each test gets fresh data

### Mocking Strategy
- **External services**: Always mock external dependencies
- **Database**: Use in-memory databases for speed
- **Network**: Mock network calls for reliability
- **File system**: Use temporary directories
- **Time**: Mock time for deterministic tests

## Debugging Tests

### Logging
- **Test output**: Use `env_logger` for test debugging
- **Assertion details**: Detailed assertion failure messages
- **State dumps**: Dump relevant state on failures
- **Trace information**: Include execution traces

### Tools
- **IDE integration**: Use IDE test runners
- **CLI debugging**: `cargo test -- --nocapture`
- **Profiling**: Use profiling tools for performance tests
- **Memory debugging**: Use memory leak detection tools

## Contributing

### Test Reviews
- **Comprehensive coverage**: Ensure all paths tested
- **Maintainability**: Tests should be easy to understand
- **Performance**: Tests should run quickly
- **Reliability**: Tests should be deterministic
- **Documentation**: Complex tests should be documented

### Adding New Tests
1. **Identify test type**: Unit, integration, or property test
2. **Create test file**: Follow naming conventions
3. **Write tests**: Follow guidelines above
4. **Update documentation**: Update this README if needed
5. **Run locally**: Ensure all tests pass locally

This test framework ensures high quality, reliability, and maintainability of the synapsed-core crate through comprehensive testing strategies.