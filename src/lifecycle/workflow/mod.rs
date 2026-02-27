#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

mod dag;
mod execution;
mod finalize;
mod progress;
mod steps;
mod types;

#[cfg(test)]
mod tests;

pub use execution::{run_lifecycle, run_lifecycle_with_progress};
pub use types::{
    LifecycleProgressUpdate, LifecycleRunFailure, LifecycleRunOutcome, LifecycleRunRequest,
    LifecycleStepStatus,
};
