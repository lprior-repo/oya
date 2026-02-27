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
    BeadData, BeadId, CompensationDiagnostic, FailureCategory, LifecycleError, LifecycleState,
    Model, PrInfo, PrNumber, RepoSlug, WorkspaceName,
};
use chrono::{SecondsFormat, Utc};
use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleRunRequest {
    pub bead_id: Option<String>,
    pub model: Option<String>,
    pub repo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleRunOutcome {
    pub state: LifecycleState,
    pub journal: Vec<EffectJournalEntry>,
    pub compensation_journal: Vec<EffectJournalEntry>,
    pub compensation_diagnostics: Vec<CompensationDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleRunFailure {
    pub error: LifecycleError,
    pub state: Option<LifecycleState>,
    pub journal: Vec<EffectJournalEntry>,
    pub compensation_journal: Vec<EffectJournalEntry>,
    pub compensation_diagnostics: Vec<CompensationDiagnostic>,
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
    Initialized {
        bead_id: String,
        steps: Vec<String>,
    },
    Step {
        step: String,
        status: LifecycleStepStatus,
        message: Option<String>,
        details: Option<Value>,
        started_at: Option<String>,
        finished_at: Option<String>,
        duration_ms: Option<u64>,
    },
    Finished {
        success: bool,
        pr_url: Option<String>,
        message: Option<String>,
        compensation_diagnostics: Vec<CompensationDiagnostic>,
    },
}

#[derive(Debug, Clone)]
pub struct LifecycleStep {
    pub name: String,
    pub effect: Effect,
    pub compensation: Option<Compensation>,
    pub transition: StepTransition,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum StepTransition {
    None,
    Static(LifecycleEvent),
    ValidateWorkspaceChanges,
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

/// Validates the lifecycle step graph for cycles and missing dependencies.
///
/// # Errors
/// Returns `LifecycleError` if the graph contains cycles or references unknown steps.
pub fn validate_dag(steps: &[LifecycleStep]) -> Result<(), LifecycleError> {
    let step_names: std::collections::HashSet<&str> =
        steps.iter().map(|step| step.name.as_str()).collect();
    for step in steps {
        for dep in &step.dependencies {
            if !step_names.contains(dep.as_str()) {
                return Err(LifecycleError::terminal(
                    FailureCategory::Validation,
                    format!("step `{}` has unknown dependency `{}`", step.name, dep),
                ));
            }
        }
    }
    detect_cycles(steps)?;
    validate_dependency_order(steps)
}

fn validate_dependency_order(steps: &[LifecycleStep]) -> Result<(), LifecycleError> {
    let mut seen = std::collections::HashSet::new();
    for step in steps {
        for dep in &step.dependencies {
            if !seen.contains(dep.as_str()) {
                return Err(LifecycleError::terminal(
                    FailureCategory::Validation,
                    format!("step `{}` depends on `{}` which appears later", step.name, dep),
                ));
            }
        }
        seen.insert(step.name.as_str());
    }
    Ok(())
}

fn detect_cycles(steps: &[LifecycleStep]) -> Result<(), LifecycleError> {
    let step_map: std::collections::HashMap<&str, &LifecycleStep> =
        steps.iter().map(|step| (step.name.as_str(), step)).collect();
    let mut visited = std::collections::HashSet::<&str>::new();
    let mut recursion_stack = std::collections::HashSet::<&str>::new();
    for step in steps {
        if !visited.contains(step.name.as_str())
            && has_cycle(step.name.as_str(), &step_map, &mut visited, &mut recursion_stack)?
        {
            return Err(LifecycleError::terminal(
                FailureCategory::Validation,
                format!("cycle detected in lifecycle step graph involving `{}`", step.name),
            ));
        }
    }
    Ok(())
}

fn has_cycle<'a>(
    step_name: &'a str,
    step_map: &std::collections::HashMap<&'a str, &'a LifecycleStep>,
    visited: &mut std::collections::HashSet<&'a str>,
    recursion_stack: &mut std::collections::HashSet<&'a str>,
) -> Result<bool, LifecycleError> {
    visited.insert(step_name);
    recursion_stack.insert(step_name);
    let step = step_map.get(step_name).ok_or_else(|| {
        LifecycleError::terminal(
            FailureCategory::Validation,
            format!("internal error: step `{step_name}` not found in map"),
        )
    })?;
    for dep in &step.dependencies {
        let dep_name = dep.as_str();
        if !visited.contains(dep_name) {
            if has_cycle(dep_name, step_map, visited, recursion_stack)? {
                return Ok(true);
            }
        } else if recursion_stack.contains(dep_name) {
            return Ok(true);
        }
    }
    recursion_stack.remove(step_name);
    Ok(false)
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
        Ok(acc) => finalize_success(executor, acc, bead.workspace, &mut on_progress).await,
        Err(failure) => {
            finalize_failure(executor, *failure, bead.workspace, &mut on_progress).await
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

fn build_steps(bead: &BeadData, model: &Model, repo: Option<&str>) -> Vec<LifecycleStep> {
    let mut steps = vec![
        br_in_progress_step(bead),
        workspace_prepare_step(bead),
        workspace_create_step(bead),
        opencode_step(bead, model),
        moon_ci_step(bead),
        jj_sync_main_step(bead),
        jj_rebase_main_step(bead),
        jj_track_step(bead),
        jj_describe_step(bead),
        validate_changes_step(bead),
        bookmark_create_step(bead),
    ];
    steps.push(bookmark_push_step(bead));
    steps.push(pr_create_step(bead, repo));
    steps
}

fn deps(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
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
        dependencies: deps(&["mark_in_progress"]),
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
        dependencies: Vec::new(),
    }
}

fn workspace_create_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "workspace_add".to_owned(),
        effect: Effect::Jj {
            args: vec![
                "workspace".to_owned(),
                "add".to_owned(),
                bead.workspace_path.clone(),
                "--name".to_owned(),
                bead.workspace.as_str().to_owned(),
            ],
            cwd: None,
        },
        compensation: Some(Compensation::ForgetWorkspace { workspace: bead.workspace.clone() }),
        transition: StepTransition::Static(LifecycleEvent::WorkspacePrepared),
        dependencies: deps(&["workspace_prepare"]),
    }
}

fn jj_sync_main_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "jj_sync_main".to_owned(),
        effect: Effect::Jj {
            args: vec![
                "git".to_owned(),
                "fetch".to_owned(),
                "--remote".to_owned(),
                "origin".to_owned(),
            ],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["moon_ci"]),
    }
}

