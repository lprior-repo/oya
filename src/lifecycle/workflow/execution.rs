#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::effects::{run_effect, CommandExecutor, Effect, EffectJournalEntry};
use crate::lifecycle::transitions::{apply_event, LifecycleEvent};
use crate::lifecycle::types::{BeadData, BeadId, FailureCategory, LifecycleError, Model, RepoSlug};

use super::dag::validate_dag;
use super::progress::{
    compute_duration_ms, make_step_progress_failure, make_step_progress_running,
    make_step_progress_success, timestamp_now,
};
use super::steps::{build_steps, LifecycleStep, StepTransition};
use super::types::{
    ExecutionAcc, LifecycleProgressUpdate, LifecycleRunFailure, LifecycleRunOutcome,
    LifecycleRunRequest, StepFailure,
};
use crate::lifecycle::transitions::planned_state;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::time::{sleep, Duration};

#[cfg(not(test))]
const STAGE_RETRY_BACKOFFS: [Duration; 3] =
    [Duration::from_secs(120), Duration::from_secs(120), Duration::from_secs(120)];

#[cfg(test)]
const STAGE_RETRY_BACKOFFS: [Duration; 3] =
    [Duration::from_millis(0), Duration::from_millis(0), Duration::from_millis(0)];

#[derive(Debug, Deserialize)]
struct ReadyIssue {
    id: String,
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
    let (bead, steps) = resolve_and_validate(executor, &request).await?;
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
    let execution = execute_steps(executor, initial, steps, &mut on_progress).await;
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

async fn resolve_and_validate(
    executor: &dyn CommandExecutor,
    request: &LifecycleRunRequest,
) -> Result<(BeadData, Vec<LifecycleStep>), LifecycleRunFailure> {
    let bead = resolve_bead_data(executor, request).await.map_err(|error| LifecycleRunFailure {
        error,
        state: None,
        journal: Vec::new(),
        compensation_journal: Vec::new(),
        compensation_diagnostics: Vec::new(),
    })?;
    let model = resolve_model(request.model.as_deref()).map_err(|error| LifecycleRunFailure {
        error,
        state: None,
        journal: Vec::new(),
        compensation_journal: Vec::new(),
        compensation_diagnostics: Vec::new(),
    })?;
    let repo =
        validate_repo_slug(request.repo.as_deref()).map_err(|error| LifecycleRunFailure {
            error,
            state: None,
            journal: Vec::new(),
            compensation_journal: Vec::new(),
            compensation_diagnostics: Vec::new(),
        })?;
    let steps = build_steps(&bead, &model, repo.as_deref());
    validate_dag(&steps).map_err(|error| LifecycleRunFailure {
        error,
        state: None,
        journal: Vec::new(),
        compensation_journal: Vec::new(),
        compensation_diagnostics: Vec::new(),
    })?;
    Ok((bead, steps))
}

async fn resolve_bead_data(
    executor: &dyn CommandExecutor,
    request: &LifecycleRunRequest,
) -> Result<BeadData, LifecycleError> {
    let selected = match &request.bead_id {
        Some(bead_id) => bead_id.clone(),
        None => pick_ready_bead(executor).await?,
    };
    BeadId::parse(&selected)
        .map(BeadData::from_bead_id)
        .map_err(|error| LifecycleError::terminal(FailureCategory::Validation, error.to_string()))
}

async fn pick_ready_bead(executor: &dyn CommandExecutor) -> Result<String, LifecycleError> {
    let entry = run_effect(
        executor,
        Effect::Br { args: vec!["ready".to_owned(), "--json".to_owned()], cwd: None },
    )
    .await?;
    let json = extract_json_array(&entry.stdout)?;
    let issues = serde_json::from_str::<Vec<ReadyIssue>>(json).map_err(|error| {
        LifecycleError::terminal(
            FailureCategory::Validation,
            format!("failed to parse br ready payload: {error}"),
        )
    })?;
    issues.first().map_or_else(
        || Err(LifecycleError::terminal(FailureCategory::Validation, "no ready beads found")),
        |issue| Ok(issue.id.clone()),
    )
}

fn extract_json_array(raw: &str) -> Result<&str, LifecycleError> {
    raw.find('[').map_or_else(
        || {
            Err(LifecycleError::terminal(
                FailureCategory::Validation,
                "br ready --json returned no JSON payload",
            ))
        },
        |index| Ok(&raw[index..]),
    )
}

fn resolve_model(model: Option<&str>) -> Result<Model, LifecycleError> {
    match model {
        Some(value) => Model::parse(value).map_err(|error| {
            LifecycleError::terminal(
                FailureCategory::Validation,
                format!("invalid model `{value}`: {error}"),
            )
        }),
        None => Ok(Model::default_model()),
    }
}

fn validate_repo_slug(repo: Option<&str>) -> Result<Option<String>, LifecycleError> {
    repo.map_or(Ok(None), |value| {
        RepoSlug::parse(value).map(|slug| Some(slug.as_str().to_owned())).map_err(|error| {
            LifecycleError::terminal(
                FailureCategory::Validation,
                format!("invalid repo slug `{value}`: {error}"),
            )
        })
    })
}

async fn execute_steps<F>(
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
) -> Result<(ExecutionAcc, Option<Value>), Box<StepFailure>> {
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

pub fn step_details(entry: &EffectJournalEntry) -> Option<Value> {
    match &entry.effect {
        Effect::Opencode { .. } => Some(json!({
            "events": parse_json_lines(&entry.stdout),
            "stderr": entry.stderr,
        })),
        _ => None,
    }
}

fn parse_json_lines(raw: &str) -> Vec<Value> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
}

fn success_acc(
    acc: ExecutionAcc,
    step: LifecycleStep,
    entry: EffectJournalEntry,
) -> Result<ExecutionAcc, Box<StepFailure>> {
    let prev_state = acc.state;
    let prev_journal = acc.journal;
    let prev_compensations = acc.completed_compensations;
    let new_state = apply_transition(&prev_state, &step.transition, &entry);
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

fn apply_transition(
    state: &crate::lifecycle::types::LifecycleState,
    transition: &StepTransition,
    entry: &EffectJournalEntry,
) -> Result<crate::lifecycle::types::LifecycleState, LifecycleError> {
    let event = match transition {
        StepTransition::None => return Ok(state.clone()),
        StepTransition::Static(event) => event.clone(),
        StepTransition::ValidateWorkspaceChanges => {
            validate_workspace_changes(&entry.stdout)?;
            return Ok(state.clone());
        }
        StepTransition::PullRequestOpened { bead } => {
            LifecycleEvent::PullRequestOpened(parse_pr_info(bead, &entry.stdout)?)
        }
    };
    apply_event(state, event)
}

pub fn validate_workspace_changes(stdout: &str) -> Result<(), LifecycleError> {
    let files = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(strip_diff_prefix)
        .filter(|line| !line.starts_with(".beads/"))
        .collect::<Vec<_>>();
    if files.is_empty() {
        Err(LifecycleError::terminal(
            FailureCategory::Command,
            "no non-.beads changes detected after opencode; refusing to open empty PR".to_owned(),
        ))
    } else {
        Ok(())
    }
}

pub fn strip_diff_prefix(line: &str) -> &str {
    line.strip_prefix("M ")
        .or_else(|| line.strip_prefix("A "))
        .or_else(|| line.strip_prefix("R "))
        .or_else(|| line.strip_prefix("D "))
        .unwrap_or(line)
}

fn parse_pr_info(
    bead: &BeadData,
    stdout: &str,
) -> Result<crate::lifecycle::types::PrInfo, LifecycleError> {
    let url = extract_pr_url(stdout).ok_or_else(|| {
        LifecycleError::terminal(
            FailureCategory::PullRequest,
            "gh pr create output did not include PR URL",
        )
    })?;
    let pr_number = parse_pr_number(&url)?;
    Ok(crate::lifecycle::types::PrInfo { number: pr_number, bookmark: bead.bookmark.clone(), url })
}

fn extract_pr_url(stdout: &str) -> Option<String> {
    stdout
        .split_whitespace()
        .map(trim_trailing_punctuation)
        .find(|token| token.starts_with("https://") && token.contains("/pull/"))
        .map(std::borrow::ToOwned::to_owned)
}

fn trim_trailing_punctuation(token: &str) -> &str {
    token.trim_end_matches([')', ']', '.', ',', ';'])
}

fn parse_pr_number(url: &str) -> Result<crate::lifecycle::types::PrNumber, LifecycleError> {
    use crate::lifecycle::types::PrNumber;
    let value = url
        .rsplit('/')
        .next()
        .ok_or_else(|| LifecycleError::terminal(FailureCategory::PullRequest, "missing PR number"))
        .and_then(|segment| {
            segment.parse::<u64>().map_err(|error| {
                LifecycleError::terminal(
                    FailureCategory::PullRequest,
                    format!("invalid PR number in URL `{url}`: {error}"),
                )
            })
        })?;
    PrNumber::new(value).map_err(|error| {
        LifecycleError::terminal(
            FailureCategory::PullRequest,
            format!("invalid PR number in URL `{url}`: {error}"),
        )
    })
}

fn failed_state(
    state: &crate::lifecycle::types::LifecycleState,
    error: &LifecycleError,
) -> crate::lifecycle::types::LifecycleState {
    match apply_event(state, LifecycleEvent::Failed(error.clone())) {
        Ok(next) => next,
        Err(_) => state.clone(),
    }
}

fn append_entry(
    journal: Vec<EffectJournalEntry>,
    entry: EffectJournalEntry,
) -> Vec<EffectJournalEntry> {
    let mut journal = journal;
    journal.push(entry);
    journal
}

fn append_compensation(
    compensations: Vec<crate::lifecycle::effects::Compensation>,
    compensation: crate::lifecycle::effects::Compensation,
) -> Vec<crate::lifecycle::effects::Compensation> {
    let mut compensations = compensations;
    compensations.push(compensation);
    compensations
}
