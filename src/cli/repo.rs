#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use tokio::time::{sleep, Duration};

const REPO_LOOKUP_BACKOFFS: [Duration; 3] =
    [Duration::from_secs(120), Duration::from_secs(120), Duration::from_secs(120)];

pub async fn resolve_repo_slug(repo: Option<String>) -> anyhow::Result<Option<String>> {
    let jj_repo = detect_jj_origin_repo_slug().await?;
    let selected = pick_repo_slug(repo, jj_repo.as_deref()).await?;
    ensure_repo_matches_jj_origin(selected.as_str(), jj_repo.as_deref())?;
    ensure_repo_exists(selected.as_str()).await?;
    Ok(Some(selected))
}

async fn detect_repo_slug() -> anyhow::Result<Option<String>> {
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

async fn detect_jj_origin_repo_slug() -> anyhow::Result<Option<String>> {
    let output = tokio::process::Command::new("jj")
        .args(["git", "remote", "list"])
        .output()
        .await
        .map_err(|error| anyhow::anyhow!("failed to run jj git remote list: {error}"))?;
    if !output.status.success() {
        return Ok(None);
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| anyhow::anyhow!("jj output was not UTF-8: {error}"))?;
    extract_repo_slug_from_jj_remote_output(&stdout)
}

async fn pick_repo_slug(explicit: Option<String>, jj_repo: Option<&str>) -> anyhow::Result<String> {
    if let Some(value) = explicit {
        return super::args::parse_repo_slug(&value).map_err(anyhow::Error::msg);
    }
    if let Some(value) = jj_repo {
        return Ok(value.to_owned());
    }
    detect_repo_slug()
        .await?
        .ok_or_else(|| anyhow::anyhow!("unable to resolve repo; pass --repo OWNER/REPO"))
}

pub fn ensure_repo_matches_jj_origin(selected: &str, jj_repo: Option<&str>) -> anyhow::Result<()> {
    if let Some(origin) = jj_repo {
        if origin != selected {
            return Err(anyhow::anyhow!(
                "repo mismatch: selected `{selected}` but jj origin is `{origin}`"
            ));
        }
    }
    Ok(())
}

async fn ensure_repo_exists(repo: &str) -> anyhow::Result<()> {
    for attempt in 0..=REPO_LOOKUP_BACKOFFS.len() {
        if attempt > 0 {
            sleep(REPO_LOOKUP_BACKOFFS[attempt - 1]).await;
        }
        match try_ensure_repo_exists(repo).await {
            Ok(()) => return Ok(()),
            Err(failure) if failure.retryable => {
                if attempt == REPO_LOOKUP_BACKOFFS.len() {
                    return Err(anyhow::anyhow!(format_repo_lookup_error_json(
                        repo,
                        attempt + 1,
                        REPO_LOOKUP_BACKOFFS.len(),
                        true,
                        &failure.message,
                    )));
                }
            }
            Err(failure) => {
                return Err(anyhow::anyhow!(format_repo_lookup_error_json(
                    repo,
                    attempt + 1,
                    REPO_LOOKUP_BACKOFFS.len(),
                    false,
                    &failure.message,
                )));
            }
        }
    }
    Err(anyhow::anyhow!(format_repo_lookup_error_json(
        repo,
        REPO_LOOKUP_BACKOFFS.len() + 1,
        REPO_LOOKUP_BACKOFFS.len(),
        false,
        "repo lookup exhausted retries",
    )))
}

#[derive(Debug)]
struct RepoLookupFailure {
    message: String,
    retryable: bool,
}

async fn try_ensure_repo_exists(repo: &str) -> Result<(), RepoLookupFailure> {
    let output = tokio::process::Command::new("gh")
        .args(["repo", "view", "--repo", repo, "--json", "nameWithOwner"])
        .output()
        .await
        .map_err(|error| RepoLookupFailure {
            message: format!("failed to run gh repo view: {error}"),
            retryable: true,
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(RepoLookupFailure {
            retryable: is_retryable_repo_lookup_stderr(&stderr),
            message: stderr,
        });
    }
    let stdout = String::from_utf8(output.stdout).map_err(|error| RepoLookupFailure {
        message: format!("gh output was not UTF-8: {error}"),
        retryable: false,
    })?;
    let resolved = extract_repo_slug_from_gh_output(&stdout)
        .map_err(|error| RepoLookupFailure { message: error.to_string(), retryable: false })?;
    if resolved == repo {
        Ok(())
    } else {
        Err(RepoLookupFailure {
            message: format!("gh resolved repo `{resolved}` but expected `{repo}`"),
            retryable: false,
        })
    }
}

pub fn is_retryable_repo_lookup_stderr(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    [
        "timed out",
        "timeout",
        "connection reset",
        "temporarily unavailable",
        "service unavailable",
        "try again",
        "rate limit",
        "502",
        "503",
        "504",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub fn format_repo_lookup_error_json(
    repo: &str,
    attempt: usize,
    retries: usize,
    retryable: bool,
    message: &str,
) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "category": "repo_lookup",
        "repo": repo,
        "attempt": attempt,
        "max_retries": retries,
        "retryable": retryable,
        "message": normalize_error_message(message),
    }))
    .unwrap_or_else(|_| {
        format!(
            "{{\"category\":\"repo_lookup\",\"repo\":\"{repo}\",\"message\":\"{}\"}}",
            normalize_error_message(message)
        )
    })
}

pub fn normalize_error_message(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn extract_repo_slug_from_gh_output(raw: &str) -> anyhow::Result<String> {
    #[derive(Debug, serde::Deserialize)]
    struct GhRepoView {
        #[serde(rename = "nameWithOwner")]
        name_with_owner: String,
    }
    let payload: GhRepoView = serde_json::from_str(raw)?;
    super::args::parse_repo_slug(&payload.name_with_owner).map_err(anyhow::Error::msg)
}

pub fn extract_repo_slug_from_jj_remote_output(raw: &str) -> anyhow::Result<Option<String>> {
    raw.lines().find_map(parse_jj_remote_line).transpose()
}

fn parse_jj_remote_line(line: &str) -> Option<anyhow::Result<String>> {
    let mut parts = line.split_whitespace();
    let remote_name = parts.next()?;
    let remote_url = parts.next()?;
    if remote_name != "origin" {
        return None;
    }
    Some(parse_repo_slug_from_remote_url(remote_url))
}

pub fn parse_repo_slug_from_remote_url(value: &str) -> anyhow::Result<String> {
    let trimmed = value.trim_end_matches('/').trim_end_matches(".git");
    let normalized = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("git@github.com:"))
        .or_else(|| trimmed.strip_prefix("ssh://git@github.com/"))
        .ok_or_else(|| {
            anyhow::anyhow!("unsupported origin URL `{value}`; expected github remote")
        })?;
    super::args::parse_repo_slug(normalized).map_err(anyhow::Error::msg)
}
