# Architecture Diagrams

> Visual representations of the hexagonal architecture and data flow in doc_transformer

---

## System Overview

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃                         DOC TRANSFORMER SYSTEM                       ┃
┃                      Hexagonal Architecture (Rust)                   ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

                                    │
                                    │ CLI Commands
                                    ▼
              ┌─────────────────────────────────────────┐
              │      PRESENTATION LAYER (main.rs)       │
              │  • Parse CLI args (clap)                │
              │  • Display results                      │
              │  • Manage async runtime (tokio)         │
              └──────────────┬──────────────────────────┘
                             │
                             │ Function Calls
                             ▼
┌────────────────────────────────────────────────────────────────────┐
│                    APPLICATION LAYER (Core Logic)                   │
│                                                                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐           │
│  │transform │  │  chunk   │  │  index   │  │ validate │           │
│  │   .rs    │  │   .rs    │  │   .rs    │  │   .rs    │           │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘           │
│                                                                      │
│  Characteristics:                                                    │
│  • Pure functions (deterministic)                                   │
│  • Immutable data structures                                        │
│  • Result<T, E> for error handling                                  │
│  • No side effects (delegates I/O)                                  │
└──────────────┬─────────────────────────────────────┬────────────────┘
               │                                     │
               │ Uses Data Types                     │ Calls Functions
               ▼                                     ▼
┌────────────────────────────────────────────────────────────────────┐
│                   PORTS LAYER (Contracts)                           │
│                                                                      │
│  Data Structures:          Function Signatures:                     │
│  • Analysis                • discover_files()                       │
│  • Chunk                   • analyze_files()                        │
│  • GraphNode               • chunk_all()                            │
│  • IndexDocument           • validate_all()                         │
│                                                                      │
│  Result Types:                                                       │
│  • Result<T, anyhow::Error>                                         │
│  • Option<T>                                                         │
└──────────────┬─────────────────────────────────────┬────────────────┘
               │                                     │
               │ Implements                          │ Provides Data
               ▼                                     ▼
┌────────────────────────────────────────────────────────────────────┐
│                   ADAPTERS LAYER (External I/O)                     │
│                                                                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐           │
│  │ discover │  │ analyze  │  │  graph   │  │  assign  │           │
│  │   .rs    │  │   .rs    │  │   .rs    │  │   .rs    │           │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘           │
│                                                                      │
│  Responsibilities:                                                   │
│  • File system I/O (walkdir, std::fs)                               │
│  • Regex matching (lazy statics)                                    │
│  • Graph algorithms (petgraph)                                      │
│  • JSON serialization (serde)                                       │
└──────────────┬─────────────────────────────────────┬────────────────┘
               │                                     │
               ▼                                     ▼
      ┌────────────────┐                  ┌────────────────┐
      │  File System   │                  │  External Libs │
      │  • Read files  │                  │  • petgraph    │
      │  • Write files │                  │  • regex       │
      │  • Walk dirs   │                  │  • serde       │
      └────────────────┘                  └────────────────┘
```

---

## Data Flow Pipeline

```
INPUT                    PIPELINE STAGES                    OUTPUT
═════                    ═══════════════                    ══════

