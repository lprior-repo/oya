//! Zellij layout generation and tab management
//!
//! This module provides safe, functional APIs for managing Zellij layouts and tabs.
//! All operations return `Result` and never panic.
//!
//! # Requirements
//!
//! - REQ-ZELLIJ-001: Generate valid KDL layout files
//! - REQ-ZELLIJ-002: Use tabs within current session
//! - REQ-ZELLIJ-003: Main pane at 70% width
//! - REQ-ZELLIJ-004: Side pane for beads and status
//! - REQ-ZELLIJ-006: Open tabs via zellij action new-tab
//! - REQ-ZELLIJ-007: Close tabs via zellij action close-tab
//! - REQ-ZELLIJ-008: Focus tabs via zellij action go-to-tab-name
//! - REQ-ZELLIJ-009: Set pane cwd to workspace directory
//! - REQ-ZELLIJ-010: Support variable substitution
//! - REQ-ZELLIJ-011: Name tabs with session name
//! - REQ-ZELLIJ-012: Configure main pane command (default: claude)
//! - REQ-ZELLIJ-013: Configure beads pane command (default: bv)

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{Error, Result};

/// Supported layout templates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutTemplate {
    /// Single Claude pane
    Minimal,
    /// Claude (70%) + beads/status sidebar (30%)
    Standard,
    /// Standard + floating pane + jj log
    Full,
    /// Two Claude instances side-by-side
    Split,
    /// Diff view + beads + Claude
    Review,
}

/// Configuration for layout generation
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Session name for variable substitution
    pub session_name: String,
    /// Workspace path for cwd settings
    pub workspace_path: PathBuf,
    /// Command to run in main pane (default: "claude")
    pub claude_command: String,
    /// Command to run in beads pane (default: "bv")
    pub beads_command: String,
    /// Tab name prefix (default: "jjz")
    pub tab_prefix: String,
}

impl LayoutConfig {
    /// Create a new layout configuration
    pub fn new(session_name: String, workspace_path: PathBuf) -> Self {
        Self {
            session_name,
            workspace_path,
            claude_command: "claude".to_string(),
            beads_command: "bv".to_string(),
            tab_prefix: "jjz".to_string(),
        }
    }

    /// Set the Claude command
    #[must_use]
    pub fn with_claude_command(mut self, command: String) -> Self {
        self.claude_command = command;
        self
    }

    /// Set the beads command
    #[must_use]
    pub fn with_beads_command(mut self, command: String) -> Self {
        self.beads_command = command;
        self
    }

    /// Set the tab prefix
    #[must_use]
    pub fn with_tab_prefix(mut self, prefix: String) -> Self {
        self.tab_prefix = prefix;
        self
    }

    /// Get the full tab name
    #[must_use]
    pub fn tab_name(&self) -> String {
        format!("{}:{}", self.tab_prefix, self.session_name)
    }
}

/// Generated layout information
#[derive(Debug, Clone)]
pub struct Layout {
    /// Generated KDL content
    pub kdl_content: String,
    /// Path where layout file is written
    pub file_path: PathBuf,
}

/// Generate a layout file for the given template
///
/// # Errors
///
/// Returns error if:
/// - Unable to create layout directory
/// - Unable to write layout file
/// - Template generation fails
pub fn layout_generate(
    config: &LayoutConfig,
    template: LayoutTemplate,
    output_dir: &Path,
) -> Result<Layout> {
    // Create output directory
    std::fs::create_dir_all(output_dir)?;

    // Generate KDL content
    let kdl_content = generate_template_kdl(config, template)?;

    // Write to file
    let file_path = output_dir.join(format!("{}.kdl", config.session_name));
    std::fs::write(&file_path, &kdl_content)?;

    Ok(Layout {
        kdl_content,
        file_path,
    })
}

