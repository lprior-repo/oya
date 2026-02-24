use std::path::{Path, PathBuf};
use std::process::Command;

use super::super::OyaError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JjCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JjWorkspaceInfo {
    pub workspace_name: String,
    pub workspace_path: String,
}

pub fn validate_bead_id(bead_id: &str) -> Result<(), OyaError> {
    let forbidden = ['/', '\\'];
    if bead_id.chars().any(|c| forbidden.contains(&c)) {
        return Err(OyaError(format!(
            "bead_id '{}' contains forbidden path separator characters",
            bead_id
        )));
    }
    if bead_id.starts_with("./") || bead_id.starts_with(".\\") {
        return Err(OyaError(format!("bead_id '{}' starts with relative path prefix", bead_id)));
    }
    if bead_id.starts_with("../") || bead_id.starts_with("..\\") {
        return Err(OyaError(format!("bead_id '{}' starts with parent directory prefix", bead_id)));
    }
    Ok(())
}

pub fn run_jj_command(
    args: &[&str],
    timeout_seconds: u64,
    repo_root: &PathBuf,
) -> Result<JjCommandOutput, OyaError> {
    let timeout_str = timeout_seconds.to_string();
    let output = if has_timeout_command() {
        run_with_timeout(args, &timeout_str, repo_root)?
    } else {
        run_with_spawn(args, timeout_seconds, repo_root)?
    };
    Ok(output)
}

fn has_timeout_command() -> bool {
    Command::new("which").arg("timeout").output().is_ok_and(|o| o.status.success())
}

fn run_with_timeout(
    args: &[&str],
    timeout_seconds: &str,
    repo_root: &PathBuf,
) -> Result<JjCommandOutput, OyaError> {
    let output = Command::new("timeout")
        .arg("--signal=TERM")
        .arg("--kill-after=5s")
        .arg(timeout_seconds)
        .arg("jj")
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|e| OyaError(format!("Failed to run jj with timeout: {}", e)))?;
    Ok(parse_command_output(output))
}