Source                        ┏━━━━━━━━━━━━━━━━━┓
Directory ───────────────────►┃ 1. DISCOVER     ┃
(*.md, *.mdx)                 ┃  discover.rs    ┃
                              ┗━━━━━━━┬━━━━━━━━━┛
                                      │
                          Vec<DiscoveryFile>
                                      │
                              ┏━━━━━━━▼━━━━━━━━━┓
                              ┃ 2. ANALYZE      ┃
                              ┃  analyze.rs     ┃──► Extract:
                              ┗━━━━━━━┬━━━━━━━━━┛    • Title
                                      │              • Headings
                              Vec<Analysis>          • Links
                                      │              • Category
                              ┏━━━━━━━▼━━━━━━━━━┓
                              ┃ 3. ASSIGN IDs   ┃
                              ┃  assign.rs      ┃──► Generate:
                              ┗━━━━━━━┬━━━━━━━━━┛    • SHA256 IDs
                                      │              • Filenames
                        (Vec<Analysis>, LinkMap)     • Link map
                                      │
                              ┏━━━━━━━▼━━━━━━━━━┓
                              ┃ 4. TRANSFORM    ┃
                              ┃  transform.rs   ┃──► Rewrite:
                              ┗━━━━━━━┬━━━━━━━━━┛    • Headings
                                      │              • Links
                           TransformResult           • Frontmatter
                                      │
                              ┏━━━━━━━▼━━━━━━━━━┓
                              ┃ 5. CHUNK        ┃
                              ┃  chunk.rs       ┃──► Split on:
                              ┗━━━━━━━┬━━━━━━━━━┛    • H2 boundaries
                                      │              • ~170 tokens
                            ChunksResult             • Add context
                                      │
                              ┏━━━━━━━▼━━━━━━━━━┓
                              ┃ 6. INDEX        ┃
                              ┃  index.rs       ┃──► Build:
                              ┗━━━━━━━┬━━━━━━━━━┛    • INDEX.json
                                      │              • COMPASS.md
                                 Index Data          • Graph (DAG)
                                      │
                              ┏━━━━━━━▼━━━━━━━━━┓
                              ┃ 7. VALIDATE     ┃
                              ┃  validate.rs    ┃──► Check:     Indexed
                              ┗━━━━━━━┬━━━━━━━━━┛    • Frontmatter    Docs
                                      │              • Headings       ──────►
                          ValidationResult           • Links       output_dir/
                                      │                              ├─ docs/
                                      ▼                              ├─ chunks/
                                   SUCCESS                           ├─ INDEX.json
                                                                     └─ COMPASS.md
```

---

## Module Dependencies

```
┌─────────────────────────────────────────────────────────────────┐
│                          main.rs                                 │
│                    (Presentation Layer)                          │
└───┬─────────┬─────────┬─────────┬─────────┬─────────┬──────────┘
    │         │         │         │         │         │
    ▼         ▼         ▼         ▼         ▼         ▼
┌────────┐┌────────┐┌────────┐┌────────┐┌────────┐┌─────────┐
│discover││analyze ││transform││ chunk  ││ index  ││validate │
│  .rs   ││  .rs   ││  .rs   ││  .rs   ││  .rs   ││  .rs    │
└────────┘└───┬────┘└───┬────┘└───┬────┘└───┬────┘└────┬────┘
             │         │         │         │         │
             │         │         │         │         │
    ┌────────┴─────────┴─────────┴─────────┴─────────┘
    │
    ▼
┌─────────┐
│ assign  │
│  .rs    │
└────┬────┘
     │
     ▼
┌─────────┐
│ graph   │
│  .rs    │
└─────────┘

Legend:
  ─►  Direct dependency
  Module at bottom = no internal dependencies
  Module at top = depends on all below
```

---

## Error Handling Flow (Railway-Oriented)

```
Happy Path                       Error Path
═════════                        ══════════

discover_files()
      │
      ├──────► Ok(files) ────────────────────┐
      │                                      │
      └──────► Err(e) ──────────────────────┼──► Return Err
                                             │
analyze_files(files)                        │
      │                                      │
      ├──────► Ok(analyses) ─────────────────┤
      │                                      │
      └──────► Err(e) ──────────────────────┼──► Return Err
                                             │
assign_ids(analyses)                        │
      │                                      │
      ├──────► Ok((analyses, map)) ─────────┤
      │                                      │
      └──────► Err(e) ──────────────────────┼──► Return Err
                                             │
transform_all(analyses, map, dir)          │
      │                                      │
      ├──────► Ok(result) ───────────────────┤
      │                                      │
      └──────► Err(e) ──────────────────────┼──► Return Err
                                             │
      ▼                                      ▼
   Success                              Early Exit
                                    (Error propagated)

