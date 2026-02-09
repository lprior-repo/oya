//! Stage prompt/context construction for recursive bead execution.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use oya_events::{BeadId, StageKind};

/// Context payload for a bead execution request.
#[derive(Debug, Clone)]
pub struct BeadContext {
    /// Bead identifier.
    pub bead_id: BeadId,
    /// Human-readable task spec.
    pub spec: String,
    /// Candidate relevant files for this bead.
    pub relevant_files: Vec<PathBuf>,
    /// Upstream artifact identifiers.
    pub upstream_artifacts: Vec<String>,
}

/// Prompt payload for a specific stage.
#[derive(Debug, Clone)]
pub struct StagePrompt {
    /// Stage this prompt is for.
    pub stage: StageKind,
    /// Prompt text to send to the agent.
    pub prompt_text: String,
    /// Tool names allowed for this stage.
    pub allowed_tools: Vec<String>,
    /// Execution timeout budget.
    pub timeout: Duration,
}

/// Errors from context/prompt construction.
#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    /// Required artifact for a stage was missing.
    #[error("missing required artifact for stage: {0}")]
    MissingArtifact(StageKind),
}

/// Builds stage-specific prompt payloads for agent execution.
#[derive(Debug, Clone)]
pub struct StageContextBuilder {
    project_root: PathBuf,
    claude_md_path: Option<PathBuf>,
}

impl StageContextBuilder {
    /// Create a new builder rooted at a project path.
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            claude_md_path: None,
        }
    }

    /// Configure optional CLAUDE.md instructions file.
    pub fn with_claude_md(mut self, path: PathBuf) -> Self {
        self.claude_md_path = Some(path);
        self
    }

    /// Build prompt payload for a specific stage.
    pub fn build_prompt(
        &self,
        stage: StageKind,
        context: &BeadContext,
        artifacts: &HashMap<StageKind, String>,
        feedback: Option<&str>,
    ) -> Result<StagePrompt, ContextError> {
        let claude_block = self.read_claude_md_block();
        let files_text = join_paths(&context.relevant_files);
        let upstream_text = join_lines(&context.upstream_artifacts);

        let (prompt_text, allowed_tools, timeout) = match stage {
            StageKind::Research => {
                let text = format!(
                    "Analyze bead spec: {spec}\n\nProject root: {root}\nRelevant files:\n{files}\n\nUpstream artifacts:\n{upstream}\n\nFind dependencies, risks, and affected modules. Output structured research.\n{claude}",
                    spec = context.spec,
                    root = self.project_root.display(),
                    files = files_text,
                    upstream = upstream_text,
                    claude = claude_block
                );
                (
                    text,
                    vec!["read", "glob", "grep"],
                    Duration::from_secs(45 * 60),
                )
            }
            StageKind::Plan => {
                let research = require_artifact(artifacts, StageKind::Research)?;
                let text = format!(
                    "Given research:\n{research}\n\nCreate implementation plan for bead {bead}.\nSpec: {spec}\nList files to modify, test strategy, and edge cases.\n{claude}",
                    research = research,
                    bead = context.bead_id,
                    spec = context.spec,
                    claude = claude_block
                );
                (
                    text,
                    vec!["read", "glob", "grep"],
                    Duration::from_secs(30 * 60),
                )
            }
            StageKind::Implement => {
                let plan = require_artifact(artifacts, StageKind::Plan)?;
                let feedback_text = feedback.map_or(String::new(), |value| {
                    format!("\nRetry feedback:\n{value}\n")
                });
                let text = format!(
                    "Implement bead {bead}.\n\nPlan:\n{plan}\n\nSpec: {spec}\nRelevant files:\n{files}\n{feedback}Run `moon run :quick` when done.\n{claude}",
                    bead = context.bead_id,
                    plan = plan,
                    spec = context.spec,
                    files = files_text,
                    feedback = feedback_text,
                    claude = claude_block
                );
                (
                    text,
                    vec!["read", "glob", "grep", "bash", "apply_patch"],
                    Duration::from_secs(60 * 60),
                )
            }
            StageKind::Review => {
                let plan = require_artifact(artifacts, StageKind::Plan)?;
                let text = format!(
                    "Review diff for bead {bead}.\nPlan was:\n{plan}\n\nSpec:\n{spec}\n\nCheck correctness, edge cases, and zero-unwrap policy.\nVerdict format: PASS or REJECT with severity (minor/major/fundamental) and feedback.\n{claude}",
                    bead = context.bead_id,
                    plan = plan,
                    spec = context.spec,
                    claude = claude_block
                );
                (text, vec!["read", "grep"], Duration::from_secs(20 * 60))
            }
            StageKind::Validate => (String::new(), vec!["bash"], Duration::from_secs(15 * 60)),
            StageKind::Accept => (String::new(), vec!["none"], Duration::from_secs(5 * 60)),
        };

        Ok(StagePrompt {
            stage,
            prompt_text,
            allowed_tools: allowed_tools.into_iter().map(String::from).collect(),
            timeout,
        })
    }

    fn read_claude_md_block(&self) -> String {
        if let Some(path) = &self.claude_md_path {
            if let Ok(content) = fs::read_to_string(path) {
                if !content.is_empty() {
                    return format!("\n\nCLAUDE.md instructions:\n{}", content);
                }
            }
        }
        String::new()
    }
}

fn require_artifact(
    artifacts: &HashMap<StageKind, String>,
    stage: StageKind,
) -> Result<String, ContextError> {
    artifacts
        .get(&stage)
        .cloned()
        .ok_or(ContextError::MissingArtifact(stage))
}

fn join_paths(paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        String::from("(none)")
    } else {
        paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<String>>()
            .join("\n")
    }
}

