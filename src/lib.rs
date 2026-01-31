#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! # llms-txt-parser
//!
//! Parser for llms.txt files following the llms.txt specification.
//!
//! ## Usage
//!
//! ```rust
//! use llms_txt_parser::parse_content;
//!
//! let content = r#"# My Project
//! > A great project
//! ## Getting Started
//! - [Intro](./intro.md)
//! "#;
//!
//! let llms_txt = parse_content(content)?;
//! println!("Project: {}", llms_txt.project_name);
//! println!("Sections: {}", llms_txt.sections.len());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Parser errors
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("file not found: {0}")]
    FileNotFound(String),

    #[error("invalid frontmatter: {0}")]
    InvalidFrontmatter(String),

    #[error("missing required field: {0}")]
    MissingField(String),

    #[error("invalid format: {0}")]
    InvalidFormat(String),
}

/// YAML frontmatter metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Frontmatter {
    pub version: Option<String>,
    pub project: Option<String>,
    pub project_version: Option<String>,
    pub updated: Option<String>,
    pub documents: Option<usize>,
    pub index: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

/// A section in llms.txt (## heading)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub title: String,
    pub content: String,
    pub links: Vec<Link>,
}

/// A link within a section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub text: String,
    pub url: String,
    pub description: Option<String>,
}

/// Parsed llms.txt file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmsTxt {
    pub frontmatter: Option<Frontmatter>,
    pub project_name: String,
    pub description: Option<String>,
    pub sections: Vec<Section>,
}

impl LlmsTxt {
    /// Get section by title
    pub fn get_section(&self, title: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.title == title)
    }

    /// Check if required sections exist
    pub fn has_required_sections(&self) -> bool {
        let required = ["Getting Started", "Core Concepts", "API Reference"];
        required.iter().all(|&r| self.get_section(r).is_some())
    }

    /// Get index reference from frontmatter or Machine-Readable Index section
    pub fn get_index_reference(&self) -> Option<String> {
        if let Some(fm) = &self.frontmatter {
            if let Some(index) = &fm.index {
                return Some(index.clone());
            }
        }

        // Fallback: search Machine-Readable Index section
        if let Some(section) = self.get_section("Machine-Readable Index") {
            for link in &section.links {
                if link.url.contains("INDEX.json") {
                    return Some(link.url.clone());
                }
            }
        }

        None
    }
}

/// Parse llms.txt from a file
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<LlmsTxt> {
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read {}", path.as_ref().display()))?;

    parse_content(&content)
}

/// Parse llms.txt from string content
pub fn parse_content(content: &str) -> Result<LlmsTxt> {
    let (frontmatter, body) = extract_frontmatter(content)?;

    let lines: Vec<&str> = body.lines().collect();
    let mut project_name = String::new();
    let mut description = None;
    let mut sections = Vec::new();
    let mut current_section: Option<Section> = None;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // H1: Project name
        if let Some(stripped) = line.strip_prefix("# ") {
            project_name = stripped.trim().to_string();
            i += 1;
            continue;
        }

        // Blockquote: Description
        if let Some(stripped) = line.strip_prefix("> ") {
            description = Some(stripped.trim().to_string());
            i += 1;
            continue;
        }

        // H2: Section heading
        if let Some(stripped) = line.strip_prefix("## ") {
            // Save previous section
            if let Some(section) = current_section.take() {
                sections.push(section);
            }

            current_section = Some(Section {
                title: stripped.trim().to_string(),
                content: String::new(),
                links: Vec::new(),
            });
            i += 1;
            continue;
        }

        // Parse list items (links)
        if let Some(link_text) = line.strip_prefix("- ") {
            if let Some(section) = &mut current_section {
                if let Some(link) = parse_link(link_text) {
                    section.links.push(link);
                } else {
                    // Regular content
                    section.content.push_str(line);
                    section.content.push('\n');
                }
            }
            i += 1;
            continue;
        }

        // Other content
        if !line.is_empty() {
            if let Some(section) = &mut current_section {
                section.content.push_str(line);
                section.content.push('\n');
            }
        }

        i += 1;
    }

    // Save last section
    if let Some(section) = current_section {
        sections.push(section);
    }

    Ok(LlmsTxt {
        frontmatter,
        project_name,
        description,
        sections,
    })
}

