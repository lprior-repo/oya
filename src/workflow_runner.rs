use super::*;
use crate::runtime_tools::{sanitize_arg, validate_command};
use std::process::Command;

mod poller;
mod types;

use poller::{
    emit_workflow_starting, emit_workflow_submitted, output_result, poll_until_complete,
    start_workflow, workflow_http_client, workflow_result_from_status,
};
use types::WorkflowConfig;

const PIPELINE_STAGES: &[&str] = &["contract", "implementation", "ship_gate"];

fn find_repo_root() -> Result<PathBuf, String> {
    let current =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    std::iter::successors(Some(current.as_path()), |p| p.parent())
        .find(|p| p.join(".beads").exists())
        .map(PathBuf::from)
        .ok_or_else(|| {
            format!("No .beads/ directory found in {} or any parent directory", current.display())
        })
}

/// Check if a command is available on the system.
///
/// Returns `true` if the command can be found, `false` otherwise.
/// This is a pure check that does not propagate errors.
fn is_command_available(name: &str) -> bool {
    // Validate command name against whitelist
    let validated = match validate_command(name) {
        Ok(cmd) => cmd,
        Err(_) => return false,
    };
    Command::new("which").arg(&validated).output().is_ok_and(|output| output.status.success())
}

fn validate_bead_exists(bead_id: &str, repo_root: &PathBuf) -> Result<bool, String> {
    // Sanitize bead_id to prevent command injection
    let safe_bead_id =
        sanitize_arg("bead_id", bead_id).map_err(|e| format!("Invalid bead_id: {}", e))?;

    if !is_command_available("br") {
        return Err(
            "The 'br' command is not found. Please install it first: https://github.com/your-org/br"
                .to_string(),
        );
    }

    let output = Command::new("br")
        .args(["show", &safe_bead_id, "--json"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("Failed to run 'br show {}': {}", safe_bead_id, e))?;

    Ok(output.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_command_available_returns_true_for_common_commands() {
        // 'echo' is available on virtually all Unix systems
        assert!(is_command_available("echo") || is_command_available("ls"));
    }

    #[test]
    fn test_is_command_available_returns_false_for_nonexistent_command() {
        assert!(!is_command_available("nonexistent_command_that_does_not_exist_xyz"));
    }

    #[test]
    fn test_validate_bead_exists_returns_error_when_br_not_available() {
        // This test verifies the error message is clear when br is not installed
        // We test by checking that the function handles the "br not available" case
        // In an environment where br IS available, we can't fully test this path,
        // but we can verify the is_command_available helper works correctly

        // If br is available, this will attempt to run it
        // If br is not available, it should return a clear error
        let repo_root = PathBuf::from("/tmp");

        if !is_command_available("br") {
            let result = validate_bead_exists("test-bead", &repo_root);
            match result {
                Err(err) => {
                    assert!(
                        err.contains("'br' command is not found"),
                        "error message should mention 'br' command is not found, got: {}",
                        err
                    );
                    assert!(
                        err.contains("install"),
                        "error message should suggest installing, got: {}",
                        err
                    );
                }
                Ok(_) => {
                    // This should not happen when br is not available
                    panic!("validate_bead_exists should return error when br is not available");
                }
            }
        }
        // If br IS available, the test is effectively skipped
        // (integration tests would cover the actual br invocation)
    }
}

pub(super) async fn run_workflow(args: RunArgs) -> Result<(), DynError> {
    let repo_root = find_repo_root()
        .map_err(|e| format!("Failed to find repo root (no .beads/ directory found): {}", e))?;

    if !validate_bead_exists(&args.bead_id, &repo_root)? {
        return Err(format!(
            "Bead '{}' not found. Run 'br list' to see available beads.",
            args.bead_id
        )
        .into());
    }

    let oya_config = oya::config::load_config(&repo_root)
        .map_err(|err| format!("Failed to load oya config: {}", err))?;
    let workflow_config = WorkflowConfig::from_args(args, repo_root, &oya_config);
    execute_workflow(workflow_config).await
}

async fn execute_workflow(config: WorkflowConfig) -> Result<(), DynError> {
    let client = workflow_http_client()?;
    emit_workflow_starting(&config);
    start_workflow(&client, &config).await?;
    emit_workflow_submitted(&config);
    let final_status = poll_until_complete(&client, &config).await?;
    let result = workflow_result_from_status(&config, &final_status);
    output_result(&result, &config)?;
    if result.status == "shipped" {
        Ok(())
    } else {
        Err(format!("Workflow ended with status: {}", result.status).into())
    }
}
