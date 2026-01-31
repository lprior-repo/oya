---
id: indexer-implementation-guide
title: Indexer Implementation Guide
category: ref
tags: [indexer, implementation, doc-transformer, knowledge-dag, semantic-chunking]
---

# Indexer Implementation Guide

> **Context**: Complete technical guide to doc_transformer's indexer component, covering architecture, pipeline execution, code patterns, usage examples, and integration points. Implements Anthropic's Contextual Retrieval pattern with 7-step pipeline, knowledge graphs, and hierarchical chunking.

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Indexer Architecture](#indexer-architecture)
3. [7-Step Pipeline](#7-step-pipeline)
4. [Index Output Format](#index-output-format)
5. [Code Examples](#code-examples)
6. [Configuration & Tuning](#configuration--tuning)
7. [Integration Guide](#integration-guide)
8. [Troubleshooting](#troubleshooting)

---

## Quick Start

### Building the Indexer

```bash
cd doc_transformer
cargo build --release
```

### Basic Usage

```bash
# Index local markdown files
./target/release/doc_transformer index ./docs_source --output ./indexed_output

# Scrape and index a documentation website
./target/release/doc_transformer ingest https://example.com/docs --output ./indexed_output

# Search indexed documentation
./target/release/doc_transformer search "query terms" --index-dir ./indexed_output
```

### Output Files

```
indexed_output/
├── INDEX.json                    # Machine-readable index + knowledge graph
├── COMPASS.md                    # Human-readable navigation guide
├── llms.txt                      # AI entry point (read this first)
├── llms-full.txt                 # Extended entry point with full content
├── AGENTS.md                     # Agent integration guide
├── docs/                         # Transformed source documents
│   ├── document-name.md          # With YAML frontmatter
│   └── ...
└── chunks/                       # Semantic chunks for retrieval
    ├── doc-name-summary.md       # High-level summary (~128 tokens)
    ├── doc-name-standard.md      # Balanced detail (~512 tokens)
    ├── doc-name-detailed.md      # Full context (~1024 tokens)
    └── ...
```

---

## Indexer Architecture

### Core Components

The indexer is built on **hexagonal architecture** with functional Rust patterns:

```
PRESENTATION (main.rs)
    ↓
APPLICATION (transform, chunk, index, validate)
    ↓
PORTS (function signatures, data structures)
    ↓
ADAPTERS (discover, analyze, assign, graph)
    ↓
EXTERNAL SYSTEMS (file system, regex, petgraph, serde)
```

### Key Design Principles

1. **Zero Panics**: All errors are `Result<T, E>` or `Option<T>`
2. **Immutability**: Data transforms create new values, never mutate
3. **Pure Functions**: Business logic has no side effects
4. **Explicit Dependencies**: Injected via function parameters
5. **Railway-Oriented Programming**: Errors flow through `?` operator

### Data Structures

```rust
// Document metadata
pub struct Analysis {
    pub source_path: String,
    pub title: String,
    pub content: String,
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
    pub category: String,
    pub first_paragraph: String,
    pub word_count: usize,
}

// Semantic chunk with navigation
pub struct Chunk {
    pub chunk_id: String,              // "doc-name#0"
    pub doc_id: String,                // "doc-name"
    pub doc_title: String,
    pub content: String,
    pub token_count: usize,
    pub heading: Option<String>,       // Section heading
    pub chunk_type: String,            // "prose", "code", "table"
    pub previous_chunk_id: Option<String>,
    pub next_chunk_id: Option<String>,
    pub summary: String,
    pub chunk_level: ChunkLevel,       // Summary|Standard|Detailed
    pub parent_chunk_id: Option<String>,
    pub child_chunk_ids: Vec<String>,
}

// Knowledge DAG node
pub struct GraphNode {
    pub id: String,
    pub node_type: NodeType,           // Document|Chunk
    pub title: String,
    pub category: Option<String>,
}

// Knowledge DAG edge (relationship)
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,           // Sequential|Parent|Related|References
    pub weight: f32,                   // 0.0-1.0 strength
}
```

---

## 7-Step Pipeline

### Step 1: DISCOVER

**Purpose**: Find all markdown files in source directory

**Input**: Source directory path
**Output**: Vector of `DiscoveryFile` structs + manifest

```rust
pub fn discover_files(source_dir: &Path)
    -> Result<(Vec<DiscoveryFile>, DiscoverManifest)>
```

**Behavior**:
- Recursively walks directory tree
- Filters by extension: `.md`, `.mdx`, `.rst`, `.txt`
- Excludes: `node_modules`, `.git`, `_build`, `target`, `.github`
- Returns relative paths and file sizes

**Example**:
```
Input:  ~/docs/
        ├── guides/
        │   ├── getting-started.md
        │   └── advanced.md
        └── README.md

Output: [
  DiscoveryFile { source_path: "guides/getting-started.md", size_bytes: 4521 },
  DiscoveryFile { source_path: "guides/advanced.md", size_bytes: 8234 },
  DiscoveryFile { source_path: "README.md", size_bytes: 1243 }
]
```

---

### Step 2: ANALYZE

**Purpose**: Extract metadata from each document

**Input**: Files from DISCOVER
**Output**: Vector of `Analysis` structs

```rust
pub fn analyze_files(files: &[DiscoveryFile], source_dir: &Path)
    -> Result<Vec<Analysis>>
```

**Extracted Metadata**:
- **Title**: From H1 heading or filename
- **Headings**: All heading levels with text and line numbers
- **Links**: Internal and external links
- **Category**: Detected from filename and content
  - `tutorial` - Step-by-step guides
  - `concept` - Explanatory content
  - `ref` - Reference/API docs
  - `ops` - Operations/deployment
  - `meta` - README, CHANGELOG, etc.
- **First Paragraph**: For summaries
- **Word Count**: Total words
- **Content Features**: Code blocks, tables, etc.

**Example**:
```rust
Analysis {
    source_path: "guides/getting-started.md",
    title: "Getting Started",
    category: "tutorial",
    word_count: 1234,
    headings: vec![
        Heading { level: 1, text: "Getting Started", line: 1 },
        Heading { level: 2, text: "Installation", line: 3 },
        Heading { level: 2, text: "First Steps", line: 15 },
    ],
    links: vec![
        Link { text: "API Docs", url: "./api.md", line: 42 },
    ],
    first_paragraph: "Learn how to set up and start using...",
    // ...
}
```

---

### Step 3: ASSIGN IDs

**Purpose**: Generate unique, URL-safe document IDs

**Input**: Analyses from STEP 2
**Output**: Updated analyses + link map

```rust
pub fn assign_ids(analyses: Vec<Analysis>)
    -> (Vec<Analysis>, HashMap<String, IdMapping>)
```

**ID Strategy**:
- Slugify filename: convert to lowercase, replace `/` and `_` with `-`
- Remove `.md`, `.mdx`, `.rst` extensions
- Keep only alphanumeric and hyphens
- Example: `docs/getting-started.md` → `docs-getting-started`

**Link Map Usage**:
- Used in TRANSFORM step to rewrite internal links
- Maps source paths to document IDs and filenames
- Enables cross-references between documents

**Example**:
```
Input:  ["docs/getting-started.md", "api/reference.md"]
Output: {
    "docs/getting-started.md" => IdMapping {
        id: "docs-getting-started",
        filename: "docs_getting_started.md"
    },
    "api/reference.md" => IdMapping {
        id: "api-reference",
        filename: "api_reference.md"
    }
}
```

---

### Step 4: TRANSFORM

**Purpose**: Standardize documents with frontmatter and navigation

**Input**: Analyses + link map
**Output**: Documents in `docs/` directory

```rust
pub fn transform_all(
    analyses: &[Analysis],
    link_map: &HashMap<String, IdMapping>,
    output_dir: &Path,
) -> Result<TransformResult>
```

**Transformations Applied**:

1. **Fix Heading Hierarchy**: No skipped levels
2. **Ensure Single H1**: One title per document
3. **Rewrite Internal Links**: Update cross-references with new IDs
4. **Add YAML Frontmatter**:
   ```yaml
   ---
   id: docs-getting-started
   title: Getting Started
   category: tutorial
   tags: [setup, install, beginner]
   word_count: 1234
   ---
   ```
5. **Add Context Block**: Summary after H1
6. **Add See Also Section**: Related links before end

**Example Output**:
```markdown
---
id: docs-getting-started
title: Getting Started
category: tutorial
tags: [setup, install, beginner]
word_count: 1234
---

# Getting Started

> **Context**: Learn how to install and configure the system for your first project.

## Installation

[... content ...]

## See Also

- [API Reference](./api-reference.md)
- [Configuration Guide](./config-guide.md)
```

---

### Step 5: CHUNK (The Secret Sauce)

**Purpose**: Split documents into semantic chunks with context prefixes

**Input**: Analyses + output directory
**Output**: Chunk files in `chunks/` + chunk metadata

```rust
pub fn chunk_all(analyses: &[Analysis], output_dir: &Path)
    -> Result<ChunksResult>
```

**Chunking Strategy**:

1. **Boundary Detection**: Split on H2 headings (`## Title`)
2. **Hierarchical Levels**: Create 3 versions of each chunk
   - **Summary** (~128 tokens): High-level overview
   - **Standard** (~512 tokens): Balanced detail (most common)
   - **Detailed** (~1024 tokens): Full context
3. **Context Prefix**: Prepend trailing content from previous chunk
   - Summary: 30 tokens of context
   - Standard: 100 tokens of context
   - Detailed: 200 tokens of context
4. **Navigation Links**: Each chunk knows previous/next
5. **Hierarchical References**: Standard chunks have summary parents, detailed children

**Token Estimation**:
```rust
fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)  // ~4 chars = 1 token
}
```

**Chunk Metadata**:
```yaml
---
doc_id: docs-getting-started
chunk_id: docs-getting-started#0
chunk_level: standard
chunk_type: prose
heading: Installation
token_count: 512
summary: "Learn how to install the system on your machine"
previous_chunk_id: null
next_chunk_id: docs-getting-started#1
parent_chunk_id: docs-getting-started#0-summary
child_chunk_ids: [docs-getting-started#0-detailed]
---
```

**Why This Works**:
- **Context Available**: AI reads chunk and has preceding context
- **Self-Contained**: No need to fetch parent/sibling chunks separately
- **Multi-turn Conversations**: Can flow naturally across chunks
- **35% Better RAG**: Reduces retrieval failures (Anthropic research)

**Example Chunk Navigation**:
```
Document: docs-getting-started

Summary Level:
  #0-summary (128 tokens)

Standard Level:
  #0 (512 tokens) ← has context from end of doc
  #1 (512 tokens) ← has context from #0
  #2 (512 tokens) ← has context from #1

Detailed Level:
  #0-detailed (1024 tokens)
  #1-detailed (1024 tokens)
```

---

### Step 6: INDEX

**Purpose**: Build searchable index and knowledge graph

**Input**: Analyses + link map + chunks
**Output**: `INDEX.json` + `COMPASS.md`

```rust
pub fn build_and_write_index(
    analyses: &[Analysis],
    link_map: &HashMap<String, IdMapping>,
    chunks_result: &ChunksResult,
    output_dir: &Path,
) -> Result<()>
```

**Index Structure** (in `INDEX.json`):

```json
{
  "version": "5.0",
  "generated": "2026-01-11T15:30:45Z",
  "stats": {
    "doc_count": 36,
    "chunk_count": 156,
    "avg_chunk_size_tokens": 512,
    "graph": {
      "node_count": 192,
      "edge_count": 450,
      "sequential_edges": 120,
      "related_edges": 250,
      "reference_edges": 80
    }
  },
  "documents": [
    {
      "id": "docs-getting-started",
      "title": "Getting Started",
      "path": "docs/docs_getting_started.md",
      "category": "tutorial",
      "tags": ["setup", "install", "beginner"],
      "summary": "Learn how to install...",
      "word_count": 1234,
      "chunk_ids": ["docs-getting-started#0", "docs-getting-started#1"]
    }
  ],
  "chunks": [
    {
      "chunk_id": "docs-getting-started#0",
      "doc_id": "docs-getting-started",
      "doc_title": "Getting Started",
      "heading": "Installation",
      "chunk_type": "prose",
      "token_count": 512,
      "summary": "Learn how to install...",
      "previous_chunk_id": null,
      "next_chunk_id": "docs-getting-started#1",
      "path": "chunks/docs-getting-started-0-standard.md",
      "related_chunks": [
        { "chunk_id": "api-reference#0", "similarity": 0.65 },
        { "chunk_id": "troubleshooting#3", "similarity": 0.52 }
      ],
      "chunk_level": "standard",
      "parent_chunk_id": "docs-getting-started#0-summary",
      "child_chunk_ids": ["docs-getting-started#0-detailed"]
    }
  ],
  "keywords": {
    "installation": ["docs-getting-started"],
    "setup": ["docs-getting-started", "config-guide"],
    "api": ["api-reference", "api-advanced"]
  },
  "graph": {
    "nodes": [
      { "id": "docs-getting-started", "type": "document", "title": "..." },
      { "id": "docs-getting-started#0", "type": "chunk", "title": "..." }
    ],
    "edges": [
      { "from": "docs-getting-started", "to": "docs-getting-started#0", "type": "parent", "weight": 1.0 },
      { "from": "docs-getting-started#0", "to": "docs-getting-started#1", "type": "sequential", "weight": 1.0 },
      { "from": "docs-getting-started#0", "to": "api-reference#0", "type": "related", "weight": 0.65 }
    ],
    "topological_order": ["docs-getting-started", "docs-getting-started#0", ...],
    "reachability": {
      "docs-getting-started": ["docs-getting-started#0", "docs-getting-started#1", ...]
    },
    "node_importance": {
      "docs-getting-started": 2.5,
      "docs-getting-started#0": 1.65
    }
  },
  "navigation": {
    "type": "contextual_retrieval_with_dag",
    "strategy": "50-100 token context prefix + H2 boundaries + knowledge DAG",
    "avg_tokens_per_chunk": 512,
    "graph_enabled": true,
    "similarity_metric": "jaccard_on_tags_and_category",
    "min_similarity_threshold": 0.3
  }
}
```

**Knowledge DAG Features**:

- **Nodes**: Documents and chunks with metadata
- **Edges**: Relationships between nodes
  - `parent`: Document contains chunk
  - `sequential`: Navigation between chunks
  - `related`: Semantic similarity (Jaccard on tags)
  - `references`: Explicit links
- **Graph Algorithms**:
  - Topological ordering for dependency resolution
  - Reachability analysis for context expansion
  - Node importance scoring (sum of outgoing weights)

**Compass Navigation**:

```markdown
# Documentation Compass

> **36 documents**

## TUTORIAL

- [Getting Started](./docs/docs_getting_started.md) `setup` `install`
- [Advanced Usage](./docs/advanced_usage.md) `advanced` `configuration`

## CONCEPT

- [Architecture Overview](./docs/architecture.md) `design` `patterns`

## REF

- [API Reference](./docs/api_reference.md) `api` `reference`

[... more categories ...]
```

---

### Step 7: VALIDATE

**Purpose**: Verify quality standards

**Input**: Output directory from previous steps
**Output**: Validation report

```rust
pub fn validate_all(output_dir: &Path)
    -> Result<ValidationResult>
```

**Checks Performed**:

1. **Documents** (`docs/` directory):
   - ✅ Exactly one H1 per document
   - ✅ Valid YAML frontmatter with required fields
   - ✅ No skipped heading levels
   - ✅ At least 3 tags
   - ✅ Has context block
   - ✅ Has See Also section

2. **Chunks** (`chunks/` directory):
   - ✅ Chunk files exist for all chunk IDs in INDEX.json
   - ✅ Valid frontmatter in chunk files
   - ✅ Navigation links point to existing chunks
   - ✅ Token counts are reasonable

3. **Index** (`INDEX.json`):
   - ✅ Valid JSON structure
   - ✅ All document IDs are unique
   - ✅ All chunk IDs are unique
   - ✅ Chunk IDs in documents exist in chunks array
   - ✅ Graph edges reference existing nodes

**Example Report**:
```
[VALIDATE] Checking 36 documents
  Docs with valid frontmatter: 34/36 ✓
  Docs with H1: 36/36 ✓
  Docs with tags: 35/36 ⚠

[VALIDATE] Checking 156 chunks
  Chunks with valid frontmatter: 156/156 ✓
  Chunks with navigation: 154/156 ⚠

[VALIDATE] Checking INDEX.json
  Document count: 36 ✓
  Chunk count: 156 ✓
  Orphaned chunks: 0 ✓

Results: 156/192 files passed (35 errors, 2 warnings)
```

---

## Index Output Format

### INDEX.json Schema

```typescript
interface Index {
  version: string;
  generated: string;  // ISO 8601 timestamp
  stats: {
    doc_count: number;
    chunk_count: number;
    avg_chunk_size_tokens: number;
    graph: {
      node_count: number;
      edge_count: number;
      sequential_edges: number;
      related_edges: number;
      reference_edges: number;
    };
  };
  documents: IndexDocument[];
  chunks: ChunkMetadata[];
  keywords: Record<string, string[]>;
  graph: {
    nodes: GraphNode[];
    edges: GraphEdge[];
    topological_order: string[];
    reachability: Record<string, string[]>;
    node_importance: Record<string, number>;
    statistics: GraphStatistics;
  };
  navigation: {
    type: string;
    strategy: string;
    avg_tokens_per_chunk: number;
    graph_enabled: boolean;
    similarity_metric: string;
    min_similarity_threshold: number;
  };
}

interface IndexDocument {
  id: string;
  title: string;
  path: string;
  category: string;
  tags: string[];
  summary: string;
  word_count: number;
  chunk_ids: string[];
}

interface ChunkMetadata {
  chunk_id: string;
  doc_id: string;
  doc_title: string;
  heading: string | null;
  chunk_type: "prose" | "code" | "table";
  token_count: number;
  summary: string;
  previous_chunk_id: string | null;
  next_chunk_id: string | null;
  path: string;
  related_chunks: RelatedChunk[];
  chunk_level: "summary" | "standard" | "detailed";
  parent_chunk_id: string | null;
  child_chunk_ids: string[];
}

interface RelatedChunk {
  chunk_id: string;
  similarity: number;  // 0.0-1.0
}

interface GraphNode {
  id: string;
  node_type: "document" | "chunk";
  title: string;
  category?: string;
}

interface GraphEdge {
  from: string;
  to: string;
  edge_type: EdgeType;
  weight: number;  // 0.0-1.0
}

type EdgeType = "sequential" | "parent" | "related" | "references" | "referenced_by" | "co_authored" | "hierarchical";
```

---

## Code Examples

### Example 1: Basic Indexing

```rust
use doc_transformer::{discover, analyze, assign, transform, chunk, index, validate};

fn main() -> anyhow::Result<()> {
    let source_dir = std::path::Path::new("./docs");
    let output_dir = std::path::Path::new("./indexed");

    // Step 1: Discover files
    let (files, _) = discover::discover_files(source_dir)?;
    println!("Found {} files", files.len());

    // Step 2: Analyze
    let analyses = analyze::analyze_files(&files, source_dir)?;
    println!("Analyzed {} documents", analyses.len());

    // Step 3: Assign IDs
    let (analyses, link_map) = assign::assign_ids(analyses);
    println!("Generated {} IDs", link_map.len());

    // Step 4: Transform
    let transform_result = transform::transform_all(&analyses, &link_map, output_dir)?;
    println!("Transformed {}", transform_result.success_count);

    // Step 5: Chunk
    let chunks_result = chunk::chunk_all(&analyses, output_dir)?;
    println!("Created {} chunks", chunks_result.total_chunks);

    // Step 6: Index
    index::build_and_write_index(&analyses, &link_map, &chunks_result, output_dir)?;
    println!("Built index");

    // Step 7: Validate
    let validation = validate::validate_all(output_dir)?;
    println!("Validation: {}/{} passed", validation.files_passed, validation.files_checked);

    Ok(())
}
```

### Example 2: Reading the Index

```rust
use serde_json::json;
use std::fs;

fn main() -> anyhow::Result<()> {
    // Load INDEX.json
    let content = fs::read_to_string("./indexed/INDEX.json")?;
    let index: serde_json::Value = serde_json::from_str(&content)?;

    // Find documents by category
    let docs = index["documents"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter(|doc| doc["category"].as_str() == Some("tutorial"))
        .collect::<Vec<_>>();

    println!("Found {} tutorials", docs.len());

    // Get chunks for a document
    if let Some(doc) = docs.first() {
        if let Some(chunk_ids) = doc["chunk_ids"].as_array() {
            println!("Document has {} chunks", chunk_ids.len());
        }
    }

    // Search for keywords
    let keywords = index["keywords"]
        .as_object()
        .unwrap_or(&json!({}).as_object().unwrap());

    if let Some(docs_with_api) = keywords.get("api") {
        println!("Found {} documents mentioning 'api'", docs_with_api.as_array().map(|a| a.len()).unwrap_or(0));
    }

    // Traverse knowledge graph
    let nodes = index["graph"]["nodes"]
        .as_array()
        .unwrap_or(&vec![]);

    let edges = index["graph"]["edges"]
        .as_array()
        .unwrap_or(&vec![]);

    println!("Graph: {} nodes, {} edges", nodes.len(), edges.len());

    Ok(())
}
```

### Example 3: Searching with BM25

```rust
fn main() -> anyhow::Result<()> {
    let query = "how to install";
    let index_dir = std::path::Path::new("./indexed");

    // Note: doc_transformer has built-in BM25 search
    // Run: ./doc_transformer search "how to install" --index-dir ./indexed

    // Or implement custom search:
    let content = std::fs::read_to_string(index_dir.join("INDEX.json"))?;
    let index: serde_json::Value = serde_json::from_str(&content)?;

    let documents = index["documents"].as_array().unwrap();

    let mut scored_docs: Vec<_> = documents
        .iter()
        .filter_map(|doc| {
            let title = doc["title"].as_str().unwrap_or("");
            let summary = doc["summary"].as_str().unwrap_or("");
            let searchable = format!("{} {}", title, summary);

            // Simple BM25 calculation (pseudo-code)
            let score = bm25_score(&searchable, query, 100.0);
            if score > 0.0 {
                Some((score, doc))
            } else {
                None
            }
        })
        .collect();

    scored_docs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    for (score, doc) in scored_docs.iter().take(5) {
        println!("{}: {} (score: {:.2})",
            doc["id"],
            doc["title"],
            score);
    }

    Ok(())
}

fn bm25_score(text: &str, query: &str, avg_length: f32) -> f32 {
    // Simplified BM25 implementation
    let query_words: Vec<&str> = query.split_whitespace().collect();
    let text_lower = text.to_lowercase();

    query_words
        .iter()
        .filter(|word| text_lower.contains(word))
        .count() as f32 / query_words.len() as f32
}
```

### Example 4: Working with Chunks

```rust
use std::fs;
use serde_yaml;

fn main() -> anyhow::Result<()> {
    let chunk_path = "./indexed/chunks/docs-getting-started-0-standard.md";
    let content = fs::read_to_string(chunk_path)?;

    // Parse frontmatter
    if let Some(fm_end) = content.find("\n---\n") {
        let fm_str = &content[4..fm_end];  // Skip opening ---
        let _frontmatter: serde_yaml::Value = serde_yaml::from_str(fm_str)?;

        let chunk_content = &content[fm_end + 5..];

        println!("Chunk content length: {} chars", chunk_content.len());

        // Process chunk content
        process_chunk_content(chunk_content)?;
    }

    Ok(())
}

fn process_chunk_content(content: &str) -> anyhow::Result<()> {
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("## ") {
            println!("Line {}: Heading: {}", i, line);
        } else if line.starts_with("```") {
            println!("Line {}: Code block", i);
        } else if line.contains("|") && line.contains("-") {
            println!("Line {}: Table", i);
        }
    }

    Ok(())
}
```

### Example 5: Graph Traversal

```rust
fn main() -> anyhow::Result<()> {
    let content = std::fs::read_to_string("./indexed/INDEX.json")?;
    let index: serde_json::Value = serde_json::from_str(&content)?;

    // Find all chunks reachable from a document
    let doc_id = "docs-getting-started";
    let reachability = &index["graph"]["reachability"];

    if let Some(reachable) = reachability.get(doc_id).and_then(|v| v.as_array()) {
        println!("From {}, can reach {} nodes:", doc_id, reachable.len());
        for node_id in reachable.iter().take(5) {
            println!("  - {}", node_id);
        }
    }

    // Find most important nodes
    let importance = &index["graph"]["node_importance"];

    let mut scored: Vec<_> = importance
        .as_object()
        .unwrap_or(&serde_json::Map::new())
        .iter()
        .filter_map(|(id, score)| {
            score.as_f64().map(|s| (s, id.clone()))
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    println!("\nMost important nodes:");
    for (score, id) in scored.iter().take(5) {
        println!("  {}: {:.2}", id, score);
    }

    Ok(())
}
```

---

## Configuration & Tuning

### Token Estimation

Current implementation uses simple character-based estimation:

```rust
fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)  // 4 chars ≈ 1 token
}
```

**To adjust**: Edit `doc_transformer/src/chunk.rs`

**For better accuracy**, implement BPE tokenizer:
```rust
// Future enhancement
use tokenizers::Tokenizer;

fn estimate_tokens_bpe(text: &str) -> usize {
    // Use HuggingFace tokenizers crate
    let tokenizer = Tokenizer::from_pretrained("gpt2", None)?;
    tokenizer.encode(text, false)?.len()
}
```

### Chunk Size Targets

```rust
pub enum ChunkLevel {
    Summary,   // 128 tokens (~512 chars)
    Standard,  // 512 tokens (~2KB)
    Detailed,  // 1024 tokens (~4KB)
}
```

**To adjust chunk sizes**, modify `chunk.rs`:

```rust
impl ChunkLevel {
    pub fn target_tokens(&self) -> usize {
        match self {
            ChunkLevel::Summary => 128,    // Change this
            ChunkLevel::Standard => 512,   // Change this
            ChunkLevel::Detailed => 1024,  // Change this
        }
    }
}
```

### Context Prefix Sizes

```rust
let context_tokens = match level {
    ChunkLevel::Summary => 30,    // Pre-context for summary chunks
    ChunkLevel::Standard => 100,  // Pre-context for standard chunks
    ChunkLevel::Detailed => 200,  // Pre-context for detailed chunks
};
```

**To adjust context sizes**, modify `chunk.rs::create_chunks_at_level()`

### Semantic Boundaries

Default: Split on H2 headings (`## Heading`)

**To split on H3 instead**:
```rust
static H2_REGEX: Lazy<Regex> = Lazy::new(||
    Regex::new(r"^### (.+)$").expect("valid H3 regex")  // Changed H2 to H3
);
```

### Similarity Threshold for Related Chunks

```rust
// In index.rs::build_knowledge_dag()
let detector = RelationshipDetector::new(0.3);  // Change threshold

// Detected relationships must have Jaccard similarity >= 0.3
```

**Lower threshold** (e.g., 0.1): More related chunks, lower precision
**Higher threshold** (e.g., 0.5): Fewer related chunks, higher precision

### Category Detection

```rust
fn detect_category(filename: &str, content: &str) -> String {
    // Customize logic in analyze.rs
    // Currently checks filename patterns and content features
    // Possible categories: tutorial, concept, ref, ops, meta
}
```

---

## Integration Guide

### Using INDEX.json in Applications

```javascript
// JavaScript / Node.js example

async function searchDocumentation(query) {
    const index = require('./indexed/INDEX.json');

    // Method 1: Keyword search
    const docs = index.keywords[query.toLowerCase()] || [];

    // Method 2: Document metadata search
    const matches = index.documents.filter(doc =>
        doc.title.includes(query) || doc.summary.includes(query)
    );

    return matches;
}

async function getChunkContent(chunkId) {
    // Chunks are stored as markdown files
    // Path is in INDEX.json[chunks[i].path]
    const index = require('./indexed/INDEX.json');
    const chunkMeta = index.chunks.find(c => c.chunk_id === chunkId);

    if (!chunkMeta) return null;

    const content = require('fs').readFileSync(chunkMeta.path, 'utf-8');
    return content;
}

async function getRelatedChunks(chunkId) {
    const index = require('./indexed/INDEX.json');
    const chunk = index.chunks.find(c => c.chunk_id === chunkId);

    if (!chunk) return [];

    // Use knowledge graph to find related content
    const related = chunk.related_chunks;  // Pre-computed

    return related.map(r => ({
        chunk_id: r.chunk_id,
        similarity: r.similarity,
        chunk: index.chunks.find(c => c.chunk_id === r.chunk_id)
    }));
}
```

### Using llms.txt for AI Agents

The `llms.txt` file is an AI entry point containing:

```
# Project Documentation Index

> Read this file to understand the documentation structure

## Overview
[Auto-generated project summary]

## Quick Links
[Top 10 most important documents]

## File Structure
[List of major sections]

---

## Full Index

[Comprehensive list of all documents with descriptions]
```

**Why use llms.txt?**
- Provides context to Claude about the documentation
- Lists documents in priority order
- Explains navigation structure
- Read by AI agent before fetching specific chunks

### Building RAG Systems

```python
# Python example: Building a RAG system with the indexed docs

import json
import os
from pathlib import Path

class DocumentationRAG:
    def __init__(self, index_path):
        with open(index_path) as f:
            self.index = json.load(f)
        self.base_dir = Path(index_path).parent

    def search(self, query: str, limit: int = 5):
        """Simple BM25-style search"""
        results = []

        # Score documents
        for doc in self.index['documents']:
            title_score = 2.0 if query.lower() in doc['title'].lower() else 0
            summary_score = 1.0 if query.lower() in doc['summary'].lower() else 0

            total_score = title_score + summary_score

            if total_score > 0:
                results.append({
                    'doc_id': doc['id'],
                    'title': doc['title'],
                    'score': total_score,
                    'chunks': doc['chunk_ids']
                })

        results.sort(key=lambda x: x['score'], reverse=True)
        return results[:limit]

    def get_chunk_content(self, chunk_id: str) -> str:
        """Load chunk content from disk"""
        chunk_meta = next((c for c in self.index['chunks'] if c['chunk_id'] == chunk_id), None)
        if not chunk_meta:
            return ""

        chunk_path = self.base_dir / chunk_meta['path']
        with open(chunk_path) as f:
            return f.read()

    def build_context(self, query: str, max_tokens: int = 2000) -> str:
        """Build context for LLM prompt"""
        search_results = self.search(query, limit=3)

        context_parts = []
        current_tokens = 0

        for result in search_results:
            for chunk_id in result['chunks']:
                chunk_content = self.get_chunk_content(chunk_id)
                chunk_tokens = len(chunk_content.split()) * 1.3  # Rough estimate

                if current_tokens + chunk_tokens > max_tokens:
                    break

                context_parts.append(f"## {result['title']}\n\n{chunk_content}")
                current_tokens += chunk_tokens

        return "\n\n---\n\n".join(context_parts)

# Usage
rag = DocumentationRAG("indexed/INDEX.json")
context = rag.build_context("how to install")
print(context)
```

---

## Troubleshooting

### Issue: "Chunk navigation links broken"

**Symptom**: `previous_chunk_id` or `next_chunk_id` point to non-existent chunks

**Cause**: Chunks at different hierarchical levels have different IDs

**Solution**:
- Standard chunks only link to other standard chunks
- Don't mix standard/summary/detailed level navigation
- Check chunk_level field in INDEX.json

### Issue: "Low similarity scores in related_chunks"

**Symptom**: No related chunks found (empty `related_chunks` array)

**Cause**: Jaccard similarity threshold too high (default: 0.3)

**Solution**: Lower similarity threshold in `index.rs`:
```rust
let detector = RelationshipDetector::new(0.1);  // Lower from 0.3
```

### Issue: "Categories not being detected correctly"

**Symptom**: Documents marked as wrong category

**Cause**: Category detection logic based on filename patterns

**Solution**: Customize in `analyze.rs::detect_category()`:
```rust
fn detect_category(filename: &str, content: &str) -> String {
    // Add your custom rules here
    // Or add category to frontmatter in source docs
}
```

### Issue: "Token counts seem wrong"

**Symptom**: Chunks have too many/few tokens than expected

**Cause**: Simple 4-char-per-token estimation is approximate

**Solution**:
1. Use actual tokenizer (see Configuration section)
2. Or adjust divisor in `estimate_tokens()`:
   ```rust
   (text.len() / 3).max(1)  // 3 chars/token (shorter chunks)
   (text.len() / 5).max(1)  // 5 chars/token (longer chunks)
   ```

### Issue: "Graph edges pointing to invalid nodes"

**Symptom**: Validation error about graph integrity

**Cause**: Missing nodes when building DAG

**Solution**:
- Run validation with verbose output
- Check that all documents and chunks are added as nodes
- Verify edge_type enum values are valid

### Issue: "Some documents not being indexed"

**Symptom**: Documents in source directory missing from INDEX.json

**Cause**: Could be several reasons:

```
1. Discovery phase filtered them:
   - Wrong file extension (only .md, .mdx, .rst, .txt)
   - In excluded directory (node_modules, .git, etc.)

2. Analysis failed:
   - Invalid markdown structure
   - Missing or multiple H1 headings

3. Transformation failed:
   - ID assignment error
   - File system write error
```

**Solution**: Check logs for each step:
```bash
# Run with explicit error logging
./doc_transformer index ./source --output ./indexed --verbose

# Check specific file
cat ./indexed/docs/problematic-doc.md  # See if it was created
```

---

## Performance Considerations

### Large Document Sets

For 1000+ documents, consider:

1. **Parallel Processing**: Modify `analyze.rs` to use `rayon`:
   ```rust
   use rayon::prelude::*;

   let analyses = files
       .par_iter()  // Parallel iterator
       .filter_map(|f| analyze_single_file(&f).ok())
       .collect::<Vec<_>>();
   ```

2. **Incremental Indexing**: Only re-process changed files
   - Track file modification times
   - Skip unchanged documents

3. **Graph Optimization**: Cache similarity computations
   - Pre-compute related chunks once
   - Store in persistent cache

### Memory Usage

- **INDEX.json size**: ~1KB per document, ~100 bytes per chunk
- **Runtime memory**: ~10MB per 100 documents
- **Disk storage**: ~5KB per chunk (depends on content size)

For 1000 documents with 3000 chunks:
- INDEX.json: ~1MB
- Chunks directory: ~15MB
- Total: ~20MB

---

## See Also

- [Indexer: AI-Optimized Documentation Transform](./INDEXER.md) - High-level overview
- [Architecture: Hexagonal Design with Functional Rust](./ARCHITECTURE.md) - System design
- [Contextual Retrieval (Anthropic)](https://www.anthropic.com) - Research basis

---

**Document ID**: `indexer-implementation-guide`
**Category**: Reference
**Tags**: indexer, implementation, guide, knowledge-dag, semantic-chunking, rust, doc-transformer
**Last Updated**: 2026-01-11
**Version**: 1.0
