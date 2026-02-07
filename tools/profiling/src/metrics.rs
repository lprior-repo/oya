#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! Memory metrics collection and storage

use crate::error::{ProfilingError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Memory metrics snapshot at a point in time
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Timestamp when metrics were captured
    timestamp: DateTime<Utc>,

    /// Process ID being profiled
    pid: u32,

    /// Memory metrics
    metrics: MemoryMetrics,

    /// Elapsed time since profiling started (seconds)
    elapsed_secs: u64,
}

impl MetricsSnapshot {
    /// Create a new metrics snapshot
    #[must_use]
    pub fn new(pid: u32, metrics: MemoryMetrics, elapsed_secs: u64) -> Self {
        Self {
            timestamp: Utc::now(),
            pid,
            metrics,
            elapsed_secs,
        }
    }

    /// Get the timestamp
    #[must_use]
    pub const fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }

    /// Get the process ID
    #[must_use]
    pub const fn pid(&self) -> u32 {
        self.pid
    }

    /// Get the memory metrics
    #[must_use]
    pub const fn metrics(&self) -> &MemoryMetrics {
        &self.metrics
    }

    /// Get elapsed seconds
    #[must_use]
    pub const fn elapsed_secs(&self) -> u64 {
        self.elapsed_secs
    }
}

/// Metric field types from /proc status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetricType {
    VmRss,
    VmSize,
    VmPeak,
    RssAnon,
}

/// Memory metrics from `/proc/[pid]/status`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryMetrics {
    /// Resident Set Size (RSS) in kilobytes
    rss: u64,

    /// Virtual Memory Size (`VmSize`) in kilobytes
    vm_size: u64,

    /// Peak RSS in kilobytes
    vm_peak: u64,

    /// Shared memory in kilobytes
    rss_shared: u64,
}

impl MemoryMetrics {
    /// Create new memory metrics
    #[must_use]
    pub const fn new(rss: u64, vm_size: u64, vm_peak: u64, rss_shared: u64) -> Self {
        Self {
            rss,
            vm_size,
            vm_peak,
            rss_shared,
        }
    }

    /// Get RSS in kilobytes
    #[must_use]
    pub const fn rss_kb(&self) -> u64 {
        self.rss
    }

    /// Get RSS in megabytes
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Acceptable precision loss for display purposes
    pub const fn rss_mb(&self) -> f64 {
        self.rss as f64 / 1024.0
    }

    /// Get virtual memory size in kilobytes
    #[must_use]
    pub const fn vm_size_kb(&self) -> u64 {
        self.vm_size
    }

    /// Get peak virtual memory in kilobytes
    #[must_use]
    pub const fn vm_peak_kb(&self) -> u64 {
        self.vm_peak
    }

    /// Get shared memory in kilobytes
    #[must_use]
    pub const fn rss_shared_kb(&self) -> u64 {
        self.rss_shared
    }

    /// Read memory metrics from `/proc/[pid]/status`
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - `/proc/[pid]/status` cannot be read
    /// - Required fields are missing
    /// - Values cannot be parsed
    pub fn read_from_proc(pid: u32) -> Result<Self> {
        let status_path = format!("/proc/{pid}/status");
        Self::read_from_file(&status_path, pid)
    }

    /// Read metrics from a status file (testable)
    fn read_from_file(path: &str, pid: u32) -> Result<Self> {
        let file = File::open(path)
            .map_err(|e| ProfilingError::MetricsReadFailed(pid, format!("failed to open: {e}")))?;

        let reader = BufReader::new(file);

        /// Intermediate accumulator for parsed metrics
        #[derive(Debug, Default)]
        struct MetricsAccumulator {
            rss_kb: Option<u64>,
            vm_size_kb: Option<u64>,
            vm_peak_kb: Option<u64>,
            rss_shared_kb: Option<u64>,
        }

        // Functional iterator pipeline: read lines -> parse -> fold into accumulator
        let accumulator = reader
            .lines()
            .map(|line_result| {
                line_result.map_err(|e| {
                    ProfilingError::MetricsReadFailed(pid, format!("failed to read line: {e}"))
                })
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter_map(|line| Self::parse_proc_line(&line).transpose())
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .fold(
                MetricsAccumulator::default(),
                |mut acc, (metric_type, value)| {
                    match metric_type {
                        MetricType::VmRss => acc.rss_kb = Some(value),
                        MetricType::VmSize => acc.vm_size_kb = Some(value),
                        MetricType::VmPeak => acc.vm_peak_kb = Some(value),
                        MetricType::RssAnon => acc.rss_shared_kb = Some(value),
                    }
                    acc
                },
            );

        let rss_kb = accumulator
            .rss_kb
            .ok_or_else(|| ProfilingError::MetricsReadFailed(pid, "VmRSS not found".to_string()))?;

        let vm_size_kb = accumulator.vm_size_kb.ok_or_else(|| {
            ProfilingError::MetricsReadFailed(pid, "VmSize not found".to_string())
        })?;

        let vm_peak_kb = accumulator.vm_peak_kb.ok_or_else(|| {
            ProfilingError::MetricsReadFailed(pid, "VmPeak not found".to_string())
        })?;

        // RssAnon is optional (older kernels may not have it)
        let rss_shared_kb = accumulator.rss_shared_kb.unwrap_or(0);

        Ok(Self::new(rss_kb, vm_size_kb, vm_peak_kb, rss_shared_kb))
    }

    /// Parse a single /proc status line into a metric type and value
    /// Format: "`FieldName`:    12345 kB"
    ///
    /// Returns None if the line is not a recognized metric field
    /// Returns error if parsing fails
    fn parse_proc_line(line: &str) -> Result<Option<(MetricType, u64)>> {
        // Extract metric type from line prefix
        let Some(metric_type) = (if line.starts_with("VmRSS:") {
            Some(MetricType::VmRss)
        } else if line.starts_with("VmSize:") {
            Some(MetricType::VmSize)
        } else if line.starts_with("VmPeak:") {
            Some(MetricType::VmPeak)
        } else if line.starts_with("RssAnon:") {
            Some(MetricType::RssAnon)
        } else {
            None
        }) else {
            return Ok(None);
        };

        // Parse the numeric value
        let value = line
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| {
                ProfilingError::MetricsParseError(format!("missing value in line: {line}"))
            })?
            .parse::<u64>()
            .map_err(|e| {
                ProfilingError::MetricsParseError(format!("failed to parse value in '{line}': {e}"))
            })?;

        Ok(Some((metric_type, value)))
    }
}

/// Logger for writing metrics snapshots to a file
pub struct MetricsLogger {
    output_path: std::path::PathBuf,
}

impl MetricsLogger {
    /// Create a new metrics logger
    #[must_use]
    pub const fn new(output_path: std::path::PathBuf) -> Self {
        Self { output_path }
    }

