//! CLI utilities and helpers

#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::Command;

use anyhow::{Context, Result};

/// Execute a shell command and return its output
pub fn run_command(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute {program}"))?;

    if output.status.success() {
        String::from_utf8(output.stdout).context("Invalid UTF-8 output from command")
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{program} failed: {stderr}")
    }
}

/// Check if we're inside a Zellij session
pub fn is_inside_zellij() -> bool {
    std::env::var("ZELLIJ").is_ok()
}

/// Check if current directory is a JJ repository
pub fn is_jj_repo() -> Result<bool> {
    let result = Command::new("jj")
        .args(["root"])
        .output()
        .context("Failed to run jj")?;

    Ok(result.status.success())
}

/// Get JJ repository root
pub fn jj_root() -> Result<String> {
    run_command("jj", &["root"]).map(|s| s.trim().to_string())
}

/// Check if a command is available in PATH
pub fn is_command_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if JJ is installed
pub fn is_jj_installed() -> bool {
    is_command_available("jj")
}

/// Check if Zellij is installed
pub fn is_zellij_installed() -> bool {
    is_command_available("zellij")
}

/// Attach to or create a Zellij session, optionally with a layout
/// This function will exec into Zellij, replacing the current process
#[cfg(unix)]
pub fn attach_to_zellij_session(layout_content: Option<&str>) -> Result<()> {
    // Check if Zellij is installed
    if !is_zellij_installed() {
        anyhow::bail!("Zellij is not installed. Please install it first.");
    }

    // Get the session name from the JJ repo root or use default
    let session_name = jj_root()
        .ok()
        .and_then(|root| {
            std::path::Path::new(&root)
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| format!("jjz-{s}"))
        })
        .unwrap_or_else(|| "jjz".to_string());

    // Print a helpful message before attaching
    eprintln!("Attaching to Zellij session '{session_name}'...");

    // We'll attach to or create the Zellij session
    // Using exec to replace the current process
    let zellij_path = which::which("zellij").context("Failed to find zellij in PATH")?;

    let mut cmd = std::process::Command::new(zellij_path);

    // If layout content provided, write it to a temp file and use it
    if let Some(layout) = layout_content {
        let temp_dir = std::env::temp_dir();
        let layout_path = temp_dir.join(format!("jjz-{}.kdl", std::process::id()));
        std::fs::write(&layout_path, layout)?;

        cmd.args([
            "--layout",
            layout_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid layout path"))?,
            "attach",
            "-c",
            &session_name,
        ]);
    } else {
        cmd.args(["attach", "-c", &session_name]);
    }

    // Exec into Zellij
    let err = cmd.exec();

    // If we get here, exec failed
    Err(anyhow::anyhow!("Failed to exec into Zellij: {err}"))
}

/// Attach to or create a Zellij session, optionally with a layout
/// Windows version - not supported
#[cfg(not(unix))]
pub fn attach_to_zellij_session(_layout_content: Option<&str>) -> Result<()> {
    anyhow::bail!("Auto-spawning Zellij is only supported on Unix systems");
}

/// Check if current directory is a Git repository
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_command_success() {
        let result = run_command("echo", &["hello"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap_or_default().trim(), "hello");
    }

    #[test]
    fn test_run_command_failure() {
        let result = run_command("false", &[]);
        assert!(result.is_err());
    }
}
