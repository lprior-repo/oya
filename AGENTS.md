{"kind":"meta","version":"1.0","updated":"2026-02","project":"oya"}
{"kind":"mandate","id":"moon-only","text":"MANDATORY: Use ONLY moon for ALL build/test/lint tasks. NEVER use cargo directly. Violation is a workflow failure."}
{"kind":"mandate","id":"codanna-only-discovery","text":"MANDATORY: Use ONLY Codanna MCP for code discovery (symbols, callers, calls, impact, dependency tracing)."}
{"kind":"rule","id":"no-glob-read-grep-explore","text":"FORBIDDEN for exploration: glob/read/grep/find/rg. Use only after Codanna identifies exact path/symbol, or for non-indexed artifacts."}
{"kind":"rule","id":"exploration-ladder","text":"Use this order: codanna_search_symbols -> codanna_find_symbol -> codanna_analyze_impact -> targeted read."}
{"kind":"rule","id":"cheap-defaults","text":"Default limits: search limit <= 5, impact depth <= 2, single-pass batched queries, no repeated re-query of same symbol."}
{"kind":"rule","id":"response-budget","text":"Default response budget: <= 8 lines, no redundant recap, no chain-of-thought, include only decision/files/next-action."}
{"kind":"policy","id":"non-codanna-explore-failure","text":"Exploration with glob/read/grep before a Codanna attempt is a workflow failure unless user explicitly requests it."}
{"kind":"cmd","tool":"codanna_mcp","list":["codanna_search_symbols","codanna_find_symbol","codanna_get_calls","codanna_find_callers","codanna_analyze_impact","codanna_semantic_search_with_context","codanna_get_index_info"]}
{"kind":"guide","id":"git-only-vcs","text":"Use Git/GitHub for version-control, branch, and PR flow. Do not require alternate version-control tools for active Oya workflows."}
{"kind":"skill","load":"/planner","when":"first","purpose":"deterministic bead decomposition"}
{"kind":"skill","load":"/functional-rust-generator","when":"coding","purpose":"zero-panic rust"}
{"kind":"skill","load":"/rust-contract","when":"planning","purpose":"contracts + tests"}
{"kind":"skill","load":"/red-queen","when":"qa","purpose":"adversarial regression and durability gates"}
{"kind":"workflow","name":"bead","steps":["bd ready","bd update <id> --status in_progress","oya lifecycle --bead <id> --repo priorlewis43/oya","moon run :ci","git fetch origin","git rebase origin/main","git push -u origin HEAD:<name>","bd close <id>","bd sync --flush-only"]}
{"kind":"workflow","name":"self-build","steps":["planner init/add-task/process (create contract-grade beads)","oya lifecycle --bead <id> --repo priorlewis43/oya","oya doctor (runtime + deployment invariants)","red-queen gate (adversarial QA against lifecycle + status)","moon run :ci","gh pr create"]}
{"kind":"guide","id":"self-build-workflow","text":"Self-build workflow for oya implementation beads. Applies when: implementing features/fixes for oya itself. Skills: (1) planner - decomposes requirements into contract-grade beads with 16-section template, validates against CUE schema; (2) rust-contract - generates design-by-contract specs and Martin Fowler Given-When-Then test plans, handles Result types and error taxonomy; (3) red-queen - adversarial evolutionary QA that drives selection/regression gates, co-evolves code and tests. Order: planner first (bead decomposition) -> rust-contract (spec generation) -> implementation -> red-queen (adversarial QA) -> moon run :ci -> gh pr create. Distinguishes from regular bead workflow by using planner/rust-contract/red-queen skills instead of manual bead creation."}
{"kind":"cmd","tool":"bd","list":["ready","show <id>","update <id> --status in_progress","close <id>","sync --flush-only","dolt push","dolt pull"]}
{"kind":"guide","id":"bd-basic","text":"Basic bd workflow: 1) `bd ready` lists available beads 2) `bd show <id>` displays details 3) `bd update <id> --status in_progress` claims bead 4) work in a Git branch or worktree when isolation is required 5) `bd close <id>` marks complete 6) `bd sync --flush-only` persists state"}
{"kind":"skill","load":"/beads","when":"bead-work","purpose":"externalize executive function via beads dolt graph"}
{"kind":"cmd","tool":"moon","list":["run :quick","run :ci","run :test","run :fmt-fix","run :build","run :check","run :coverage","run :mutants-quick"]}
{"kind":"cmd","tool":"git","list":["status --short","fetch origin","rebase origin/main","switch -c <branch>","worktree add <path> <branch>","push -u origin HEAD:<branch>","log --oneline -n 10"]}
{"kind":"rule","id":"moon","text":"NEVER cargo. moon run only."}
{"kind":"rule","id":"panic","text":"Zero unwrap/panic/expect. Result<T,E> + ?"}
{"kind":"rule","id":"tdd","text":"Tests FIRST. RED-GREEN-REFACTOR."}
{"kind":"rule","id":"clippy","text":"Fix code, never lint config."}
{"kind":"rule","id":"fn-lines","text":"Source functions must be <= 40 lines (clippy::too_many_lines)."}
{"kind":"rule","id":"fn-args","text":"Source functions must take <= 5 inputs (clippy::too_many_arguments)."}
{"kind":"rule","id":"workspace","text":"Use Git branch/worktree isolation before starting work when isolation is required."}
{"kind":"rule","id":"planner-contract-first","text":"Every non-trivial change starts with planner bead decomposition and rust-contract artifacts before implementation."}
{"kind":"rule","id":"self-build-required","text":"Oya must self-build through `oya lifecycle` for implementation beads; manual one-off edits are only for emergency hotfixes."}
{"kind":"rule","id":"no-empty-pr","text":"PR creation is forbidden when lifecycle diff contains only .beads or no meaningful source changes."}
{"kind":"rule","id":"qa-red-queen","text":"Before merge, run adversarial QA with red-queen and keep regression evidence in status/PR output."}
{"kind":"lint","rust":"#![deny(clippy::unwrap_used)] #![deny(clippy::expect_used)] #![deny(clippy::panic)] #![deny(clippy::too_many_lines)] #![deny(clippy::too_many_arguments)] #![forbid(unsafe_code)]"}
{"kind":"land","steps":["moon run :ci","git fetch origin","git rebase origin/main","git status --short","bd close <id>","bd sync --flush-only","git add .beads/","git commit","git push -u origin HEAD:<branch>"]}
{"kind":"ref","moon":"/home/lewis/src/oya/.moon/tasks.yml"}
{"kind":"ref","rust":"/home/lewis/src/oya/docs/FUNCTIONAL_RUST.md"}
{"kind":"ref","beads":"/home/lewis/src/oya/docs/BEADS.md"}
{"kind":"restate","ui":"http://localhost:9070","ingress":"http://localhost:909","service":"http://localhost:9180","default_runtime":"oya init"}
{"kind":"rule","id":"runtime-init","text":"Please use `oya init` to bootstrap local runtime. Use `oya init --down` to stop Docker Restate."}
{"kind":"cmd","tool":"restate","list":["oya init (fresh Docker Restate + handler registration + validations)","oya init --down (stop Docker Restate)","http://localhost:9070 (Admin/UI)","http://localhost:909 (Ingress API)","http://localhost:909/restate/health (Health)","http://localhost:9180/discover (Oya discovery endpoint)","http://localhost:909/Oya/<key>/run (workflow run endpoint)","http://localhost:909/OyaService/get_lifecycle (status endpoint)","http://localhost:909/OyaService/cancel (cancel endpoint)","http://localhost:909/OyaMemory/<id>/start (memory start endpoint)","http://localhost:909/OyaMemory/<id>/run_pipeline (memory pipeline endpoint)"]}
{"kind":"observability","name":"OpenObserve","ui":"http://localhost:5080","otlp_grpc":"localhost:4317","otlp_http":"http://localhost:4318","credentials":"~/.local/share/observability/.env"}
{"kind":"cmd","tool":"observability","list":["systemctl --user start observability.service (start stack)","systemctl --user stop observability.service (stop stack)","systemctl --user status observability.service (check status)","~/.local/share/observability/observability.sh start (alt start)","~/.local/share/observability/observability.sh stop (alt stop)","~/.local/share/observability/observability.sh logs [service] (view logs)","~/.local/share/observability/observability.sh ui (open browser)","~/.local/share/observability/observability.sh creds (show credentials)"]}
{"kind":"env","name":"OTEL_EXPORTER_OTLP_ENDPOINT","value":"http://localhost:4318","purpose":"OTLP exporter endpoint for traces/metrics/logs"}
{"kind":"env","name":"OTEL_SERVICE_NAME","value":"oya-orchestrator","purpose":"Service name in OpenObserve"}
{"kind":"ref","observability":"~/.local/share/observability/README.md"}
{"kind":"rule","id":"clippy-test-exemption","text":"Test files (tests/*.rs, src/lib_tests.rs) are EXEMPT from clippy::unwrap_used for brevity in assertions. CLIPPY task excludes tests."}

