#![allow(dead_code)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;

pub fn benchmark_clone_vs_arc(c: &mut Criterion) {
    let large_data = vec![0u8; 10_000];

    c.bench_function("deep_clone", |b| b.iter(|| black_box(large_data.clone())));

    let arc_data = Arc::new(large_data);
    c.bench_function("arc_clone", |b| b.iter(|| black_box(Arc::clone(&arc_data))));
}

pub fn benchmark_arc_read_performance(c: &mut Criterion) {
    let data = vec![42u8; 1_000];
    let cloned_data = data.clone();
    let arc_data = Arc::new(data);

    c.bench_function("owned_access", |b| {
        b.iter(|| {
            let sum: u32 = cloned_data.iter().map(|&x| x as u32).sum();
            black_box(sum)
        })
    });

    c.bench_function("arc_access", |b| {
        b.iter(|| {
            let sum: u32 = arc_data.iter().map(|&x| x as u32).sum();
            black_box(sum)
        })
    });
}

pub fn benchmark_struct_clone_vs_arc(c: &mut Criterion) {
    #[derive(Clone)]
    struct LargeStruct {
        _data: Vec<u64>,
        _metadata: Vec<String>,
    }

    let large_struct = LargeStruct {
        _data: (0..1000).collect(),
        _metadata: (0..100).map(|i| format!("item_{}", i)).collect(),
    };

    c.bench_function("struct_clone", |b| {
        b.iter(|| black_box(large_struct.clone()))
    });

    let arc_struct = Arc::new(large_struct);
    c.bench_function("struct_arc_clone", |b| {
        b.iter(|| black_box(Arc::clone(&arc_struct)))
    });
}

criterion_group!(
    benches,
    benchmark_clone_vs_arc,
    benchmark_arc_read_performance,
    benchmark_struct_clone_vs_arc
);
criterion_main!(benches);
