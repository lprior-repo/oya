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
///
/// # Edge Cases
///
/// - If `max_chars` is 0 and input is non-empty, returns `"[truncated]"` marker
/// - If both input is empty and `max_chars` is 0, returns empty string
pub fn truncate_clean(input: &str, max_chars: usize) -> String {
    let stripped = strip_ansi_codes(input);
    let input_len = stripped.chars().count();

    // Handle edge case: max_chars=0 with non-empty input shows truncation marker
    if max_chars == 0 {
        return if input_len > 0 { "…[truncated]".to_string() } else { String::new() };
    }

    let chars: Vec<char> = stripped.chars().take(max_chars).collect();
    if input_len > max_chars {
        format!("{}…[truncated]", chars.into_iter().collect::<String>())
    } else {
        chars.into_iter().collect()
    }
}

/// Sanitize a URL for safe logging/error output by removing credentials.
///
/// Replaces any embedded credentials (username:password@) with `***:***@`.
/// This prevents accidental credential leakage in logs and error messages.
///
/// # Examples
///
/// ```
/// use oya::types::sanitize_url_for_logging;
///
/// // URL with credentials
/// assert_eq!(
///     sanitize_url_for_logging("https://user:secret@example.com/path"),
///     "https://***:***@example.com/path"
/// );
///
/// // URL without credentials is unchanged
/// assert_eq!(
///     sanitize_url_for_logging("https://example.com/path"),
///     "https://example.com/path"
/// );
///
/// // URL with username only (we still show ***:***@ for consistency)
/// assert_eq!(
///     sanitize_url_for_logging("https://user@example.com/path"),
///     "https://***:***@example.com/path"
/// );
/// ```
pub fn sanitize_url_for_logging(url: &str) -> String {
    match reqwest::Url::parse(url) {
        Ok(parsed) => {
            let has_credentials = !parsed.username().is_empty() || parsed.password().is_some();
            if has_credentials {
                // Rebuild URL without credentials
                let scheme = parsed.scheme();
                let host = parsed.host_str().map_or("", |h| h);
                let port = parsed.port().map_or(String::new(), |p| format!(":{p}"));
                let path = parsed.path();
                let query = parsed.query().map_or(String::new(), |q| format!("?{q}"));
                let fragment = parsed.fragment().map_or(String::new(), |f| format!("#{f}"));

                format!("{scheme}://***:***@{host}{port}{path}{query}{fragment}")
            } else {
                url.to_string()
            }
        }
        Err(_) => {
            // If URL parsing fails, return as-is but mask anything that looks like credentials
            // Pattern: protocol://user:pass@ or protocol://user@
            let mut result = url.to_string();
            // Simple heuristic: if it looks like credentials, mask them
            if let Some(at_pos) = result.find('@') {
                if let Some(proto_end) = result.find("://") {
                    if at_pos > proto_end {
                        // There's something between :// and @, likely credentials
                        let proto = &result[..proto_end + 3];
                        let rest = &result[at_pos + 1..];
                        result = format!("{proto}***:***@{rest}");
                    }
                }
            }
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // URL Sanitization Tests (src-1rf: Secrets may leak in error output)
    // -------------------------------------------------------------------------

    #[test]
    fn sanitize_url_removes_password() {
        let url = "https://user:secret@example.com/api";
        let result = sanitize_url_for_logging(url);
        assert_eq!(result, "https://***:***@example.com/api");
        assert!(!result.contains("secret"));
    }

    #[test]
    fn sanitize_url_removes_username_only() {
        let url = "https://admin@example.com/api";
        let result = sanitize_url_for_logging(url);
        // We show ***:***@ for any URL with credentials (even if only username)
        assert_eq!(result, "https://***:***@example.com/api");
        assert!(!result.contains("admin"));
    }

    #[test]
    fn sanitize_url_preserves_url_without_credentials() {
        let url = "https://example.com/api?query=value";
        let result = sanitize_url_for_logging(url);
        assert_eq!(result, url);
    }

    #[test]
    fn sanitize_url_preserves_port() {
        let url = "https://user:pass@example.com:8443/api";
        let result = sanitize_url_for_logging(url);
        assert_eq!(result, "https://***:***@example.com:8443/api");
    }

    #[test]
    fn sanitize_url_preserves_path_and_query() {
        let url = "https://user:pass@example.com/path?query=value#fragment";
        let result = sanitize_url_for_logging(url);
        assert_eq!(result, "https://***:***@example.com/path?query=value#fragment");
    }

    #[test]
    fn sanitize_url_handles_http() {
        let url = "http://user:pass@localhost:8080/endpoint";
        let result = sanitize_url_for_logging(url);
        assert_eq!(result, "http://***:***@localhost:8080/endpoint");
    }

    #[test]
    fn sanitize_url_handles_malformed_url_gracefully() {
        // Malformed URL should not panic, should return safely
        let url = "not-a-valid-url";
        let result = sanitize_url_for_logging(url);
        assert_eq!(result, "not-a-valid-url");
    }

    #[test]
    fn sanitize_url_handles_credential_like_string_in_malformed() {
        // If URL is malformed but contains something that looks like credentials
        let url = "https://user:pass@@invalid";
        let result = sanitize_url_for_logging(url);
        // Should mask the credential-like part
        assert!(!result.contains("pass") || result.contains("***"));
    }

    #[test]
    fn sanitize_url_handles_empty_string() {
        let result = sanitize_url_for_logging("");
        assert_eq!(result, "");
    }

    #[test]
    fn sanitize_url_special_chars_in_password() {
        let url = "https://user:p@ss:w0rd@example.com/api";
        let result = sanitize_url_for_logging(url);
        assert_eq!(result, "https://***:***@example.com/api");
        assert!(!result.contains("w0rd"));
    }

    // -------------------------------------------------------------------------

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
    fn truncate_clean_zero_max_chars_shows_truncated_marker() {
        // When max_chars=0, any non-empty input should show truncation marker
        // rather than silently returning empty string
        let input = "Some content here";
        let result = truncate_clean(input, 0);
        assert!(
            result.contains("truncated"),
            "Expected truncation marker for max_chars=0, got: {:?}",
            result
        );
    }

    #[test]
    fn truncate_clean_zero_max_chars_empty_input_returns_empty() {
        // When both input is empty and max_chars=0, empty string is correct
        let result = truncate_clean("", 0);
        assert_eq!(result, "");
    }

    #[test]
    fn duration_ms_display() {
        assert_eq!(format!("{}", DurationMs(500)), "500ms");
        assert_eq!(format!("{}", DurationMs(1500)), "1.5s");
        assert_eq!(format!("{}", DurationMs(61000)), "61.0s");
    }
}