fn jj_rebase_main_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "jj_rebase_main".to_owned(),
        effect: Effect::Jj {
            args: vec!["rebase".to_owned(), "-d".to_owned(), "main@origin".to_owned()],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["jj_sync_main"]),
    }
}

fn opencode_step(bead: &BeadData, model: &Model) -> LifecycleStep {
    let prompt = format!(
        "Implement bead {} in this workspace with real code changes. Do not call `oya` or `br`. Use moon/jj/gh as needed. Return short JSON summary with changed_files and ci_status.",
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
        dependencies: deps(&["workspace_add"]),
    }
}

fn validate_changes_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "validate_changes".to_owned(),
        effect: Effect::Jj {
            args: vec![
                "diff".to_owned(),
                "--name-only".to_owned(),
                "--from".to_owned(),
                "main@origin".to_owned(),
                "--to".to_owned(),
                "@".to_owned(),
            ],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::ValidateWorkspaceChanges,
        dependencies: deps(&["jj_describe"]),
    }
}

fn moon_ci_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "moon_ci".to_owned(),
        effect: Effect::MoonCi { cwd: Some(bead.workspace_path.clone()) },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["opencode"]),
    }
}

fn jj_track_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "jj_track".to_owned(),
        effect: Effect::Jj {
            args: vec!["file".to_owned(), "track".to_owned(), ".".to_owned()],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["jj_rebase_main"]),
    }
}

fn jj_describe_step(bead: &BeadData) -> LifecycleStep {
    let message = format!("chore: implement {} via lifecycle", bead.bead_id.as_str());
    LifecycleStep {
        name: "jj_describe".to_owned(),
        effect: Effect::Jj {
            args: vec!["describe".to_owned(), "-m".to_owned(), message],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["jj_track"]),
    }
}