Characteristics:
• Each stage returns Result<T, E>
• ? operator propagates errors up
• No exceptions or panics
• Caller decides error handling
```

---

## Hexagonal Architecture Detailed View

```
┌─────────────────────────────────────────────────────────────────┐
│                        EXTERNAL WORLD                            │
│                                                                   │
│  ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐        │
│  │   CLI   │   │  File   │   │  Regex  │   │  JSON   │        │
│  │  User   │   │ System  │   │ Engine  │   │  Serde  │        │
│  └────┬────┘   └────┬────┘   └────┬────┘   └────┬────┘        │
└───────┼─────────────┼─────────────┼─────────────┼──────────────┘
        │             │             │             │
        │             │             │             │
┌───────▼─────────────▼─────────────▼─────────────▼──────────────┐
│                          ADAPTERS                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   main.rs    │  │  discover.rs │  │  analyze.rs  │          │
│  │              │  │              │  │              │          │
│  │ Parses CLI   │  │ Walks dirs   │  │ Parses MD    │          │
│  │ args, shows  │  │ with walkdir │  │ with regex   │          │
│  │ output       │  │              │  │              │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   graph.rs   │  │   index.rs   │  │  search.rs   │          │
│  │              │  │  (writes)    │  │  (reads)     │          │
│  │ Uses petgraph│  │ Uses serde   │  │ BM25 search  │          │
│  │ for DAG      │  │ for JSON     │  │              │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                   Implements Ports
                          │
┌─────────────────────────▼───────────────────────────────────────┐
│                          PORTS                                   │
│                   (Abstract Contracts)                           │
│                                                                   │
│  Function Signatures:           Data Structures:                 │
│  • discover_files(...)          • DiscoveryFile                  │
│  • analyze_files(...)           • Analysis                       │
│  • transform_all(...)           • Chunk                          │
│  • chunk_all(...)               • GraphNode                      │
│  • build_and_write_index(...)   • IndexDocument                  │
│                                                                   │
│  These define the "what" (contract), not "how" (implementation)  │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                    Uses Ports
                          │
┌─────────────────────────▼───────────────────────────────────────┐
│                  APPLICATION CORE                                │
│                   (Business Logic)                               │
│                                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ transform.rs │  │   chunk.rs   │  │ validate.rs  │          │
│  │              │  │              │  │              │          │
│  │ • Fix        │  │ • Smart      │  │ • Check      │          │
│  │   headings   │  │   chunking   │  │   quality    │          │
│  │ • Rewrite    │  │ • Add        │  │ • Validate   │          │
│  │   links      │  │   context    │  │   structure  │          │
│  │ • Add        │  │ • Link       │  │ • Find       │          │
│  │   metadata   │  │   chunks     │  │   broken     │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                                                                   │
│  Pure Functions:                                                  │
│  • Deterministic (same input → same output)                      │
│  • No side effects (no I/O, no mutations)                        │
│  • Testable without mocks                                        │
│  • Composable via Result chains                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## Functional Programming Patterns

```
┌─────────────────────────────────────────────────────────────────┐
│              Immutability Pattern                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  Input: &str (borrowed)                                          │
│     │                                                             │
│     ▼                                                             │
│  ┌───────────────────┐                                           │
│  │  Transform Logic  │  • No mutations                           │
│  │  (Pure Function)  │  • Build new data                         │
│  └─────────┬─────────┘  • Return new value                       │
│            │                                                      │
│            ▼                                                      │
│  Output: String (owned, new)                                     │
│                                                                   │
│  Example:                                                         │
│  fn fix_headings(content: &str) -> String {                      │
│      let lines: Vec<String> = content.lines()                    │
│          .map(|s| s.to_string())                                 │
│          .collect();                                             │
│      // transform lines...                                       │
│      lines.join("\n")  // new String                             │
│  }                                                                │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│              Result Chaining (Railway-Oriented)                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  discover_files()                                                │
│       │                                                           │
│       ▼                                                           │
│  ┌─────────┐    ? operator                                       │
│  │ Result  ├──────────────────► Early return on Err              │
│  │<T, E>   │                                                      │
│  └────┬────┘                                                      │
│       │ Ok(value)                                                 │
│       ▼                                                           │
│  analyze_files(value)                                            │
│       │                                                           │
│       ▼                                                           │
│  ┌─────────┐    ? operator                                       │
│  │ Result  ├──────────────────► Early return on Err              │
│  │<T, E>   │                                                      │
│  └────┬────┘                                                      │
│       │ Ok(value)                                                 │
│       ▼                                                           │
│  transform_all(value)                                            │
│       │                                                           │
│       ▼                                                           │
│  ┌─────────┐                                                      │
│  │ Result  │  Final result                                       │
│  │<T, E>   │  (success or first error)                           │
│  └─────────┘                                                      │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│              Option<T> for Nullable Values                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  pub struct Chunk {                                              │
│      pub heading: Option<String>,  // May not have heading       │
│      pub previous_chunk_id: Option<String>,  // First chunk      │
│      pub next_chunk_id: Option<String>,      // Last chunk       │
│  }                                                                │
│                                                                   │
│  Pattern Matching:                                               │
│  match chunk.heading {                                           │
│      Some(h) => println!("Heading: {}", h),                      │
│      None => println!("No heading"),                             │
│  }                                                                │
│                                                                   │
│  Unwrap with Default:                                            │
│  let heading = chunk.heading.unwrap_or("Introduction");          │
└─────────────────────────────────────────────────────────────────┘
```