fn run_with_spawn(
    args: &[&str],
    timeout_seconds: u64,
    repo_root: &PathBuf,
) -> Result<JjCommandOutput, OyaError> {
    let child = Command::new("jj")
        .args(args)
        .current_dir(repo_root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(spawn_error)?;
    wait_for_child(child, timeout_seconds)
}

fn spawn_error(error: std::io::Error) -> OyaError {
    if error.kind() == std::io::ErrorKind::NotFound {
        OyaError("jj command not found. Please ensure jj is installed and in PATH.".to_string())
    } else {
        OyaError(format!("Failed to spawn jj: {}", error))
    }
}

fn wait_for_child(
    mut child: std::process::Child,
    timeout_seconds: u64,
) -> Result<JjCommandOutput, OyaError> {
    let timeout = std::time::Duration::from_secs(timeout_seconds);
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            let _kill = child.kill();
            let _wait = child.wait();
            return Ok(JjCommandOutput {
                stdout: String::new(),
                stderr: format!("Command timed out after {} seconds", timeout_seconds),
                exit_code: 124,
            });
        }
        if let Some(result) = child_try_wait(&mut child)? {
            return Ok(result);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn child_try_wait(child: &mut std::process::Child) -> Result<Option<JjCommandOutput>, OyaError> {
    match child.try_wait() {
        Ok(Some(status)) => {
            let exit_code = status.code().unwrap_or(128);
            let stdout = read_pipe_to_string(child.stdout.take());
            let stderr = read_pipe_to_string(child.stderr.take());
            Ok(Some(JjCommandOutput { stdout, stderr, exit_code }))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(OyaError(format!("Failed to wait for jj process: {}", e))),
    }
}

fn read_pipe_to_string<T: std::io::Read>(pipe: Option<T>) -> String {
    pipe.map_or_else(String::new, |mut p| {
        let mut s = String::new();
        let _ = std::io::Read::read_to_string(&mut p, &mut s);
        s
    })
}

fn parse_command_output(output: std::process::Output) -> JjCommandOutput {
    JjCommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(128),
    }
}

pub fn create_workspace(bead_id: &str, repo_root: &PathBuf) -> Result<JjWorkspaceInfo, OyaError> {
    validate_bead_id(bead_id)?;
    let workspace_name = format!("oya-{}", bead_id);
    let workspace_path = repo_root.join(&workspace_name);
    prepare_workspace_destination(&workspace_name, &workspace_path, repo_root)?;
    add_workspace_with_recovery(&workspace_name, &workspace_path, repo_root)?;
    let workspace_path = workspace_path.to_string_lossy().to_string();
    Ok(JjWorkspaceInfo { workspace_name, workspace_path })
}

pub fn forget_workspace(bead_id: &str, repo_root: &PathBuf) -> Result<(), OyaError> {
    validate_bead_id(bead_id)?;
    let workspace_name = format!("oya-{}", bead_id);
    forget_workspace_by_name(&workspace_name, repo_root)
}

fn prepare_workspace_destination(
    workspace_name: &str,
    workspace_path: &Path,
    repo_root: &PathBuf,
) -> Result<(), OyaError> {
    forget_workspace_by_name(workspace_name, repo_root)?;
    clear_workspace_path(workspace_path)?;
    Ok(())
}

fn add_workspace_with_recovery(
    workspace_name: &str,
    workspace_path: &Path,
    repo_root: &PathBuf,
) -> Result<(), OyaError> {
    let args = workspace_add_args(workspace_name, workspace_path);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = run_jj_command(&arg_refs, 30, repo_root)?;
    if result.exit_code == 0 {
        return Ok(());
    }
    if should_recover_workspace(&result.stderr) {
        prepare_workspace_destination(workspace_name, workspace_path, repo_root)?;
        let retry = run_jj_command(&arg_refs, 30, repo_root)?;
        if retry.exit_code == 0 {
            return Ok(());
        }
        return Err(OyaError(format!(
            "Failed to create workspace '{}' after recovery: {}",
            workspace_name, retry.stderr
        )));
    }
    Err(OyaError(format!("Failed to create workspace '{}': {}", workspace_name, result.stderr)))
}

fn workspace_add_args(workspace_name: &str, workspace_path: &Path) -> Vec<String> {
    vec![
        "workspace".to_string(),
        "add".to_string(),
        "--name".to_string(),
        workspace_name.to_string(),
        "--revision".to_string(),
        "@".to_string(),
        workspace_path.to_string_lossy().to_string(),
    ]
}

fn forget_workspace_by_name(workspace_name: &str, repo_root: &PathBuf) -> Result<(), OyaError> {
    let args = ["workspace", "forget", workspace_name];
    let result = run_jj_command(&args, 30, repo_root)?;
    if result.exit_code == 0 || is_workspace_not_found(&result.stderr) {
        return Ok(());
    }
    if is_concurrent_modification(&result.stderr) {
        let retry = run_jj_command(&args, 30, repo_root)?;
        if retry.exit_code == 0 || is_workspace_not_found(&retry.stderr) {
            return Ok(());
        }
        return Err(OyaError(format!(
            "Failed to forget workspace '{}': {}",
            workspace_name, retry.stderr
        )));
    }
    Err(OyaError(format!("Failed to forget workspace '{}': {}", workspace_name, result.stderr)))
}

fn clear_workspace_path(workspace_path: &Path) -> Result<(), OyaError> {
    if !workspace_path.exists() {
        return Ok(());
    }
    if workspace_path.is_dir() {
        std::fs::remove_dir_all(workspace_path).map_err(|error| {
            OyaError(format!(
                "Failed to remove existing workspace path '{}': {}",
                workspace_path.display(),
                error
            ))
        })
    } else {
        Err(OyaError(format!(
            "Workspace path '{}' exists but is not a directory",
            workspace_path.display()
        )))
    }
}

fn is_workspace_not_found(stderr: &str) -> bool {
    stderr.contains("No such workspace") || stderr.contains("not found")
}

fn is_workspace_already_exists(stderr: &str) -> bool {
    stderr.contains("already exists")
        || stderr.contains("Workspace") && stderr.contains("exists")
        || stderr.contains("Concurrent modification detected")
}

fn should_recover_workspace(stderr: &str) -> bool {
    is_workspace_already_exists(stderr)
        || is_concurrent_modification(stderr)
        || stderr.contains("Destination path exists and is not an empty directory")
}

fn is_concurrent_modification(stderr: &str) -> bool {
    stderr.contains("Concurrent modification detected")
}

#[allow(dead_code)]
pub fn rebase_onto_main(bead_id: &str, repo_root: &PathBuf) -> Result<(), OyaError> {
    validate_bead_id(bead_id)?;
    let workspace_name = format!("oya-{}", bead_id);
    let args = ["rebase", "-s", workspace_name.as_str(), "-d", "main"];
    let result = run_jj_command(&args, 60, repo_root)?;
    if result.exit_code != 0 {
        return Err(OyaError(format!(
            "Failed to rebase workspace '{}': {}",
            workspace_name, result.stderr
        )));
    }
    Ok(())
}

#[allow(dead_code)]
pub fn bookmark_and_push(bead_id: &str, repo_root: &PathBuf) -> Result<(), OyaError> {
    validate_bead_id(bead_id)?;
    let bookmark_name = format!("oya-{}", bead_id);
    let create_args = ["bookmark", "create", bookmark_name.as_str()];
    let create_result = run_jj_command(&create_args, 30, repo_root)?;
    if create_result.exit_code != 0 && !is_bookmark_exists(&create_result.stderr) {
        return Err(OyaError(format!(
            "Failed to create bookmark '{}': {}",
            bookmark_name, create_result.stderr
        )));
    }
    let push_args = ["bookmark", "push", bookmark_name.as_str()];
    let push_result = run_jj_command(&push_args, 60, repo_root)?;
    if push_result.exit_code != 0 {
        return Err(OyaError(format!(
            "Failed to push bookmark '{}': {}",
            bookmark_name, push_result.stderr
        )));
    }
    Ok(())
}

#[allow(dead_code)]
fn is_bookmark_exists(stderr: &str) -> bool {
    stderr.contains("already exists") || stderr.contains("Duplicate")
}
