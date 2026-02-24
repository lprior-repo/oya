#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::too_many_lines)]
#![deny(clippy::too_many_arguments)]
#![forbid(unsafe_code)]

//! Landing phase step functions for jj-based workspace merge operations.

use crate::types::{FailureCategory, StageName};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandStep {
    pub id: String,
    pub label: String,
    pub program: String,
    pub args: Vec<String>,
    pub timeout_seconds: u64,
    pub failure_category: FailureCategory,
    pub next_stage: StageName,
}

pub fn jj_fetch_step() -> CommandStep {
    CommandStep {
        id: "jj_fetch".to_string(),
        label: "jj git fetch".to_string(),
        program: "jj".to_string(),
        args: vec!["git".to_string(), "fetch".to_string()],
        timeout_seconds: 60,
        failure_category: FailureCategory::MergeConflict,
        next_stage: StageName::Implementation,
    }
}

pub fn jj_rebase_step(bead_id: &str) -> CommandStep {
    let workspace = format!("oya-{}", bead_id);
    CommandStep {
        id: "jj_rebase".to_string(),
        label: "jj rebase onto main".to_string(),
        program: "jj".to_string(),
        args: vec![
            "rebase".to_string(),
            "-s".to_string(),
            workspace,
            "-d".to_string(),
            "main".to_string(),
        ],
        timeout_seconds: 60,
        failure_category: FailureCategory::MergeConflict,
        next_stage: StageName::Implementation,
    }
}

pub fn jj_bookmark_set_step(bead_id: &str) -> CommandStep {
    let bookmark = format!("oya-{}", bead_id);
    CommandStep {
        id: "jj_bookmark_set".to_string(),
        label: "jj bookmark set".to_string(),
        program: "jj".to_string(),
        args: vec!["bookmark".to_string(), "create".to_string(), bookmark],
        timeout_seconds: 30,
        failure_category: FailureCategory::MergeConflict,
        next_stage: StageName::Implementation,
    }
}

pub fn jj_git_push_step(bead_id: &str) -> CommandStep {
    let bookmark = format!("oya-{}", bead_id);
    CommandStep {
        id: "jj_git_push".to_string(),
        label: "jj git push".to_string(),
        program: "jj".to_string(),
        args: vec!["git".to_string(), "push".to_string(), "-b".to_string(), bookmark],
        timeout_seconds: 60,
        failure_category: FailureCategory::MergeConflict,
        next_stage: StageName::Implementation,
    }
}

pub fn jj_workspace_forget_step(bead_id: &str) -> CommandStep {
    let workspace = format!("oya-{}", bead_id);
    CommandStep {
        id: "jj_workspace_forget".to_string(),
        label: "jj workspace forget".to_string(),
        program: "jj".to_string(),
        args: vec!["workspace".to_string(), "forget".to_string(), workspace],
        timeout_seconds: 30,
        failure_category: FailureCategory::MergeConflict,
        next_stage: StageName::Implementation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jj_fetch_step_returns_correct_command_step() {
        let step = jj_fetch_step();
        assert_eq!(step.id, "jj_fetch");
        assert_eq!(step.label, "jj git fetch");
        assert_eq!(step.program, "jj");
        assert_eq!(step.args, vec!["git", "fetch"]);
        assert_eq!(step.failure_category, FailureCategory::MergeConflict);
        assert_eq!(step.next_stage, StageName::Implementation);
        assert!(step.timeout_seconds > 0);
    }

    #[test]
    fn test_jj_rebase_step_includes_workspace_name() {
        let bead_id = "src-abc";
        let step = jj_rebase_step(bead_id);
        let expected_workspace = format!("oya-{}", bead_id);
        assert!(step.args.contains(&expected_workspace));
    }

    #[test]
    fn test_jj_bookmark_set_step_uses_bead_id() {
        let bead_id = "src-xyz";
        let step = jj_bookmark_set_step(bead_id);
        let expected_bookmark = format!("oya-{}", bead_id);
        assert!(step.args.contains(&expected_bookmark));
    }

    #[test]
    fn test_jj_git_push_step_pushes_specific_bookmark() {
        let bead_id = "src-123";
        let step = jj_git_push_step(bead_id);
        let expected_bookmark = format!("oya-{}", bead_id);
        assert!(step.args.contains(&expected_bookmark));
    }

    #[test]
    fn test_jj_workspace_forget_step_forgets_specific_workspace() {
        let bead_id = "src-forget";
        let step = jj_workspace_forget_step(bead_id);
        let expected_workspace = format!("oya-{}", bead_id);
        assert!(step.args.contains(&expected_workspace));
    }
}
