//! Input document representation for chunking
//!
//! Design by Contract:
//! - Invariants: id and title are non-empty; content can be empty but is valid UTF-8
//! - Precondition: id must be a valid unique identifier (URL-safe)
//! - Postcondition: all fields immutable after construction

use serde::{Deserialize, Serialize};

/// A document to be chunked into semantic segments
///
/// This is the primary input type for the chunking engine.
/// It encapsulates the minimal information needed for semantic chunking:
/// unique identification, presentation, and content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier for the document (e.g., file path, URL slug)
    ///
    /// Used to generate chunk IDs and establish relationships.
    /// Must be unique within a chunking session.
    /// Example: "guides/getting-started" or "api-reference"
    pub id: String,

    /// Human-readable document title
    ///
    /// Used in chunk metadata and summaries.
    /// Should be descriptive but concise (< 200 chars recommended).
    /// Example: "Getting Started Guide" or "API Reference"
    pub title: String,

    /// The actual document content (markdown format recommended)
    ///
    /// Can be empty for testing purposes.
    /// Expected to be valid UTF-8 (panics on invalid UTF-8 indicate corruption).
    /// Supports:
    /// - Markdown (recommended): H2 headings (##) as natural chunk boundaries
    /// - HTML: Ignored but preserved
    /// - Plain text: Chunked by token count alone
    pub content: String,
}

impl Document {
    /// Create a new document
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the document
    /// * `title` - Human-readable document title
    /// * `content` - Document content (usually markdown)
    ///
    /// # Examples
    ///
    /// ```
    /// use contextual_chunker::Document;
    ///
    /// let doc = Document::new(
    ///     "guides/intro".to_string(),
    ///     "Introduction Guide".to_string(),
    ///     "## Getting Started\n\nThis is an introduction...".to_string(),
    /// );
    /// assert_eq!(doc.id, "guides/intro");
    /// ```
    pub fn new(id: String, title: String, content: String) -> Self {
        Document { id, title, content }
    }

    /// Validate document has required fields
    ///
    /// Returns true if id and title are non-empty.
    /// Content can be empty for test documents.
    pub fn is_valid(&self) -> bool {
        !self.id.is_empty() && !self.title.is_empty()
    }

    /// Calculate rough content size in tokens
    ///
    /// Uses simple estimation: ~4 characters per token.
    /// Useful for determining if document is within expected range.
    pub fn estimated_tokens(&self) -> usize {
        (self.content.len() / 4).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::new(
            "test-doc".to_string(),
            "Test Document".to_string(),
            "This is test content.".to_string(),
        );
        assert_eq!(doc.id, "test-doc");
        assert_eq!(doc.title, "Test Document");
        assert!(doc.is_valid());
    }

    #[test]
    fn test_document_validation() {
        let valid_doc = Document::new("id".to_string(), "title".to_string(), "".to_string());
        assert!(valid_doc.is_valid());

        let invalid_id = Document::new("".to_string(), "title".to_string(), "content".to_string());
        assert!(!invalid_id.is_valid());

        let invalid_title = Document::new("id".to_string(), "".to_string(), "content".to_string());
        assert!(!invalid_title.is_valid());
    }

    #[test]
    fn test_token_estimation() {
        let doc = Document::new(
            "test".to_string(),
            "Test".to_string(),
            "This is a test with about sixteen characters".to_string(),
        );
        let tokens = doc.estimated_tokens();
        assert!(tokens > 0);
        assert!((10..=12).contains(&tokens)); // ~44 chars / 4
    }

    #[test]
    fn test_unicode_content() {
        let doc = Document::new(
            "unicode".to_string(),
            "Unicode Test".to_string(),
            "This contains emoji 🎉 and CJK 中文 characters".to_string(),
        );
        assert!(doc.is_valid());
        assert!(doc.estimated_tokens() > 0);
    }
}
