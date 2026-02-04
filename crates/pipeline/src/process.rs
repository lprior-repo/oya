//! Shell command execution with proper error handling.
//!
//! Executes external commands with timeout support and captures output.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::{
    io::Read,
    path::Path,
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::error::{Error, Result};

/// Default command timeout in milliseconds.
const DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// Result of command execution.
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CommandResult {
    /// Check if command succeeded (exit code 0).
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get stdout if successful, error if failed.
    pub fn get_stdout(&self) -> Result<&str> {
        if self.is_success() {
            Ok(&self.stdout)
        } else {
            Err(Error::command_failed(self.exit_code, &self.stderr))
        }
    }

    /// Convert to Result, checking for success.
    pub fn check_success(&self) -> Result<()> {
        if self.is_success() {
            Ok(())
        } else {
            Err(Error::command_failed(self.exit_code, &self.stderr))
        }
    }
}

/// Execute a command with arguments in a working directory.
pub fn run_command(cmd: &str, args: &[&str], cwd: &Path) -> Result<CommandResult> {
    run_command_with_timeout(cmd, args, cwd, DEFAULT_TIMEOUT_MS)
}

/// Execute a command with custom timeout.
pub fn run_command_with_timeout(
    cmd: &str,
    args: &[&str],
    cwd: &Path,
    timeout_ms: u64,
) -> Result<CommandResult> {
    let mut command = Command::new(cmd);
    command
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = spawn_command(command, cmd)?;

    let stdout_handle = child.stdout.take().map(|mut out| {
        thread::spawn(move || {
            let mut buffer = String::new();
            let _ = out.read_to_string(&mut buffer);
            buffer
        })
    });

    let stderr_handle = child.stderr.take().map(|mut err| {
        thread::spawn(move || {
            let mut buffer = String::new();
            let _ = err.read_to_string(&mut buffer);
            buffer
        })
    });

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);

    loop {
        if let Some(status) = child.try_wait()? {
            let stdout: String = match stdout_handle {
                Some(handle) => handle.join().unwrap_or_default(),
                None => String::new(),
            };

            let stderr: String = match stderr_handle {
                Some(handle) => handle.join().unwrap_or_default(),
                None => String::new(),
            };

            return Ok(CommandResult {
                stdout,
                stderr,
                exit_code: status.code().unwrap_or(-1),
            });
        }

        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            if let Some(handle) = stdout_handle {
                let _ = handle.join();
            }
            if let Some(handle) = stderr_handle {
                let _ = handle.join();
            }
            return Err(Error::CommandTimeout { timeout_ms });
        }

        thread::sleep(Duration::from_millis(50));
    }
}

/// Execute a command with environment variables.
pub fn run_command_with_env(
    cmd: &str,
    args: &[&str],
    cwd: &Path,
    env: &[(&str, &str)],
) -> Result<CommandResult> {
    let mut command = Command::new(cmd);
    command
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (key, value) in env {
        command.env(key, value);
    }

    let output = command.output().map_err(|e| map_command_error(cmd, e))?;
    Ok(parse_output(&output))
}

/// Check if a command exists in PATH.
pub fn command_exists(cmd: &str) -> Result<bool> {
    let result = Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    Ok(result.is_ok_and(|status| status.success()))
}

/// Run a command only if it exists in PATH.
pub fn run_command_safe(cmd: &str, args: &[&str], cwd: &Path) -> Result<CommandResult> {
    if !command_exists(cmd)? {
        return Err(Error::CommandNotFound {
            cmd: cmd.to_string(),
        });
    }
    run_command(cmd, args, cwd)
}

/// Parse command output into `CommandResult`.
fn parse_output(output: &Output) -> CommandResult {
    CommandResult {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code: output.status.code().unwrap_or(-1),
    }
}

fn spawn_command(mut command: Command, cmd: &str) -> Result<std::process::Child> {
    command.spawn().map_err(|e| map_command_error(cmd, e))
}

fn map_command_error(cmd: &str, error: std::io::Error) -> Error {
    if error.kind() == std::io::ErrorKind::NotFound {
        Error::CommandNotFound {
            cmd: cmd.to_string(),
        }
    } else {
        Error::Io(error)
    }
}

/// Escape a string for safe shell execution using single quotes.
#[must_use]
pub fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}

/// Read text from a file.
pub fn read_text_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| Error::file_read_failed(path, e.to_string()))
}

/// Write text to a file.
pub fn write_text_file(path: &Path, content: &str) -> Result<()> {
    std::fs::write(path, content).map_err(|e| Error::file_write_failed(path, e.to_string()))
}

/// Append text to a file.
pub fn append_text_file(path: &Path, content: &str) -> Result<()> {
    use std::io::Write;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| Error::file_write_failed(path, e.to_string()))?;

    file.write_all(content.as_bytes())
        .map_err(|e| Error::file_write_failed(path, e.to_string()))
}

/// Create directory and all parent directories.
pub fn create_dir_all(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).map_err(|e| Error::directory_creation_failed(path, e.to_string()))
}

/// Check if a file exists.
#[must_use]
pub fn file_exists(path: &Path) -> bool {
    path.is_file()
}

/// Check if a directory exists.
#[must_use]
pub fn dir_exists(path: &Path) -> bool {
    path.is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_command_exists() {
        // 'ls' should exist on any Unix system
        let result = command_exists("ls");
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_command() {
        let result = run_command("echo", &["hello"], &PathBuf::from("."));
        assert!(result.is_ok());
        let cmd_result = result.ok();
        assert!(cmd_result.map(|r| r.is_success()).unwrap_or(false));
    }

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("hello"), "'hello'");
        assert_eq!(shell_escape("it's"), "'it'\"'\"'s'");
    }
}
