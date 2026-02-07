use criterion::{black_box, criterion_group, criterion_main, Criterion};
use im::HashMap as ImHashMap;

pub fn benchmark_hashmap_insert(c: &mut Criterion) {
    c.bench_function("std_hashmap_insert", |b| {
        b.iter(|| {
            let mut map = std::collections::HashMap::new();
            for i in 0..100 {
                map.insert(i, i * 2);
            }
            black_box(map)
        })
    });

    c.bench_function("im_hashmap_insert", |b| {
        b.iter(|| {
            let mut map = ImHashMap::new();
            for i in 0..100 {
                map = map.update(i, i * 2);
            }
            black_box(map)
        })
    });
}

pub fn benchmark_hashmap_lookup(c: &mut Criterion) {
    let mut std_map = std::collections::HashMap::new();
    let mut im_map = ImHashMap::new();
    for i in 0..100 {
        std_map.insert(i, i * 2);
        im_map = im_map.update(i, i * 2);
    }

    c.bench_function("std_hashmap_lookup", |b| {
        b.iter(|| {
            for i in 0..100 {
                black_box(std_map.get(&i));
            }
        })
    });

    c.bench_function("im_hashmap_lookup", |b| {
        b.iter(|| {
            for i in 0..100 {
                black_box(im_map.get(&i));
            }
        })
    });
}

pub fn benchmark_hashmap_clone(c: &mut Criterion) {
    let std_map: std::collections::HashMap<_, _> = (0..100).map(|i| (i, i * 2)).collect();
    let im_map: ImHashMap<_, _> = (0..100).map(|i| (i, i * 2)).collect();

    c.bench_function("std_hashmap_clone", |b| {
        b.iter(|| black_box(std_map.clone()))
    });

    c.bench_function("im_hashmap_clone", |b| b.iter(|| black_box(im_map.clone())));
}

pub fn benchmark_hashmap_iteration(c: &mut Criterion) {
    let std_map: std::collections::HashMap<_, _> = (0..100).map(|i| (i, i * 2)).collect();
    let im_map: ImHashMap<_, _> = (0..100).map(|i| (i, i * 2)).collect();

    c.bench_function("std_hashmap_iterate", |b| {
        b.iter(|| {
            let sum: usize = std_map.values().sum();
            black_box(sum)
        })
    });

    c.bench_function("im_hashmap_iterate", |b| {
        b.iter(|| {
            let sum: usize = im_map.values().sum();
            black_box(sum)
        })
    });
}

criterion_group!(
    benches,
    benchmark_hashmap_insert,
    benchmark_hashmap_lookup,
    benchmark_hashmap_clone,
    benchmark_hashmap_iteration
);
criterion_main!(benches);
