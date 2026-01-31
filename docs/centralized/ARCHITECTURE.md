# Architecture: Hexagonal Design with Functional Rust

> **Context**: This document defines the hexagonal (ports and adapters) architecture pattern used in the doc_transformer project, implemented in Rust with functional programming principles, zero-panic guarantees, and Railway-Oriented Programming.

---

## Table of Contents

1. [Overview](#overview)
2. [Hexagonal Architecture Layers](#hexagonal-architecture-layers)
3. [Functional Programming Patterns](#functional-programming-patterns)
4. [Data Flow and Pipeline](#data-flow-and-pipeline)
5. [Module Structure](#module-structure)
6. [Error Handling Strategy](#error-handling-strategy)
7. [Testing Strategy](#testing-strategy)
8. [Design Principles](#design-principles)
9. [Examples from Codebase](#examples-from-codebase)

---

## Overview

The doc_transformer project follows **Hexagonal Architecture** (also known as Ports and Adapters), which separates core business logic from external dependencies. This enables:

- **Testability**: Core logic can be tested without I/O dependencies
- **Maintainability**: Clear separation of concerns with explicit boundaries
- **Flexibility**: Easy to swap implementations (file system, databases, APIs)
- **Type Safety**: Rust's type system enforces architectural boundaries at compile time

### Key Architectural Principles

1. **Zero Panics**: All error paths are explicit via `Result<T, E>` and `Option<T>`
2. **Immutability**: Data structures are immutable; transformations return new values
3. **Pure Functions**: Business logic is deterministic with no side effects
4. **Explicit Dependencies**: All dependencies are injected, not instantiated internally
5. **Railway-Oriented Programming**: Error handling flows through `Result` chains

---

## Hexagonal Architecture Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                        PRESENTATION LAYER                        │
│                   (main.rs, CLI Commands)                        │
│                                                                   │
│  Responsibilities:                                                │
│  • Parse command-line arguments (clap)                           │
│  • Coordinate high-level workflow                                │
│  • Display results to user                                       │
│  • Handle async runtime (tokio)                                  │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                       APPLICATION LAYER                          │
│               (Orchestration & Business Logic)                   │
│                                                                   │
│  Modules: transform, chunk, index, validate, search              │
│                                                                   │
│  Responsibilities:                                                │
│  • Orchestrate multi-step transformations                        │
│  • Implement pure business logic                                 │
│  • Compose operations via Result chains                          │
│  • Define workflow algorithms                                    │
│                                                                   │
│  Pattern: Pure functions returning Result<T, E>                  │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                          PORTS LAYER                             │
│                    (Trait Definitions)                           │
│                                                                   │
│  Defined implicitly via:                                         │
│  • Public function signatures                                    │
│  • Data structures (Analysis, Chunk, GraphNode)                  │
│  • Result types that define contracts                            │
│                                                                   │
│  In Rust: Ports are the module boundaries and public APIs        │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                        ADAPTERS LAYER                            │
│              (External System Integrations)                      │
│                                                                   │
│  Modules: discover, analyze, assign, graph                       │
│                                                                   │
│  Responsibilities:                                                │
│  • File system I/O (walkdir, std::fs)                           │
│  • Regex matching (once_cell lazy statics)                      │
│  • Graph algorithms (petgraph)                                   │
│  • JSON serialization (serde_json)                               │
│  • Content hashing (sha2)                                        │
│                                                                   │
│  Pattern: Convert external data to/from domain types             │
└─────────────────────────────────────────────────────────────────┘
```

### Layer Definitions

#### 1. Presentation Layer (`main.rs`)

**Purpose**: Entry point and CLI interface

**Responsibilities**:
- Parse CLI arguments using `clap`
- Dispatch to appropriate command handlers
- Format and display output to users
- Manage async runtime (`tokio::main`)

**Example**:
```rust
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Commands::Transform { source_dir, output_dir, incremental, force, verbose } => {
            run_transform(&source_dir, &output_dir, incremental, force, verbose).await
        }
        Commands::Search { query, index_dir, limit, chunks } => {
            run_search(&query, &index_dir, limit, chunks)
        }
        Commands::Graph { node_id, index_dir, reachable } => {
            run_graph(&node_id, &index_dir, reachable)
        }
    }
}
```

**Key Characteristics**:
- No business logic
- Thin coordination layer
- Delegates to Application layer

---

#### 2. Application Layer (Core Business Logic)

**Purpose**: Orchestrate multi-step transformations with pure functions

**Modules**:
- `transform.rs`: Document transformation pipeline
- `chunk.rs`: Semantic chunking algorithm
- `index.rs`: Index and graph building
- `validate.rs`: Validation rules and checks
- `search.rs`: Search implementation

**Responsibilities**:
- Implement domain algorithms
- Compose operations into workflows
- Transform data between stages
- Enforce business rules

**Pattern**: Pure functions with explicit error handling

```rust
pub fn transform_all(
    analyses: &[Analysis],
    link_map: &HashMap<String, IdMapping>,
    output_dir: &Path,
) -> Result<TransformResult> {
    let docs_dir = output_dir.join("docs");
    fs::create_dir_all(&docs_dir)?;

    let mut success_count = 0;
    let mut error_count = 0;

    for analysis in analyses {
        match link_map.get(&analysis.source_path) {
            Some(mapping) => {
                match transform_file(analysis, mapping, link_map, &docs_dir) {
                    Ok(_) => success_count += 1,
                    Err(e) => error_count += 1,
                }
            }
            None => { /* Handle missing mapping */ }
        }
    }

    Ok(TransformResult { success_count, error_count, ... })
}
```

**Key Characteristics**:
- No direct I/O (delegates to adapters)
- Immutable data structures
- Explicit error propagation via `?` operator
- Returns structured results

---

#### 3. Ports Layer (Interfaces)

**Purpose**: Define contracts between layers

In Rust, ports are implicitly defined through:

1. **Public Function Signatures**: The API contract
   ```rust
   pub fn discover_files(source_dir: &Path) -> Result<(Vec<DiscoveryFile>, DiscoverManifest)>
   pub fn analyze_files(files: &[DiscoveryFile], source_dir: &Path) -> Result<Vec<Analysis>>
   ```

2. **Data Structures**: The domain model
   ```rust
   pub struct Analysis {
       pub source_path: String,
       pub title: String,
       pub headings: Vec<Heading>,
       pub links: Vec<Link>,
       pub category: String,
       // ...
   }
   ```

3. **Result Types**: Success/failure contracts
   ```rust
   Result<TransformResult, anyhow::Error>
   ```

**Ports in this Architecture**:
- **Discovery Port**: `discover_files()`
- **Analysis Port**: `analyze_files()`
- **ID Assignment Port**: `assign_ids()`
- **Transform Port**: `transform_all()`
- **Chunking Port**: `chunk_all()`
- **Indexing Port**: `build_and_write_index()`
- **Validation Port**: `validate_all()`
- **Search Port**: `search_documents()`, `search_chunks()`

---

#### 4. Adapters Layer (External Systems)

**Purpose**: Implement ports by interacting with external systems

**Adapters**:

| Adapter Module | External System | Responsibility |
|----------------|-----------------|----------------|
| `discover.rs` | File System | Walk directories, filter by extension |
| `analyze.rs` | Regex Engine | Parse markdown, extract metadata |
| `assign.rs` | Hashing | Generate content-addressed IDs |
| `graph.rs` | Graph Library (petgraph) | Build and query knowledge DAG |
| `index.rs` | JSON Serialization | Write INDEX.json |
| `validate.rs` | File System | Check output files |
| `search.rs` | Text Search | BM25-style ranking |

**Example Adapter**: `discover.rs`

```rust
pub fn discover_files(source_dir: &Path) -> Result<(Vec<DiscoveryFile>, DiscoverManifest)> {
    if !source_dir.exists() {
        anyhow::bail!("Source directory not found: {}", source_dir.display());
    }

    let mut files = Vec::new();
    let extensions = [".md", ".mdx", ".rst", ".txt"];
    let exclude_dirs = ["node_modules", ".git", "_build"];

    for entry in WalkDir::new(source_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        // Skip excluded directories
        if exclude_dirs.iter().any(|excl| path.components().any(|c| { ... })) {
            continue;
        }

        if path.is_file() && has_valid_extension(path, &extensions) {
            files.push(DiscoveryFile {
                source_path: path.strip_prefix(source_dir)?.to_string_lossy().to_string(),
                size_bytes: path.metadata()?.len(),
            });
        }
    }

    Ok((files, manifest))
}
```

**Key Characteristics**:
- Encapsulates external dependencies
- Converts between external formats and domain types
- Handles I/O errors explicitly
- Isolates side effects

---

## Functional Programming Patterns

### 1. Immutability

All data structures are immutable by default. Transformations return new values.

```rust
// Immutable transformation
fn fix_headings(content: &str) -> String {
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    // ... transform lines ...
    lines.join("\n")  // Return new String
}
```

### 2. No Null/Nil - Use Option<T>

```rust
pub struct Chunk {
    pub heading: Option<String>,          // May or may not have heading
    pub previous_chunk_id: Option<String>, // First chunk has no previous
    pub next_chunk_id: Option<String>,     // Last chunk has no next
}
```

### 3. Explicit Error Handling - Result<T, E>

Every fallible operation returns `Result`:

```rust
pub fn analyze_single_file(source_path: &str, file_path: &Path) -> Result<Analysis> {
    let content = fs::read_to_string(file_path)?;  // ? operator propagates errors
    let title = extract_title(&content, source_path);
    let headings = extract_headings(&content);

    Ok(Analysis {
        source_path: source_path.to_string(),
        title,
        headings,
        // ...
    })
}
```

### 4. Railway-Oriented Programming

Chain operations where errors propagate automatically:

```rust
pub fn transform_all(...) -> Result<TransformResult> {
    let docs_dir = output_dir.join("docs");
    fs::create_dir_all(&docs_dir)?;  // Early return on error

    for analysis in analyses {
        match link_map.get(&analysis.source_path) {
            Some(mapping) => {
                transform_file(analysis, mapping, link_map, &docs_dir)?;  // Propagate errors
            }
            None => { /* Handle missing case */ }
        }
    }

    Ok(result)
}
```

### 5. Pure Functions

Functions with no side effects (where possible):

```rust
fn detect_category(filename: &str, content: &str) -> String {
    let fname_lower = Path::new(filename)
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_lowercase();

    if matches!(fname_lower.as_str(), "readme" | "changelog" | "license") {
        return "meta".to_string();
    }

    // ... deterministic logic ...
    "concept".to_string()
}
```

**Characteristics**:
- Same input always produces same output
- No hidden state
- No I/O operations
- Testable without mocks

### 6. Lazy Static Initialization

Thread-safe singleton pattern for expensive resources:

```rust
use once_cell::sync::Lazy;
use regex::Regex;

static H1_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^# (.+)$").expect("H1 regex is valid")
});

// Used in functions without re-compilation
fn extract_title(content: &str) -> String {
    if let Some(cap) = H1_REGEX.captures_iter(content).next() {
        // ...
    }
}
```

---

## Data Flow and Pipeline

The transformation pipeline follows a strict sequence:

```
Input: Source Directory
         │
         ▼
┌────────────────────┐
│  1. DISCOVER       │  → Find all .md/.mdx/.rst files
│  (discover.rs)     │  → Filter excluded directories
└────────┬───────────┘  → Return Vec<DiscoveryFile>
         │
         ▼
┌────────────────────┐
│  2. ANALYZE        │  → Parse markdown content
│  (analyze.rs)      │  → Extract title, headings, links
└────────┬───────────┘  → Detect category (ref/concept/tutorial/ops)
         │              → Return Vec<Analysis>
         ▼
┌────────────────────┐
│  3. ASSIGN IDs     │  → Generate content-addressed IDs
│  (assign.rs)       │  → Create filename mappings
└────────┬───────────┘  → Build link_map for rewrites
         │              → Return (Vec<Analysis>, HashMap<String, IdMapping>)
         ▼
┌────────────────────┐
│  4. TRANSFORM      │  → Fix heading structure
│  (transform.rs)    │  → Rewrite internal links
└────────┬───────────┘  → Add frontmatter and context
         │              → Write to docs/ directory
         ▼
┌────────────────────┐
│  5. CHUNK          │  → Split on semantic boundaries (H2)
│  (chunk.rs)        │  → Add contextual prefixes
└────────┬───────────┘  → Generate chunk metadata
         │              → Write to chunks/ directory
         ▼
┌────────────────────┐
│  6. INDEX          │  → Build knowledge graph (DAG)
│  (index.rs)        │  → Create document index
└────────┬───────────┘  → Extract keywords
         │              → Write INDEX.json and COMPASS.md
         ▼
┌────────────────────┐
│  7. VALIDATE       │  → Check frontmatter
│  (validate.rs)     │  → Verify heading structure
└────────┬───────────┘  → Detect broken links
         │              → Return ValidationResult
         ▼
Output: Indexed Documentation
```

### Pipeline Execution

```rust
async fn run_transform(source_dir: &Path, output_dir: &Path, ...) -> Result<()> {
    // STEP 1: DISCOVER
    let (files, _manifest) = discover::discover_files(source_dir)?;

    // STEP 2: ANALYZE
    let analyses = analyze::analyze_files(&files, source_dir)?;

    // STEP 3: ASSIGN IDs
    let (analyses, link_map) = assign::assign_ids(analyses)?;

    // STEP 4: TRANSFORM
    let transform_result = transform::transform_all(&analyses, &link_map, output_dir)?;

    // STEP 5: CHUNK
    let chunks_result = chunk::chunk_all(&analyses, output_dir)?;

    // STEP 6: INDEX
    index::build_and_write_index(&analyses, &link_map, &chunks_result, output_dir)?;
    index::build_and_write_compass(&analyses, &link_map, output_dir)?;

    // STEP 7: VALIDATE
    let validation_result = validate::validate_all(output_dir)?;

    Ok(())
}
```

**Characteristics**:
- Each step is independent and composable
- Failures propagate via `?` operator
- State flows through function parameters (no global state)
- Easy to add new steps or reorder

---

## Module Structure

```
doc_transformer/
├── src/
│   ├── main.rs           # Presentation layer (CLI)
│   ├── lib.rs            # Public API exports
│   │
│   ├── discover.rs       # Adapter: File system discovery
│   ├── analyze.rs        # Adapter: Content analysis (regex)
│   ├── assign.rs         # Application: ID assignment logic
│   ├── transform.rs      # Application: Transformation pipeline
│   ├── chunk.rs          # Application: Semantic chunking
│   ├── graph.rs          # Adapter: Graph data structure (petgraph)
│   ├── index.rs          # Application: Index building
│   ├── validate.rs       # Application: Validation rules
│   ├── search.rs         # Application: Search implementation
│   └── incremental.rs    # Application: Incremental update logic
│
├── tests/
│   ├── discover_tests.rs
│   ├── analyze_tests.rs
│   ├── validate_tests.rs
│   ├── index_tests.rs
│   └── search_tests.rs
│
└── Cargo.toml            # Dependencies
```

### Module Dependencies

```
main.rs
  │
  ├─► discover.rs  (no internal deps)
  ├─► analyze.rs   (uses: discover types)
  ├─► assign.rs    (uses: analyze types)
  ├─► transform.rs (uses: analyze, assign types)
  ├─► chunk.rs     (uses: analyze types)
  ├─► graph.rs     (uses: petgraph, no internal deps)
  ├─► index.rs     (uses: analyze, assign, chunk, graph types)
  ├─► validate.rs  (uses: analyze types)
  └─► search.rs    (uses: index types)
```

**Dependency Rules**:
1. Adapters have no dependencies on other modules (only external crates)
2. Application modules depend on adapter types
3. Presentation depends on all modules
4. No circular dependencies

---

## Error Handling Strategy

### 1. Use `anyhow` for Application Errors

```rust
use anyhow::{Result, Context};

pub fn transform_file(...) -> Result<()> {
    let content = fs::read_to_string(path)
        .context("Failed to read file")?;  // Add context to errors

    // ... transformation logic ...

    Ok(())
}
```

### 2. Use `thiserror` for Domain Errors (if needed)

For structured error types with variants:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Missing frontmatter in {0}")]
    MissingFrontmatter(String),

    #[error("Invalid heading structure at line {line}")]
    InvalidHeading { line: usize },
}
```

### 3. Never Panic

Replace all `.expect()` and `.unwrap()` with proper error handling:

```rust
// BAD - Can panic
let title = caps.get(1).unwrap().as_str();

// GOOD - Returns None, caller handles
let title = caps.get(1).map(|m| m.as_str());

// BEST - Provides fallback
let title = caps.get(1)
    .map(|m| m.as_str())
    .unwrap_or("Untitled");
```

### 4. Validated Lazy Statics

Regex compilation is the only acceptable use of `.expect()` because patterns are compile-time constants:

```rust
static H1_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^# (.+)$").expect("H1 regex is valid")  // OK: pattern is hardcoded
});
```

---

## Testing Strategy

### Test Coverage Targets

| Layer | Coverage | Strategy |
|-------|----------|----------|
| Presentation | Minimal | Integration tests for CLI commands |
| Application | ≥90% | Unit tests with table-driven approach |
| Adapters | ≥85% | Integration tests with real I/O |

### 1. Unit Tests (Application Layer)

Test pure functions with multiple cases:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_category() {
        let cases = vec![
            ("README.md", "", "meta"),
            ("guide.md", "## Step 1\n## Step 2", "tutorial"),
            ("api.md", "## API Reference", "ref"),
            ("overview.md", "This explains...", "concept"),
        ];

        for (filename, content, expected) in cases {
            assert_eq!(detect_category(filename, content), expected);
        }
    }

    #[test]
    fn test_safe_truncate() {
        assert_eq!(safe_truncate("hello", 10), "hello");
        assert_eq!(safe_truncate("hello world", 5), "hello");
        // UTF-8 boundary safety
        assert_eq!(safe_truncate("hello 世界", 8), "hello ");
    }
}
```

### 2. Integration Tests (Adapters)

Test with real file system using `tempfile`:

```rust
#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    #[test]
    fn test_discover_files() {
        let temp = tempdir().unwrap();
        let source = temp.path();

        // Create test files
        fs::write(source.join("doc1.md"), "# Test").unwrap();
        fs::write(source.join("doc2.md"), "# Test").unwrap();

        let (files, manifest) = discover_files(source).unwrap();

        assert_eq!(files.len(), 2);
        assert_eq!(manifest.total_files, 2);
    }
}
```

### 3. Property-Based Testing (Future)

Use `proptest` for invariant testing:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn safe_truncate_never_panics(s: String, max: usize) {
        let _ = safe_truncate(&s, max);  // Should never panic
    }
}
```

---

## Design Principles

### 1. Separation of Concerns

Each module has a single, well-defined responsibility:

- `discover`: Find files
- `analyze`: Parse content
- `transform`: Rewrite documents
- `chunk`: Split into semantic units
- `graph`: Manage relationships
- `index`: Build search index
- `validate`: Check quality

### 2. Dependency Inversion

High-level modules (Application) don't depend on low-level details (Adapters). They depend on abstractions (function signatures, data structures).

```rust
// High-level module (transform.rs) depends on abstraction
pub fn transform_all(
    analyses: &[Analysis],  // Abstract data type
    link_map: &HashMap<String, IdMapping>,
    output_dir: &Path,
) -> Result<TransformResult>

// Low-level module (analyze.rs) implements abstraction
pub fn analyze_files(files: &[DiscoveryFile], ...) -> Result<Vec<Analysis>>
```

### 3. Single Trace Flow (Future Enhancement)

For observability, add trace context:

```rust
use tracing::{info, instrument};

#[instrument]
pub fn transform_all(...) -> Result<TransformResult> {
    info!("Starting transformation");
    // ...
    info!("Transformation complete");
    Ok(result)
}
```

### 4. Immutable Data Structures

All structs derive `Clone` and are passed by reference:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analysis {
    pub source_path: String,
    pub title: String,
    // ... all fields are owned, no &str
}
```

### 5. Type-Driven Design

Use Rust's type system to enforce correctness:

```rust
// Type alias for clarity
type LinkMap = HashMap<String, IdMapping>;

// Wrapper types for domain concepts
pub struct DocumentId(String);
pub struct ChunkId(String);

// Newtype pattern prevents mixing IDs
impl DocumentId {
    pub fn new(id: String) -> Self {
        DocumentId(id)
    }
}
```

---

## Examples from Codebase

### Example 1: Pure Transformation Function

```rust
/// Fix heading structure: no skipped levels, max level 4
fn fix_headings(content: &str) -> String {
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let heading_pattern = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();

    // Find all headings and their levels
    let mut heading_lines: Vec<(usize, usize)> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if let Some(caps) = heading_pattern.captures(line) {
            if let Some(level_match) = caps.get(1) {
                let level = level_match.as_str().len();
                heading_lines.push((i, level));
            }
        }
    }

    // Fix skipped levels (pure logic)
    for j in 1..heading_lines.len() {
        let prev_level = heading_lines[j - 1].1;
        let curr_level = heading_lines[j].1;
        let line_idx = heading_lines[j].0;

        if curr_level > prev_level + 1 {
            let new_level = prev_level + 1;
            let new_hashes = "#".repeat(new_level);
            let text = lines[line_idx].trim_start_matches('#').trim_start();
            lines[line_idx] = format!("{} {}", new_hashes, text);
        }
    }

    lines.join("\n")  // Return new String
}
```

**Characteristics**:
- Input: `&str`, Output: `String`
- No side effects
- Deterministic
- Testable in isolation

---

### Example 2: Adapter with Error Handling

```rust
pub fn discover_files(source_dir: &Path) -> Result<(Vec<DiscoveryFile>, DiscoverManifest)> {
    // Validate preconditions
    if !source_dir.exists() {
        anyhow::bail!("Source directory not found: {}", source_dir.display());
    }

    let mut files = Vec::new();
    let extensions = [".md", ".mdx", ".rst", ".txt"];

    // I/O operation with error handling
    for entry in WalkDir::new(source_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = format!(".{}", ext.to_string_lossy());
                if extensions.contains(&ext_str.as_str()) {
                    let rel_path = path.strip_prefix(source_dir)?
                        .to_string_lossy()
                        .to_string();
                    let size = path.metadata()?.len();

                    files.push(DiscoveryFile {
                        source_path: rel_path,
                        size_bytes: size,
                    });
                }
            }
        }
    }

    // Return structured result
    let manifest = DiscoverManifest {
        source_dir: source_dir.to_string_lossy().to_string(),
        discovered_at: chrono::Utc::now().to_rfc3339(),
        total_files: files.len(),
        files: files.clone(),
    };

    Ok((files, manifest))
}
```

**Characteristics**:
- Encapsulates file system I/O
- Validates input
- Returns domain types
- Propagates errors via `?`

---

### Example 3: Graph Data Structure (Adapter)

```rust
pub struct KnowledgeDAG {
    graph: DiGraph<GraphNode, GraphEdgeData>,  // petgraph dependency
    node_map: HashMap<String, NodeIndex>,
    pub nodes_vec: Vec<GraphNode>,
    pub edges_vec: Vec<GraphEdge>,
}

