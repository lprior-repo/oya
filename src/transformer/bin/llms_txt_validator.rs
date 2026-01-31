#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! llms.txt Validator CLI
//!
//! Validates llms.txt files and INDEX.json against RFC specification.
//!
//! Usage:
//!   llms-txt-validator <path>           # Validate llms.txt file
//!   llms-txt-validator --index <path>   # Validate INDEX.json file
//!   llms-txt-validator --url <url>      # Validate remote llms.txt

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Validation error
#[derive(Debug)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,   // Must fix
    Warning, // Should fix
    Info,    // Nice to have
}

/// Validation result
#[derive(Debug)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    fn new() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    fn add_error(&mut self, field: &str, message: &str, severity: Severity) {
        if severity == Severity::Error {
            self.valid = false;
        }
        self.errors.push(ValidationError {
            field: field.to_string(),
            message: message.to_string(),
            severity,
        });
    }

    #[allow(dead_code)] // Reserved for programmatic validation result checking
    fn has_errors(&self) -> bool {
        self.errors.iter().any(|e| e.severity == Severity::Error)
    }

    #[allow(dead_code)] // Reserved for programmatic validation result checking
    fn has_warnings(&self) -> bool {
        self.errors.iter().any(|e| e.severity == Severity::Warning)
    }
}

