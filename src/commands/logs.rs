#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Logs command implementation
//!
//! Provides centralized log viewing with filtering and tailing capabilities.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::Parser;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use tokio::fs;
use tokio::io::AsyncBufReadExt;
use tracing::{debug, warn};

/// Arguments for the logs command
#[derive(Parser, Debug, Clone)]
pub struct LogsArgs {
    /// Filter by bead ID
    #[arg(long)]
    pub bead: Option<String>,

    /// Filter by stage name
    #[arg(long)]
    pub stage: Option<String>,

    /// Filter by agent ID
    #[arg(long)]
    pub agent: Option<String>,

    /// Filter by log level (error, warn, info, debug)
    #[arg(long)]
    pub level: Option<String>,

    /// Follow mode (tail logs as they arrive)
    #[arg(long)]
    pub follow: bool,

    /// Export logs to file
    #[arg(long)]
    pub export: Option<PathBuf>,

    /// Number of lines to show (default: all)
    #[arg(long, default_value = "0")]
    pub lines: usize,

    /// Log directory path (default: from OYA_LOG_DIR or ./logs)
    #[arg(long)]
    pub log_dir: Option<PathBuf>,
}

/// Output from the logs command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsOutput {
    /// Filtered log entries
    pub entries: Vec<LogEntry>,
    /// Total entries found
    pub total_count: usize,
    /// Number of entries filtered
    pub filtered_count: usize,
}

/// A single log entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogEntry {
    /// Timestamp of the log entry
    pub timestamp: DateTime<Utc>,
    /// Log level
    pub level: LogLevel,
    /// Source of the log entry
    pub source: LogSource,
    /// Log message
    pub message: String,
}

/// Log level enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

/// Log source enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogSource {
    /// Bead ID
    pub bead: Option<String>,
    /// Stage name
    pub stage: Option<String>,
    /// Agent ID
    pub agent: Option<String>,
    /// Component name
    pub component: String,
}

/// Errors specific to the logs command
#[derive(Debug, Error)]
pub enum LogsError {
    #[error("Log directory not found at {path}")]
    DirectoryNotFound { path: PathBuf },

    #[error("Permission denied reading {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Invalid filter: {filter}")]
    InvalidFilter { filter: String },

    #[error("Failed to write logs to {path}")]
    ExportFailed { path: PathBuf },

    #[error("Corrupted log entry in {file} at line {line}")]
    CorruptedLog { file: String, line: usize },
}

impl LogsError {
    /// Get the exit code for this error
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::DirectoryNotFound { .. } => 3,
            Self::PermissionDenied { .. } => 4,
            Self::InvalidFilter { .. } => 5,
            Self::ExportFailed { .. } => 6,
            Self::CorruptedLog { .. } => 7,
        }
    }

    /// Get a hint for remediation
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::DirectoryNotFound { .. } => {
                Some("Check OYA_LOG_DIR environment variable or default location".to_string())
            }
            Self::PermissionDenied { .. } => Some("Check file permissions with 'ls -la'".to_string()),
            Self::InvalidFilter { .. } => Some(
                "Filters must match pattern: --bead ID, --stage NAME, --agent ID, --level LEVEL"
                    .to_string(),
            ),
            Self::ExportFailed { .. } => {
                Some("Check directory exists and is writable".to_string())
            }
            Self::CorruptedLog { .. } => {
                Some("Log file may be partially written or truncated".to_string())
            }
        }
    }
}

/// Core function to validate filter arguments
fn validate_filters(args: &LogsArgs) -> Result<(), LogsError> {
    // Validate bead ID format (alphanumeric with hyphens/underscores)
    if let Some(ref bead) = args.bead {
        if !is_valid_bead_id(bead) {
            return Err(LogsError::InvalidFilter {
                filter: format!("--bead {bead}"),
            });
        }
    }

    // Validate log level
    if let Some(ref level) = args.level {
        if !matches!(level.to_lowercase().as_str(), "error" | "warn" | "info" | "debug") {
            return Err(LogsError::InvalidFilter {
                filter: format!("--level {level}"),
            });
        }
    }

    Ok(())
}

