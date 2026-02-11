//! Subprocess handle and output types for agent execution.

use std::time::Duration;

use oya_events::StageKind;
use tokio::process::Child;

use super::{AgentSwarmError, AgentSwarmResult};

/// Captured output from agent subprocess execution.
#[derive(Debug, Clone)]
pub struct AgentOutput {
    /// Stage that produced this output
    pub stage: StageKind,
    /// Full stdout capture
    pub stdout: String,
    /// Full stderr capture
    pub stderr: String,
    /// Exit code from subprocess
    pub exit_code: Option<i32>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Success indicator (exit_code == 0)
    pub success: bool,
}

impl AgentOutput {
    /// Create a new agent output.
    #[must_use]
    pub fn new(
        stage: StageKind,
        stdout: String,
        stderr: String,
        exit_code: Option<i32>,
        duration_ms: u64,
    ) -> Self {
        let success = exit_code.map_or(false, |code| code == 0);
        Self {
            stage,
            stdout,
            stderr,
            exit_code,
            duration_ms,
            success,
        }
    }

    /// Check if execution was successful.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.success
    }

    /// Get combined output (stdout + stderr).
    #[must_use]
    pub fn combined_output(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }
}

impl From<AgentOutput> for crate::stage_gate::StageOutput {
    fn from(value: AgentOutput) -> Self {
        Self {
            stage: value.stage,
            success: value.success,
            output: value.stdout,
            exit_code: value.exit_code,
            duration_ms: value.duration_ms,
        }
    }
}

/// Active subprocess handle for managing agent lifecycle.
#[derive(Debug)]
pub struct SubprocessHandle {
    /// Child process handle from tokio
    child: Option<Child>,
    /// When the process started
    started_at: chrono::DateTime<chrono::Utc>,
    /// Associated stage kind
    stage: StageKind,
    /// Associated bead ID
    bead_id: oya_events::BeadId,
}

impl SubprocessHandle {
    /// Create a new subprocess handle.
    #[must_use]
    pub const fn new(
        child: Child,
        started_at: chrono::DateTime<chrono::Utc>,
        stage: StageKind,
        bead_id: oya_events::BeadId,
    ) -> Self {
        Self {
            child: Some(child),
            started_at,
            stage,
            bead_id,
        }
    }

    /// Get the stage kind.
    #[must_use]
    pub const fn stage(&self) -> StageKind {
        self.stage
    }

    /// Get the bead ID.
    #[must_use]
    pub const fn bead_id(&self) -> oya_events::BeadId {
        self.bead_id
    }

