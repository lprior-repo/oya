//! Stage-scoped write allowlists for governance.
//!
//! This module enforces write permissions per stage, preventing accidental or
//! malicious writes outside allowed paths. Each stage can only write to specific
//! directories/files based on its role in the TDD pipeline.
//!
//! # Security Model
//!
//! - Deny by default: any path not explicitly allowed is rejected
//! - Stage-scoped: each stage has its own allowed write paths
//! - Workspace-isolated: writes are constrained to the stage workspace
//!
//! # Allowed Paths per Stage
//!
//! | Stage | Allowed Paths |
//! |-------|---------------|
//! | `Contract` | `docs/`, `*.md` |
//! | `Implementation` | `src/`, `tests/`, `benches/`, `lib.rs`, `main.rs` |
//! | `ShipGate` | `.beads/`, `.git/` (merge operations only) |
//!
//! # Example
//!
//! ```ignore
//! use oya::runtime_tools::write_allowlist::{validate_write_path, is_write_allowed, StageWriteConfig};
//! use oya::types::StageName;
//! use std::path::Path;
//!
//! let workspace = Path::new("/home/user/project");
//!
//! // Validate that Implementation stage can write to src/
//! let src_path = workspace.join("src").join("main.rs");
//! assert!(validate_write_path(&StageName::Implementation, &src_path, workspace).is_ok());
//!
//! // Check if write is allowed (convenience function)
//! assert!(is_write_allowed(&StageName::Implementation, &src_path, workspace));
//!
//! // Get stage write configuration
//! let config = StageWriteConfig::for_stage(StageName::Contract);
//! assert!(!config.read_only);
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use thiserror::Error;

use oya::types::StageName;

/// Maximum path length to prevent denial-of-service attacks
const MAX_PATH_LEN: usize = 4096;

/// Error type for write allowlist violations
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WriteAllowlistError {
    #[error("path is empty")]
    EmptyPath,

    #[error("path exceeds maximum length: {0} > {MAX_PATH_LEN}")]
    PathTooLong(usize),

    #[error("path contains forbidden control characters")]
    PathContainsControlChars,

    #[error("path contains directory traversal sequence: {0}")]
    PathTraversalDetected(String),

    #[error("stage {stage:?} cannot write to path: {path}")]
    WriteNotAllowed { stage: StageName, path: String },

    #[error("path is not within workspace root: {path} is not under {workspace_root}")]
    NotWithinWorkspace { path: String, workspace_root: String },

    #[error("path is not absolute: {0}")]
    RelativePath(String),
}

/// Write permission configuration for a stage
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageWriteConfig {
    /// The stage this config applies to
    pub stage: StageName,
    /// Allowed directory prefixes (relative to workspace root)
    pub allowed_dirs: Vec<PathBuf>,
    /// Allowed file patterns (glob-style, relative to workspace root)
    pub allowed_patterns: Vec<String>,
    /// Whether the stage is read-only (no writes allowed)
    pub read_only: bool,
}

impl StageWriteConfig {
    /// Create write configuration for a given stage
    #[must_use]
    pub fn for_stage(stage: StageName) -> Self {
        match stage {
            StageName::Explore | StageName::Red | StageName::Witness => {
                Self::read_only_config(stage)
            }
            StageName::Contract => Self::contract_config(stage),
            StageName::Implementation => Self::implementation_config(stage),
            StageName::ShipGate => Self::ship_gate_config(stage),
        }
    }

    const fn read_only_config(stage: StageName) -> Self {
        Self { stage, allowed_dirs: vec![], allowed_patterns: vec![], read_only: true }
    }

    fn contract_config(stage: StageName) -> Self {
        Self {
            stage,
            allowed_dirs: vec![PathBuf::from("docs")],
            allowed_patterns: vec!["*.md".to_string()],
            read_only: false,
        }
    }

    fn implementation_config(stage: StageName) -> Self {
        Self {
            stage,
            allowed_dirs: vec![
                PathBuf::from("src"),
                PathBuf::from("tests"),
                PathBuf::from("benches"),
            ],
            allowed_patterns: vec![
                "lib.rs".to_string(),
                "main.rs".to_string(),
                "build.rs".to_string(),
            ],
            read_only: false,
        }
    }

