//! Strongly-typed identifiers and domain types for the document transformer.

use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::cmp::PartialOrd;
use std::fmt;
use std::ops::Deref;

/// A newtype wrapper for document IDs that prevents accidental mixing with other string types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DocumentId(String);

impl DocumentId {
    /// Create a new DocumentId, validating that it's not empty.
    #[allow(dead_code)]
    pub fn new(id: impl Into<String>) -> Result<Self, DocumentIdError> {
        let s = id.into();
        if s.trim().is_empty() {
            Err(DocumentIdError::Empty)
        } else {
            Ok(DocumentId(s))
        }
    }

    /// Get the underlying string value.
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to owned String.
    #[allow(dead_code)]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for DocumentId {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for DocumentId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for DocumentId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// A newtype wrapper for chunk IDs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ChunkId(String);

impl ChunkId {
    /// Create a new ChunkId, validating that it's not empty.
    #[allow(dead_code)]
    pub fn new(id: impl Into<String>) -> Result<Self, ChunkIdError> {
        let s = id.into();
        if s.trim().is_empty() {
            Err(ChunkIdError::Empty)
        } else {
            Ok(ChunkId(s))
        }
    }

    /// Get the underlying string value.
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to owned String.
    #[allow(dead_code)]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for ChunkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for ChunkId {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for ChunkId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for ChunkId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// A newtype wrapper for tags.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Tag(String);

impl Tag {
    /// Create a new Tag, validating that it's not empty and reasonably sized.
    #[allow(dead_code)]
    pub fn new(tag: impl Into<String>) -> Result<Self, TagError> {
        let s = tag.into();
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err(TagError::Empty);
        }

        if trimmed.len() > 100 {
            return Err(TagError::TooLong(trimmed.len()));
        }

        Ok(Tag(trimmed.to_lowercase()))
    }

    /// Get the underlying string value.
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to owned String.
    #[allow(dead_code)]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for Tag {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for Tag {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// A newtype wrapper for keywords.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Keyword(String);

impl Keyword {
    /// Create a new Keyword, validating that it's not empty and meets minimum length.
    #[allow(dead_code)]
    pub fn new(keyword: impl Into<String>) -> Result<Self, KeywordError> {
        let s = keyword.into();
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err(KeywordError::Empty);
        }

        if trimmed.len() < 2 {
            return Err(KeywordError::TooShort(trimmed.len()));
        }

        if trimmed.len() > 50 {
            return Err(KeywordError::TooLong(trimmed.len()));
        }

        Ok(Keyword(trimmed.to_lowercase()))
    }

    /// Get the underlying string value.
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to owned String.
    #[allow(dead_code)]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for Keyword {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for Keyword {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for Keyword {
    fn borrow(&self) -> &str {
        &self.0
    }
}

// Error types

/// Errors that can occur when creating DocumentId.
#[derive(Debug, Clone, thiserror::Error)]
#[allow(dead_code)]
pub enum DocumentIdError {
    #[error("Document ID cannot be empty")]
    Empty,
}

/// Errors that can occur when creating ChunkId.
#[derive(Debug, Clone, thiserror::Error)]
#[allow(dead_code)]
pub enum ChunkIdError {
    #[error("Chunk ID cannot be empty")]
    Empty,
}

/// Errors that can occur when creating Tag.
#[derive(Debug, Clone, thiserror::Error)]
#[allow(dead_code)]
pub enum TagError {
    #[error("Tag cannot be empty")]
    Empty,
    #[error("Tag too long: {0} characters (max 100)")]
    TooLong(usize),
}

/// Errors that can occur when creating Keyword.
#[derive(Debug, Clone, thiserror::Error)]
#[allow(dead_code)]
pub enum KeywordError {
    #[error("Keyword cannot be empty")]
    Empty,
    #[error("Keyword too short: {0} characters (min 2)")]
    TooShort(usize),
    #[error("Keyword too long: {0} characters (max 50)")]
    TooLong(usize),
}

/// A validated wrapper for max_related_chunks configuration parameter (1-1000).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MaxRelatedChunks(u16); // u16 is sufficient for max 1000

impl MaxRelatedChunks {
    /// Create a new MaxRelatedChunks, validating the range (1-1000).
    pub fn new(value: usize) -> Result<Self, ConfigError> {
        if value < 1 {
            return Err(ConfigError::MaxRelatedChunksTooSmall(value));
        }
        if value > 1000 {
            return Err(ConfigError::MaxRelatedChunksTooLarge(value));
        }
        // Convert to u16 since we validated the range
        Ok(MaxRelatedChunks(value as u16))
    }

