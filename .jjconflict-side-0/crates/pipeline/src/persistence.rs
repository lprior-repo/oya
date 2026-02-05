//! Persistence module - Save/load task status using SurrealDB.
//!
//! Tracks which stages passed/failed for each task using embedded SurrealDB.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::{
    Surreal,
    engine::local::{Db, RocksDb},
};

use crate::{
    domain::{Language, Priority, Slug, Task, TaskStatus},
    error::{Error, Result},
};

/// Stage result type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StageResult {
    Passed,
    Failed,
}

impl StageResult {
    /// Convert to string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }
}

/// Stage status record for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageRecord {
    pub stage_name: String,
    pub status: String,
    pub attempts: i32,
    pub last_error: String,
}

impl StageRecord {
    /// Create a new stage record.
    #[must_use]
    pub fn new(stage_name: String, result: StageResult, attempts: i32, error: String) -> Self {
        Self {
            stage_name,
            status: result.as_str().to_string(),
            attempts,
            last_error: error,
        }
    }
}

/// Complete task record for persistence in SurrealDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub slug: String,
    pub language: String,
    pub status: String,
    pub priority: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub stages: Vec<StageRecord>,
    pub worktree_path: String,
    pub branch: String,
    #[serde(default)]
    pub current_stage: String,
    #[serde(default)]
    pub current_error: String,
}

/// SurrealDB connection wrapper.
pub struct DbConnection {
    db: Surreal<Db>,
}

impl DbConnection {
    /// Create new database connection at repo root.
    pub async fn new(repo_root: &Path) -> Result<Self> {
        let db_path = repo_root.join(".OYA").join("db");
        if let Some(parent) = db_path.parent() {
            crate::process::create_dir_all(parent)?;
        }

        let db = Surreal::new::<RocksDb>(db_path)
            .await
            .map_err(|e| Error::DatabaseError {
                reason: format!("Failed to open SurrealDB: {e}"),
            })?;

        db.use_ns("oya")
            .use_db("OYA")
            .await
            .map_err(|e| Error::DatabaseError {
                reason: format!("Failed to select namespace/database: {e}"),
            })?;

        Ok(Self { db })
    }

    /// Get the underlying Surreal instance.
    pub fn inner(&self) -> &Surreal<Db> {
        &self.db
    }
}

/// Convert domain task to record for persistence.
#[must_use]
pub fn task_to_record(task: &Task) -> TaskRecord {
    let language_str = task.language.as_str().to_string();

    let (status_str, current_stage, current_error) = match &task.status {
        TaskStatus::Created => ("created".to_string(), String::new(), String::new()),
        TaskStatus::InProgress { stage } => {
            ("in_progress".to_string(), stage.clone(), String::new())
        }
        TaskStatus::PassedPipeline => ("passed".to_string(), String::new(), String::new()),
        TaskStatus::FailedPipeline { stage, reason } => {
            ("failed".to_string(), stage.clone(), reason.clone())
        }
        TaskStatus::Integrated => ("integrated".to_string(), String::new(), String::new()),
    };

    let priority_str = task.priority.as_str().to_string();
    let timestamp = Utc::now();

    TaskRecord {
        slug: task.slug.to_string(),
        language: language_str,
        status: status_str,
        priority: priority_str,
        created_at: timestamp,
        updated_at: timestamp,
        stages: Vec::new(),
        worktree_path: String::new(),
        branch: task.branch.clone(),
        current_stage,
        current_error,
    }
}

/// Convert record to domain task.
pub fn record_to_task(record: &TaskRecord) -> Result<Task> {
    let lang = Language::parse(&record.language)?;

    let status = match record.status.as_str() {
        "in_progress" => TaskStatus::InProgress {
            stage: record.current_stage.clone(),
        },
        "passed" => TaskStatus::PassedPipeline,
        "failed" => TaskStatus::FailedPipeline {
            stage: record.current_stage.clone(),
            reason: record.current_error.clone(),
        },
        "integrated" => TaskStatus::Integrated,
        _ => TaskStatus::Created,
    };

    let priority = Priority::parse(&record.priority).unwrap_or_else(|_| Priority::default());
    let slug = Slug::new(&record.slug)?;

    Ok(Task {
        slug,
        language: lang,
        status,
        priority,
        branch: record.branch.clone(),
    })
}

/// Save a task record to SurrealDB.
pub async fn save_task_record(task: &Task, repo_root: &Path) -> Result<()> {
    let conn = DbConnection::new(repo_root).await?;
    let record = task_to_record(task);
    let slug = record.slug.clone();

    // Check if task exists
    let existing: Option<TaskRecord> =
        conn.inner()
            .select(("tasks", &slug))
            .await
            .map_err(|e| Error::DatabaseError {
                reason: format!("Failed to check task existence: {e}"),
            })?;

    if existing.is_some() {
        // Update existing task
        conn.inner()
            .update::<Option<TaskRecord>>(("tasks", &slug))
            .content(record.clone())
            .await
            .map_err(|e| Error::DatabaseError {
                reason: format!("Failed to update task: {e}"),
            })?;
    } else {
        // Create new task
        conn.inner()
            .create::<Option<TaskRecord>>(("tasks", &slug))
            .content(record)
            .await
            .map_err(|e| Error::DatabaseError {
                reason: format!("Failed to create task: {e}"),
            })?;
    }

    Ok(())
}

