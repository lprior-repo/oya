//! Persistence module - Save/load task status as JSON.
//!
//! Tracks which stages passed/failed for each task.
//! Uses `.factory/tasks.json` for storage.

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    domain::{Language, Priority, Slug, Task, TaskStatus},
    error::{Error, Result},
    process::{create_dir_all, read_text_file, write_text_file},
};

/// Branch prefix for feature branches.
const BRANCH_PREFIX: &str = "feat/";

/// Colon escape sequence for encoded strings.
const COLON_ESCAPE: &str = "\\c";

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

/// Complete task record for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub slug: String,
    pub language: String,
    pub status: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    pub created_at: String,
    pub updated_at: String,
    pub stages: Vec<StageRecord>,
    #[serde(default)]
    pub worktree_path: String,
    #[serde(default)]
    pub current_stage: String,
    #[serde(default)]
    pub current_error: String,
}

fn default_priority() -> String {
    "P2".to_string()
}

/// Get the status file path.
#[must_use]
pub fn status_file_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".factory").join("tasks.json")
}

/// Get current timestamp in ISO format.
fn current_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Decode reason string (unescape colons).
fn decode_reason(encoded: &str) -> String {
    encoded.replace(COLON_ESCAPE, ":")
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
    let timestamp = current_timestamp();

    TaskRecord {
        slug: task.slug.to_string(),
        language: language_str,
        status: status_str,
        priority: priority_str,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        stages: Vec::new(),
        worktree_path: task.worktree_path.to_string_lossy().to_string(),
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
            reason: decode_reason(&record.current_error),
        },
        "integrated" => TaskStatus::Integrated,
        // "created" and any unknown status default to Created
        _ => TaskStatus::Created,
    };

    let priority = Priority::parse(&record.priority).unwrap_or_default();
    let slug = Slug::new(&record.slug)?;

    Ok(Task {
        slug,
        language: lang,
        status,
        priority,
        worktree_path: PathBuf::from(&record.worktree_path),
        branch: format!("{BRANCH_PREFIX}{}", record.slug),
    })
}

/// Load all task records from JSON file.
fn load_all_records(repo_root: &Path) -> Result<Vec<TaskRecord>> {
    let file_path = status_file_path(repo_root);

    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content = read_text_file(&file_path)?;

    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    // Try parsing as array first
    serde_json::from_str(&content).or_else(|_| {
        // Try parsing as single object
        serde_json::from_str::<TaskRecord>(&content)
            .map(|r| vec![r])
            .map_err(|e| Error::json_parse_failed(e.to_string()))
    })
}

/// Save all records to JSON file.
fn save_all_records(repo_root: &Path, records: &[TaskRecord]) -> Result<()> {
    let file_path = status_file_path(repo_root);
    let factory_dir = repo_root.join(".factory");

    create_dir_all(&factory_dir)?;

    let json = serde_json::to_string_pretty(records)
        .map_err(|e| Error::json_parse_failed(e.to_string()))?;

    write_text_file(&file_path, &json)
}

/// Save a task record to `.factory/tasks.json`.
pub fn save_task_record(task: &Task, repo_root: &Path) -> Result<()> {
    let record = task_to_record(task);
    let mut records = load_all_records(repo_root)?;

    // Update existing or append new
    let existing_idx = records.iter().position(|r| r.slug == record.slug);

    match existing_idx {
        Some(idx) => {
            records[idx] = record;
        }
        None => {
            records.push(record);
        }
    }

    save_all_records(repo_root, &records)
}

/// Load a task record by slug.
pub fn load_task_record(slug: &str, repo_root: &Path) -> Result<Task> {
    let records = load_all_records(repo_root)?;

    let record = records
        .iter()
        .find(|r| r.slug == slug)
        .ok_or_else(|| Error::TaskNotFound {
            slug: slug.to_string(),
        })?;

    record_to_task(record)
}

/// List all tasks from `.factory/tasks.json`.
pub fn list_all_tasks(repo_root: &Path) -> Result<Vec<Task>> {
    let records = load_all_records(repo_root)?;

    records
        .iter()
        .map(record_to_task)
        .collect::<Result<Vec<_>>>()
}

/// Filter tasks by status.
#[must_use]
pub fn filter_tasks_by_status(tasks: &[Task], status_filter: TaskStatus) -> Vec<&Task> {
    tasks
        .iter()
        .filter(|task| match (&task.status, &status_filter) {
            (TaskStatus::Created, TaskStatus::Created) => true,
            (TaskStatus::InProgress { .. }, TaskStatus::InProgress { .. }) => true,
            (TaskStatus::PassedPipeline, TaskStatus::PassedPipeline) => true,
            (TaskStatus::FailedPipeline { .. }, TaskStatus::FailedPipeline { .. }) => true,
            (TaskStatus::Integrated, TaskStatus::Integrated) => true,
            _ => false,
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
    let mut counts = std::collections::HashMap::new();

    for task in tasks {
        let status_key = match &task.status {
            TaskStatus::Created => "created".to_string(),
            TaskStatus::InProgress { .. } => "in_progress".to_string(),
            TaskStatus::PassedPipeline => "passed".to_string(),
            TaskStatus::FailedPipeline { .. } => "failed".to_string(),
            TaskStatus::Integrated => "integrated".to_string(),
        };

        *counts.entry(status_key).or_insert(0) += 1;
    }

    counts
}

/// Update stage status in task record.
#[allow(clippy::too_many_arguments)]
pub fn update_stage_status(
    task: &Task,
    stage_name: &str,
    result: StageResult,
    attempts: i32,
    error: &str,
    repo_root: &Path,
) -> Result<()> {
    let factory_dir = repo_root.join(".factory");
    create_dir_all(&factory_dir)?;

    let slug_str = task.slug.to_string();
    let mut task_record = task_to_record(task);

    // Load existing stages if task exists
    if let Ok(existing_records) = load_all_records(repo_root) {
        if let Some(existing) = existing_records.iter().find(|r| r.slug == slug_str) {
            task_record.stages.clone_from(&existing.stages);
        }
    }

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

    // Save updated record
    let mut records = load_all_records(repo_root)?;
    let existing_idx = records.iter().position(|r| r.slug == slug_str);

    match existing_idx {
        Some(idx) => {
            records[idx] = task_record;
        }
        None => {
            records.push(task_record);
        }
    }

    save_all_records(repo_root, &records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_record_roundtrip() {
        let slug = Slug::new("test-task");
        assert!(slug.is_ok());

        if let Ok(s) = slug {
            let task = Task::new(s, Language::Rust, PathBuf::from("/tmp/test"));

            let record = task_to_record(&task);
            assert_eq!(record.slug, "test-task");
            assert_eq!(record.language, "rust");
            assert_eq!(record.status, "created");

            let restored = record_to_task(&record);
            assert!(restored.is_ok());
        }
    }

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts.contains('T'));
        assert!(ts.ends_with('Z'));
    }
}
