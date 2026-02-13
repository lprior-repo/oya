//! Show command implementation
//!
//! Displays detailed information about a specific task.

use anyhow::Result;
use clap::Parser;
use oya_pipeline::{Task, load_task_record};
use std::path::PathBuf;

/// Arguments for the show command
#[derive(Parser, Debug, Clone)]
pub struct ShowArgs {
    /// Task slug to show
    #[arg(short, long)]
    pub slug: String,

    /// Format output as JSON
    #[arg(long)]
    pub json: bool,

    /// Repository root path (default: current directory)
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// Output from the show command
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShowOutput {
    /// Task details
    pub task: Task,
}

/// Show details of a specific task
///
/// # Errors
/// Returns error if task cannot be found or parsed
pub async fn show_command(args: ShowArgs) -> Result<ShowOutput> {
    let root = args.root.unwrap_or_else(|| PathBuf::from("."));
    let task = load_task_record(args.slug.as_str(), &root).await?;

    let output = ShowOutput { task };

    if args.json {
        let json = serde_json::to_string_pretty(&output)?;
        println!("{}", json);
    } else {
        println!("Task: {}", output.task.slug());
        println!("  Status: {}", output.task.status);
        println!("  Language: {}", output.task.language());
        println!("  Priority: {}", output.task.priority());
        println!("  Branch: {}", output.task.branch());
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oya_pipeline::{Language, Task, TaskStatus};

    #[tokio::test]
    async fn show_command_finds_task() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let slug = oya_pipeline::Slug::new("task-1")?;
        let task = Task::new(slug, Language::Rust);

        oya_pipeline::save_task_record(&task, temp_dir.path())
            .await?;

        let args = ShowArgs {
            slug: "task-1".to_string(),
            json: false,
            root: Some(temp_dir.path().to_path_buf()),
        };

        let result = show_command(args).await?;

        assert_eq!(result.task.slug().as_str(), "task-1");
        assert_eq!(result.task.status, TaskStatus::Created);
        Ok(())
    }

    #[tokio::test]
    async fn show_command_rejects_missing_task() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let args = ShowArgs {
            slug: "missing-task".to_string(),
            json: false,
            root: Some(temp_dir.path().to_path_buf()),
        };

        let result = show_command(args).await;

        assert!(result.is_err());
        Ok(())
    }
}
