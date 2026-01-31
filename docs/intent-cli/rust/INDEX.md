# ZJJ Complete Documentation Index

All ZJJ documentation in one place, token-efficient and searchable.

## 📋 Documents by Number

| # | Document | Purpose | Read Time |
|---|----------|---------|-----------|
| **00** | [START HERE](00_START_HERE.md) | 5-minute crash course + navigation | 5 min |
| **01** | [ERROR HANDLING](01_ERROR_HANDLING.md) | Fallible operations, Result patterns | 20 min |
| **02** | [MOON BUILD](02_MOON_BUILD.md) | Building, testing, caching | 15 min |
| **03** | [WORKFLOW](03_WORKFLOW.md) | Daily dev workflow (Beads + jj + Moon) | 20 min |
| **04** | [FUNCTIONAL PATTERNS](04_FUNCTIONAL_PATTERNS.md) | Iterator combinators, FP techniques | 25 min |
| **05** | [RUST STANDARDS](05_RUST_STANDARDS.md) | Zero unwrap/panic law + enforcement | 20 min |
| **06** | [COMBINATORS](06_COMBINATORS.md) | Complete combinator reference | Reference |
| **07** | [TESTING](07_TESTING.md) | Testing without panics | 15 min |
| **08** | [BEADS](08_BEADS.md) | Issue tracking, triage, graph metrics | 25 min |
| **09** | [JUJUTSU](09_JUJUTSU.md) | Version control, stacking commits | 20 min |
| **10** | [MOON CICD INDEXED](10_MOON_CICD_INDEXED.md) | Complete moon task catalog (indexed) | Reference |
| **INDEX** | This file | Document map | - |

## 🚀 Quick Navigation by Task

### I'm New Here
1. Read [00_START_HERE.md](00_START_HERE.md) (5 min)
2. Read [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) (20 min)
3. Read [02_MOON_BUILD.md](02_MOON_BUILD.md) (15 min)
4. Bookmark [06_COMBINATORS.md](06_COMBINATORS.md) for reference

### I'm Working on a Feature
1. See [08_BEADS.md](08_BEADS.md) - `bv --robot-triage` (pick task)
2. See [03_WORKFLOW.md](03_WORKFLOW.md) - Daily workflow
3. See [02_MOON_BUILD.md](02_MOON_BUILD.md) - Testing
4. See [09_JUJUTSU.md](09_JUJUTSU.md) - Committing & pushing

### How Do I Handle Errors?
→ [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) - 10 patterns with examples

### What Combinators Can I Use?
→ [06_COMBINATORS.md](06_COMBINATORS.md) - Complete reference

### How Do I Build/Test?
→ [02_MOON_BUILD.md](02_MOON_BUILD.md) - Commands and workflow

### What Are the Rules?
→ [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) - The law of zero panics

### How Do I Use Functional Programming?
→ [04_FUNCTIONAL_PATTERNS.md](04_FUNCTIONAL_PATTERNS.md) - FP techniques

### How Do I Pick Work?
→ [08_BEADS.md](08_BEADS.md) - Using `bv` for triage

### How Do I Commit & Push?
→ [09_JUJUTSU.md](09_JUJUTSU.md) - Version control

### How Do I Test Code?
→ [07_TESTING.md](07_TESTING.md) - Testing patterns

## 📚 By Topic

### The Core Law
- [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) - No unwrap, no panic, no unsafe

### Error Handling (The Most Important)
- [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) - 10 patterns + examples
- [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) - Requirements + enforcement
- [07_TESTING.md](07_TESTING.md) - Testing error paths

### Building & Testing
- [02_MOON_BUILD.md](02_MOON_BUILD.md) - Moon build system + caching (user guide)
- [10_MOON_CICD_INDEXED.md](10_MOON_CICD_INDEXED.md) - Complete moon task catalog (reference)
- [07_TESTING.md](07_TESTING.md) - Testing strategy

### Functional Programming
- [04_FUNCTIONAL_PATTERNS.md](04_FUNCTIONAL_PATTERNS.md) - FP patterns + libraries
- [06_COMBINATORS.md](06_COMBINATORS.md) - Combinator reference

### Development Tools
- [03_WORKFLOW.md](03_WORKFLOW.md) - Daily workflow integration
- [08_BEADS.md](08_BEADS.md) - Issue tracking + triage
- [09_JUJUTSU.md](09_JUJUTSU.md) - Version control

## 🔑 Key Commands Quick Reference

### Beads (Issues)
```bash
bv --robot-triage        # Get recommendations
bd claim BD-123          # Start working
bd complete BD-123       # Mark done
```

### Moon (Build)
```bash
moon run :ci    # Full pipeline
moon run :test  # Tests only
moon run :quick # Lint only
```

### Jujutsu (VCS)
```bash
jj describe -m "feat: description"  # Commit
jj git push                         # Push
```

### More Commands
See [02_MOON_BUILD.md](02_MOON_BUILD.md), [08_BEADS.md](08_BEADS.md), [09_JUJUTSU.md](09_JUJUTSU.md) for full command lists.

