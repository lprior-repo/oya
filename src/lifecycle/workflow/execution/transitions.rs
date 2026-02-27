#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::lifecycle::effects::{Compensation, EffectJournalEntry};
use crate::lifecycle::transitions::{apply_event, LifecycleEvent};
use crate::lifecycle::types::{
    BeadData, FailureCategory, LifecycleError, LifecycleState, PrInfo, PrNumber,
};

use crate::lifecycle::workflow::steps::{LifecycleStep, StepTransition};
use crate::lifecycle::workflow::types::{ExecutionAcc, StepFailure};

#[derive(Debug, thiserror::Error)]
enum PullRequestParseError {
    #[error("gh pr create output did not include PR URL")]
    MissingUrl,
    #[error("missing PR number in URL")]
    MissingNumber,
    #[error("invalid PR number in URL `{url}`: {source}")]
    InvalidNumber { url: String, source: std::num::ParseIntError },
    #[error("invalid PR number in URL `{url}`: {source}")]
    InvalidDomain { url: String, source: crate::lifecycle::types::PrNumberError },
}

pub fn success_acc(
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

pub fn failed_state(state: &LifecycleState, error: &LifecycleError) -> LifecycleState {
    match apply_event(state, LifecycleEvent::Failed(error.clone())) {
        Ok(next) => next,
        Err(_) => state.clone(),
    }
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
    ["M ", "A ", "R ", "D "]
        .into_iter()
        .find_map(|prefix| line.strip_prefix(prefix))
        .map_or(line, |trimmed| trimmed)
}

fn parse_pr_info(bead: &BeadData, stdout: &str) -> Result<PrInfo, LifecycleError> {
    let url = extract_pr_url(stdout)
        .ok_or(PullRequestParseError::MissingUrl)
        .map_err(|error| map_pr_parse_error(&error))?;
    let pr_number = parse_pr_number(&url).map_err(|error| map_pr_parse_error(&error))?;
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

fn parse_pr_number(url: &str) -> Result<PrNumber, PullRequestParseError> {
    let value =
        url.rsplit('/').next().ok_or(PullRequestParseError::MissingNumber).and_then(|segment| {
            segment.parse::<u64>().map_err(|source| PullRequestParseError::InvalidNumber {
                url: url.to_owned(),
                source,
            })
        })?;
    PrNumber::new(value)
        .map_err(|source| PullRequestParseError::InvalidDomain { url: url.to_owned(), source })
}

fn map_pr_parse_error(error: &PullRequestParseError) -> LifecycleError {
    LifecycleError::terminal(FailureCategory::PullRequest, error.to_string())
}

fn append_entry(
    journal: Vec<EffectJournalEntry>,
    entry: EffectJournalEntry,
) -> Vec<EffectJournalEntry> {
    journal.into_iter().chain(std::iter::once(entry)).collect()
}

fn append_compensation(
    compensations: Vec<Compensation>,
    compensation: Compensation,
) -> Vec<Compensation> {
    compensations.into_iter().chain(std::iter::once(compensation)).collect()
}
