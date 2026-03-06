#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::lifecycle::effects::{run_effect, CommandExecutor, Effect};
use crate::lifecycle::types::{BeadData, BeadId, FailureCategory, LifecycleError, Model, RepoSlug};
use serde::Deserialize;

use crate::lifecycle::workflow::dag::validate_dag;
use crate::lifecycle::workflow::steps::{build_steps, LifecycleStep};
use crate::lifecycle::workflow::types::{LifecycleRunFailure, LifecycleRunRequest};

#[derive(Debug, Deserialize)]
struct ReadyIssue {
    id: ReadyBeadId,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
struct ReadyBeadId(String);

impl ReadyBeadId {
    fn into_string(self) -> String {
        self.0
    }
}

pub async fn resolve_and_validate(
    executor: &dyn CommandExecutor,
    request: &LifecycleRunRequest,
) -> Result<(BeadData, Vec<LifecycleStep>), LifecycleRunFailure> {
    let bead = resolve_bead_data(executor, request).await.map_err(map_startup_failure)?;
    let model = resolve_model(request.model.as_deref()).map_err(map_startup_failure)?;
    let repo = validate_repo_slug(request.repo.as_deref()).map_err(map_startup_failure)?;
    let steps = build_steps(&bead, &model, repo.as_deref());
    validate_dag(&steps).map_err(map_startup_failure)?;
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
        Effect::Bd { args: vec!["ready".to_owned(), "--json".to_owned()], cwd: None },
    )
    .await?;
    let json = extract_json_array(&entry.stdout)?;
    let issues = serde_json::from_str::<Vec<ReadyIssue>>(json).map_err(|error| {
        LifecycleError::terminal(
            FailureCategory::Validation,
            format!("failed to parse bd ready payload: {error}"),
        )
    })?;
    issues.first().map_or_else(
        || Err(LifecycleError::terminal(FailureCategory::Validation, "no ready beads found")),
        |issue| Ok(issue.id.clone().into_string()),
    )
}

fn extract_json_array(raw: &str) -> Result<&str, LifecycleError> {
    raw.find('[').map_or_else(
        || {
            Err(LifecycleError::terminal(
                FailureCategory::Validation,
                "bd ready --json returned no JSON payload",
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

fn map_startup_failure(error: LifecycleError) -> LifecycleRunFailure {
    LifecycleRunFailure {
        error,
        state: None,
        journal: Vec::new(),
        compensation_journal: Vec::new(),
        compensation_diagnostics: Vec::new(),
    }
}
