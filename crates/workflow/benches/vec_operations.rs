#![allow(dead_code)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn benchmark_vec_build(c: &mut Criterion) {
    c.bench_function("imperative_vec_build", |b| {
        b.iter(|| {
            let mut result = Vec::new();
            for i in 0..100 {
                result.push(i * 2);
            }
            black_box(result)
        })
    });

    c.bench_function("functional_vec_build", |b| {
        b.iter(|| {
            let result: Vec<usize> = (0..100).map(|i| i * 2).collect();
            black_box(result)
        })
    });
}

pub fn benchmark_vec_transform(c: &mut Criterion) {
    let data: Vec<usize> = (0..1000).collect();

    c.bench_function("imperative_vec_transform", |b| {
        b.iter(|| {
            let mut result = Vec::with_capacity(data.len());
            for item in &data {
                result.push(item * 2);
            }
            black_box(result)
        })
    });

    c.bench_function("functional_vec_transform", |b| {
        b.iter(|| {
            let result: Vec<usize> = data.iter().map(|x| x * 2).collect();
            black_box(result)
        })
    });
}

pub fn benchmark_vec_filter_map(c: &mut Criterion) {
    let data: Vec<usize> = (0..1000).collect();

    c.bench_function("imperative_filter_map", |b| {
        b.iter(|| {
            let mut result = Vec::new();
            for item in &data {
                if item % 2 == 0 {
                    result.push(item * 2);
                }
            }
            black_box(result)
        })
    });

    c.bench_function("functional_filter_map", |b| {
        b.iter(|| {
            let result: Vec<usize> = data.iter().filter(|x| *x % 2 == 0).map(|x| x * 2).collect();
            black_box(result)
        })
    });
}

pub fn benchmark_vec_partition(c: &mut Criterion) {
    let data: Vec<usize> = (0..1000).collect();

    c.bench_function("imperative_partition", |b| {
        b.iter(|| {
            let mut evens = Vec::new();
            let mut odds = Vec::new();
            for item in &data {
                if item % 2 == 0 {
                    evens.push(*item);
                } else {
                    odds.push(*item);
                }
            }
            black_box((evens, odds))
        })
    });

    c.bench_function("functional_partition", |b| {
        b.iter(|| {
            let (evens, odds): (Vec<&usize>, Vec<&usize>) = data.iter().partition(|x| *x % 2 == 0);
            black_box((evens, odds))
        })
    });
}

pub fn benchmark_vec_fold(c: &mut Criterion) {
    let data: Vec<usize> = (0..1000).collect();

    c.bench_function("imperative_fold", |b| {
        b.iter(|| {
            let mut product = 1usize;
            for item in &data {
                product = product.saturating_mul(*item);
            }
            black_box(product)
        })
    });

    c.bench_function("functional_fold", |b| {
        b.iter(|| {
            let product = data.iter().fold(1usize, |acc, x| acc.saturating_mul(*x));
            black_box(product)
        })
    });
}

criterion_group!(
    benches,
    benchmark_vec_build,
    benchmark_vec_transform,
    benchmark_vec_filter_map,
    benchmark_vec_partition,
    benchmark_vec_fold
);
criterion_main!(benches);
