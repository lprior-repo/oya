use super::super::*;
use super::command_exec::run_command_with_timeout_with_exit;
use chrono::Utc;
use oya::types::GitHubPrMetadata;
use std::path::PathBuf;

#[allow(dead_code)]
pub struct GitHubAdapter {
    pub repo_root: PathBuf,
}

#[allow(dead_code)]
impl GitHubAdapter {
    pub fn new(repo_root: PathBuf) -> Self {
        Self { repo_root }
    }

    /// Fetches an existing PR for the given head branch.
    pub fn get_existing_pr(&self, head_branch: &str) -> Result<Option<(u64, String)>, OyaError> {
        let (passed, stdout, stderr, exit_code) = run_command_with_timeout_with_exit(
            "gh",
            &["pr", "list", "--head", head_branch, "--json", "number,url", "--limit", "1"],
            30,
            &self.repo_root,
        )?;

        if !passed {
            // If the command itself failed (e.g. gh not installed), propagate error.
            // But if it just found no PRs, it usually returns empty array or exit 0.
            if exit_code != 0 {
                return Err(OyaError(format!(
                    "gh pr list failed (exit {}): {}",
                    exit_code, stderr
                )));
            }
        }

        let json: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| OyaError(format!("Failed to parse gh output: {}", e)))?;

        let pr = json.as_array().and_then(|arr| arr.first()).and_then(|obj| {
            let number = obj.get("number")?.as_u64()?;
            let url = obj.get("url")?.as_str()?.to_string();
            Some((number, url))
        });

        Ok(pr)
    }

    /// Creates a new PR and returns its metadata.
    pub fn create_pr(
        &self,
        head: &str,
        title: &str,
        body: &str,
        bead_id: &str,
    ) -> Result<GitHubPrMetadata, OyaError> {
        let (passed, _stdout, stderr, exit_code) = run_command_with_timeout_with_exit(
            "gh",
            &["pr", "create", "--head", head, "--title", title, "--body", body],
            60,
            &self.repo_root,
        )?;

        if !passed {
            return Err(OyaError(format!("gh pr create failed (exit {}): {}", exit_code, stderr)));
        }

        let (number, url) = self
            .get_existing_pr(head)?
            .ok_or_else(|| OyaError("Created PR not found after creation".to_string()))?;

        Ok(GitHubPrMetadata {
            pr_url: url,
            pr_number: number,
            head_branch: head.to_string(),
            base_branch: "main".to_string(),
            bead_id: bead_id.to_string(),
            last_updated_at: Utc::now(),
        })
    }

    /// Updates the body of an existing PR.
    pub fn update_pr(&self, number: u64, body: &str) -> Result<(), OyaError> {
        let num_str = number.to_string();
        let (passed, _stdout, stderr, exit_code) = run_command_with_timeout_with_exit(
            "gh",
            &["pr", "edit", &num_str, "--body", body],
            60,
            &self.repo_root,
        )?;

        if !passed {
            return Err(OyaError(format!("gh pr edit failed (exit {}): {}", exit_code, stderr)));
        }

        Ok(())
    }

    /// Synchronizes PR state (create if missing, update if exists).
    pub fn sync_pr(
        &self,
        head: &str,
        title: &str,
        body: &str,
        bead_id: &str,
    ) -> Result<GitHubPrMetadata, OyaError> {
        if let Some((number, url)) = self.get_existing_pr(head)? {
            self.update_pr(number, body)?;
            Ok(GitHubPrMetadata {
                pr_url: url,
                pr_number: number,
                head_branch: head.to_string(),
                base_branch: "main".to_string(),
                bead_id: bead_id.to_string(),
                last_updated_at: Utc::now(),
            })
        } else {
            self.create_pr(head, title, body, bead_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_github_pr_metadata_serialization() {
        let meta = GitHubPrMetadata {
            pr_url: "https://github.com/org/repo/pull/1".to_string(),
            pr_number: 1,
            head_branch: "head".to_string(),
            base_branch: "main".to_string(),
            bead_id: "bead-1".to_string(),
            last_updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: GitHubPrMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, deserialized);
    }
}
