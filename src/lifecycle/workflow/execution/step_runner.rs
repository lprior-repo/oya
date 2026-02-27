#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::effects::{run_effect, CommandExecutor, Effect, EffectJournalEntry};
use crate::lifecycle::types::{FailureCategory, LifecycleError};
use tokio::time::{sleep, Duration};

use crate::lifecycle::workflow::execution::details::step_details;
use crate::lifecycle::workflow::execution::transitions::{failed_state, success_acc};
use crate::lifecycle::workflow::progress::{
    compute_duration_ms, make_step_progress_failure, make_step_progress_running,
    make_step_progress_success, timestamp_now,
};
use crate::lifecycle::workflow::steps::LifecycleStep;
use crate::lifecycle::workflow::types::{ExecutionAcc, LifecycleProgressUpdate, StepFailure};

#[cfg(not(test))]
const STAGE_RETRY_BACKOFFS: [Duration; 3] =
    [Duration::from_secs(120), Duration::from_secs(120), Duration::from_secs(120)];

#[cfg(test)]
const STAGE_RETRY_BACKOFFS: [Duration; 3] =
    [Duration::from_millis(0), Duration::from_millis(0), Duration::from_millis(0)];

pub async fn execute_steps<F>(
    executor: &dyn CommandExecutor,
    initial: ExecutionAcc,
    steps: Vec<LifecycleStep>,
    on_progress: &mut F,
) -> Result<ExecutionAcc, Box<StepFailure>>
where
    F: FnMut(LifecycleProgressUpdate),
{
    let mut acc = initial;
    for step in steps {
        let step_name = step.name.clone();
        let started_at = timestamp_now();
        let start_instant = std::time::Instant::now();
        on_progress(make_step_progress_running(&step_name, &started_at));
        let result = execute_step(executor, acc, step).await;
        let finished_at = timestamp_now();
        let duration_ms = compute_duration_ms(&start_instant);
        match result {
            Ok((next, details)) => {
                on_progress(make_step_progress_success(
                    step_name,
                    details,
                    &started_at,
                    &finished_at,
                    duration_ms,
                ));
                acc = next;
            }
            Err(failure) => {
                on_progress(make_step_progress_failure(
                    step_name,
                    failure.error.to_string(),
                    &started_at,
                    &finished_at,
                    duration_ms,
                ));
                return Err(failure);
            }
        }
    }
    Ok(acc)
}

async fn execute_step(
    executor: &dyn CommandExecutor,
    acc: ExecutionAcc,
    step: LifecycleStep,
) -> Result<(ExecutionAcc, Option<serde_json::Value>), Box<StepFailure>> {
    let effect = step.effect.clone();
    match run_effect_with_retries(executor, effect).await {
        Ok(entry) => {
            let details = step_details(&entry);
            let next = success_acc(acc, step, entry)?;
            Ok((next, details))
        }
        Err(error) => Err(Box::new(StepFailure {
            state: failed_state(&acc.state, &error),
            journal: acc.journal,
            completed_compensations: acc.completed_compensations,
            error,
        })),
    }
}

async fn run_effect_with_retries(
    executor: &dyn CommandExecutor,
    effect: Effect,
) -> Result<EffectJournalEntry, LifecycleError> {
    for attempt in 0..=STAGE_RETRY_BACKOFFS.len() {
        if attempt > 0 {
            sleep(STAGE_RETRY_BACKOFFS[attempt - 1]).await;
        }
        match run_effect(executor, effect.clone()).await {
            Ok(entry) => return Ok(entry),
            Err(error) if should_retry_stage(&error) && attempt < STAGE_RETRY_BACKOFFS.len() => {}
            Err(error) if should_retry_stage(&error) => {
                return Err(with_retry_context(error, attempt + 1, STAGE_RETRY_BACKOFFS.len()));
            }
            Err(error) => return Err(error),
        }
    }
    Err(LifecycleError::terminal(
        FailureCategory::Command,
        "unreachable retry state in run_effect_with_retries",
    ))
}

fn should_retry_stage(error: &LifecycleError) -> bool {
    !error.is_terminal()
}

fn with_retry_context(error: LifecycleError, attempts: usize, retries: usize) -> LifecycleError {
    match error {
        LifecycleError::Transient { category, message } => LifecycleError::transient(
            category,
            format!("after {attempts} attempts ({retries} retries): {message}"),
        ),
        LifecycleError::Terminal { category, message } => {
            LifecycleError::terminal(category, message)
        }
    }
}