    /// Get the underlying value.
    pub fn get(&self) -> usize {
        self.0 as usize
    }
}

// Comparison with usize for test ergonomics
impl PartialEq<usize> for MaxRelatedChunks {
    fn eq(&self, other: &usize) -> bool {
        self.get() == *other
    }
}

impl PartialEq<MaxRelatedChunks> for usize {
    fn eq(&self, other: &MaxRelatedChunks) -> bool {
        *self == other.get()
    }
}

impl PartialOrd<usize> for MaxRelatedChunks {
    fn partial_cmp(&self, other: &usize) -> Option<std::cmp::Ordering> {
        self.get().partial_cmp(other)
    }
}

impl PartialOrd<MaxRelatedChunks> for usize {
    fn partial_cmp(&self, other: &MaxRelatedChunks) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&other.get())
    }
}

impl fmt::Display for MaxRelatedChunks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for MaxRelatedChunks {
    fn default() -> Self {
        MaxRelatedChunks(20) // Default value
    }
}

/// A validated wrapper for HNSW M parameter (4-64).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HnswM(u8); // u8 is sufficient for max 64

impl HnswM {
    /// Create a new HnswM, validating the range (4-64).
    pub fn new(value: usize) -> Result<Self, ConfigError> {
        if value < 4 {
            return Err(ConfigError::HnswMTooSmall(value));
        }
        if value > 64 {
            return Err(ConfigError::HnswMTooLarge(value));
        }
        // Convert to u8 since we validated the range
        Ok(HnswM(value as u8))
    }

    /// Get the underlying value.
    pub fn get(&self) -> usize {
        self.0 as usize
    }
}

// Comparison with usize for test ergonomics
impl PartialEq<usize> for HnswM {
    fn eq(&self, other: &usize) -> bool {
        self.get() == *other
    }
}

impl PartialEq<HnswM> for usize {
    fn eq(&self, other: &HnswM) -> bool {
        *self == other.get()
    }
}

impl PartialOrd<usize> for HnswM {
    fn partial_cmp(&self, other: &usize) -> Option<std::cmp::Ordering> {
        self.get().partial_cmp(other)
    }
}

impl PartialOrd<HnswM> for usize {
    fn partial_cmp(&self, other: &HnswM) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&other.get())
    }
}

impl fmt::Display for HnswM {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for HnswM {
    fn default() -> Self {
        HnswM(16) // Default value
    }
}

/// A validated wrapper for HNSW ef_construction parameter (50-1000).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HnswEfConstruction(u16); // u16 is sufficient for max 1000

impl HnswEfConstruction {
    /// Create a new HnswEfConstruction, validating the range (50-1000).
    pub fn new(value: usize) -> Result<Self, ConfigError> {
        if value < 50 {
            return Err(ConfigError::HnswEfConstructionTooSmall(value));
        }
        if value > 1000 {
            return Err(ConfigError::HnswEfConstructionTooLarge(value));
        }
        // Convert to u16 since we validated the range
        Ok(HnswEfConstruction(value as u16))
    }

