//! JJ workspace lifecycle management
//!
//! This module provides safe, functional APIs for managing JJ workspaces.
//! All operations return `Result` and never panic.

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{Error, Result};

/// Helper to create a JJ command error with appropriate context
fn jj_command_error(operation: &str, error: &std::io::Error) -> Error {
    let is_not_found = error.kind() == ErrorKind::NotFound;
    Error::JjCommandError {
        operation: operation.to_string(),
        source: error.to_string(),
        is_not_found,
    }
}

/// Information about a JJ workspace
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    /// Workspace name
    pub name: String,
    /// Workspace path
    pub path: PathBuf,
    /// Whether the workspace is stale (directory doesn't exist)
    pub is_stale: bool,
}

/// Summary of changes in a workspace
#[derive(Debug, Clone, Default)]
pub struct DiffSummary {
    /// Number of lines added
    pub insertions: usize,
    /// Number of lines deleted
    pub deletions: usize,
}

/// Status of files in a workspace
#[derive(Debug, Clone)]
pub struct Status {
    /// Modified files
    pub modified: Vec<PathBuf>,
    /// Added files
    pub added: Vec<PathBuf>,
    /// Deleted files
    pub deleted: Vec<PathBuf>,
    /// Renamed files (`old_path`, `new_path`)
    pub renamed: Vec<(PathBuf, PathBuf)>,
    /// Unknown files
    pub unknown: Vec<PathBuf>,
}

impl Status {
    /// Check if there are any changes
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.modified.is_empty()
            && self.added.is_empty()
            && self.deleted.is_empty()
            && self.renamed.is_empty()
    }

    /// Count total number of changed files
    #[must_use]
    pub const fn change_count(&self) -> usize {
        self.modified.len() + self.added.len() + self.deleted.len() + self.renamed.len()
    }
}

/// Create a new JJ workspace
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Workspace name already exists
/// - Unable to create workspace directory
/// - JJ command fails
pub fn workspace_create(name: &str, path: &Path) -> Result<()> {
    // Validate inputs
    if name.is_empty() {
        return Err(Error::InvalidConfig(
            "workspace name cannot be empty".into(),
        ));
    }

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::IoError(format!("Failed to create workspace directory: {e}")))?;
    }

    // Execute: jj workspace add --name <name> <path>
    let output = Command::new("jj")
        .args(["workspace", "add", "--name", name])
        .arg(path)
        .output()
        .map_err(|e| jj_command_error("create workspace", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "create workspace".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    Ok(())
}

/// Forget (remove) a JJ workspace
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Workspace doesn't exist
/// - JJ command fails
pub fn workspace_forget(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::InvalidConfig(
            "workspace name cannot be empty".into(),
        ));
    }

    // Execute: jj workspace forget <name>
    let output = Command::new("jj")
        .args(["workspace", "forget", name])
        .output()
        .map_err(|e| jj_command_error("forget workspace", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "forget workspace".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    Ok(())
}

/// List all JJ workspaces
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Unable to parse JJ output
pub fn workspace_list() -> Result<Vec<WorkspaceInfo>> {
    // Execute: jj workspace list
    let output = Command::new("jj")
        .args(["workspace", "list"])
        .output()
        .map_err(|e| jj_command_error("list workspaces", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "list workspaces".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_workspace_list(&stdout)
}

/// Parse output from 'jj workspace list'
fn parse_workspace_list(output: &str) -> Result<Vec<WorkspaceInfo>> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            // Format: "workspace_name: /path/to/workspace"
            // or "workspace_name: /path/to/workspace (stale)"
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(Error::ParseError(format!(
                    "Invalid workspace list format: {line}"
                )));
            }

            let name = parts[0].trim().to_string();
            let rest = parts[1].trim();

            let (path_str, is_stale) = rest
                .strip_suffix("(stale)")
                .map_or((rest, false), |path_part| (path_part.trim(), true));

            Ok(WorkspaceInfo {
                name,
                path: PathBuf::from(path_str),
                is_stale,
            })
        })
        .collect()
}

/// Get status of a workspace
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Unable to parse JJ output
pub fn workspace_status(path: &Path) -> Result<Status> {
    // Execute: jj status (in the workspace directory)
    let output = Command::new("jj")
        .args(["status"])
        .current_dir(path)
        .output()
        .map_err(|e| jj_command_error("get workspace status", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "get workspace status".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_status(&stdout))
}

/// Parse output from 'jj status'
fn parse_status(output: &str) -> Status {
    let mut status = Status {
        modified: Vec::new(),
        added: Vec::new(),
        deleted: Vec::new(),
        renamed: Vec::new(),
        unknown: Vec::new(),
    };

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Look for status markers: M, A, D, R, ?
        if let Some(rest) = line.strip_prefix('M') {
            status.modified.push(PathBuf::from(rest.trim()));
        } else if let Some(rest) = line.strip_prefix('A') {
            status.added.push(PathBuf::from(rest.trim()));
        } else if let Some(rest) = line.strip_prefix('D') {
            status.deleted.push(PathBuf::from(rest.trim()));
        } else if let Some(rest) = line.strip_prefix('R') {
            // Renamed: "R old_path => new_path"
            if let Some((old, new)) = rest.split_once("=>") {
                status
                    .renamed
                    .push((PathBuf::from(old.trim()), PathBuf::from(new.trim())));
            }
        } else if let Some(rest) = line.strip_prefix('?') {
            status.unknown.push(PathBuf::from(rest.trim()));
        }
    }

    status
}

