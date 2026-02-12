//! New command implementation
//!
//! Creates a new task with workspace isolation via zjj.

use anyhow::Result;
use clap::Parser;
use oya_pipeline::{Language, Priority, TaskStatus};
use oya_pipeline::{Slug, Task, save_task_record};
use std::path::PathBuf;

/// Arguments for the new command
#[derive(Parser, Debug, Clone)]
pub struct NewArgs {
    /// Slug for the task (lowercase alphanumeric + hyphens)
    #[arg(short, long)]
    pub slug: String,

    /// Language for the task (rust, go, python, js, gleam)
    #[arg(short, long, default_value = "rust")]
    pub language: String,

    /// Priority level (P0, P1, P2, P3)
    #[arg(short, long, default_value = "P2")]
    pub priority: String,

    /// Repository root path (default: current directory)
    #[arg(long)]
    pub root: Option<PathBuf>,

    /// Skip workspace creation (debug mode)
    #[arg(long)]
    pub skip_workspace: bool,
}

/// Output from the new command
#[derive(Debug, Clone)]
pub struct NewOutput {
    /// Created task
    pub task: Task,
    /// Workspace path (if created)
    pub workspace_path: Option<String>,
}

/// Create a new task with slug validation and optional workspace isolation
///
/// # Errors
/// Returns error if slug is invalid or task cannot be saved
pub async fn new_command(args: NewArgs) -> Result<NewOutput> {
    // Validate slug format
    let slug = Slug::new(&args.slug)?;

    // Parse language
    let language = match args.language.to_lowercase().as_str() {
        "rust" => Language::Rust,
        "go" => Language::Go,
        "python" => Language::Python,
        "javascript" | "js" => Language::JavaScript,
        "gleam" => Language::Gleam,
        _ => return Err(anyhow::anyhow!("Unknown language: {}", args.language)),
    };

    // Parse priority
    let priority = Priority::parse(&args.priority)?;

    // Create task with default values
    let task = Task::new(slug, language)
        .with_priority(priority)
        .with_status(TaskStatus::Created);

    // Save task record
    let root = args.root.unwrap_or_else(|| PathBuf::from("."));
    save_task_record(&task, &root).await?;

    // Create workspace via zjj (15 lines of code)
    let workspace_path = if !args.skip_workspace {
        create_zjj_workspace(&args.slug, &root).await?
    } else {
        None
    };

    Ok(NewOutput {
        task,
        workspace_path,
    })
}

/// Create zjj workspace for the task
///
/// This is the 15-line implementation of workspace isolation.
async fn create_zjj_workspace(slug: &str, root: &PathBuf) -> Result<Option<String>> {
    use tokio::process::Command;

    // Check if zjj is available
    let zjj_path = match std::env::var("ZJJ_PATH") {
        Ok(path) => path,
        Err(_) => "zjj".to_string(), // Use PATH lookup
    };

    // Create workspace with zjj
    let output = Command::new(&zjj_path)
        .arg("spawn")
        .arg(slug)
        .arg("--root")
        .arg(root)
        .output()
        .await?;

    if output.status.success() {
        // Parse workspace path from output
        let workspace = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Some(workspace))
    } else {
        // zjj failed, return error but task is still created
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!(
            "Failed to create zjj workspace: {}",
            stderr
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oya_pipeline::{Language, Priority, TaskStatus};

    #[tokio::test]
    async fn new_command_creates_task() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let args = NewArgs {
            slug: "test-task".to_string(),
            language: "rust".to_string(),
            priority: "P1".to_string(),
            root: Some(temp_dir.path().to_path_buf()),
            skip_workspace: true,
        };

        let result = new_command(args).await.expect("new should succeed");

        assert_eq!(result.task.slug.as_str(), "test-task");
        assert_eq!(result.task.language, Language::Rust);
        assert_eq!(result.task.priority, Priority::P1);
        assert_eq!(result.task.status, TaskStatus::Created);
    }

    #[tokio::test]
    async fn new_command_rejects_invalid_slug() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let args = NewArgs {
            slug: "Invalid_Slug".to_string(), // uppercase not allowed
            language: "rust".to_string(),
            priority: "P1".to_string(),
            root: Some(temp_dir.path().to_path_buf()),
            skip_workspace: true,
        };

        let result = new_command(args).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn new_command_rejects_path_traversal() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let args = NewArgs {
            slug: "../etc/passwd".to_string(),
            language: "rust".to_string(),
            priority: "P1".to_string(),
            root: Some(temp_dir.path().to_path_buf()),
            skip_workspace: true,
        };

        let result = new_command(args).await;

        assert!(result.is_err());
    }
}