    fn ship_gate_config(stage: StageName) -> Self {
        Self {
            stage,
            allowed_dirs: vec![PathBuf::from(".beads"), PathBuf::from(".git")],
            allowed_patterns: vec![],
            read_only: false,
        }
    }
}

/// Validates a path for write operations within a stage's workspace.
///
/// # Arguments
///
/// * `stage` - The stage attempting the write
/// * `path` - The absolute path to validate
/// * `workspace_root` - The absolute path to the workspace root
///
/// # Errors
///
/// Returns `WriteAllowlistError` if:
/// - The path is empty or too long
/// - The path contains forbidden characters or traversal sequences
/// - The path is not absolute
/// - The path is not within the workspace root
/// - The stage is read-only
/// - The path is not in the stage's allowed list
///
/// # Example
///
/// ```
/// use oya::runtime_tools::write_allowlist::{validate_write_path, WriteAllowlistError};
/// use oya::types::StageName;
/// use std::path::Path;
///
/// let workspace = Path::new("/home/user/project");
/// let stage = StageName::Contract;
///
/// // Allowed: writing to docs directory
/// let docs_path = workspace.join("docs").join("contract.md");
/// assert!(validate_write_path(&stage, &docs_path, workspace).is_ok());
///
/// // Blocked: writing to src directory from Contract stage
/// let src_path = workspace.join("src").join("main.rs");
/// assert!(matches!(
///     validate_write_path(&stage, &src_path, workspace),
///     Err(WriteAllowlistError::WriteNotAllowed { .. })
/// ));
/// ```
pub fn validate_write_path(
    stage: &StageName,
    path: &Path,
    workspace_root: &Path,
) -> Result<(), WriteAllowlistError> {
    validate_path_structure(path)?;
    validate_path_within_workspace(path, workspace_root)?;
    validate_stage_write_permission(stage, path, workspace_root)
}

/// Validates the basic structure of a path (length, characters, traversal).
fn validate_path_structure(path: &Path) -> Result<(), WriteAllowlistError> {
    let path_str = path.to_string_lossy();

    if path_str.is_empty() {
        return Err(WriteAllowlistError::EmptyPath);
    }

    if path_str.len() > MAX_PATH_LEN {
        return Err(WriteAllowlistError::PathTooLong(path_str.len()));
    }

    // Check for forbidden control characters
    if path_str.chars().any(char::is_control) {
        return Err(WriteAllowlistError::PathContainsControlChars);
    }

    // Check for directory traversal
    let path_str_owned = path_str.into_owned();
    if path_str_owned.contains("..") {
        return Err(WriteAllowlistError::PathTraversalDetected(path_str_owned));
    }

    // Ensure absolute path
    if !path.is_absolute() {
        return Err(WriteAllowlistError::RelativePath(path.to_string_lossy().to_string()));
    }

    Ok(())
}

/// Validates that a path is within the workspace root.
fn validate_path_within_workspace(
    path: &Path,
    workspace_root: &Path,
) -> Result<(), WriteAllowlistError> {
    if path.strip_prefix(workspace_root).is_err() {
        return Err(WriteAllowlistError::NotWithinWorkspace {
            path: path.to_string_lossy().to_string(),
            workspace_root: workspace_root.to_string_lossy().to_string(),
        });
    }

    Ok(())
}

/// Validates write permission for a specific stage.
fn validate_stage_write_permission(
    stage: &StageName,
    path: &Path,
    workspace_root: &Path,
) -> Result<(), WriteAllowlistError> {
    let config = StageWriteConfig::for_stage(stage.clone());

    // Check read-only stages
    if config.read_only {
        return Err(WriteAllowlistError::WriteNotAllowed {
            stage: stage.clone(),
            path: path.to_string_lossy().to_string(),
        });
    }

    // Get relative path from workspace root
    let Ok(relative) = path.strip_prefix(workspace_root) else {
        return Err(WriteAllowlistError::NotWithinWorkspace {
            path: path.to_string_lossy().to_string(),
            workspace_root: workspace_root.to_string_lossy().to_string(),
        });
    };

    // Check if path matches any allowed directory
    let in_allowed_dir =
        config.allowed_dirs.iter().any(|allowed_dir| relative.starts_with(allowed_dir));

    if in_allowed_dir {
        return Ok(());
    }

    // Check if path matches any allowed pattern
    let file_name = relative.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let matches_pattern =
        config.allowed_patterns.iter().any(|pattern| matches_glob_pattern(file_name, pattern));

    if matches_pattern {
        return Ok(());
    }

    Err(WriteAllowlistError::WriteNotAllowed {
        stage: stage.clone(),
        path: path.to_string_lossy().to_string(),
    })
}

