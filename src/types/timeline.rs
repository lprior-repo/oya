//! Timeline and stage-event types used for run observability.

use super::domain::GateResult;
use super::pipeline::FailureCategory;
use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Rich value objects
// ---------------------------------------------------------------------------

/// Newtype for workspace names - prevents stringly typed confusion
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct WorkspaceName(pub String);

impl WorkspaceName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Newtype for duration in milliseconds
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DurationMs(pub u64);

impl DurationMs {
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs * 1000)
    }

    pub const fn as_secs(self) -> u64 {
        self.0 / 1000
    }
}

impl std::fmt::Display for DurationMs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 < 1000 {
            write!(f, "{}ms", self.0)
        } else {
            write!(f, "{:.1}s", self.0 as f64 / 1000.0)
        }
    }
}

// ---------------------------------------------------------------------------
// Gate summary
// ---------------------------------------------------------------------------

/// Gate result with minimal data (lightweight, used in timeline events)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateSummary {
    pub gate: String,
    pub passed: bool,
}

// ---------------------------------------------------------------------------
// Stage outcome
// ---------------------------------------------------------------------------

/// Outcome of a stage - mutually exclusive states
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StageOutcome {
    Passed { gates: Vec<GateResult> },
    Failed { category: FailureCategory, message: String },
    RetryScheduled { next_attempt: u32, reason: String },
}

impl StageOutcome {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }
}

// ---------------------------------------------------------------------------
// Timeline entry (canonical event log)
// ---------------------------------------------------------------------------

/// Unified timeline event - ~3 per stage + run-level events.
/// Replaces verbose 24+ events with ~11 rich events.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum TimelineEntry {
    RunStarted {
        bead_id: String,
        context: String,
        at: DateTime<Utc>,
    },
    StageStarted {
        stage: String,
        attempt: u32,
        workspace: Option<String>,
        at: DateTime<Utc>,
    },
    StageCompleted {
        stage: String,
        attempt: u32,
        workspace: Option<String>,
        duration_ms: u64,
        gates: Vec<GateSummary>,
        at: DateTime<Utc>,
    },
    StageFailed {
        stage: String,
        attempt: u32,
        workspace: Option<String>,
        duration_ms: u64,
        category: String,
        message: String,
        retry_scheduled: bool,
        at: DateTime<Utc>,
    },
    RunShipped {
        total_duration_ms: u64,
        stages_passed: u32,
        at: DateTime<Utc>,
    },
    RunFailed {
        stage: String,
        category: String,
        at: DateTime<Utc>,
    },
}

impl TimelineEntry {
    pub fn timestamp(&self) -> &DateTime<Utc> {
        match self {
            Self::RunStarted { at, .. } => at,
            Self::StageStarted { at, .. } => at,
            Self::StageCompleted { at, .. } => at,
            Self::StageFailed { at, .. } => at,
            Self::RunShipped { at, .. } => at,
            Self::RunFailed { at, .. } => at,
        }
    }

    pub fn stage(&self) -> Option<&str> {
        match self {
            Self::RunStarted { .. } => None,
            Self::StageStarted { stage, .. } => Some(stage),
            Self::StageCompleted { stage, .. } => Some(stage),
            Self::StageFailed { stage, .. } => Some(stage),
            Self::RunShipped { .. } => None,
            Self::RunFailed { stage, .. } => Some(stage),
        }
    }
}

// ---------------------------------------------------------------------------
// Text utilities
// ---------------------------------------------------------------------------

/// Strip ANSI escape codes from a string (pure function)
pub fn strip_ansi_codes(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if let Some(&next) = chars.peek() {
                if next == '[' {
                    chars.next();
                    while let Some(&peek) = chars.peek() {
                        match peek {
                            '0'..='9' | ';' | '?' => {
                                chars.next();
                            }
                            _ => {
                                chars.next();
                                break;
                            }
                        }
                    }
                    continue;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Truncate text to max chars, appending truncation marker if needed.
pub fn truncate_clean(input: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let stripped = strip_ansi_codes(input);
    let chars: Vec<char> = stripped.chars().take(max_chars).collect();
    if stripped.chars().count() > max_chars {
        format!("{}…[truncated]", chars.into_iter().collect::<String>())
    } else {
        chars.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_color_codes() {
        let input = "\x1b[32mSuccess\x1b[0m";
        assert_eq!(strip_ansi_codes(input), "Success");
    }

    #[test]
    fn strip_ansi_removes_dim_codes() {
        let input = "\x1b[2m2026-02-18\x1b[0m INFO";
        assert_eq!(strip_ansi_codes(input), "2026-02-18 INFO");
    }

    #[test]
    fn strip_ansi_preserves_plain_text() {
        let input = "Hello, world!";
        assert_eq!(strip_ansi_codes(input), "Hello, world!");
    }

    #[test]
    fn truncate_clean_strips_and_truncates() {
        let input = "\x1b[32mThis is a long message\x1b[0m";
        let result = truncate_clean(input, 10);
        assert!(result.contains("This is a"));
        assert!(result.contains("truncated"));
    }

    #[test]
    fn duration_ms_display() {
        assert_eq!(format!("{}", DurationMs(500)), "500ms");
        assert_eq!(format!("{}", DurationMs(1500)), "1.5s");
        assert_eq!(format!("{}", DurationMs(61000)), "61.0s");
    }
}