impl KnowledgeDAG {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
            nodes_vec: Vec::new(),
            edges_vec: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: GraphNode) {
        let idx = self.graph.add_node(node.clone());
        self.node_map.insert(node.id.clone(), idx);
        self.nodes_vec.push(node);
    }

    pub fn add_edge(&mut self, edge: GraphEdge) {
        if let (Some(&from_idx), Some(&to_idx)) =
            (self.node_map.get(&edge.from), self.node_map.get(&edge.to))
        {
            self.graph.add_edge(from_idx, to_idx, GraphEdgeData { ... });
            self.edges_vec.push(edge);
        }
    }

    pub fn topological_order(&self) -> Vec<String> {
        match toposort(&self.graph, None) {
            Ok(sorted) => sorted.into_iter()
                .filter_map(|idx| self.graph.node_weight(idx).map(|n| n.id.clone()))
                .collect(),
            Err(_) => Vec::new(),  // No panic on cycles
        }
    }
}
```

**Characteristics**:
- Encapsulates `petgraph` library
- Provides domain-specific API
- No panics (returns `Vec::new()` on error)
- Maintains internal consistency

---

### Example 4: Semantic Chunking (Application Logic)

```rust
pub fn chunk_all(analyses: &[Analysis], output_dir: &Path) -> Result<ChunksResult> {
    let chunks_dir = output_dir.join("chunks");
    fs::create_dir_all(&chunks_dir)?;

    let mut all_chunks = Vec::new();

    for analysis in analyses {
        let doc_id = slugify(&analysis.source_path);
        let chunks = create_chunks_smart(
            &analysis.content,
            &doc_id,
            &analysis.title,
            &analysis.source_path
        );

        for chunk in chunks {
            all_chunks.push(chunk);
        }
    }

    // Add navigation links
    link_chunks(&mut all_chunks);

    // Write to disk (side effect isolated to end)
    for chunk in &all_chunks {
        let chunk_filename = format!("{}.md", chunk.chunk_id.replace(['/', '#'], "-"));
        let chunk_file = chunks_dir.join(&chunk_filename);
        let content = format!("{}\n{}", frontmatter, chunk.content);
        fs::write(chunk_file, content)?;
    }

    Ok(ChunksResult {
        total_chunks: all_chunks.len(),
        document_count: analyses.len(),
        chunks_metadata: all_chunks,
    })
}
```

**Characteristics**:
- Pure logic (`create_chunks_smart`, `link_chunks`)
- Side effects isolated to final write loop
- Returns structured result
- Testable by inspecting `ChunksResult`

---

## Diagram: Ports and Adapters

```
┌────────────────────────────────────────────────────────────────────┐
│                                                                    │
│                          EXTERNAL WORLD                            │
│                                                                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐         │
│  │   CLI    │  │   File   │  │  Regex   │  │  Graph   │         │
│  │  (clap)  │  │  System  │  │  Engine  │  │  (petgraph)│       │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘         │
│       │             │              │              │               │
└───────┼─────────────┼──────────────┼──────────────┼───────────────┘
        │             │              │              │
        ▼             ▼              ▼              ▼
