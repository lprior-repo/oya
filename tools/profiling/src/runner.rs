#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! Main profiling runner orchestrating the 1-hour load test

use crate::config::ProfilingConfig;
use crate::error::{ProfilingError, Result};
use crate::metrics::{MemoryMetrics, MetricsLogger, MetricsSnapshot};
use crate::process::ProfiledProcess;
use std::thread;
use std::time::Instant;

/// Main profiling runner
pub struct ProfilingRunner {
    config: ProfilingConfig,
}

impl ProfilingRunner {
    /// Create a new profiling runner
    #[must_use]
    pub const fn new(config: ProfilingConfig) -> Self {
        Self { config }
    }

    /// Run the profiling session
    ///
    /// This will:
    /// 1. Spawn the command under heaptrack
    /// 2. Sample RSS every `sampling_interval` seconds
    /// 3. Log metrics to the output file
    /// 4. Run for the configured duration
    /// 5. Clean up the process
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Process fails to spawn
    /// - Process terminates unexpectedly
    /// - Metrics collection fails
    /// - Log writing fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use oya_profiling::{ProfilingConfig, ProfilingRunner};
    /// # use std::time::Duration;
    /// # use std::path::PathBuf;
    /// let config = ProfilingConfig::one_hour_default(
    ///     "my-app".to_string(),
    ///     vec!["--load-test".to_string()],
    /// ).unwrap();
    ///
    /// let runner = ProfilingRunner::new(config);
    /// let result = runner.run();
    /// ```
    pub fn run(self) -> Result<ProfilingSummary> {
        let logger = MetricsLogger::new(self.config.output_path().clone());

        // Spawn the profiled process
        let mut process = ProfiledProcess::spawn(
            self.config.command(),
            self.config.args(),
            self.config.working_dir(),
        )?;

        let pid = process.pid();
        let start_time = Instant::now();
        let duration = self.config.duration();
        let sampling_interval = self.config.sampling_interval();

        let mut sample_count = 0_u64;
        let mut max_rss_kb = 0_u64;
        let mut total_rss_kb = 0_u64;

        // Main profiling loop
        loop {
            let elapsed = start_time.elapsed();

            // Check if duration exceeded
            if elapsed >= duration {
                break;
            }

            // Check if process is still running
            let is_running = process.is_running()?;
            if !is_running {
                return Err(ProfilingError::ProcessTerminated(
                    "process exited before profiling duration completed".to_string(),
                ));
            }

            // Collect metrics
            let metrics_result = MemoryMetrics::read_from_proc(pid);

            match metrics_result {
                Ok(metrics) => {
                    let snapshot = MetricsSnapshot::new(pid, metrics.clone(), elapsed.as_secs());

                    // Update statistics
                    sample_count += 1;
                    let rss = metrics.rss_kb();
                    total_rss_kb += rss;
                    if rss > max_rss_kb {
                        max_rss_kb = rss;
                    }

                    // Log snapshot
                    logger.log_snapshot(&snapshot)?;
                }
                Err(e) => {
                    // Log error but continue (process may have just exited)
                    eprintln!("Warning: failed to read metrics: {e}");
                }
            }

            // Sleep until next sample
            thread::sleep(sampling_interval);
        }

        // Terminate the process
        let _ = process.kill(); // Ignore error if already exited

        // Calculate summary statistics
        let avg_rss_kb = if sample_count > 0 {
            total_rss_kb / sample_count
        } else {
            0
        };

        Ok(ProfilingSummary {
            sample_count,
            max_rss_kb,
            avg_rss_kb,
            duration_secs: start_time.elapsed().as_secs(),
        })
    }

    /// Run the profiling session in a non-blocking way (returns immediately)
    ///
    /// This spawns a background thread that runs the profiling session.
    /// Use this when you want to profile without blocking the main thread.
    ///
    /// # Errors
    ///
    /// Returns error if thread spawning fails
    pub fn run_background(self) -> Result<ProfilingHandle> {
        let handle = thread::Builder::new()
            .name("profiling-runner".to_string())
            .spawn(move || self.run())
            .map_err(|e| ProfilingError::ProcessSpawnFailed(format!("thread spawn: {e}")))?;

        Ok(ProfilingHandle { handle })
    }
}

/// Summary statistics from a profiling run
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfilingSummary {
    /// Number of samples collected
    sample_count: u64,

    /// Maximum RSS observed (kilobytes)
    max_rss_kb: u64,

    /// Average RSS (kilobytes)
    avg_rss_kb: u64,

    /// Actual duration profiled (seconds)
    duration_secs: u64,
}

impl ProfilingSummary {
    /// Get the sample count
    #[must_use]
    pub const fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// Get the maximum RSS in kilobytes
    #[must_use]
    pub const fn max_rss_kb(&self) -> u64 {
        self.max_rss_kb
    }

    /// Get the maximum RSS in megabytes
    #[must_use]
    pub const fn max_rss_mb(&self) -> f64 {
        self.max_rss_kb as f64 / 1024.0
    }

    /// Get the average RSS in kilobytes
    #[must_use]
    pub const fn avg_rss_kb(&self) -> u64 {
        self.avg_rss_kb
    }

    /// Get the average RSS in megabytes
    #[must_use]
    pub const fn avg_rss_mb(&self) -> f64 {
        self.avg_rss_kb as f64 / 1024.0
    }

    /// Get the duration in seconds
    #[must_use]
    pub const fn duration_secs(&self) -> u64 {
        self.duration_secs
    }
}

/// Handle for a background profiling run
pub struct ProfilingHandle {
    handle: thread::JoinHandle<Result<ProfilingSummary>>,
}

impl ProfilingHandle {
    /// Wait for the profiling run to complete
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Thread panicked
    /// - Profiling run failed
    pub fn join(self) -> Result<ProfilingSummary> {
        self.handle
            .join()
            .map_err(|_| ProfilingError::ProcessTerminated("profiling thread panicked".to_string()))
            .and_then(|result| result)
    }

    /// Check if the profiling run has completed
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    #[test]
    fn test_profiling_summary() {
        let summary = ProfilingSummary {
            sample_count: 360,
            max_rss_kb: 102400,
            avg_rss_kb: 51200,
            duration_secs: 3600,
        };

        assert_eq!(summary.sample_count(), 360);
        assert_eq!(summary.max_rss_kb(), 102400);
        assert!((summary.max_rss_mb() - 100.0).abs() < 0.1);
        assert_eq!(summary.avg_rss_kb(), 51200);
        assert!((summary.avg_rss_mb() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_runner_creation() {
        let config = ProfilingConfig::new(
            Duration::from_secs(10),
            Duration::from_secs(5),
            PathBuf::from("/tmp/test-profile.jsonl"),
            "echo".to_string(),
            vec!["test".to_string()],
        );

        assert!(config.is_ok());
        if let Ok(cfg) = config {
            let runner = ProfilingRunner::new(cfg);
            // Just verify it constructs
            assert_eq!(runner.config.duration().as_secs(), 10);
        }
    }
}