/// Extract YAML frontmatter if present
fn extract_frontmatter(content: &str) -> Result<(Option<Frontmatter>, String)> {
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() || lines[0].trim() != "---" {
        return Ok((None, content.to_string()));
    }

    // Find closing ---
    let mut end_idx = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_idx = Some(i);
            break;
        }
    }

    let Some(end) = end_idx else {
        return Ok((None, content.to_string()));
    };

    // Parse frontmatter
    let fm_content = lines[1..end].join("\n");
    let frontmatter: Frontmatter =
        serde_yaml::from_str(&fm_content).with_context(|| "Failed to parse YAML frontmatter")?;

    // Body is everything after the closing ---
    let body = lines[end + 1..].join("\n");

    Ok((Some(frontmatter), body))
}

/// Parse a markdown link: [text](url) or [text](url): description
fn parse_link(text: &str) -> Option<Link> {
    // Match [text](url)
    let start = text.find('[')?;
    let middle = text.find("](")?;
    let end = text[middle..].find(')')?;

    let link_text = text[start + 1..middle].to_string();
    let url = text[middle + 2..middle + end].to_string();

    // Check for description after the link
    let rest = text[middle + end + 1..].trim();
    let description = if let Some(stripped) = rest.strip_prefix(':') {
        Some(stripped.trim().to_string())
    } else if !rest.is_empty() {
        Some(rest.to_string())
    } else {
        None
    };

    Some(Link {
        text: link_text,
        url,
        description,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() -> anyhow::Result<()> {
        let content = r#"# My Project

> A great project

## Getting Started

- [Introduction](./intro.md): Getting started guide

## Core Concepts

- [Concepts](./concepts.md)
"#;

        let llms_txt = parse_content(content)?;
        assert_eq!(llms_txt.project_name, "My Project");
        assert_eq!(llms_txt.description, Some("A great project".to_string()));
        assert_eq!(llms_txt.sections.len(), 2);
        assert_eq!(llms_txt.sections[0].title, "Getting Started");
        assert_eq!(llms_txt.sections[0].links.len(), 1);
        Ok(())
    }

    #[test]
    fn test_parse_with_frontmatter() -> anyhow::Result<()> {
        let content = r#"---
version: "1.0"
project: "Test Project"
documents: 42
---

# Test Project

> Description

## Getting Started
"#;

        let llms_txt = parse_content(content)?;
        assert!(llms_txt.frontmatter.is_some());
        let fm = llms_txt
            .frontmatter
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("frontmatter is None"))?;
        assert_eq!(fm.version, Some("1.0".to_string()));
        assert_eq!(fm.project, Some("Test Project".to_string()));
        assert_eq!(fm.documents, Some(42));
        Ok(())
    }

    #[test]
    fn test_parse_link() -> anyhow::Result<()> {
        let link = parse_link("[Title](./path.md): Description")
            .ok_or_else(|| anyhow::anyhow!("parse_link returned None"))?;
        assert_eq!(link.text, "Title");
        assert_eq!(link.url, "./path.md");
        assert_eq!(link.description, Some("Description".to_string()));
        Ok(())
    }

    #[test]
    fn test_get_section() -> anyhow::Result<()> {
        let content = r#"# Project

## Getting Started

Content here
"#;

        let llms_txt = parse_content(content)?;
        assert!(llms_txt.get_section("Getting Started").is_some());
        assert!(llms_txt.get_section("Missing").is_none());
        Ok(())
    }

    #[test]
    fn test_required_sections() -> anyhow::Result<()> {
        let content = r#"# Project

## Getting Started
## Core Concepts
## API Reference
"#;

        let llms_txt = parse_content(content)?;
        assert!(llms_txt.has_required_sections());
        Ok(())
    }
}
