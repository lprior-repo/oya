use super::{
    bookmark_and_push, create_workspace, forget_workspace, rebase_onto_main, run_jj_command,
    validate_bead_id,
};
use std::path::PathBuf;
use std::process::Command;

fn init_temp_jj_repo() -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| {
        panic!("failed to create temporary directory for jj tests: {}", error)
    });
    let output = Command::new("jj")
        .arg("git")
        .arg("init")
        .current_dir(temp_dir.path())
        .output()
        .unwrap_or_else(|error| panic!("failed to launch 'jj git init': {}", error));
    assert!(
        output.status.success(),
        "jj git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    temp_dir
}

fn run_jj_direct(args: &[&str], repo_root: &std::path::Path) {
    let output = Command::new("jj")
        .args(args)
        .current_dir(repo_root)
        .output()
        .unwrap_or_else(|error| panic!("failed to launch jj {:?}: {}", args, error));
    assert!(
        output.status.success(),
        "jj {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_validate_bead_id_rejects_path_separators() {
    let result = validate_bead_id("src/foo");
    assert!(result.is_err(), "bead_id with / should be rejected");

    let result = validate_bead_id("src\\bar");
    assert!(result.is_err(), "bead_id with \\ should be rejected");

    let result = validate_bead_id("./hidden");
    assert!(result.is_err(), "bead_id with ./ should be rejected");

    let result = validate_bead_id("../escape");
    assert!(result.is_err(), "bead_id with ../ should be rejected");
}

#[test]
fn test_validate_bead_id_accepts_valid_bead_ids() {
    let result = validate_bead_id("src-1uc");
    assert!(result.is_ok(), "valid bead_id src-1uc should be accepted");

    let result = validate_bead_id("feature-123");
    assert!(result.is_ok(), "valid bead_id feature-123 should be accepted");

    let result = validate_bead_id("abc123");
    assert!(result.is_ok(), "valid bead_id abc123 should be accepted");

    let result = validate_bead_id("a_b_c");
    assert!(result.is_ok(), "valid bead_id a_b_c should be accepted");
}

#[test]
fn test_run_jj_command_returns_output() {
    let temp_repo = init_temp_jj_repo();
    let repo_root = PathBuf::from(temp_repo.path());
    let result = run_jj_command(&["version"], 30, &repo_root);

    assert!(result.is_ok(), "run_jj_command should return Ok for valid command");
    match result {
        Ok(output) => assert_eq!(output.exit_code, 0, "jj version should succeed"),
        Err(error) => panic!("expected successful jj version command, got error: {}", error),
    }
}

#[test]
fn test_create_workspace_returns_info() {
    let temp_repo = init_temp_jj_repo();
    let repo_root = PathBuf::from(temp_repo.path());
    let result = create_workspace("test-bead-workspace", &repo_root);

    assert!(result.is_ok(), "create_workspace should return Ok");
    match result {
        Ok(info) => {
            assert_eq!(info.workspace_name, "oya-test-bead-workspace");
            assert!(PathBuf::from(info.workspace_path).exists());
        }
        Err(error) => panic!("expected workspace info, got error: {}", error),
    }
}

#[test]
fn test_create_workspace_recovers_stale_directory_contents() {
    let temp_repo = init_temp_jj_repo();
    let repo_root = PathBuf::from(temp_repo.path());

    let first = create_workspace("stale-dir", &repo_root);
    assert!(first.is_ok(), "initial create_workspace should succeed");

    let workspace_path = repo_root.join("oya-stale-dir");
    let stale_file = workspace_path.join("stale.txt");
    let write_result = std::fs::write(&stale_file, "stale");
    assert!(write_result.is_ok(), "failed to write stale file in workspace");
    assert!(stale_file.exists(), "stale file should exist before refresh");

    let second = create_workspace("stale-dir", &repo_root);
    assert!(
        second.is_ok(),
        "create_workspace should recover from non-empty stale directory: {:?}",
        second.err()
    );
    assert!(!stale_file.exists(), "stale file should be removed by workspace refresh");
}

#[test]
fn test_create_workspace_recovers_stale_record_without_directory() {
    let temp_repo = init_temp_jj_repo();
    let repo_root = PathBuf::from(temp_repo.path());

    let first = create_workspace("stale-record", &repo_root);
    assert!(first.is_ok(), "initial create_workspace should succeed");

    let workspace_path = repo_root.join("oya-stale-record");
    let remove_result = std::fs::remove_dir_all(&workspace_path);
    assert!(
        remove_result.is_ok(),
        "expected to remove workspace directory for stale-record simulation"
    );

    let second = create_workspace("stale-record", &repo_root);
    assert!(
        second.is_ok(),
        "create_workspace should recover from stale jj workspace record: {:?}",
        second.err()
    );
    assert!(workspace_path.exists(), "workspace path should be recreated");
}

#[test]
fn test_create_workspace_uses_current_working_copy_content() {
    let temp_repo = init_temp_jj_repo();
    let repo_root = PathBuf::from(temp_repo.path());

    let tracked_file = repo_root.join("tracked.txt");
    let initial_write = std::fs::write(&tracked_file, "base\n");
    assert!(initial_write.is_ok(), "failed to write tracked file");
    run_jj_direct(&["file", "track", "tracked.txt"], temp_repo.path());

    let updated_write = std::fs::write(&tracked_file, "updated\n");
    assert!(updated_write.is_ok(), "failed to update tracked file");

    let created = create_workspace("sync-current", &repo_root);
    assert!(created.is_ok(), "create_workspace should succeed: {:?}", created.err());

    let workspace_file = repo_root.join("oya-sync-current").join("tracked.txt");
    let workspace_contents = std::fs::read_to_string(&workspace_file)
        .unwrap_or_else(|error| panic!("failed to read workspace tracked file: {}", error));
    assert_eq!(workspace_contents, "updated\n");
}

#[test]
fn test_forget_workspace_returns_ok() {
    let temp_repo = init_temp_jj_repo();
    let repo_root = PathBuf::from(temp_repo.path());
    let created = create_workspace("test-bead-forget", &repo_root);
    assert!(created.is_ok(), "create_workspace setup should succeed");

    let result = forget_workspace("test-bead-forget", &repo_root);
    assert!(result.is_ok(), "forget_workspace should return Ok");

    let second_forget = forget_workspace("test-bead-forget", &repo_root);
    assert!(second_forget.is_ok(), "forget_workspace should be idempotent");
}

#[test]
fn test_rebase_onto_main_rejects_invalid_bead_id() {
    let repo_root = PathBuf::from(".");
    let result = rebase_onto_main("../invalid", &repo_root);
    assert!(result.is_err(), "rebase_onto_main should reject path traversal");
}

#[test]
fn test_bookmark_and_push_rejects_invalid_bead_id() {
    let repo_root = PathBuf::from(".");
    let result = bookmark_and_push("../invalid", &repo_root);
    assert!(result.is_err(), "bookmark_and_push should reject path traversal");
}