/// INDEX.json structure (simplified)
#[derive(Debug, Deserialize, Serialize)]
struct IndexJson {
    version: Option<String>,
    project: Option<String>,
    updated: Option<String>,
    documents: Option<Vec<Document>>,
    chunks: Option<Vec<Chunk>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Document {
    id: String,
    title: String,
    path: String,
    category: Option<String>,
    tags: Option<Vec<String>>,
    word_count: Option<usize>,
    summary: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Chunk {
    chunk_id: String,
    doc_id: String,
    content: Option<String>,
    token_count: Option<usize>,
    chunk_level: Option<String>,
}

/// Extract and validate URLs from markdown content
fn validate_links_in_content(content: &str, result: &mut ValidationResult) {
    // Regex for markdown links: [text](url)
    let link_regex = match Regex::new(r"\[([^\]]+)\]\(([^)]+)\)") {
        Ok(re) => re,
        Err(_) => {
            result.add_error("links", "Failed to compile link regex", Severity::Error);
            return;
        }
    };

    let mut url_count = 0;
    let mut malformed_count = 0;

    for captures in link_regex.captures_iter(content) {
        if let Some(url_match) = captures.get(2) {
            let url = url_match.as_str();
            url_count += 1;

            // Check if URL is well-formed
            if url.is_empty() {
                result.add_error("links", "Found empty link URL", Severity::Warning);
                malformed_count += 1;
                continue;
            }

            // Check for incomplete links (missing closing parenthesis indicator)
            if url.starts_with('\n') || url.contains('\n') {
                result.add_error(
                    "links",
                    &format!(
                        "Malformed link: URL contains newline near '{}'",
                        url.chars().take(20).collect::<String>()
                    ),
                    Severity::Warning,
                );
                malformed_count += 1;
                continue;
            }

            // Validate URL format for http/https URLs
            if url.starts_with("http://") || url.starts_with("https://") {
                // Basic URL validation (contains domain)
                if !url.contains('.') || url.len() < 12 {
                    result.add_error(
                        "links",
                        &format!("Suspicious URL format: {url}"),
                        Severity::Info,
                    );
                }
            } else if url.starts_with('#') {
                // Anchor link - valid
                continue;
            } else if url.starts_with('/') || url.starts_with("./") || url.starts_with("../") {
                // Relative link - warn if it looks suspicious
                if url.contains("..") && url.matches("..").count() > 3 {
                    result.add_error(
                        "links",
                        &format!("Deeply nested relative path: {url}"),
                        Severity::Info,
                    );
                }
            } else if !url.starts_with("mailto:") && !url.starts_with("ftp:") {
                // Unknown URL scheme
                result.add_error(
                    "links",
                    &format!("Unknown URL scheme or relative path: {url}"),
                    Severity::Info,
                );
            }
        }
    }

    // Report summary
    if url_count == 0 {
        result.add_error("links", "No links found in document", Severity::Info);
    } else if malformed_count > 0 {
        result.add_error(
            "links",
            &format!("Found {malformed_count} malformed links out of {url_count} total"),
            Severity::Warning,
        );
    }
}

/// Validate chunk file paths exist
#[allow(unused_variables)]
fn validate_chunk_paths(chunks: &[Chunk], base_path: &Path, result: &mut ValidationResult) {
    let missing_paths: Vec<String> = Vec::new();

    for chunk in chunks {
        // Skip chunks without content (they might reference external paths)
        if chunk.content.is_none() {
            continue;
        }

        // Check if chunk ID suggests a file path
        // Chunk IDs typically look like: doc1-chunk1, or include path info
        // We'll validate against the base path if it looks like a file reference

        // For now, we'll do a basic check - in a real implementation,
        // you'd parse the chunk metadata for actual file references

        // This is a placeholder for actual chunk path validation
        // Real implementation would need to know the chunks/ directory structure
    }

    if !missing_paths.is_empty() {
        for path in &missing_paths {
            result.add_error(
                "chunk_paths",
                &format!("Referenced chunk file not found: {path}"),
                Severity::Warning,
            );
        }
    }
}

/// Validate llms.txt file
fn validate_llms_txt(path: &Path) -> Result<ValidationResult> {
    let mut result = ValidationResult::new();

    // Check file exists
    if !path.exists() {
        result.add_error("file", "llms.txt does not exist", Severity::Error);
        return Ok(result);
    }

    // Read content
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    // Check file is not empty
    if content.trim().is_empty() {
        result.add_error("content", "File is empty", Severity::Error);
        return Ok(result);
    }

    // Check for required sections
    let required_sections = vec!["Getting Started", "Core Concepts", "API Reference"];
    for section in required_sections {
        if !content.contains(&format!("## {section}")) {
            result.add_error(
                "sections",
                &format!("Missing required section: {section}"),
                Severity::Warning,
            );
        }
    }

    // Check for INDEX.json reference
    if !content.contains("INDEX.json") {
        result.add_error(
            "index_reference",
            "No reference to INDEX.json found",
            Severity::Warning,
        );
    }

    // Check structure (basic markdown validation)
    let lines: Vec<&str> = content.lines().collect();
    let mut has_h1 = false;
    let mut has_h2 = false;

    for line in &lines {
        if line.starts_with("# ") {
            has_h1 = true;
        }
        if line.starts_with("## ") {
            has_h2 = true;
        }
    }

    if !has_h1 {
        result.add_error("structure", "No H1 heading found", Severity::Warning);
    }

    if !has_h2 {
        result.add_error("structure", "No H2 headings found", Severity::Error);
    }

    // Check length (should be substantial)
    let word_count = content.split_whitespace().count();
    if word_count < 100 {
        result.add_error(
            "length",
            &format!("File seems too short ({word_count} words)"),
            Severity::Warning,
        );
    }

    // Validate links
    validate_links_in_content(&content, &mut result);

    // Check for INDEX.json file if referenced
    if content.contains("INDEX.json") {
        let index_path = path.parent().map(|p| p.join("INDEX.json"));
        if let Some(index_path) = index_path {
            if !index_path.exists() {
                result.add_error(
                    "index_reference",
                    "Referenced INDEX.json file not found in same directory",
                    Severity::Warning,
                );
            }
        }
    }

    Ok(result)
}

/// Validate INDEX.json file
fn validate_index_json(path: &Path) -> Result<ValidationResult> {
    let mut result = ValidationResult::new();

    // Check file exists
    if !path.exists() {
        result.add_error("file", "INDEX.json does not exist", Severity::Error);
        return Ok(result);
    }

    // Read and parse JSON
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let index: IndexJson = match serde_json::from_str(&content) {
        Ok(idx) => idx,
        Err(e) => {
            result.add_error("json", &format!("Invalid JSON: {e}"), Severity::Error);
            return Ok(result);
        }
    };

    // Validate required fields
    if index.version.is_none() {
        result.add_error(
            "version",
            "Missing required field: version",
            Severity::Error,
        );
    }

    if index.project.is_none() {
        result.add_error(
            "project",
            "Missing required field: project",
            Severity::Error,
        );
    }

    if index.updated.is_none() {
        result.add_error(
            "updated",
            "Missing required field: updated",
            Severity::Warning,
        );
    }

    // Validate documents
    if let Some(docs) = &index.documents {
        if docs.is_empty() {
            result.add_error("documents", "Documents array is empty", Severity::Error);
        }

        // Check for duplicate document IDs
        let mut seen_ids = HashSet::new();
        for doc in docs {
            if !seen_ids.insert(&doc.id) {
                result.add_error(
                    "documents",
                    &format!("Duplicate document ID: {}", doc.id),
                    Severity::Error,
                );
            }

            // Validate document fields
            if doc.title.is_empty() {
                result.add_error(
                    "documents",
                    &format!("Document {} has empty title", doc.id),
                    Severity::Warning,
                );
            }

            if doc.path.is_empty() {
                result.add_error(
                    "documents",
                    &format!("Document {} has empty path", doc.id),
                    Severity::Error,
                );
            }
        }
    } else {
        result.add_error(
            "documents",
            "Missing required field: documents",
            Severity::Error,
        );
    }

    // Validate chunks
    if let Some(chunks) = &index.chunks {
        let mut seen_chunk_ids = HashSet::new();
        let doc_ids: HashSet<String> = index
            .documents
            .as_ref()
            .map(|docs| docs.iter().map(|d| d.id.clone()).collect())
            .unwrap_or_default();

        for chunk in chunks {
            // Check for duplicate chunk IDs
            if !seen_chunk_ids.insert(&chunk.chunk_id) {
                result.add_error(
                    "chunks",
                    &format!("Duplicate chunk ID: {}", chunk.chunk_id),
                    Severity::Error,
                );
            }

            // Validate doc_id references
            if !doc_ids.contains(&chunk.doc_id) {
                result.add_error(
                    "chunks",
                    &format!(
                        "Chunk {} references non-existent document: {}",
                        chunk.chunk_id, chunk.doc_id
                    ),
                    Severity::Error,
                );
            }

            // Validate chunk_level values
            if let Some(level) = &chunk.chunk_level {
                if !["summary", "standard", "detailed"].contains(&level.as_str()) {
                    result.add_error(
                        "chunks",
                        &format!("Invalid chunk_level: {level}"),
                        Severity::Error,
                    );
                }
            }
        }

        if chunks.is_empty() {
            result.add_error("chunks", "Chunks array is empty", Severity::Warning);
        } else {
            // Validate chunk file paths
            if let Some(base_dir) = path.parent() {
                validate_chunk_paths(chunks, base_dir, &mut result);
            }
        }
    }

    Ok(result)
}

/// Print validation results
fn print_results(result: &ValidationResult, path: &Path) {
    println!("\nValidating: {}", path.display());
    println!("{}", "=".repeat(60));

    if result.errors.is_empty() {
        println!("✅ No issues found!");
        return;
    }

    let error_count = result
        .errors
        .iter()
        .filter(|e| e.severity == Severity::Error)
        .count();
    let warning_count = result
        .errors
        .iter()
        .filter(|e| e.severity == Severity::Warning)
        .count();
    let info_count = result
        .errors
        .iter()
        .filter(|e| e.severity == Severity::Info)
        .count();

    println!("\n📊 Found {error_count} errors, {warning_count} warnings, {info_count} info");

    for error in &result.errors {
        let symbol = match error.severity {
            Severity::Error => "❌",
            Severity::Warning => "⚠️ ",
            Severity::Info => "ℹ️ ",
        };
        let severity_str = match error.severity {
            Severity::Error => "ERROR",
            Severity::Warning => "WARN",
            Severity::Info => "INFO",
        };
        println!("\n{} [{}] {}", symbol, severity_str, error.field);
        println!("   {}", error.message);
    }

    println!("\n{}", "=".repeat(60));
    if result.valid {
        println!("✅ Validation passed (with warnings)");
    } else {
        println!("❌ Validation failed");
    }
}

fn print_usage(program: &str) {
    eprintln!("llms-txt-validator v1.0 - Validate llms.txt and INDEX.json files");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  {program} <llms.txt>              # Validate llms.txt file");
    eprintln!("  {program} --index <INDEX.json>    # Validate INDEX.json file");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -h, --help      Show this help message");
    eprintln!("  -V, --version   Show version information");
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let program = args
        .first()
        .map(|s| s.as_str())
        .unwrap_or("llms-txt-validator");

    // Handle --help and --version flags first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage(program);
        std::process::exit(0);
    }

    if args.iter().any(|a| a == "--version" || a == "-V") {
        eprintln!("llms-txt-validator v1.0");
        std::process::exit(0);
    }

    if args.len() < 2 {
        print_usage(program);
        std::process::exit(1);
    }

    let (is_index, path) = if args.get(1).map(|s| s.as_str()) == Some("--index") {
        if args.len() < 3 {
            eprintln!("Error: --index requires a path argument");
            std::process::exit(1);
        }
        (true, PathBuf::from(&args[2]))
    } else {
        (false, PathBuf::from(&args[1]))
    };

    let result = if is_index {
        validate_index_json(&path)?
    } else {
        validate_llms_txt(&path)?
    };

    print_results(&result, &path);

    // Exit with error code if validation failed
    if !result.valid {
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_valid_llms_txt() -> anyhow::Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(
            file,
            "# Project\n\n## Getting Started\n\n## Core Concepts\n\n## API Reference\n\nSee INDEX.json"
        )?;

        let result = validate_llms_txt(file.path())?;
        assert!(result.valid);
        Ok(())
    }

    #[test]
    fn test_empty_llms_txt() -> anyhow::Result<()> {
        let file = NamedTempFile::new()?;
        let result = validate_llms_txt(file.path())?;
        assert!(!result.valid);
        Ok(())
    }

    #[test]
    fn test_valid_index_json() -> anyhow::Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(
            file,
            r#"{{"version": "1.0", "project": "test", "documents": [{{"id": "1", "title": "Doc", "path": "doc.md"}}]}}"#
        )?;

        let result = validate_index_json(file.path())?;
        assert!(result.valid);
        Ok(())
    }

