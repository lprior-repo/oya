# Oya Ubiquitous Language

> "A vocabulary of domain-driven design" - Evans, Fowler, North

## Core Domain Terms

| Term | Definition | Example |
|------|------------|---------|
| **Bead** | Unit of work from Steve Yegge's beads system | `oya-a1b2` |
| **Pipeline** | CI/CD-style flow for software creation | Contract → Shipped |
| **Run** | One execution of a Bead through the Pipeline | - |
| **Stage** | Discrete step in Pipeline | Explore, Contract, Red, Implementation, Witness, ShipGate |
| **Gate** | Quality check at end of each Stage | lint, compile, test |
| **Attempt** | One try at passing a Stage | max 2 per Stage |
| **Shipped** | Bead merged to main, passed ALL gates | - |
| **OpenCode Adapter** | CLI subprocess path for AI stage execution | `opencode run ...` |
| **Orchestrator** | Restate workflow runtime governing state transitions | Restate service |

## Pipeline Stages (in order)

```
Explore → Contract → Red → Implementation → Witness → ShipGate → [SHIP]
```

Each Stage has:
- **Input**: Bead + context from previous stages
- **Model**: Token-efficient LLM for this stage
- **Output**: Artifacts + validation
- **Decision**: Pass → next Stage, Fail → Retry (max 3) or Block

## Retry Logic

```
Stage starts
    ↓
Attempt 1: Process
    ↓
Gate passes? ──No──→ Retry with adjusted context
    │                    ↓
   Yes               Attempt 2: Process
    │                    ↓
    ↓               Gate passes? ──No──→ Retry with more context
Stage complete              ↓
   Yes               Attempt 3: Process
    │                    ↓
    ↓               Gate passes? ──No──→ Block (max retries)
Stage advance               ↓
                          Yes
                           ↓
                    Stage complete
                           ↓
                    Stage advance
```

## Model Selection (Token Efficiency)

Use the right model for the right stage:

| Stage | Model Profile | Reasoning |
|-------|--------------|-----------|
| **Contract** | Fast, cheap | Simple specification writing |
| **Red** | Balanced | Acceptance tests are created and sealed in RED state |
| **Implementation** | Balanced | Production code turns sealed tests GREEN |
| **Witness** | Capable | Hidden holdout scenario verification |
| **ShipGate** | Best | Final validation requires maximum capability |

## State Machine

```
Pending → Running(Stage) → Waiting → Shipped
                    ↓              ↓
                  Blocked      Failed
```

## Events

| Event | Trigger | Response |
|-------|---------|----------|
| `BeadSubmitted` | User: "get to work" | Create Run |
| `StageStarted` | Agent begins Stage | Log event |
| `StagePassed` | Gate validates | Advance to next Stage |
| `StageFailed` | Gate fails | Retry or Block |
| `RunShipped` | ShipGate passes + merge | Mark Bead as Shipped |
| `RunBlocked` | Max retries exceeded | Mark Bead as Blocked |

## Quality Gates

Each Stage has specific Gates:

| Stage | Gates |
|-------|-------|
| Explore | None |
| Contract | Compiles |
| Red | Compiles |
| Implementation | Compiles + Tests Pass |
| Witness | Holdout Scenarios |
| ShipGate | CUE Artifact + jj Bookmark |

## Aggregates

- **Run**: Aggregate root - owns Bead lifecycle
- **StageAttempt**: Entity - one attempt at a Stage
- **Artifact**: Value Object - output from Stages

## Dependencies

- Steve Yegge's Beads (task tracking)
- Restate (stateful orchestration)
- Sled (embedded DB)
- OpenCode CLI (AI subprocess execution)
- jj (workspace isolation + merge flow)
