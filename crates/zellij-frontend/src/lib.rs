// zellij-frontend - Zellij WASM plugin and UI components for OYA SDLC visualization
//
// This crate provides a terminal-based UI for visualizing OYA workflows,
// including bead status, pipeline progress, and workflow graphs.
//
// Architecture:
// - Plugin: Main plugin entry point implementing Zellij protocol
// - Layout: 3-pane layout system (BeadList, BeadDetail, WorkflowGraph)
// - IPC: Communication with oya-orchestrator for real-time data
// - Components: UI widgets for rendering different views
// - Correlation: Request correlation for distributed tracing
// - Log: Multi-source log aggregation and display
// - Metrics: Agent pool and performance metrics
// - Timer: Auto-refresh timer for periodic UI updates
// - Web Client: HTTP client with graceful error handling

#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(clippy::panic)]
#![warn(clippy::unimplemented)]
#![warn(clippy::unreachable)]
#![warn(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]
#![warn(clippy::unwrap_in_result)]

// Zellij plugin modules (from oya-ui)
pub mod backpressure;
pub mod components;
pub mod exports;
pub mod ipc;
pub mod ipc_zellij;
pub mod layout;
pub mod plugin;
pub mod render;

// UI component modules (from oya-zellij)
pub mod correlation;
pub mod health;
pub mod integration_test_worker;
pub mod log;
pub mod metrics;
pub mod state;
pub mod timer;

// Web client is not available in WASM builds (Zellij plugin communicates via IPC)
#[cfg(not(target_arch = "wasm32"))]
pub mod web_client;

// Re-exports for convenience - Zellij plugin
pub use layout::{Layout, Pane, PaneType};
pub use plugin::{OyaPlugin, PluginEvent, PluginInfo, Size};
pub use render::Renderer;

// Re-exports for convenience - UI components
pub use correlation::{CorrelationContext, RequestId, keys};
pub use health::{
    ComponentHealth, HealthError, ResourceUsage, SystemHealthSnapshot, SystemStatus,
    ThroughputMetrics,
};
pub use integration_test_worker::{
    IntegrationTestConfig, IntegrationTestError, IntegrationTestWorker, TestMode, TestResult,
    TestSummary,
};
pub use ipc_zellij::{ZellijIpcClient, ZellijStdin, ZellijStdout};
pub use web_client::{
    ConfigError, HttpResponse, WebClient, WebClientConfig, WebClientError,
};
pub use log::{LogAggregator, LogEntry, LogLevel, LogSource};
pub use metrics::{AgentMetrics, MetricsSnapshot, PoolMetrics};
pub use state::{STATE_VERSION, StateError, StateManager, StateSnapshot};
pub use timer::{RefreshTimer, TimerConfig, TimerError, TimerEvent, TimerState};

// Re-exports for backpressure handling
pub use backpressure::{
    BackpressureConfig, BackpressureReader, BackpressureState, BackpressureWriter,
};
#[cfg(test)]
pub use backpressure::backpressure_pipe;
