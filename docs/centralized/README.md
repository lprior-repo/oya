# centralized-docs

A pure Rust CLI tool for transforming raw documentation into AI-optimized, searchable knowledge structures.

## Overview

**centralized-docs** (v5.0) is the **best documentation indexer for AI agents**, transforming any documentation into a semantic knowledge graph with:

- 🕷️ **Web scraping** via spider-rs with sitemap.xml support
- 🎯 **Content filtering** using BM25 relevance and text density pruning
- 🤖 **llms.txt generation** - AI-first entry point files
- 📑 **Automatic metadata extraction** (titles, headings, categories, tags)
- 🔗 **Knowledge Graph DAG** with Jaccard similarity relationships
- 📝 **Semantic chunking** with 50-100 token context prefixes
- 🔍 **Full-text search** using Tantivy with BM25 scoring
- 🧠 **Semantic similarity** via HNSW approximate nearest neighbor
- 🧭 **Navigation guides** (COMPASS.md and AGENTS.md)
- ✅ **Automated validation** and quality checking

## Quick Start

### Build
```bash
moon run :build      # Build release binaries (cached)
moon run :install    # Install to ~/.local/bin
```

### Usage

**Scrape a documentation website:**
```bash
./target/release/doc_transformer scrape https://docs.example.com \
  --output ./scraped \
  --delay 250
```

**Index local markdown files:**
```bash
./target/release/doc_transformer index ./source_docs \
  --output ./indexed \
  --llms-txt
```

**One-shot scrape + index:**
```bash
./target/release/doc_transformer ingest https://docs.example.com \
  --output ./indexed
```

**Search indexed documentation:**
```bash
./target/release/doc_transformer search "query terms" \
  --index-dir ./indexed \
  --limit 10
```

**Legacy mode (backward compatible):**
```bash
./target/release/doc_transformer ./source_docs ./output_index
```

### Output Structure
```
output_index/
├── llms.txt                 # AI entry point (read this first!)
├── llms-full.txt            # Full content for large context models
├── AGENTS.md                # Instructions for AI coding agents
├── INDEX.json               # Complete searchable index + knowledge graph
├── COMPASS.md               # Human-readable navigation guide
├── docs/                    # Transformed documents with YAML frontmatter
├── chunks/                  # Semantic chunks with context prefixes
└── .tantivy_index/          # Full-text search index
```

## Architecture

### 7-Step Pipeline

1. **DISCOVER** - Scan directories for markdown files
2. **ANALYZE** - Extract metadata (titles, headings, categories)
3. **ASSIGN IDs** - Generate hierarchical document IDs
4. **TRANSFORM** - Apply standard formatting and frontmatter
5. **CHUNK** - Semantic splitting with context prefixes (~512 tokens/chunk)
6. **INDEX** - Build searchable index (INDEX.json)
7. **VALIDATE** - Quality checks and validation

### Key Features

**Knowledge Graph (DAG)**
- Automatic relationship detection
- Jaccard similarity scoring
- Topological ordering
- Semantic navigation

**Contextual Retrieval**
- Each chunk includes 50-100 token context prefix
- Natural multi-turn AI conversations
- 35% fewer retrieval failures (Anthropic research)

**Full-Text Search**
- Keyword indexing
- Category and tag filtering
- Complete chunk navigation

## Example

```bash
# Transform CUE documentation (36 files)
./target/release/doc_transformer ./cue_docs ./indexed_output

# Output
# ======================================================================
# DOC_TRANSFORMER v4.3 (Knowledge DAG)
# ======================================================================
# [STEP 1] DISCOVER: Found 36 files
# [STEP 2] ANALYZE: Processed 36 files
# [STEP 3] ASSIGN IDs: Generated 36 IDs
# [STEP 4] TRANSFORM: 36/36 files (0 errors)
# [STEP 5] CHUNK: Generated 156 chunks
# [STEP 6] INDEX: Created COMPASS.md and INDEX.json
# [STEP 7] VALIDATE: 36/36 files passed
# ======================================================================
# COMPLETE
```

## For AI Agents

Load the generated INDEX.json to:
1. Search by keyword
2. Get document metadata and chunk list
3. Retrieve individual chunks with context
4. Navigate related documents via Knowledge Graph

See `docs/INDEXER.md` for complete integration guide.

## Testing

```bash
moon run :test       # Run all tests
moon run :ci         # Full CI pipeline
moon run :quick      # Quick format + lint check
```

## Project Structure

```
centralized-docs/
├── doc_transformer/              # Rust transformer binary
│   ├── src/
│   │   ├── main.rs              # Entry point
│   │   ├── discover.rs          # File discovery
│   │   ├── analyze.rs           # Metadata extraction
│   │   ├── assign.rs            # ID generation
│   │   ├── transform.rs         # Document transformation
│   │   ├── chunk.rs             # Semantic chunking
│   │   ├── graph.rs             # Knowledge DAG
│   │   ├── index.rs             # Indexing
│   │   └── validate.rs          # Validation
│   └── Cargo.toml
├── cue_docs/                     # Example: CUE documentation (36 files)
├── docs/
│   └── INDEXER.md               # Complete documentation
├── CLAUDE.md                     # AI development rules
└── README.md                     # This file
```

## Dependencies

**Core:**
- **petgraph** - Graph data structures (Knowledge DAG)
- **serde** / **serde_json** - Serialization
- **regex** - Pattern matching
- **walkdir** - Directory traversal
- **chrono** - Timestamps
- **clap** - CLI parsing
- **tokio** - Async runtime
- **anyhow** - Error handling

**v5.0 Web Scraping & Search:**
- **spider** - Web scraping with sitemap support
- **spider_transformations** - HTML to markdown conversion
- **url** - URL parsing and manipulation
- **scraper** - HTML parsing for content filtering
- **tantivy** - Full-text search engine with BM25
- **hnsw_rs** - Approximate nearest neighbor search
- **readability** - Content extraction (Mozilla algorithm)
- **pulldown-cmark** - Markdown parsing

## Version

**v5.0** - AI-Optimized Documentation Indexer with Web Scraping
- Web scraping via spider-rs with sitemap support
- Content filtering (BM25 + pruning algorithms)
- llms.txt generation for AI entry points
- Full-text search with Tantivy
- HNSW semantic similarity search
- CLI subcommands: scrape, index, ingest, search

## License

MIT
