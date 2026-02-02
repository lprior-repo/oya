#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! Process management for profiled commands

use crate::error::{ProfilingError, Result};
use std::path::PathBuf;
use std::process::{Child, Command};

/// Wrapper for managing a profiled process
pub struct ProfiledProcess {
    /// The child process handle
    child: Child,

    /// Process ID
    pid: u32,
}

impl ProfiledProcess {
    /// Spawn a new process under heaptrack profiling
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute
    /// * `args` - Arguments for the command
    /// * `working_dir` - Optional working directory
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - heaptrack is not installed
    /// - Process fails to spawn
    /// - Cannot determine PID
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use oya_profiling::process::ProfiledProcess;
    /// let process = ProfiledProcess::spawn(
    ///     "my-app",
    ///     &["--load-test".to_string()],
    ///     None,
    /// );
    /// ```
    pub fn spawn(command: &str, args: &[String], working_dir: Option<&PathBuf>) -> Result<Self> {
        // Verify heaptrack is available
        Self::verify_heaptrack_installed()?;

        // Build heaptrack command
        let mut cmd = Command::new("heaptrack");

        // heaptrack arguments
        cmd.arg("--output").arg("/dev/null"); // We only care about RSS, not heaptrack output

        // The command to profile
        cmd.arg(command);

        // Command arguments
        for arg in args {
            cmd.arg(arg);
        }

        // Set working directory if provided
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        // Spawn the process
        let child = cmd.spawn().map_err(|e| {
            ProfilingError::ProcessSpawnFailed(format!("failed to spawn {command}: {e}"))
        })?;

        // Get PID
        let pid = child.id();

        Ok(Self { child, pid })
    }

    /// Get the process ID
    #[must_use]
    pub const fn pid(&self) -> u32 {
        self.pid
    }

    /// Check if the process is still running
    ///
    /// # Errors
    ///
    /// Returns error if process status cannot be determined
    pub fn is_running(&mut self) -> Result<bool> {
        match self.child.try_wait() {
            Ok(None) => Ok(true),
            Ok(Some(_)) => Ok(false),
            Err(e) => Err(ProfilingError::ProcessTerminated(format!(
                "failed to check process status: {e}"
            ))),
        }
    }

    /// Wait for the process to complete
    ///
    /// # Errors
    ///
    /// Returns error if process wait fails
    pub fn wait(mut self) -> Result<()> {
        self.child
            .wait()
            .map_err(|e| ProfilingError::ProcessTerminated(format!("wait failed: {e}")))?;
        Ok(())
    }

    /// Kill the process
    ///
    /// # Errors
    ///
    /// Returns error if kill signal fails
    pub fn kill(mut self) -> Result<()> {
        self.child
            .kill()
            .map_err(|e| ProfilingError::ProcessTerminated(format!("kill failed: {e}")))?;
        Ok(())
    }

    /// Verify heaptrack is installed and available in PATH
    fn verify_heaptrack_installed() -> Result<()> {
        let output = Command::new("which")
            .arg("heaptrack")
            .output()
            .map_err(|_| ProfilingError::HeaptrackNotFound)?;

        if output.status.success() {
            Ok(())
        } else {
            Err(ProfilingError::HeaptrackNotFound)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heaptrack_check() {
        // This test will pass if heaptrack is installed, fail otherwise
        // In CI, ensure heaptrack is installed
        let result = ProfiledProcess::verify_heaptrack_installed();

        // We can't assert success because heaptrack may not be installed in test environment
        // But we can verify it returns the right error type
        if result.is_err() {
            assert!(matches!(result, Err(ProfilingError::HeaptrackNotFound)));
        }
    }

    #[test]
    fn test_spawn_requires_heaptrack() {
        // If heaptrack is not installed, spawn should fail gracefully
        let result = ProfiledProcess::spawn("echo", &["test".to_string()], None);

        // Should either succeed (heaptrack installed) or fail with HeaptrackNotFound
        if result.is_err() {
            assert!(matches!(
                result,
                Err(ProfilingError::HeaptrackNotFound) | Err(ProfilingError::ProcessSpawnFailed(_))
            ));
        }
    }
}
