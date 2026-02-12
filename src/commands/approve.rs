//! Approve command implementation
//!
//! Approves a task for integration.

use anyhow::Result;
use clap::Parser;
use oya_pipeline::{Slug, approve_task, load_task_record};
use std::path::PathBuf;

/// Arguments for the approve command
#[derive(Parser, Debug, Clone)]
pub struct ApproveArgs {
    /// Slug for the task
    #[arg(short, long)]
    pub slug: String,

    /// Force approval even if pipeline not passed
    #[arg(long)]
    pub force: bool,

    /// Repository root path (default: current directory)
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// Output from the approve command
#[derive(Debug, Clone)]
pub struct ApproveOutput {
    /// Approved task
    pub task: oya_pipeline::Task,
}

/// Approve a task for integration
///
/// # Errors
/// Returns error if task cannot be found, is not eligible for integration, or force flag is not set
pub async fn approve_command(args: ApproveArgs) -> Result<ApproveOutput> {
    let root = args.root.unwrap_or_else(|| PathBuf::from("."));
    let _slug = Slug::new(&args.slug)?;

    // Load existing task
    let task = load_task_record(args.slug.as_str(), &root).await?;

    // Approve task for integration
    let task = approve_task(task, args.force)?;

    // Persist approval
    oya_pipeline::save_task_record(&task, &root).await?;

    Ok(ApproveOutput { task })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oya_pipeline::{Language, Task};

    #[tokio::test]
    async fn approve_command_rejects_non_eligible_task() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let slug = oya_pipeline::Slug::new("task-1").expect("slug should parse");
        let task = Task::new(slug, Language::Rust);

        oya_pipeline::save_task_record(&task, temp_dir.path())
            .await
            .expect("save should succeed");

        let args = ApproveArgs {
            slug: "task-1".to_string(),
            force: false,
            root: Some(temp_dir.path().to_path_buf()),
        };

        let result = approve_command(args).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn approve_command_approves_passed_pipeline_task() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let slug = oya_pipeline::Slug::new("task-1").expect("slug should parse");
        let task = Task::new(slug, Language::Rust);

        oya_pipeline::save_task_record(&task, temp_dir.path())
            .await
            .expect("save should succeed");

        let args = ApproveArgs {
            slug: "task-1".to_string(),
            force: false,
            root: Some(temp_dir.path().to_path_buf()),
        };

        let result = approve_command(args).await;

        assert!(result.is_err());
    }
}
