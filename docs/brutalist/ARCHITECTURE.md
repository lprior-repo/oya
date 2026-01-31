# Factory - System Architecture

Contract-driven CI/CD pipeline for multi-language projects built in Gleam.

Status: Active development
Last Updated: January 29, 2026

## Core System

Factory creates isolated jj worktrees per task, runs language-specific validation stages, tracks progress via .factory/tasks.json.

## Key Components

CLI Interface (cli.gleam)
  new -s <slug>        Create task with isolated worktree
  stage -s <slug> --stage <name> [--from X] [--to Y]  Run pipeline stage
  approve -s <slug> [-f]  Mark task ready for integration
  show -s <slug>         Display task status
  list [--priority P1|P2|P3] [--status open|in_progress|done]  Query tasks

Pipeline Stages
  implement    - Code compiles (go build, gleam build, cargo build, py_compile)
  unit-test    - All tests pass (go test, gleam test, cargo test, pytest)
  coverage      - 80% coverage (language-specific tools)
  lint          - Code formatted (gofmt, gleam format, cargo fmt, black)
  static        - Static analysis passes (go vet, gleam check, clippy, mypy)
  integration   - Integration tests pass
  security      - No vulnerabilities (gosec, cargo audit, bandit)
  review        - Code review passes
  accept        - Ready for merge

Domain Model (domain.gleam)
  Task(slug, language, status, priority, worktree_path, branch)
  Stage(name, gate, retries)
  Language(Go|Gleam|Rust|Python|Javascript)
  TaskStatus(Created|InProgress|PassedPipeline|FailedPipeline|Integrated)
  Priority(P1|P2|P3)

Persistence (persistence.gleam)
  .factory/tasks.json - Task records
  .factory/audit.log - Event log
  JSON serialization with atomic updates

Integrations
  jj - Workspace isolation and management
  Language tools - Go, Gleam, Rust, Python, JavaScript
  Beads - Issue tracking (.beads/beads.jsonl with EARS format)

Testing
  gleeunit framework
  Unit tests, integration tests, property tests (qcheck)

## Development Directories

src/           - Core Gleam source code
test/          - Test files
docs/          - Documentation
examples/       - Example code
build/         - Gleam build packages
.claude/       - Claude configuration
.jjz/           - JJ workspace manager
.beads/        - Issue tracking database
.codanna/       - Code analysis and indexing
.planning/      - Planning and test analysis
.moon/          - Moon build tool
.factory/       - Factory task state
.tdd15-cache/  - TDD15 workflow cache