/// Check if a bead ID is valid
fn is_valid_bead_id(id: &str) -> bool {
    !id.is_empty()
        && id.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Core function to parse a log line
fn parse_log_line(line: &str, _file: &str, _line_num: usize) -> Option<LogEntry> {
    // Expected format: [TIMESTAMP] LEVEL SOURCE MESSAGE
    // Example: [2024-02-08T20:15:13.903Z] ERROR bead=abc123 stage=build Error message

    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Parse timestamp
    let timestamp_start = line.find('[')?;
    let timestamp_end = line.find(']')?;
    let timestamp_str = &line[timestamp_start + 1..timestamp_end];

    let timestamp: DateTime<Utc> = timestamp_str.parse().ok()?;

    // Parse level
    let after_timestamp = &line[timestamp_end + 1..].trim_start();
    let level_end = after_timestamp.find(' ')?;
    let level_str = &after_timestamp[..level_end];

    let level = match level_str {
        "ERROR" => LogLevel::Error,
        "WARN" => LogLevel::Warn,
        "INFO" => LogLevel::Info,
        "DEBUG" => LogLevel::Debug,
        _ => return None,
    };

    // Parse source
    let rest = &after_timestamp[level_end..].trim_start();
    let source = parse_log_source(rest);

    // Extract message
    let message = rest
        .split(' ')
        .skip_while(|s| s.contains('='))
        .join(" ");

    Some(LogEntry {
        timestamp,
        level,
        source,
        message,
    })
}

/// Parse log source from log line
fn parse_log_source(rest: &str) -> LogSource {
    let mut bead = None;
    let mut stage = None;
    let mut agent = None;
    let mut component = "unknown".to_string();

    for part in rest.split(' ') {
        if let Some((key, value)) = part.split_once('=') {
            match key {
                "bead" => bead = Some(value.to_string()),
                "stage" => stage = Some(value.to_string()),
                "agent" => agent = Some(value.to_string()),
                "component" => component = value.to_string(),
                _ => {}
            }
        }
    }

    LogSource {
        bead,
        stage,
        agent,
        component,
    }
}

/// Core function to filter log entries
fn filter_entries(entries: Vec<LogEntry>, args: &LogsArgs) -> Vec<LogEntry> {
    entries
        .into_iter()
        .filter(|entry| {
            // Filter by bead
            if let Some(ref bead) = args.bead {
                if entry.source.bead.as_ref().map_or(true, |b| b != bead) {
                    return false;
                }
            }

            // Filter by stage
            if let Some(ref stage) = args.stage {
                if entry.source.stage.as_ref().map_or(true, |s| s != stage) {
                    return false;
                }
            }

            // Filter by agent
            if let Some(ref agent) = args.agent {
                if entry.source.agent.as_ref().map_or(true, |a| a != agent) {
                    return false;
                }
            }

            // Filter by level
            if let Some(ref level) = args.level {
                let level_enum = match level.to_lowercase().as_str() {
                    "error" => LogLevel::Error,
                    "warn" => LogLevel::Warn,
                    "info" => LogLevel::Info,
                    "debug" => LogLevel::Debug,
                    _ => return false,
                };

                if entry.level != level_enum {
                    return false;
                }
            }

            true
        })
        .collect()
}

/// Core function to sort log entries chronologically
fn sort_entries(entries: Vec<LogEntry>) -> Vec<LogEntry> {
    entries.into_iter().sorted_by(|a, b| a.timestamp.cmp(&b.timestamp)).collect()
}

/// Shell function: Read log directory and parse all entries
async fn read_log_directory(log_dir: &PathBuf) -> Result<Vec<LogEntry>, LogsError> {
    // Check if directory exists
    if !log_dir.exists() {
        return Err(LogsError::DirectoryNotFound {
            path: log_dir.clone(),
        });
    }

    // Read directory
    let mut entries = Vec::new();

    let dir_result = fs::read_dir(log_dir).await;

    let mut dir = match dir_result {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            return Err(LogsError::PermissionDenied {
                path: log_dir.clone(),
            })
        }
        Err(_) => return Err(LogsError::DirectoryNotFound {
            path: log_dir.clone(),
        }),
    };

    loop {
        let entry_result = dir.next_entry().await;
        let entry = match entry_result {
            Ok(Some(e)) => e,
            Ok(None) => break,
            Err(_) => continue,
        };

        let path = entry.path();

        // Only process .log files
        if path.extension().map_or(true, |e| e != "log") {
            continue;
        }

        let file_name = match path.file_name().and_then(std::ffi::OsStr::to_str) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Read file
        let file = match fs::File::open(&path).await {
            Ok(f) => f,
            Err(_) => continue,
        };

        let reader = tokio::io::BufReader::new(file);
        let mut lines = reader.lines();

        let mut line_num = 0;
        loop {
            let line_result = lines.next_line().await;
            let line = match line_result {
                Ok(Some(l)) => l,
                Ok(None) => break,
                Err(_) => continue,
            };

            line_num += 1;

            if let Some(log_entry) = parse_log_line(&line, &file_name, line_num) {
                entries.push(log_entry);
            } else {
                warn!("Corrupted log entry in {file_name} at line {line_num}");
            }
        }
    }

    Ok(entries)
}

