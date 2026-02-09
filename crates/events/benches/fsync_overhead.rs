// Fsync Overhead Benchmark
//
// Measures fsync write latency to verify 2-3ms overhead is acceptable.
// Uses criterion for statistically significant measurements.
//
// Performance Targets:
// - Event append with fsync: <3ms (p99)
// - Event append without fsync: <0.5ms (p99)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

/// Benchmark fixture for temp file management
pub struct BenchmarkFixture {
    temp_dir: TempDir,
}

impl BenchmarkFixture {
    /// Create isolated temporary directory for benchmark
    pub fn setup() -> Result<Self, String> {
        TempDir::new()
            .map(|temp_dir| Self { temp_dir })
            .map_err(|e| format!("Failed to create temp dir: {}", e))
    }

    /// Get path to test file
    pub fn test_file_path(&self) -> std::path::PathBuf {
        self.temp_dir.path().join("benchmark_data.bin")
    }
}

/// Core benchmark function (measured by Criterion)
/// Measures time to write data + fsync
async fn benchmark_fsync_overhead(file_size: usize) -> Result<(), String> {
    let fixture = BenchmarkFixture::setup()?;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(fixture.test_file_path())
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let data = vec![0u8; file_size];

    file.write_all(&data)
        .await
        .map_err(|e| format!("Write failed: {}", e))?;

    file.sync_all()
        .await
        .map_err(|e| format!("Fsync failed: {}", e))?;

    Ok(())
}

/// Baseline benchmark (write without fsync)
async fn benchmark_write_no_fsync(file_size: usize) -> Result<(), String> {
    let fixture = BenchmarkFixture::setup()?;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(fixture.test_file_path())
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let data = vec![0u8; file_size];

    file.write_all(&data)
        .await
        .map_err(|e| format!("Write failed: {}", e))?;

    Ok(())
}

/// Criterion benchmark wrapper for fsync overhead
fn bench_fsync_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("fsync_overhead");

    // Configure measurement time and sample size
    group
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(10))
        .sample_size(100);

    let file_sizes = vec![1024, 10 * 1024, 100 * 1024]; // 1KB, 10KB, 100KB

    for size in file_sizes {
        let size_label = format!("{}_bytes", size);

        // Benchmark with fsync
        group.bench_with_input(
            BenchmarkId::new("with_fsync", &size_label),
            &size,
            |b, &size| {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        eprintln!("Failed to create runtime: {}", e);
                        return;
                    }
                };

                b.iter(|| {
                    let future = benchmark_fsync_overhead(black_box(size));
                    match rt.block_on(future) {
                        Ok(_) => {}
                        Err(e) => eprintln!("Benchmark error: {}", e),
                    }
                });
            },
        );

        // Benchmark without fsync (baseline)
        group.bench_with_input(
            BenchmarkId::new("without_fsync", &size_label),
            &size,
            |b, &size| {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        eprintln!("Failed to create runtime: {}", e);
                        return;
                    }
                };

                b.iter(|| {
                    let future = benchmark_write_no_fsync(black_box(size));
                    match rt.block_on(future) {
                        Ok(_) => {}
                        Err(e) => eprintln!("Benchmark error: {}", e),
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_fsync_overhead);
criterion_main!(benches);