┌────────────────────────────────────────────────────────────────────┐
│                           ADAPTERS                                  │
│                                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐          │
│  │  main.rs │  │ discover │  │ analyze  │  │  graph   │          │
│  │          │  │   .rs    │  │   .rs    │  │   .rs    │          │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘          │
└────────┬───────────────┬──────────────┬──────────────┬────────────┘
         │               │              │              │
         │     ┌─────────┴──────────────┴──────────────┴─────────┐
         │     │                PORTS                             │
         │     │  (Function Signatures, Data Structures, Results) │
         │     └─────────┬──────────────┬──────────────┬─────────┘
         │               │              │              │
         ▼               ▼              ▼              ▼
┌────────────────────────────────────────────────────────────────────┐
│                      APPLICATION CORE                               │
│                                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐          │
│  │transform │  │  chunk   │  │  index   │  │ validate │          │
│  │   .rs    │  │   .rs    │  │   .rs    │  │   .rs    │          │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘          │
│                                                                     │
│  Pure Business Logic:                                               │
│  • Deterministic transformations                                   │
│  • No side effects                                                  │
│  • Explicit error handling                                         │
│  • Immutable data                                                   │
└────────────────────────────────────────────────────────────────────┘
```

---

## Summary

This architecture achieves:

1. **Testability**: Core logic isolated from I/O
2. **Maintainability**: Clear module boundaries
3. **Type Safety**: Rust compiler enforces contracts
4. **Zero Panics**: All errors are explicit
5. **Functional Purity**: Transformations are deterministic
6. **Flexibility**: Easy to swap adapters without changing core

### Key Takeaways

- **Hexagonal Architecture**: Core is independent of external systems
- **Functional Patterns**: Immutability, pure functions, Result types
- **Explicit Dependencies**: No hidden state or global variables
- **Pipeline Design**: Sequential stages with clear data flow
- **Error Handling**: Railway-oriented programming via `?` operator

---

## See Also

- [Cargo.toml](/home/lewis/src/centralized-docs/doc_transformer/Cargo.toml) - Project dependencies
- [main.rs](/home/lewis/src/centralized-docs/doc_transformer/src/main.rs) - Entry point and CLI
- [lib.rs](/home/lewis/src/centralized-docs/doc_transformer/src/lib.rs) - Public API
- [Testing Documentation](./TESTING.md) - Comprehensive testing guide (TODO)

---

**Document ID**: `architecture-hexagonal-design`
**Category**: Reference
**Tags**: architecture, hexagonal, ports-adapters, functional-programming, rust, design-patterns
**Last Updated**: 2026-01-11
