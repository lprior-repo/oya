//! Worktree module - Git/jj workspace isolation.
//!
//! Manages creating, retrieving, and removing isolated work directories.

use std::path::{Path, PathBuf};

use crate::{
    domain::Language,
    error::{Error, Result},
    process::{dir_exists, run_command},
    quality_gates::enforce_functional_quality,
    repo::detect_language,
};

/// Directory name for workspaces.
const WORKSPACES_DIR: &str = ".factory-workspaces";

/// Factory configuration directory.
const FACTORY_DIR: &str = ".factory";

/// Branch prefix for feature branches.
const BRANCH_PREFIX: &str = "feat/";

/// Worktree information.
#[derive(Debug, Clone)]
pub struct Worktree {
    pub slug: String,
    pub path: PathBuf,
    pub branch: String,
    pub language: Language,
}

/// Create a new worktree for a task.
pub fn create_worktree(slug: &str, language: Language, repo_root: &Path) -> Result<Worktree> {
    check_slug_not_exists(slug, repo_root)?;

    let workspaces_base = repo_root.join(WORKSPACES_DIR);
    let unique_id = generate_unique_id();
    let worktree_name = format!("{slug}-{unique_id}");
    let worktree_path = workspaces_base.join(&worktree_name);
    let branch = format!("{BRANCH_PREFIX}{slug}");

    tracing::info!(slug, ?worktree_path, branch, "Creating worktree");

    create_base_dir(&workspaces_base, repo_root)?;
    create_jj_workspace(&worktree_name, &worktree_path, repo_root)?;
    create_bookmark(&worktree_path, &branch, repo_root);
    create_symlink(&worktree_path, slug, repo_root)?;

    tracing::info!(slug, ?worktree_path, "Worktree created");

    Ok(Worktree {
        slug: slug.to_string(),
        path: worktree_path,
        branch,
        language,
    })
}

/// Check that slug doesn't already exist.
fn check_slug_not_exists(slug: &str, repo_root: &Path) -> Result<()> {
    let symlink_path = repo_root.join(FACTORY_DIR).join(slug);

    if symlink_path.exists() {
        return Err(Error::WorktreeExists {
            slug: slug.to_string(),
        });
    }

    Ok(())
}

/// Create the base workspaces directory.
fn create_base_dir(workspaces_base: &Path, repo_root: &Path) -> Result<()> {
    run_command_checked(
        "mkdir",
        &["-p", &workspaces_base.to_string_lossy()],
        repo_root,
    )
}

/// Create a jj workspace, falling back to git worktree.
fn create_jj_workspace(worktree_name: &str, worktree_path: &Path, repo_root: &Path) -> Result<()> {
    let worktree_str = worktree_path.to_string_lossy();

    let jj_result = run_command(
        "jj",
        &["workspace", "add", "--name", worktree_name, &worktree_str],
        repo_root,
    );

    match jj_result {
        Ok(r) if r.is_success() => Ok(()),
        _ => create_git_worktree(worktree_name, worktree_path, repo_root),
    }
}

/// Create a git worktree as fallback.
fn create_git_worktree(worktree_name: &str, worktree_path: &Path, repo_root: &Path) -> Result<()> {
    let branch = format!("{BRANCH_PREFIX}{worktree_name}");
    let repo_str = repo_root.to_string_lossy();
    let worktree_str = worktree_path.to_string_lossy();

    run_command_checked(
        "git",
        &[
            "-C",
            &repo_str,
            "worktree",
            "add",
            &worktree_str,
            "-b",
            &branch,
        ],
        repo_root,
    )
}

/// Create a bookmark/branch for the worktree.
fn create_bookmark(worktree_path: &Path, branch: &str, repo_root: &Path) {
    let worktree_str = worktree_path.to_string_lossy();

    // Try jj bookmark first, git branch is already created by worktree add
    let _ = run_command(
        "jj",
        &["-R", &worktree_str, "bookmark", "create", branch],
        repo_root,
    );
}

/// Create symlink in .factory directory.
fn create_symlink(worktree_path: &Path, slug: &str, repo_root: &Path) -> Result<()> {
    let symlink_dir = repo_root.join(FACTORY_DIR);
    let worktree_str = worktree_path.to_string_lossy();
    let symlink_path = symlink_dir.join(slug);
    let symlink_str = symlink_path.to_string_lossy();

    run_command_checked("mkdir", &["-p", &symlink_dir.to_string_lossy()], repo_root)?;
    run_command_checked("ln", &["-sf", &worktree_str, &symlink_str], repo_root)
}

/// Run a command and check for success.
fn run_command_checked(cmd: &str, args: &[&str], cwd: &Path) -> Result<()> {
    run_command(cmd, args, cwd)?
        .check_success()
        .map_err(|_| Error::WorktreeCreationFailed {
            reason: format!("{cmd} failed"),
        })
}

