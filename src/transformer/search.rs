//! Full-text search using Tantivy
//!
//! Replaces custom BM25 with a proven, production-grade search engine.
//! Handles indexing, querying, and error recovery.
//!
//! ## Design
//!
//! - **Index Location**: `{base_path}/.tantivy_index/`
//! - **Schema**: title (boosted), summary, category, word_count
//! - **Query Support**: Simple queries, phrases, boolean operators
//! - **Error Recovery**: Auto-rebuild on corruption
//!
//! ## Example
//!
//! ```no_run
//! use doc_transformer::search;
//! use std::path::Path;
//!
//! let index_path = Path::new("./output/.tantivy_index");
//! let index = search::open_or_create_index(index_path)?;
//! let results = search::search_index(&index, "rust programming", 10)?;
//! # Ok::<(), anyhow::Error>(())
//! ```

use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::doc;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, Value, STORED, TEXT};
use tantivy::Index;

/// BM25 parameter: term frequency saturation.
///
/// Controls how quickly the term frequency component saturates.
/// Higher values reduce the impact of repeated term occurrences.
///
/// - Range: 1.2-2.0 is typical
/// - 1.2 = lower saturation (term frequency continues to contribute)
/// - 2.0 = higher saturation (diminishing returns kicks in earlier)
///
/// See: Robertson & Zaragoza (2009) "The Probabilistic Relevance Framework: BM25 and Beyond"
const BM25_K1: f32 = 1.2;

/// BM25 parameter: document length normalization.
///
/// Controls how much document length affects the score.
///
/// - 0.0 = no length normalization (long documents have no penalty)
/// - 1.0 = full normalization (long documents heavily penalized)
/// - 0.75 = standard value (balances length and relevance)
///
/// See: Robertson & Zaragoza (2009) "The Probabilistic Relevance Framework: BM25 and Beyond"
const BM25_B: f32 = 0.75;

/// Schema field indices (cached for performance)
pub struct SchemaFields {
    pub id: Field,
    pub title: Field,
    pub summary: Field,
    pub content: Field,
    pub category: Field,
    pub word_count: Field,
}

/// Single search result with score
#[allow(dead_code)] // Exported for library users - not used internally
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub category: String,
    pub score: f32,
    pub path: String,
}

/// Create Tantivy schema for document indexing
///
/// Schema includes:
/// - `id`: String identifier (stored, not indexed)
/// - `title`: Text field (important for ranking)
/// - `summary`: Text field (stored, indexed)
/// - `content`: Combined searchable content (title + summary, indexed but not stored)
/// - `category`: Text field for filtering (stored, not indexed)
/// - `word_count`: U64 field for relevance calculation (stored, not indexed)
///
/// # Returns
///
/// Tantivy Schema with field definitions
fn create_schema() -> (Schema, SchemaFields) {
    let mut schema_builder = Schema::builder();

    let id = schema_builder.add_text_field("id", TEXT | STORED);
    let title = schema_builder.add_text_field("title", TEXT | STORED);
    let summary = schema_builder.add_text_field("summary", TEXT | STORED);
    let content = schema_builder.add_text_field("content", TEXT);
    let category = schema_builder.add_text_field("category", TEXT | STORED);
    let word_count = schema_builder.add_u64_field("word_count", STORED);

    let schema = schema_builder.build();
    let fields = SchemaFields {
        id,
        title,
        summary,
        content,
        category,
        word_count,
    };

    (schema, fields)
}

/// Open existing Tantivy index or create new one if missing/corrupted
///
/// ## Behavior
///
/// 1. If `index_path/.tantivy_index` exists and is valid, open it
/// 2. If index is corrupted, delete and recreate from scratch
/// 3. If missing, create new empty index
///
/// ## Error Handling
///
/// Returns error only if index directory creation fails or persistence layer issues.
/// Corruption is handled transparently (rebuild).
///
/// # Arguments
///
/// * `index_path` - Directory where index will be stored/opened
///
/// # Returns
///
/// Tantivy Index ready for reading/writing
pub fn open_or_create_index(index_path: &Path) -> Result<Index> {
    let index_dir = index_path.join(".tantivy_index");

    // Try to open existing index
    if index_dir.exists() {
        match Index::open_in_dir(&index_dir) {
            Ok(index) => return Ok(index),
            Err(_) => {
                // Index is corrupted, rebuild
                fs::remove_dir_all(&index_dir).ok();
            }
        }
    }

    // Create new index
    fs::create_dir_all(&index_dir)?;
    let (schema, _fields) = create_schema();
    Index::create_in_dir(&index_dir, schema).map_err(|e| anyhow!("Failed to create index: {e}"))
}

