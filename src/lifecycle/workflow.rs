#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::lifecycle::effects::{
    run_compensation, run_effect, CommandExecutor, Compensation, Effect, EffectJournalEntry,
};
use crate::lifecycle::transitions::{apply_event, planned_state, LifecycleEvent};
use crate::lifecycle::types::{
    BeadData, BeadId, FailureCategory, LifecycleError, LifecycleState, Model, PrInfo, PrNumber,
    WorkspaceName,
};
use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleRunRequest {
    pub bead_id: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleStepStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleProgressUpdate {
    Initialized { bead_id: String, steps: Vec<String> },
    Step { step: String, status: LifecycleStepStatus, message: Option<String> },
    Finished { success: bool, pr_url: Option<String>, message: Option<String> },
}

#[derive(Debug, Clone)]
struct LifecycleStep {
    name: String,
    effect: Effect,
    compensation: Option<Compensation>,
    transition: StepTransition,
}

#[derive(Debug, Clone)]
enum StepTransition {
    None,
    Static(LifecycleEvent),
    PullRequestOpened { bead: BeadData },
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

#[derive(Debug, Deserialize)]
struct ReadyIssue {
    id: String,
}

/// Runs lifecycle steps and applies reverse-order compensations on terminal failures.
///
/// # Errors
/// Returns `LifecycleRunFailure` for validation, command, or transition failures.
pub async fn run_lifecycle(
    executor: &dyn CommandExecutor,
    request: LifecycleRunRequest,
) -> Result<LifecycleRunOutcome, LifecycleRunFailure> {
    run_lifecycle_with_progress(executor, request, |_| {}).await
}

/// Runs lifecycle with progress callbacks for live status publishing.
///
/// # Errors
/// Returns `LifecycleRunFailure` for validation, command, or transition failures.
pub async fn run_lifecycle_with_progress<F>(
    executor: &dyn CommandExecutor,
    request: LifecycleRunRequest,
    mut on_progress: F,
) -> Result<LifecycleRunOutcome, LifecycleRunFailure>
where
    F: FnMut(LifecycleProgressUpdate),
{
    let bead =
        resolve_bead_data(executor, &request).await.map_err(|error| LifecycleRunFailure {
            error,
            state: None,
            journal: Vec::new(),
            compensation_journal: Vec::new(),
        })?;
    let steps = build_steps(&bead, request.model);
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
        Ok(acc) => finalize_success(executor, acc, bead.workspace, &mut on_progress).await,
        Err(failure) => {
            finalize_failure(executor, *failure, bead.workspace, &mut on_progress).await
        }
    }
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

fn build_steps(bead: &BeadData, model: Option<String>) -> Vec<LifecycleStep> {
    let chosen_model =
        model.and_then(|value| Model::parse(&value).ok()).unwrap_or_else(Model::default_model);
    vec![
        br_in_progress_step(bead),
        workspace_prepare_step(bead),
        workspace_create_step(bead),
        opencode_step(bead, &chosen_model),
        moon_ci_step(bead),
        git_add_step(bead),
        git_commit_step(bead),
        bookmark_create_step(bead),
        bookmark_push_step(bead),
        pr_create_step(bead),
    ]
}

fn workspace_prepare_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "workspace_prepare".to_owned(),
        effect: Effect::WorkspacePrepare {
            workspace: bead.workspace.clone(),
            path: bead.workspace_path.clone(),
        },
        compensation: None,
        transition: StepTransition::None,
    }
}

fn br_in_progress_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "mark_in_progress".to_owned(),
        effect: Effect::Br {
            args: vec![
                "update".to_owned(),
                bead.bead_id.as_str().to_owned(),
                "--status".to_owned(),
                "in_progress".to_owned(),
            ],
            cwd: None,
        },
        compensation: Some(Compensation::MarkBeadBlocked {
            bead: bead.clone(),
            reason: "lifecycle failed after terminal error".to_owned(),
        }),
        transition: StepTransition::None,
    }
}

fn workspace_create_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "workspace_add".to_owned(),
        effect: Effect::Jj {
            args: vec!["workspace".to_owned(), "add".to_owned(), bead.workspace_path.clone()],
            cwd: None,
        },
        compensation: Some(Compensation::ForgetWorkspace { workspace: bead.workspace.clone() }),
        transition: StepTransition::Static(LifecycleEvent::WorkspacePrepared),
    }
}

