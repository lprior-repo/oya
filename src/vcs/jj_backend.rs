#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use jj_lib::backend::CommitId;
use jj_lib::repo::{MutableRepo, ReadonlyRepo, Repo};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VcsError {
    #[error("bookmark not found: {0}")]
    BookmarkNotFound(String),
    #[error("parent not found: {0}")]
    NotFound(String),
    #[error("dirty working directory")]
    DirtyWorkingDirectory,
    #[error("failed to acquire lock: {0}")]
    LockAcquisitionFailed(String),
    #[error("jj internal error: {0}")]
    JjInternalError(String),
    #[error("invalid commit ID: {0}")]
    InvalidCommitId(String),
    #[error("workspace error: {0}")]
    WorkspaceError(String),
    #[error("repo error: {0}")]
    RepoError(String),
    #[error("rebase failed: {0}")]
    RebaseFailed(String),
    #[error("transaction error: {0}")]
    TransactionError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebaseStats {
    pub rebased_count: u32,
    pub abandoned_count: u32,
}

pub struct JjBackend {
    repo_path: std::path::PathBuf,
    lock: Mutex<()>,
}

impl JjBackend {
    pub fn open(workspace_path: &Path) -> Result<Self, VcsError> {
        let repo_path = workspace_path.to_path_buf();
        if !repo_path.exists() {
            return Err(VcsError::WorkspaceError(format!(
                "repo path does not exist: {}",
                repo_path.display()
            )));
        }
        Ok(Self { repo_path, lock: Mutex::new(()) })
    }

    pub fn get_branch_tip(&self, branch: &str) -> Result<String, VcsError> {
        let _lock = acquire_lock(&self.lock)?;
        let repo = load_repo(&self.repo_path)?;
        let view = repo.view();
        let bookmarks = view.bookmarks();
        let branch_target =
            bookmarks.get(branch).ok_or_else(|| VcsError::BookmarkNotFound(branch.to_owned()))?;
        let commit_id = branch_target
            .push
            .as_ref()
            .ok_or_else(|| VcsError::BookmarkNotFound(branch.to_owned()))?;
        Ok(commit_id.hex().to_string())
    }

    pub fn fetch(&self) -> Result<(), VcsError> {
        let _lock = acquire_lock(&self.lock)?;
        let repo = load_repo(&self.repo_path)?;
        let mut tx = repo.start_transaction();
        let mut tx_repo = tx.repo_mut();
        jj_lib::git::import_refs(&mut tx_repo, &jj_lib::git::GitImportOptions::default())
            .map_err(|e| VcsError::JjInternalError(e.to_string()))?;
        tx_repo
            .update_rewritten_references(&jj_lib::repo::RewriteRefsOptions::default())
            .map_err(|e| VcsError::JjInternalError(e.to_string()))?;
        tx.finish().map_err(|e| VcsError::TransactionError(e.to_string()))?;
        Ok(())
    }

    pub fn sync(&self, branch: &str, parent: &str) -> Result<(), VcsError> {
        let _lock = acquire_lock(&self.lock)?;
        let repo = load_repo(&self.repo_path)?;
        check_working_directory_clean(&repo)?;
        let branch_commit_id = self.get_branch_tip_internal(&repo, branch)?;
        let parent_commit_id = self.get_branch_tip_internal(&repo, parent)?;
        if self.is_ancestor_internal(&repo, &parent_commit_id, &branch_commit_id)? {
            return Ok(());
        }
        let mut tx = repo.start_transaction();
        let rebased = rebase_branch(tx.repo_mut(), branch, &branch_commit_id, &parent_commit_id)?;
        if rebased > 0 {
            tx.repo_mut()
                .update_rewritten_references(&jj_lib::repo::RewriteRefsOptions::default())
                .map_err(|e| VcsError::JjInternalError(e.to_string()))?;
        }
        tx.finish().map_err(|e| VcsError::TransactionError(e.to_string()))?;
        Ok(())
    }

    pub fn rebase_branch_onto(&self, branch: &str, parent: &str) -> Result<RebaseStats, VcsError> {
        let _lock = acquire_lock(&self.lock)?;
        let repo = load_repo(&self.repo_path)?;
        let branch_commit_id = self.get_branch_tip_internal(&repo, branch)?;
        let parent_commit_id = self.get_branch_tip_internal(&repo, parent)?;
        if branch_commit_id == parent_commit_id {
            return Ok(RebaseStats { rebased_count: 0, abandoned_count: 0 });
        }
        let mut tx = repo.start_transaction();
        let rebased = rebase_branch(tx.repo_mut(), branch, &branch_commit_id, &parent_commit_id)?;
        tx.repo_mut()
            .update_rewritten_references(&jj_lib::repo::RewriteRefsOptions::default())
            .map_err(|e| VcsError::JjInternalError(e.to_string()))?;
        let stats = tx.finish().map_err(|e| VcsError::TransactionError(e.to_string()))?;
        Ok(RebaseStats { rebased_count: rebased, abandoned_count: stats.abandoned_count() })
    }

    fn get_branch_tip_internal(
        &self,
        repo: &ReadonlyRepo,
        branch: &str,
    ) -> Result<CommitId, VcsError> {
        let view = repo.view();
        let bookmarks = view.bookmarks();
        let branch_target =
            bookmarks.get(branch).ok_or_else(|| VcsError::BookmarkNotFound(branch.to_owned()))?;
        branch_target
            .push
            .as_ref()
            .ok_or_else(|| VcsError::BookmarkNotFound(branch.to_owned()))
            .map(|id| id.clone())
    }

    fn is_ancestor_internal(
        &self,
        repo: &ReadonlyRepo,
        ancestor: &CommitId,
        descendant: &CommitId,
    ) -> Result<bool, VcsError> {
        // Simple ancestor check using the index
        let index = repo.index();
        Ok(index.is_ancestor(ancestor, descendant))
    }
}

fn acquire_lock(lock: &Mutex<()>) -> Result<MutexGuard<'_, ()>, VcsError> {
    lock.lock().map_err(|e| VcsError::LockAcquisitionFailed(e.to_string()))
}