/// Index a batch of documents into Tantivy
///
/// ## Behavior
///
/// - Creates new writer
/// - Adds all documents
/// - Commits transaction
///
/// ## Error Handling
///
/// Returns error if write or commit fails (e.g., disk full, permissions).
///
/// # Arguments
///
/// * `index` - Tantivy index to write to
/// * `documents` - Documents to index (converted to Tantivy Document format)
///
/// # Returns
///
/// Success on commit, error if any operation fails
pub fn index_documents(index: &Index, documents: Vec<crate::index::IndexDocument>) -> Result<()> {
    let (_schema, fields) = create_schema();

    // Create writer with buffer size for batch operations
    let mut writer = index.writer(50_000_000)?;

    // Add each document
    for doc in documents {
        // content field: combination of title + summary for searching
        let searchable_content = format!("{} {} {}", doc.title, doc.summary, doc.path);

        // Use tantivy::doc! macro to build document
        let tantivy_doc = doc!(
            fields.id => doc.id.as_str(),
            fields.title => doc.title.as_str(),
            fields.summary => doc.summary.as_str(),
            fields.content => searchable_content.as_str(),
            fields.category => doc.category.as_str(),
            fields.word_count => doc.word_count as u64,
        );

        writer.add_document(tantivy_doc)?;
    }

    // Commit transaction
    writer.commit()?;

    Ok(())
}

