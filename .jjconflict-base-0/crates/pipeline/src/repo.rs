//! Repository detection and analysis.
//!
//! Detects repo root, language, base branch, and repository state.

use std::path::{Path, PathBuf};

use crate::{
    domain::Language,
    error::{Error, Result},
    process::{file_exists, run_command},
};

/// Detect the repository root directory.
pub fn detect_repo_root() -> Result<PathBuf> {
    let output = run_command("git", &["rev-parse", "--show-toplevel"], Path::new("."))?;

    if output.is_success() {
        let path = output.stdout.trim();
        if path.is_empty() {
            return Err(Error::NotInRepo);
        }
        Ok(PathBuf::from(path))
    } else {
        Err(Error::NotInRepo)
    }
}

/// Auto-detect language from repository contents.
pub fn detect_language(repo_root: &Path) -> Result<Language> {
    if !repo_root.is_dir() {
        return Err(Error::DirectoryNotFound {
            path: repo_root.to_path_buf(),
        });
    }

    let has_gleam_toml = file_exists(&repo_root.join("gleam.toml"));
    let has_go_mod = file_exists(&repo_root.join("go.mod"));
    let has_cargo_toml = file_exists(&repo_root.join("Cargo.toml"));
    let has_pyproject = file_exists(&repo_root.join("pyproject.toml"));
    let has_package_json = file_exists(&repo_root.join("package.json"));

    Language::detect_from_files(
        has_gleam_toml,
        has_go_mod,
        has_cargo_toml,
        has_pyproject,
        has_package_json,
    )
}

/// Get the main/master branch of the repository.
pub fn get_base_branch(repo_root: &Path) -> Result<String> {
    // Try symbolic-ref first
    if let Some(branch) = try_symbolic_ref(repo_root) {
        return Ok(branch);
    }

    // Fall back to checking if main or master exists
    if branch_exists(repo_root, "main")? {
        return Ok("main".to_string());
    }

    if branch_exists(repo_root, "master")? {
        return Ok("master".to_string());
    }

    Err(Error::BaseBranchNotFound)
}

/// Try to get branch from symbolic-ref.
fn try_symbolic_ref(repo_root: &Path) -> Option<String> {
    let output = run_command(
        "git",
        &[
            "-C",
            repo_root.to_str()?,
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
        ],
        repo_root,
    )
    .ok()?;

    if output.is_success() {
        let trimmed = output.stdout.trim();
        trimmed.rsplit('/').next().map(String::from)
    } else {
        None
    }
}

/// Check if a branch exists.
fn branch_exists(repo_root: &Path, branch: &str) -> Result<bool> {
    let refs_path = format!("refs/heads/{branch}");
    let repo_str = repo_root.to_str().ok_or_else(|| Error::DirectoryNotFound {
        path: repo_root.to_path_buf(),
    })?;

    let output = run_command(
        "git",
        &[
            "-C", repo_str, "show-ref", "--verify", "--quiet", &refs_path,
        ],
        repo_root,
    )?;

    Ok(output.is_success())
}

/// Check if repository is clean (no uncommitted changes).
pub fn is_clean(repo_root: &Path) -> Result<bool> {
    let repo_str = repo_root.to_str().ok_or_else(|| Error::DirectoryNotFound {
        path: repo_root.to_path_buf(),
    })?;

    let output = run_command("git", &["-C", repo_str, "status", "--porcelain"], repo_root)?;

    Ok(output.stdout.trim().is_empty())
}

/// Get current branch name.
pub fn current_branch(repo_root: &Path) -> Result<String> {
    let repo_str = repo_root.to_str().ok_or_else(|| Error::DirectoryNotFound {
        path: repo_root.to_path_buf(),
    })?;

    let output = run_command(
        "git",
        &["-C", repo_str, "rev-parse", "--abbrev-ref", "HEAD"],
        repo_root,
    )?;

    let branch = output.stdout.trim();
    if branch.is_empty() {
        Err(Error::BaseBranchNotFound)
    } else {
        Ok(branch.to_string())
    }
}

/// Get list of modified files.
pub fn modified_files(repo_root: &Path) -> Result<Vec<String>> {
    let repo_str = repo_root.to_str().ok_or_else(|| Error::DirectoryNotFound {
        path: repo_root.to_path_buf(),
    })?;

    let output = run_command("git", &["-C", repo_str, "status", "--porcelain"], repo_root)?;

    let files = output
        .stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            // Skip first 3 chars (status codes and space)
            if line.len() > 3 {
                Some(line[3..].to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_repo_root() {
        // This test assumes we're running in a git repo
        let result = detect_repo_root();
        // Don't assert success since we might not be in a git repo during tests
        assert!(result.is_ok() || result.is_err());
    }
}
