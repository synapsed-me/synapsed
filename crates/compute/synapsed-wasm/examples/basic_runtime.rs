//! Basic WASM runtime example

use synapsed_wasm::prelude::*;

#[tokio::main]
async fn main() -> WasmResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Synapsed WASM Runtime Example");
    println!("================================");

    // Create runtime configuration
    let config = RuntimeConfig::development();
    println!("📋 Using development configuration");
    println!("   - Debug info: {}", config.debug.enable_debug_info);
    println!("   - Sandboxing: {}", config.security.enable_sandboxing);
    println!("   - Timeout: {:?}", config.limits.default_timeout);

    // Initialize WASM runtime
    println!("\n🔧 Initializing WASM runtime...");
    let runtime = WasmRuntime::with_config(config).await?;
    println!("✅ Runtime initialized successfully");

    // Register a custom host function
    println!("\n📦 Registering host functions...");
    runtime.register_host_function(
        "multiply".to_string(),
        |args| {
            if let (Some(WasmValue::I32(a)), Some(WasmValue::I32(b))) = (args.get(0), args.get(1)) {
                Ok(vec![WasmValue::I32(a * b)])
            } else {
                Ok(vec![WasmValue::I32(0)])
            }
        }
    ).await?;
    println!("✅ Host function 'multiply' registered");

    // Create a simple WASM module using WAT (WebAssembly Text format)
    let wat_source = r#"
        (module
            (import "env" "multiply" (func $multiply (param i32 i32) (result i32)))
            
            (func $add (param $a i32) (param $b i32) (result i32)
                local.get $a
                local.get $b
                i32.add
            )
            
            (func $calculate (param $x i32) (param $y i32) (result i32)
                local.get $x
                local.get $y
                call $add
                local.get $x
                local.get $y
                call $multiply
                i32.add
            )
            
            (export "add" (func $add))
            (export "calculate" (func $calculate))
        )
    "#;

    // Compile WAT to WASM bytecode
    println!("\n🔨 Compiling WAT to WASM...");
    let wasm_bytes = wat::parse_str(wat_source)
        .map_err(|e| WasmError::ModuleCompilation(e.to_string()))?;
    println!("✅ Module compiled successfully ({} bytes)", wasm_bytes.len());

    // Create module metadata
    let metadata = ModuleMetadata::new("1.0.0".to_string())
        .with_capability("arithmetic")
        .with_tag("example");

    // Load the module
    println!("\n📥 Loading WASM module...");
    let module_id = runtime.load_module(
        "arithmetic_module".to_string(),
        &wasm_bytes,
        metadata,
    ).await?;
    println!("✅ Module loaded with ID: {}", module_id);

    // Create execution context
    let context = ExecutionContext::new()
        .with_timeout(std::time::Duration::from_secs(5))
        .with_caller("example_caller");

    // Execute the 'add' function
    println!("\n🚀 Executing 'add' function...");
    let add_args = vec![WasmValue::I32(10), WasmValue::I32(20)];
    let add_result = runtime.execute_function(&module_id, "add", &add_args, context.clone()).await?;
    
    if let Some(WasmValue::I32(result)) = add_result.first() {
        println!("✅ add(10, 20) = {}", result);
    }

    // Execute the 'calculate' function (uses both add and multiply)
    println!("\n🚀 Executing 'calculate' function...");
    let calc_args = vec![WasmValue::I32(5), WasmValue::I32(3)];
    let calc_result = runtime.execute_function(&module_id, "calculate", &calc_args, context).await?;
    
    if let Some(WasmValue::I32(result)) = calc_result.first() {
        println!("✅ calculate(5, 3) = {} (should be 5+3 + 5*3 = 23)", result);
    }

    // Get runtime statistics
    println!("\n📊 Runtime Statistics:");
    let stats = runtime.get_stats().await;
    println!("   - Modules loaded: {}", stats.modules_loaded);
    println!("   - Functions executed: {}", stats.functions_executed);
    println!("   - Average load time: {:?}", stats.average_load_time());
    println!("   - Average execution time: {:?}", stats.average_execution_time());

    // Get module information
    println!("\n📋 Module Information:");
    let module_info = runtime.get_module_info(&module_id).await?;
    println!("   - Version: {}", module_info.version);
    println!("   - Capabilities: {:?}", module_info.capabilities);
    println!("   - Tags: {:?}", module_info.tags);

    // List all loaded modules
    println!("\n📚 Loaded Modules:");
    let modules = runtime.list_modules().await?;
    for (i, module) in modules.iter().enumerate() {
        println!("   {}. {}", i + 1, module);
    }

    // Demonstrate error handling
    println!("\n❌ Testing error handling...");
    match runtime.execute_function(&module_id, "nonexistent", &[], ExecutionContext::new()).await {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("✅ Expected error caught: {}", e),
    }

    // Test module execution with direct bytecode
    println!("\n🔄 Testing direct module execution...");
    let direct_result = runtime.execute_module(&wasm_bytes, "add", &add_args).await?;
    if let Some(WasmValue::I32(result)) = direct_result.first() {
        println!("✅ Direct execution: add(10, 20) = {}", result);
    }

    // Cleanup
    println!("\n🧹 Cleaning up...");
    runtime.unload_module(&module_id).await?;
    println!("✅ Module unloaded");

    // Final statistics
    let final_stats = runtime.get_stats().await;
    println!("\n📊 Final Statistics:");
    println!("   - Modules loaded: {}", final_stats.modules_loaded);
    println!("   - Modules unloaded: {}", final_stats.modules_unloaded);
    println!("   - Current modules: {}", final_stats.current_modules());

    println!("\n🎉 Example completed successfully!");

    Ok(())
}