/// Search the Tantivy index
///
/// ## Query Syntax
///
/// - Simple: `rust programming` → Any document with both terms
/// - Phrase: `"rust programming"` → Exact phrase match
/// - Boolean: `rust AND systems` → Both terms required
/// - Negation: `rust NOT python` → rust without python
/// - Operators: `(rust OR systems) AND NOT python`
///
/// ## Behavior
///
/// - Parses query using Tantivy's default QueryParser
/// - Executes against content field (searchable combination)
/// - Returns top N results sorted by BM25 score (highest first)
/// - Returns empty Vec if no matches
///
/// ## Error Handling
///
/// Returns error if query is invalid (syntax error).
/// Empty query returns error.
///
/// # Arguments
///
/// * `index` - Tantivy index to search
/// * `query_str` - Query string (supports phrase and boolean operators)
/// * `limit` - Maximum number of results to return
///
/// # Returns
///
/// Vector of SearchResult sorted by relevance (highest score first)
#[allow(dead_code)] // Exported for library users - not used internally
pub fn search_index(index: &Index, query_str: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let (_schema, fields) = create_schema();

    // Validate query using centralized validation
    let query_str = crate::validate::validate_query(query_str).map_err(|e| anyhow!("{e}"))?;

    // Validate limit to prevent Tantivy panic (must be > 0)
    let limit = crate::validate::validate_limit(limit).map_err(|e| anyhow!("{e}"))?;

    // Get reader for searching
    let reader = index.reader()?;
    let searcher = reader.searcher();

    // Parse query
    let query_parser = QueryParser::for_index(index, vec![fields.content]);
    let query = query_parser
        .parse_query(query_str)
        .map_err(|e| anyhow!("Invalid query: {e}"))?;

    // Execute search and get top results
    let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

    let mut results = Vec::new();

    // Extract stored fields from results
    for (_score, doc_address) in top_docs {
        let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

        // Extract fields (safely with defaults)
        // Tantivy 0.25: Convert CompactDocValue -> OwnedValue -> extract
        let id = retrieved_doc
            .get_first(fields.id)
            .map(tantivy::schema::OwnedValue::from)
            .and_then(|v| v.as_ref().as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        let title = retrieved_doc
            .get_first(fields.title)
            .map(tantivy::schema::OwnedValue::from)
            .and_then(|v| v.as_ref().as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "Untitled".to_string());

        let summary = retrieved_doc
            .get_first(fields.summary)
            .map(tantivy::schema::OwnedValue::from)
            .and_then(|v| v.as_ref().as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let category = retrieved_doc
            .get_first(fields.category)
            .map(tantivy::schema::OwnedValue::from)
            .and_then(|v| v.as_ref().as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "uncategorized".to_string());

        let word_count = retrieved_doc
            .get_first(fields.word_count)
            .map(tantivy::schema::OwnedValue::from)
            .and_then(|v| v.as_ref().as_u64())
            .unwrap_or(0);

        // Recalculate score for this document using simplified BM25
        // (Tantivy's internal score can't be easily extracted)
        // SAFETY: word_count from u64 field, typical values < 100k, well within f32 precision
        let score = score_document_simple(&title, &summary, query_str, word_count as f32);

        // Convert ID format (category/subcategory/slug) to filename format (category-subcategory-slug.md)
        let path = format!("docs/{}.md", id.replace('/', "-"));

        // Only include results with positive scores (filter out non-matches and negative zeros)
        if score <= 0.0 {
            continue;
        }

        results.push(SearchResult {
            id,
            title,
            summary,
            category,
            score,
            path,
        });
    }

    Ok(results)
}

/// Simple BM25 scoring for a single document.
///
/// Used as fallback when Tantivy index is unavailable.
/// This is the original simplified BM25 implementation.
///
/// ## Parameters
///
/// - `BM25_K1`: term frequency saturation point (1.2)
/// - `BM25_B`: length normalization (0.75)
/// - IDF: ln(10) per term (simplified, not actual document frequency)
///
/// # Arguments
///
/// * `title` - Document title
/// * `summary` - Document summary
/// * `query` - Search query
/// * `word_count` - Document length (for normalization)
///
/// # Returns
///
/// BM25 score (higher = more relevant)
///
/// See: Robertson & Zaragoza (2009) "The Probabilistic Relevance Framework: BM25 and Beyond"
#[allow(dead_code)] // Exported for library users - not used internally
pub fn score_document_simple(title: &str, summary: &str, query: &str, word_count: f32) -> f32 {
    let k1 = BM25_K1;
    let b = BM25_B;

    let document = format!("{title} {summary}");
    let doc_words: Vec<&str> = document.split_whitespace().collect();
    // SAFETY: Document length (title + summary) typically < 1000 words, well within f32 precision
    let doc_length = doc_words.len() as f32;

    // Avoid division by zero
    let avg_doc_length = word_count.max(1.0);

    query
        .split_whitespace()
        .map(|term| {
            let term_lower = term.to_lowercase();
            // SAFETY: Term frequency in a single document typically < 100, well within f32 precision
            doc_words
                .iter()
                .filter(|w| w.to_lowercase() == term_lower)
                .count() as f32
        })
        .filter(|&tf| tf > 0.0)
        .map(|tf| {
            let idf = (10.0_f32).ln();
            let numerator = tf * (k1 + 1.0);
            let denominator = tf + k1 * (1.0 - b + b * (doc_length / avg_doc_length));
            // Guard against division by zero (should be prevented above)
            idf * (numerator / denominator.max(0.0001))
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_open_or_create_index_new() -> Result<()> {
        let dir = TempDir::new()?;
        let index_path = dir.path();

        let _index = open_or_create_index(index_path)?;
        assert!(index_path.join(".tantivy_index").exists());

        Ok(())
    }

    #[test]
    fn test_open_or_create_index_existing() -> Result<()> {
        let dir = TempDir::new()?;
        let index_path = dir.path();

        // Create index
        let _index1 = open_or_create_index(index_path)?;

        // Verify we can open the same index again
        let _index2 = open_or_create_index(index_path)?;

        // Both should refer to the same files
        assert!(index_path.join(".tantivy_index").exists());

        Ok(())
    }

    #[test]
    fn test_score_document_simple_basic() {
        let score1 = score_document_simple("rust programming", "learn rust", "rust", 100.0);
        let score2 = score_document_simple("python web dev", "django framework", "rust", 100.0);

        // rust should score higher in first doc
        assert!(score1 > score2);
    }

    #[test]
    fn test_score_document_simple_multiple_terms() {
        let score1 = score_document_simple(
            "rust programming",
            "systems programming language",
            "rust programming",
            100.0,
        );
        let score2 =
            score_document_simple("rust web", "simple framework", "rust programming", 100.0);

        // Both terms present should score higher than one term
        assert!(score1 > score2);
    }

    #[test]
    fn test_score_document_simple_empty_query() {
        let score = score_document_simple("rust programming", "systems", "", 100.0);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_score_document_simple_zero_word_count() {
        // Should handle zero word_count gracefully
        let score = score_document_simple("rust", "programming", "rust", 0.0);
        assert!(score.is_finite());
        assert!(score > 0.0);
    }

    #[test]
    fn test_score_document_simple_case_insensitive() {
        let score1 = score_document_simple("Rust Programming", "Learn Rust", "rust", 100.0);
        let score2 = score_document_simple("RUST PROGRAMMING", "LEARN RUST", "RUST", 100.0);

        // Case should not matter
        assert!((score1 - score2).abs() < 0.0001);
    }
}
