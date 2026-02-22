# zjj Restate Pivot - Comprehensive Plan

## Overview
Pivot from 137,000 lines of custom Rust infrastructure to ~10,000 lines using Restate for durable execution.

## The Breakdown

### What STAYS (Business Logic - ~8,000 lines)
| Module | Lines | Reason |
|--------|-------|--------|
| jj.rs | 1,075 | JJ git operations - core business logic |
| jj_operation_sync.rs | 791 | JJ operation locking |
| moon_gates.rs | 517 | Build/test gate execution |
| hooks.rs | 607 | User hook system |
| workspace_integrity.rs | 1,299 | Workspace validation logic |
| zellij.rs | 1,007 | Zellij integration |
| watcher.rs | 466 | File watching |
| worker_error.rs | 414 | Error types for workers |
| contracts.rs | 769 | Contract definitions |
| introspection.rs | 1,207 | Introspection utilities |
| hints.rs | 1,235 | Error hints |
| types.rs | 1,251 | Core types |
| Partial: error.rs | ~500 | Keep core errors, simplify Restate errors |
| Partial: config.rs | ~400 | Keep CLI config, remove state config |

### What ELIMINATES (Infrastructure - ~120,000 lines)
| Category | Lines | Replacement |
|----------|-------|-------------|
| coordination/ | 8,193 | Restate workflows |
| session_state.rs | 1,373 | Restate keyed services |
| recovery.rs | 485 | Restate durability |
| json.rs (serializers) | 1,997 | Restate handles |
| config.rs (80%) | ~1,900 | Restate env config |
| commands/ | 34,105 | Become thin Restate clients |
| database layer | ~2,000 | Restate state |
| locking everywhere | ~5,000 | Restate concurrency |
| self-healing code | ~1,500 | Restate handles |
| test infrastructure | ~20,000 | Simplified |
| Queue/Merge train code | ~10,000 | Restate workflows |
| State management | ~8,000 | Restate state |
| | **~127,000** | |

## Migration Strategy: 77 Beads

### Phase 1: Foundation (12 beads) - 4 hours
**Restate Setup (10 beads)**
036: restate-sdk-add - Add restate-sdk to Cargo.toml
037: restate-dev-server - Configure local Restate dev server
038: restate-service-scaffold - Create basic service structure
039: restate-http-client - Base HTTP client for Restate
040: restate-error-handling - Error types for Restate calls
041: restate-retry-policy - Retry logic for HTTP calls
042: restate-serde - Serialization for Restate
043: restate-tests-setup - Test infrastructure
044: restate-config - Config for Restate endpoint
045: restate-discovery - Service discovery

**JSONL (2 beads)**
046: jsonl-core-types - OutputLine enum + Restate metadata
047: jsonl-writer - Writer with Restate support

### Phase 2: Session Migration (10 beads) - 6 hours
**Session Service (8 beads)**
048: session-service-define - Define SessionService in Restate
049: session-get - GET operation with Restate
050: session-set - SET operation with Restate
051: session-delete - DELETE operation with Restate
052: session-list - LIST operation with Restate
053: session-query - Query operations
054: session-parent-child - Parent/child relationships
055: session-migration-tool - Migrate existing state.db

**Tests (2 beads)**
056: session-unit-tests - Unit tests for session service
057: session-integration-tests - Integration tests

### Phase 3: Queue Migration (14 beads) - 8 hours
**Queue Workflow (7 beads)**
058: queue-workflow-define - Define QueueWorkflow in Restate
059: queue-submit - Submit to workflow
060: queue-process - Process step in workflow
061: queue-retry - Retry with backoff
062: queue-fail - Failure handling
063: queue-complete - Completion step
064: queue-states - State management

**Merge Train Workflow (5 beads)**
065: train-workflow-define - Define TrainWorkflow
066: train-add - Add to train
067: train-process - Process train step
068: train-failure-rebase - Auto-rebase on failure
069: train-complete - Complete train

**Tests (2 beads)**
070: queue-unit-tests - Queue workflow tests
071: train-unit-tests - Train workflow tests

### Phase 4: Conflict Resolution (8 beads) - 4 hours
**Conflict Service (6 beads)**
072: conflict-service-define - Define ConflictService
073: conflict-analyze - Analysis as service
074: conflict-resolve - Resolution as service
075: conflict-journal - Audit via Restate journaling
076: conflict-quality - Quality signals
077: conflict-migration - Migrate conflict data