/// Shell function: Export logs to file
async fn export_logs(entries: &[LogEntry], path: &PathBuf) -> Result<(), LogsError> {
    let content = entries
        .iter()
        .map(|entry| format!("{entry}"))
        .join("\n");

    // Write to temp file first
    let temp_path = path.with_extension("tmp");

    fs::write(&temp_path, content)
        .await
        .map_err(|_| LogsError::ExportFailed {
            path: path.clone(),
        })?;

    // Atomic rename
    fs::rename(&temp_path, path)
        .await
        .map_err(|_| LogsError::ExportFailed {
            path: path.clone(),
        })?;

    Ok(())
}

/// Main logs command implementation
pub async fn logs_command(args: LogsArgs) -> Result<LogsOutput, LogsError> {
    debug!("Running logs command with args: {args:?}");

    // Validate filters
    validate_filters(&args)?;

    // Determine log directory
    let log_dir = args
        .log_dir
        .clone()
        .or_else(|| std::env::var("OYA_LOG_DIR").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("./logs"));

    // Read log entries
    let mut entries = read_log_directory(&log_dir).await?;

    debug!("Read {} log entries from directory", entries.len());

    // Filter entries
    let total_count = entries.len();
    entries = filter_entries(entries, &args);
    let filtered_count = entries.len();

    debug!("Filtered to {} entries", filtered_count);

    // Sort chronologically
    entries = sort_entries(entries);

    // Apply line limit
    if args.lines > 0 && entries.len() > args.lines {
        entries = entries.into_iter().rev().take(args.lines).collect();
        entries = sort_entries(entries);
    }

    // Export if requested
    if let Some(ref export_path) = args.export {
        export_logs(&entries, export_path).await?;
        debug!("Exported {} entries to {export_path:?}", entries.len());
    }

    // Print entries
    for entry in &entries {
        println!("{entry}");
    }

    Ok(LogsOutput {
        entries,
        total_count,
        filtered_count,
    })
}