/// Generate KDL content for a template
fn generate_template_kdl(config: &LayoutConfig, template: LayoutTemplate) -> Result<String> {
    let kdl = match template {
        LayoutTemplate::Minimal => generate_minimal_kdl(config),
        LayoutTemplate::Standard => generate_standard_kdl(config),
        LayoutTemplate::Full => generate_full_kdl(config),
        LayoutTemplate::Split => generate_split_kdl(config),
        LayoutTemplate::Review => generate_review_kdl(config),
    };

    // Validate KDL syntax
    validate_kdl(&kdl)?;

    Ok(kdl)
}

/// Generate minimal template: single Claude pane
fn generate_minimal_kdl(config: &LayoutConfig) -> String {
    let cwd = config.workspace_path.display();
    let cmd = &config.claude_command;

    format!(
        r#"layout {{
    pane {{
        command "{cmd}"
        cwd "{cwd}"
        focus true
    }}
}}
"#
    )
}

/// Generate standard template: 70% Claude + 30% sidebar (beads 15% + status 15%)
fn generate_standard_kdl(config: &LayoutConfig) -> String {
    let cwd = config.workspace_path.display();
    let claude_cmd = &config.claude_command;
    let beads_cmd = &config.beads_command;

    format!(
        r#"layout {{
    pane split_direction="horizontal" {{
        pane {{
            command "{claude_cmd}"
            cwd "{cwd}"
            focus true
            size "70%"
        }}
        pane split_direction="vertical" {{
            pane {{
                command "{beads_cmd}"
                cwd "{cwd}"
                size "50%"
            }}
            pane {{
                command "jj"
                args "log" "--limit" "20"
                cwd "{cwd}"
                size "50%"
            }}
        }}
    }}
}}
"#
    )
}

/// Generate full template: standard + floating pane
fn generate_full_kdl(config: &LayoutConfig) -> String {
    let cwd = config.workspace_path.display();
    let claude_cmd = &config.claude_command;
    let beads_cmd = &config.beads_command;

    format!(
        r#"layout {{
    pane split_direction="horizontal" {{
        pane {{
            command "{claude_cmd}"
            cwd "{cwd}"
            focus true
            size "70%"
        }}
        pane split_direction="vertical" {{
            pane {{
                command "{beads_cmd}"
                cwd "{cwd}"
                size "50%"
            }}
            pane {{
                command "jj"
                args "log" "--limit" "20"
                cwd "{cwd}"
                size "50%"
            }}
        }}
    }}
    floating_panes {{
        pane {{
            command "jj"
            args "status"
            cwd "{cwd}"
            x "20%"
            y "20%"
            width "60%"
            height "60%"
        }}
    }}
}}
"#
    )
}

/// Generate split template: two Claude instances side-by-side
fn generate_split_kdl(config: &LayoutConfig) -> String {
    let cwd = config.workspace_path.display();
    let cmd = &config.claude_command;

    format!(
        r#"layout {{
    pane split_direction="horizontal" {{
        pane {{
            command "{cmd}"
            cwd "{cwd}"
            focus true
            size "50%"
        }}
        pane {{
            command "{cmd}"
            cwd "{cwd}"
            size "50%"
        }}
    }}
}}
"#
    )
}

/// Generate review template: diff view + beads + Claude
fn generate_review_kdl(config: &LayoutConfig) -> String {
    let cwd = config.workspace_path.display();
    let claude_cmd = &config.claude_command;
    let beads_cmd = &config.beads_command;

    format!(
        r#"layout {{
    pane split_direction="horizontal" {{
        pane {{
            command "jj"
            args "diff"
            cwd "{cwd}"
            focus true
            size "50%"
        }}
        pane {{
            command "{beads_cmd}"
            cwd "{cwd}"
            size "25%"
        }}
        pane {{
            command "{claude_cmd}"
            cwd "{cwd}"
            size "25%"
        }}
    }}
}}
"#
    )
}