    /// Get the started timestamp.
    #[must_use]
    pub const fn started_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.started_at
    }

    /// Send prompt input to running agent subprocess.
    ///
    /// # Errors
    ///
    /// Returns error if stdin is not available or write fails.
    pub async fn send_prompt(&mut self, prompt: &str) -> AgentSwarmResult<()> {
        let child = self
            .child
            .as_mut()
            .ok_or(AgentSwarmError::NoActiveSubprocess)?;

        let stdin = child.stdin.as_mut().ok_or(AgentSwarmError::StdinUnavailable)?;

        use tokio::io::AsyncWriteExt;
        stdin
            .write_all(prompt.as_bytes())
            .await
            .map_err(|e| AgentSwarmError::SpawnFailed {
                message: format!("failed to write prompt: {}", e),
            })?;

        stdin
            .flush()
            .await
            .map_err(|e| AgentSwarmError::SpawnFailed {
                message: format!("failed to flush stdin: {}", e),
            })?;

        tracing::debug!("Prompt sent to agent subprocess for bead {}", self.bead_id);
        Ok(())
    }

    /// Read a single line of output from agent stdout.
    ///
    /// # Errors
    ///
    /// Returns error if stdout is not available or read fails.
    pub async fn read_output_line(&mut self) -> AgentSwarmResult<Option<String>> {
        let child = self
            .child
            .as_mut()
            .ok_or(AgentSwarmError::NoActiveSubprocess)?;

        let stdout = child.stdout.as_mut().ok_or(AgentSwarmError::StdoutUnavailable)?;

        use tokio::io::{AsyncBufReadExt, BufReader};
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        match lines.next_line().await {
            Ok(Some(line)) => Ok(Some(line)),
            Ok(None) => Ok(None),
            Err(e) => Err(AgentSwarmError::Io(e)),
        }
    }

    /// Wait for subprocess completion and collect all output.
    ///
    /// # Errors
    ///
    /// Returns error if timeout occurs or process wait fails.
    pub async fn wait_for_completion(
        mut self,
        timeout: Duration,
    ) -> AgentSwarmResult<AgentOutput> {
        let mut child = self
            .child
            .take()
            .ok_or(AgentSwarmError::NoActiveSubprocess)?;

        // Extract stdout/stderr before waiting (they're Option<ChildStdout>)
        let mut stdout = child
            .stdout
            .take()
            .ok_or(AgentSwarmError::StdoutUnavailable)?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or(AgentSwarmError::SpawnFailed {
                message: "stderr not captured".to_string(),
            })?;

        // Race between process completion and timeout
        let sleep = tokio::time::sleep(timeout);
        tokio::pin!(sleep);

        let status = tokio::select! {
            // Wait for process to complete
            result = child.wait() => result?,
            // Timeout elapsed - attempt to kill and return error
            _ = &mut sleep => {
                tracing::warn!(
                    "Agent timeout for bead {} stage {:?}, terminating process",
                    self.bead_id,
                    self.stage
                );
                let _ = child.start_kill();
                return Err(AgentSwarmError::Timeout { timeout });
            }
        };

        // Read remaining output from pipes
        use tokio::io::AsyncReadExt;
        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        let _ = stdout.read_to_end(&mut stdout_buf).await;
        let _ = stderr.read_to_end(&mut stderr_buf).await;

        let duration_ms = {
            let now = chrono::Utc::now();
            let elapsed = now.signed_duration_since(self.started_at);
            elapsed.num_milliseconds().max(0) as u64
        };

        let stdout_str = String::from_utf8_lossy(&stdout_buf).to_string();
        let stderr_str = String::from_utf8_lossy(&stderr_buf).to_string();

        let exit_code = status.code();
        let success = status.success();

        tracing::info!(
            "Agent completed for bead {} stage {:?}: exit_code={:?}, duration={}ms",
            self.bead_id,
            self.stage,
            exit_code,
            duration_ms
        );

        Ok(AgentOutput {
            stage: self.stage,
            stdout: stdout_str,
            stderr: stderr_str,
            exit_code,
            duration_ms,
            success,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oya_events::BeadId;

    #[test]
    fn test_agent_output_new_success() {
        let output = AgentOutput::new(
            StageKind::Implement,
            "Implementation complete".to_string(),
            String::new(),
            Some(0),
            1000,
        );

        assert!(output.is_success());
        assert_eq!(output.stage, StageKind::Implement);
        assert_eq!(output.stdout, "Implementation complete");
        assert_eq!(output.exit_code, Some(0));
        assert_eq!(output.duration_ms, 1000);
    }

    #[test]
    fn test_agent_output_new_failure() {
        let output = AgentOutput::new(
            StageKind::Implement,
            String::new(),
            "Error occurred".to_string(),
            Some(1),
            500,
        );

        assert!(!output.is_success());
        assert_eq!(output.exit_code, Some(1));
    }

    #[test]
    fn test_agent_output_no_exit_code() {
        let output = AgentOutput::new(
            StageKind::Implement,
            "Output".to_string(),
            String::new(),
            None,
            100,
        );

        assert!(!output.is_success());
        assert_eq!(output.exit_code, None);
    }

    #[test]
    fn test_agent_output_combined() {
        let output = AgentOutput::new(
            StageKind::Research,
            "stdout line".to_string(),
            "stderr line".to_string(),
            Some(0),
            100,
        );

        let combined = output.combined_output();
        assert!(combined.contains("stdout line"));
        assert!(combined.contains("stderr line"));
    }

    #[test]
    fn test_agent_output_to_stage_output() {
        let agent_output = AgentOutput::new(
            StageKind::Review,
            "Review passed".to_string(),
            String::new(),
            Some(0),
            2000,
        );

        let stage_output: crate::stage_gate::StageOutput = agent_output.into();
        assert_eq!(stage_output.stage, StageKind::Review);
        assert!(stage_output.success);
        assert_eq!(stage_output.output, "Review passed");
    }

    #[tokio::test]
    async fn test_subprocess_handle_send_prompt_no_process() {
        let bead_id = BeadId::new();
        let mut handle = SubprocessHandle {
            child: None,
            started_at: chrono::Utc::now(),
            stage: StageKind::Implement,
            bead_id,
        };

        let result = handle.send_prompt("test prompt").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentSwarmError::NoActiveSubprocess
        ));
    }

    #[tokio::test]
    async fn test_subprocess_handle_read_output_no_process() {
        let bead_id = BeadId::new();
        let mut handle = SubprocessHandle {
            child: None,
            started_at: chrono::Utc::now(),
            stage: StageKind::Implement,
            bead_id,
        };

        let result = handle.read_output_line().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentSwarmError::NoActiveSubprocess
        ));
    }

    #[tokio::test]
    async fn test_subprocess_handle_wait_no_process() {
        let bead_id = BeadId::new();
        let handle = SubprocessHandle {
            child: None,
            started_at: chrono::Utc::now(),
            stage: StageKind::Implement,
            bead_id,
        };

        let result = handle.wait_for_completion(Duration::from_secs(1)).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentSwarmError::NoActiveSubprocess
        ));
    }
}
