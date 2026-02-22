use super::super::*;
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
    _request: WorkspacePrepRequest,
) -> Result<Option<WorkspaceLifecycleEvent>, OyaError> {
    Ok(None)
}
