use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn basic_benchmark(c: &mut Criterion) {
    c.bench_function("placeholder", |b| {
        b.iter(|| {
            // Placeholder benchmark
            black_box(1 + 1)
        })
    });
}

criterion_group!(benches, basic_benchmark);
criterion_main!(benches);