//! Benchmark for StreamChunk performance improvements
//!
//! This benchmark compares the performance of the original Vec<u8> implementation
//! vs the new Bytes zero-copy implementation.

use bytes::Bytes;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::time::Duration;

// Simulate the old Vec<u8> implementation
#[derive(Debug, Clone)]
struct StreamChunkOld {
    _stream_id: String,
    data: Vec<u8>,
    _offset: u64,
}

impl StreamChunkOld {
    fn new(stream_id: impl Into<String>, data: Vec<u8>, offset: u64) -> Self {
        Self {
            _stream_id: stream_id.into(),
            data,
            _offset: offset,
        }
    }

    fn slice(&self, start: usize, end: usize) -> Vec<u8> {
        self.data[start..end].to_vec() // This copies!
    }
}

// New implementation with Bytes
#[derive(Debug, Clone)]
struct StreamChunkNew {
    _stream_id: String,
    data: Bytes,
    _offset: u64,
}

impl StreamChunkNew {
    fn new(stream_id: impl Into<String>, data: Vec<u8>, offset: u64) -> Self {
        Self {
            _stream_id: stream_id.into(),
            data: Bytes::from(data),
            _offset: offset,
        }
    }

    fn slice(&self, start: usize, end: usize) -> Bytes {
        self.data.slice(start..end) // Zero-copy!
    }
}

/// Generate test data of various sizes
fn generate_test_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

fn criterion_benchmark(c: &mut Criterion) {
    // Test various data sizes to show performance characteristics
    let test_sizes = [
        10,     // Very small (few bytes)
        100,    // Small
        1000,   // Medium
        10000,  // Large
        100000, // Very large
    ];

    for &size in &test_sizes {
        let test_data = generate_test_data(size);
        let chunk_old = StreamChunkOld::new("stream-1", test_data.clone(), 0);
        let chunk_new = StreamChunkNew::new("stream-1", test_data.clone(), 0);

        let mut group = c.benchmark_group(format!("slice_{}_bytes", size));
        group.measurement_time(Duration::from_secs(5));

        // Benchmark old implementation
        group.bench_function("vec_copy", |b| {
            b.iter(|| {
                let slice = chunk_old.slice(black_box(0), black_box(size / 2));
                black_box(slice);
            })
        });

        // Benchmark new implementation
        group.bench_function("bytes_zero_copy", |b| {
            b.iter(|| {
                let slice = chunk_new.slice(black_box(0), black_box(size / 2));
                black_box(slice);
            })
        });

        group.finish();
    }
}

fn criterion_benchmark_construction(c: &mut Criterion) {
    let test_sizes = [100, 1000, 10000];

    for &size in &test_sizes {
        let test_data = generate_test_data(size);

        let mut group = c.benchmark_group(format!("construct_{}_bytes", size));
        group.measurement_time(Duration::from_secs(3));

        // Benchmark old construction
        group.bench_function("vec_construction", |b| {
            b.iter(|| {
                let chunk = StreamChunkOld::new("stream-1", black_box(test_data.clone()), 0);
                black_box(chunk);
            })
        });

        // Benchmark new construction
        group.bench_function("bytes_construction", |b| {
            b.iter(|| {
                let chunk = StreamChunkNew::new("stream-1", black_box(test_data.clone()), 0);
                black_box(chunk);
            })
        });

        group.finish();
    }
}

fn criterion_benchmark_cloning(c: &mut Criterion) {
    let test_sizes = [1000, 10000];

    for &size in &test_sizes {
        let test_data = generate_test_data(size);
        let chunk_old = StreamChunkOld::new("stream-1", test_data.clone(), 0);
        let chunk_new = StreamChunkNew::new("stream-1", test_data.clone(), 0);

        let mut group = c.benchmark_group(format!("clone_{}_bytes", size));
        group.measurement_time(Duration::from_secs(3));

        // Benchmark old cloning (copies data)
        group.bench_function("vec_clone", |b| {
            b.iter(|| {
                let cloned = black_box(chunk_old.clone());
                black_box(cloned);
            })
        });

        // Benchmark new cloning (reference count increment - O(1))
        group.bench_function("bytes_clone", |b| {
            b.iter(|| {
                let cloned = black_box(chunk_new.clone());
                black_box(cloned);
            })
        });

        group.finish();
    }
}

criterion_group!(
    benches,
    criterion_benchmark,
    criterion_benchmark_construction,
    criterion_benchmark_cloning
);
criterion_main!(benches);
