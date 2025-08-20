//! WASM runtime benchmarks

use criterion::{criterion_group, criterion_main, Criterion};
use synapsed_wasm::prelude::*;

fn benchmark_runtime_creation(c: &mut Criterion) {
    c.bench_function("runtime_creation", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                WasmRuntime::new().await.unwrap()
            });
        });
    });
}

fn benchmark_module_loading(c: &mut Criterion) {
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
            (export "add" (func $add))
        )
    "#;
    
    let wasm_bytes = wat::parse_str(wat).unwrap();
    let metadata = ModuleMetadata::new("1.0.0".to_string());

    c.bench_function("module_loading", |b| {
        b.iter(|| {
            rt.block_on(async {
                let module_id = runtime.load_module(
                    format!("test_module_{}", uuid::Uuid::new_v4()),
                    &wasm_bytes,
                    metadata.clone(),
                ).await.unwrap();
                
                runtime.unload_module(&module_id).await.unwrap();
            });
        });
    });
}

fn benchmark_function_execution(c: &mut Criterion) {
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
            (export "add" (func $add))
        )
    "#;
    
    let wasm_bytes = wat::parse_str(wat).unwrap();
    let metadata = ModuleMetadata::new("1.0.0".to_string());
    
    let module_id = rt.block_on(async {
        runtime.load_module(
            "benchmark_module".to_string(),
            &wasm_bytes,
            metadata,
        ).await.unwrap()
    });

    c.bench_function("function_execution", |b| {
        b.iter(|| {
            rt.block_on(async {
                let args = vec![WasmValue::I32(10), WasmValue::I32(20)];
                let context = ExecutionContext::new();
                runtime.execute_function(&module_id, "add", &args, context).await.unwrap()
            });
        });
    });
}

criterion_group!(benches, benchmark_runtime_creation, benchmark_module_loading, benchmark_function_execution);
criterion_main!(benches);