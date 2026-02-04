//! Process pool for managing worker subprocesses.
//!
//! Provides async subprocess spawning, lifecycle management, and output capture
//! using tokio::process for efficient async I/O.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use crate::error::{Error, Result};

/// Configuration for spawning a worker process.
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    /// Command to execute
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Working directory
    pub working_dir: Option<String>,
    /// Environment variables to set
    pub env: Vec<(String, String)>,
    /// Whether to capture stdout
    pub capture_stdout: bool,
    /// Whether to capture stderr
    pub capture_stderr: bool,
}

impl ProcessConfig {
    /// Create a new process configuration with defaults.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            working_dir: None,
            env: Vec::new(),
            capture_stdout: true,
            capture_stderr: true,
        }
    }

    /// Add a command argument.
    #[must_use]
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple command arguments.
    #[must_use]
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Set the working directory.
    #[must_use]
    pub fn working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Add an environment variable.
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Set whether to capture stdout.
    #[must_use]
    pub const fn capture_stdout(mut self, capture: bool) -> Self {
        self.capture_stdout = capture;
        self
    }

    /// Set whether to capture stderr.
    #[must_use]
    pub const fn capture_stderr(mut self, capture: bool) -> Self {
        self.capture_stderr = capture;
        self
    }
}

/// Result of process execution.
#[derive(Debug, Clone)]
pub struct ProcessResult {
    /// Exit code of the process
    pub exit_code: Option<i32>,
    /// Captured stdout (if enabled)
    pub stdout: Vec<String>,
    /// Captured stderr (if enabled)
    pub stderr: Vec<String>,
}

impl ProcessResult {
    /// Check if the process succeeded (exit code 0).
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.exit_code == Some(0)
    }

    /// Get the exit code or return an error if process was terminated.
    pub fn exit_code(&self) -> Result<i32> {
        self.exit_code
            .ok_or_else(|| Error::command_failed(-1, "Process was terminated without exit code"))
    }

    /// Convert to Result, checking for success.
    pub fn check_success(&self) -> Result<()> {
        if self.is_success() {
            Ok(())
        } else {
            let code = self.exit_code.unwrap_or(-1);
            let stderr = self.stderr.join("\n");
            Err(Error::command_failed(code, stderr))
        }
    }
}

/// Handle to a running worker process.
pub struct WorkerProcess {
    /// The child process handle
    child: Child,
    /// Process configuration
    config: ProcessConfig,
}

impl WorkerProcess {
    /// Spawn a new worker process.
    pub async fn spawn(config: ProcessConfig) -> Result<Self> {
        let mut command = Command::new(&config.command);

        // Set arguments
        command.args(&config.args);

        // Set working directory
        if let Some(ref dir) = config.working_dir {
            command.current_dir(Path::new(dir));
        }

        // Set environment variables
        for (key, value) in &config.env {
            command.env(key, value);
        }

        // Configure stdio
        command.stdin(Stdio::null());

        if config.capture_stdout {
            command.stdout(Stdio::piped());
        } else {
            command.stdout(Stdio::null());
        }

        if config.capture_stderr {
            command.stderr(Stdio::piped());
        } else {
            command.stderr(Stdio::null());
        }

        // Spawn the process
        let child = command.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::CommandNotFound {
                    cmd: config.command.clone(),
                }
            } else {
                Error::command_failed(
                    -1,
                    format!("Failed to spawn process '{}': {}", config.command, e),
                )
            }
        })?;

        Ok(Self { child, config })
    }

    /// Get the process ID.
    #[must_use]
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    /// Wait for the process to complete and collect output.
    pub async fn wait_with_output(mut self) -> Result<ProcessResult> {
        let stdout_task = if self.config.capture_stdout {
            let stdout = self.child.stdout.take();
            Some(tokio::spawn(async move { collect_lines(stdout).await }))
        } else {
            None
        };

        let stderr_task = if self.config.capture_stderr {
            let stderr = self.child.stderr.take();
            Some(tokio::spawn(async move { collect_lines(stderr).await }))
        } else {
            None
        };

        // Wait for the process to complete
        let status =
            self.child.wait().await.map_err(|e| {
                Error::command_failed(-1, format!("Failed to wait for process: {}", e))
            })?;

        let stdout_lines = match stdout_task {
            Some(task) => task
                .await
                .map_err(|e| Error::command_failed(-1, format!("stdout task failed: {}", e)))??,
            None => Vec::new(),
        };

        let stderr_lines = match stderr_task {
            Some(task) => task
                .await
                .map_err(|e| Error::command_failed(-1, format!("stderr task failed: {}", e)))??,
            None => Vec::new(),
        };

        Ok(ProcessResult {
            exit_code: status.code(),
            stdout: stdout_lines,
            stderr: stderr_lines,
        })
    }

    /// Kill the process.
    pub async fn kill(mut self) -> Result<()> {
        self.child
            .kill()
            .await
            .map_err(|e| Error::command_failed(-1, format!("Failed to kill process: {}", e)))
    }

    /// Try to kill the process (does not fail if already dead).
    pub fn try_kill(&mut self) -> Result<()> {
        self.child
            .start_kill()
            .map_err(|e| Error::command_failed(-1, format!("Failed to start kill process: {}", e)))
    }
}

