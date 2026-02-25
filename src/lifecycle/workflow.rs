#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::lifecycle::effects::{
    run_compensation, run_effect, CommandExecutor, Compensation, Effect, EffectJournalEntry,
};
use crate::lifecycle::transitions::{apply_event, planned_state, LifecycleEvent};
use crate::lifecycle::types::{BeadData, BeadId, FailureCategory, LifecycleError, LifecycleState};
use futures_util::stream::{self, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};

const DEFAULT_MODEL: &str = "openai/gpt-5.3-codex";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleRunRequest {
    pub bead_id: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleRunOutcome {
    pub state: LifecycleState,
    pub journal: Vec<EffectJournalEntry>,
    pub compensation_journal: Vec<EffectJournalEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleRunFailure {
    pub error: LifecycleError,
    pub state: Option<LifecycleState>,
    pub journal: Vec<EffectJournalEntry>,
    pub compensation_journal: Vec<EffectJournalEntry>,
}

#[derive(Debug, Clone)]
struct LifecycleStep {
    effect: Effect,
    compensation: Option<Compensation>,
    success_event: Option<LifecycleEvent>,
}

#[derive(Debug, Clone)]
struct ExecutionAcc {
    state: LifecycleState,
    journal: Vec<EffectJournalEntry>,
    completed_compensations: Vec<Compensation>,
}

#[derive(Debug, Clone)]
struct StepFailure {
    state: LifecycleState,
    journal: Vec<EffectJournalEntry>,
    completed_compensations: Vec<Compensation>,
    error: LifecycleError,
}

/// Runs lifecycle steps and applies reverse-order compensations on terminal failures.
///
/// # Errors
/// Returns `LifecycleRunFailure` for validation, command, or transition failures.
pub async fn run_lifecycle(
    executor: &dyn CommandExecutor,
    request: LifecycleRunRequest,
) -> Result<LifecycleRunOutcome, LifecycleRunFailure> {
    let bead = parse_bead_data(&request).map_err(|error| LifecycleRunFailure {
        error,
        state: None,
        journal: Vec::new(),
        compensation_journal: Vec::new(),
    })?;
    let steps = build_steps(&bead, request.model);
    let initial = ExecutionAcc {
        state: planned_state(bead),
        journal: Vec::new(),
        completed_compensations: Vec::new(),
    };

    let execution = stream::iter(steps.into_iter().map(Ok::<LifecycleStep, Box<StepFailure>>))
        .try_fold(initial, |acc, step| execute_step(executor, acc, step))
        .await;

    match execution {
        Ok(acc) => Ok(LifecycleRunOutcome {
            state: acc.state,
            journal: acc.journal,
            compensation_journal: Vec::new(),
        }),
        Err(failure) => {
            let failure = *failure;
            let compensation_journal = if failure.error.is_terminal() {
                run_compensations(executor, failure.completed_compensations).await
            } else {
                Vec::new()
            };
            Err(LifecycleRunFailure {
                error: failure.error,
                state: Some(failure.state),
                journal: failure.journal,
                compensation_journal,
            })
        }
    }
}

fn parse_bead_data(request: &LifecycleRunRequest) -> Result<BeadData, LifecycleError> {
    BeadId::parse(&request.bead_id)
        .map(BeadData::from_bead_id)
        .map_err(|error| LifecycleError::terminal(FailureCategory::Validation, error.to_string()))
}

fn build_steps(bead: &BeadData, model: Option<String>) -> Vec<LifecycleStep> {
    let chosen_model = model.map_or_else(|| DEFAULT_MODEL.to_owned(), std::convert::identity);
    vec![
        br_in_progress_step(bead),
        workspace_create_step(bead),
        LifecycleStep { effect: Effect::MoonCi, compensation: None, success_event: None },
        opencode_step(bead, chosen_model),
        pr_create_step(bead),
    ]
}

fn br_in_progress_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        effect: Effect::Br {
            args: vec![
                "update".to_owned(),
                bead.bead_id.as_str().to_owned(),
                "--status".to_owned(),
                "in_progress".to_owned(),
            ],
        },
        compensation: Some(Compensation::MarkBeadBlocked {
            bead: bead.clone(),
            reason: "lifecycle failed after terminal error".to_owned(),
        }),
        success_event: None,
    }
}

fn workspace_create_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        effect: Effect::Jj {
            args: vec![
                "workspace".to_owned(),
                "add".to_owned(),
                bead.workspace.as_str().to_owned(),
            ],
        },
        compensation: Some(Compensation::ForgetWorkspace { workspace: bead.workspace.clone() }),
        success_event: Some(LifecycleEvent::WorkspacePrepared),
    }
}

