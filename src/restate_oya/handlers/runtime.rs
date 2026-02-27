#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

mod progress;
mod store;
mod workspace;

pub use progress::{
    apply_progress_update, default_step_snapshots, initialize_lifecycle_status,
    store_lifecycle_state,
};
pub use store::{
    cleanup_targets_for_key, get_runtime_status, seed_runtime_status, update_runtime_progress,
};
pub use workspace::forget_workspace_for_targets;

#[cfg(test)]
pub use progress::{upsert_step, StepUpdate};
