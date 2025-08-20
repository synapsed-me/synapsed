# Synapsed WASM

A comprehensive WebAssembly runtime and module system for the Synapsed framework, providing WASM-based execution environments for smart contracts, payment processing, cryptographic operations, and blockchain integration.

## Features

### 🚀 Core Runtime
- **High-Performance WASM Execution**: Built on Wasmtime with optimized compilation
- **Multiple Compilation Targets**: Native, Web, WASI, and Substrate support
- **Async/Await Support**: Fully asynchronous execution with tokio integration
- **Resource Management**: Memory management, execution timeouts, and resource limits
- **Security**: Sandboxing, bytecode validation, and security policies

### 🔧 Module System
- **Dynamic Module Loading**: Load modules from bytecode, files, or WAT format
- **Module Registry**: Centralized module management and lifecycle
- **Host Functions**: Extensible host function system with custom imports
- **State Persistence**: Serialize/deserialize module state for persistence
- **Performance Monitoring**: Detailed execution statistics and metrics

### 🛡️ Security Features
- **Sandboxed Execution**: Isolated module execution environments
- **Bytecode Validation**: Comprehensive security validation
- **Resource Limits**: Memory, execution time, and stack size limits
- **Import Restrictions**: Control over allowed imports and host functions
- **Deterministic Execution**: Support for blockchain/smart contract requirements

### 🔗 Integration
- **Synapsed Crypto**: Post-quantum cryptographic operations in WASM
- **Synapsed Storage**: WASM-compatible storage operations
- **Synapsed Payments**: Payment processing modules
- **Synapsed Network**: WASM-based networking capabilities
- **Substrate Integration**: Smart contract execution for Substrate chains

## Quick Start

### Basic Usage

```rust
use synapsed_wasm::prelude::*;

#[tokio::main]
async fn main() -> WasmResult<()> {
    // Initialize WASM runtime
    let runtime = WasmRuntime::new().await?;

    // Load and execute a WASM module
    let wat_source = r#"
        (module
            (func $add (param $a i32) (param $b i32) (result i32)
                local.get $a
                local.get $b
                i32.add
            )
            (export "add" (func $add))
        )
    "#;

    let wasm_bytes = wat::parse_str(wat_source)?;
    let metadata = ModuleMetadata::new("1.0.0".to_string());

    // Load module
    let module_id = runtime.load_module(
        "math_module".to_string(),
        &wasm_bytes,
        metadata,
    ).await?;

    // Execute function
    let args = vec![WasmValue::I32(10), WasmValue::I32(20)];
    let context = ExecutionContext::new();
    let result = runtime.execute_function(&module_id, "add", &args, context).await?;

    println!("Result: {:?}", result);
    Ok(())
}
```

### Host Functions

```rust
use synapsed_wasm::prelude::*;

#[tokio::main]
async fn main() -> WasmResult<()> {
    let runtime = WasmRuntime::new().await?;

    // Register custom host function
    runtime.register_host_function(
        "log_message".to_string(),
        |args| {
            if let Some(WasmValue::String(msg)) = args.first() {
                println!("WASM says: {}", msg);
            }
            Ok(vec![])
        }
    ).await?;

    // Now WASM modules can import and call "log_message"
    Ok(())
}
```

### Configuration

```rust
use synapsed_wasm::prelude::*;

#[tokio::main]
async fn main() -> WasmResult<()> {
    // Different configurations for different use cases
    let dev_config = RuntimeConfig::development();     // Development
    let prod_config = RuntimeConfig::production();     // Production
    let blockchain_config = RuntimeConfig::blockchain(); // Smart contracts
    let web_config = RuntimeConfig::web();             // Web deployment

    let runtime = WasmRuntime::with_config(prod_config).await?;
    Ok(())
}
```

## Architecture

### Runtime Components

```
┌─────────────────────────────────────────────────────────────┐
│                    WASM Runtime                             │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Engine    │  │   Memory    │  │     Security        │  │
│  │ Management  │  │ Management  │  │    Manager          │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Module    │  │    Host     │  │    Execution        │  │
│  │  Registry   │  │  Functions  │  │     Context         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                 Wasmtime Engine                             │
└─────────────────────────────────────────────────────────────┘
```

