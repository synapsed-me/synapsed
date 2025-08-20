//! Integration tests for synapsed-wasm

use synapsed_wasm::prelude::*;
use std::time::Duration;

#[tokio::test]
async fn test_basic_runtime_functionality() {
    let runtime = WasmRuntime::new().await.unwrap();
    
    // Test runtime initialization
    assert_eq!(runtime.config().compilation.target, CompilationTarget::Native);
    
    // Test module list (should be empty initially)
    let modules = runtime.list_modules().await.unwrap();
    assert!(modules.is_empty());
}

#[tokio::test]
async fn test_module_loading_and_execution() {
    let runtime = WasmRuntime::new().await.unwrap();
    
    // Simple WAT module that adds two numbers
    let wat = r#"
        (module
            (func $add (param $a i32) (param $b i32) (result i32)
                local.get $a
                local.get $b
                i32.add
            )
            (export "add" (func $add))
        )
    "#;
    
    let wasm_bytes = wat::parse_str(wat).unwrap();
    let metadata = ModuleMetadata::new("1.0.0".to_string());
    
    // Load module
    let module_id = runtime.load_module(
        "test_module".to_string(),
        &wasm_bytes,
        metadata,
    ).await.unwrap();
    
    // Verify module is loaded
    assert!(runtime.has_module(&module_id).await);
    let modules = runtime.list_modules().await.unwrap();
    assert_eq!(modules.len(), 1);
    
    // Execute function
    let args = vec![WasmValue::I32(10), WasmValue::I32(20)];
    let context = ExecutionContext::new();
    let result = runtime.execute_function(&module_id, "add", &args, context).await.unwrap();
    
    assert_eq!(result.len(), 1);
    if let WasmValue::I32(sum) = &result[0] {
        assert_eq!(*sum, 30);
    } else {
        panic!("Expected I32 result");
    }
    
    // Unload module
    runtime.unload_module(&module_id).await.unwrap();
    assert!(!runtime.has_module(&module_id).await);
}

#[tokio::test]
async fn test_host_function_integration() {
    let runtime = WasmRuntime::new().await.unwrap();
    
    // Register host function
    runtime.register_host_function(
        "host_multiply".to_string(),
        |args| {
            if let (Some(WasmValue::I32(a)), Some(WasmValue::I32(b))) = (args.get(0), args.get(1)) {
                Ok(vec![WasmValue::I32(a * b)])
            } else {
                Err(WasmError::HostFunction("Invalid arguments".to_string()))
            }
        }
    ).await.unwrap();
    
    // WAT module that uses the host function
    let wat = r#"
        (module
            (import "env" "host_multiply" (func $host_multiply (param i32 i32) (result i32)))
            
            (func $test (param $a i32) (param $b i32) (result i32)
                local.get $a
                local.get $b
                call $host_multiply
            )
            
            (export "test" (func $test))
        )
    "#;
    
    let wasm_bytes = wat::parse_str(wat).unwrap();
    let metadata = ModuleMetadata::new("1.0.0".to_string());
    
    let module_id = runtime.load_module(
        "host_test_module".to_string(),
        &wasm_bytes,
        metadata,
    ).await.unwrap();
    
    // Execute function that calls host function
    let args = vec![WasmValue::I32(6), WasmValue::I32(7)];
    let context = ExecutionContext::new();
    let result = runtime.execute_function(&module_id, "test", &args, context).await.unwrap();
    
    if let WasmValue::I32(product) = &result[0] {
        assert_eq!(*product, 42);
    } else {
        panic!("Expected I32 result");
    }
}

