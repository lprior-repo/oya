//! Oya Zellij UI - Terminal UI components for Oya

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod correlation;
pub mod log;
pub mod metrics;
pub mod timer;
pub mod web_client;

pub use correlation::keys;
pub use correlation::{CorrelationContext, RequestId};
pub use log::{LogAggregator, LogEntry, LogLevel, LogSource};
pub use metrics::{AgentMetrics, MetricsSnapshot, PoolMetrics};
pub use timer::{RefreshTimer, TimerConfig, TimerError, TimerEvent, TimerState};
pub use web_client::{HttpResponse, WebClient, WebClientConfig, WebClientError};
