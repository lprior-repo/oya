#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::effects::{run_effect, CommandExecutor, Effect, EffectJournalEntry};
use crate::lifecycle::telemetry::{emit_step_telemetry, emit_unwind_signal};
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

use std::collections::HashSet;

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
    let mut succeeded_steps: HashSet<String> = HashSet::new();
    let implementation_step = steps.iter().find(|step| step.name == IMPL_STEP_NAME).cloned();
    for step in steps {
        let step_name = step.name.clone();
        validate_dependencies(&step, &succeeded_steps, &acc)?;
        let is_qa_step = step.name == QA_STEP_NAME;
        let qa_step = if is_qa_step { Some(step.clone()) } else { None };
        let result = run_step_with_telemetry(executor, acc, step, on_progress).await;
        match result {
            Ok(next) => {
                succeeded_steps.insert(step_name);
                acc = next;
            }
            Err(failure) => {
                if is_qa_step {
                    if let (Some(implementation), Some(qa)) =
                        (implementation_step.clone(), qa_step.clone())
                    {
                        let recovered =
                            retry_qa_loop(executor, failure, implementation, qa, on_progress)
                                .await?;
                        succeeded_steps.insert(step_name);
                        acc = recovered;
                    } else {
                        return Err(failure);
                    }
                } else {
                    return Err(failure);
                }
            }
        }
    }
    Ok(acc)
}

fn validate_dependencies(
    step: &LifecycleStep,
    succeeded_steps: &HashSet<String>,
    acc: &ExecutionAcc,
) -> Result<(), Box<StepFailure>> {
    for dep in &step.dependencies {
        if !succeeded_steps.contains(dep) {
            return Err(Box::new(StepFailure {
                state: acc.state.clone(),
                journal: acc.journal.clone(),
                completed_compensations: acc.completed_compensations.clone(),
                error: LifecycleError::terminal(
                    FailureCategory::Validation,
                    format!(
                        "step `{}` cannot execute: dependency `{}` has not succeeded",
                        step.name, dep
                    ),
                ),
            }));
        }
    }
    Ok(())
}

const IMPL_STEP_NAME: &str = "opencode";
const QA_STEP_NAME: &str = "qa_enforcer";
const QA_MAX_RETRIES: usize = 3;

async fn retry_qa_loop<F>(
    executor: &dyn CommandExecutor,
    failure: Box<StepFailure>,
    implementation_step: LifecycleStep,
    qa_step: LifecycleStep,
    on_progress: &mut F,
) -> Result<ExecutionAcc, Box<StepFailure>>
where
    F: FnMut(LifecycleProgressUpdate),
{
    let mut current_failure = failure;
    for _ in 0..QA_MAX_RETRIES {
        let StepFailure { state, journal, completed_compensations, .. } = *current_failure;
        let acc = ExecutionAcc { state, journal, completed_compensations };
        let after_impl =
            run_step_with_telemetry(executor, acc, implementation_step.clone(), on_progress)
                .await?;
        match run_step_with_telemetry(executor, after_impl, qa_step.clone(), on_progress).await {
            Ok(next) => return Ok(next),
            Err(next_failure) => current_failure = next_failure,
        }
    }
    Err(current_failure)
}

async fn run_step_with_telemetry<F>(
    executor: &dyn CommandExecutor,
    acc: ExecutionAcc,
    step: LifecycleStep,
    on_progress: &mut F,
) -> Result<ExecutionAcc, Box<StepFailure>>
where
    F: FnMut(LifecycleProgressUpdate),
{
    let step_name = step.name.clone();
    let started_at = timestamp_now();
    let start_instant = std::time::Instant::now();
    let running_progress = make_step_progress_running(&step_name, &started_at);
    on_progress(running_progress.clone());
    emit_step_telemetry(&running_progress);
    let result = execute_step(executor, acc, step).await;
    let finished_at = timestamp_now();
    let duration_ms = compute_duration_ms(&start_instant);
    match result {
        Ok((next, details)) => {
            let success_progress = make_step_progress_success(
                step_name,
                details,
                &started_at,
                &finished_at,
                duration_ms,
            );
            on_progress(success_progress.clone());
            emit_step_telemetry(&success_progress);
            Ok(next)
        }
        Err(failure) => {
            let timing = StepTiming { step_name, started_at, finished_at, duration_ms };
            handle_step_failure(on_progress, &timing, &failure);
            Err(failure)
        }
    }
}

struct StepTiming {
    step_name: String,
    started_at: String,
    finished_at: String,
    duration_ms: u64,
}

fn handle_step_failure<F>(on_progress: &mut F, timing: &StepTiming, failure: &StepFailure)
where
    F: FnMut(LifecycleProgressUpdate),
{
    let failure_progress = make_step_progress_failure(
        timing.step_name.clone(),
        failure.error.to_string(),
        &timing.started_at,
        &timing.finished_at,
        timing.duration_ms,
    );
    on_progress(failure_progress.clone());
    emit_step_telemetry(&failure_progress);
    emit_pending_compensation_signals(&failure.completed_compensations);
}

fn emit_pending_compensation_signals(compensations: &[crate::lifecycle::effects::Compensation]) {
    for compensation in compensations {
        if let crate::lifecycle::effects::Compensation::MarkBeadBlocked { bead, reason } =
            compensation
        {
            let diagnostic = crate::lifecycle::types::CompensationDiagnostic {
                compensation_type: "mark_bead_blocked".to_owned(),
                target: bead.bead_id.as_str().to_owned(),
                success: false,
                error: Some(reason.clone()),
            };
            emit_unwind_signal(&diagnostic);
        }
    }
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
