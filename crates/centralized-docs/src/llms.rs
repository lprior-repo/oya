//! llms.txt generation module
//!
//! Generates llms.txt and llms-full.txt files following the llms.txt specification.
//! These files provide AI-friendly entry points into the documentation.
//!
//! Specification: https://llmstxt.org/

use crate::analyze::Analysis;
use crate::assign::IdMapping;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Configuration for llms.txt generation
#[derive(Debug, Clone)]
pub struct LlmsConfig {
    /// Project name (H1 in llms.txt)
    pub project_name: String,
    /// Brief project description (blockquote)
    pub project_description: String,
    /// Maximum documents per category in llms.txt (default: 5)
    pub max_per_category: usize,
    /// Include full content in llms-full.txt (default: true)
    pub generate_full: bool,
    /// llms.txt specification version (default: "1.0")
    pub spec_version: String,
    /// Project version (default: "0.1.0")
    pub project_version: String,
    /// Enable YAML frontmatter with metadata (default: true)
    pub include_frontmatter: bool,
}

impl Default for LlmsConfig {
    fn default() -> Self {
        Self {
            project_name: "Documentation".to_string(),
            project_description: "AI-optimized documentation index".to_string(),
            max_per_category: 5,
            generate_full: true,
            spec_version: "1.0".to_string(),
            project_version: "0.1.0".to_string(),
            include_frontmatter: true,
        }
    }
}

/// Generate llms.txt - curated overview for AI consumption
///
/// Format follows the llms.txt specification:
/// - H1: Project name
/// - Blockquote: Brief description
/// - H2 sections: Categorized document links
/// - Optional section: Secondary content
pub fn generate_llms_txt(
    analyses: &[Analysis],
    link_map: &HashMap<String, IdMapping>,
    config: &LlmsConfig,
    output_dir: &Path,
) -> Result<()> {
    let mut content = String::new();

    // Optional YAML frontmatter with metadata
    if config.include_frontmatter {
        content.push_str("---\n");
        content.push_str(&format!("version: \"{}\"\n", config.spec_version));
        content.push_str(&format!("project: \"{}\"\n", config.project_name));
        content.push_str(&format!(
            "project_version: \"{}\"\n",
            config.project_version
        ));
        content.push_str(&format!(
            "updated: \"{}\"\n",
            chrono::Utc::now().format("%Y-%m-%d")
        ));
        content.push_str(&format!("documents: {}\n", analyses.len()));
        content.push_str("index: \"./INDEX.json\"\n");
        content.push_str("---\n\n");
    }

    // H1: Project name (required)
    content.push_str(&format!("# {}\n\n", config.project_name));

    // Blockquote: Description
    content.push_str(&format!("> {}\n\n", config.project_description));

    // Key context for AI
    content.push_str("Key context for AI:\n");
    content.push_str(&format!("- Total documents: {}\n", analyses.len()));
    content.push_str("- Format: Markdown with YAML frontmatter\n");
    content.push_str("- Chunking: Semantic chunks with context prefix (~170 tokens)\n");
    content.push_str("- Navigation: Knowledge DAG with Jaccard similarity\n\n");

    // Group by category
    let mut by_category: HashMap<&str, Vec<(&Analysis, &IdMapping)>> = HashMap::new();
    for analysis in analyses {
        if let Some(mapping) = link_map.get(&analysis.source_path) {
            by_category
                .entry(&analysis.category)
                .or_default()
                .push((analysis, mapping));
        }
    }

    // Smart section detection: only include sections with documents
    let has_tutorials = by_category.get("tutorial").is_some_and(|v| !v.is_empty());
    let has_concepts = by_category.get("concept").is_some_and(|v| !v.is_empty());
    let has_refs = by_category.get("ref").is_some_and(|v| !v.is_empty());
    let has_ops = by_category.get("ops").is_some_and(|v| !v.is_empty());
    let has_meta = by_category.get("meta").is_some_and(|v| !v.is_empty());

    // Getting Started (tutorials) - only if documents exist
    if has_tutorials {
        if let Some(tutorials) = by_category.get("tutorial") {
            content.push_str("## Getting Started\n\n");
            for (analysis, mapping) in tutorials.iter().take(config.max_per_category) {
                let desc = truncate_summary(&analysis.first_paragraph, 60);
                content.push_str(&format!(
                    "- [{}](./docs/{}): {}\n",
                    analysis.title, mapping.filename, desc
                ));
            }
            content.push('\n');
        }
    }

    // Core Concepts - only if documents exist
    if has_concepts {
        if let Some(concepts) = by_category.get("concept") {
            content.push_str("## Core Concepts\n\n");
            for (analysis, mapping) in concepts.iter().take(config.max_per_category) {
                let desc = truncate_summary(&analysis.first_paragraph, 60);
                content.push_str(&format!(
                    "- [{}](./docs/{}): {}\n",
                    analysis.title, mapping.filename, desc
                ));
            }
            content.push('\n');
        }
    }

    // API Reference - only if documents exist
    if has_refs {
        if let Some(refs) = by_category.get("ref") {
            content.push_str("## API Reference\n\n");
            for (analysis, mapping) in refs.iter().take(config.max_per_category) {
                let desc = truncate_summary(&analysis.first_paragraph, 60);
                content.push_str(&format!(
                    "- [{}](./docs/{}): {}\n",
                    analysis.title, mapping.filename, desc
                ));
            }
            content.push('\n');
        }
    }

    // Operations - only if documents exist
    if has_ops {
        if let Some(ops) = by_category.get("ops") {
            content.push_str("## Operations\n\n");
            for (analysis, mapping) in ops.iter().take(config.max_per_category) {
                let desc = truncate_summary(&analysis.first_paragraph, 60);
                content.push_str(&format!(
                    "- [{}](./docs/{}): {}\n",
                    analysis.title, mapping.filename, desc
                ));
            }
            content.push('\n');
        }
    }

    // Optional section (meta) - only if documents exist
    if has_meta {
        if let Some(meta) = by_category.get("meta") {
            content.push_str("## Optional\n\n");
            for (analysis, mapping) in meta.iter().take(config.max_per_category) {
                content.push_str(&format!(
                    "- [{}](./docs/{})\n",
                    analysis.title, mapping.filename
                ));
            }
            content.push('\n');
        }
    }

    // Machine-readable index reference
    content.push_str("## Machine-Readable Index\n\n");
    content.push_str("- [INDEX.json](./INDEX.json): Complete searchable index with keywords, chunks, and knowledge graph\n");
    content.push_str("- [COMPASS.md](./COMPASS.md): Human-readable navigation guide\n");

    fs::write(output_dir.join("llms.txt"), content)?;

    Ok(())
}