fn opencode_step(bead: &BeadData, model: &Model) -> LifecycleStep {
    let prompt = format!(
        "Implement bead {} with functional Rust lifecycle workflow. Run moon run :ci before finishing.",
        bead.bead_id.as_str()
    );
    LifecycleStep {
        name: "opencode".to_owned(),
        effect: Effect::Opencode {
            prompt,
            model: model.as_str().to_owned(),
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
    }
}

fn moon_ci_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "moon_ci".to_owned(),
        effect: Effect::MoonCi { cwd: Some(bead.workspace_path.clone()) },
        compensation: None,
        transition: StepTransition::None,
    }
}

fn git_add_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "git_add".to_owned(),
        effect: Effect::Git {
            args: vec!["add".to_owned(), ".".to_owned()],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
    }
}

fn git_commit_step(bead: &BeadData) -> LifecycleStep {
    let message = format!("chore: implement {} via lifecycle", bead.bead_id.as_str());
    LifecycleStep {
        name: "git_commit".to_owned(),
        effect: Effect::Git {
            args: vec!["commit".to_owned(), "-m".to_owned(), message],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
    }
}

fn bookmark_create_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "bookmark_create".to_owned(),
        effect: Effect::Jj {
            args: vec![
                "bookmark".to_owned(),
                "create".to_owned(),
                bead.bookmark.as_str().to_owned(),
            ],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
    }
}

fn bookmark_push_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "bookmark_push".to_owned(),
        effect: Effect::Git {
            args: vec![
                "push".to_owned(),
                "--set-upstream".to_owned(),
                "origin".to_owned(),
                bead.bookmark.as_str().to_owned(),
            ],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
    }
}

fn pr_create_step(bead: &BeadData) -> LifecycleStep {
    let title = format!("Lifecycle {}", bead.bead_id.as_str());
    let body = format!(
        "## Summary\n- Implements bead `{}` via lifecycle automation\n- Runs `moon run :ci` in workspace before opening PR\n- Publishes lifecycle status updates for polling",
        bead.bead_id.as_str()
    );
    LifecycleStep {
        name: "pr_create".to_owned(),
        effect: Effect::Gh {
            args: vec![
                "pr".to_owned(),
                "create".to_owned(),
                "--title".to_owned(),
                title,
                "--body".to_owned(),
                body,
            ],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::PullRequestOpened { bead: bead.clone() },
    }
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
        on_progress(LifecycleProgressUpdate::Step {
            step: step.name.clone(),
            status: LifecycleStepStatus::Running,
            message: None,
        });
        acc = execute_step(executor, acc, step, on_progress).await?;
    }
    Ok(acc)
}

async fn execute_step<F>(
    executor: &dyn CommandExecutor,
    acc: ExecutionAcc,
    step: LifecycleStep,
    on_progress: &mut F,
) -> Result<ExecutionAcc, Box<StepFailure>>
where
    F: FnMut(LifecycleProgressUpdate),
{
    let effect = step.effect.clone();
    let step_name = step.name.clone();
    match run_effect(executor, effect).await {
        Ok(entry) => {
            let next = success_acc(acc, step, entry, on_progress)?;
            on_progress(LifecycleProgressUpdate::Step {
                step: step_name,
                status: LifecycleStepStatus::Succeeded,
                message: None,
            });
            Ok(next)
        }
        Err(error) => {
            on_progress(LifecycleProgressUpdate::Step {
                step: step_name,
                status: LifecycleStepStatus::Failed,
                message: Some(error.to_string()),
            });
            Err(Box::new(StepFailure {
                state: failed_state(&acc.state, &error),
                journal: acc.journal,
                completed_compensations: acc.completed_compensations,
                error,
            }))
        }
    }
}