fn opencode_step(bead: &BeadData, model: String) -> LifecycleStep {
    let prompt = format!(
        "Implement bead {} with functional Rust lifecycle workflow. Run moon run :ci before finishing.",
        bead.bead_id.as_str()
    );
    LifecycleStep {
        effect: Effect::Opencode { prompt, model },
        compensation: None,
        success_event: None,
    }
}

fn pr_create_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        effect: Effect::Gh {
            args: vec![
                "pr".to_owned(),
                "create".to_owned(),
                "--title".to_owned(),
                format!("Lifecycle {}", bead.bead_id.as_str()),
            ],
        },
        compensation: None,
        success_event: Some(LifecycleEvent::Completed),
    }
}

async fn execute_step(
    executor: &dyn CommandExecutor,
    acc: ExecutionAcc,
    step: LifecycleStep,
) -> Result<ExecutionAcc, Box<StepFailure>> {
    let effect = step.effect.clone();
    match run_effect(executor, effect).await {
        Ok(entry) => success_acc(acc, step, entry),
        Err(error) => Err(Box::new(StepFailure {
            state: failed_state(&acc.state, &error),
            journal: acc.journal,
            completed_compensations: acc.completed_compensations,
            error,
        })),
    }
}

fn success_acc(
    acc: ExecutionAcc,
    step: LifecycleStep,
    entry: EffectJournalEntry,
) -> Result<ExecutionAcc, Box<StepFailure>> {
    let prev_state = acc.state;
    let prev_journal = acc.journal;
    let prev_compensations = acc.completed_compensations;
    let new_state = step
        .success_event
        .map_or_else(|| Ok(prev_state.clone()), |event| apply_event(&prev_state, event));
    let state = new_state.map_err(|error| {
        Box::new(StepFailure {
            state: prev_state.clone(),
            journal: append_entry(prev_journal.clone(), entry.clone()),
            completed_compensations: prev_compensations.clone(),
            error,
        })
    })?;
    let completed_compensations = step.compensation.map_or_else(
        || prev_compensations.clone(),
        |item| append_compensation(prev_compensations.clone(), item),
    );

    Ok(ExecutionAcc { state, journal: append_entry(prev_journal, entry), completed_compensations })
}

fn failed_state(state: &LifecycleState, error: &LifecycleError) -> LifecycleState {
    match apply_event(state, LifecycleEvent::Failed(error.clone())) {
        Ok(next) => next,
        Err(_) => state.clone(),
    }
}

async fn run_compensations(
    executor: &dyn CommandExecutor,
    compensations: Vec<Compensation>,
) -> Vec<EffectJournalEntry> {
    let reversed = compensations.into_iter().rev().collect::<Vec<_>>();
    let attempts = stream::iter(reversed.into_iter())
        .then(|compensation| async move { run_compensation(executor, compensation).await })
        .collect::<Vec<anyhow::Result<EffectJournalEntry>>>()
        .await;
    attempts.into_iter().filter_map(Result::ok).collect()
}

fn append_entry(
    entries: Vec<EffectJournalEntry>,
    entry: EffectJournalEntry,
) -> Vec<EffectJournalEntry> {
    entries.into_iter().chain(std::iter::once(entry)).collect()
}

fn append_compensation(
    compensations: Vec<Compensation>,
    compensation: Compensation,
) -> Vec<Compensation> {
    compensations.into_iter().chain(std::iter::once(compensation)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::effects::{CommandFailure, CommandResult};
    use async_trait::async_trait;
    use std::time::Duration;

    #[derive(Debug)]
    struct FakeExecutor;

    #[async_trait]
    impl CommandExecutor for FakeExecutor {
        async fn run(
            &self,
            program: &str,
            _args: &[String],
            _timeout: Duration,
        ) -> Result<CommandResult, CommandFailure> {
            let status_code = if program == "gh" { Some(1) } else { Some(0) };
            Ok(CommandResult { status_code, stdout: String::new(), stderr: String::new() })
        }
    }

    #[tokio::test]
    async fn terminal_error_runs_compensation_in_reverse_order() {
        let request =
            LifecycleRunRequest { bead_id: "src-31nq".to_owned(), model: Some("model".to_owned()) };
        let result = run_lifecycle(&FakeExecutor, request).await;
        assert!(result.is_err());

        if let Err(failure) = result {
            let first = failure.compensation_journal.first();
            let second = failure.compensation_journal.get(1);
            assert!(matches!(first, Some(EffectJournalEntry { effect: Effect::Jj { .. }, .. })));
            assert!(matches!(second, Some(EffectJournalEntry { effect: Effect::Br { .. }, .. })));
        }
    }
}