<!-- BEGIN BEADS INTEGRATION -->
## Issue Tracking with bd (beads)

**IMPORTANT**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why bd?

- Dependency-aware: Track blockers and relationships between issues
- Git-friendly: Dolt-powered version control with native sync
- Agent-optimized: JSON output, ready work detection, discovered-from links
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**

```bash
bd ready --json
```

**Create new issues:**

```bash
bd create "Issue title" --description="Detailed context" -t bug|feature|task -p 0-4 --json
bd create "Issue title" --description="What this issue is about" -p 1 --deps discovered-from:bd-123 --json
```

**Claim and update:**

```bash
bd update <id> --claim --json
bd update bd-42 --priority 1 --json
```

**Complete work:**

```bash
bd close bd-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task atomically**: `bd update <id> --claim`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" --description="Details about what was found" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`

### Auto-Sync

bd automatically syncs via Dolt:

- Each write auto-commits to Dolt history
- Use `bd dolt push`/`bd dolt pull` for remote sync
- No manual export/import needed!

### Important Rules

- ✅ Use bd for ALL task tracking
- ✅ Always use `--json` flag for programmatic use
- ✅ Link discovered work with `discovered-from` dependencies
- ✅ Check `bd ready` before asking "what should I work on?"
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems

For more details, see README.md and docs/QUICKSTART.md.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

<!-- END BEADS INTEGRATION -->