fn success_acc<F>(
    acc: ExecutionAcc,
    step: LifecycleStep,
    entry: EffectJournalEntry,
    on_progress: &mut F,
) -> Result<ExecutionAcc, Box<StepFailure>>
where
    F: FnMut(LifecycleProgressUpdate),
{
    let prev_state = acc.state;
    let prev_journal = acc.journal;
    let prev_compensations = acc.completed_compensations;
    let new_state = apply_transition(&prev_state, &step.transition, &entry);
    let state = new_state.map_err(|error| {
        on_progress(LifecycleProgressUpdate::Step {
            step: step.name.clone(),
            status: LifecycleStepStatus::Failed,
            message: Some(error.to_string()),
        });
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
    state: &LifecycleState,
    transition: &StepTransition,
    entry: &EffectJournalEntry,
) -> Result<LifecycleState, LifecycleError> {
    let event = match transition {
        StepTransition::None => return Ok(state.clone()),
        StepTransition::Static(event) => event.clone(),
        StepTransition::PullRequestOpened { bead } => {
            LifecycleEvent::PullRequestOpened(parse_pr_info(bead, &entry.stdout)?)
        }
    };
    apply_event(state, event)
}

fn parse_pr_info(bead: &BeadData, stdout: &str) -> Result<PrInfo, LifecycleError> {
    let url = extract_pr_url(stdout).ok_or_else(|| {
        LifecycleError::terminal(
            FailureCategory::PullRequest,
            "gh pr create output did not include PR URL",
        )
    })?;
    let pr_number = parse_pr_number(&url)?;
    Ok(PrInfo { number: pr_number, bookmark: bead.bookmark.clone(), url })
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

fn parse_pr_number(url: &str) -> Result<PrNumber, LifecycleError> {
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

fn failed_state(state: &LifecycleState, error: &LifecycleError) -> LifecycleState {
    match apply_event(state, LifecycleEvent::Failed(error.clone())) {
        Ok(next) => next,
        Err(_) => state.clone(),
    }
}

async fn finalize_success<F>(
    executor: &dyn CommandExecutor,
    mut acc: ExecutionAcc,
    workspace: WorkspaceName,
    on_progress: &mut F,
) -> Result<LifecycleRunOutcome, LifecycleRunFailure>
where
    F: FnMut(LifecycleProgressUpdate),
{
    let completed_state = apply_event(&acc.state, LifecycleEvent::Completed).map_err(|error| {
        LifecycleRunFailure {
            error,
            state: Some(acc.state.clone()),
            journal: acc.journal.clone(),
            compensation_journal: Vec::new(),
        }
    })?;
    acc.state = completed_state;
    let cleanup = workspace_cleanup(executor, workspace).await;
    let pr_url = pr_url_from_state(&acc.state);
    on_progress(LifecycleProgressUpdate::Finished { success: true, pr_url, message: None });
    Ok(LifecycleRunOutcome {
        state: acc.state,
        journal: acc.journal,
        compensation_journal: cleanup,
    })
}

async fn finalize_failure<F>(
    executor: &dyn CommandExecutor,
    failure: StepFailure,
    workspace: WorkspaceName,
    on_progress: &mut F,
) -> Result<LifecycleRunOutcome, LifecycleRunFailure>
where
    F: FnMut(LifecycleProgressUpdate),
{
    let mut compensation_journal = if failure.error.is_terminal() {
        run_compensations(executor, failure.completed_compensations).await
    } else {
        Vec::new()
    };
    let cleanup = workspace_cleanup(executor, workspace).await;
    compensation_journal = compensation_journal.into_iter().chain(cleanup).collect();
    on_progress(LifecycleProgressUpdate::Finished {
        success: false,
        pr_url: pr_url_from_state(&failure.state),
        message: Some(failure.error.to_string()),
    });
    Err(LifecycleRunFailure {
        error: failure.error,
        state: Some(failure.state),
        journal: failure.journal,
        compensation_journal,
    })
}

fn pr_url_from_state(state: &LifecycleState) -> Option<String> {
    match &state.phase {
        crate::lifecycle::types::Phase::PrOpen { pr, .. } => Some(pr.url.clone()),
        crate::lifecycle::types::Phase::Completed(result) => {
            result.pr.as_ref().map(|pr| pr.url.clone())
        }
        crate::lifecycle::types::Phase::Planned(_)
        | crate::lifecycle::types::Phase::WorkspaceReady(_)
        | crate::lifecycle::types::Phase::Failed { .. } => None,
    }
}

async fn workspace_cleanup(
    executor: &dyn CommandExecutor,
    workspace: WorkspaceName,
) -> Vec<EffectJournalEntry> {
    run_compensation(executor, Compensation::ForgetWorkspace { workspace })
        .await
        .ok()
        .into_iter()
        .collect()
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