/// Get worktree information by slug.
pub fn get_worktree(slug: &str, repo_root: &Path) -> Result<Worktree> {
    let symlink_path = repo_root.join(FACTORY_DIR).join(slug);

    let output =
        run_command("readlink", &[&symlink_path.to_string_lossy()], repo_root).map_err(|_| {
            Error::WorktreeNotFound {
                slug: slug.to_string(),
            }
        })?;

    if !output.is_success() {
        return Err(Error::WorktreeNotFound {
            slug: slug.to_string(),
        });
    }

    let worktree_path = PathBuf::from(output.stdout.trim());
    let language = detect_language(&worktree_path).unwrap_or(Language::Go);

    Ok(Worktree {
        slug: slug.to_string(),
        path: worktree_path,
        branch: format!("{BRANCH_PREFIX}{slug}"),
        language,
    })
}

/// Remove a worktree.
pub fn remove_worktree(slug: &str, repo_root: &Path) -> Result<()> {
    tracing::info!(slug, "Removing worktree");

    let wt = get_worktree(slug, repo_root)?;
    let repo_str = repo_root.to_string_lossy();

    // Try jj workspace forget first
    let _ = run_command(
        "jj",
        &["-R", &repo_str, "workspace", "forget", &format!("{slug}-*")],
        repo_root,
    );

    // Also try git worktree remove
    let _ = run_command(
        "git",
        &[
            "-C",
            &repo_str,
            "worktree",
            "remove",
            &wt.path.to_string_lossy(),
            "--force",
        ],
        repo_root,
    );

    // Remove directory manually if needed
    let _ = run_command("rm", &["-rf", &wt.path.to_string_lossy()], repo_root);

    // Remove symlink
    let symlink_path = repo_root.join(FACTORY_DIR).join(slug);
    let _ = run_command("rm", &["-f", &symlink_path.to_string_lossy()], repo_root);

    tracing::info!(slug, ?wt.path, "Worktree removed");

    Ok(())
}

/// List all worktrees.
pub fn list_worktrees(repo_root: &Path) -> Result<Vec<Worktree>> {
    let factory_path = repo_root.join(FACTORY_DIR);

    if !dir_exists(&factory_path) {
        return Ok(Vec::new());
    }

    let output = run_command("ls", &["-1", &factory_path.to_string_lossy()], repo_root)?;

    if !output.is_success() {
        return Ok(Vec::new());
    }

    output
        .stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|slug| get_worktree(slug.trim(), repo_root))
        .collect()
}

/// Get the workspaces base path.
#[must_use]
pub fn workspaces_base(repo_root: &Path) -> PathBuf {
    repo_root.join(WORKSPACES_DIR)
}

/// Generate a unique ID for worktree names.
fn generate_unique_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let nanos = duration.as_nanos();
    format!("{}", nanos % 100_000_000)
}

/// Create a worktree from a bead specification with functional quality enforcement.
///
/// This function is the main entry point for pulling down bead tasks.
/// It ensures all generated/modified code follows strict functional Rust patterns.
pub fn create_worktree_from_bead(
    bead_id: &str,
    bead_spec_json: &str,
    language: Language,
    repo_root: &Path,
) -> Result<Worktree> {
    tracing::info!(
        bead_id,
        ?language,
        "Creating worktree from bead specification"
    );

    // Parse bead spec
    let spec = crate::codegen::parse_bead_spec(bead_spec_json)
        .map_err(|e| Error::invalid_record(format!("Invalid bead spec: {e}")))?;

    // Create worktree
    let worktree = create_worktree(bead_id, language, repo_root)?;

    // Generate functional code from bead spec
    let generated_code = crate::codegen::generate_from_bead(&spec);

    // Write generated code to src/lib.rs
    let lib_path = worktree.path.join("src/lib.rs");
    std::fs::create_dir_all(
        lib_path
            .parent()
            .ok_or_else(|| Error::invalid_record("Cannot create src directory"))?,
    )
    .map_err(|e| Error::directory_creation_failed(lib_path.parent().as_ref(), e.to_string()))?;

    std::fs::write(&lib_path, generated_code)
        .map_err(|e| Error::file_write_failed(&lib_path, e.to_string()))?;

    tracing::info!(?lib_path, "Generated functional code from bead spec");

    // Enforce functional quality gate
    tracing::info!(bead_id, "Running functional quality gate");
    enforce_functional_quality(&worktree.path).map_err(|e| {
        tracing::error!(error = %e, "Functional quality gate failed");
        Error::invalid_record(format!(
            "Code from bead '{}' does not meet functional requirements: {}",
            bead_id, e
        ))
    })?;

    tracing::info!(
        bead_id,
        ?worktree.path,
        "Worktree created and functional quality enforced"
    );

    Ok(worktree)
}

/// Validate that an existing worktree meets functional requirements.
pub fn validate_worktree_functional(slug: &str, repo_root: &Path) -> Result<()> {
    tracing::info!(slug, "Validating worktree functional compliance");

    let worktree = get_worktree(slug, repo_root)?;

    enforce_functional_quality(&worktree.path).map_err(|e| {
        Error::invalid_record(format!(
            "Worktree '{}' does not meet functional requirements: {}",
            slug, e
        ))
    })?;

    tracing::info!(slug, "Worktree functional validation passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_unique_id() {
        let id1 = generate_unique_id();
        let id2 = generate_unique_id();
        // IDs should be numeric strings
        assert!(id1.chars().all(|c| c.is_ascii_digit()));
        assert!(id2.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_workspaces_base() {
        let path = workspaces_base(Path::new("/repo"));
        assert_eq!(path, PathBuf::from("/repo/.factory-workspaces"));
    }
}
