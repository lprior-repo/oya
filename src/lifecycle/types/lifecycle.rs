use serde::{Deserialize, Serialize};

use super::{BeadId, BookmarkName, LifecycleError, PrInfo, WorkspaceName};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeadData {
    pub bead_id: BeadId,
    pub workspace: WorkspaceName,
    pub workspace_path: String,
    pub bookmark: BookmarkName,
}

impl BeadData {
    #[must_use]
    pub fn from_bead_id(bead_id: BeadId) -> Self {
        let workspace = WorkspaceName::from_bead_id(&bead_id);
        let bookmark = BookmarkName::from_bead_id(&bead_id);
        let workspace_path = workspace.workspace_path();
        Self { bead_id, workspace, workspace_path, bookmark }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleResult {
    pub bead: BeadData,
    pub pr: Option<PrInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Planned(BeadData),
    WorkspaceReady(BeadData),
    PrOpen { bead: BeadData, pr: PrInfo },
    Failed { bead: BeadData, error: LifecycleError },
    Completed(LifecycleResult),
}

impl Phase {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed(_) | Self::Failed { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleState {
    pub phase: Phase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CancelState {
    #[default]
    Active,
    CancelRequested,
}

impl CancelState {
    #[must_use]
    pub fn is_cancel_requested(self) -> bool {
        matches!(self, Self::CancelRequested)
    }
}
