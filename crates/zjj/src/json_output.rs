//! JSON output structures for zjj commands

use serde::Serialize;

/// Init command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct InitOutput {
    pub success: bool,
    pub message: String,
    pub jjz_dir: String,
    pub config_file: String,
    pub state_db: String,
    pub layouts_dir: String,
}

/// Add command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct AddOutput {
    pub success: bool,
    pub session_name: String,
    pub workspace_path: String,
    pub zellij_tab: String,
    pub status: String,
}

/// Remove command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct RemoveOutput {
    pub success: bool,
    pub session_name: String,
    pub message: String,
}

/// Focus command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct FocusOutput {
    pub success: bool,
    pub session_name: String,
    pub zellij_tab: String,
    pub message: String,
}

/// Sync command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct SyncOutput {
    pub success: bool,
    pub session_name: Option<String>,
    pub synced_count: usize,
    pub failed_count: usize,
    pub errors: Vec<SyncError>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct SyncError {
    pub session_name: String,
    pub error: String,
}

/// Diff command JSON output
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct DiffOutput {
    pub session_name: String,
    pub base: String,
    pub head: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_stat: Option<DiffStat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DiffStat {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub files: Vec<FileDiffStat>,
}

#[derive(Debug, Serialize)]
pub struct FileDiffStat {
    pub path: String,
    pub insertions: usize,
    pub deletions: usize,
    pub status: String,
}
