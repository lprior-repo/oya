#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Memory profiling harness using heaptrack
//!
//! This module provides a functional, panic-free interface for running memory profiling
//! with heaptrack. It monitors RSS (Resident Set Size) every 10 seconds during a sustained
//! 1-hour load test, with profiler overhead kept below 10%.

pub mod config;
pub mod error;
pub mod metrics;
pub mod process;
pub mod runner;

pub use config::ProfilingConfig;
pub use error::{ProfilingError, Result};
pub use metrics::{MemoryMetrics, MetricsSnapshot};
pub use runner::ProfilingRunner;
