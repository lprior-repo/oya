use std::path::{Path, PathBuf};

use tokio::fs;

use crate::domain::{Slug, Task};
use crate::error::{Error, Result};

fn tasks_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".oya").join("tasks.json")
}

async fn read_tasks(path: &Path) -> Result<Vec<Task>> {
    match fs::read(path).await {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|source| Error::ParseFailure {
            path: path.to_path_buf(),
            source,
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(source) => Err(Error::ReadFailure {
            path: path.to_path_buf(),
            source,
        }),
    }
}

async fn write_tasks(path: &Path, tasks: &[Task]) -> Result<()> {
    let payload = serde_json::to_vec_pretty(tasks).map_err(|source| Error::SerializeFailure {
        path: path.to_path_buf(),
        source,
    })?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|source| Error::WriteFailure {
                path: parent.to_path_buf(),
                source,
            })?;
    }

    fs::write(path, payload)
        .await
        .map_err(|source| Error::WriteFailure {
            path: path.to_path_buf(),
            source,
        })
}

/// Save (create or update) a task record.
///
/// # Errors
/// Returns errors for read/write or serialization failures.
pub async fn save_task_record(task: &Task, repo_root: &Path) -> Result<()> {
    let path = tasks_path(repo_root);
    let mut tasks = read_tasks(&path).await?;

    if let Some(existing) = tasks.iter_mut().find(|existing| existing.slug == task.slug) {
        *existing = task.clone();
    } else {
        tasks.push(task.clone());
    }

    write_tasks(&path, &tasks).await
}

/// Update a task status using transition validation.
///
/// # Errors
/// Returns errors when the task is missing, transition is invalid, or IO fails.
pub async fn update_task_status(
    slug: &str,
    status: crate::domain::TaskStatus,
    repo_root: &Path,
) -> Result<Task> {
    let path = tasks_path(repo_root);
    let mut tasks = read_tasks(&path).await?;
    let lookup = Slug::new(slug)?;

    let updated = tasks
        .iter()
        .find(|task| task.slug == lookup)
        .cloned()
        .ok_or_else(|| Error::TaskNotFound(slug.to_string()))?
        .transition_to(status)?;

    tasks = tasks
        .into_iter()
        .map(|task| {
            if task.slug == lookup {
                updated.clone()
            } else {
                task
            }
        })
        .collect();

    write_tasks(&path, &tasks).await?;

    Ok(updated)
}

/// Load a task record by slug.
///
/// # Errors
/// Returns `TaskNotFound` when the slug is missing.
pub async fn load_task_record(slug: &str, repo_root: &Path) -> Result<Task> {
    let path = tasks_path(repo_root);
    let tasks = read_tasks(&path).await?;
    let lookup = Slug::new(slug)?;

    tasks
        .into_iter()
        .find(|task| task.slug == lookup)
        .ok_or_else(|| Error::TaskNotFound(slug.to_string()))
}

/// List all tasks in the repository.
///
/// # Errors
/// Returns errors for read or parse failures.
pub async fn list_all_tasks(repo_root: &Path) -> Result<Vec<Task>> {
    let path = tasks_path(repo_root);
    read_tasks(&path).await
}

#[cfg(test)]
mod tests {

    #![allow(clippy::expect_used)]
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::domain::{Language, Priority, TaskStatus};

    #[tokio::test]
    async fn save_and_load_round_trip() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let slug = Slug::new("task-1").expect("slug should parse");
        let task = Task::new(slug, Language::Rust)
            .with_priority(Priority::P1)
            .with_status(TaskStatus::PassedPipeline);

        save_task_record(&task, temp_dir.path())
            .await
            .expect("save should succeed");

        let loaded = load_task_record("task-1", temp_dir.path())
            .await
            .expect("load should succeed");

        assert_eq!(loaded, task);
    }

    #[tokio::test]
    async fn list_all_tasks_returns_empty_when_missing() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let tasks = list_all_tasks(temp_dir.path())
            .await
            .expect("list should succeed");

        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn save_overwrites_existing_task() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let slug = Slug::new("task-2").expect("slug should parse");
        let task = Task::new(slug.clone(), Language::Rust);

        save_task_record(&task, temp_dir.path())
            .await
            .expect("save should succeed");

        let updated = task.with_status(TaskStatus::Integrated);

        save_task_record(&updated, temp_dir.path())
            .await
            .expect("save should succeed");

        let loaded = load_task_record(slug.as_str(), temp_dir.path())
            .await
            .expect("load should succeed");

        assert_eq!(loaded.status, TaskStatus::Integrated);
    }

    #[tokio::test]
    async fn load_returns_not_found_for_missing_task() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let result = load_task_record("missing-task", temp_dir.path()).await;

        assert!(matches!(result, Err(Error::TaskNotFound(_))));
    }

    #[tokio::test]
    async fn update_task_status_applies_transition_rules() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let slug = Slug::new("task-3").expect("slug should parse");
        let task = Task::new(slug.clone(), Language::Rust);

        save_task_record(&task, temp_dir.path())
            .await
            .expect("save should succeed");

        let updated = update_task_status(
            slug.as_str(),
            TaskStatus::InProgress {
                stage: "implement".to_string(),
            },
            temp_dir.path(),
        )
        .await
        .expect("transition should succeed");

        assert!(matches!(updated.status, TaskStatus::InProgress { .. }));
    }
}