## 🎓 Learning Paths

### Path 1: Quick Start (1 hour)
1. [00_START_HERE.md](00_START_HERE.md) - 5 min
2. [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) - 20 min
3. [02_MOON_BUILD.md](02_MOON_BUILD.md) - 15 min
4. [03_WORKFLOW.md](03_WORKFLOW.md) - 20 min

### Path 2: Deep Dive (2 hours)
1. [00_START_HERE.md](00_START_HERE.md) - 5 min
2. [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) - 20 min
3. [04_FUNCTIONAL_PATTERNS.md](04_FUNCTIONAL_PATTERNS.md) - 25 min
4. [06_COMBINATORS.md](06_COMBINATORS.md) - 20 min
5. [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) - 20 min
6. [07_TESTING.md](07_TESTING.md) - 15 min

### Path 3: Practitioner (1.5 hours)
1. [03_WORKFLOW.md](03_WORKFLOW.md) - 20 min
2. [08_BEADS.md](08_BEADS.md) - 25 min
3. [09_JUJUTSU.md](09_JUJUTSU.md) - 20 min
4. [06_COMBINATORS.md](06_COMBINATORS.md) - 25 min

## 📊 Documentation Stats

- **Total Pages**: 11
- **Total Content**: ~52k tokens
- **Average Page**: ~4.7k tokens
- **Token Efficiency**: Highly optimized for AI + human reading
- **Latest Addition**: 10_MOON_CICD_INDEXED.md (comprehensive moon task catalog)

## 🔍 Search Guide

### By Error Type
→ [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) - All error patterns

### By Iterator Operation
→ [06_COMBINATORS.md](06_COMBINATORS.md) - Complete iterator reference

### By Testing Scenario
→ [07_TESTING.md](07_TESTING.md) - Test patterns

### Moon CICD Tasks & Pipelines
→ [10_MOON_CICD_INDEXED.md](10_MOON_CICD_INDEXED.md) - All 17 tasks + 4 pipelines indexed

### By Tool Command
- Beads: [08_BEADS.md](08_BEADS.md)
- Moon User Guide: [02_MOON_BUILD.md](02_MOON_BUILD.md)
- Moon CICD Reference: [10_MOON_CICD_INDEXED.md](10_MOON_CICD_INDEXED.md)
- Jujutsu: [09_JUJUTSU.md](09_JUJUTSU.md)

## 💡 Core Concepts Summary

| Concept | Location | Summary |
|---------|----------|---------|
| Zero Unwrap Law | [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) | No panics enforced by compiler |
| Result Pattern | [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) | All fallible ops return Result |
| Combinators | [06_COMBINATORS.md](06_COMBINATORS.md) | map, filter, fold, and_then, etc. |
| Functional Style | [04_FUNCTIONAL_PATTERNS.md](04_FUNCTIONAL_PATTERNS.md) | Immutability, composition, lazy eval |
| Moon Caching | [02_MOON_BUILD.md](02_MOON_BUILD.md) | Smart task skipping for speed |
| Beads Triage | [08_BEADS.md](08_BEADS.md) | Graph-aware issue prioritization |
| Jujutsu Stacking | [09_JUJUTSU.md](09_JUJUTSU.md) | Instant branches, reorderable commits |

## 🚫 What NOT to Do

**These cause compiler errors (good!)**:
- `unwrap()` - See [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md)
- `expect()` - See [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md)
- `panic!()` - See [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md)
- `unsafe { }` - See [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md)
- Direct `cargo` commands - Use `moon run` instead ([02_MOON_BUILD.md](02_MOON_BUILD.md))

## ✅ What TO Do

- Return `Result<T, Error>` for fallible ops ([01_ERROR_HANDLING.md](01_ERROR_HANDLING.md))
- Use `?` operator, match, or combinators ([01_ERROR_HANDLING.md](01_ERROR_HANDLING.md))
- Build with `moon run` ([02_MOON_BUILD.md](02_MOON_BUILD.md))
- Use functional patterns ([04_FUNCTIONAL_PATTERNS.md](04_FUNCTIONAL_PATTERNS.md))
- Test all paths ([07_TESTING.md](07_TESTING.md))

## 📞 Getting Help

1. **Quick question?** → Find in [00_START_HERE.md](00_START_HERE.md)
2. **Error handling?** → [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md)
3. **Which combinator?** → [06_COMBINATORS.md](06_COMBINATORS.md)
4. **Build issue?** → [02_MOON_BUILD.md](02_MOON_BUILD.md)
5. **Workflow question?** → [03_WORKFLOW.md](03_WORKFLOW.md)

## 🎯 The Philosophy

> "All fallible operations return `Result<T, Error>`. The compiler enforces this. We write safe, correct, idiomatic Rust without panics."

Everything in these docs supports this principle.

---

**Start Here**: [00_START_HERE.md](00_START_HERE.md)

**The Law**: No unwraps, no panics, no unsafe. Period.