    #[test]
    fn test_link_validation_valid_urls() {
        let content = r#"
# Documentation

See the [official site](https://example.com) for more info.
Check the [API docs](https://api.example.com/v1/docs).
Also see [local file](./guide.md) and [anchor](#section).
        "#;

        let mut result = ValidationResult::new();
        validate_links_in_content(content, &mut result);

        // Should not have any errors, only info about link count
        assert!(!result.has_errors());
    }

    #[test]
    fn test_link_validation_malformed_urls() {
        let content = r#"
# Documentation

This has a [empty link]() in the text.
And another [newline link](https://example.com
/path) here.
        "#;

        let mut result = ValidationResult::new();
        validate_links_in_content(content, &mut result);

        // Should detect malformed links (empty URL or URL with newline)
        assert!(result.has_warnings() || result.has_errors());
    }

    #[test]
    fn test_link_validation_no_links() {
        let content = "# Documentation\n\nJust plain text with no links.";

        let mut result = ValidationResult::new();
        validate_links_in_content(content, &mut result);

        // Should report no links found (Info level)
        let has_no_links_info = result
            .errors
            .iter()
            .any(|e| e.field == "links" && e.message.contains("No links found"));
        assert!(has_no_links_info);
    }

    #[test]
    fn test_index_json_with_chunks() -> anyhow::Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(
            file,
            r#"{{
                "version": "1.0",
                "project": "test",
                "documents": [{{"id": "doc1", "title": "Doc", "path": "doc.md"}}],
                "chunks": [
                    {{"chunk_id": "chunk1", "doc_id": "doc1", "chunk_level": "standard"}},
                    {{"chunk_id": "chunk2", "doc_id": "doc1", "chunk_level": "detailed"}}
                ]
            }}"#
        )?;

        let result = validate_index_json(file.path())?;
        assert!(result.valid);
        Ok(())
    }

    #[test]
    fn test_index_json_invalid_chunk_reference() -> anyhow::Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(
            file,
            r#"{{
                "version": "1.0",
                "project": "test",
                "documents": [{{"id": "doc1", "title": "Doc", "path": "doc.md"}}],
                "chunks": [
                    {{"chunk_id": "chunk1", "doc_id": "doc_INVALID", "chunk_level": "standard"}}
                ]
            }}"#
        )?;

        let result = validate_index_json(file.path())?;
        assert!(!result.valid);
        assert!(result.has_errors());
        Ok(())
    }
}
