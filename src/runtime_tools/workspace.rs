use super::super::*;
use super::command_exec::{combine_command_output, run_command_with_timeout_with_exit};
use crate::pipeline::WorkspacePreparationPolicy;
use oya::build_zjj_workspace_name;
use std::path::{Path, PathBuf};

const ZJJ_TIMEOUT_SECONDS: u64 = 60;

fn stage_uses_workspace(stage: &Stage) -> bool {
    matches!(stage, Stage::Contract | Stage::Implementation | Stage::ShipGate)
}

fn stage_requires_merge_queue(stage: &Stage) -> bool {
    matches!(stage, Stage::ShipGate)
}

#[derive(Clone)]
pub(crate) struct WorkspacePrepRequest {
    pub(crate) run_id: String,
    pub(crate) bead_id: String,
    pub(crate) stage: Stage,
    pub(crate) attempt: u32,
    pub(crate) recorded_at: String,
    pub(crate) workspace_policy: WorkspacePreparationPolicy,
    pub(crate) repo_root: PathBuf,
}

struct WorkspaceCommandResult {
    command: String,
    passed: bool,
    exit_code: i32,
    output: String,
}

fn ensure_workspace_name(request: &WorkspacePrepRequest) -> Result<String, OyaError> {
    build_zjj_workspace_name(request.run_id.as_str(), request.stage.as_str(), request.attempt)
        .map_err(|error| OyaError(format!("Invalid workspace name for stage prep: {}", error)))
}

fn queue_workspace(
    request: &WorkspacePrepRequest,
    workspace: &str,
) -> Result<WorkspaceCommandResult, OyaError> {
    if !stage_requires_merge_queue(&request.stage) {
        return Ok(WorkspaceCommandResult {
            command: "zjj queue --add <workspace> --bead <bead_id> (disabled)".to_string(),
            passed: true,
            exit_code: 0,
            output: "merge queue disabled in runtime".to_string(),
        });
    }
    let command = format!("zjj queue --add {} --bead {}", workspace, request.bead_id);
    let args = ["queue", "--add", workspace, "--bead", request.bead_id.as_str()];
    match run_workspace_command(request, &command, &args, "zjj queue", workspace) {
        Ok(result) => Ok(result),
        Err(error) if queue_duplicate_error(error.to_string().as_str()) => {
            Ok(WorkspaceCommandResult {
                command,
                passed: true,
                exit_code: 0,
                output: "workspace already queued; treating queue add as idempotent".to_string(),
            })
        }
        Err(error) => Err(error),
    }
}

fn queue_duplicate_error(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("already in the queue") || normalized.contains("already queued")
}

fn add_workspace(
    request: &WorkspacePrepRequest,
    workspace: &str,
) -> Result<WorkspaceCommandResult, OyaError> {
    let command = format!("zjj add {} --idempotent", workspace);
    let args = ["add", workspace, "--idempotent"];
    run_workspace_command(request, &command, &args, "zjj add", workspace)
}

fn resolve_workspace_path(repo_root: &Path, workspace: &str) -> String {
    let repo_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| "workspace".to_string(), |name| name.to_string());
    let parent = repo_root.parent().unwrap_or(repo_root);
    parent.join(format!("{}__workspaces", repo_name)).join(workspace).to_string_lossy().to_string()
}

fn run_workspace_command(
    request: &WorkspacePrepRequest,
    command: &str,
    args: &[&str],
    operation: &str,
    workspace: &str,
) -> Result<WorkspaceCommandResult, OyaError> {
    let (passed, stdout, stderr, exit_code) =
        run_command_with_timeout_with_exit("zjj", args, ZJJ_TIMEOUT_SECONDS, &request.repo_root)?;
    let output = combine_command_output(stdout, stderr);
    if !passed {
        return Err(OyaError(format!(
            "{} failed for workspace {} (exit={}): {}",
            operation,
            workspace,
            exit_code,
            truncate_clean(output.as_str(), 2000)
        )));
    }
    Ok(WorkspaceCommandResult { command: command.to_string(), passed, exit_code, output })
}

pub(crate) fn prepare_stage_workspace(
    request: WorkspacePrepRequest,
) -> Result<Option<WorkspaceLifecycleEvent>, OyaError> {
    if request.workspace_policy.should_skip() || !stage_uses_workspace(&request.stage) {
        return Ok(None);
    }
    let workspace = ensure_workspace_name(&request)?;
    let workspace_path = resolve_workspace_path(request.repo_root.as_path(), workspace.as_str());
    let queue = queue_workspace(&request, &workspace)?;
    let add = add_workspace(&request, &workspace)?;
    Ok(Some(WorkspaceLifecycleEvent {
        workspace,
        workspace_path,
        queue_command: queue.command,
        queue_passed: queue.passed,
        queue_exit_code: queue.exit_code,
        queue_output: truncate_clean(queue.output.as_str(), 4000),
        add_command: add.command,
        add_passed: add.passed,
        add_exit_code: add.exit_code,
        add_output: truncate_clean(add.output.as_str(), 4000),
        recorded_at: request.recorded_at,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_requires_merge_queue_only_for_ship_gate() {
        assert!(!stage_requires_merge_queue(&Stage::Contract));
        assert!(!stage_requires_merge_queue(&Stage::Implementation));
        assert!(stage_requires_merge_queue(&Stage::ShipGate));
    }

    #[test]
    fn queue_duplicate_error_detects_duplicate_workspace_message() {
        let message = "Error: Invalid configuration: Workspace 'foo' is already in the queue";
        assert!(queue_duplicate_error(message));
    }

    #[test]
    fn queue_duplicate_error_detects_already_queued_message() {
        let message = "workspace foo is already queued";
        assert!(queue_duplicate_error(message));
    }

    #[test]
    fn queue_duplicate_error_ignores_other_queue_errors() {
        let message = "Error: queue backend unavailable";
        assert!(!queue_duplicate_error(message));
    }
}
