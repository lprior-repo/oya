//! Stage command implementation
//!
//! Runs a specific pipeline stage for a task.

use anyhow::Result;
use clap::Parser;
use oya_pipeline::resolve_stage_range;
use oya_pipeline::{load_task_record, save_task_record, Slug};
use std::path::PathBuf;

/// Arguments for the stage command
#[derive(Parser, Debug, Clone)]
pub struct StageArgs {
    /// Slug for the task
    #[arg(short, long)]
    pub slug: String,

    /// Stage name to run
    #[arg(short, long)]
    pub stage: String,

    /// Optional start stage for range (for run_full_pipeline)
    #[arg(long)]
    pub from: Option<String>,

    /// Optional end stage for range (for run_full_pipeline)
    #[arg(long)]
    pub to: Option<String>,

    /// Dry run (validate but don't persist)
    #[arg(long)]
    pub dry_run: bool,

    /// Repository root path (default: current directory)
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// Output from the stage command
#[derive(Debug, Clone)]
pub struct StageOutput {
    /// Updated task
    pub task: oya_pipeline::Task,
    /// Stage run report
    pub report: String,
}

/// Run a specific pipeline stage for a task
///
/// # Errors
/// Returns error if task cannot be found, stage is invalid, or transition fails
pub async fn stage_command(args: StageArgs) -> Result<StageOutput> {
    let root = args.root.unwrap_or_else(|| PathBuf::from("."));
    let _slug = Slug::new(&args.slug)?;

    // Load existing task
    let task = load_task_record(args.slug.as_str(), &root).await?;

    // Resolve stage range
    let stages = resolve_stage_range(&args.stage, args.from.as_deref(), args.to.as_deref())?;

    // Build stage plan for the task
    let task = oya_pipeline::apply_stage_plan(task, &stages)?;

    // Persist if not dry run
    if !args.dry_run {
        save_task_record(&task, &root).await?;
    }

    let stages_str: Vec<String> = stages.iter().map(|s| s.as_str().to_string()).collect();

    Ok(StageOutput {
        task,
        report: if args.dry_run {
            format!("[DRY RUN] Would execute stages: {}", stages_str.join(", "))
        } else {
            format!("Executed stages: {}", stages_str.join(", "))
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oya_pipeline::{Language, Task, TaskStatus};

    #[tokio::test]
    async fn stage_command_runs_stage() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let slug = oya_pipeline::Slug::new("task-1")?;
        let task = Task::new(slug, Language::Rust);

        oya_pipeline::save_task_record(&task, temp_dir.path()).await?;

        let args = StageArgs {
            slug: "task-1".to_string(),
            stage: "implement".to_string(),
            from: None,
            to: None,
            dry_run: false,
            root: Some(temp_dir.path().to_path_buf()),
        };

        let result = stage_command(args).await?;

        assert_eq!(result.task.slug().as_str(), "task-1");
        assert!(matches!(result.task.status, TaskStatus::InProgress { .. }));
        Ok(())
    }

    #[tokio::test]
    async fn stage_command_rejects_invalid_stage() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let slug = oya_pipeline::Slug::new("task-1")?;
        let task = Task::new(slug, Language::Rust);

        oya_pipeline::save_task_record(&task, temp_dir.path()).await?;

        let args = StageArgs {
            slug: "task-1".to_string(),
            stage: "invalid_stage".to_string(),
            from: None,
            to: None,
            dry_run: false,
            root: Some(temp_dir.path().to_path_buf()),
        };

        let result = stage_command(args).await;

        assert!(result.is_err());
        Ok(())
    }
}
