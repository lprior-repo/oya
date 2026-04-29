#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VcsError {
    #[error("repo error: {0}")]
    RepoError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebaseStats {
    pub rebased_count: u32,
    pub abandoned_count: u32,
}

pub struct JjBackend {
    repo_path: std::path::PathBuf,
}

impl JjBackend {
    pub fn open(workspace_path: &Path) -> Result<Self, VcsError> {
        let repo_path = workspace_path.to_path_buf();
        if !repo_path.exists() {
            return Err(VcsError::RepoError(format!(
                "repo path does not exist: {}",
                repo_path.display()
            )));
        }
        Ok(Self { repo_path })
    }

    pub fn repo_path(&self) -> &Path {
        self.repo_path.as_path()
    }
}
