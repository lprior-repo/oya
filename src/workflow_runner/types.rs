use std::path::PathBuf;

use oya::config;

use super::{RunArgs, RunIdMode, PIPELINE_STAGES};

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
        let run_id = resolve_run_id(&args);
        let restate_ingress = args.restate_url.trim_end_matches('/').to_string();
        let restate_admin = restate_ingress.replace(":8080", ":9070");
        let model = args.model.unwrap_or_else(|| oya_config.model.clone());
        Self {
            run_id,
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

fn resolve_run_id(args: &RunArgs) -> String {
    match args.run_id_mode {
        RunIdMode::Bead => args.bead_id.clone(),
        RunIdMode::Unique => unique_run_id(args.bead_id.as_str()),
    }
}

fn unique_run_id(bead_id: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    format!("{}-{}", bead_id, timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_args(model: Option<&str>) -> RunArgs {
        RunArgs {
            bead_id: "src-36h".to_string(),
            restate_url: "http://127.0.0.1:8080".to_string(),
            context: "ctx".to_string(),
            timeout: 3600,
            poll_interval: Some(5),
            model: model.map(str::to_string),
            run_id_mode: RunIdMode::Bead,
        }
    }

    #[test]
    fn workflow_config_cli_model_overrides_config_model() {
        let args = sample_args(Some("cli/model"));
        let repo_root = PathBuf::from("/tmp/repo");
        let oya_config = config::OyaConfig { model: "config/model".to_string() };

        let cfg = WorkflowConfig::from_args(args, repo_root, &oya_config);

        assert_eq!(cfg.model, "cli/model");
    }

    #[test]
    fn workflow_config_uses_config_model_when_cli_missing() {
        let args = sample_args(None);
        let repo_root = PathBuf::from("/tmp/repo");
        let oya_config = config::OyaConfig { model: "config/model".to_string() };

        let cfg = WorkflowConfig::from_args(args, repo_root, &oya_config);

        assert_eq!(cfg.model, "config/model");
    }

    #[test]
    fn workflow_config_unique_mode_generates_prefixed_run_id() {
        let mut args = sample_args(None);
        args.run_id_mode = RunIdMode::Unique;
        let repo_root = PathBuf::from("/tmp/repo");
        let oya_config = config::OyaConfig { model: "config/model".to_string() };

        let cfg = WorkflowConfig::from_args(args, repo_root, &oya_config);

        assert!(cfg.run_id.starts_with("src-36h-"));
    }

    #[test]
    fn workflow_status_is_complete_requires_terminal_orchestration_status() {
        let completed_running = WorkflowStatus {
            status: "completed".to_string(),
            stage: "explore".to_string(),
            attempt: 1,
            orchestration_status: "running".to_string(),
            last_failure: String::new(),
        };
        let completed_shipped = WorkflowStatus {
            status: "completed".to_string(),
            stage: "ship_gate".to_string(),
            attempt: 1,
            orchestration_status: "shipped".to_string(),
            last_failure: String::new(),
        };

        assert!(!completed_running.is_complete());
        assert!(completed_shipped.is_complete());
    }

    #[test]
    fn workflow_status_is_failed_includes_orchestration_failure() {
        let invocation_completed_but_failed = WorkflowStatus {
            status: "completed".to_string(),
            stage: "red".to_string(),
            attempt: 2,
            orchestration_status: "failed".to_string(),
            last_failure: "gate failed".to_string(),
        };
        let invocation_failed = WorkflowStatus {
            status: "failed".to_string(),
            stage: "contract".to_string(),
            attempt: 1,
            orchestration_status: "running".to_string(),
            last_failure: "invoke failed".to_string(),
        };

        assert!(invocation_completed_but_failed.is_failed());
        assert!(invocation_failed.is_failed());
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
        let state_str = state_outer.as_str().unwrap_or("{}");
        let state: serde_json::Value =
            serde_json::from_str(state_str).map_err(|e| format!("Invalid state string: {}", e))?;

        Ok(Self {
            status,
            stage: state.get("stage").and_then(|s| s.as_str()).unwrap_or("unknown").to_string(),
            attempt: state.get("attempt").and_then(|a| a.as_u64()).unwrap_or(0) as u32,
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
        self.status == "completed" && self.orchestration_status != "running"
    }

    pub(super) fn is_failed(&self) -> bool {
        self.status == "failed" || self.orchestration_status == "failed"
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