    /// Get the underlying value.
    pub fn get(&self) -> usize {
        self.0 as usize
    }
}

// Comparison with usize for test ergonomics
impl PartialEq<usize> for HnswEfConstruction {
    fn eq(&self, other: &usize) -> bool {
        self.get() == *other
    }
}

impl PartialEq<HnswEfConstruction> for usize {
    fn eq(&self, other: &HnswEfConstruction) -> bool {
        *self == other.get()
    }
}

impl PartialOrd<usize> for HnswEfConstruction {
    fn partial_cmp(&self, other: &usize) -> Option<std::cmp::Ordering> {
        self.get().partial_cmp(other)
    }
}

impl PartialOrd<HnswEfConstruction> for usize {
    fn partial_cmp(&self, other: &HnswEfConstruction) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&other.get())
    }
}

impl fmt::Display for HnswEfConstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for HnswEfConstruction {
    fn default() -> Self {
        HnswEfConstruction(200) // Default value
    }
}

/// Configuration validation errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    #[error("max_related_chunks must be at least 1, got {0}")]
    MaxRelatedChunksTooSmall(usize),
    #[error("max_related_chunks must be at most 1000, got {0}")]
    MaxRelatedChunksTooLarge(usize),
    #[error("hnsw_m must be at least 4, got {0}")]
    HnswMTooSmall(usize),
    #[error("hnsw_m must be at most 64, got {0}")]
    HnswMTooLarge(usize),
    #[error("hnsw_ef_construction must be at least 50, got {0}")]
    HnswEfConstructionTooSmall(usize),
    #[error("hnsw_ef_construction must be at most 1000, got {0}")]
    HnswEfConstructionTooLarge(usize),
}

/// Stopwords to filter out from tags and keywords.
///
/// Used across multiple modules to ensure consistent filtering.
pub const STOPWORDS: [&str; 10] = [
    "this", "that", "these", "those", "about", "guide", "the", "and", "or", "for",
];

/// Check if a word is a stopword (case-insensitive).
///
/// Used in tag and keyword extraction to filter common words.
pub fn is_stopword(word: &str) -> bool {
    STOPWORDS.contains(&word.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_documentid_valid() {
        let result = DocumentId::new("doc1");
        assert!(matches!(result, Ok(ref id) if id.as_str() == "doc1" && id.to_string() == "doc1"));
    }

    #[test]
    fn test_documentid_empty() {
        let result = DocumentId::new("");
        assert!(result.is_err());
        assert!(matches!(result, Err(DocumentIdError::Empty)));
    }

    #[test]
    fn test_documentid_whitespace_only() {
        let result = DocumentId::new("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_chunkid_valid() {
        let result = ChunkId::new("chunk_1");
        assert!(matches!(result, Ok(ref id) if id.as_str() == "chunk_1"));
    }

    #[test]
    fn test_tag_valid() {
        let result = Tag::new("Rust");
        assert!(matches!(result, Ok(ref tag) if tag.as_str() == "rust"));
    }

    #[test]
    fn test_tag_case_insensitive() {
        let result = Tag::new("RUST");
        assert!(matches!(result, Ok(ref tag) if tag.as_str() == "rust"));
    }

    #[test]
    fn test_tag_too_long() {
        let long_tag = "a".repeat(101);
        let result = Tag::new(long_tag);
        assert!(result.is_err());
        assert!(matches!(result, Err(TagError::TooLong(_))));
    }

    #[test]
    fn test_keyword_valid() {
        let result = Keyword::new("function");
        assert!(matches!(result, Ok(ref kw) if kw.as_str() == "function"));
    }

    #[test]
    fn test_keyword_too_short() {
        let result = Keyword::new("a");
        assert!(result.is_err());
        assert!(matches!(result, Err(KeywordError::TooShort(1))));
    }

    #[test]
    fn test_keyword_too_long() {
        let long_kw = "a".repeat(51);
        let result = Keyword::new(long_kw);
        assert!(result.is_err());
        assert!(matches!(result, Err(KeywordError::TooLong(_))));
    }

    #[test]
    fn test_keyword_case_insensitive() {
        let result = Keyword::new("FUNCTION");
        assert!(matches!(result, Ok(ref kw) if kw.as_str() == "function"));
    }
}