### Module Lifecycle

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│    Load     │───▶│   Compile   │───▶│ Instantiate │
│   Module    │    │   & Validate│    │  & Link     │
└─────────────┘    └─────────────┘    └─────────────┘
       │                                      │
       ▼                                      ▼
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Persist   │◀───│   Execute   │───▶│   Monitor   │
│    State    │    │  Functions  │    │ Performance │
└─────────────┘    └─────────────┘    └─────────────┘
```

## Configuration

### Runtime Configuration

The runtime supports various configuration profiles:

- **Development**: Debug info enabled, longer timeouts, less strict security
- **Production**: Optimizations enabled, strict security, performance monitoring
- **Blockchain**: Deterministic execution, gas metering, sandboxing enforced
- **Web**: Browser compatibility, restricted I/O, size optimizations

### Security Configuration

```rust
let config = RuntimeConfig {
    security: SecurityConfig {
        enable_sandboxing: true,
        strict_validation: true,
        enable_deterministic_execution: true,
        disable_unsafe_host_functions: true,
        max_imports: 100,
        max_exports: 100,
    },
    // ... other config
};
```

### Memory Configuration

```rust
let config = RuntimeConfig {
    memory: MemoryConfig {
        memory_pool_size: 256 * 1024 * 1024,  // 256MB
        enable_memory_sharing: true,
        enable_gc: true,
        gc_threshold: 64 * 1024 * 1024,       // 64MB
        enable_memory_protection: true,
    },
    // ... other config
};
```

## Features

### Compilation Targets

Enable different compilation targets with feature flags:

```toml
[dependencies]
synapsed-wasm = { version = "0.1", features = ["web"] }          # Web target
synapsed-wasm = { version = "0.1", features = ["substrate-modules"] } # Substrate
synapsed-wasm = { version = "0.1", features = ["full"] }         # All features
```

### Available Features

- `std` - Standard library support (default)
- `runtime` - Core runtime functionality (default)
- `web` - Web/browser support with wasm-bindgen
- `crypto-modules` - Cryptographic WASM modules
- `storage-modules` - Storage operation modules
- `payment-modules` - Payment processing modules
- `network-modules` - Networking modules
- `substrate-modules` - Substrate integration
- `full` - All features enabled

## Examples

### Smart Contract Execution

```rust
use synapsed_wasm::prelude::*;

#[tokio::main]
async fn main() -> WasmResult<()> {
    // Use blockchain configuration for smart contracts
    let config = RuntimeConfig::blockchain();
    let runtime = WasmRuntime::with_config(config).await?;

    // Load smart contract
    let contract_bytes = std::fs::read("contract.wasm")?;
    let metadata = ModuleMetadata::new("1.0.0".to_string())
        .with_capability("smart-contract")
        .with_tag("defi");

    let contract_id = runtime.load_module(
        "defi_contract".to_string(),
        &contract_bytes,
        metadata,
    ).await?;

    // Execute contract function with gas limit
    let context = ExecutionContext::new()
        .with_gas_limit(1_000_000)
        .with_timeout(std::time::Duration::from_secs(10));

    let result = runtime.execute_function(
        &contract_id,
        "transfer",
        &[WasmValue::String("alice".to_string()), WasmValue::I64(100)],
        context,
    ).await?;

    println!("Transfer result: {:?}", result);
    Ok(())
}
```

### Cryptographic Operations

```rust
#[cfg(feature = "crypto-modules")]
use synapsed_wasm::crypto::*;

#[tokio::main]
async fn main() -> WasmResult<()> {
    let runtime = WasmRuntime::new().await?;

    // Register crypto host functions
    let crypto_functions = synapsed_wasm::crypto::create_crypto_host_functions();
    for (name, func) in crypto_functions {
        runtime.register_host_function(name, func).await?;
    }

    // Load crypto WASM module
    let crypto_module = load_crypto_module("kyber_encrypt").await?;
    
    // Execute encryption
    let plaintext = b"Hello, quantum-safe world!";
    let result = crypto_module.encrypt(plaintext).await?;
    
    println!("Encrypted: {:?}", result);
    Ok(())
}
```

## Performance

### Benchmarks

The runtime includes comprehensive benchmarks:

```bash
cargo bench --features benchmarks
```

### Monitoring

Runtime and module statistics are available:

```rust
let stats = runtime.get_stats().await;
println!("Modules loaded: {}", stats.modules_loaded);
println!("Functions executed: {}", stats.functions_executed);
println!("Average execution time: {:?}", stats.average_execution_time());
```

## Testing

Run the test suite:

```bash
# All tests
cargo test

# Integration tests only
cargo test --test integration_tests

# With all features
cargo test --all-features
```

## Safety and Security

### Memory Safety
- All WASM execution is memory-safe by design
- Optional memory protection and bounds checking
- Configurable memory limits and garbage collection

### Execution Safety
- Sandboxed execution environments
- Configurable execution timeouts
- Resource limit enforcement
- Bytecode validation and security scanning

### Host Function Security
- Controlled import/export mechanisms
- Validation of host function signatures
- Optional restriction of unsafe operations

## Integration with Synapsed Ecosystem

### Crypto Integration
```rust
#[cfg(feature = "crypto-modules")]
use synapsed_crypto::Kyber;

// WASM modules can use post-quantum crypto
let kyber = Kyber::new();
let (public_key, secret_key) = kyber.generate_keypair();
```

### Storage Integration
```rust
#[cfg(feature = "storage-modules")]
use synapsed_storage::Storage;

// WASM modules can persist data
let storage = Storage::new();
storage.set("key", "value").await?;
```

### Payment Integration
```rust
#[cfg(feature = "payment-modules")]
use synapsed_payments::PaymentProcessor;

// WASM modules can process payments
let processor = PaymentProcessor::new();
processor.process_payment(payment_request).await?;
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

This project is licensed under the MIT OR Apache-2.0 license.

## Changelog

### v0.1.0
- Initial release
- Core WASM runtime functionality
- Module management system
- Security and sandboxing
- Integration with Synapsed ecosystem
- Comprehensive test suite and examples