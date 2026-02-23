use super::super::*;
use super::command_exec::{combine_command_output, run_command_with_timeout_with_exit};
use oya::types::{
    derive_merge_decision, parse_queue_record, select_next_merge_candidate, FullSha, MergeDecision,
    QueuePosition, SelectionDecision, ValidationError,
};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub(crate) struct WorkspacePrepRequest {
    pub(crate) run_id: String,
    pub(crate) bead_id: String,
    pub(crate) stage: Stage,
    pub(crate) attempt: u32,
    pub(crate) recorded_at: String,
    pub(crate) repo_root: PathBuf,
}

/// Triple of (passed, combined_output, exit_code) for a zjj sub-command.
type CommandResult = (bool, String, i32);

pub(crate) fn prepare_stage_workspace(
    request: WorkspacePrepRequest,
) -> Result<Option<WorkspaceLifecycleEvent>, OyaError> {
    let queue_result = run_zjj_queue(&request)?;
    let add_result = run_zjj_add(&request)?;
    let coordination = build_coordination(&request.bead_id)?;
    let workspace_path = resolve_workspace_path(&request.repo_root, &request.bead_id);
    Ok(Some(build_lifecycle_event(
        &request,
        queue_result,
        add_result,
        coordination,
        workspace_path,
    )))
}

fn run_zjj_queue(request: &WorkspacePrepRequest) -> Result<CommandResult, OyaError> {
    let bead = request.bead_id.as_str();
    let args = ["queue", "--add", bead, "--bead", bead];
    let (passed, stdout, stderr, exit_code) =
        run_command_with_timeout_with_exit("zjj", &args, 60, &request.repo_root)?;
    Ok((passed, combine_command_output(stdout, stderr), exit_code))
}

fn run_zjj_add(request: &WorkspacePrepRequest) -> Result<CommandResult, OyaError> {
    let bead = request.bead_id.as_str();
    let args = ["add", bead];
    let (passed, stdout, stderr, exit_code) =
        run_command_with_timeout_with_exit("zjj", &args, 60, &request.repo_root)?;
    Ok((passed, combine_command_output(stdout, stderr), exit_code))
}

fn build_coordination(bead_id: &str) -> Result<WorkspaceCoordination, OyaError> {
    const PLACEHOLDER_SHA: &str = "0000000000000000000000000000000000000000";
    let queue_item = parse_queue_record(bead_id, bead_id, 5, PLACEHOLDER_SHA, PLACEHOLDER_SHA)
        .map_err(|e| OyaError(format!("parse_queue_record: {e}")))?;
    let main_rev =
        FullSha::try_from(PLACEHOLDER_SHA).map_err(|e| OyaError(format!("main_rev: {e}")))?;
    let decision = select_next_merge_candidate(&[queue_item], None, 0, &main_rev)
        .map_err(|e| OyaError(format!("select_next_merge_candidate: {e}")))?;
    let queue_position = QueuePosition::try_from(1u32)
        .map_err(|e: ValidationError| OyaError(format!("queue position: {e}")))?;
    let merge_decision = merge_decision_from_selection(&decision, queue_position);
    Ok(WorkspaceCoordination { queue_position, merge_decision })
}

fn merge_decision_from_selection(
    decision: &SelectionDecision,
    queue_position: QueuePosition,
) -> MergeDecision {
    match decision {
        SelectionDecision::Ready { .. } => derive_merge_decision(queue_position, None, true),
        _ => derive_merge_decision(queue_position, None, false),
    }
}

fn resolve_workspace_path(repo_root: &Path, bead_id: &str) -> String {
    repo_root.join(".zjj").join("workspaces").join(bead_id).display().to_string()
}

fn build_lifecycle_event(
    request: &WorkspacePrepRequest,
    queue_result: CommandResult,
    add_result: CommandResult,
    coordination: WorkspaceCoordination,
    workspace_path: String,
) -> WorkspaceLifecycleEvent {
    let bead = &request.bead_id;
    let stage_name = request.stage.as_str();
    let attempt = request.attempt;
    let run_id = &request.run_id;
    let queue_cmd = format!(
        "zjj queue --add {bead} --bead {bead}  # run={run_id} stage={stage_name} attempt={attempt}"
    );
    let add_cmd = format!("zjj add {bead}");
    WorkspaceLifecycleEvent {
        workspace: bead.clone(),
        workspace_path,
        queue_command: queue_cmd,
        queue_passed: queue_result.0,
        queue_exit_code: queue_result.2,
        queue_output: queue_result.1,
        add_command: add_cmd,
        add_passed: add_result.0,
        add_exit_code: add_result.2,
        add_output: add_result.1,
        coordination,
        recorded_at: request.recorded_at.clone(),
    }
}
