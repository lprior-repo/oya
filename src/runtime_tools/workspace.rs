use super::super::*;
use super::jj;
use oya::types::{derive_merge_decision, QueuePosition};
use std::path::PathBuf;

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct WorkspacePrepRequest {
    pub(crate) run_id: String,
    pub(crate) bead_id: String,
    pub(crate) stage: Stage,
    pub(crate) attempt: u32,
    pub(crate) recorded_at: String,
    pub(crate) repo_root: PathBuf,
}

pub(crate) fn prepare_stage_workspace(
    request: WorkspacePrepRequest,
) -> Result<Option<WorkspaceLifecycleEvent>, OyaError> {
    let workspace_info = jj::create_workspace(&request.bead_id, &request.repo_root)?;
    let coordination = build_coordination()?;
    Ok(Some(WorkspaceLifecycleEvent {
        workspace_name: workspace_info.workspace_name,
        workspace_path: workspace_info.workspace_path,
        coordination,
        timestamp: request.recorded_at.clone(),
    }))
}

fn build_coordination() -> Result<WorkspaceCoordination, OyaError> {
    let queue_position =
        QueuePosition::try_from(1u32).map_err(|e| OyaError(format!("queue position: {e}")))?;
    let merge_decision = derive_merge_decision(queue_position, None, true);
    Ok(WorkspaceCoordination { queue_position, merge_decision })
}