fn join_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        String::from("(none)")
    } else {
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_context() -> BeadContext {
        BeadContext {
            bead_id: BeadId::new(),
            spec: String::from("implement recursive dag execution"),
            relevant_files: vec![PathBuf::from("src/lib.rs"), PathBuf::from("src/main.rs")],
            upstream_artifacts: vec![String::from("artifact-a")],
        }
    }

    #[test]
    fn test_research_prompt_includes_spec() {
        let builder = StageContextBuilder::new(PathBuf::from("/tmp/proj"));
        let context = sample_context();
        let artifacts = HashMap::new();

        let result = builder.build_prompt(StageKind::Research, &context, &artifacts, None);
        assert!(result.is_ok());
        if let Ok(prompt) = result {
            assert!(prompt.prompt_text.contains(&context.spec));
        }
    }

    #[test]
    fn test_plan_prompt_includes_research_artifact() {
        let builder = StageContextBuilder::new(PathBuf::from("/tmp/proj"));
        let context = sample_context();
        let mut artifacts = HashMap::new();
        artifacts.insert(StageKind::Research, String::from("research-output"));

        let result = builder.build_prompt(StageKind::Plan, &context, &artifacts, None);
        assert!(result.is_ok());
        if let Ok(prompt) = result {
            assert!(prompt.prompt_text.contains("research-output"));
        }
    }

    #[test]
    fn test_implement_prompt_includes_plan() {
        let builder = StageContextBuilder::new(PathBuf::from("/tmp/proj"));
        let context = sample_context();
        let mut artifacts = HashMap::new();
        artifacts.insert(StageKind::Plan, String::from("plan-output"));

        let result = builder.build_prompt(StageKind::Implement, &context, &artifacts, None);
        assert!(result.is_ok());
        if let Ok(prompt) = result {
            assert!(prompt.prompt_text.contains("plan-output"));
        }
    }

    #[test]
    fn test_implement_retry_prompt_includes_feedback() {
        let builder = StageContextBuilder::new(PathBuf::from("/tmp/proj"));
        let context = sample_context();
        let mut artifacts = HashMap::new();
        artifacts.insert(StageKind::Plan, String::from("plan-output"));

        let result = builder.build_prompt(
            StageKind::Implement,
            &context,
            &artifacts,
            Some("CI failed in lint stage"),
        );
        assert!(result.is_ok());
        if let Ok(prompt) = result {
            assert!(prompt.prompt_text.contains("CI failed in lint stage"));
        }
    }

    #[test]
    fn test_review_prompt_includes_diff_and_plan() {
        let builder = StageContextBuilder::new(PathBuf::from("/tmp/proj"));
        let context = sample_context();
        let mut artifacts = HashMap::new();
        artifacts.insert(StageKind::Plan, String::from("plan-output"));

        let result = builder.build_prompt(StageKind::Review, &context, &artifacts, None);
        assert!(result.is_ok());
        if let Ok(prompt) = result {
            assert!(prompt.prompt_text.contains("plan-output"));
            assert!(prompt.prompt_text.to_ascii_lowercase().contains("diff"));
        }
    }

    #[test]
    fn test_validate_returns_no_prompt() {
        let builder = StageContextBuilder::new(PathBuf::from("/tmp/proj"));
        let context = sample_context();
        let artifacts = HashMap::new();

        let result = builder.build_prompt(StageKind::Validate, &context, &artifacts, None);
        assert!(result.is_ok());
        if let Ok(prompt) = result {
            assert!(prompt.prompt_text.is_empty());
        }
    }

    #[test]
    fn test_accept_returns_no_prompt() {
        let builder = StageContextBuilder::new(PathBuf::from("/tmp/proj"));
        let context = sample_context();
        let artifacts = HashMap::new();

        let result = builder.build_prompt(StageKind::Accept, &context, &artifacts, None);
        assert!(result.is_ok());
        if let Ok(prompt) = result {
            assert!(prompt.prompt_text.is_empty());
        }
    }

    #[test]
    fn test_context_builder_reads_claude_md() {
        let path = std::env::temp_dir().join(format!("oya-claude-md-{}.md", std::process::id()));
        let write_result = fs::write(&path, "follow local instructions");
        assert!(write_result.is_ok());

        let builder =
            StageContextBuilder::new(PathBuf::from("/tmp/proj")).with_claude_md(path.clone());
        let context = sample_context();
        let artifacts = HashMap::new();
        let result = builder.build_prompt(StageKind::Research, &context, &artifacts, None);
        assert!(result.is_ok());
        if let Ok(prompt) = result {
            assert!(prompt.prompt_text.contains("follow local instructions"));
        }

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_context_builder_missing_claude_md_ok() {
        let builder = StageContextBuilder::new(PathBuf::from("/tmp/proj"))
            .with_claude_md(PathBuf::from("/tmp/does-not-exist.md"));
        let context = sample_context();
        let artifacts = HashMap::new();
        let result = builder.build_prompt(StageKind::Research, &context, &artifacts, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_prompt_timeout_varies_by_stage() {
        let builder = StageContextBuilder::new(PathBuf::from("/tmp/proj"));
        let context = sample_context();
        let mut artifacts = HashMap::new();
        artifacts.insert(StageKind::Research, String::from("research-output"));
        artifacts.insert(StageKind::Plan, String::from("plan-output"));

        let research = builder.build_prompt(StageKind::Research, &context, &artifacts, None);
        let review = builder.build_prompt(StageKind::Review, &context, &artifacts, None);
        assert!(research.is_ok());
        assert!(review.is_ok());
        if let (Ok(research_prompt), Ok(review_prompt)) = (research, review) {
            assert!(research_prompt.timeout > review_prompt.timeout);
        }
    }
}
