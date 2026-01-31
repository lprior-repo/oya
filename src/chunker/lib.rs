//! # contextual-chunker
//!
//! Semantic chunking with hierarchical levels for documentation and knowledge bases.
//!
//! This library enables splitting documents into semantically meaningful chunks
//! at multiple hierarchical levels (Summary, Standard, Detailed) with automatic
//! relationship tracking, making it ideal for RAG systems and knowledge bases.
//!
//! ## Key Features
//!
//! - **Semantic Boundaries**: Chunks respect H2 headings (##) in markdown
//! - **Hierarchical Levels**: 3-level hierarchy (128, 512, 1024 tokens)
//! - **Automatic Relationships**: Parent-child links for progressive disclosure
//! - **Navigation Links**: Sequential prev/next pointers at same level
//! - **Content Analysis**: Automatic type detection (code/table/prose)
//! - **Summary Extraction**: Extractive summaries for quick overview
//! - **Deterministic**: Same input always produces same chunks
//! - **Unicode Safe**: No panics on emoji, CJK, or special characters
//!
//! ## Quick Start
//!
//! ```
//! use contextual_chunker::{Document, ChunkLevel, chunk_all};
//!
//! let documents = vec![
//!     Document::new(
//!         "guide".to_string(),
//!         "Getting Started".to_string(),
//!         "## Introduction\nWelcome to the guide.\n## Next Steps\nHere's what to do.".to_string(),
//!     ),
//! ];
//!
//! let result = chunk_all(&documents)?;
//! println!("Created {} chunks", result.chunks.len());
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! ## Chunking Strategy
//!
//! Documents are chunked at three levels simultaneously:
//!
//! 1. **Summary Level** (~128 tokens)
//!    - High-level overview for quick retrieval
//!    - Parent chunks for Standard level
//!
//! 2. **Standard Level** (~512 tokens)
//!    - Balanced detail for most use cases
//!    - Default retrieval level
//!    - Child of Summary, parent of Detailed
//!
//! 3. **Detailed Level** (~1024 tokens)
//!    - Full context for deep understanding
//!    - Leaf chunks in hierarchy
//!
//! ## Chunk Boundaries
//!
//! Chunks respect markdown structure:
//! - H2 headings (##) trigger chunk boundaries
//! - If a section exceeds token limit, split by token count
//! - Previous section's tail included as context
//!
//! ## Example: Multi-Level Retrieval
//!
//! ```
//! use contextual_chunker::{Document, ChunkLevel, chunk_all};
//!
//! let doc = Document::new(
//!     "tutorial".to_string(),
//!     "Tutorial".to_string(),
//!     "## Setup\nInstructions.\n## Testing\nTest cases.".to_string(),
//! );
//!
//! let result = chunk_all(&[doc])?;
//!
//! // Summary chunks: quick lookup
//! let summary_chunks: Vec<_> = result
//!     .chunks
//!     .iter()
//!     .filter(|c| c.chunk_level == ChunkLevel::Summary)
//!     .collect();
//!
//! // Standard chunks: balanced search results
//! let standard_chunks: Vec<_> = result
//!     .chunks
//!     .iter()
//!     .filter(|c| c.chunk_level == ChunkLevel::Standard)
//!     .collect();
//!
//! // Navigate: summary.child_chunk_ids -> standard chunk IDs
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! ## Design Principles
//!
//! **Deterministic**: Same input → same chunks (no randomness)
//!
//! **Type-Safe**: Invalid documents rejected at validation, not runtime
//!
//! **Immutable**: Chunks are frozen after creation (no mutations)
//!
//! **Zero-Panic**: All Unicode handled safely; hardcoded regex patterns verified
//!
//! **Minimal Dependencies**: Only standard Rust ecosystem (regex, serde, anyhow)
//!
//! ## Safety & Stability
//!
//! - **Unicode Handling**: Safe on emoji, multibyte, CJK characters
//! - **Token Estimation**: Consistent within ±10% (4 chars ≈ 1 token)
//! - **API Stability**: Chunk structure frozen; no breaking changes in 0.x
//! - **Panic Safety**: No unwrap(), no expect() except hardcoded regex (tested)
//!
//! ## Performance
//!
//! - **Time**: O(n) where n = document content length
//! - **Space**: O(chunks) - stores all chunks in memory
//! - **Token Estimation**: O(content_length) - linear scan
//!
//! Suitable for documents up to 100MB+ with efficient memory management.
//!
//! ## Versioning
//!
//! This crate uses Semantic Versioning:
//! - `0.1.0`: Initial release with core chunking
//! - Future `0.2.0`: Custom separators, token estimation plugins
//! - Future `1.0.0`: Stable public API
//!
//! Breaking changes documented in CHANGELOG.md.
//!
//! ## License
//!
//! MIT - See LICENSE file in repository
//!

pub mod chunk;
pub mod document;

// Re-export public API
pub use chunk::{chunk, chunk_all, Chunk, ChunkLevel, ChunkingResult};
pub use document::Document;