/// Load a task record by slug.
pub async fn load_task_record(slug: &str, repo_root: &Path) -> Result<Task> {
    let conn = DbConnection::new(repo_root).await?;

    let record: Option<TaskRecord> =
        conn.inner()
            .select(("tasks", slug))
            .await
            .map_err(|e| Error::DatabaseError {
                reason: format!("Failed to load task: {e}"),
            })?;

    let record = record.ok_or_else(|| Error::TaskNotFound {
        slug: slug.to_string(),
    })?;

    record_to_task(&record)
}

/// List all tasks from SurrealDB.
pub async fn list_all_tasks(repo_root: &Path) -> Result<Vec<Task>> {
    let conn = DbConnection::new(repo_root).await?;

    let records: Vec<TaskRecord> =
        conn.inner()
            .select("tasks")
            .await
            .map_err(|e| Error::DatabaseError {
                reason: format!("Failed to list tasks: {e}"),
            })?;

    records
        .iter()
        .map(record_to_task)
        .collect::<Result<Vec<_>>>()
}

/// Update stage status in task record.
#[allow(clippy::too_many_arguments)]
pub async fn update_stage_status(
    task: &Task,
    stage_name: &str,
    result: StageResult,
    attempts: i32,
    error: &str,
    repo_root: &Path,
) -> Result<()> {
    let conn = DbConnection::new(repo_root).await?;
    let slug = task.slug.to_string();

    // Load existing task to get current stages
    let record: Option<TaskRecord> =
        conn.inner()
            .select(("tasks", &slug))
            .await
            .map_err(|e| Error::DatabaseError {
                reason: format!("Failed to load task for stage update: {e}"),
            })?;

    let mut task_record = record.unwrap_or_else(|| task_to_record(task));

    // Update or append stage
    let new_stage = StageRecord::new(stage_name.to_string(), result, attempts, error.to_string());

    let existing_stage_idx = task_record
        .stages
        .iter()
        .position(|s| s.stage_name == stage_name);

    match existing_stage_idx {
        Some(idx) => {
            task_record.stages[idx] = new_stage;
        }
        None => {
            task_record.stages.push(new_stage);
        }
    }

    task_record.updated_at = Utc::now();

    // Update the task with new stages
    conn.inner()
        .update::<Option<TaskRecord>>(("tasks", &slug))
        .content(task_record)
        .await
        .map_err(|e| Error::DatabaseError {
            reason: format!("Failed to update stage status: {e}"),
        })?;

    Ok(())
}

/// Filter tasks by status.
#[must_use]
pub fn filter_tasks_by_status(tasks: &[Task], status_filter: TaskStatus) -> Vec<&Task> {
    tasks
        .iter()
        .filter(|task| {
            matches!(
                (&task.status, &status_filter),
                (TaskStatus::Created, TaskStatus::Created)
                    | (TaskStatus::InProgress { .. }, TaskStatus::InProgress { .. })
                    | (TaskStatus::PassedPipeline, TaskStatus::PassedPipeline)
                    | (
                        TaskStatus::FailedPipeline { .. },
                        TaskStatus::FailedPipeline { .. }
                    )
                    | (TaskStatus::Integrated, TaskStatus::Integrated)
            )
        })
        .collect()
}

/// Filter tasks by language.
#[must_use]
pub fn filter_tasks_by_language(tasks: &[Task], language: Language) -> Vec<&Task> {
    tasks
        .iter()
        .filter(|task| task.language == language)
        .collect()
}

/// Filter tasks by priority.
#[must_use]
pub fn filter_tasks_by_priority(tasks: &[Task], priority: Priority) -> Vec<&Task> {
    tasks
        .iter()
        .filter(|task| task.priority == priority)
        .collect()
}

/// Get tasks ready for integration (PassedPipeline).
#[must_use]
pub fn get_ready_tasks(tasks: &[Task]) -> Vec<&Task> {
    tasks
        .iter()
        .filter(|task| task.status.is_ready_for_integration())
        .collect()
}

/// Get failed tasks.
#[must_use]
pub fn get_failed_tasks(tasks: &[Task]) -> Vec<&Task> {
    tasks
        .iter()
        .filter(|task| task.status.is_failed())
        .collect()
}

/// Get in-progress tasks.
#[must_use]
pub fn get_in_progress_tasks(tasks: &[Task]) -> Vec<&Task> {
    tasks
        .iter()
        .filter(|task| task.status.is_transient())
        .collect()
}

/// Count tasks by status.
#[must_use]
pub fn count_tasks_by_status(tasks: &[Task]) -> std::collections::HashMap<String, usize> {
    tasks
        .iter()
        .fold(std::collections::HashMap::new(), |mut counts, task| {
            let status_key = match &task.status {
                TaskStatus::Created => "created".to_string(),
                TaskStatus::InProgress { .. } => "in_progress".to_string(),
                TaskStatus::PassedPipeline => "passed".to_string(),
                TaskStatus::FailedPipeline { .. } => "failed".to_string(),
                TaskStatus::Integrated => "integrated".to_string(),
            };

            *counts.entry(status_key).or_insert(0) += 1;
            counts
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_record_conversion() {
        let slug = Slug::new("test-task");
        assert!(slug.is_ok());

        if let Ok(s) = slug {
            let task = Task::new(s, Language::Rust);

            let record = task_to_record(&task);
            assert_eq!(record.slug, "test-task");
            assert_eq!(record.language, "rust");
            assert_eq!(record.status, "created");

            let restored = record_to_task(&record);
            assert!(restored.is_ok());
        }
    }
}