async fn collect_lines<T>(source: Option<T>) -> Result<Vec<String>>
where
    T: tokio::io::AsyncRead + Unpin,
{
    let Some(stream) = source else {
        return Ok(Vec::new());
    };

    let reader = BufReader::new(stream);
    let mut lines = reader.lines();
    let mut collected = Vec::new();

    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| Error::command_failed(-1, format!("Failed to read output: {}", e)))?
    {
        collected.push(line);
    }

    Ok(collected)
}

/// Convenience function to spawn and wait for a process.
pub async fn spawn_and_wait(config: ProcessConfig) -> Result<ProcessResult> {
    let process = WorkerProcess::spawn(config).await?;
    process.wait_with_output().await
}

/// Convenience function to run a simple command and check success.
pub async fn run_command(
    command: impl Into<String>,
    args: impl IntoIterator<Item = impl Into<String>>,
) -> Result<ProcessResult> {
    let config = ProcessConfig::new(command).args(args);
    let result = spawn_and_wait(config).await?;
    result.check_success()?;
    Ok(result)
}

/// Convenience function to run a command in a specific directory.
pub async fn run_command_in_dir(
    command: impl Into<String>,
    args: impl IntoIterator<Item = impl Into<String>>,
    working_dir: impl Into<String>,
) -> Result<ProcessResult> {
    let config = ProcessConfig::new(command)
        .args(args)
        .working_dir(working_dir);
    let result = spawn_and_wait(config).await?;
    result.check_success()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_simple_command() {
        let config = ProcessConfig::new("echo").arg("hello");
        let result = spawn_and_wait(config).await;
        assert!(result.is_ok());

        let result = result.ok();
        if let Some(res) = result {
            assert!(res.is_success());
            assert_eq!(res.stdout.len(), 1);
            assert_eq!(res.stdout[0], "hello");
        }
    }

    #[tokio::test]
    async fn test_spawn_with_args() {
        let config = ProcessConfig::new("echo").args(["hello", "world"]);
        let result = spawn_and_wait(config).await;
        assert!(result.is_ok());

        let result = result.ok();
        if let Some(res) = result {
            assert!(res.is_success());
            assert_eq!(res.stdout.len(), 1);
            assert_eq!(res.stdout[0], "hello world");
        }
    }

    #[tokio::test]
    async fn test_failed_command() {
        let config = ProcessConfig::new("false");
        let result = spawn_and_wait(config).await;
        assert!(result.is_ok());

        let result = result.ok();
        if let Some(res) = result {
            assert!(!res.is_success());
            assert_eq!(res.exit_code, Some(1));
        }
    }

    #[tokio::test]
    async fn test_command_not_found() {
        let config = ProcessConfig::new("this_command_does_not_exist_xyz");
        let result = spawn_and_wait(config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_working_directory() {
        let config = ProcessConfig::new("pwd").working_dir("/tmp");
        let result = spawn_and_wait(config).await;
        assert!(result.is_ok());

        let result = result.ok();
        if let Some(res) = result {
            assert!(res.is_success());
            assert_eq!(res.stdout.len(), 1);
            assert_eq!(res.stdout[0], "/tmp");
        }
    }

    #[tokio::test]
    async fn test_environment_variables() {
        let config = ProcessConfig::new("sh")
            .arg("-c")
            .arg("echo $TEST_VAR")
            .env("TEST_VAR", "test_value");
        let result = spawn_and_wait(config).await;
        assert!(result.is_ok());

        let result = result.ok();
        if let Some(res) = result {
            assert!(res.is_success());
            assert_eq!(res.stdout.len(), 1);
            assert_eq!(res.stdout[0], "test_value");
        }
    }

    #[tokio::test]
    async fn test_run_command_helper() {
        let result = run_command("echo", ["hello"]).await;
        assert!(result.is_ok());

        let result = result.ok();
        if let Some(res) = result {
            assert!(res.is_success());
            assert_eq!(res.stdout[0], "hello");
        }
    }

    #[tokio::test]
    async fn test_run_command_in_dir_helper() {
        let result = run_command_in_dir("pwd", Vec::<String>::new(), "/tmp").await;
        assert!(result.is_ok());

        let result = result.ok();
        if let Some(res) = result {
            assert!(res.is_success());
            assert_eq!(res.stdout[0], "/tmp");
        }
    }

    #[tokio::test]
    async fn test_process_id() {
        let config = ProcessConfig::new("sleep").arg("0.1");
        let process = WorkerProcess::spawn(config).await;
        assert!(process.is_ok());

        if let Ok(proc) = process {
            let pid = proc.id();
            assert!(pid.is_some());
            let _result = proc.wait_with_output().await;
        }
    }

    #[tokio::test]
    async fn test_kill_process() {
        let config = ProcessConfig::new("sleep").arg("10");
        let process = WorkerProcess::spawn(config).await;
        assert!(process.is_ok());

        if let Ok(proc) = process {
            let result = proc.kill().await;
            assert!(result.is_ok());
        }
    }
}
