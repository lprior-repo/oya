use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::stages::{Stage, validate_stage_sequence};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Slug(String);

impl Slug {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create a slug from user input.
    ///
    /// # Errors
    /// Returns an error when the slug is empty or contains invalid characters.
    pub fn new(input: impl AsRef<str>) -> Result<Self> {
        let raw = input.as_ref().trim();
        if raw.is_empty() {
            return Err(Error::InvalidSlug("slug cannot be empty".to_string()));
        }

        let invalid = raw
            .chars()
            .any(|ch| !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-');

        if invalid {
            return Err(Error::InvalidSlug(
                "slug must use lowercase letters, digits, or '-'".to_string(),
            ));
        }

        Ok(Self(raw.to_string()))
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Go,
    Python,
    JavaScript,
    Gleam,
}

impl Language {
    /// Detect the language from repository marker files.
    ///
    /// # Errors
    /// Returns an error if no marker is present or multiple markers conflict.
    pub fn detect_from_files(
        has_gleam: bool,
        has_go: bool,
        has_cargo: bool,
        has_python: bool,
        has_js: bool,
    ) -> Result<Self> {
        let markers = [has_gleam, has_go, has_cargo, has_python, has_js];
        let count = markers.iter().filter(|value| **value).count();

        if count == 0 {
            return Err(Error::UnknownLanguage);
        }

        if count > 1 {
            return Err(Error::UnknownLanguage);
        }

        if has_gleam {
            Ok(Self::Gleam)
        } else if has_go {
            Ok(Self::Go)
        } else if has_cargo {
            Ok(Self::Rust)
        } else if has_python {
            Ok(Self::Python)
        } else if has_js {
            Ok(Self::JavaScript)
        } else {
            Err(Error::UnknownLanguage)
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Rust => "Rust",
            Self::Go => "Go",
            Self::Python => "Python",
            Self::JavaScript => "JavaScript",
            Self::Gleam => "Gleam",
        };
        write!(f, "{name}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
}

impl Priority {
    /// Parse priority from a string.
    ///
    /// # Errors
    /// Returns an error when the priority is unknown.
    pub fn parse(input: &str) -> Result<Self> {
        match input.trim().to_uppercase().as_str() {
            "P0" => Ok(Self::P0),
            "P1" => Ok(Self::P1),
            "P2" => Ok(Self::P2),
            "P3" => Ok(Self::P3),
            other => Err(Error::UnknownPriority(other.to_string())),
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::P0 => "P0",
            Self::P1 => "P1",
            Self::P2 => "P2",
            Self::P3 => "P3",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Created,
    InProgress { stage: String },
    PassedPipeline,
    FailedPipeline { stage: String, reason: String },
    Integrated,
}

impl TaskStatus {
    /// Construct an in-progress status after validating the stage label.
    ///
    /// # Errors
    /// Returns an error when the stage is empty.
    pub fn in_progress(stage: impl AsRef<str>) -> Result<Self> {
        let trimmed = stage.as_ref().trim();
        if trimmed.is_empty() {
            return Err(Error::InvalidStage("stage cannot be empty".to_string()));
        }

        Ok(Self::InProgress {
            stage: trimmed.to_string(),
        })
    }

    /// Construct a failure status after validating the stage and reason.
    ///
    /// # Errors
    /// Returns an error when the stage or reason is empty.
    pub fn failed(stage: impl AsRef<str>, reason: impl AsRef<str>) -> Result<Self> {
        let stage_trimmed = stage.as_ref().trim();
        if stage_trimmed.is_empty() {
            return Err(Error::InvalidStage("stage cannot be empty".to_string()));
        }

        let reason_trimmed = reason.as_ref().trim();
        if reason_trimmed.is_empty() {
            return Err(Error::InvalidFailureReason(
                "reason cannot be empty".to_string(),
            ));
        }

        Ok(Self::FailedPipeline {
            stage: stage_trimmed.to_string(),
            reason: reason_trimmed.to_string(),
        })
    }

    #[must_use]
    pub const fn is_transient(&self) -> bool {
        matches!(self, Self::InProgress { .. })
    }

    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::FailedPipeline { .. })
    }

    #[must_use]
    pub fn to_filter_status(&self) -> String {
        match self {
            Self::Created | Self::InProgress { .. } => "open".to_string(),
            Self::PassedPipeline => "passed".to_string(),
            Self::FailedPipeline { .. } => "failed".to_string(),
            Self::Integrated => "integrated".to_string(),
        }
    }

    /// Construct an in-progress status from a canonical stage.
    #[must_use]
    pub fn in_progress_stage(stage: Stage) -> Self {
        Self::InProgress {
            stage: stage.as_str().to_string(),
        }
    }

    /// Construct a failed status from a canonical stage and reason.
    ///
    /// # Errors
    /// Returns an error when the failure reason is empty.
    pub fn failed_stage(stage: Stage, reason: impl AsRef<str>) -> Result<Self> {
        let reason_trimmed = reason.as_ref().trim();
        if reason_trimmed.is_empty() {
            return Err(Error::InvalidFailureReason(
                "reason cannot be empty".to_string(),
            ));
        }

        Ok(Self::FailedPipeline {
            stage: stage.as_str().to_string(),
            reason: reason_trimmed.to_string(),
        })
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "Created"),
            Self::InProgress { stage } => write!(f, "In Progress ({stage})"),
            Self::PassedPipeline => write!(f, "Passed"),
            Self::FailedPipeline { stage, reason } => {
                write!(f, "Failed ({stage}: {reason})")
            }
            Self::Integrated => write!(f, "Integrated"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub slug: Slug,
    pub language: Language,
    pub status: TaskStatus,
    pub priority: Priority,
    pub branch: String,
}

impl Task {
    #[must_use]
    pub fn new(slug: Slug, language: Language) -> Self {
        let branch = format!("task/{}", slug.as_str());
        Self {
            slug,
            language,
            status: TaskStatus::Created,
            priority: Priority::P2,
            branch,
        }
    }

    #[must_use]
    pub const fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status = status;
        self
    }

    /// Transition the task to a new status if the transition is valid.
    ///
    /// # Errors
    /// Returns an error when the transition is not allowed.
    pub fn transition_to(&self, next: TaskStatus) -> Result<Self> {
        validate_transition(&self.status, &next)?;
        Ok(self.clone().with_status(next))
    }

    /// Begin a pipeline stage with canonical stage validation.
    ///
    /// # Errors
    /// Returns an error if the transition is invalid.
    pub fn start_stage(&self, stage: Stage) -> Result<Self> {
        let status = TaskStatus::in_progress_stage(stage);
        self.transition_to(status)
    }

    /// Mark a stage failure with a canonical stage.
    ///
    /// # Errors
    /// Returns an error if the transition is invalid or the reason is empty.
    pub fn fail_stage(&self, stage: Stage, reason: impl AsRef<str>) -> Result<Self> {
        let status = TaskStatus::failed_stage(stage, reason)?;
        self.transition_to(status)
    }

    /// Mark the pipeline as passed.
    ///
    /// # Errors
    /// Returns an error if the transition is invalid.
    pub fn pass_pipeline(&self) -> Result<Self> {
        self.transition_to(TaskStatus::PassedPipeline)
    }

    /// Mark the task as integrated.
    ///
    /// # Errors
    /// Returns an error if the transition is invalid.
    pub fn integrate(&self) -> Result<Self> {
        self.transition_to(TaskStatus::Integrated)
    }
}

fn stage_progresses(from_stage: &str, to_stage: &str) -> Result<()> {
    let from = Stage::parse(from_stage)?;
    let to = Stage::parse(to_stage)?;
    validate_stage_sequence(&[from, to])
}

fn invalid_transition(from: &TaskStatus, to: &TaskStatus) -> Error {
    Error::InvalidTransition {
        from: from.to_string(),
        to: to.to_string(),
    }
}

fn validate_transition(from: &TaskStatus, to: &TaskStatus) -> Result<()> {
    if from == to {
        return Ok(());
    }

    match (from, to) {
        (
            TaskStatus::Created,
            TaskStatus::InProgress { .. } | TaskStatus::FailedPipeline { .. },
        ) => Ok(()),
        (
            TaskStatus::InProgress { stage: from_stage },
            TaskStatus::InProgress { stage: to_stage },
        ) => stage_progresses(from_stage, to_stage).map_err(|_| invalid_transition(from, to)),
        (TaskStatus::InProgress { .. }, TaskStatus::PassedPipeline)
        | (TaskStatus::InProgress { .. }, TaskStatus::FailedPipeline { .. })
        | (TaskStatus::FailedPipeline { .. }, TaskStatus::InProgress { .. })
        | (TaskStatus::PassedPipeline, TaskStatus::Integrated) => Ok(()),
        _ => Err(invalid_transition(from, to)),
    }
}

#[cfg(test)]
mod tests {

    #![allow(clippy::expect_used)]
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn slug_rejects_invalid_characters() {
        let result = Slug::new("Bad_Slug");
        assert!(result.is_err());
    }

    #[test]
    fn slug_accepts_lowercase_and_dashes() {
        let slug = Slug::new("good-slug-1").expect("slug should parse");
        assert_eq!(slug.as_str(), "good-slug-1");
    }

    #[test]
    fn language_detection_requires_single_marker() {
        let result = Language::detect_from_files(true, true, false, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn language_detection_prefers_single_marker() {
        let result = Language::detect_from_files(false, false, true, false, false);
        assert!(matches!(result, Ok(Language::Rust)));
    }

    #[test]
    fn priority_parse_accepts_known_values() {
        let parsed = Priority::parse("p1").expect("priority should parse");
        assert_eq!(parsed, Priority::P1);
    }

    #[test]
    fn status_helpers_match_behavior() {
        let status = TaskStatus::in_progress("implement").expect("stage should be valid");

        assert!(status.is_transient());
        assert!(!status.is_failed());
        assert_eq!(status.to_filter_status(), "open");
    }

    #[test]
    fn in_progress_rejects_blank_stage() {
        let result = TaskStatus::in_progress("   ");
        assert!(result.is_err());
    }

    #[test]
    fn failed_rejects_blank_reason() {
        let result = TaskStatus::failed("lint", " ");
        assert!(result.is_err());
    }

    #[test]
    fn failed_accepts_valid_inputs() {
        let status =
            TaskStatus::failed("lint", "error").expect("failure status should be constructed");
        assert!(status.is_failed());
        assert_eq!(status.to_filter_status(), "failed");
    }

    #[test]
    fn task_builds_default_fields() {
        let slug = Slug::new("task-1").expect("slug should parse");
        let task = Task::new(slug, Language::Rust);

        assert_eq!(task.priority, Priority::P2);
        assert_eq!(task.status, TaskStatus::Created);
        assert!(task.branch.starts_with("task/"));
    }

    #[test]
    fn task_transition_rejects_invalid_jump() {
        let slug = Slug::new("task-2").expect("slug should parse");
        let task = Task::new(slug, Language::Rust);
        let result = task.transition_to(TaskStatus::Integrated);
        assert!(result.is_err());
    }

    #[test]
    fn task_transition_allows_retry_after_failure() {
        let slug = Slug::new("task-3").expect("slug should parse");
        let task = Task::new(slug, Language::Rust)
            .start_stage(Stage::Implement)
            .expect("stage should start");

        let failed = task
            .fail_stage(Stage::Implement, "failure")
            .expect("failure should be recorded");

        let retried = failed
            .start_stage(Stage::Implement)
            .expect("retry should be allowed");

        assert!(matches!(retried.status, TaskStatus::InProgress { .. }));
    }

    #[test]
    fn task_transition_allows_stage_to_stage_progression() {
        let slug = Slug::new("task-5").expect("slug should parse");
        let task = Task::new(slug, Language::Rust)
            .start_stage(Stage::Implement)
            .expect("stage should start");

        let advanced = task
            .start_stage(Stage::UnitTest)
            .expect("transition to next stage should be allowed");

        assert_eq!(
            advanced.status,
            TaskStatus::InProgress {
                stage: Stage::UnitTest.as_str().to_string()
            }
        );
    }

    #[test]
    fn task_transition_rejects_regressing_stage() {
        let slug = Slug::new("task-6").expect("slug should parse");
        let task = Task::new(slug, Language::Rust)
            .start_stage(Stage::UnitTest)
            .expect("stage should start");

        let result = task.start_stage(Stage::Implement);
        assert!(matches!(result, Err(Error::InvalidTransition { .. })));
    }

    #[test]
    fn task_pass_and_integrate_follow_order() {
        let slug = Slug::new("task-4").expect("slug should parse");
        let task = Task::new(slug, Language::Rust)
            .start_stage(Stage::Implement)
            .expect("stage should start");

        let passed = task.pass_pipeline().expect("pipeline should pass");
        let integrated = passed.integrate().expect("integration should succeed");

        assert_eq!(integrated.status, TaskStatus::Integrated);
    }
}
