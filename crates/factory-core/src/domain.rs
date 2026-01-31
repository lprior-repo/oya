//! Domain types for Factory pipeline.
//!
//! Pure data types that make illegal states unrepresentable.
//! Uses opaque validated types for `Slug` and `GitHash`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Maximum length for a slug.
const MAX_SLUG_LENGTH: usize = 50;

/// Check if a string contains only valid slug characters.
fn is_valid_slug(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Opaque validated slug type - guaranteed to be valid if constructed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Slug(String);

impl Slug {
    /// Validate and create a new slug.
    ///
    /// Valid slugs are:
    /// - Non-empty
    /// - 1-50 characters
    /// - Only contain: lowercase a-z, digits 0-9, hyphen, underscore
    pub fn new(s: impl Into<String>) -> Result<Self> {
        let s = s.into();
        let len = s.len();

        if len == 0 {
            return Err(Error::invalid_slug("slug cannot be empty"));
        }

        if len > MAX_SLUG_LENGTH {
            return Err(Error::invalid_slug(format!(
                "slug must be 1-{MAX_SLUG_LENGTH} characters"
            )));
        }

        if !is_valid_slug(&s) {
            return Err(Error::invalid_slug(
                "slug contains invalid characters (use a-z, 0-9, -, _)",
            ));
        }

        Ok(Self(s))
    }

    /// Get the inner string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if slug contains separators (hyphens or underscores).
    #[must_use]
    pub fn has_separators(&self) -> bool {
        self.0.contains('-') || self.0.contains('_')
    }
}

impl TryFrom<String> for Slug {
    type Error = Error;

    fn try_from(s: String) -> Result<Self> {
        Self::new(s)
    }
}

impl From<Slug> for String {
    fn from(slug: Slug) -> Self {
        slug.0
    }
}

impl std::fmt::Display for Slug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Opaque validated git hash type - guaranteed to be a valid 40-char hex string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct GitHash(String);

impl GitHash {
    /// Validate and create a new git hash.
    pub fn new(s: impl Into<String>) -> Result<Self> {
        let s = s.into();

        if s.len() != 40 {
            return Err(Error::InvalidGitHash {
                reason: "git hash must be exactly 40 characters".into(),
            });
        }

        if !s.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::InvalidGitHash {
                reason: "git hash must contain only hex characters".into(),
            });
        }

        Ok(Self(s.to_lowercase()))
    }

    /// Get the inner string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for GitHash {
    type Error = Error;

    fn try_from(s: String) -> Result<Self> {
        Self::new(s)
    }
}

impl From<GitHash> for String {
    fn from(hash: GitHash) -> Self {
        hash.0
    }
}

impl std::fmt::Display for GitHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Supported programming languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Go,
    Gleam,
    Rust,
    Python,
    Javascript,
}

impl Language {
    /// Parse language from string.
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "go" => Ok(Self::Go),
            "gleam" => Ok(Self::Gleam),
            "rust" => Ok(Self::Rust),
            "python" => Ok(Self::Python),
            "javascript" | "js" => Ok(Self::Javascript),
            other => Err(Error::UnsupportedLanguage {
                lang: other.to_string(),
            }),
        }
    }

    /// Detect language from repository file markers.
    #[allow(clippy::fn_params_excessive_bools)]
    pub const fn detect_from_files(
        has_gleam_toml: bool,
        has_go_mod: bool,
        has_cargo_toml: bool,
        has_pyproject: bool,
        has_package_json: bool,
    ) -> Result<Self> {
        if has_gleam_toml {
            Ok(Self::Gleam)
        } else if has_go_mod {
            Ok(Self::Go)
        } else if has_cargo_toml {
            Ok(Self::Rust)
        } else if has_pyproject {
            Ok(Self::Python)
        } else if has_package_json {
            Ok(Self::Javascript)
        } else {
            Err(Error::LanguageDetectionFailed)
        }
    }

    /// Get display name for the language.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Go => "Go",
            Self::Gleam => "Gleam",
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::Javascript => "JavaScript",
        }
    }

    /// Get the lowercase identifier.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Go => "go",
            Self::Gleam => "gleam",
            Self::Rust => "rust",
            Self::Python => "python",
            Self::Javascript => "javascript",
        }
    }

    /// Check if language is compiled.
    #[must_use]
    pub const fn is_compiled(&self) -> bool {
        matches!(self, Self::Go | Self::Rust | Self::Gleam)
    }

    /// Check if language is dynamically typed.
    #[must_use]
    pub const fn is_dynamic(&self) -> bool {
        matches!(self, Self::Python)
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Priority levels for tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Priority {
    P1,
    #[default]
    P2,
    P3,
}

impl Priority {
    /// Parse priority from string.
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "P1" => Ok(Self::P1),
            "P2" => Ok(Self::P2),
            "P3" => Ok(Self::P3),
            other => Err(Error::InvalidPriority {
                value: other.to_string(),
            }),
        }
    }

    /// Get string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::P1 => "P1",
            Self::P2 => "P2",
            Self::P3 => "P3",
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Task status - overall pipeline status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Created,
    InProgress {
        stage: String,
    },
    PassedPipeline,
    FailedPipeline {
        stage: String,
        reason: String,
    },
    Integrated,
}

