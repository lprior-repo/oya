#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use super::commands::run_command_outcome;

const GH_REPO_VIEW_ARGS: &[&str] = &["repo", "view", "--json", "nameWithOwner"];

pub async fn detect_repo_slug() -> anyhow::Result<Option<String>> {
    let output = run_command_outcome("gh", GH_REPO_VIEW_ARGS, None).await?;
    if !output.success {
        return Ok(None);
    }
    extract_repo_slug_from_gh_output(&output.stdout).map(Some)
}

fn extract_repo_slug_from_gh_output(raw: &str) -> anyhow::Result<String> {
    #[derive(Debug, serde::Deserialize)]
    struct GhRepoView {
        #[serde(rename = "nameWithOwner")]
        name_with_owner: String,
    }

    let payload: GhRepoView = serde_json::from_str(raw)?;
    crate::cli::args::parse_repo_slug(&payload.name_with_owner).map_err(anyhow::Error::msg)
}