#[tokio::test]
async fn test_execution_timeout() {
    let runtime = WasmRuntime::new().await.unwrap();
    
    // WAT module with an infinite loop
    let wat = r#"
        (module
            (func $infinite_loop (result i32)
                (loop $loop
                    br $loop
                )
                i32.const 42
            )
            (export "infinite_loop" (func $infinite_loop))
        )
    "#;
    
    let wasm_bytes = wat::parse_str(wat).unwrap();
    let metadata = ModuleMetadata::new("1.0.0".to_string());
    
    let module_id = runtime.load_module(
        "timeout_test_module".to_string(),
        &wasm_bytes,
        metadata,
    ).await.unwrap();
    
    // Execute with short timeout
    let context = ExecutionContext::new()
        .with_timeout(Duration::from_millis(100));
    
    let result = runtime.execute_function(&module_id, "infinite_loop", &[], context).await;
    
    // Should timeout
    assert!(result.is_err());
    if let Err(WasmError::ExecutionTimeout { seconds }) = result {
        assert!(seconds <= 1); // Should be less than 1 second
    } else {
        panic!("Expected timeout error");
    }
}

#[tokio::test]
async fn test_memory_operations() {
    let runtime = WasmRuntime::new().await.unwrap();
    
    // WAT module that works with memory
    let wat = r#"
        (module
            (memory $mem 1)
            
            (func $store_and_load (param $offset i32) (param $value i32) (result i32)
                local.get $offset
                local.get $value
                i32.store
                
                local.get $offset
                i32.load
            )
            
            (export "memory" (memory $mem))
            (export "store_and_load" (func $store_and_load))
        )
    "#;
    
    let wasm_bytes = wat::parse_str(wat).unwrap();
    let metadata = ModuleMetadata::new("1.0.0".to_string());
    
    let module_id = runtime.load_module(
        "memory_test_module".to_string(),
        &wasm_bytes,
        metadata,
    ).await.unwrap();
    
    // Test memory operations
    let args = vec![WasmValue::I32(0), WasmValue::I32(42)];
    let context = ExecutionContext::new();
    let result = runtime.execute_function(&module_id, "store_and_load", &args, context).await.unwrap();
    
    if let WasmValue::I32(value) = &result[0] {
        assert_eq!(*value, 42);
    } else {
        panic!("Expected I32 result");
    }
}

