//! Module execution benchmarks

use criterion::{criterion_group, criterion_main, Criterion};
use synapsed_wasm::prelude::*;

fn benchmark_simple_arithmetic(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let runtime = rt.block_on(async {
        WasmRuntime::new().await.unwrap()
    });

    let wat = r#"
        (module
            (func $add (param $a i32) (param $b i32) (result i32)
                local.get $a
                local.get $b
                i32.add
            )
            (func $multiply (param $a i32) (param $b i32) (result i32)
                local.get $a
                local.get $b
                i32.mul
            )
            (export "add" (func $add))
            (export "multiply" (func $multiply))
        )
    "#;
    
    let wasm_bytes = wat::parse_str(wat).unwrap();
    let metadata = ModuleMetadata::new("1.0.0".to_string());
    
    let module_id = rt.block_on(async {
        runtime.load_module(
            "arithmetic_module".to_string(),
            &wasm_bytes,
            metadata,
        ).await.unwrap()
    });

    let args = vec![WasmValue::I32(100), WasmValue::I32(200)];
    let context = ExecutionContext::new();

    c.bench_function("arithmetic_add", |b| {
        b.iter(|| {
            rt.block_on(async {
                runtime.execute_function(&module_id, "add", &args, context.clone()).await.unwrap()
            });
        });
    });

    c.bench_function("arithmetic_multiply", |b| {
        b.iter(|| {
            rt.block_on(async {
                runtime.execute_function(&module_id, "multiply", &args, context.clone()).await.unwrap()
            });
        });
    });
}

fn benchmark_memory_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let runtime = rt.block_on(async {
        WasmRuntime::new().await.unwrap()
    });

    let wat = r#"
        (module
            (memory $mem 1)
            
            (func $store_load (param $offset i32) (param $value i32) (result i32)
                local.get $offset
                local.get $value
                i32.store
                
                local.get $offset
                i32.load
            )
            
            (export "memory" (memory $mem))
            (export "store_load" (func $store_load))
        )
    "#;
    
    let wasm_bytes = wat::parse_str(wat).unwrap();
    let metadata = ModuleMetadata::new("1.0.0".to_string());
    
    let module_id = rt.block_on(async {
        runtime.load_module(
            "memory_module".to_string(),
            &wasm_bytes,
            metadata,
        ).await.unwrap()
    });

    c.bench_function("memory_store_load", |b| {
        b.iter(|| {
            rt.block_on(async {
                let args = vec![WasmValue::I32(0), WasmValue::I32(42)];
                let context = ExecutionContext::new();
                runtime.execute_function(&module_id, "store_load", &args, context).await.unwrap()
            });
        });
    });
}

fn benchmark_loop_execution(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let runtime = rt.block_on(async {
        WasmRuntime::new().await.unwrap()
    });

    let wat = r#"
        (module
            (func $fibonacci (param $n i32) (result i32)
                (local $a i32)
                (local $b i32)
                (local $temp i32)
                (local $i i32)
                
                i32.const 0
                local.set $a
                i32.const 1
                local.set $b
                i32.const 0
                local.set $i
                
                (loop $loop
                    local.get $i
                    local.get $n
                    i32.ge_s
                    br_if 1
                    
                    local.get $a
                    local.get $b
                    i32.add
                    local.set $temp
                    
                    local.get $b
                    local.set $a
                    local.get $temp
                    local.set $b
                    
                    local.get $i
                    i32.const 1
                    i32.add
                    local.set $i
                    
                    br $loop
                )
                
                local.get $a
            )
            
            (export "fibonacci" (func $fibonacci))
        )
    "#;
    
    let wasm_bytes = wat::parse_str(wat).unwrap();
    let metadata = ModuleMetadata::new("1.0.0".to_string());
    
    let module_id = rt.block_on(async {
        runtime.load_module(
            "fibonacci_module".to_string(),
            &wasm_bytes,
            metadata,
        ).await.unwrap()
    });

    c.bench_function("fibonacci_20", |b| {
        b.iter(|| {
            rt.block_on(async {
                let args = vec![WasmValue::I32(20)];
                let context = ExecutionContext::new();
                runtime.execute_function(&module_id, "fibonacci", &args, context).await.unwrap()
            });
        });
    });
}

criterion_group!(benches, benchmark_simple_arithmetic, benchmark_memory_operations, benchmark_loop_execution);
criterion_main!(benches);