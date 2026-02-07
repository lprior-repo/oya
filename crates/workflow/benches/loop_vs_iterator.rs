use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_loop_vs_iterator(c: &mut Criterion) {
    let data: Vec<usize> = (0..1000).collect();

    c.bench_function("imperative_loop", |b| {
        b.iter(|| {
            let mut sum = 0;
            for item in &data {
                sum += item * 2;
            }
            black_box(sum)
        })
    });

    c.bench_function("functional_iterator", |b| {
        b.iter(|| {
            let sum: usize = data.iter().map(|x| x * 2).sum();
            black_box(sum)
        })
    });
}

fn benchmark_filter_operations(c: &mut Criterion) {
    let data: Vec<usize> = (0..1000).collect();

    c.bench_function("imperative_filter", |b| {
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

    c.bench_function("functional_filter", |b| {
        b.iter(|| {
            let result: Vec<usize> = data.iter().filter(|x| *x % 2 == 0).map(|x| x * 2).collect();
            black_box(result)
        })
    });
}

fn benchmark_nested_loops(c: &mut Criterion) {
    let data1: Vec<usize> = (0..100).collect();
    let data2: Vec<usize> = (0..100).collect();

    c.bench_function("imperative_nested_loop", |b| {
        b.iter(|| {
            let mut result = Vec::new();
            for a in &data1 {
                for b in &data2 {
                    if a + b < 100 {
                        result.push(a * b);
                    }
                }
            }
            black_box(result)
        })
    });

    c.bench_function("functional_nested_iter", |b| {
        b.iter(|| {
            let result: Vec<usize> = data1
                .iter()
                .flat_map(|a| data2.iter().map(move |b| (a, b)))
                .filter(|(a, b)| *a + *b < 100)
                .map(|(a, b)| a * b)
                .collect();
            black_box(result)
        })
    });
}

criterion_group!(
    benches,
    benchmark_loop_vs_iterator,
    benchmark_filter_operations,
    benchmark_nested_loops
);
criterion_main!(benches);
