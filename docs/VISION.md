# Vision: Project Philosophy and Purpose

## Executive Summary

**centralized-docs** is a pure Rust CLI tool that transforms raw documentation into AI-optimized, searchable knowledge structures. It implements Anthropic's Contextual Retrieval pattern, semantic chunking, and hexagonal architecture to deliver the most effective documentation system for both human developers and AI agents.

This document defines the philosophical approach, core goals, design principles, and technical implementation strategy that guides all development decisions.

---

## Table of Contents

1. [Why This Project Exists](#why-this-project-exists)
2. [Core Philosophy: 5 Pillars](#core-philosophy-5-pillars)
3. [Design Principles](#design-principles)
4. [Core Features](#core-features)
5. [Success Criteria](#success-criteria)
6. [Implementation Strategy](#implementation-strategy)
7. [Use Cases](#use-cases)
8. [Roadmap](#roadmap)

---

## Why This Project Exists

### The Problem

Documentation systems fail AI agents. Current approaches suffer from:

1. **Context Loss**: Chunks without surrounding context → AI answers "I don't have enough information"
2. **Disconnection**: Documents treated as isolated units → AI misses relationships and dependencies
3. **Search Friction**: Traditional indexing doesn't optimize for AI consumption → retrieval failures
4. **Scaling Delays**: Converting documentation to AI-friendly formats is manual and error-prone
5. **No Navigation**: Without relationship mapping, AI can't discover related content

### The Solution

**centralized-docs** solves this through:
- **Contextual Chunking**: Each chunk carries 50-100 tokens of surrounding context (35% fewer retrieval failures per Anthropic research)
- **Knowledge Graphs**: Automatic DAG-based relationship detection between documents
- **Semantic Indexing**: Full-text search optimized for AI agent queries
- **Validation Pipeline**: Quality checks and metadata verification at every step
- **Navigation Guides**: COMPASS.md for human discovery + INDEX.json for AI search

### Ideal For

- **Personal Knowledge Management**: Private documentation archives with instant search
- **Team Documentation**: Shared knowledge bases discoverable via search and graphs
- **AI Integration**: RAG systems that need contextual, relationship-aware retrieval
- **Developer Onboarding**: Self-serve knowledge systems with clear navigation
- **Open Source Projects**: Maintainers who want docs that work with AI tools

---

## Core Philosophy: 5 Pillars

Every decision in this project follows these five core principles:

### 1. Version Control as Source of Truth

- **Single Source**: Documentation lives in version control (git), not separate systems
- **Audit Trail**: All changes tracked with commit history and authorship
- **Reproducibility**: Same input produces identical output across runs
- **Offline First**: Works without cloud dependencies or external services

**Why This Matters**: AI agents need deterministic, auditable transformations. Version control provides both.

### 2. Search as Exploration

- **Discovery**: Documentation isn't useful unless people can find it
- **Multiple Paths**: Keyword search, relationship graphs, navigation guides
- **Context-Aware**: Search results include document relationships and chunking context
- **Semantic Understanding**: Index captures not just words but meaning

**Why This Matters**: Without search, documentation is just data. Search transforms data into knowledge.

### 3. Portability Above All

- **No Lock-in**: Output formats (JSON, Markdown) are open standards
- **Framework Agnostic**: Works with any documentation source format
- **Easy Integration**: AI agents can load INDEX.json without special tools
- **Minimal Dependencies**: Pure Rust, no external services required

**Why This Matters**: Documentation systems must outlast frameworks. Portability ensures longevity.

### 4. Complete Traceability

- **One Trace Per Operation**: Each command execution records what was transformed
- **Audit Metadata**: Every output includes source, transformation, validation status
- **Relationship Mapping**: Complete knowledge graph showing document dependencies
- **Quality Signals**: Validation reports identify problematic content

**Why This Matters**: When transformations fail, traceability enables rapid debugging. For AI agents, complete context prevents hallucination.

### 5. Testable by Default

- **No Untestable Code**: Every transformation is covered by unit tests (≥90% coverage)
- **Table-Driven Tests**: Comprehensive test cases for edge cases and variants
- **Deterministic**: No randomness, no timing-dependent behavior
- **Validation Pipeline**: Quality checks prevent bad data from reaching users

**Why This Matters**: Documentation systems must be reliable. Testing catches regressions before they reach users.

---

## Design Principles

These principles guide architectural and implementation decisions:

### Modularity

- **Single Responsibility**: Each module has one reason to change
- **Clear Boundaries**: Explicit public APIs, private implementation details
- **Composition**: Complex workflows are built from simple, testable parts
- **No Circular Dependencies**: Clean dependency graph, no module cycles

**Example**: The 7-step pipeline (discover → analyze → assign → transform → chunk → index → validate) separates concerns so each step can be tested, debugged, and improved independently.

### Cohesion

- **Related Logic Together**: Functions that work together stay in the same module
- **Strong Internal Coupling**: High cohesion within modules
- **Weak External Coupling**: Low coupling between modules via explicit contracts
- **Data Co-location**: Types and functions that operate on the same data are together

**Example**: All chunking logic (semantic splitting, context prefixes, navigation links) lives in `chunk.rs`, not scattered across multiple files.

### Separation of Concerns

- **Business Logic Isolated**: Core transformation logic doesn't depend on I/O
- **I/O Adapters**: File system, serialization, and external APIs are swappable adapters
- **Pure Functions**: Business logic uses pure functions; I/O effects are confined to adapters
- **Clear Boundaries**: Each layer has explicit responsibilities and interfaces

**Example**: The graph module contains pure relationship-detection logic. The index module handles JSON serialization separately, allowing easy format changes.

### Abstraction

- **Hide Complexity**: Implementation details are hidden behind clear interfaces
- **Reveal Intent**: Public APIs express what matters, not how it works
- **Stable Contracts**: Public types and functions are stable; private code can change freely
- **Progressive Disclosure**: Simple cases are easy; complex cases are possible

**Example**: Users call `doc_transformer /source /output` without knowing about the petgraph algorithm underneath. If we improve the algorithm, no public API changes.

### Loose Coupling

- **Dependency Injection**: All dependencies are passed in, not created internally
- **Protocol-Based Design**: Components communicate via stable contracts, not tight coupling
- **Easy Testing**: Tests can inject mock implementations without modifying production code
- **Swappable Implementations**: Storage backend, graph algorithm, indexing strategy can all change

**Example**: The analysis module doesn't call the graph module directly. Instead, it returns metadata; the main orchestrator composes the pieces. This allows testing each independently.

---

## Core Features

### 1. Document Discovery & Analysis

**What It Does**: Scans directories recursively for markdown files, extracts metadata.

**What Gets Extracted**:
- Title (from H1 or filename)
- Heading hierarchy (all H2, H3, etc.)
- Internal and external links
- First paragraph (for summaries)
- Word count and reading time
- Category detection (tutorial, reference, concept, ops)
- Code block and table detection

**Why It Matters**: Metadata enables search, relationships, and navigation.

**Example**:
```
Input:  docs/getting-started.md (1234 words, 3 code blocks)
Output: Analysis {
  title: "Getting Started",
  category: Tutorial,
  tags: [setup, guide],
  links: ["docs/reference.md", "https://example.com"],
  ...
}
```

### 2. Semantic Chunking with Context

**What It Does**: Splits documents into AI-friendly chunks that include surrounding context.

**How It Works**:
1. Split on H2 boundaries (semantic natural breaks)
2. Estimate tokens (~4 chars = 1 token, target ~170 tokens/chunk)
3. Prepend 50-100 tokens from the previous chunk (context)
4. Include navigation metadata (previous/next chunk, heading, position)

**Why Context Matters**:
- Without context: AI reads chunk in isolation → missing context → answers "I don't have enough information"
- With context: AI understands what came before → provides complete answer → 35% fewer retrieval failures (Anthropic research)

**Example**:
```markdown
---
doc_id: api-reference
chunk_id: api-reference#2
heading: Authentication
context_prefix: "## Overview\nThis section covers the API.
  Authentication is required..."
---

## Authentication

Token-based authentication is required for all API requests...
```

### 3. Knowledge Graph (DAG)

**What It Does**: Automatically detects relationships between documents.

**How It Works**:
1. Analyze all documents simultaneously
2. Compute Jaccard similarity between documents based on shared concepts
3. Create directed edges where document A relates to document B
4. Enforce DAG property (no cycles, no circular dependencies)
5. Order topologically for presentation

**Why Graphs Matter**:
- **Discovery**: "I need info on authentication" → graph shows related API docs, security guides, troubleshooting
- **Context for AI**: Instead of isolated chunks, AI understands document ecosystem
- **Navigation**: Users can follow relationship chains to discover related content
- **Completeness**: Documentation is never truly standalone; graphs show how pieces fit together

**Example**:
```json
{
  "documents": [
    { "id": "api-reference", "title": "API Reference" },
    { "id": "authentication", "title": "Authentication Guide" },
    { "id": "security", "title": "Security Best Practices" }
  ],
  "edges": [
    { "from": "api-reference", "to": "authentication", "weight": 0.85 },
    { "from": "authentication", "to": "security", "weight": 0.72 }
  ]
}
```

### 4. Full-Text Indexing

**What It Does**: Creates a searchable index of all documents and chunks.

**Index Contains**:
- Keyword → document mapping (which docs contain this keyword?)
- Keyword → chunk mapping (which chunks match this keyword?)
- Metadata filtering (by category, tags, author)
- Relevance scoring (BM25 or similar)
- Navigation links (previous/next chunks, related documents)

**Why Indexing Matters**:
- AI agents can search by keyword
- Humans can discover docs they didn't know existed
- Search results are complete (including all relevant chunks)
- Metadata filters narrow results efficiently

### 5. Navigation Guide (COMPASS.md)

**What It Does**: Creates a human-friendly table of contents and navigation guide.

**Contains**:
- Document hierarchy organized by category
- Suggested reading order
- Relationship maps (which docs relate to which?)
- Search tips (what keywords to use?)
- Quick links to important sections

**Why Navigation Matters**:
- Humans learn by browsing; COMPASS enables that
- Self-serve onboarding without "where do I start?" questions
- Shows structure at a glance
- Complements search for discovery

### 6. Quality Validation

**What It Does**: Runs checks on the transformed documentation.

**Checks Include**:
- All referenced links are valid (internal or external)
- No orphaned documents (unreachable from navigation)
- Chunk context is well-formed
- Metadata is complete (titles, categories, etc.)
- No duplicate document IDs
- Knowledge graph is acyclic

**Why Validation Matters**:
- Catches errors before documentation reaches users
- Prevents broken links and orphaned content
- Ensures consistency and completeness
- Provides confidence that output is usable

---

## Success Criteria

These measurable checkpoints define when we've succeeded:

### Coverage: ≥90% Unit Test Coverage
- **Metric**: `cargo tarpaulin` reports ≥90% line coverage
- **Why**: Catches regressions before they reach users
- **Enforcement**: CI/CD pipeline blocks merges with coverage < 90%

### Zero Panic Points
- **Metric**: `cargo clippy --all-targets` has zero panic-related warnings
- **Why**: Production systems must never panic; failures are explicit via Result types
- **Enforcement**: All errors handled via `Result<T, E>` or `Option<T>`

### Deterministic Output
- **Metric**: Running the same transformation twice produces identical output
- **Why**: Reproducibility enables debugging and auditing
- **Enforcement**: No randomness, no timing-dependent behavior, no floating-point errors

### Performance: <5 seconds for 100 documents
- **Metric**: Benchmarks show document transformation completes in <5 seconds for 100 files
- **Why**: Documentation systems must be fast enough for interactive use
- **Enforcement**: Benchmark suite tracks performance across releases

### Relationship Coverage: ≥70% of documents have relationships
- **Metric**: At least 70% of documents show at least one related document
- **Why**: Navigation graphs are valuable only if densely connected
- **Enforcement**: Validation reports document coverage statistics

### Zero Nil Pointers in Public API
- **Metric**: All public functions return `Result<T, E>` or `Option<T>`; no `.unwrap()` or `.panic!()`
- **Why**: Makes error handling explicit and testable
- **Enforcement**: Code review checks every public function

### Documentation Completeness
- **Metric**: Every public module and function has documentation comments
- **Why**: Users and maintainers can understand code without reading implementation
- **Enforcement**: `cargo doc --no-deps` has no undocumented items

### Integration Tests Pass
- **Metric**: `cargo test --all-features` passes with all integration tests
- **Why**: End-to-end tests catch issues that unit tests miss
- **Enforcement**: CI/CD blocks releases with failing tests

---

## Implementation Strategy

### Architecture: Hexagonal (Ports & Adapters)

The system is organized in concentric layers:

```
┌──────────────────────────────────────────────────┐
│  Presentation (CLI, argument parsing)            │
├──────────────────────────────────────────────────┤
│  Application (7-step pipeline orchestration)    │
├──────────────────────────────────────────────────┤
│  Domain (Pure business logic, types, rules)      │
├──────────────────────────────────────────────────┤
│  Adapters (File I/O, serialization, graphs)      │
└──────────────────────────────────────────────────┘
```

**Benefits**:
- Core logic is testable without I/O
- Easy to swap storage backends
- Clear dependency direction (outer depends on inner)
- Pure functions in domain layer

### Pure Functional Programming

All business logic follows functional principles:

1. **Immutability**: Data structures are immutable; transformations return new values
2. **Pure Functions**: Functions always produce same output for same input; no side effects
3. **Composition**: Complex operations built by composing simple functions
4. **Result/Option**: All errors are explicit; no exception throwing
5. **No Hidden I/O**: I/O effects are confined to adapters, not business logic

**Example Pattern**:
```rust
// Pure function: takes input, returns Result
fn chunk_document(doc: &Document) -> Result<Vec<Chunk>, ChunkError> {
    doc.content
        .split_on_headings()
        .map(estimate_tokens)
        .flat_map(|chunk| add_context(chunk))
        .collect()
}

// Calling it doesn't perform I/O, doesn't panic, always returns Result
let result = chunk_document(&my_doc);
```

### 7-Step Pipeline

All transformations follow this sequence:

1. **DISCOVER**: Scan directories, find markdown files
2. **ANALYZE**: Extract metadata from each document
3. **ASSIGN IDs**: Generate unique, stable document IDs
4. **TRANSFORM**: Apply standard formatting and frontmatter
5. **CHUNK**: Semantic splitting with context prefixes
6. **INDEX**: Create INDEX.json and COMPASS.md
7. **VALIDATE**: Quality checks and validation

**Why Sequential**: Each step depends on previous; clean dependencies.

### Testing Strategy

- **Unit Tests**: Test individual functions with table-driven test cases
- **Integration Tests**: Test full pipeline end-to-end
- **Property Tests**: Where applicable, use property-based testing
- **Coverage Enforcement**: Block commits with coverage < 90%

**Pattern: Table-Driven Tests**
```rust
#[test]
fn chunk_respects_token_limit() {
    let cases = vec![
        ("small doc", 50, 1),      // (content, max_tokens, expected_chunks)
        ("medium doc", 170, 2),
        ("large doc", 500, 4),
    ];
    for (content, max_tokens, expected) in cases {
        let chunks = chunk(content, max_tokens).unwrap();
        assert_eq!(chunks.len(), expected);
    }
}
```

### Dependency Management

- **Minimal External Dependencies**: Every dependency must justify its weight
- **Pure Rust**: No FFI, no external language runtime
- **Pinned Versions**: Reproducible builds via locked dependencies
- **Regular Updates**: Security patches applied promptly

**Current Dependencies**:
- `petgraph`: Graph algorithms (knowledge DAG)
- `serde`/`serde_json`: Serialization
- `regex`: Pattern matching
- `walkdir`: Directory traversal
- `chrono`: Timestamps
- `clap`: CLI parsing
- `tokio`: Async runtime (for scraping)
- `anyhow`: Error handling

---

## Use Cases

### Use Case 1: Personal Knowledge Management

**Scenario**: A developer maintains personal notes, code snippets, and research links.

**How centralized-docs Helps**:
- `doc_transformer ./notes ./indexed` transforms all notes at once
- Search via INDEX.json (using AI tools)
- COMPASS.md provides browseable structure
- Knowledge graph shows connections between topics
- Validation ensures no broken links or orphaned notes

**Value**: Instant searchability across personal knowledge base; AI agents can query notes for context.

### Use Case 2: Team Documentation

**Scenario**: Team maintains shared documentation (guides, runbooks, architecture).

**How centralized-docs Helps**:
- Automated transformation as part of CI/CD (on each push to docs/)
- INDEX.json shared with team for search
- COMPASS.md serves as navigation guide in wiki/wiki system
- Knowledge graph shows dependencies between guides
- Version control provides audit trail of documentation changes

**Value**: Consistent documentation structure; easy discovery; audit trail; AI-friendly for automation.

### Use Case 3: AI Integration & RAG

**Scenario**: Building a RAG system that answers questions using documentation.

**How centralized-docs Helps**:
- INDEX.json provides all metadata and chunks
- Contextual prefixes in chunks improve retrieval quality
- Knowledge graph provides relationship awareness
- Consistent IDs enable reliable linking
- Validation ensures data quality for ML models

**Value**: 35% fewer retrieval failures; better-informed AI responses; complete context preservation.

### Use Case 4: Open Source Project Documentation

**Scenario**: Maintainers have extensive documentation but users can't find anything.

**How centralized-docs Helps**:
- COMPASS.md provides clear navigation for human users
- INDEX.json enables AI-powered doc search features
- Knowledge graph shows relationships between concepts
- One transformation generates both human-friendly and AI-friendly outputs

**Value**: Self-serve user onboarding; better documentation discoverability; AI tools can help answer questions.

### Use Case 5: Developer Onboarding

**Scenario**: New team member needs to understand codebase, architecture, and patterns.

**How centralized-docs Helps**:
- COMPASS.md provides recommended reading order
- Knowledge graph shows architectural relationships
- Search enables quick answers to questions
- Validation ensures documentation is complete and consistent

**Value**: Structured learning path; fast onboarding; self-serve knowledge discovery.

---

## Roadmap

### Phase 1: Foundation (Current)
- [x] 7-step pipeline (discover → validate)
- [x] Semantic chunking with context
- [x] Knowledge graph (DAG-based relationships)
- [x] Full-text indexing (INDEX.json)
- [x] Navigation guide (COMPASS.md)
- [x] Quality validation
- [ ] Complete documentation

### Phase 2: Web Integration (In Progress)
- [ ] Web scraping via spider-rs
- [ ] Content filtering (duplicate detection, quality scores)
- [ ] llms.txt generation (for AI model context)
- [ ] Sitemap support
- [ ] Incremental updates

### Phase 3: AI Optimization (Planned)
- [ ] LLM-assisted content extraction
- [ ] Automatic category classification
- [ ] Semantic relationship scoring (not just text-based)
- [ ] Query-time context enrichment
- [ ] Chunk quality scoring

### Phase 4: Scalability (Future)
- [ ] Database backends (SQLite, PostgreSQL)
- [ ] Distributed processing for large doc sets
- [ ] Real-time indexing (watch filesystem)
- [ ] REST API for query
- [ ] Web UI for navigation

### Phase 5: Ecosystem (Long-term)
- [ ] Plugin system for custom transformations
- [ ] Integration with documentation platforms (Notion, Confluence)
- [ ] Custom graph algorithms (user-defined relationships)
- [ ] Analytics and usage tracking
- [ ] Enterprise features (audit logging, access control)

---

## Design Decisions & Tradeoffs

### Why Rust, Not Go?

**Decision**: Implement in pure Rust with functional patterns.

**Tradeoffs**:
| Advantage | Cost |
|-----------|------|
| Zero-cost abstractions (no GC pauses) | Steeper learning curve for team |
| Compile-time safety (no nil pointers) | Longer development time initially |
| Pure functional patterns (immutability, Result types) | Small ecosystem vs. Go |
| Single binary (no runtime required) | Build time slower than interpreted languages |

**Rationale**: Documentation systems must be bulletproof. Rust's type system prevents entire categories of bugs at compile time. Pure functional patterns make error handling explicit and testable.

### Why Semantic Chunking, Not Fixed Size?

**Decision**: Split on H2 boundaries instead of fixed token sizes.

**Tradeoffs**:
| Advantage | Cost |
|-----------|------|
| Respects document structure | Some chunks larger or smaller than ideal |
| Context is naturally available | Need to prepend context from previous chunk |
| Chunks align with reader expectations | More complex implementation |

**Rationale**: Documentation has intentional structure (sections). Respecting that structure makes chunks more useful for both humans and AI.

### Why Knowledge Graphs, Not Tags?

**Decision**: Compute automatic relationships instead of requiring manual tags.

**Tradeoffs**:
| Advantage | Cost |
|-----------|------|
| No manual work (scales automatically) | Relationships are inferred, not authoritative |
| Discovers unexpected connections | May miss intended relationships |
| Works with existing markdown (no changes needed) | Requires tuning similarity threshold |

**Rationale**: Manual tagging is error-prone and doesn't scale. Computed relationships provide value without maintenance burden. Users can always add manual tags if needed.

### Why JSON, Not Database?

**Decision**: Output INDEX.json as serialized text, not a database.

**Tradeoffs**:
| Advantage | Cost |
|-----------|------|
| Portable (works anywhere) | Not efficient for very large datasets (100k+ docs) |
| Human-readable | Query performance slower than database |
| No external dependencies | Can't do complex queries directly |
| Version control friendly | Need to load entire file for any query |

**Rationale**: Portability and simplicity trump performance for current use cases. Future phase can add database backends if needed.

---

## Questions This Document Answers

### Q: Why no Cobra framework or CLI library boilerplate?
**A**: Pure, minimal CLI parsing. Every line of code must justify its existence. This keeps the system maintainable and understandable.

### Q: Why Result[T] instead of (T, error)?
**A**: Railway-Oriented Programming. Result types make error handling explicit and composable. Prevents silent failures and makes error paths testable.

### Q: Why hexagonal architecture?
**A**: Clear boundaries. Business logic is testable without I/O. Storage backend, graph algorithm, or indexing strategy can change without touching core logic.

### Q: Why one trace per command?
**A**: Complete context for debugging and auditing. When something fails, the trace shows exactly what was processed, with timestamps and decisions.

### Q: Why 90% coverage?
**A**: Catches regressions. With 90% coverage, most code paths are tested. Below 90%, untested edge cases often hide bugs.

### Q: Where do I start as a new developer?
**A**: Read ARCHITECTURE.md for design patterns, then CLAUDE.md for development workflow (TCR jail), then AGENTS.md for coding standards. Review existing code in chunks.rs to see patterns in action.

### Q: Can I add a database backend?
**A**: Yes. The adapter layer isolates I/O. Create a new adapter that implements the same interface as the JSON serializer. Core business logic is unaffected.

### Q: Can I customize the relationship algorithm?
**A**: Yes. The graph module computes relationships. Replace the similarity function without touching the rest of the pipeline.

### Q: Why can't I use git commit manually?
**A**: The TCR jail ensures quality. Automated tests run before any commit. If tests fail, changes revert immediately. This prevents broken code from reaching main.

---

## Related Documents

- **ARCHITECTURE.md**: System design and component structure in detail
- **CLAUDE.md**: TCR jail rules and development workflow
- **AGENTS.md**: AI agent coding standards and constraints
- **INDEXER.md**: Complete guide to transforming documentation

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-01-11 | Initial vision document; defines 5 pillars, design principles, success criteria, implementation strategy |

---

## Conclusion

**centralized-docs** exists to solve a real problem: documentation systems fail AI agents. By combining semantic chunking, knowledge graphs, and validation, we're building a system that works equally well for humans and machines.

Every design decision flows from the core philosophy: simple, reliable, portable, traceable, testable. Every feature serves the goal of making documentation discoverable and usable.

For developers: Read this document to understand why the project makes the choices it does. When you have a design question, come back here. The answers are here.

For AI agents: This document establishes the constraints and principles. Follow them, and your code will fit naturally into the system.

For users: This document explains what you're using and why it works. When you have questions about the system, the answers are here.
