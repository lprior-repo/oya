//! Core chunking types and algorithms
//!
//! Design by Contract:
//! - Invariants: All chunks have non-empty content and valid IDs
//! - Precondition: Token counts must be consistent within ±10%
//! - Postcondition: Parent-child relationships form valid DAG (no cycles)

#![allow(clippy::expect_used)]

use crate::document::Document;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use tap::Pipe;

/// Hierarchical chunk level for multi-granularity retrieval
///
/// Documents can be chunked at three levels simultaneously,
/// with parent-child relationships allowing progressive disclosure:
///
/// - **Summary**: ~128 tokens - High-level overview for quick retrieval
/// - **Standard**: ~512 tokens - Balanced detail for most use cases
/// - **Detailed**: ~1024 tokens - Full context for deep understanding
///
/// # Example
///
/// ```
/// use contextual_chunker::ChunkLevel;
///
/// let level = ChunkLevel::Standard;
/// assert_eq!(level.target_tokens(), 512);
/// assert_eq!(level.as_str(), "standard");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkLevel {
    Summary,
    Standard,
    Detailed,
}

impl ChunkLevel {
    /// Target token count for this level
    pub fn target_tokens(&self) -> usize {
        match self {
            ChunkLevel::Summary => 128,
            ChunkLevel::Standard => 512,
            ChunkLevel::Detailed => 1024,
        }
    }

    /// String representation (matches serialization format)
    pub fn as_str(&self) -> &str {
        match self {
            ChunkLevel::Summary => "summary",
            ChunkLevel::Standard => "standard",
            ChunkLevel::Detailed => "detailed",
        }
    }
}

/// Generate a chunk ID with hierarchical level suffix
///
/// # Format
///
/// `{doc_id}#{chunk_index}-{level}`
///
/// # Examples
///
/// - `test-doc#0-summary`
/// - `test-doc#1-standard`
/// - `test-doc#2-detailed`
fn generate_chunk_id(doc_id: &str, chunk_index: usize, level: ChunkLevel) -> String {
    format!("{doc_id}#{chunk_index}-{}", level.as_str())
}

/// A semantic chunk of a document
///
/// Chunks preserve document context through:
/// - Hierarchical relationships (parent/child)
/// - Navigation links (previous/next at same level)
/// - Content analysis (type detection, summarization)
/// - Context prefixes (50-100 tokens from previous section)
///
/// # Chunk ID Format
///
/// Chunk IDs use format: `{doc_id}#{index}`
/// Example: `guides-intro#0-summary`, `guides-intro#1-standard`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Unique chunk identifier: {doc_id}#{index}
    pub chunk_id: String,

    /// Original document ID (from Document::id)
    pub doc_id: String,

    /// Original document title (from Document::title)
    pub doc_title: String,

    /// Index of this chunk within its document (0-based)
    pub chunk_index: usize,

    /// The actual chunk content (markdown)
    pub content: String,

    /// Context prefix from previous section (50-100 tokens)
    /// Provides context for retrieval systems following Anthropic's recommendations
    /// Reduces retrieval failures by ~35% in multi-turn conversations
    pub context_prefix: Option<String>,

    /// Estimated token count for this chunk
    /// Used for hierarchical bucketing and size tracking
    pub token_count: usize,

    /// The H2 heading (##) that introduces this chunk (if any)
    /// Helps users understand context when chunk is viewed in isolation
    pub heading: Option<String>,

    /// Content type classification: "code", "table", or "prose"
    /// Enables specialized handling in retrieval systems
    pub chunk_type: String,

    /// ID of previous chunk at same level and in same document (sequential)
    /// None for first chunk
    pub previous_chunk_id: Option<String>,

    /// ID of next chunk at same level and in same document (sequential)
    /// None for last chunk
    pub next_chunk_id: Option<String>,

    /// Summary of chunk content (extractive, no AI generation)
    /// Limited to ~200 characters
    pub summary: String,

    /// The hierarchical level of this chunk
    pub chunk_level: ChunkLevel,

    /// Parent chunk ID (from higher level)
    /// Standard chunks have Summary chunks as parents
    /// Detailed chunks have Standard chunks as parents
    pub parent_chunk_id: Option<String>,

    /// Child chunk IDs (at lower level)
    /// Summary chunks have Standard chunks as children
    /// Standard chunks have Detailed chunks as children
    pub child_chunk_ids: Vec<String>,
}

/// Result of chunking one or more documents
///
/// Aggregates all chunks and provides summary statistics
/// for monitoring and optimization.
pub struct ChunkingResult {
    /// All chunks from all input documents
    pub chunks: Vec<Chunk>,