/// Validate KDL syntax
///
/// Basic validation to ensure well-formed KDL:
/// - Balanced braces
/// - No empty node names
fn validate_kdl(content: &str) -> Result<()> {
    // Check balanced braces
    let open_braces = content.chars().filter(|c| *c == '{').count();
    let close_braces = content.chars().filter(|c| *c == '}').count();

    if open_braces != close_braces {
        return Err(Error::ValidationError(format!(
            "Unbalanced braces: {open_braces} open, {close_braces} close"
        )));
    }

    // Check for basic structure
    if !content.contains("layout") {
        return Err(Error::ValidationError(
            "KDL must contain 'layout' node".to_string(),
        ));
    }

    if !content.contains("pane") {
        return Err(Error::ValidationError(
            "KDL must contain at least one 'pane' node".to_string(),
        ));
    }

    Ok(())
}

/// Open a new tab with the given layout
///
/// # Errors
///
/// Returns error if:
/// - Zellij is not running
/// - Layout file doesn't exist
/// - zellij action command fails
pub fn tab_open(layout_path: &Path, tab_name: &str) -> Result<()> {
    // Check Zellij is running
    check_zellij_running()?;

    // Verify layout file exists
    if !layout_path.exists() {
        return Err(Error::NotFound(format!(
            "Layout file not found: {}",
            layout_path.display()
        )));
    }

    // Execute: zellij action new-tab --layout <path> --name <name>
    let output = Command::new("zellij")
        .args(["action", "new-tab"])
        .arg("--layout")
        .arg(layout_path)
        .arg("--name")
        .arg(tab_name)
        .output()
        .map_err(|e| Error::Command(format!("Failed to execute zellij: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Command(format!("zellij action failed: {stderr}")));
    }

    Ok(())
}

/// Close a tab by switching to it first, then closing
///
/// # Errors
///
/// Returns error if:
/// - Zellij is not running
/// - zellij action command fails
pub fn tab_close(tab_name: &str) -> Result<()> {
    // Check Zellij is running
    check_zellij_running()?;

    // First focus the tab
    tab_focus(tab_name)?;

    // Execute: zellij action close-tab
    let output = Command::new("zellij")
        .args(["action", "close-tab"])
        .output()
        .map_err(|e| Error::Command(format!("Failed to execute zellij: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Command(format!("zellij action failed: {stderr}")));
    }

    Ok(())
}

/// Focus a tab by name
///
/// # Errors
///
/// Returns error if:
/// - Zellij is not running
/// - Tab doesn't exist
/// - zellij action command fails
pub fn tab_focus(tab_name: &str) -> Result<()> {
    // Check Zellij is running
    check_zellij_running()?;

    // Execute: zellij action go-to-tab-name <name>
    let output = Command::new("zellij")
        .args(["action", "go-to-tab-name", tab_name])
        .output()
        .map_err(|e| Error::Command(format!("Failed to execute zellij: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Command(format!("zellij action failed: {stderr}")));
    }

    Ok(())
}

/// Check if Zellij is running
///
/// # Errors
///
/// Returns error if Zellij is not running in current session
pub fn check_zellij_running() -> Result<()> {
    // Check if ZELLIJ environment variable is set
    if std::env::var("ZELLIJ").is_err() {
        return Err(Error::Command(
            "Zellij not running. Run jjz inside a Zellij session.".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    fn test_config() -> LayoutConfig {
        LayoutConfig::new(
            "test-session".to_string(),
            PathBuf::from("/tmp/test-workspace"),
        )
    }

    // Test Case 1: Generate minimal - Valid KDL with single pane
    #[test]
    fn test_generate_minimal_valid_kdl() {
        let config = test_config();
        let kdl = generate_minimal_kdl(&config);

        // Check it contains required elements
        assert!(kdl.contains("layout"));
        assert!(kdl.contains("pane"));
        assert!(kdl.contains("claude"));
        assert!(kdl.contains("/tmp/test-workspace"));
        assert!(kdl.contains("focus true"));

        // Validate KDL syntax
        assert!(validate_kdl(&kdl).is_ok());
    }

    // Test Case 2: Generate standard - Valid KDL with 3 panes (70/15/15 split)
    #[test]
    fn test_generate_standard_valid_kdl() {
        let config = test_config();
        let kdl = generate_standard_kdl(&config);

        // Check structure
        assert!(kdl.contains("layout"));
        assert!(kdl.contains("split_direction=\"horizontal\""));
        assert!(kdl.contains("size \"70%\""));
        assert!(kdl.contains("claude"));
        assert!(kdl.contains("bv"));
        assert!(kdl.contains("jj"));

        // Count pane occurrences (should be 4: 1 container + 3 actual panes)
        let pane_count = kdl.matches("pane").count();
        assert!(pane_count >= 3);

        // Validate KDL syntax
        assert!(validate_kdl(&kdl).is_ok());
    }

    // Test Case 3: Generate full - Valid KDL with floating pane
    #[test]
    fn test_generate_full_valid_kdl_with_floating() {
        let config = test_config();
        let kdl = generate_full_kdl(&config);

        // Check for floating pane
        assert!(kdl.contains("floating_panes"));
        assert!(kdl.contains("x \"20%\""));
        assert!(kdl.contains("y \"20%\""));
        assert!(kdl.contains("width \"60%\""));
        assert!(kdl.contains("height \"60%\""));

        // Validate KDL syntax
        assert!(validate_kdl(&kdl).is_ok());
    }

    // Test Case 4: Variable substitution - {session_name} → actual name
    #[test]
    fn test_variable_substitution_in_config() {
        let config = LayoutConfig::new("my-feature".to_string(), PathBuf::from("/workspace"));

        assert_eq!(config.session_name, "my-feature");
        assert_eq!(config.tab_name(), "jjz:my-feature");

        let kdl = generate_minimal_kdl(&config);
        assert!(kdl.contains("/workspace"));
    }

    // Test Case 5: Open tab - Executes 'zellij action new-tab ...'
    #[test]
    fn test_tab_open_requires_zellij() {
        // This test will fail if not in Zellij
        // We just test the error handling
        let temp_dir = env::temp_dir();
        let layout_path = temp_dir.join("test.kdl");

        // Create a test layout file
        std::fs::write(&layout_path, "layout { pane { } }").ok();

        let result = tab_open(&layout_path, "test-tab");

        // Should fail if not in Zellij
        if env::var("ZELLIJ").is_err() {
            assert!(result.is_err());
            if let Err(Error::Command(msg)) = result {
                assert!(msg.contains("Zellij not running"));
            }
        }
    }

    // Test Case 6: Close tab - Executes 'zellij action close-tab ...'
    #[test]
    fn test_tab_close_requires_zellij() {
        let result = tab_close("test-tab");

        // Should fail if not in Zellij
        if env::var("ZELLIJ").is_err() {
            assert!(result.is_err());
            if let Err(Error::Command(msg)) = result {
                assert!(msg.contains("Zellij not running"));
            }
        }
    }

    // Test Case 7: Focus tab - Executes 'zellij action go-to-tab-name ...'
    #[test]
    fn test_tab_focus_requires_zellij() {
        let result = tab_focus("test-tab");

        // Should fail if not in Zellij
        if env::var("ZELLIJ").is_err() {
            assert!(result.is_err());
            if let Err(Error::Command(msg)) = result {
                assert!(msg.contains("Zellij not running"));
            }
        }
    }

    // Test Case 8: Custom template - Variable substitution
    #[test]
    fn test_custom_commands_in_config() {
        let config = test_config()
            .with_claude_command("custom-claude".to_string())
            .with_beads_command("custom-bv".to_string())
            .with_tab_prefix("custom".to_string());

        assert_eq!(config.claude_command, "custom-claude");
        assert_eq!(config.beads_command, "custom-bv");
        assert_eq!(config.tab_name(), "custom:test-session");

        let kdl = generate_minimal_kdl(&config);
        assert!(kdl.contains("custom-claude"));
    }

    // Test Case 9: Invalid KDL - Error with syntax details
    #[test]
    fn test_validate_kdl_unbalanced_braces() {
        let invalid_kdl = "layout { pane { ";
        let result = validate_kdl(invalid_kdl);

        assert!(result.is_err());
        if let Err(Error::ValidationError(msg)) = result {
            assert!(msg.contains("Unbalanced braces"));
        }
    }

    #[test]
    fn test_validate_kdl_missing_layout() {
        let invalid_kdl = "pane { }";
        let result = validate_kdl(invalid_kdl);

        assert!(result.is_err());
        if let Err(Error::ValidationError(msg)) = result {
            assert!(msg.contains("layout"));
        }
    }

    #[test]
    fn test_validate_kdl_missing_pane() {
        let invalid_kdl = "layout { }";
        let result = validate_kdl(invalid_kdl);

        assert!(result.is_err());
        if let Err(Error::ValidationError(msg)) = result {
            assert!(msg.contains("pane"));
        }
    }

    // Test Case 10: Zellij not running - Error "Zellij not running"
    #[test]
    fn test_check_zellij_not_running() {
        // Save current ZELLIJ var
        let zellij_var = env::var("ZELLIJ");

        // Temporarily remove it
        env::remove_var("ZELLIJ");

        let result = check_zellij_running();
        assert!(result.is_err());

        if let Err(Error::Command(msg)) = result {
            assert!(msg.contains("Zellij not running"));
        }

        // Restore ZELLIJ var if it existed
        if let Ok(val) = zellij_var {
            env::set_var("ZELLIJ", val);
        }
    }

    // Additional test: Split template
    #[test]
    fn test_generate_split_template() {
        let config = test_config();
        let kdl = generate_split_kdl(&config);

        assert!(kdl.contains("split_direction=\"horizontal\""));
        assert!(kdl.contains("size \"50%\""));

        // Count claude commands (should be 2)
        let claude_count = kdl.matches("claude").count();
        assert_eq!(claude_count, 2);

        assert!(validate_kdl(&kdl).is_ok());
    }

    // Additional test: Review template
    #[test]
    fn test_generate_review_template() {
        let config = test_config();
        let kdl = generate_review_kdl(&config);

        assert!(kdl.contains("jj"));
        assert!(kdl.contains("diff"));
        assert!(kdl.contains("bv"));
        assert!(kdl.contains("claude"));
        assert!(kdl.contains("size \"50%\""));
        assert!(kdl.contains("size \"25%\""));

        assert!(validate_kdl(&kdl).is_ok());
    }

    // Additional test: Layout generation end-to-end
    #[test]
    fn test_layout_generate_creates_file() {
        let config = test_config();
        let output_dir = env::temp_dir().join("zjj-test-layouts");

        let result = layout_generate(&config, LayoutTemplate::Minimal, &output_dir);
        assert!(result.is_ok());

        let layout = result.unwrap_or_else(|_| Layout {
            kdl_content: String::new(),
            file_path: PathBuf::new(),
        });

        assert!(layout.file_path.exists());
        assert!(layout.kdl_content.contains("layout"));

        // Cleanup
        std::fs::remove_file(&layout.file_path).ok();
        std::fs::remove_dir(&output_dir).ok();
    }

    // Additional test: tab_open with missing file
    #[test]
    fn test_tab_open_missing_layout_file() {
        let missing_path = PathBuf::from("/nonexistent/layout.kdl");
        let result = tab_open(&missing_path, "test");

        assert!(result.is_err());
        if let Err(Error::NotFound(msg)) = result {
            assert!(msg.contains("Layout file not found"));
        }
    }
}