fn bookmark_create_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "bookmark_create".to_owned(),
        effect: Effect::Jj {
            args: vec![
                "bookmark".to_owned(),
                "set".to_owned(),
                bead.bookmark.as_str().to_owned(),
                "-r".to_owned(),
                "@".to_owned(),
            ],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["validate_changes"]),
    }
}

fn bookmark_push_step(bead: &BeadData) -> LifecycleStep {
    LifecycleStep {
        name: "bookmark_push".to_owned(),
        effect: Effect::Jj {
            args: vec![
                "git".to_owned(),
                "push".to_owned(),
                "--remote".to_owned(),
                "origin".to_owned(),
                "--bookmark".to_owned(),
                bead.bookmark.as_str().to_owned(),
            ],
            cwd: Some(bead.workspace_path.clone()),
        },
        compensation: None,
        transition: StepTransition::None,
        dependencies: deps(&["bookmark_create"]),
    }
}

fn pr_create_step(bead: &BeadData, repo: Option<&str>) -> LifecycleStep {
    let title = format!("Lifecycle {}", bead.bead_id.as_str());
    let body = format!(
        "## Summary\n- Implements bead `{}` via lifecycle automation\n- Runs `moon run :ci` in workspace before opening PR\n- Publishes lifecycle status updates for polling",
        bead.bead_id.as_str()
    );
    let mut args = vec![
        "pr".to_owned(),
        "create".to_owned(),
        "--head".to_owned(),
        bead.bookmark.as_str().to_owned(),
    ];
    if let Some(value) = repo {
        args.push("--repo".to_owned());
        args.push(value.to_owned());
    }
    args.extend([
        "--base".to_owned(),
        "main".to_owned(),
        "--title".to_owned(),
        title,
        "--body".to_owned(),
        body,
    ]);
    LifecycleStep {
        name: "pr_create".to_owned(),
        effect: Effect::Gh { args, cwd: Some(bead.workspace_path.clone()) },
        compensation: None,
        transition: StepTransition::PullRequestOpened { bead: bead.clone() },
        dependencies: deps(&["bookmark_push"]),
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
    match run_effect(executor, effect).await {
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

fn make_step_progress_running(step: &str, started_at: &str) -> LifecycleProgressUpdate {
    LifecycleProgressUpdate::Step {
        step: step.to_owned(),
        status: LifecycleStepStatus::Running,
        message: None,
        details: None,
        started_at: Some(started_at.to_owned()),
        finished_at: None,
        duration_ms: None,
    }
}

fn make_step_progress_success(
    step: String,
    details: Option<Value>,
    started_at: &str,
    finished_at: &str,
    duration_ms: u64,
) -> LifecycleProgressUpdate {
    LifecycleProgressUpdate::Step {
        step,
        status: LifecycleStepStatus::Succeeded,
        message: None,
        details,
        started_at: Some(started_at.to_owned()),
        finished_at: Some(finished_at.to_owned()),
        duration_ms: Some(duration_ms),
    }
}

fn make_step_progress_failure(
    step: String,
    message: String,
    started_at: &str,
    finished_at: &str,
    duration_ms: u64,
) -> LifecycleProgressUpdate {
    LifecycleProgressUpdate::Step {
        step,
        status: LifecycleStepStatus::Failed,
        message: Some(message),
        details: None,
        started_at: Some(started_at.to_owned()),
        finished_at: Some(finished_at.to_owned()),
        duration_ms: Some(duration_ms),
    }
}

fn compute_duration_ms(start: &std::time::Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn timestamp_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn step_details(entry: &EffectJournalEntry) -> Option<Value> {
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
    state: &LifecycleState,
    transition: &StepTransition,
    entry: &EffectJournalEntry,
) -> Result<LifecycleState, LifecycleError> {
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

fn validate_workspace_changes(stdout: &str) -> Result<(), LifecycleError> {
    let files = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(strip_diff_prefix)
        .filter(|line| !line.starts_with(".beads/"))
        .collect::<Vec<_>>();
    if files.is_empty() {
        Err(LifecycleError::terminal(
            FailureCategory::EmptyChanges,
            "no non-.beads changes detected after opencode; refusing to open empty PR".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn strip_diff_prefix(line: &str) -> &str {
    line.strip_prefix("M ")
        .or_else(|| line.strip_prefix("A "))
        .or_else(|| line.strip_prefix("R "))
        .or_else(|| line.strip_prefix("D "))
        .unwrap_or(line)
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
            compensation_diagnostics: Vec::new(),
        }
    })?;
    acc.state = completed_state;
    let (cleanup, cleanup_diagnostics) = workspace_cleanup(executor, workspace).await;
    let pr_url = pr_url_from_state(&acc.state);
    on_progress(LifecycleProgressUpdate::Finished {
        success: true,
        pr_url,
        message: None,
        compensation_diagnostics: cleanup_diagnostics.clone(),
    });
    Ok(LifecycleRunOutcome {
        state: acc.state,
        journal: acc.journal,
        compensation_journal: cleanup,
        compensation_diagnostics: cleanup_diagnostics,
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
    let (mut compensation_journal, mut compensation_diagnostics) = if failure.error.is_terminal() {
        run_compensations(executor, failure.completed_compensations).await
    } else {
        (Vec::new(), Vec::new())
    };
    let (cleanup, cleanup_diagnostics) = workspace_cleanup(executor, workspace).await;
    compensation_journal = compensation_journal.into_iter().chain(cleanup).collect();
    compensation_diagnostics =
        compensation_diagnostics.into_iter().chain(cleanup_diagnostics.into_iter()).collect();
    on_progress(LifecycleProgressUpdate::Finished {
        success: false,
        pr_url: pr_url_from_state(&failure.state),
        message: Some(failure.error.to_string()),
        compensation_diagnostics: compensation_diagnostics.clone(),
    });
    Err(LifecycleRunFailure {
        error: failure.error,
        state: Some(failure.state),
        journal: failure.journal,
        compensation_journal,
        compensation_diagnostics,
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
) -> (Vec<EffectJournalEntry>, Vec<CompensationDiagnostic>) {
    let compensation = Compensation::ForgetWorkspace { workspace };
    let (entry, diagnostic) = run_compensation_with_diagnostic(executor, compensation).await;
    let journal = entry.into_iter().collect::<Vec<_>>();
    (journal, vec![diagnostic])
}

async fn run_compensations(
    executor: &dyn CommandExecutor,
    compensations: Vec<Compensation>,
) -> (Vec<EffectJournalEntry>, Vec<CompensationDiagnostic>) {
    let reversed = compensations.into_iter().rev().collect::<Vec<_>>();
    let attempts = stream::iter(reversed.into_iter())
        .then(|compensation| async move {
            run_compensation_with_diagnostic(executor, compensation).await
        })
        .collect::<Vec<(Option<EffectJournalEntry>, CompensationDiagnostic)>>()
        .await;
    let mut journal = Vec::new();
    let mut diagnostics = Vec::new();
    for (entry, diagnostic) in attempts {
        if let Some(item) = entry {
            journal.push(item);
        }
        diagnostics.push(diagnostic);
    }
    (journal, diagnostics)
}

async fn run_compensation_with_diagnostic(
    executor: &dyn CommandExecutor,
    compensation: Compensation,
) -> (Option<EffectJournalEntry>, CompensationDiagnostic) {
    let (comp_type, target) = compensation_metadata(&compensation);
    match run_compensation(executor, compensation).await {
        Ok(entry) => (
            Some(entry),
            CompensationDiagnostic {
                compensation_type: comp_type,
                target,
                success: true,
                error: None,
            },
        ),
        Err(error) => (
            None,
            CompensationDiagnostic {
                compensation_type: comp_type,
                target,
                success: false,
                error: Some(error.to_string()),
            },
        ),
    }
}

fn compensation_metadata(compensation: &Compensation) -> (String, String) {
    match compensation {
        Compensation::ForgetWorkspace { workspace } => {
            ("forget_workspace".to_owned(), workspace.as_str().to_owned())
        }
        Compensation::MarkBeadBlocked { bead, .. } => {
            ("mark_bead_blocked".to_owned(), bead.bead_id.as_str().to_owned())
        }
    }
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
#[path = "workflow_tests.rs"]
mod workflow_tests;