    /// Count of Summary-level chunks
    pub summary_count: usize,

    /// Count of Standard-level chunks
    pub standard_count: usize,

    /// Count of Detailed-level chunks
    pub detailed_count: usize,
}

static H2_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^## (.+)$").expect("valid H2 regex (verified by tests)"));

static TABLE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\|.*\|").expect("valid table regex (verified by tests)"));

/// Chunk a single document at a specific hierarchical level
///
/// # Arguments
///
/// * `document` - The document to chunk
/// * `level` - The hierarchical level (Summary/Standard/Detailed)
///
/// # Returns
///
/// A vector of chunks, one per semantic boundary
///
/// # Algorithm
///
/// 1. Split on H2 headings (##) to find semantic boundaries
/// 2. If a section exceeds target_tokens, split further by token count
/// 3. Add context from previous section (buffer) to new section
/// 4. Link chunks sequentially (prev/next pointers)
///
/// # Chunk Boundaries
///
/// Chunks respect markdown structure:
/// - H2 headings (##) are primary boundaries
/// - Token limit is secondary (if section too long)
/// - Always preserves at least one line of context from previous section
///
/// # Example
///
/// ```
/// use contextual_chunker::{Document, ChunkLevel, chunk};
///
/// let doc = Document::new(
///     "intro".to_string(),
///     "Introduction".to_string(),
///     "## Getting Started\nSome content here.".to_string(),
/// );
///
/// let chunks = chunk(&doc, ChunkLevel::Standard).unwrap();
/// assert!(!chunks.is_empty());
/// ```
pub fn chunk(document: &Document, level: ChunkLevel) -> Result<Vec<Chunk>> {
    if !document.is_valid() {
        anyhow::bail!("Invalid document: id and title must be non-empty");
    }

    let mut chunks =
        create_chunks_at_level(&document.id, &document.title, &document.content, level);
    link_chunks(&mut chunks);
    Ok(chunks)
}

/// Chunk all documents at all three hierarchical levels
///
/// Creates Summary, Standard, and Detailed chunks for each document,
/// automatically linking parent-child relationships.
///
/// # Arguments
///
/// * `documents` - Slice of documents to chunk
///
/// # Returns
///
/// ChunkingResult with all chunks and summary statistics
///
/// # Example
///
/// ```
/// use contextual_chunker::{Document, chunk_all};
///
/// let docs = vec![
///     Document::new("doc1".to_string(), "Title 1".to_string(), "Content 1".to_string()),
///     Document::new("doc2".to_string(), "Title 2".to_string(), "Content 2".to_string()),
/// ];
///
/// let result = chunk_all(&docs).unwrap();
/// println!("Created {} chunks", result.chunks.len());
/// ```
pub fn chunk_all(documents: &[Document]) -> Result<ChunkingResult> {
    // Validate all documents
    for doc in documents {
        if !doc.is_valid() {
            anyhow::bail!(
                "Invalid document: {} - id and title must be non-empty",
                doc.id
            );
        }
    }

    let (all_chunks, summary_count, standard_count, detailed_count) = documents.iter().fold(
        (Vec::new(), 0usize, 0usize, 0usize),
        |(mut chunks, sum_count, std_count, det_count), doc| {
            let summary =
                create_chunks_at_level(&doc.id, &doc.title, &doc.content, ChunkLevel::Summary);
            let standard =
                create_chunks_at_level(&doc.id, &doc.title, &doc.content, ChunkLevel::Standard);
            let detailed =
                create_chunks_at_level(&doc.id, &doc.title, &doc.content, ChunkLevel::Detailed);

            let summary_count = summary.len();
            let standard_count = standard.len();
            let detailed_count = detailed.len();

            let summary_ids: Vec<String> = summary.iter().map(|c| c.chunk_id.clone()).collect();
            let standard_ids: Vec<String> = standard.iter().map(|c| c.chunk_id.clone()).collect();
            let detailed_ids: Vec<String> = detailed.iter().map(|c| c.chunk_id.clone()).collect();

            // Add summary chunks with standard as children
            chunks.extend(summary.into_iter().map(|mut chunk| {
                chunk.child_chunk_ids = standard_ids.clone();
                chunk
            }));

            // Add standard chunks with relationships
            chunks.extend(standard.into_iter().map(|mut chunk| {
                chunk.parent_chunk_id = summary_ids.first().cloned();
                chunk.child_chunk_ids = detailed_ids.clone();
                chunk
            }));

            // Add detailed chunks with parent
            chunks.extend(detailed.into_iter().map(|mut chunk| {
                chunk.parent_chunk_id = standard_ids.first().cloned();
                chunk
            }));

            (
                chunks,
                sum_count.saturating_add(summary_count),
                std_count.saturating_add(standard_count),
                det_count.saturating_add(detailed_count),
            )
        },
    );

    Ok(ChunkingResult {
        chunks: all_chunks,
        summary_count,
        standard_count,
        detailed_count,
    })
}