impl std::fmt::Display for LogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self.level {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
        };

        write!(
            f,
            "[{}] {} {}",
            self.timestamp.format("%Y-%m-%dT%H:%M:%SZ"),
            level,
            self.message
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn test_validate_filters_with_valid_bead() {
        let args = LogsArgs {
            bead: Some("abc123".to_string()),
            stage: None,
            agent: None,
            level: None,
            follow: false,
            export: None,
            lines: 0,
            log_dir: None,
        };

        assert!(validate_filters(&args).is_ok());
    }

    #[test]
    fn test_validate_filters_with_invalid_bead() {
        let args = LogsArgs {
            bead: Some("@invalid@".to_string()),
            stage: None,
            agent: None,
            level: None,
            follow: false,
            export: None,
            lines: 0,
            log_dir: None,
        };

        assert!(validate_filters(&args).is_err());
    }

    #[test]
    fn test_validate_filters_with_valid_level() {
        let args = LogsArgs {
            bead: None,
            stage: None,
            agent: None,
            level: Some("error".to_string()),
            follow: false,
            export: None,
            lines: 0,
            log_dir: None,
        };

        assert!(validate_filters(&args).is_ok());
    }

    #[test]
    fn test_validate_filters_with_invalid_level() {
        let args = LogsArgs {
            bead: None,
            stage: None,
            agent: None,
            level: Some("invalid".to_string()),
            follow: false,
            export: None,
            lines: 0,
            log_dir: None,
        };

        assert!(validate_filters(&args).is_err());
    }

    #[test]
    fn test_filter_entries_by_bead() {
        let entries = vec![
            LogEntry {
                timestamp: DateTime::parse_from_rfc3339("2024-02-08T20:15:13Z")
                    .unwrap()
                    .with_timezone(&Utc),
                level: LogLevel::Info,
                source: LogSource {
                    bead: Some("abc123".to_string()),
                    stage: None,
                    agent: None,
                    component: "test".to_string(),
                },
                message: "Test message 1".to_string(),
            },
            LogEntry {
                timestamp: DateTime::parse_from_rfc3339("2024-02-08T20:15:14Z")
                    .unwrap()
                    .with_timezone(&Utc),
                level: LogLevel::Info,
                source: LogSource {
                    bead: Some("def456".to_string()),
                    stage: None,
                    agent: None,
                    component: "test".to_string(),
                },
                message: "Test message 2".to_string(),
            },
        ];

        let args = LogsArgs {
            bead: Some("abc123".to_string()),
            stage: None,
            agent: None,
            level: None,
            follow: false,
            export: None,
            lines: 0,
            log_dir: None,
        };

        let filtered = filter_entries(entries, &args);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].source.bead.as_ref().unwrap(), "abc123");
    }

    #[test]
    fn test_sort_entries_chronologically() {
        let entries = vec![
            LogEntry {
                timestamp: DateTime::parse_from_rfc3339("2024-02-08T20:15:14Z")
                    .unwrap()
                    .with_timezone(&Utc),
                level: LogLevel::Info,
                source: LogSource {
                    bead: None,
                    stage: None,
                    agent: None,
                    component: "test".to_string(),
                },
                message: "Second".to_string(),
            },
            LogEntry {
                timestamp: DateTime::parse_from_rfc3339("2024-02-08T20:15:13Z")
                    .unwrap()
                    .with_timezone(&Utc),
                level: LogLevel::Info,
                source: LogSource {
                    bead: None,
                    stage: None,
                    agent: None,
                    component: "test".to_string(),
                },
                message: "First".to_string(),
            },
        ];

        let sorted = sort_entries(entries);

        assert_eq!(sorted[0].message, "First");
        assert_eq!(sorted[1].message, "Second");
    }

    #[test]
    fn test_parse_log_line_valid() {
        let line = "[2024-02-08T20:15:13.903Z] ERROR bead=abc123 stage=build Error message";

        let entry = parse_log_line(line, "test.log", 1);

        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.level, LogLevel::Error);
        assert_eq!(entry.source.bead.as_ref().unwrap(), "abc123");
        assert_eq!(entry.source.stage.as_ref().unwrap(), "build");
    }

    #[test]
    fn test_is_valid_bead_id() {
        assert!(is_valid_bead_id("abc123"));
        assert!(is_valid_bead_id("abc-123"));
        assert!(is_valid_bead_id("abc_123"));
        assert!(!is_valid_bead_id("@invalid@"));
        assert!(!is_valid_bead_id(""));
    }
}
