//! Adapter layer between doc_transformer and contextual-chunker
//!
//! This module provides conversion functions between doc_transformer's types
//! and contextual-chunker's types, enabling clean separation of concerns while
//! maintaining all doc_transformer-specific functionality.

use crate::analyze::Analysis;
use crate::assign::IdMapping;
use anyhow::Result;
use contextual_chunker::{self, Document};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

/// Create directory with improved error context for permission issues
fn create_dir_with_context(path: &Path, context: &str) -> Result<()> {
    fs::create_dir_all(path).map_err(|e| {
        if e.kind() == io::ErrorKind::PermissionDenied {
            anyhow::anyhow!(
                "Permission denied: cannot create {} directory '{}'\n  \
                 Hint: Check directory permissions or run with appropriate access",
                context,
                path.display()
            )
        } else {
            anyhow::anyhow!(
                "Failed to create {} directory '{}': {}",
                context,
                path.display(),
                e
            )
        }
    })
}

/// Extended chunk type for doc_transformer with knowledge graph relationships
///
/// This extends contextual_chunker::Chunk with doc_transformer-specific fields
/// like `related_chunk_ids` for the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub chunk_id: String,
    pub doc_id: String,
    pub doc_title: String,
    pub chunk_index: usize,
    pub content: String,
    pub token_count: usize,
    pub heading: Option<String>,
    pub chunk_type: String,
    pub previous_chunk_id: Option<String>,
    pub next_chunk_id: Option<String>,
    pub related_chunk_ids: Vec<String>,
    pub summary: String,
    pub chunk_level: contextual_chunker::ChunkLevel,
    pub parent_chunk_id: Option<String>,
    pub child_chunk_ids: Vec<String>,
}

/// Extended chunking result for doc_transformer
#[derive(Debug)]
pub struct ChunksResult {
    pub total_chunks: usize,
    pub document_count: usize,
    pub chunks_metadata: Vec<Chunk>,
    pub summary_chunks: usize,
    pub standard_chunks: usize,
    pub detailed_chunks: usize,
}

/// Convert Analysis to contextual_chunker::Document
///
/// Maps doc_transformer's Analysis type to the simpler Document type
/// used by contextual-chunker. Uses link_map to get the assigned doc ID,
/// falling back to slugified source path.
#[allow(clippy::panic)]
fn analysis_to_document(analysis: &Analysis, link_map: &HashMap<String, IdMapping>) -> Document {
    let doc_id = link_map
        .get(&analysis.source_path)
        .map(|m| m.id.clone())
        .unwrap_or_else(|| panic!(
            "link_map missing entry for source_path '{}'. This should never happen - link_map is built from the same analyses vector.",
            analysis.source_path
        ));

    Document::new(doc_id, analysis.title.clone(), analysis.content.clone())
}

/// Convert contextual_chunker::Chunk to doc_transformer::Chunk
///
/// Creates extended chunk with empty related_chunk_ids (filled later by graph analysis)
fn convert_chunk(chunk: contextual_chunker::Chunk) -> Chunk {
    Chunk {
        chunk_id: chunk.chunk_id,
        doc_id: chunk.doc_id,
        doc_title: chunk.doc_title,
        chunk_index: chunk.chunk_index,
        content: chunk.content,
        token_count: chunk.token_count,
        heading: chunk.heading,
        chunk_type: chunk.chunk_type,
        previous_chunk_id: chunk.previous_chunk_id,
        next_chunk_id: chunk.next_chunk_id,
        related_chunk_ids: Vec::new(), // Populated later by knowledge graph
        summary: chunk.summary,
        chunk_level: chunk.chunk_level,
        parent_chunk_id: chunk.parent_chunk_id,
        child_chunk_ids: chunk.child_chunk_ids,
    }
}

/// Convert contextual_chunker::ChunkingResult to doc_transformer::ChunksResult
fn convert_chunking_result(
    result: contextual_chunker::ChunkingResult,
    document_count: usize,
) -> ChunksResult {
    let chunks_metadata = result.chunks.into_iter().map(convert_chunk).collect();

    ChunksResult {
        total_chunks: result
            .summary_count
            .saturating_add(result.standard_count)
            .saturating_add(result.detailed_count),
        document_count,
        chunks_metadata,
        summary_chunks: result.summary_count,
        standard_chunks: result.standard_count,
        detailed_chunks: result.detailed_count,
    }
}

/// Escape frontmatter values
fn escape_frontmatter(s: &str) -> String {
    s.replace('\n', " ").replace('\"', "\\\"")
}

