//! Stage execution module - Language-specific pipeline stages.
//!
//! Routes stage execution to the appropriate language handler.

mod gleam;
mod go;
mod javascript;
mod python;
mod rust;

use std::path::Path;

use tracing::{error, info};

use crate::{domain::Language, error::Result};

/// Execute a pipeline stage for the given language.
pub fn execute_stage(stage_name: &str, language: Language, worktree_path: &Path) -> Result<()> {
    let lang_str = language.display_name();
    log_stage_start(stage_name, lang_str, worktree_path);

    let result = match language {
        Language::Go => go::execute_go_stage(stage_name, worktree_path),
        Language::Gleam => gleam::execute_gleam_stage(stage_name, worktree_path),
        Language::Rust => rust::execute_rust_stage(stage_name, worktree_path),
        Language::Python => python::execute_python_stage(stage_name, worktree_path),
        Language::Javascript => javascript::execute_javascript_stage(stage_name, worktree_path),
    };

    match &result {
        Ok(()) => log_stage_complete(stage_name, lang_str),
        Err(e) => log_stage_failed(stage_name, lang_str, &e.to_string()),
    }

    result
}

/// Dry run preview for a stage.
#[derive(Debug, Clone)]
pub struct StagePreview {
    pub name: String,
    pub command: String,
    pub estimated_duration: u64,
}

/// Get dry run preview for stages.
pub fn execute_stages_dry_run(
    stages: &[crate::domain::Stage],
    language: Language,
) -> Vec<StagePreview> {
    stages
        .iter()
        .map(|stage| get_stage_preview(&stage.name, language))
        .collect()
}

/// Get preview for a single stage.
fn get_stage_preview(stage_name: &str, language: Language) -> StagePreview {
    let (command, duration) = match language {
        Language::Go => go::get_go_preview(stage_name),
        Language::Gleam => gleam::get_gleam_preview(stage_name),
        Language::Rust => rust::get_rust_preview(stage_name),
        Language::Python => python::get_python_preview(stage_name),
        Language::Javascript => javascript::get_javascript_preview(stage_name),
    };

    StagePreview {
        name: stage_name.to_string(),
        command,
        estimated_duration: duration,
    }
}

fn log_stage_start(stage_name: &str, lang_str: &str, worktree_path: &Path) {
    info!(
        stage = stage_name,
        language = lang_str,
        path = ?worktree_path,
        "Stage starting"
    );
}

fn log_stage_complete(stage_name: &str, lang_str: &str) {
    info!(stage = stage_name, language = lang_str, "Stage completed");
}

fn log_stage_failed(stage_name: &str, lang_str: &str, error: &str) {
    error!(
        stage = stage_name,
        language = lang_str,
        error,
        "Stage failed"
    );
}