impl TaskStatus {
    /// Check if status is transient (in progress).
    #[must_use]
    pub const fn is_transient(&self) -> bool {
        matches!(self, Self::InProgress { .. })
    }

    /// Check if status indicates failure.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::FailedPipeline { .. })
    }

    /// Check if status indicates completion (success).
    #[must_use]
    pub const fn is_completed(&self) -> bool {
        matches!(self, Self::PassedPipeline | Self::Integrated)
    }

    /// Get failure reason if failed.
    #[must_use]
    pub fn failure_reason(&self) -> Option<&str> {
        match self {
            Self::FailedPipeline { reason, .. } => Some(reason),
            _ => None,
        }
    }

    /// Get current stage if in progress.
    #[must_use]
    pub fn current_stage(&self) -> Option<&str> {
        match self {
            Self::InProgress { stage } => Some(stage),
            _ => None,
        }
    }

    /// Check if task is ready for integration.
    #[must_use]
    pub const fn is_ready_for_integration(&self) -> bool {
        matches!(self, Self::PassedPipeline)
    }

    /// Convert to simple status string for filtering.
    #[must_use]
    pub const fn to_filter_status(&self) -> &'static str {
        match self {
            Self::Created | Self::FailedPipeline { .. } => "open",
            Self::InProgress { .. } => "in_progress",
            Self::PassedPipeline | Self::Integrated => "done",
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::InProgress { stage } => write!(f, "in_progress ({stage})"),
            Self::PassedPipeline => write!(f, "passed_pipeline"),
            Self::FailedPipeline { stage, reason } => {
                write!(f, "failed_pipeline ({stage}: {reason})")
            }
            Self::Integrated => write!(f, "integrated"),
        }
    }
}

/// Pipeline stage definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    pub name: String,
    pub gate: String,
    pub retries: u32,
}

impl Stage {
    /// Create a new stage.
    #[must_use]
    pub fn new(name: impl Into<String>, gate: impl Into<String>, retries: u32) -> Self {
        Self {
            name: name.into(),
            gate: gate.into(),
            retries,
        }
    }
}

/// Standard 9-stage pipeline.
#[must_use]
pub fn standard_pipeline() -> Vec<Stage> {
    vec![
        Stage::new("implement", "Code compiles", 5),
        Stage::new("unit-test", "All tests pass", 3),
        Stage::new("coverage", "80% coverage", 5),
        Stage::new("lint", "Code formatted", 3),
        Stage::new("static", "Static analysis passes", 3),
        Stage::new("integration", "Integration tests pass", 3),
        Stage::new("security", "No vulnerabilities", 2),
        Stage::new("review", "Code review passes", 3),
        Stage::new("accept", "Ready for merge", 1),
    ]
}

/// Get a stage by name from the standard pipeline.
pub fn get_stage(name: &str) -> Result<Stage> {
    standard_pipeline()
        .into_iter()
        .find(|s| s.name == name)
        .ok_or_else(|| Error::UnknownStage {
            name: name.to_string(),
        })
}