/// Simple glob pattern matching for file names.
///
/// Supports:
/// - `*` matches any sequence of characters (except /)
/// - `?` matches any single character (except /)
/// - literal characters match themselves
fn matches_glob_pattern(name: &str, pattern: &str) -> bool {
    let name_chars: Vec<char> = name.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();

    matches_glob_helper(&name_chars, &pattern_chars)
}

fn matches_glob_helper(name: &[char], pattern: &[char]) -> bool {
    match (name.first(), pattern.first()) {
        (None, None) => true,
        (None, Some('*')) => {
            // * can match empty string
            matches_glob_helper(name, &pattern[1..])
        }
        (Some(_), Some('*')) => {
            // Try matching * with zero characters or one+ characters
            matches_glob_helper(name, &pattern[1..]) || matches_glob_helper(&name[1..], pattern)
        }
        (Some(_), Some('?')) => matches_glob_helper(&name[1..], &pattern[1..]),
        (Some(n), Some(p)) if *n == *p => matches_glob_helper(&name[1..], &pattern[1..]),
        (Some(_), None | Some(_)) | (None, Some(_)) => false,
    }
}

/// Checks if a write operation would be allowed for a given stage and path.
///
/// This is a convenience function that returns a boolean instead of a Result.
#[must_use]
pub fn is_write_allowed(stage: &StageName, path: &Path, workspace_root: &Path) -> bool {
    validate_write_path(stage, path, workspace_root).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // Path Structure Validation Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn validate_path_structure_rejects_empty_path() {
        let result = validate_path_structure(Path::new(""));
        assert!(matches!(result, Err(WriteAllowlistError::EmptyPath)));
    }

    #[test]
    fn validate_path_structure_rejects_oversized_path() {
        let long_path = "/".repeat(MAX_PATH_LEN + 1);
        let result = validate_path_structure(Path::new(&long_path));
        assert!(matches!(result, Err(WriteAllowlistError::PathTooLong(_))));
    }

    #[test]
    fn validate_path_structure_rejects_control_characters() {
        let path_with_null = "/home/user/file\0.txt";
        let result = validate_path_structure(Path::new(path_with_null));
        assert!(matches!(result, Err(WriteAllowlistError::PathContainsControlChars)));
    }

    #[test]
    fn validate_path_structure_rejects_directory_traversal() {
        let traversal_path = "/home/user/../etc/passwd";
        let result = validate_path_structure(Path::new(traversal_path));
        assert!(matches!(result, Err(WriteAllowlistError::PathTraversalDetected(_))));
    }

    #[test]
    fn validate_path_structure_rejects_relative_paths() {
        let result = validate_path_structure(Path::new("relative/path.txt"));
        assert!(matches!(result, Err(WriteAllowlistError::RelativePath(_))));
    }

    #[test]
    fn validate_path_structure_accepts_valid_absolute_paths() {
        assert!(validate_path_structure(Path::new("/home/user/file.txt")).is_ok());
        assert!(validate_path_structure(Path::new("/tmp/test")).is_ok());
        assert!(validate_path_structure(Path::new("/")).is_ok());
    }

    // ---------------------------------------------------------------------------
    // Workspace Containment Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn validate_path_within_workspace_accepts_subpaths() {
        let workspace = Path::new("/home/user/project");
        let file_path = Path::new("/home/user/project/src/main.rs");
        assert!(validate_path_within_workspace(file_path, workspace).is_ok());
    }

    #[test]
    fn validate_path_within_workspace_rejects_outside_paths() {
        let workspace = Path::new("/home/user/project");
        let outside_path = Path::new("/home/other/file.txt");
        assert!(matches!(
            validate_path_within_workspace(outside_path, workspace),
            Err(WriteAllowlistError::NotWithinWorkspace { .. })
        ));
    }

    #[test]
    fn validate_path_within_workspace_rejects_sibling_paths() {
        let workspace = Path::new("/home/user/project");
        let sibling_path = Path::new("/home/user/project-other/file.txt");
        assert!(matches!(
            validate_path_within_workspace(sibling_path, workspace),
            Err(WriteAllowlistError::NotWithinWorkspace { .. })
        ));
    }

    // ---------------------------------------------------------------------------
    // Contract Stage Write Allowlist Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn contract_stage_allows_docs_directory() {
        let workspace = Path::new("/home/user/project");
        let docs_path = workspace.join("docs").join("contract.md");
        assert!(validate_write_path(&StageName::Contract, &docs_path, workspace).is_ok());
    }

    #[test]
    fn contract_stage_allows_markdown_files() {
        let workspace = Path::new("/home/user/project");
        let md_path = workspace.join("README.md");
        assert!(validate_write_path(&StageName::Contract, &md_path, workspace).is_ok());
    }

    #[test]
    fn contract_stage_blocks_src_directory() {
        let workspace = Path::new("/home/user/project");
        let src_path = workspace.join("src").join("main.rs");
        assert!(matches!(
            validate_write_path(&StageName::Contract, &src_path, workspace),
            Err(WriteAllowlistError::WriteNotAllowed { .. })
        ));
    }

    #[test]
    fn contract_stage_blocks_tests_directory() {
        let workspace = Path::new("/home/user/project");
        let test_path = workspace.join("tests").join("test.rs");
        assert!(matches!(
            validate_write_path(&StageName::Contract, &test_path, workspace),
            Err(WriteAllowlistError::WriteNotAllowed { .. })
        ));
    }

    // ---------------------------------------------------------------------------
    // Implementation Stage Write Allowlist Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn implementation_stage_allows_src_directory() {
        let workspace = Path::new("/home/user/project");
        let src_path = workspace.join("src").join("lib.rs");
        assert!(validate_write_path(&StageName::Implementation, &src_path, workspace).is_ok());
    }

    #[test]
    fn implementation_stage_allows_benches_directory() {
        let workspace = Path::new("/home/user/project");
        let bench_path = workspace.join("benches").join("my_bench.rs");
        assert!(validate_write_path(&StageName::Implementation, &bench_path, workspace).is_ok());
    }

    #[test]
    fn implementation_stage_allows_root_source_files() {
        let workspace = Path::new("/home/user/project");

        // lib.rs
        assert!(validate_write_path(
            &StageName::Implementation,
            &workspace.join("lib.rs"),
            workspace
        )
        .is_ok());

        // main.rs
        assert!(validate_write_path(
            &StageName::Implementation,
            &workspace.join("main.rs"),
            workspace
        )
        .is_ok());

        // build.rs
        assert!(validate_write_path(
            &StageName::Implementation,
            &workspace.join("build.rs"),
            workspace
        )
        .is_ok());
    }

    #[test]
    fn implementation_stage_allows_tests_directory() {
        let workspace = Path::new("/home/user/project");
        let test_path = workspace.join("tests").join("test.rs");
        assert!(validate_write_path(&StageName::Implementation, &test_path, workspace).is_ok());
    }

    #[test]
    fn implementation_stage_blocks_docs_directory() {
        let workspace = Path::new("/home/user/project");
        let docs_path = workspace.join("docs").join("README.md");
        assert!(matches!(
            validate_write_path(&StageName::Implementation, &docs_path, workspace),
            Err(WriteAllowlistError::WriteNotAllowed { .. })
        ));
    }

    // ---------------------------------------------------------------------------
    // ShipGate Stage Write Allowlist Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn ship_gate_stage_allows_beads_directory() {
        let workspace = Path::new("/home/user/project");
        let beads_path = workspace.join(".beads").join("src-123.json");
        assert!(validate_write_path(&StageName::ShipGate, &beads_path, workspace).is_ok());
    }

    #[test]
    fn ship_gate_stage_allows_git_directory() {
        let workspace = Path::new("/home/user/project");
        let git_path = workspace.join(".git").join("MERGE_HEAD");
        assert!(validate_write_path(&StageName::ShipGate, &git_path, workspace).is_ok());
    }

    #[test]
    fn ship_gate_stage_blocks_src_directory() {
        let workspace = Path::new("/home/user/project");
        let src_path = workspace.join("src").join("main.rs");
        assert!(matches!(
            validate_write_path(&StageName::ShipGate, &src_path, workspace),
            Err(WriteAllowlistError::WriteNotAllowed { .. })
        ));
    }

    // ---------------------------------------------------------------------------
    // Glob Pattern Matching Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn glob_star_matches_any_characters() {
        assert!(matches_glob_pattern("file.txt", "*.txt"));
        assert!(matches_glob_pattern("anything.rs", "*.rs"));
        assert!(matches_glob_pattern("test", "*"));
    }

    #[test]
    fn glob_question_matches_single_character() {
        assert!(matches_glob_pattern("file1.txt", "file?.txt"));
        assert!(matches_glob_pattern("fileX.txt", "file?.txt"));
        assert!(!matches_glob_pattern("file10.txt", "file?.txt"));
        assert!(!matches_glob_pattern("file.txt", "file?.txt"));
    }

    #[test]
    fn glob_literal_matches_exactly() {
        assert!(matches_glob_pattern("lib.rs", "lib.rs"));
        assert!(matches_glob_pattern("main.rs", "main.rs"));
        assert!(!matches_glob_pattern("lib_test.rs", "lib.rs"));
    }

    #[test]
    fn glob_combined_patterns() {
        assert!(matches_glob_pattern("my_module_test.rs", "*_test.rs"));
        assert!(matches_glob_pattern("a_test.rs", "*_test.rs"));
        assert!(!matches_glob_pattern("test.rs", "*_test.rs"));
    }

    // ---------------------------------------------------------------------------
    // is_write_allowed Convenience Function Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn is_write_allowed_returns_true_for_valid_writes() {
        let workspace = Path::new("/home/user/project");
        assert!(is_write_allowed(
            &StageName::Contract,
            &workspace.join("docs").join("test.md"),
            workspace
        ));
        assert!(is_write_allowed(
            &StageName::Implementation,
            &workspace.join("src").join("main.rs"),
            workspace
        ));
    }

    #[test]
    fn is_write_allowed_returns_false_for_invalid_writes() {
        let workspace = Path::new("/home/user/project");
        assert!(!is_write_allowed(
            &StageName::Contract,
            &workspace.join("src").join("main.rs"),
            workspace
        ));
        assert!(!is_write_allowed(
            &StageName::ShipGate,
            &workspace.join("src").join("main.rs"),
            workspace
        ));
    }

    // ---------------------------------------------------------------------------
    // Security Boundary Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn blocks_escape_attempts_with_traversal() {
        let workspace = Path::new("/home/user/project");
        let escape_path = workspace.join("docs").join("..").join("src").join("main.rs");
        assert!(matches!(
            validate_write_path(&StageName::Contract, &escape_path, workspace),
            Err(WriteAllowlistError::PathTraversalDetected(_))
        ));
    }

    #[test]
    fn blocks_symlink_escape_attempts_via_workspace_check() {
        let workspace = Path::new("/home/user/project");
        let outside_path = Path::new("/etc/passwd");
        assert!(matches!(
            validate_write_path(&StageName::Implementation, outside_path, workspace),
            Err(WriteAllowlistError::NotWithinWorkspace { .. })
        ));
    }

    #[test]
    fn blocks_cross_stage_contamination() {
        let workspace = Path::new("/home/user/project");

        // Contract stage should not write to src/ or tests/
        assert!(!is_write_allowed(
            &StageName::Contract,
            &workspace.join("src").join("lib.rs"),
            workspace
        ));
        assert!(!is_write_allowed(
            &StageName::Contract,
            &workspace.join("tests").join("test.rs"),
            workspace
        ));

        // ShipGate should not write to src/
        assert!(!is_write_allowed(
            &StageName::ShipGate,
            &workspace.join("src").join("main.rs"),
            workspace
        ));
    }
}