/// Generate llms-full.txt - all documentation content concatenated
///
/// This file contains all document content for models with large context windows.
/// Each document is separated by a header with metadata.
pub fn generate_llms_full_txt(
    analyses: &[Analysis],
    link_map: &HashMap<String, IdMapping>,
    output_dir: &Path,
) -> Result<()> {
    let docs_dir = output_dir.join("docs");
    let mut content = String::new();

    content.push_str("# Full Documentation\n\n");
    content.push_str(&format!(
        "> This file contains all {} documents concatenated for large context models.\n\n",
        analyses.len()
    ));
    content.push_str("---\n\n");

    // Sort by category then title for consistent ordering
    let mut sorted: Vec<_> = analyses
        .iter()
        .filter_map(|a| link_map.get(&a.source_path).map(|m| (a, m)))
        .collect();
    sorted.sort_by(|a, b| {
        a.0.category
            .cmp(&b.0.category)
            .then_with(|| a.0.title.cmp(&b.0.title))
    });

    for (analysis, mapping) in sorted {
        // Document header
        content.push_str(&format!(
            "## {} [{}]\n\n",
            analysis.title, analysis.category
        ));
        content.push_str(&format!("**Path**: docs/{}\n", mapping.filename));
        content.push_str(&format!("**ID**: {}\n\n", mapping.id));

        // Read and include document content
        let doc_path = docs_dir.join(&mapping.filename);
        if let Ok(doc_content) = fs::read_to_string(&doc_path) {
            // Skip frontmatter if present
            let body = skip_frontmatter(&doc_content);
            content.push_str(body);
            content.push_str("\n\n");
        } else {
            // Fall back to summary if file not found
            content.push_str(&analysis.first_paragraph);
            content.push_str("\n\n");
        }

        content.push_str("---\n\n");
    }

    fs::write(output_dir.join("llms-full.txt"), content)?;

    Ok(())
}

/// Truncate summary to fit in a description
fn truncate_summary(text: &str, max_len: usize) -> String {
    let cleaned = text.replace('\n', " ").trim().to_string();
    let char_count = cleaned.chars().count();
    if char_count <= max_len {
        cleaned
    } else {
        let truncated: String = cleaned.chars().take(max_len.saturating_sub(3)).collect();
        format!("{truncated}...")
    }
}

/// Skip YAML frontmatter from document content using functional pattern
fn skip_frontmatter(content: &str) -> &str {
    content
        .strip_prefix("---")
        .and_then(|stripped| {
            stripped
                .find("---")
                .map(|end| stripped[end.saturating_add(3)..].trim_start())
        })
        .unwrap_or(content)
}

