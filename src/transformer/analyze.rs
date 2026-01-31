use crate::config::CategoryConfig;
use crate::discover::DiscoveryFile;
use anyhow::Result;
use itertools::Itertools;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tap::Pipe;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    pub level: u32,
    pub text: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub text: String,
    pub target: String,
    pub is_internal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analysis {
    pub source_path: String,
    pub title: String,
    pub frontmatter: Option<HashMap<String, String>>,
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
    pub first_paragraph: String,
    pub word_count: usize,
    pub has_code: bool,
    pub has_tables: bool,
    pub category: String,
    pub content: String,
}

/// Analyze files using functional composition with filter_map
///
/// Returns an error if files were discovered but none could be analyzed.
/// This prevents silent failures where I/O errors or encoding issues
/// cause the pipeline to proceed with 0 documents.
pub fn analyze_files(
    files: &[DiscoveryFile],
    source_dir: &Path,
    category_config_path: Option<&Path>,
) -> Result<Vec<Analysis>> {
    // Load category config if provided
    let config = if let Some(path) = category_config_path {
        Some(CategoryConfig::load_from_file(path)?)
    } else {
        None
    };

    let input_count = files.len();

    let analyses: Vec<_> = files
        .iter()
        .filter_map(|file| {
            let file_path = source_dir.join(&file.source_path);
            analyze_single_file(&file.source_path, &file_path, config.as_ref())
                .map_err(|e| eprintln!("ANALYZE ERROR: {}: {}", file.source_path, e))
                .ok()
        })
        .collect();

    // If we had input files but produced no analyses, all files failed.
    // This is a critical error - we should not proceed with 0 documents.
    if input_count > 0 && analyses.is_empty() {
        anyhow::bail!(
            "Failed to analyze any of the {input_count} discovered file(s). \
            Check file permissions, encoding (files must be valid UTF-8), \
            and that files are not corrupted."
        );
    }

    Ok(analyses)
}

fn analyze_single_file(
    source_path: &str,
    file_path: &Path,
    category_config: Option<&CategoryConfig>,
) -> Result<Analysis> {
    let content = fs::read_to_string(file_path)?;

    let title = extract_title(&content, source_path);
    let (frontmatter, clean_content) = extract_frontmatter(&content);
    let headings = extract_headings(&clean_content);
    let links = extract_links(&clean_content);
    let first_paragraph = extract_first_paragraph(&clean_content);
    let word_count = clean_content.split_whitespace().count();
    let has_code = clean_content.contains("```");
    let has_tables = has_table(&clean_content);

    let category = if let Some(config) = category_config {
        let filename = Path::new(source_path)
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: no filename in {source_path}"))?
            .to_string_lossy();
        config.detect_category(&filename, &clean_content, source_path)
    } else {
        detect_category(source_path, &clean_content)
    };

    Ok(Analysis {
        source_path: source_path.to_string(),
        title,
        frontmatter,
        headings,
        links,
        first_paragraph,
        word_count,
        has_code,
        has_tables,
        category,
        content: clean_content,
    })
}

fn extract_title(content: &str, filename: &str) -> String {
    // (?m) enables multiline mode so ^ matches start of any line, not just start of string
    #[expect(clippy::expect_used)]
    let h1_regex = Regex::new(r"(?m)^# (.+)$").expect("hardcoded regex pattern is valid");
    if let Some(cap) = h1_regex.captures_iter(content).next() {
        if let Some(title_match) = cap.get(1) {
            return title_match.as_str().trim().to_string();
        }
    }

    // Use filename - fallback to "untitled" if no valid stem
    let stem = Path::new(filename)
        .file_stem()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| std::ffi::OsStr::new("untitled"))
        .to_string_lossy();
    let title = stem.replace(['-', '_'], " ").trim().to_string();

    title
        .split_whitespace()
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_frontmatter(content: &str) -> (Option<HashMap<String, String>>, String) {
    if !content.starts_with("---") {
        return (None, content.to_string());
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut end_idx = None;

    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.starts_with("---") {
            end_idx = Some(i);
            break;
        }
    }

    let end_idx = match end_idx {
        Some(idx) => idx,
        None => return (None, content.to_string()),
    };

    let mut fm = HashMap::new();
    for line in &lines[1..end_idx] {
        if let Some(pos) = line.find(':') {
            let key = line[..pos].trim().to_string();
            let val = line
                .get(pos.saturating_add(1)..)
                .unwrap_or("")
                .trim()
                .to_string();
            fm.insert(key, val);
        }
    }

    let remaining = lines
        .get(end_idx.saturating_add(1)..)
        .map(|slice| slice.join("\n"))
        .unwrap_or_default();
    (Some(fm), remaining)
}

