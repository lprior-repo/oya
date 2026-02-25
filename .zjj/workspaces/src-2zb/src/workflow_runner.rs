use super::*;
use std::process::Command;

mod poller;
mod types;

use poller::{
    emit_workflow_starting, emit_workflow_submitted, output_result, poll_until_complete,
    start_workflow, workflow_http_client, workflow_result_from_status,
};
use types::WorkflowConfig;

const PIPELINE_STAGES: &[&str] =
    &["explore", "contract", "red", "implementation", "witness", "ship_gate"];

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

fn validate_bead_exists(bead_id: &str, repo_root: &PathBuf) -> Result<bool, String> {
    let br_available = Command::new("which")
        .arg("br")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !br_available {
        return Err(
            "The 'br' command is not found. Please install it first: https://github.com/your-org/br"
                .to_string(),
        );
    }

    let output = Command::new("br")
        .args(["show", bead_id, "--json"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "The 'br' command is not found. Please install it first.".to_string()
            } else {
                format!("Failed to run 'br show {}': {}", bead_id, e)
            }
        })?;

    Ok(output.status.success())
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