/// Find the index of a stage in the pipeline.
fn find_stage_index(pipeline: &[Stage], name: &str) -> Option<usize> {
    pipeline.iter().position(|s| s.name == name)
}

/// Filter stages from `start_name` to `end_name` (inclusive).
pub fn filter_stages(start_name: &str, end_name: &str) -> Result<Vec<Stage>> {
    let pipeline = standard_pipeline();

    let start_idx = find_stage_index(&pipeline, start_name);
    let end_idx = find_stage_index(&pipeline, end_name);

    match (start_idx, end_idx) {
        (Some(si), Some(ei)) if si <= ei => {
            Ok(pipeline.into_iter().skip(si).take(ei - si + 1).collect())
        }
        (Some(_), Some(_)) => Err(Error::StageRangeError {
            reason: format!("start stage '{start_name}' must come before end stage '{end_name}'"),
        }),
        _ => Err(Error::StageRangeError {
            reason: format!("one or both stages not found in pipeline: {start_name} to {end_name}"),
        }),
    }
}

/// Get max retries across all stages in pipeline.
#[must_use]
pub fn max_pipeline_retries(pipeline: &[Stage]) -> u32 {
    pipeline.iter().map(|s| s.retries).max().unwrap_or(0)
}

/// Get all gate names from pipeline.
#[must_use]
pub fn gate_names(pipeline: &[Stage]) -> Vec<&str> {
    pipeline.iter().map(|s| s.gate.as_str()).collect()
}

/// Count stages with specific gate name.
#[must_use]
pub fn count_stages_by_gate(pipeline: &[Stage], gate: &str) -> usize {
    pipeline.iter().filter(|s| s.gate == gate).count()
}

/// Task represents a single unit of work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub slug: Slug,
    pub language: Language,
    pub status: TaskStatus,
    pub priority: Priority,
    pub worktree_path: PathBuf,
    pub branch: String,
}

impl Task {
    /// Branch prefix for feature branches.
    pub const BRANCH_PREFIX: &'static str = "feat/";

    /// Create a new task with default status.
    #[must_use]
    pub fn new(slug: Slug, language: Language, worktree_path: PathBuf) -> Self {
        let branch = format!("{}{}", Self::BRANCH_PREFIX, slug);
        Self {
            slug,
            language,
            status: TaskStatus::Created,
            priority: Priority::default(),
            worktree_path,
            branch,
        }
    }

    /// Create a task with specified priority.
    #[must_use]
    pub const fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Update task status.
    #[must_use]
    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status = status;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug_validation() {
        // Valid slugs
        assert!(Slug::new("my-task").is_ok());
        assert!(Slug::new("task_123").is_ok());
        assert!(Slug::new("a").is_ok());

        // Invalid slugs
        assert!(Slug::new("").is_err());
        assert!(Slug::new("a".repeat(51)).is_err());
        assert!(Slug::new("My-Task").is_err()); // uppercase
        assert!(Slug::new("my task").is_err()); // space
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(
            Language::detect_from_files(true, false, false, false, false).ok(),
            Some(Language::Gleam)
        );
        assert_eq!(
            Language::detect_from_files(false, true, false, false, false).ok(),
            Some(Language::Go)
        );
        assert!(Language::detect_from_files(false, false, false, false, false).is_err());
    }

    #[test]
    fn test_stage_filtering() {
        let stages = filter_stages("implement", "lint").ok();
        let names: Option<Vec<String>> =
            stages.map(|s| s.iter().map(|st| st.name.clone()).collect());
        assert_eq!(
            names,
            Some(vec![
                "implement".to_string(),
                "unit-test".to_string(),
                "coverage".to_string(),
                "lint".to_string()
            ])
        );
    }

    #[test]
    fn test_priority_parse() {
        assert_eq!(Priority::parse("P1").ok(), Some(Priority::P1));
        assert_eq!(Priority::parse("p2").ok(), Some(Priority::P2));
        assert!(Priority::parse("P4").is_err());
    }
}
