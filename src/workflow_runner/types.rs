use std::path::PathBuf;

use oya::config;
use oya::types::normalize_attempt;

use super::{RunArgs, PIPELINE_STAGES};

#[derive(Debug, Clone)]
pub(super) struct WorkflowConfig {
    pub(super) bead_id: String,
    pub(super) run_id: String,
    pub(super) restate_ingress: String,
    pub(super) restate_admin: String,
    pub(super) context: String,
    pub(super) model: String,
    pub(super) timeout_secs: u64,
    pub(super) poll_interval_secs: u64,
    pub(super) repo_root: PathBuf,
    pub(super) stages: &'static [&'static str],
}

impl WorkflowConfig {
    pub(super) fn from_args(
        args: RunArgs,
        repo_root: PathBuf,
        oya_config: &config::OyaConfig,
    ) -> Self {
        let restate_ingress = args.restate_url.trim_end_matches('/').to_string();
        let restate_admin = restate_ingress.replace(":8080", ":9070");
        let model = args.model.unwrap_or_else(|| oya_config.model.clone());
        Self {
            run_id: args.bead_id.clone(),
            bead_id: args.bead_id,
            restate_ingress,
            restate_admin,
            context: args.context,
            model,
            timeout_secs: args.timeout,
            poll_interval_secs: args.poll_interval.unwrap_or(5),
            repo_root,
            stages: PIPELINE_STAGES,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct WorkflowStatus {
    pub(super) status: String,
    pub(super) stage: String,
    pub(super) attempt: u32,
    pub(super) orchestration_status: String,
    pub(super) last_failure: String,
}

impl WorkflowStatus {
    pub(super) fn from_query_response(body: &str) -> Result<Self, String> {
        let response: serde_json::Value =
            serde_json::from_str(body).map_err(|e| format!("Invalid JSON response: {}", e))?;
        let rows = response
            .get("rows")
            .ok_or("Missing 'rows' field in response")?
            .as_array()
            .ok_or("'rows' field is not an array")?;
        let row = rows.first().ok_or("No rows in response")?;
        let status = row
            .get("status")
            .and_then(|s| s.as_str())
            .ok_or("Missing or invalid 'status' field")?
            .to_string();

        let state_json_str = row.get("state_json").and_then(|s| s.as_str()).unwrap_or("{}");
        let state_outer: serde_json::Value = serde_json::from_str(state_json_str)
            .map_err(|e| format!("Invalid state_json: {}", e))?;
        let state_str = state_outer.as_str().ok_or("state_json is not a string")?;
        let state: serde_json::Value =
            serde_json::from_str(state_str).map_err(|e| format!("Invalid state string: {}", e))?;

        // Normalize attempt: 0 or missing becomes 1 (first attempt)
        let raw_attempt = state.get("attempt").and_then(|a| a.as_u64()).unwrap_or(0) as u32;
        let attempt = normalize_attempt(raw_attempt);

        Ok(Self {
            status,
            stage: state.get("stage").and_then(|s| s.as_str()).unwrap_or("unknown").to_string(),
            attempt,
            orchestration_status: state
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown")
                .to_string(),
            last_failure: state
                .get("last_failure")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }

    pub(super) fn is_complete(&self) -> bool {
        self.status == "completed"
    }

    pub(super) fn is_failed(&self) -> bool {
        self.status == "failed"
    }
}

#[derive(Debug, Clone)]
pub(super) struct WorkflowResult {
    pub(super) bead_id: String,
    pub(super) run_id: String,
    pub(super) status: String,
    pub(super) final_stage: String,
    pub(super) error: Option<String>,
    pub(super) repo_root: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_query_response_returns_error_for_malformed_state_json() {
        let body = r#"{"rows":[{"status":"running","state_json":"NOT VALID JSON"}]}"#;
        match WorkflowStatus::from_query_response(body) {
            Err(err) => {
                assert!(err.contains("state_json"), "Error should mention state_json: {err}")
            }
            Ok(status) => panic!("Expected error for malformed state_json, got: {status:?}"),
        }
    }

    #[test]
    fn from_query_response_returns_error_for_non_string_state_outer() {
        // state_json contains valid JSON but not a string (should be a string that contains more JSON)
        let body = r#"{"rows":[{"status":"running","state_json":"{\"not\": \"a string, but\"}"}]}"#;
        match WorkflowStatus::from_query_response(body) {
            Err(err) => assert!(
                err.contains("state_json") || err.contains("string"),
                "Error should mention state_json or string: {err}"
            ),
            Ok(status) => {
                panic!("Expected error when state_outer is not a string, got: {status:?}")
            }
        }
    }

    #[test]
    fn from_query_response_returns_error_for_malformed_inner_state_string() {
        // state_json is a string but contains invalid JSON
        let body = r#"{"rows":[{"status":"running","state_json":"\"{not: valid json}\""}]}"#;
        match WorkflowStatus::from_query_response(body) {
            Err(err) => assert!(err.contains("state"), "Error should mention state: {err}"),
            Ok(status) => panic!("Expected error for malformed inner state, got: {status:?}"),
        }
    }

    #[test]
    fn from_query_response_parses_valid_response() {
        let body = r#"{"rows":[{"status":"running","state_json":"\"{\\\"stage\\\": \\\"tdd15\\\", \\\"attempt\\\": 1, \\\"status\\\": \\\"running\\\", \\\"last_failure\\\": \\\"\\\"}\""}]}"#;
        match WorkflowStatus::from_query_response(body) {
            Ok(status) => {
                assert_eq!(status.status, "running");
                assert_eq!(status.stage, "tdd15");
                assert_eq!(status.attempt, 1);
            }
            Err(err) => panic!("Expected Ok for valid response, got error: {err}"),
        }
    }

    #[test]
    fn from_query_response_returns_error_for_missing_rows() {
        let body = r#"{"not_rows":[]}"#;
        match WorkflowStatus::from_query_response(body) {
            Err(_) => {}
            Ok(status) => panic!("Expected error for missing rows, got: {status:?}"),
        }
    }

    #[test]
    fn from_query_response_returns_error_for_empty_rows() {
        let body = r#"{"rows":[]}"#;
        match WorkflowStatus::from_query_response(body) {
            Err(_) => {}
            Ok(status) => panic!("Expected error for empty rows, got: {status:?}"),
        }
    }

    #[test]
    fn from_query_response_returns_error_for_missing_status() {
        let body = r#"{"rows":[{}]}"#;
        match WorkflowStatus::from_query_response(body) {
            Err(_) => {}
            Ok(status) => panic!("Expected error for missing status, got: {status:?}"),
        }
    }

    /// Test: attempt=0 should be normalized to 1 (default to first attempt)
    /// This addresses the bug where attempt=0 is not validated consistently.
    #[test]
    fn from_query_response_normalizes_attempt_zero_to_one() {
        // When attempt is 0 in the state, it should be normalized to 1
        let body = r#"{"rows":[{"status":"running","state_json":"\"{\\\"stage\\\": \\\"tdd15\\\", \\\"attempt\\\": 0, \\\"status\\\": \\\"running\\\", \\\"last_failure\\\": \\\"\\\"}\""}]}"#;
        match WorkflowStatus::from_query_response(body) {
            Ok(status) => {
                // CRITICAL: attempt=0 is invalid, must be normalized to 1
                assert_eq!(status.attempt, 1, "attempt=0 must be normalized to 1");
            }
            Err(err) => panic!("Expected Ok with normalized attempt, got error: {err}"),
        }
    }

    /// Test: missing attempt should default to 1 (not 0)
    #[test]
    fn from_query_response_defaults_missing_attempt_to_one() {
        // When attempt is missing from state, it should default to 1
        let body = r#"{"rows":[{"status":"running","state_json":"\"{\\\"stage\\\": \\\"tdd15\\\", \\\"status\\\": \\\"running\\\", \\\"last_failure\\\": \\\"\\\"}\""}]}"#;
        match WorkflowStatus::from_query_response(body) {
            Ok(status) => {
                // CRITICAL: missing attempt must default to 1 (first attempt)
                assert_eq!(status.attempt, 1, "missing attempt must default to 1");
            }
            Err(err) => panic!("Expected Ok with default attempt=1, got error: {err}"),
        }
    }

    /// Test: normalize_attempt helper enforces attempt >= 1 invariant
    #[test]
    fn normalize_attempt_converts_zero_to_one() {
        assert_eq!(normalize_attempt(0), 1, "attempt=0 must become 1");
    }

    #[test]
    fn normalize_attempt_preserves_valid_attempts() {
        assert_eq!(normalize_attempt(1), 1);
        assert_eq!(normalize_attempt(2), 2);
        assert_eq!(normalize_attempt(100), 100);
    }
}
