#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub async fn detect_repo_slug() -> anyhow::Result<Option<String>> {
    let output = tokio::process::Command::new("gh")
        .args(["repo", "view", "--json", "nameWithOwner"])
        .output()
        .await
        .map_err(|error| anyhow::anyhow!("failed to run gh repo view: {error}"))?;
    if !output.status.success() {
        return Ok(None);
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| anyhow::anyhow!("gh output was not UTF-8: {error}"))?;
    extract_repo_slug_from_gh_output(&stdout).map(Some)
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