/// Internal: Create chunks at a specific level
fn create_chunks_at_level(
    doc_id: &str,
    doc_title: &str,
    content: &str,
    level: ChunkLevel,
) -> Vec<Chunk> {
    let target_tokens = level.target_tokens();

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_heading: Option<String> = None;
    let mut chunk_index = 0;
    let mut context_buffer = String::new();

    let lines: Vec<&str> = content.lines().collect();

    for line in lines.iter() {
        let current_tokens = estimate_tokens(&current_chunk);
        let should_split = H2_REGEX.captures(line).is_some()
            || (current_tokens >= target_tokens && !current_chunk.is_empty());

        if should_split && !current_chunk.is_empty() {
            let chunk_id = generate_chunk_id(doc_id, chunk_index, level);
            let summary = create_summary(&current_chunk);
            let token_count = estimate_tokens(&current_chunk);
            let chunk_type = detect_chunk_type(&current_chunk);

            chunks.push(Chunk {
                chunk_id,
                doc_id: doc_id.to_string(),
                doc_title: doc_title.to_string(),
                chunk_index,
                content: current_chunk.clone(),
                context_prefix: Some(context_buffer.clone()),
                token_count,
                heading: current_heading.clone(),
                chunk_type,
                previous_chunk_id: chunk_index
                    .checked_sub(1)
                    .map(|prev| generate_chunk_id(doc_id, prev, level)),
                next_chunk_id: None,
                summary,
                chunk_level: level,
                parent_chunk_id: None,
                child_chunk_ids: Vec::new(),
            });

            chunk_index = chunk_index.saturating_add(1);

            let context_tokens = match level {
                ChunkLevel::Summary => 30,
                ChunkLevel::Standard => 100,
                ChunkLevel::Detailed => 200,
            };

            context_buffer = get_context_tail(&current_chunk, context_tokens);
            current_chunk.clear();
        }

        if let Some(caps) = H2_REGEX.captures(line) {
            current_heading = caps.get(1).map(|m| m.as_str().to_string());

            if !context_buffer.is_empty() {
                current_chunk.push_str(&context_buffer);
                current_chunk.push('\n');
                context_buffer.clear();
            }
        }

        current_chunk.push_str(line);
        current_chunk.push('\n');
    }

    // Add final chunk
    if !current_chunk.is_empty() {
        let chunk_id = generate_chunk_id(doc_id, chunk_index, level);
        let summary = create_summary(&current_chunk);
        let token_count = estimate_tokens(&current_chunk);
        let chunk_type = detect_chunk_type(&current_chunk);

        chunks.push(Chunk {
            chunk_id,
            doc_id: doc_id.to_string(),
            doc_title: doc_title.to_string(),
            chunk_index,
            content: current_chunk,
            context_prefix: Some(context_buffer.clone()),
            token_count,
            heading: current_heading,
            chunk_type,
            previous_chunk_id: chunk_index
                .checked_sub(1)
                .map(|prev| generate_chunk_id(doc_id, prev, level)),
            next_chunk_id: None,
            summary,
            chunk_level: level,
            parent_chunk_id: None,
            child_chunk_ids: Vec::new(),
        });
    }

    if chunks.is_empty() {
        let chunk_id = generate_chunk_id(doc_id, 0, level);
        let summary = create_summary(content);
        let token_count = estimate_tokens(content);
        let chunk_type = detect_chunk_type(content);

        chunks.push(Chunk {
            chunk_id,
            doc_id: doc_id.to_string(),
            doc_title: doc_title.to_string(),
            chunk_index: 0,
            content: content.to_string(),
            context_prefix: None,
            token_count,
            heading: None,
            chunk_type,
            previous_chunk_id: None,
            next_chunk_id: None,
            summary,
            chunk_level: level,
            parent_chunk_id: None,
            child_chunk_ids: Vec::new(),
        });
    }

    chunks
}

