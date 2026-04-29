#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use super::commands::run_command_outcome;

const GIT_PROGRAM: &str = "git";
const GIT_ORIGIN_REMOTE_ARGS: &[&str] = &["remote", "get-url", "origin"];

pub async fn detect_repo_slug() -> anyhow::Result<Option<String>> {
    let output = run_command_outcome(GIT_PROGRAM, GIT_ORIGIN_REMOTE_ARGS, None).await?;
    if !output.success {
        return Ok(None);
    }
    extract_repo_slug_from_git_remote(&output.stdout).map(Some)
}

fn extract_repo_slug_from_git_remote(raw: &str) -> anyhow::Result<String> {
    let remote = raw
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| anyhow::anyhow!("git origin remote URL was empty"))?;
    crate::cli::repo::parse_repo_slug_from_remote_url(remote)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_repo_slug_from_https_origin() {
        let Ok(slug) =
            extract_repo_slug_from_git_remote("https://github.com/priorlewis43/oya.git\n")
        else {
            assert!(false, "https origin should parse");
            return;
        };
        assert_eq!(slug, "priorlewis43/oya");
    }

    #[test]
    fn extracts_repo_slug_from_ssh_origin() {
        let Ok(slug) = extract_repo_slug_from_git_remote("git@github.com:priorlewis43/oya.git\n")
        else {
            assert!(false, "ssh origin should parse");
            return;
        };
        assert_eq!(slug, "priorlewis43/oya");
    }

    #[test]
    fn rejects_empty_origin_output() {
        assert!(extract_repo_slug_from_git_remote("\n").is_err());
    }

    #[test]
    fn repo_detection_uses_only_git_origin_remote() {
        assert_eq!(GIT_PROGRAM, "git");
        assert_eq!(GIT_ORIGIN_REMOTE_ARGS, &["remote", "get-url", "origin"]);
    }
}