/// Extract headings from content using functional composition
fn extract_headings(content: &str) -> Vec<Heading> {
    #[expect(clippy::expect_used)]
    let regex = Regex::new(r"^(#{1,6})\s+(.+)$").expect("hardcoded regex pattern is valid");

    content
        .lines()
        .enumerate()
        .filter_map(|(line_num, line)| {
            regex.captures(line).and_then(|cap| {
                // Safe extraction of level from capture group 1
                let level_match = cap.get(1)?;
                let text_match = cap.get(2)?;

                // Safe conversion: markdown headers are 1-6 hashes, so length always fits in u32
                // Using try_from for explicit overflow protection
                let level = u32::try_from(level_match.as_str().len()).unwrap_or(1);

                Some(Heading {
                    level,
                    text: text_match.as_str().trim().to_string(),
                    line: line_num,
                })
            })
        })
        .collect()
}

/// Extract links from content using functional composition
fn extract_links(content: &str) -> Vec<Link> {
    #[expect(clippy::expect_used)]
    let regex = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").expect("hardcoded regex pattern is valid");

    regex
        .captures_iter(content)
        .filter_map(|cap| {
            // Safe extraction of text from capture group 1
            let text_match = cap.get(1)?;
            let target_match = cap.get(2)?;

            let text = text_match.as_str().to_string();
            let target = target_match.as_str().to_string();
            let is_internal = !target.starts_with("http://")
                && !target.starts_with("https://")
                && !target.starts_with("mailto:");

            Some(Link {
                text,
                target,
                is_internal,
            })
        })
        .collect()
}

/// Extract first paragraph using functional composition with fold
fn extract_first_paragraph(content: &str) -> String {
    content
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter(|line| !line.starts_with('>') && !line.starts_with('|'))
        .fold(String::new(), |mut acc, line| {
            if acc.len() < 20 {
                acc.push_str(line);
                acc.push(' ');
            }
            acc
        })
        .trim()
        .pipe(|s| {
            let char_count = s.chars().count();
            if char_count > 200 {
                s.chars().take(200).collect()
            } else {
                s.to_string()
            }
        })
}

fn has_table(content: &str) -> bool {
    #[expect(clippy::expect_used)]
    let re = Regex::new(r"\|.*\|.*\|").expect("hardcoded regex pattern is valid");
    re.is_match(content)
}

fn detect_category(filename: &str, content: &str) -> String {
    let fname_lower = Path::new(filename)
        .file_stem()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| std::ffi::OsStr::new("untitled"))
        .to_string_lossy()
        .to_lowercase();

    let content_lower = content.to_lowercase();

    // Meta
    if matches!(
        fname_lower.as_str(),
        "readme" | "changelog" | "contributing" | "index" | "license"
    ) {
        return "meta".to_string();
    }

    // Tutorial
    if content_lower.contains("getting started")
        || content_lower.contains("step 1")
        || content_lower.contains("step 2")
        || content_lower.contains("## step")
        || {
            #[expect(clippy::expect_used)]
            let step_re = Regex::new(r"^\d+\.\s+").expect("hardcoded regex pattern is valid");
            step_re.is_match(content)
        }
    {
        return "tutorial".to_string();
    }

    // Ops
    if content_lower.contains("deploy")
        || content_lower.contains("install")
        || content_lower.contains("troubleshoot")
        || content_lower.contains("debug")
        || content_lower.contains("production")
        || content_lower.contains("monitoring")
        || content_lower.contains("error:")
    {
        return "ops".to_string();
    }

    // Ref
    if content_lower.contains("## api")
        || content_lower.contains("## reference")
        || content_lower.contains("## configuration")
        || content_lower.contains("parameters:")
        || content_lower.contains("returns:")
        || content_lower.contains("arguments:")
    {
        return "ref".to_string();
    }

    "concept".to_string()
}

/// Count categories using functional composition with counts
pub fn count_categories(analyses: &[Analysis]) -> HashMap<String, usize> {
    analyses.iter().map(|a| a.category.clone()).counts()
}
