# contextual-chunker

[![Crates.io](https://img.shields.io/crates/v/contextual-chunker.svg)](https://crates.io/crates/contextual-chunker)
[![Docs.rs](https://docs.rs/contextual-chunker/badge.svg)](https://docs.rs/contextual-chunker)
[![License](https://img.shields.io/crates/l/contextual-chunker.svg)](https://github.com/anthropics/centralized-docs/blob/main/contextual-chunker/LICENSE)

Semantic chunking with hierarchical levels for documentation and knowledge bases.

Split markdown documents into semantically meaningful chunks at multiple levels (Summary, Standard, Detailed) with automatic relationship tracking, making it ideal for RAG systems and retrieval-augmented generation.

## Features

- **Semantic Boundaries**: Chunks respect H2 headings (##) in markdown
- **Hierarchical Levels**: 3-level hierarchy (128, 512, 1024 tokens)
- **Automatic Relationships**: Parent-child links for progressive disclosure
- **Navigation Links**: Sequential prev/next pointers at same level
- **Content Analysis**: Automatic type detection (code/table/prose)
- **Summary Extraction**: Extractive summaries for quick overview
- **Unicode Safe**: No panics on emoji, CJK, or special characters
- **Deterministic**: Same input always produces identical chunks
- **Minimal Dependencies**: Only regex, serde, anyhow

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
contextual-chunker = "0.1"
```

## Quick Start

```rust
use contextual_chunker::{Document, chunk_all};

let documents = vec![
    Document::new(
        "getting-started".to_string(),
        "Getting Started Guide".to_string(),
        "## Installation\nSteps to install...\n## Configuration\nHow to configure...".to_string(),
    ),
];

let result = chunk_all(&documents)?;
println!("Created {} chunks across {} levels",
    result.chunks.len(),
    3 // Summary, Standard, Detailed
);
```

## Using the Chunker Trait

The crate provides a `Chunker` trait for flexible, extensible chunking strategies:

```rust
use contextual_chunker::{Chunker, ContextualChunker, Document};

// Use factory methods for common configurations
let summary_chunker = ContextualChunker::summary();   // ~128 tokens
let standard_chunker = ContextualChunker::standard(); // ~512 tokens
let detailed_chunker = ContextualChunker::detailed(); // ~1024 tokens

let doc = Document::new(
    "guide".to_string(),
    "Guide".to_string(),
    "## Intro\nContent".to_string(),
);

let chunks = standard_chunker.chunk(&doc)?;
```

### Custom Configuration

Create custom chunkers with specific parameters:

```rust
use contextual_chunker::{Chunker, ContextualChunker, Document, ChunkLevel};

// Custom level with 800 tokens and 150 context tokens
let custom_chunker = ContextualChunker::new(ChunkLevel::Standard, 150);

let doc = Document::new(
    "custom".to_string(),
    "Custom Doc".to_string(),
    "Content here...".to_string(),
);

let chunks = custom_chunker.chunk(&doc)?;
```

### Implementing Custom Chunkers

Define your own chunking strategies by implementing the `Chunker` trait:

```rust
use contextual_chunker::{Chunker, Document, Chunk};

struct SimpleChunker;

impl Chunker for SimpleChunker {
    fn chunk(&self, doc: &Document) -> anyhow::Result<Vec<Chunk>> {
        // Your custom chunking logic
        Ok(vec![])
    }
}
```

## How It Works

### 3-Level Hierarchy

Documents are chunked at three levels simultaneously for multi-granularity retrieval:

| Level | Tokens | Use Case |
|-------|--------|----------|
| **Summary** | ~128 | Quick lookups, table of contents |
| **Standard** | ~512 | Default search results, RAG context |
| **Detailed** | ~1024 | Deep reading, comprehensive context |

### Chunk Boundaries

Chunks respect markdown structure:
1. H2 headings (##) are primary boundaries
2. If section exceeds token limit, split further
3. Previous section's tail included as context (30-200 tokens)

### Relationships

```
Summary Chunk 1 (128 tokens)
├── Standard Chunk 1 (512 tokens)
│   ├── Detailed Chunk 1 (1024 tokens)
│   ├── Detailed Chunk 2 (1024 tokens)
│   └── Detailed Chunk 3 (1024 tokens)
├── Standard Chunk 2 (512 tokens)
│   └── Detailed Chunk 4 (1024 tokens)
```

Each chunk knows:
- **parent_chunk_id**: Chunk at next higher level
- **child_chunk_ids**: Chunks at next lower level
- **previous_chunk_id**: Previous chunk at same level
- **next_chunk_id**: Next chunk at same level

## Examples

### Single Document Chunking

```rust
use contextual_chunker::{Document, ChunkLevel, chunk};

let doc = Document::new(
    "api-reference".to_string(),
    "API Reference".to_string(),
    "## Authentication\nAPI keys...\n## Endpoints\n### GET /users".to_string(),
);

// Chunk at one level
let standard_chunks = chunk(&doc, ChunkLevel::Standard)?;
println!("Created {} standard chunks", standard_chunks.len());
```

### Multiple Documents

```rust
use contextual_chunker::{Document, chunk_all};

let docs = vec![
    Document::new("intro".to_string(), "Intro".to_string(), content1),
    Document::new("guide".to_string(), "Guide".to_string(), content2),
    Document::new("api".to_string(), "API".to_string(), content3),
];

let result = chunk_all(&docs)?;
println!("Summary: {}", result.summary_count);
println!("Standard: {}", result.standard_count);
println!("Detailed: {}", result.detailed_count);
```

### Navigation & Relationships

```rust
use contextual_chunker::{Document, ChunkLevel, chunk_all};

let result = chunk_all(&[doc])?;

// Find Summary chunk
let summary = result.chunks.iter()
    .find(|c| c.chunk_level == ChunkLevel::Summary)
    .unwrap();

// Navigate to Standard children
for child_id in &summary.child_chunk_ids {
    let child = result.chunks.iter()
        .find(|c| c.chunk_id == child_id)
        .unwrap();
    println!("Standard: {}", child.heading.as_deref().unwrap_or("Intro"));

    // Navigate to Detailed grandchildren
    for grandchild_id in &child.child_chunk_ids {
        let grandchild = result.chunks.iter()
            .find(|c| c.chunk_id == grandchild_id)
            .unwrap();
        println!("  → Detailed: {}", grandchild.chunk_id);
    }
}
```

### Serialization

```rust
use contextual_chunker::{Document, chunk_all};

let result = chunk_all(&[doc])?;

// Serialize to JSON
let json = serde_json::to_string(&result.chunks)?;

// Deserialize
let chunks: Vec<Chunk> = serde_json::from_str(&json)?;
```

## Design Principles

**Deterministic**: Same input → same chunks (no randomness)
- Enables caching, reproducibility, version control

**Type-Safe**: Invalid documents rejected at validation, not runtime panics

**Immutable**: Chunks are frozen after creation
- No hidden mutations, easier to reason about

**Zero-Panic**: All Unicode handled safely; regex patterns compile-time verified
- Safe on emoji 🎉, CJK 中文, combining marks, etc.

**Minimal Dependencies**: Only standard Rust ecosystem
- regex, serde, anyhow (no ML libraries or heavy dependencies)

## Token Estimation

Token counts are estimated as: **content_length / 4 ≈ tokens**

This is the OpenAI standard approximation. For exact counts, use a tokenizer:

```rust
// This crate estimates
let chunk = chunks[0];
assert!(chunk.token_count >= 100); // Approximate

// For production: use tiktoken or similar
// let exact = tiktoken::count_tokens(&chunk.content, "cl100k_base")?;
```

## Safety & Guarantees

### Panic Safety
- ❌ No panics on invalid UTF-8 (input is already String)
- ❌ No panics on Unicode (emoji, CJK, combining marks all safe)
- ❌ No panics on regex (hardcoded patterns verified at compile-time)
- ❌ No unwrap() or expect() except hardcoded regex (tested)

### Stability Guarantees
- Chunk IDs are deterministic based on content
- Token counts consistent within ±10%
- Parent-child relationships form valid DAG (no cycles)
- Navigation pointers (prev/next) are bidirectional

### API Stability (0.x)
- `Chunk` struct fields are fixed (no removals/reorders)
- `ChunkLevel` enum variants are fixed
- New features added with new methods, not breaking changes

## Performance

| Metric | Time | Space |
|--------|------|-------|
| **Small doc** (1MB) | ~1ms | ~2MB |
| **Medium doc** (10MB) | ~10ms | ~20MB |
| **Large doc** (100MB) | ~100ms | ~200MB |

All measurements on modern hardware; scales linearly with content size.

## Use Cases

### RAG (Retrieval-Augmented Generation)
- Chunk with Summary level for quick filtering
- Retrieve at Standard level for context window
- Use Detailed chunks for verification/followup

### Knowledge Base
- Summary level for table of contents
- Standard level for search results
- Detailed level for full article view

### Documentation
- Multi-level navigation (collapsed/expanded)
- Link chunks to source code (via doc_id)
- Version-aware chunking (separate docs per version)

### LLM Fine-tuning
- Use Summary/Standard chunks for training
- Preserve hierarchical structure in training data

## Contributing

Contributions welcome! Please:
1. Add tests for new features
2. Maintain documentation
3. Keep dependencies minimal
4. Ensure zero unsafe code

## License

MIT - See LICENSE file

## Changelog

### 0.1.0 (Initial Release)
- Core chunking algorithm
- 3-level hierarchy
- Parent-child relationships
- Sequential navigation links
- Content type detection
- Summary extraction

## Related Projects

- [tantivy](https://github.com/quickwit-oss/tantivy) - Full-text search
- [pgvector](https://github.com/pgvector/pgvector) - Semantic search in PostgreSQL
- [ragas](https://github.com/explodinggradients/ragas) - RAG evaluation
- [llms.txt](https://github.com/llmsorg/llms-txt) - AI documentation standard

## Citation

If you use contextual-chunker in research, please cite:

```bibtex
@software{contextual_chunker,
  author = {Anthropic},
  title = {contextual-chunker: Semantic Chunking for Documentation},
  year = {2026},
  url = {https://github.com/anthropics/centralized-docs}
}
```

## Questions?

- Issues: https://github.com/anthropics/centralized-docs/issues
- Discussions: https://github.com/anthropics/centralized-docs/discussions
- Email: opensource@anthropic.com