/// Chunk all analyses using contextual-chunker
///
/// This is the main entry point for chunking in doc_transformer.
/// It converts Analysis types to Documents, calls contextual-chunker,
/// converts the results back to doc_transformer types, and writes
/// chunk files to disk.
pub fn chunk_all(
    analyses: &[Analysis],
    link_map: &HashMap<String, IdMapping>,
    output_dir: &Path,
) -> Result<ChunksResult> {
    // Create chunks directory
    let chunks_dir = output_dir.join("chunks");
    create_dir_with_context(&chunks_dir, "chunks")?;

    // Convert analyses to documents
    let documents: Vec<Document> = analyses
        .iter()
        .map(|a| analysis_to_document(a, link_map))
        .collect();

    // Call contextual-chunker
    let result = contextual_chunker::chunk_all(&documents)?;

    // Convert result back to doc_transformer types
    let chunks_result = convert_chunking_result(result, analyses.len());

    // Write chunks to disk
    chunks_result.chunks_metadata.iter().try_for_each(|chunk| {
        let level_suffix = match chunk.chunk_level {
            contextual_chunker::ChunkLevel::Summary => "summary",
            contextual_chunker::ChunkLevel::Standard => "standard",
            contextual_chunker::ChunkLevel::Detailed => "detailed",
        };

        let chunk_filename = format!(
            "{}-{}.md",
            chunk.chunk_id.replace(['/', '#'], "-"),
            level_suffix
        );
        let chunk_file = chunks_dir.join(&chunk_filename);

        let frontmatter = format!(
            "---\ndoc_id: {}\nchunk_id: {}\nchunk_level: {}\nchunk_type: {}\nheading: {}\ntoken_count: {}\nsummary: {}\n---\n",
            chunk.doc_id,
            chunk.chunk_id,
            level_suffix,
            chunk.chunk_type,
            chunk.heading.as_ref().unwrap_or(&"Introduction".to_string()),
            chunk.token_count,
            escape_frontmatter(&chunk.summary)
        );

        let content = format!("{}\n{}", frontmatter, chunk.content);
        fs::write(chunk_file, content)
    })?;

    Ok(chunks_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_to_document_with_link_map() -> anyhow::Result<()> {
        let analysis = Analysis {
            source_path: "concept/general/test.md".to_string(),
            title: "Test Document".to_string(),
            content: "## Section\nContent here".to_string(),
            frontmatter: None,
            headings: vec![],
            links: vec![],
            first_paragraph: "Content here".to_string(),
            word_count: 2,
            has_code: false,
            has_tables: false,
            category: "concept".to_string(),
        };

        let mut link_map = HashMap::new();
        link_map.insert(
            "concept/general/test.md".to_string(),
            IdMapping {
                id: "concept/general/test".to_string(),
                filename: "concept-general-test.md".to_string(),
                subcategory: "general".to_string(),
                slug: "test".to_string(),
            },
        );

        let doc = analysis_to_document(&analysis, &link_map);

        // Should use link_map entry, not slugified path
        assert_eq!(doc.id, "concept/general/test");
        assert_eq!(doc.title, "Test Document");
        assert_eq!(doc.content, "## Section\nContent here");

        Ok(())
    }

    #[test]
    fn test_analysis_to_document_missing_link_map_panics() {
        let analysis = Analysis {
            source_path: "concept/general/test.md".to_string(),
            title: "Test Document".to_string(),
            content: "## Section\nContent here".to_string(),
            frontmatter: None,
            headings: vec![],
            links: vec![],
            first_paragraph: "Content here".to_string(),
            word_count: 2,
            has_code: false,
            has_tables: false,
            category: "concept".to_string(),
        };

        let link_map = HashMap::new();

        // Should panic with helpful error message when link_map entry is missing
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            analysis_to_document(&analysis, &link_map);
        }));

        assert!(
            result.is_err(),
            "Should panic when link_map entry is missing"
        );
    }

    #[test]
    fn test_chunk_conversion() -> anyhow::Result<()> {
        let cc_chunk = contextual_chunker::Chunk {
            chunk_id: "test#0".to_string(),
            doc_id: "test".to_string(),
            doc_title: "Test".to_string(),
            chunk_index: 0,
            content: "Content".to_string(),
            context_prefix: Some("Context from previous".to_string()),
            token_count: 10,
            heading: Some("Section".to_string()),
            chunk_type: "prose".to_string(),
            previous_chunk_id: None,
            next_chunk_id: None,
            summary: "Summary".to_string(),
            chunk_level: contextual_chunker::ChunkLevel::Standard,
            parent_chunk_id: None,
            child_chunk_ids: vec![],
        };

        let chunk = convert_chunk(cc_chunk);

        assert_eq!(chunk.chunk_id, "test#0");
        assert_eq!(chunk.chunk_level, contextual_chunker::ChunkLevel::Standard);
        assert!(chunk.related_chunk_ids.is_empty()); // Populated later by graph

        Ok(())
    }
}
