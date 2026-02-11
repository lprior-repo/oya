//! List command implementation
//!
//! Displays all tasks in the workspace.

use anyhow::Result;
use clap::Parser;
use oya_pipeline::{Task, list_all_tasks};
use std::path::PathBuf;

/// Arguments for the list command
#[derive(Parser, Debug, Clone)]
pub struct ListArgs {
    /// Format output as JSON
    #[arg(long)]
    pub json: bool,

    /// Repository root path (default: current directory)
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// Output from the list command
#[derive(Debug, Clone, serde::Serialize)]
pub struct ListOutput {
    /// List of tasks
    pub tasks: Vec<Task>,
    /// Total count
    pub total: usize,
}

/// List all tasks in the workspace
///
/// # Errors
/// Returns error if tasks file cannot be read or parsed
pub async fn list_command(args: ListArgs) -> Result<ListOutput> {
    let root = args.root.unwrap_or_else(|| PathBuf::from("."));
    let tasks = list_all_tasks(&root).await?;
    let total = tasks.len();

    let output = ListOutput { tasks, total };

    if args.json {
        let json = serde_json::to_string_pretty(&output)?;
        println!("{}", json);
    } else {
        println!("Tasks ({total} total):");
        for task in &output.tasks {
            println!(
                "  - {} [{}] {} (priority: {}, branch: {})",
                task.slug, task.status, task.language, task.priority, task.branch
            );
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oya_pipeline::{Language, Task};

    #[tokio::test]
    async fn list_command_returns_tasks() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let slug = oya_pipeline::Slug::new("task-1").expect("slug should parse");
        let task = Task::new(slug, Language::Rust);

        oya_pipeline::save_task_record(&task, temp_dir.path())
            .await
            .expect("save should succeed");

        let args = ListArgs {
            json: false,
            root: Some(temp_dir.path().to_path_buf()),
        };

        let result = list_command(args).await.expect("list should succeed");

        assert_eq!(result.total, 1);
        assert_eq!(result.tasks[0].slug.as_str(), "task-1");
    }
}
