#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::path::PathBuf;
use std::time::Duration;
use std::io;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Benchmark configuration with validation
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of samples per benchmark (min 100)
    pub sample_size: usize,
    /// Warm-up time in seconds
    pub warm_up_duration: Duration,
    /// Measurement time in seconds
    pub measurement_duration: Duration,
}

impl BenchmarkConfig {
    /// Create default configuration meeting success criteria
    pub fn default() -> Self {
        Self {
            sample_size: 100,
            warm_up_duration: Duration::from_secs(3),
            measurement_duration: Duration::from_secs(10),
        }
    }

    /// Validate configuration invariants
    pub fn validate(&self) -> Result<(), ConfigurationError> {
        match self.sample_size >= 100 {
            true => Ok(()),
            false => Err(ConfigurationError::InvalidSampleSize {
                actual: self.sample_size,
                required: 100,
            }),
        }
    }
}

/// Configuration errors
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConfigurationError {
    #[error("sample_size must be >= {required}, got {actual}")]
    InvalidSampleSize { actual: usize, required: usize },
}

/// Benchmark setup helper (functional, zero unwraps)
pub struct BenchmarkFixture {
    temp_dir: tempfile::TempDir,
    test_file: PathBuf,
}

impl BenchmarkFixture {
    /// Create isolated temporary directory for benchmark
    pub fn setup() -> Result<Self, BenchmarkSetupError> {
        tempfile::tempdir()
            .map(|temp_dir| {
                let test_file = temp_dir.path().join("benchmark_data.bin");
                Self { temp_dir, test_file }
            })
            .map_err(|e| BenchmarkSetupError::TempDirCreationFailed {
                reason: e.to_string(),
            })
    }

    /// Get path to test file
    pub fn test_file_path(&self) -> &PathBuf {
        &self.test_file
    }

    /// Write test data and return file handle
    pub async fn write_test_data(&self, size: usize) -> Result<File, BenchmarkError> {
        let data = vec![0u8; size];

        File::create(&self.test_file)
            .await
            .map_err(|e| BenchmarkError::FileCreationFailed {
                path: self.test_file.clone(),
                reason: e.to_string(),
            })
            .and_then(|mut file| async move {
                file.write_all(&data)
                    .await
                    .map_err(|e| BenchmarkError::WriteFailed {
                        path: self.test_file.clone(),
                        reason: e.to_string(),
                    })?;
                Ok(file)
            })
            .await
    }
}

/// Benchmark setup errors
#[derive(Debug, Error)]
pub enum BenchmarkSetupError {
    #[error("failed to create temp dir: {reason}")]
    TempDirCreationFailed { reason: String },
}

/// Benchmark execution errors
#[derive(Debug, Error)]
pub enum BenchmarkError {
    #[error("failed to create file {path}: {reason}")]
    FileCreationFailed { path: PathBuf, reason: String },

    #[error("failed to write to {path}: {reason}")]
    WriteFailed { path: PathBuf, reason: String },

    #[error("failed to sync {path}: {reason}")]
    SyncFailed { path: PathBuf, reason: String },
}

/// Core benchmark function (measured by Criterion)
/// Measures time to write data + fsync
async fn benchmark_fsync_overhead(file_size: usize) -> Result<(), BenchmarkError> {
    let fixture = BenchmarkFixture::setup()?;

    let bench_func = async {
        let mut file = fixture.write_test_data(file_size).await?;

        file.sync_all()
            .await
            .map_err(|e| BenchmarkError::SyncFailed {
                path: fixture.test_file_path().clone(),
                reason: e.to_string(),
            })?;

        Ok::<(), BenchmarkError>(())
    };

    bench_func.await
}

/// Baseline benchmark (write without fsync)
async fn benchmark_write_no_fsync(file_size: usize) -> Result<(), BenchmarkError> {
    let fixture = BenchmarkFixture::setup()?;

    let bench_func = async {
        fixture.write_test_data(file_size).await?;
        Ok::<(), BenchmarkError>(())
    };

    bench_func.await
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
        let size_str = format!("{} bytes", size);

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            BenchmarkId::new("with_fsync", size_str),
            &size,
            |b, &size| {
                // Run synchronous wrapper around async benchmark
                let rt = tokio::runtime::Runtime::new()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to create runtime: {}", e);
                        std::process::exit(1);
                    });

                b.iter(|| {
                    let future = benchmark_fsync_overhead(black_box(size));
                    rt.block_on(future).map_or_else(
                        |e| {
                            eprintln!("Benchmark error: {}", e);
                        },
                        |_| {},
                    );
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("without_fsync", size_str),
            &size,
            |b, &size| {
                let rt = tokio::runtime::Runtime::new()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to create runtime: {}", e);
                        std::process::exit(1);
                    });

                b.iter(|| {
                    let future = benchmark_write_no_fsync(black_box(size));
                    rt.block_on(future).map_or_else(
                        |e| {
                            eprintln!("Benchmark error: {}", e);
                        },
                        |_| {},
                    );
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_fsync_overhead);
criterion_main!(benches);