**Tests (2 beads)**
078: conflict-unit-tests - Conflict service tests
079: conflict-integration-tests - Integration tests

### Phase 5: Stack Commands (8 beads) - 4 hours
**Stack Service (6 beads)**
080: stack-service-define - Define StackService
081: stack-create - Create with parent
082: stack-list - List stacks
083: stack-restack - Reorder stacks
084: stack-dependencies - Dependency management
085: stack-migration - Migrate existing stacks

**Tests (2 beads)**
086: stack-unit-tests - Stack service tests
087: stack-integration-tests - Integration tests

### Phase 6: CLI Migration (20 beads) - 10 hours
**Command by Command Migration (15 beads)**
088: cmd-status-jsonl - Status via Restate
089: cmd-list-jsonl - List via Restate
090: cmd-add-jsonl - Add via Restate
091: cmd-remove-jsonl - Remove via Restate
092: cmd-sync-jsonl - Sync via Restate
093: cmd-focus-jsonl - Focus via Restate
094: cmd-queue-submit - Queue via Restate
095: cmd-stack-create - Stack via Restate
096: cmd-stack-list - Stack list via Restate
097: cmd-stack-restack - Stack restack via Restate
098: cmd-conflict-analyze - Conflict via Restate
099: cmd-conflict-resolve - Resolve via Restate
100: cmd-work - Worker via Restate
101: cmd-claim - Claim via Restate
102: cmd-spawn - Spawn via Restate

**CLI Cleanup (5 beads)**
103: remove-confirm - Remove interactive confirms
104: remove-force-flags - Remove --force flags
105: remove-color-deps - Remove color dependencies
106: remove-human-output - Remove human output
107: thin-cli-refactor - Thin CLI architecture

### Phase 7: Deletion (10 beads) - 4 hours
**Delete Infrastructure (8 beads)**
108: delete-coordination - Remove coordination/ (8,193 lines)
109: delete-database - Remove db.rs, SQL schemas
110: delete-recovery - Remove recovery.rs (485 lines)
111: delete-session-state - Remove session_state.rs (1,373 lines)
112: delete-json-serializers - Remove json.rs (1,997 lines)
113: delete-lock-usage - Remove all lock() calls
114: delete-config-state - Remove state management from config
115: delete-dead-tests - Remove tests for deleted code

**Verification (2 beads)**
116: verify-compilation - Verify everything builds
117: verify-no-dead-code - No dead code remaining

### Phase 8: Integration (10 beads) - 6 hours
**Integration & Testing (6 beads)**
118: integration-e2e-tests - Full end-to-end tests
119: migration-validation - Test state migration
120: performance-baseline - Performance comparison
121: load-tests - Load test Restate
122: chaos-tests - Chaos testing with Restate

**Documentation & CI (4 beads)**
123: docs-restate-architecture - Restate architecture docs
124: docs-migration-guide - Migration guide
125: ci-update-restate - Update CI for Restate
126: smoke-test-production - Smoke test

## Summary
- **Total beads**: 77 (up from 35, but different scope)
- **Estimated effort**: 42 hours (up from 26 hours, but worth it)
- **Code reduction**: 137k → 10k lines (93% reduction)
- **Business logic preserved**: ~8,000 lines
- **Infrastructure eliminated**: ~127,000 lines

## Timeline
- Week 1: Phases 1-3 (Foundation, Sessions, Queue) - 18 hours
- Week 2: Phases 4-6 (Conflict, Stacks, CLI) - 18 hours
- Week 3: Phases 7-8 (Deletion, Integration) - 14 hours
- Buffer & Testing: 8 hours

## Decision Matrix

| Option | Pro | Con |
|--------|-----|-----|
| **Complete 35-bead plan first** | Working system, understand pain points | 26h wasted on code to delete |
| **Pivot now to Restate** | Build right architecture from start | Delay features, learn while doing |
| **Hybrid: Core on Restate, keep current CLI** | Incremental migration, lower risk | Two systems to maintain |
| **Start new project "zjj-restate"** | Clean slate, no baggage | Split attention, duplicate work |

**Recommendation**: Start new "zjj-restate" project. Keep current zjj for reference. Migrate business logic modules directly (jj.rs, moon_gates.rs, hooks.rs, etc.) as-is to Restate services. Build CLI as thin Rust wrapper.
