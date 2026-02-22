//! Parse Restate invocation data into enriched view models.
//! Boundary: converts untrusted JSON into trusted domain types.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use super::types::{
    Attempt, GateName, GateState, GateView, InvocationState, InvocationView, RestateInvocationRow,
    RunId, StageName,
};
use chrono::{DateTime, Utc};
use itertools::Itertools;

/// Parse a Restate invocation row into an enriched view (boundary function).
/// Converts untrusted external data into trusted domain types.
pub fn parse_invocation(row: &RestateInvocationRow) -> InvocationView {
    let run_id = RunId::new_or_fallback(row.target_service_key.clone());

    let state = InvocationState::from_restate_status(&row.status, row.completion_result.as_deref());

    let parsed_failure = parse_completion_failure(row.completion_failure.as_deref());

    let age_seconds = calculate_age_seconds(&row.modified_at);

    InvocationView {
        run_id,
        state,
        stage: parsed_failure.stage,
        attempt: parsed_failure.attempt,
        gates: parsed_failure.gates,
        last_output_lines: parsed_failure.last_output_lines,
        age_seconds,
    }
}

struct ParsedFailure {
    stage: Option<StageName>,
    attempt: Option<Attempt>,
    gates: Vec<GateView>,
    last_output_lines: Vec<String>,
}

/// Parse the nested JSON from completion_failure field (boundary).
/// Returns: (stage, attempt, gates, output_lines)
fn parse_completion_failure(failure: Option<&str>) -> ParsedFailure {
    let Some(failure_str) = failure else {
        return ParsedFailure {
            stage: None,
            attempt: None,
            gates: Vec::new(),
            last_output_lines: Vec::new(),
        };
    };

    // Try to parse as JSON
    let Ok(json) = serde_json::from_str::<serde_json::Value>(failure_str) else {
        return ParsedFailure {
            stage: None,
            attempt: None,
            gates: Vec::new(),
            last_output_lines: Vec::new(),
        };
    };

    // Extract stage from the mismatch message if present
    let stage = extract_stage_from_failure(&json);

    // Extract attempt with proper invariant enforcement
    let max_attempts = stage.as_ref().map_or(2, |s| s.max_attempts());
    let attempt = extract_attempt(&json, max_attempts);

    // Extract gates from the failure
    let gates = extract_gates(&json);

    // Extract last output lines
    let last_output = extract_output_lines(&json);

    ParsedFailure { stage, attempt, gates, last_output_lines: last_output }
}

fn extract_stage_from_failure(json: &serde_json::Value) -> Option<StageName> {
    json.as_object()
        .and_then(|obj| obj.get("stage"))
        .and_then(|s| s.as_str())
        .map(|s| StageName::new(s.to_string()))
}

/// Extract attempt with type-safe invariant: 1 <= attempt <= max_attempts.
fn extract_attempt(json: &serde_json::Value, max_attempts: u32) -> Option<Attempt> {
    json.as_object()
        .and_then(|obj| obj.get("attempt"))
        .and_then(|a| a.as_u64())
        .map(|v| v as u32)
        .and_then(|v| Attempt::new(v, max_attempts))
        .or_else(|| Some(Attempt::first()))
}

fn extract_gates(json: &serde_json::Value) -> Vec<GateView> {
    let mut gates = Vec::new();

    if let Some(obj) = json.as_object() {
        // Look for gates array
        if let Some(gates_arr) = obj.get("gates").and_then(|g| g.as_array()) {
            for gate in gates_arr {
                if let (Some(name), Some(passed)) = (
                    gate.get("gate").and_then(|g| g.as_str()),
                    gate.get("passed").and_then(|p| p.as_bool()),
                ) {
                    gates.push(GateView {
                        name: GateName::new(name.to_string()),
                        state: if passed { GateState::Passed } else { GateState::Failed },
                    });
                }
            }
        }

        // Also check for gate results in output parsing
        if let Some(output) = obj.get("output").and_then(|o| o.as_str()) {
            gates.extend(parse_gates_from_output(output));
        }
    }

    gates
}

fn parse_gates_from_output(output: &str) -> Vec<GateView> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.contains('\u{2705}') || trimmed.contains("PASS") {
                extract_gate_name(trimmed).map(|name| GateView { name, state: GateState::Passed })
            } else if trimmed.contains('\u{274c}') || trimmed.contains("FAIL") {
                extract_gate_name(trimmed).map(|name| GateView { name, state: GateState::Failed })
            } else if trimmed.contains('\u{231b}') || trimmed.contains("running") {
                extract_gate_name(trimmed).map(|name| GateView { name, state: GateState::Running })
            } else {
                None
            }
        })
        .collect()
}

fn extract_gate_name(line: &str) -> Option<GateName> {
    // Extract gate name from lines like "moon:check" or "oya:check (cached)"
    let cleaned = line.replace(['\u{2705}', '\u{274c}', '\u{231b}'], "").trim().to_string();

    // Take first word/segment as gate name
    cleaned
        .split_whitespace()
        .next()
        .and_then(|s| s.split_once(':').map(|(base, _)| base).or(Some(s)))
        .map(|s| GateName::new(s.to_string()))
}

fn extract_output_lines(json: &serde_json::Value) -> Vec<String> {
    let output = json.as_object().and_then(|obj| obj.get("output")).and_then(|o| o.as_str());

    if let Some(output) = output {
        output
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .collect_vec()
            .into_iter()
            .rev()
            .take(10)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    } else {
        Vec::new()
    }
}

fn calculate_age_seconds(modified_at: &str) -> u64 {
    DateTime::parse_from_rfc3339(modified_at).map_or(0, |parsed| {
        let now = Utc::now();
        (now - parsed.with_timezone(&Utc)).num_seconds().max(0) as u64
    })
}

/// Format age in human-readable form.
pub fn format_age(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}h", seconds / 3600)
    }
}

/// Format duration in human-readable form.
pub fn format_duration(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_age() {
        assert_eq!(format_age(30), "30s");
        assert_eq!(format_age(90), "1m");
        assert_eq!(format_age(3661), "1h");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(125), "2m 5s");
    }

    #[test]
    fn test_extract_gate_name() {
        assert_eq!(extract_gate_name("moon:check"), Some(GateName::new("moon".to_string())));
        assert_eq!(extract_gate_name("zjj:sync (cached)"), Some(GateName::new("zjj".to_string())));
    }

    #[test]
    fn test_attempt_invariant() {
        // Valid attempts
        assert!(Attempt::new(1, 3).is_some());
        assert!(Attempt::new(2, 3).is_some());
        assert!(Attempt::new(3, 3).is_some());

        // Invalid attempts - violates invariant
        assert!(Attempt::new(0, 3).is_none()); // Too low
        assert!(Attempt::new(4, 3).is_none()); // Too high
    }

    #[test]
    fn test_invocation_state_is_exhaustive() {
        // Ensure all state transitions are explicit
        use InvocationState::*;

        let state = Running;
        assert!(state.is_running());
        assert!(!state.is_terminal());

        let state = CompletedSuccess;
        assert!(!state.is_running());
        assert!(state.is_terminal());

        let state = CompletedFailure;
        assert!(!state.is_running());
        assert!(state.is_terminal());
    }

    #[test]
    fn test_gate_state_icons() {
        assert_eq!(GateState::Passed.icon(), '\u{2705}');
        assert_eq!(GateState::Failed.icon(), '\u{274c}');
        assert_eq!(GateState::Running.icon(), '\u{231b}');
    }
}