/// Get diff summary for a workspace
///
/// # Errors
///
/// Returns error if:
/// - JJ is not installed
/// - Not in a JJ repository
/// - Unable to parse JJ output
pub fn workspace_diff(path: &Path) -> Result<DiffSummary> {
    // Execute: jj diff --stat (in the workspace directory)
    let output = Command::new("jj")
        .args(["diff", "--stat"])
        .current_dir(path)
        .output()
        .map_err(|e| jj_command_error("get workspace diff", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "get workspace diff".to_string(),
            source: stderr.to_string(),
            is_not_found: false,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_diff_stat(&stdout))
}

/// Parse output from 'jj diff --stat'
fn parse_diff_stat(output: &str) -> DiffSummary {
    // Look for summary line like: "5 files changed, 123 insertions(+), 45 deletions(-)"
    let summary_line = output
        .lines()
        .find(|line| line.contains("insertion") || line.contains("deletion"))
        .unwrap_or("");

    let mut insertions = 0;
    let mut deletions = 0;

    // Parse insertions
    if let Some(ins_str) = summary_line.split("insertion").next() {
        if let Some(num_str) = ins_str.split_whitespace().last() {
            insertions = num_str.parse().unwrap_or(0);
        }
    }

    // Parse deletions
    if let Some(del_str) = summary_line.split("deletion").next() {
        if let Some(num_str) = del_str
            .rsplit(',')
            .next()
            .and_then(|s| s.split_whitespace().next())
        {
            deletions = num_str.parse().unwrap_or(0);
        }
    }

    DiffSummary {
        insertions,
        deletions,
    }
}

/// Check if JJ is installed and available
///
/// # Errors
///
/// Returns error if JJ is not found in PATH
pub fn check_jj_installed() -> Result<()> {
    Command::new("jj")
        .arg("--version")
        .output()
        .map_err(|e| jj_command_error("check JJ installation", &e))
        .and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(Error::JjCommandError {
                    operation: "check JJ installation".to_string(),
                    source: "JJ command returned non-zero exit code".to_string(),
                    is_not_found: false,
                })
            }
        })
}

/// Check if current directory is in a JJ repository
///
/// # Errors
///
/// Returns error if not in a JJ repository
pub fn check_in_jj_repo() -> Result<PathBuf> {
    let output = Command::new("jj")
        .args(["root"])
        .output()
        .map_err(|e| jj_command_error("find JJ repository root", &e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjCommandError {
            operation: "find JJ repository root".to_string(),
            source: format!("Not in a JJ repository. {stderr}"),
            is_not_found: false,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let root = stdout.trim();

    if root.is_empty() {
        Err(Error::JjCommandError {
            operation: "find JJ repository root".to_string(),
            source: "Could not determine JJ repository root".to_string(),
            is_not_found: false,
        })
    } else {
        Ok(PathBuf::from(root))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_workspace_list() {
        let output = "default: /home/user/repo\nfeature: /home/user/repo/.jjz/workspaces/feature\nstale-ws: /home/user/old (stale)";
        let result = parse_workspace_list(output);
        assert!(result.is_ok());

        let workspaces = result.unwrap_or_default();
        assert_eq!(workspaces.len(), 3);
        assert_eq!(workspaces[0].name, "default");
        assert!(!workspaces[0].is_stale);
        assert_eq!(workspaces[2].name, "stale-ws");
        assert!(workspaces[2].is_stale);
    }

    #[test]
    fn test_parse_status() {
        let output = "M file1.rs\nA file2.rs\nD file3.rs\n? unknown.txt";
        let status = parse_status(output);
        assert_eq!(status.modified.len(), 1);
        assert_eq!(status.added.len(), 1);
        assert_eq!(status.deleted.len(), 1);
        assert_eq!(status.unknown.len(), 1);
        assert!(!status.is_clean());
        assert_eq!(status.change_count(), 3);
    }

    #[test]
    fn test_parse_diff_stat() {
        let output = "file1.rs | 10 +++++++---\nfile2.rs | 5 ++---\n2 files changed, 12 insertions(+), 3 deletions(-)";
        let summary = parse_diff_stat(output);
        assert_eq!(summary.insertions, 12);
        assert_eq!(summary.deletions, 3);
    }

    #[test]
    fn test_status_is_clean() {
        let clean_status = Status {
            modified: Vec::new(),
            added: Vec::new(),
            deleted: Vec::new(),
            renamed: Vec::new(),
            unknown: Vec::new(),
        };
        assert!(clean_status.is_clean());

        let dirty_status = Status {
            modified: vec![PathBuf::from("file.rs")],
            added: Vec::new(),
            deleted: Vec::new(),
            renamed: Vec::new(),
            unknown: Vec::new(),
        };
        assert!(!dirty_status.is_clean());
    }
}