#[tokio::test]
async fn test_module_validation_errors() {
    let runtime = WasmRuntime::new().await.unwrap();
    
    // Test invalid bytecode
    let invalid_bytes = vec![0x00, 0x61, 0x73, 0x6d]; // Invalid WASM magic
    let metadata = ModuleMetadata::new("1.0.0".to_string());
    
    let result = runtime.load_module(
        "invalid_module".to_string(),
        &invalid_bytes,
        metadata,
    ).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_nonexistent_function_error() {
    let runtime = WasmRuntime::new().await.unwrap();
    
    let wat = r#"
        (module
            (func $dummy (result i32)
                i32.const 42
            )
            (export "dummy" (func $dummy))
        )
    "#;
    
    let wasm_bytes = wat::parse_str(wat).unwrap();
    let metadata = ModuleMetadata::new("1.0.0".to_string());
    
    let module_id = runtime.load_module(
        "function_test_module".to_string(),
        &wasm_bytes,
        metadata,
    ).await.unwrap();
    
    // Try to call nonexistent function
    let context = ExecutionContext::new();
    let result = runtime.execute_function(&module_id, "nonexistent", &[], context).await;
    
    assert!(result.is_err());
    if let Err(WasmError::FunctionNotFound(name)) = result {
        assert_eq!(name, "nonexistent");
    } else {
        panic!("Expected FunctionNotFound error");
    }
}

#[tokio::test]
async fn test_runtime_statistics() {
    let runtime = WasmRuntime::new().await.unwrap();
    
    // Get initial stats
    let initial_stats = runtime.get_stats().await;
    assert_eq!(initial_stats.modules_loaded, 0);
    assert_eq!(initial_stats.functions_executed, 0);
    
    // Load and execute a module
    let wat = r#"
        (module
            (func $test (result i32)
                i32.const 123
            )
            (export "test" (func $test))
        )
    "#;
    
    let wasm_bytes = wat::parse_str(wat).unwrap();
    let metadata = ModuleMetadata::new("1.0.0".to_string());
    
    let module_id = runtime.load_module(
        "stats_test_module".to_string(),
        &wasm_bytes,
        metadata,
    ).await.unwrap();
    
    let context = ExecutionContext::new();
    let _result = runtime.execute_function(&module_id, "test", &[], context).await.unwrap();
    
    // Check updated stats
    let final_stats = runtime.get_stats().await;
    assert_eq!(final_stats.modules_loaded, 1);
    assert_eq!(final_stats.functions_executed, 1);
    assert!(final_stats.total_load_time > Duration::ZERO);
    assert!(final_stats.total_execution_time > Duration::ZERO);
}

#[tokio::test]
async fn test_direct_module_execution() {
    let runtime = WasmRuntime::new().await.unwrap();
    
    let wat = r#"
        (module
            (func $square (param $x i32) (result i32)
                local.get $x
                local.get $x
                i32.mul
            )
            (export "square" (func $square))
        )
    "#;
    
    let wasm_bytes = wat::parse_str(wat).unwrap();
    
    // Direct execution without explicit loading
    let args = vec![WasmValue::I32(5)];
    let result = runtime.execute_module(&wasm_bytes, "square", &args).await.unwrap();
    
    if let WasmValue::I32(squared) = &result[0] {
        assert_eq!(*squared, 25);
    } else {
        panic!("Expected I32 result");
    }
}

#[tokio::test]
async fn test_multiple_modules() {
    let runtime = WasmRuntime::new().await.unwrap();
    
    // Load multiple modules
    let wat1 = r#"
        (module
            (func $add (param $a i32) (param $b i32) (result i32)
                local.get $a
                local.get $b
                i32.add
            )
            (export "add" (func $add))
        )
    "#;
    
    let wat2 = r#"
        (module
            (func $sub (param $a i32) (param $b i32) (result i32)
                local.get $a
                local.get $b
                i32.sub
            )
            (export "sub" (func $sub))
        )
    "#;
    
    let wasm1 = wat::parse_str(wat1).unwrap();
    let wasm2 = wat::parse_str(wat2).unwrap();
    let metadata = ModuleMetadata::new("1.0.0".to_string());
    
    let module1_id = runtime.load_module(
        "math_add".to_string(),
        &wasm1,
        metadata.clone(),
    ).await.unwrap();
    
    let module2_id = runtime.load_module(
        "math_sub".to_string(),
        &wasm2,
        metadata,
    ).await.unwrap();
    
    // Test both modules
    let context = ExecutionContext::new();
    let args = vec![WasmValue::I32(20), WasmValue::I32(5)];
    
    let add_result = runtime.execute_function(&module1_id, "add", &args, context.clone()).await.unwrap();
    let sub_result = runtime.execute_function(&module2_id, "sub", &args, context).await.unwrap();
    
    if let (WasmValue::I32(sum), WasmValue::I32(diff)) = (&add_result[0], &sub_result[0]) {
        assert_eq!(*sum, 25);
        assert_eq!(*diff, 15);
    } else {
        panic!("Expected I32 results");
    }
    
    // Verify module count
    let modules = runtime.list_modules().await.unwrap();
    assert_eq!(modules.len(), 2);
}

#[tokio::test]
async fn test_configuration_variants() {
    // Test different configurations
    let dev_config = RuntimeConfig::development();
    let prod_config = RuntimeConfig::production();
    let blockchain_config = RuntimeConfig::blockchain();
    let web_config = RuntimeConfig::web();
    
    // All configs should be valid
    assert!(dev_config.validate().is_ok());
    assert!(prod_config.validate().is_ok());
    assert!(blockchain_config.validate().is_ok());
    assert!(web_config.validate().is_ok());
    
    // Test runtime creation with different configs
    let _dev_runtime = WasmRuntime::with_config(dev_config).await.unwrap();
    let _prod_runtime = WasmRuntime::with_config(prod_config).await.unwrap();
    let _blockchain_runtime = WasmRuntime::with_config(blockchain_config).await.unwrap();
    let _web_runtime = WasmRuntime::with_config(web_config).await.unwrap();
}