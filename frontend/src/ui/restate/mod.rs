#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Restate UI components
//!
//! Components for displaying Restate invocation details and journal entries

#[cfg(target_arch = "wasm32")]
pub mod deployment_browser;
#[cfg(target_arch = "wasm32")]
pub mod details_panel;
pub mod invocation_actions;
pub mod journal_viewer;
pub mod lifecycle_status_model;
pub mod lifecycle_status_panel;
#[cfg(target_arch = "wasm32")]
pub mod opencode_trace_panel;
#[cfg(target_arch = "wasm32")]
pub mod panel;
#[cfg(target_arch = "wasm32")]
pub mod promise_browser;
pub mod state_browser;

#[cfg(target_arch = "wasm32")]
pub use deployment_browser::DeploymentBrowserPanel;
#[cfg(target_arch = "wasm32")]
pub use details_panel::RestateInvocationDetails;
pub use journal_viewer::RestateJournalViewer;
#[cfg(target_arch = "wasm32")]
pub use lifecycle_status_panel::LifecycleStatusPanel;
#[cfg(target_arch = "wasm32")]
pub use opencode_trace_panel::OpenCodeTracePanel;
#[cfg(target_arch = "wasm32")]
pub use panel::RestateInvocationsPanel;
#[cfg(target_arch = "wasm32")]
pub use promise_browser::PromiseBrowserPanel;
#[cfg(target_arch = "wasm32")]
pub use state_browser::StateBrowserPanel;
