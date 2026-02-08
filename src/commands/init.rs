#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Init command implementation
//!
//! Scaffolds new Oya projects with best-practice templates.

use anyhow::{Context, Result};
use clap::Parser;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use tokio::fs;
use tokio::process::Command;
use tracing::{debug, info};

/// Arguments for the init command
#[derive(Parser, Debug, Clone)]
pub struct InitArgs {
    /// Project name
    pub name: String,

    /// Template to use (minimal, full)
    #[arg(long, default_value = "minimal")]
    pub template: String,

    /// Force overwrite existing directory
    #[arg(long)]
    pub force: bool,

    /// Skip git initialization
    #[arg(long)]
    pub no_git: bool,

    /// Non-interactive mode (use all defaults)
    #[arg(long)]
    pub non_interactive: bool,

    /// Project description
    #[arg(long)]
    pub description: Option<String>,
}

/// Output from the init command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitOutput {
    /// Project directory path
    pub project_path: PathBuf,
    /// Files created
    pub files_created: Vec<String>,
    /// Git initialized
    pub git_initialized: bool,
}

/// Errors specific to the init command
#[derive(Debug, Error)]
pub enum InitError {
    #[error("Directory already exists: {path}")]
    DirectoryExists { path: PathBuf },

    #[error("Invalid project name: {name}")]
    InvalidProjectName { name: String },

    #[error("Template not found: {template}")]
    TemplateNotFound { template: String },

    #[error("Permission denied creating directory: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Git initialization failed: {error}")]
    GitFailed { error: String },

    #[error("Template rendering error: {error}")]
    TemplateRender { error: String },
}

impl InitError {
    /// Get the exit code for this error
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::DirectoryExists { .. } => 3,
            Self::InvalidProjectName { .. } => 4,
            Self::TemplateNotFound { .. } => 5,
            Self::PermissionDenied { .. } => 6,
            Self::GitFailed { .. } => 7,
            Self::TemplateRender { .. } => 8,
        }
    }

    /// Get a hint for remediation
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::DirectoryExists { .. } => {
                Some("Use --force to overwrite, or choose a different name".to_string())
            }
            Self::InvalidProjectName { .. } => Some(
                "Project names must be alphanumeric with hyphens, cannot be Rust keywords"
                    .to_string(),
            ),
            Self::TemplateNotFound { .. } => {
                Some("Available templates: minimal, full".to_string())
            }
            Self::PermissionDenied { .. } => Some("Check parent directory permissions".to_string()),
            Self::GitFailed { .. } => {
                Some("Check git installation or use --no-git".to_string())
            }
            Self::TemplateRender { .. } => {
                Some("Template file may have invalid syntax".to_string())
            }
        }
    }
}

/// Template file content
#[derive(Debug, Clone)]
struct TemplateFile {
    path: String,
    content: String,
}