---

## Graph Structure (Knowledge DAG)

```
┌─────────────────────────────────────────────────────────────────┐
│                    KNOWLEDGE GRAPH (DAG)                         │
└─────────────────────────────────────────────────────────────────┘

Nodes: Documents and Chunks
Edges: Relationships with weights

Document Layer:
┌──────────────┐    references    ┌──────────────┐
│   doc-001    │─────────────────►│   doc-002    │
│  (tutorial)  │                  │  (concept)   │
└──────┬───────┘                  └──────┬───────┘
       │                                 │
       │ parent                          │ parent
       ▼                                 ▼
┌──────────────┐  sequential   ┌──────────────┐
│  chunk-001-0 │──────────────►│  chunk-001-1 │
└──────────────┘               └──────┬───────┘
       │                              │
       │ related (0.65)               │ related (0.72)
       └──────────────┐   ┌───────────┘
                      ▼   ▼
                  ┌──────────────┐
                  │  chunk-002-0 │
                  └──────────────┘

Edge Types:
• Sequential:     Next chunk in document (weight: 1.0)
• Parent:         Document → Chunk relationship (weight: 1.0)
• Hierarchical:   Category-based organization (weight: 0.5-0.8)
• Related:        Semantic similarity via tags (weight: 0.3-0.9)
• References:     Explicit markdown link (weight: 0.7)
• ReferencedBy:   Reverse reference (weight: 0.7)
• CoAuthored:     Shares tags/category (weight: 0.4)

Graph Operations:
• topological_order() - Dependency-safe ordering
• reachable_from(id) - Find all connected nodes
• get_related_chunks(id) - Semantic neighbors
```

---

## Testing Pyramid

```
                        ┌─────────────────┐
                        │  Integration    │  ← CLI commands
                        │  Tests (few)    │    End-to-end
                        └────────┬────────┘
                                │
                    ┌───────────▼───────────┐
                    │   Integration Tests   │  ← Real file I/O
                    │   (moderate)          │    with tempfile
                    └───────────┬───────────┘
                                │
                ┌───────────────▼───────────────┐
                │      Unit Tests               │  ← Pure functions
                │      (many)                   │    Table-driven
                └───────────────────────────────┘

Unit Tests (≥90% coverage):
• Test pure functions with multiple inputs
• Table-driven test cases
• No mocking needed (pure logic)

Integration Tests (≥85% coverage):
• Test adapters with real I/O
• Use tempfile for file system tests
• Verify external library integration

E2E Tests (critical paths):
• Full CLI command execution
• Validate output structure
• Check idempotency
```

---

## See Also

- [ARCHITECTURE.md](./ARCHITECTURE.md) - Detailed architecture documentation
- [Cargo.toml](/home/lewis/src/centralized-docs/doc_transformer/Cargo.toml) - Dependencies
- [main.rs](/home/lewis/src/centralized-docs/doc_transformer/src/main.rs) - Entry point

---

**Last Updated**: 2026-01-11