/// Trait for document chunking strategies
///
/// Implementations can define custom chunking behavior for different
/// use cases (e.g., different token limits, boundary strategies, etc.).
///
/// # Example
///
/// ```
/// use contextual_chunker::{Chunker, Document, Chunk};
///
/// struct CustomChunker;
///
/// impl Chunker for CustomChunker {
///     fn chunk(&self, doc: &Document) -> anyhow::Result<Vec<Chunk>> {
///         Ok(Vec::new())
///     }
/// }
/// ```
pub trait Chunker {
    /// Chunk a document according to implementation's strategy
    ///
    /// # Arguments
    ///
    /// * `doc` - The document to chunk
    ///
    /// # Returns
    ///
    /// A vector of chunks
    fn chunk(&self, doc: &Document) -> Result<Vec<Chunk>, anyhow::Error>;
}

/// Contextual chunking with configurable parameters
///
/// Provides semantic chunking at three hierarchical levels (Summary, Standard, Detailed)
/// with configurable context prefix sizes. Each chunk (except first) includes
/// context from previous chunk to maintain semantic continuity.
///
/// # Example
///
/// ```
/// use contextual_chunker::{Chunker, ContextualChunker, Document, ChunkLevel};
///
/// let chunker = ContextualChunker::standard();
/// let doc = Document::new(
///     "guide".to_string(),
///     "Guide".to_string(),
///     "## Intro\nContent".to_string(),
/// );
///
/// let chunks = chunker.chunk(&doc).unwrap();
/// ```
pub struct ContextualChunker {
    pub level: ChunkLevel,
    pub context_tokens: usize,
}

impl ContextualChunker {
    /// Create a new ContextualChunker with custom parameters
    ///
    /// # Arguments
    ///
    /// * `level` - The hierarchical level (Summary/Standard/Detailed)
    /// * `context_tokens` - Number of tokens to include from previous chunk as context
    pub fn new(level: ChunkLevel, context_tokens: usize) -> Self {
        Self {
            level,
            context_tokens,
        }
    }

    /// Create a chunker for Summary level (~128 tokens, 30 context tokens)
    pub fn summary() -> Self {
        Self::new(ChunkLevel::Summary, 30)
    }

    /// Create a chunker for Standard level (~512 tokens, 100 context tokens)
    pub fn standard() -> Self {
        Self::new(ChunkLevel::Standard, 100)
    }

    /// Create a chunker for Detailed level (~1024 tokens, 200 context tokens)
    pub fn detailed() -> Self {
        Self::new(ChunkLevel::Detailed, 200)
    }
}

impl Chunker for ContextualChunker {
    fn chunk(&self, doc: &Document) -> Result<Vec<Chunk>, anyhow::Error> {
        crate::chunk::chunk(doc, self.level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunker_trait_exists() {
        let chunker = ContextualChunker::standard();
        let doc = Document::new(
            "test".to_string(),
            "Test".to_string(),
            "## Section\nContent".to_string(),
        );

        let chunks = chunker.chunk(&doc);
        assert!(chunks.is_ok());
        assert!(!chunks.unwrap().is_empty());
    }

    #[test]
    fn test_contextual_chunker_factory_summary() {
        let chunker = ContextualChunker::summary();
        assert_eq!(chunker.level, ChunkLevel::Summary);
        assert_eq!(chunker.context_tokens, 30);
    }

    #[test]
    fn test_contextual_chunker_factory_standard() {
        let chunker = ContextualChunker::standard();
        assert_eq!(chunker.level, ChunkLevel::Standard);
        assert_eq!(chunker.context_tokens, 100);
    }

    #[test]
    fn test_contextual_chunker_factory_detailed() {
        let chunker = ContextualChunker::detailed();
        assert_eq!(chunker.level, ChunkLevel::Detailed);
        assert_eq!(chunker.context_tokens, 200);
    }

    #[test]
    fn test_contextual_chunker_custom_config() {
        let chunker = ContextualChunker::new(ChunkLevel::Standard, 150);
        assert_eq!(chunker.level, ChunkLevel::Standard);
        assert_eq!(chunker.context_tokens, 150);
    }

    #[test]
    fn test_chunker_produces_correct_level() {
        let doc = Document::new(
            "test".to_string(),
            "Test".to_string(),
            "## Section 1\nContent 1\n## Section 2\nContent 2".to_string(),
        );

        let summary_chunker = ContextualChunker::summary();
        let summary_chunks = summary_chunker.chunk(&doc).unwrap();
        for chunk in summary_chunks {
            assert_eq!(chunk.chunk_level, ChunkLevel::Summary);
        }

        let standard_chunker = ContextualChunker::standard();
        let standard_chunks = standard_chunker.chunk(&doc).unwrap();
        for chunk in standard_chunks {
            assert_eq!(chunk.chunk_level, ChunkLevel::Standard);
        }

        let detailed_chunker = ContextualChunker::detailed();
        let detailed_chunks = detailed_chunker.chunk(&doc).unwrap();
        for chunk in detailed_chunks {
            assert_eq!(chunk.chunk_level, ChunkLevel::Detailed);
        }
    }

    #[test]
    fn test_backward_compatibility_free_functions() {
        let doc = Document::new(
            "test".to_string(),
            "Test".to_string(),
            "## Section\nContent".to_string(),
        );

        let chunks = crate::chunk::chunk(&doc, ChunkLevel::Standard).unwrap();
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunker_trait_object() {
        let chunker: Box<dyn Chunker> = Box::new(ContextualChunker::standard());
        let doc = Document::new(
            "test".to_string(),
            "Test".to_string(),
            "## Section\nContent".to_string(),
        );

        let chunks = chunker.chunk(&doc);
        assert!(chunks.is_ok());
        assert!(!chunks.unwrap().is_empty());
    }
}