/// Get template files for minimal template
fn get_minimal_template() -> Vec<TemplateFile> {
    vec![
        TemplateFile {
            path: "Cargo.toml".to_string(),
            content: r#"[workspace]
members = [
    "crates/*",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.85"
authors = ["Your Name <you@example.com>"]
license = "MIT"
repository = "https://github.com/yourusername/{{project_name}}"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"

[workspace.dependencies]
# Add your workspace dependencies here
"#
            .to_string(),
        },
        TemplateFile {
            path: "moon.yml".to_string(),
            content: r#"# Moon CI/CD Configuration
"$schema": 'https://moonrepo.dev/schemas/project.json'

# Project configuration
"#
            .to_string(),
        },
        TemplateFile {
            path: ".beads/README.md".to_string(),
            content: r#"# Beads Directory

This directory contains bead definitions for the OYA workflow system.

Beads are units of work that can be tracked, tested, and integrated.
"#
            .to_string(),
        },
        TemplateFile {
            path: "CLAUDE.md".to_string(),
            content: r#"# {{project_name}}

{{description}}

## Getting Started

This project uses the OYA SDLC system for development workflow.

## Commands

- `moon run :build` - Build the project
- `moon run :test` - Run tests
- `moon run :check` - Run linters and type checking

## Project Structure

```
.
├── crates/           # Workspace crates
├── .beads/           # Bead definitions
├── moon.yml          # Moon configuration
└── Cargo.toml        # Workspace configuration
```
"#
            .to_string(),
        },
        TemplateFile {
            path: ".gitignore".to_string(),
            content: r#"# Rust
/target
**/*.rs.bk
*.pdb

# Cargo
Cargo.lock

# IDE
.idea/
.vscode/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# OYA
.beads/*.db
"#
            .to_string(),
        },
    ]
}

/// Get template files for full template
fn get_full_template() -> Vec<TemplateFile> {
    let mut files = get_minimal_template();

    files.extend(vec![
        TemplateFile {
            path: ".github/workflows/ci.yml".to_string(),
            content: r#"name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: moonrepo/setup-moon@v1
      - run: moon run ci
"#
            .to_string(),
        },
        TemplateFile {
            path: "crates/core/Cargo.toml".to_string(),
            content: r#"[package]
name = "{{project_name}}-core"
version.workspace = true
edition.workspace = true

[lints]
workspace = true

[dependencies]
thiserror = { workspace = true }
anyhow = { workspace = true }
"#
            .to_string(),
        },
        TemplateFile {
            path: "crates/core/src/lib.rs".to_string(),
            content: r#"#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Core library for {{project_name}}
"#
            .to_string(),
        },
        TemplateFile {
            path: "tests/integration_test.rs".to_string(),
            content: r#"// Integration tests for {{project_name}}
"#
            .to_string(),
        },
    ]);

    files
}

/// Core function to validate project name
fn validate_project_name(name: &str) -> Result<(), InitError> {
    if name.is_empty() {
        return Err(InitError::InvalidProjectName {
            name: name.to_string(),
        });
    }

    // Check for valid characters
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(InitError::InvalidProjectName {
            name: name.to_string(),
        });
    }

    // Check for reserved Rust keywords
    let keywords = [
        "fn", "struct", "enum", "impl", "trait", "type", "const", "static", "let", "mut",
        "ref", "move", "async", "await", "loop", "while", "for", "match", "if", "else",
    ];

    if keywords.contains(&name) {
        return Err(InitError::InvalidProjectName {
            name: name.to_string(),
        });
    }

    Ok(())
}

/// Core function to get template files
fn get_template_files(template: &str) -> Result<Vec<TemplateFile>, InitError> {
    match template {
        "minimal" => Ok(get_minimal_template()),
        "full" => Ok(get_full_template()),
        _ => Err(InitError::TemplateNotFound {
            template: template.to_string(),
        }),
    }
}

/// Core function to render template variables
fn render_template(content: &str, vars: &HashMap<String, String>) -> String {
    let mut result = content.to_string();

    for (key, value) in vars {
        let placeholder = format!("{{{{{key}}}}}");
        result = result.replace(&placeholder, value);
    }

    result
}

/// Core function to sanitize project name
fn sanitize_project_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .join("-")
}

/// Shell function: Create directory
async fn create_directory(path: &PathBuf) -> Result<(), InitError> {
    fs::create_dir_all(path)
        .await
        .map_err(|_| InitError::PermissionDenied {
            path: path.clone(),
        })?;

    Ok(())
}

/// Shell function: Write file
async fn write_file(path: &PathBuf, content: &str) -> Result<(), InitError> {
    let parent = match path.parent() {
        Some(p) => p,
        None => return Ok(()),
    };

    fs::create_dir_all(parent)
        .await
        .map_err(|_| InitError::PermissionDenied {
            path: parent.to_path_buf(),
        })?;

    fs::write(path, content)
        .await
        .map_err(|_| InitError::PermissionDenied {
            path: path.clone(),
        })?;

    Ok(())
}

/// Shell function: Initialize git repository
async fn init_git(project_path: &PathBuf) -> Result<bool, InitError> {
    let git_dir = project_path.join(".git");

    // Check if already initialized
    if git_dir.exists() {
        debug!("Git already initialized");
        return Ok(false);
    }

    let output = Command::new("git")
        .args(["init"])
        .current_dir(project_path)
        .output()
        .await;

    match output {
        Ok(_) => {
            info!("Initialized git repository");
            Ok(true)
        }
        Err(e) => Err(InitError::GitFailed {
            error: e.to_string(),
        }),
    }
}

/// Main init command implementation
pub async fn init_command(args: InitArgs) -> Result<InitOutput, InitError> {
    debug!("Running init command with args: {args:?}");

    // Validate and sanitize project name
    validate_project_name(&args.name)?;

    let sanitized_name = sanitize_project_name(&args.name);
    let project_path = PathBuf::from(&sanitized_name);

    // Check if directory exists
    if project_path.exists() && !args.force {
        return Err(InitError::DirectoryExists {
            path: project_path,
        });
    }

    // Remove existing directory if force is enabled
    if project_path.exists() && args.force {
        debug!("Removing existing directory: {project_path:?}");
        fs::remove_dir_all(&project_path)
            .await
            .map_err(|_| InitError::PermissionDenied {
                path: project_path.clone(),
            })?;
    }

    // Create project directory
    create_directory(&project_path).await?;

    info!("Created project directory: {project_path:?}");

    // Get template files
    let template_files = get_template_files(&args.template)?;

    // Prepare template variables
    let mut vars = HashMap::new();
    vars.insert(
        "project_name".to_string(),
        sanitized_name.clone(),
    );
    vars.insert(
        "description".to_string(),
        args
            .description
            .clone()
            .unwrap_or_else(|| "A new Oya project".to_string()),
    );

    // Create files
    let mut files_created = Vec::new();

    for template_file in template_files {
        let file_path = project_path.join(&template_file.path);
        let content = render_template(&template_file.content, &vars);

        write_file(&file_path, &content).await?;

        debug!("Created file: {}", template_file.path);
        files_created.push(template_file.path);
    }

    // Initialize git if not disabled
    let git_initialized = if !args.no_git {
        init_git(&project_path).await?
    } else {
        false
    };

    Ok(InitOutput {
        project_path,
        files_created,
        git_initialized,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn test_validate_project_name_valid() {
        assert!(validate_project_name("my-project").is_ok());
        assert!(validate_project_name("my_project").is_ok());
        assert!(validate_project_name("MyProject123").is_ok());
    }

    #[test]
    fn test_validate_project_name_invalid() {
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("@invalid@").is_err());
        assert!(validate_project_name("fn").is_err()); // Rust keyword
    }

    #[test]
    fn test_sanitize_project_name() {
        assert_eq!(sanitize_project_name("MyProject"), "myproject");
        assert_eq!(sanitize_project_name("my-project"), "my-project");
        assert_eq!(sanitize_project_name("my_project"), "my-project");
        assert_eq!(sanitize_project_name("My@Project"), "my-project");
    }

    #[test]
    fn test_render_template() {
        let mut vars = HashMap::new();
        vars.insert("project_name".to_string(), "test-project".to_string());
        vars.insert("description".to_string(), "Test desc".to_string());

        let content = "Project: {{project_name}}, Desc: {{description}}";
        let rendered = render_template(content, &vars);

        assert_eq!(rendered, "Project: test-project, Desc: Test desc");
    }

    #[test]
    fn test_get_template_files_minimal() {
        let files = get_template_files("minimal");

        assert!(files.is_ok());
        let files = files.unwrap();

        assert!(files.iter().any(|f| f.path == "Cargo.toml"));
        assert!(files.iter().any(|f| f.path == "moon.yml"));
        assert!(files.iter().any(|f| f.path == "CLAUDE.md"));
    }

    #[test]
    fn test_get_template_files_full() {
        let files = get_template_files("full");

        assert!(files.is_ok());
        let files = files.unwrap();

        assert!(files.iter().any(|f| f.path == "Cargo.toml"));
        assert!(files.iter().any(|f| f.path == ".github/workflows/ci.yml"));
        assert!(files.iter().any(|f| f.path == "crates/core/src/lib.rs"));
    }

    #[test]
    fn test_get_template_files_invalid() {
        let files = get_template_files("nonexistent");

        assert!(files.is_err());
    }
}