/// Generate AGENTS.md - coding instructions for AI assistants
///
/// This file provides project-specific instructions that AI coding assistants
/// should follow when working with this codebase. Adopted by OpenAI Codex,
/// Google Jules, and Cursor.
pub fn generate_agents_md(
    analyses: &[Analysis],
    _link_map: &HashMap<String, IdMapping>,
    config: &LlmsConfig,
    output_dir: &Path,
) -> Result<()> {
    let mut content = String::new();

    // Header
    content.push_str(&format!(
        "# {} - Agent Instructions\n\n",
        config.project_name
    ));
    content.push_str(&format!("> {}\n\n", config.project_description));

    // Project overview
    content.push_str("## Project Overview\n\n");
    content.push_str(&format!(
        "This documentation index contains {} documents organized by category.\n\n",
        analyses.len()
    ));

    // Count categories
    let mut categories: HashMap<&str, usize> = HashMap::new();
    for analysis in analyses {
        let count = categories.entry(&analysis.category).or_insert(0);
        *count = count.saturating_add(1);
    }

    content.push_str("### Document Categories\n\n");
    for (cat, count) in &categories {
        content.push_str(&format!("- **{cat}**: {count} documents\n"));
    }
    content.push('\n');

    // Navigation instructions
    content.push_str("## Navigation Guide\n\n");
    content.push_str("When working with this documentation:\n\n");
    content.push_str("1. **Start with llms.txt** - Read this first to understand the structure\n");
    content.push_str("2. **Use INDEX.json** - For programmatic lookup of documents and chunks\n");
    content.push_str("3. **Follow the DAG** - Use knowledge graph edges to find related content\n");
    content.push_str(
        "4. **Chunk navigation** - Each chunk has `previous_chunk_id` and `next_chunk_id`\n\n",
    );

    // File structure
    content.push_str("## File Structure\n\n");
    content.push_str("```\n");
    content.push_str("./\n");
    content.push_str("├── llms.txt           # AI entry point (read first)\n");
    content.push_str("├── llms-full.txt      # Full content for large context models\n");
    content.push_str("├── AGENTS.md          # This file - coding instructions\n");
    content.push_str("├── INDEX.json         # Machine-readable index + knowledge graph\n");
    content.push_str("├── COMPASS.md         # Human-readable navigation\n");
    content.push_str("├── docs/              # Transformed documents with frontmatter\n");
    content.push_str("└── chunks/            # Semantic chunks with context prefix\n");
    content.push_str("```\n\n");

    // Chunk format
    content.push_str("## Chunk Format\n\n");
    content.push_str("Each chunk file contains:\n");
    content.push_str(
        "- YAML frontmatter with `chunk_id`, `doc_id`, `token_count`, navigation pointers\n",
    );
    content.push_str("- Context prefix from previous chunk (~50-100 tokens)\n");
    content.push_str("- Main content (~170 tokens average)\n\n");

    // Index structure
    content.push_str("## INDEX.json Structure\n\n");
    content.push_str("```json\n");
    content.push_str("{\n");
    content.push_str("  \"documents\": [...],    // Document metadata\n");
    content.push_str("  \"chunks\": [...],       // Chunk metadata with navigation\n");
    content.push_str("  \"keywords\": {...},     // Term → doc_id lookup\n");
    content.push_str("  \"graph\": {             // Knowledge DAG\n");
    content.push_str("    \"nodes\": [...],      // Documents and chunks\n");
    content.push_str("    \"edges\": [...]       // Relationships (Parent, Sequential, Related)\n");
    content.push_str("  }\n");
    content.push_str("}\n");
    content.push_str("```\n\n");

    // Best practices
    content.push_str("## Best Practices\n\n");
    content.push_str("- **Don't guess**: Use INDEX.json to find exact document/chunk IDs\n");
    content.push_str(
        "- **Read context**: When reading a chunk, consider reading previous/next chunks\n",
    );
    content.push_str("- **Follow relationships**: Use graph edges to find related content\n");
    content.push_str(
        "- **Check frontmatter**: Every document has `category`, `tags`, and `summary`\n",
    );

    fs::write(output_dir.join("AGENTS.md"), content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_summary() {
        assert_eq!(truncate_summary("Short text", 20), "Short text");
        assert_eq!(
            truncate_summary(
                "This is a much longer piece of text that needs truncation",
                20
            ),
            "This is a much lo..."
        );
    }

    #[test]
    fn test_skip_frontmatter() {
        let with_fm = "---\ntitle: Test\n---\n\nContent here";
        assert_eq!(skip_frontmatter(with_fm), "Content here");

        let without_fm = "Just content";
        assert_eq!(skip_frontmatter(without_fm), "Just content");
    }
}
