# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-01-11

### Added
- Initial release of contextual-chunker
- Core chunking algorithm with semantic boundary detection
- 3-level hierarchical chunking:
  - Summary level (~128 tokens)
  - Standard level (~512 tokens)
  - Detailed level (~1024 tokens)
- Automatic parent-child relationship linking
- Sequential navigation (previous/next) links at same level
- Content type detection (code/table/prose)
- Extractive summary generation
- Full Unicode support (emoji, CJK, combining marks)
- Deterministic chunk generation
- `Document` input type for clean API
- `Chunk` output type with comprehensive metadata
- `ChunkingResult` for multi-document results
- `chunk()` function for single-level chunking
- `chunk_all()` function for hierarchical chunking
- Comprehensive documentation with examples
- Full test coverage including Unicode edge cases

### Design Decisions
- Used `LazyLock` for regex initialization (BEAD-006 compliant)
- Token estimation via character counting (4 chars ≈ 1 token)
- H2 headings (##) as primary chunk boundaries
- Context buffer (30-200 tokens) included in new chunks
- Minimal dependencies (regex, serde, anyhow only)

## Future (Roadmap)

### Planned for 0.2.0
- [ ] Custom chunk separators (configurable beyond H2)
- [ ] Token estimation plugins (OpenAI, Anthropic, etc.)
- [ ] Async chunking for large documents
- [ ] Streaming API for memory-constrained environments
- [ ] Chunk serialization formats (JSON, MessagePack, protobuf)

### Planned for 1.0.0
- [ ] Stable public API guarantee
- [ ] Zero-copy mode for huge documents
- [ ] Caching layer for repeated documents
- [ ] Integration with vector databases (pinecone, milvus, etc.)
- [ ] LLM-based chunk refinement (optional)

## Notes on Stability

### Breaking Changes
None planned for 0.x. Any breaking changes in future versions will be:
1. Announced in CHANGELOG with "BREAKING" marker
2. Provided with migration guide
3. Released only in major version bumps (1.0, 2.0, etc.)

### Stability Guarantees
- **0.1.0+**: `Chunk` struct fields are frozen (additions only, no removals/reorders)
- **0.1.0+**: `ChunkLevel` enum variants are frozen
- **0.1.0+**: `chunk()` and `chunk_all()` function signatures are stable
- **0.1.0+**: Token estimation algorithm locked (changes documented as algorithm updates)

### API Additions (Non-Breaking)
These can be added without version bump restrictions:
- New methods on existing types
- New optional builder parameters
- New helper functions
- New derive traits on existing types

### Data Stability
- Chunk IDs are deterministic (same content → same ID)
- Token counts within ±10% (algorithm locked after 0.1.0)
- Parent-child relationships form valid DAG (invariant maintained)

## Deprecation Policy

Any deprecated functionality will:
1. Be marked with `#[deprecated]` attribute
2. Include migration instructions in deprecation message
3. Provide examples of new approach
4. Have minimum 2 minor versions before removal (0.2.0 → 0.4.0+)