fn load_repo(repo_path: &Path) -> Result<ReadonlyRepo, VcsError> {
    // This is a simplified repo loading approach for the proof of concept
    // In reality jj_lib requires environment setup
    jj_lib::repo::Repo::load(repo_path).map_err(|e| VcsError::RepoError(e.to_string()))
}

fn check_working_directory_clean(repo: &ReadonlyRepo) -> Result<(), VcsError> {
    let wc_commit = repo.working_copy().current_commit_id();
    if !wc_commit.is_zero() {
        return Err(VcsError::DirtyWorkingDirectory);
    }
    Ok(())
}

fn rebase_branch(
    mut_repo: &mut MutableRepo,
    branch: &str,
    branch_commit_id: &CommitId,
    parent_commit_id: &CommitId,
) -> Result<u32, VcsError> {
    let store = mut_repo.store();
    let _branch_commit =
        store.get_commit(branch_commit_id).map_err(|e| VcsError::JjInternalError(e.to_string()))?;
    let _parent_commit =
        store.get_commit(parent_commit_id).map_err(|e| VcsError::JjInternalError(e.to_string()))?;
    mut_repo.set_rewritten_commit(branch_commit_id.clone(), parent_commit_id.clone());
    let rebased_count: usize =
        mut_repo.rebase_descendants().map_err(|e| VcsError::RebaseFailed(e.to_string()))?;
    let view = mut_repo.view();
    let mut bookmarks = view.bookmarks();
    if let Some(branch_target) = bookmarks.get(branch) {
        let mut new_target = branch_target.clone();
        new_target.push = Some(parent_commit_id.clone());
        bookmarks.insert(branch, new_target);
    }
    Ok(rebased_count as u32)
}
