#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::effects::CommandExecutor;

use super::types::{
    ExecutionAcc, LifecycleProgressUpdate, LifecycleRunFailure, LifecycleRunOutcome,
    LifecycleRunRequest,
};
use crate::lifecycle::transitions::planned_state;
#[cfg(test)]
use crate::lifecycle::{effects::EffectJournalEntry, types::LifecycleError};

mod details;
mod resolve;
mod step_runner;
mod transitions;

#[cfg(test)]
pub fn step_details(entry: &EffectJournalEntry) -> Option<serde_json::Value> {
    details::step_details(entry)
}

#[cfg(test)]
pub fn validate_workspace_changes(stdout: &str) -> Result<(), LifecycleError> {
    transitions::validate_workspace_changes(stdout)
}

#[cfg(test)]
pub fn strip_diff_prefix(line: &str) -> &str {
    transitions::strip_diff_prefix(line)
}

/// Runs the lifecycle workflow without progress callbacks.
///
/// # Errors
///
/// Returns `LifecycleRunFailure` if any step fails or validation fails.
pub async fn run_lifecycle(
    executor: &dyn CommandExecutor,
    request: LifecycleRunRequest,
) -> Result<LifecycleRunOutcome, LifecycleRunFailure> {
    run_lifecycle_with_progress(executor, request, |_| {}).await
}

/// Runs the lifecycle workflow with a progress callback.
///
/// # Errors
///
/// Returns `LifecycleRunFailure` if any step fails or validation fails.
pub async fn run_lifecycle_with_progress<F>(
    executor: &dyn CommandExecutor,
    request: LifecycleRunRequest,
    mut on_progress: F,
) -> Result<LifecycleRunOutcome, LifecycleRunFailure>
where
    F: FnMut(LifecycleProgressUpdate),
{
    let (bead, steps) = resolve::resolve_and_validate(executor, &request).await?;
    let step_names = steps.iter().map(|step| step.name.clone()).collect::<Vec<_>>();
    on_progress(LifecycleProgressUpdate::Initialized {
        bead_id: bead.bead_id.as_str().to_owned(),
        steps: step_names,
    });
    let initial = ExecutionAcc {
        state: planned_state(bead.clone()),
        journal: Vec::new(),
        completed_compensations: Vec::new(),
    };
    let execution = step_runner::execute_steps(executor, initial, steps, &mut on_progress).await;
    match execution {
        Ok(acc) => {
            super::finalize::finalize_success(executor, acc, bead.workspace, &mut on_progress).await
        }
        Err(failure) => {
            super::finalize::finalize_failure(executor, *failure, bead.workspace, &mut on_progress)
                .await
        }
    }
}