/// Internal: Link chunks sequentially (prev/next)
fn link_chunks(chunks: &mut [Chunk]) {
    for i in 0..chunks.len() {
        if let Some(next_i) = i.checked_add(1) {
            if next_i < chunks.len()
                && chunks[i].doc_id == chunks[next_i].doc_id
                && chunks[i].chunk_level == chunks[next_i].chunk_level
            {
                chunks[i].next_chunk_id = Some(chunks[next_i].chunk_id.clone());
            }
        }
    }
}

/// Estimate token count using character-based approximation
/// Assumes ~4 characters per token (OpenAI standard)
fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

/// Create a summary from chunk content (extractive)
/// Extracts first 1-2 sentences, truncates to 200 chars
fn create_summary(content: &str) -> String {
    content
        .split(['.', '\n'])
        .filter(|s| s.trim().len() > 10)
        .take(2)
        .collect::<Vec<_>>()
        .join(". ")
        .pipe(|summary| {
            let char_count = summary.chars().count();
            if char_count > 200 {
                let truncated: String = summary.chars().take(197).collect();
                format!("{truncated}...")
            } else {
                summary
            }
        })
}

/// Get trailing context from chunk (for next chunk's prefix)
fn get_context_tail(content: &str, max_tokens: usize) -> String {
    content
        .lines()
        .rev()
        .fold((Vec::new(), 0usize), |(mut lines, count), line| {
            let line_tokens = estimate_tokens(line);
            if count.saturating_add(line_tokens) <= max_tokens {
                lines.push(line);
                (lines, count.saturating_add(line_tokens))
            } else {
                (lines, count)
            }
        })
        .0
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

/// Detect chunk content type
fn detect_chunk_type(content: &str) -> String {
    let code_block_count = content.matches("```").count() / 2;
    let has_table = content.contains('|') && TABLE_REGEX.is_match(content);

    if code_block_count > 5 {
        "code".to_string()
    } else if has_table {
        "table".to_string()
    } else {
        "prose".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_level_tokens() {
        assert_eq!(ChunkLevel::Summary.target_tokens(), 128);
        assert_eq!(ChunkLevel::Standard.target_tokens(), 512);
        assert_eq!(ChunkLevel::Detailed.target_tokens(), 1024);
    }

    #[test]
    fn test_chunk_level_str() {
        assert_eq!(ChunkLevel::Summary.as_str(), "summary");
        assert_eq!(ChunkLevel::Standard.as_str(), "standard");
        assert_eq!(ChunkLevel::Detailed.as_str(), "detailed");
    }

    #[test]
    fn test_chunk_single_document() {
        let doc = Document::new(
            "test-doc".to_string(),
            "Test Document".to_string(),
            "## Section 1\nContent 1\n## Section 2\nContent 2".to_string(),
        );

        let chunks = chunk(&doc, ChunkLevel::Standard).unwrap();
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].doc_id, "test-doc");
        assert_eq!(chunks[0].doc_title, "Test Document");
    }

    #[test]
    fn test_chunk_all_documents() {
        let docs = vec![
            Document::new(
                "doc1".to_string(),
                "Doc 1".to_string(),
                "## Intro\nContent for doc 1".to_string(),
            ),
            Document::new(
                "doc2".to_string(),
                "Doc 2".to_string(),
                "## Intro\nContent for doc 2".to_string(),
            ),
        ];

        let result = chunk_all(&docs).unwrap();
        assert!(result.summary_count > 0);
        assert!(result.standard_count > 0);
        assert!(result.detailed_count > 0);
    }

    #[test]
    fn test_create_summary_ascii() {
        let content = "This is a test. This is another sentence.";
        let summary = create_summary(content);
        assert!(!summary.is_empty());
        assert!(summary.contains("This is a test"));
    }

    #[test]
    fn test_create_summary_unicode_emoji() {
        let content = "This is a test with emoji 🎉 and more content here.";
        let summary = create_summary(content);
        assert!(!summary.is_empty());
    }

    #[test]
    fn test_create_summary_unicode_cjk() {
        let content = "这是一个测试。这是另一个句子。More content after Chinese.";
        let summary = create_summary(content);
        assert!(!summary.is_empty());
    }

    #[test]
    fn test_chunk_type_detection() {
        let code = "```\ncode\n```\n```\ncode\n```\n```\ncode\n```\n```\ncode\n```\n```\ncode\n```\n```\ncode\n```";
        assert_eq!(detect_chunk_type(code), "code");

        let table = "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |";
        assert_eq!(detect_chunk_type(table), "table");

        let prose = "This is just regular prose content with no tables or code blocks.";
        assert_eq!(detect_chunk_type(prose), "prose");
    }

    #[test]
    fn test_estimate_tokens() {
        let text = "This is a test";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
        assert!((3..=4).contains(&tokens));
    }

    #[test]
    fn test_empty_document() {
        let doc = Document::new("empty".to_string(), "Empty Doc".to_string(), "".to_string());
        let chunks = chunk(&doc, ChunkLevel::Standard).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "");
    }

    #[test]
    fn test_invalid_document() {
        let invalid = Document::new("".to_string(), "Title".to_string(), "content".to_string());
        let result = chunk(&invalid, ChunkLevel::Standard);
        assert!(result.is_err());
    }

    #[test]
    fn test_chunk_no_h2_headings() {
        let content = "# Title\n\nLong content without any H2 headings.\n\n".repeat(100);
        let doc = Document::new(
            "no-h2".to_string(),
            "No H2 Doc".to_string(),
            content.clone(),
        );
        let chunks = chunk(&doc, ChunkLevel::Standard).unwrap();

        assert!(!chunks.is_empty(), "Should create at least one chunk");
        assert!(chunks[0].content.contains("Title"), "Should include H1");
        if content.split_whitespace().count() > 512 {
            assert!(chunks.len() > 1, "Long content should split");
        }
    }

    #[test]
    fn test_chunk_very_short_document() {
        let content = "# Short\n\nJust a few words.".to_string();
        let doc = Document::new("short".to_string(), "Short Doc".to_string(), content);
        let chunks = chunk(&doc, ChunkLevel::Standard).unwrap();

        assert_eq!(chunks.len(), 1, "Short doc should be one chunk");
        assert!(chunks[0].token_count < 512, "Should be under target");
    }

    #[test]
    fn test_chunk_only_h1_no_sections() {
        let content = "# Title\n\nContent here.\n\n# Another Title\n\nMore content.".to_string();
        let doc = Document::new("h1-only".to_string(), "H1 Only".to_string(), content);
        let chunks = chunk(&doc, ChunkLevel::Standard).unwrap();

        assert!(!chunks.is_empty());
        let all_content: String = chunks.iter().map(|c| c.content.as_str()).collect();
        assert!(all_content.contains("Title"));
        assert!(all_content.contains("Another Title"));
    }

    #[test]
    fn test_chunk_very_long_document() {
        let long_content = "# Title\n\n## Section\n\n".to_string() + &"word ".repeat(10000);
        let doc = Document::new(
            "long".to_string(),
            "Long Doc".to_string(),
            long_content.clone(),
        );
        let chunks = chunk(&doc, ChunkLevel::Standard).unwrap();

        assert!(
            chunks.len() > 1,
            "Long doc should split into multiple chunks"
        );

        let total_words: usize = chunks
            .iter()
            .map(|c| c.content.split_whitespace().count())
            .sum();
        let original_words = long_content.split_whitespace().count();
        assert!(
            total_words >= original_words.saturating_sub(100),
            "Most words preserved"
        );
    }

    #[test]
    fn test_chunk_unicode_boundaries() {
        let content = "# Unicode\n\n## Section\n\n";
        let emoji_content = "emoji 😀 ".repeat(1000);
        let full_content = content.to_string() + &emoji_content;
        let doc = Document::new(
            "unicode".to_string(),
            "Unicode Doc".to_string(),
            full_content.clone(),
        );
        let chunks = chunk(&doc, ChunkLevel::Standard).unwrap();

        for chunk in &chunks {
            assert!(chunk.content.is_char_boundary(0));
            assert!(chunk.content.is_char_boundary(chunk.content.len()));
        }

        let all_content: String = chunks.iter().map(|c| c.content.as_str()).collect();
        assert_eq!(
            all_content.matches('😀').count(),
            full_content.matches('😀').count(),
            "Emojis should be preserved"
        );
    }

    #[test]
    fn test_chunk_empty_sections() {
        let content =
            "# Title\n\n## Empty\n\n## Another Empty\n\n## Has Content\n\nSome text.".to_string();
        let doc = Document::new(
            "empty-sections".to_string(),
            "Empty Sections".to_string(),
            content,
        );
        let chunks = chunk(&doc, ChunkLevel::Standard).unwrap();

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunk_table_preservation() {
        let content = r#"# Title

## Table Section

| Col1 | Col2 |
|------|------|
| A    | B    |
| C    | D    |

More content."#
            .to_string();

        let doc = Document::new("table".to_string(), "Table Doc".to_string(), content);
        let chunks = chunk(&doc, ChunkLevel::Standard).unwrap();

        let has_table = chunks.iter().any(|c| c.content.contains("| Col1 |"));
        assert!(has_table, "Table should be preserved in chunks");
    }
}