    /// Append a metrics snapshot to the log file (JSON lines format)
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be written or JSON serialization fails
    pub fn log_snapshot(&self, snapshot: &MetricsSnapshot) -> Result<()> {
        use std::io::Write;

        let json = serde_json::to_string(snapshot)?;

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.output_path)
            .map_err(|e| ProfilingError::LogWriteFailed(e.to_string()))?;

        writeln!(file, "{json}").map_err(|e| ProfilingError::LogWriteFailed(e.to_string()))?;

        Ok(())
    }

    /// Read all snapshots from the log file
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be read or JSON parsing fails
    pub fn read_snapshots(&self) -> Result<Vec<MetricsSnapshot>> {
        if !Path::new(&self.output_path).exists() {
            return Ok(Vec::new());
        }

        let file =
            File::open(&self.output_path).map_err(|e| ProfilingError::IoError(e.to_string()))?;

        let reader = BufReader::new(file);

        reader
            .lines()
            .map(|line_result| {
                let line = line_result.map_err(|e| ProfilingError::IoError(e.to_string()))?;
                serde_json::from_str(&line).map_err(Into::into)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_metrics_snapshot_creation() {
        let metrics = MemoryMetrics::new(1024, 2048, 3072, 512);
        let snapshot = MetricsSnapshot::new(1234, metrics, 60);

        assert_eq!(snapshot.pid(), 1234);
        assert_eq!(snapshot.elapsed_secs(), 60);
        assert_eq!(snapshot.metrics().rss_kb(), 1024);
    }

    #[test]
    fn test_memory_metrics_rss_mb() {
        let metrics = MemoryMetrics::new(2048, 4096, 5120, 1024);
        assert!((metrics.rss_mb() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_proc_status() {
        let mut temp_file = NamedTempFile::new().ok().filter(|_| true);
        if let Some(ref mut file) = temp_file {
            let content = "Name:\ttest\nVmRSS:\t   1024 kB\nVmSize:\t   2048 kB\nVmPeak:\t   3072 kB\nRssAnon:\t   512 kB\n";
            let _ = write!(file, "{content}");
            let _ = file.flush();

            let path = file.path().to_str().filter(|s| !s.is_empty());
            if let Some(p) = path {
                let metrics = MemoryMetrics::read_from_file(p, 1234);
                assert!(metrics.is_ok());

                if let Ok(m) = metrics {
                    assert_eq!(m.rss_kb(), 1024);
                    assert_eq!(m.vm_size_kb(), 2048);
                    assert_eq!(m.vm_peak_kb(), 3072);
                    assert_eq!(m.rss_shared_kb(), 512);
                }
            }
        }
    }

    #[test]
    fn test_metrics_logger_roundtrip() {
        let temp_file = NamedTempFile::new().ok().filter(|_| true);
        if let Some(file) = temp_file {
            let logger = MetricsLogger::new(file.path().to_path_buf());
            let metrics = MemoryMetrics::new(1024, 2048, 3072, 512);
            let snapshot = MetricsSnapshot::new(1234, metrics, 60);

            let log_result = logger.log_snapshot(&snapshot);
            assert!(log_result.is_ok());

            let read_result = logger.read_snapshots();
            assert!(read_result.is_ok());

            if let Ok(snapshots) = read_result {
                assert_eq!(snapshots.len(), 1);
                assert_eq!(snapshots[0].pid(), 1234);
            }
        }
    }
}
