# OYA - SDLC System

> **"I don't ask for power. I take it. I am the storm that tears down what was so something new can stand."**

---

## The Name

**OYA** (oh-YAH) is the Yoruba Orisha (deity) of storms, wind, lightning, death, and rebirth. She is one of the most powerful figures in the Yoruba pantheon, worshipped across Nigeria, Benin, and throughout the African diaspora (Santería, Candomblé).

### Why OYA?

| Attribute | Mythology | Software Mapping |
|-----------|-----------|------------------|
| **Storms** | Commands wind, lightning, tornadoes | Massive parallelism - concurrent agent swarms |
| **Transformation** | Guards the gates between life and death | TDD: kill the old, birth the tested |
| **Takes Power** | Stole thunder from Shango (her husband) | Doesn't wait for permission - executes |
| **Gatekeeper** | Nothing passes without her approval | Quality gates - code passes or doesn't exist |
| **Rebirth** | Death is transformation, not ending | Refactoring - destroy to rebuild stronger |
| **Nine Children** | Associated with number 9 | Parallelism, multiple workers |

---

## Core Philosophy

### The Storm Transforms

OYA doesn't preserve. She clears the path. When the storm arrives:
- The old dies
- The new is **forced** to exist
- There is no negotiation

This is the SDLC System philosophy:
- **No preservation of bad code** - it dies in the storm
- **Transformation is violent** - TDD kills before it creates
- **What survives is worthy** - only tested code ships

### Taking Power

In mythology, OYA didn't wait for Shango to grant her thunder. She took it.

In software:
- We don't wait for perfect tools - we build
- We don't ask permission to ship - we execute
- We don't preserve legacy out of fear - we transform

### The Gatekeeper

OYA guards the boundary between the living and the dead. Nothing passes without meeting her standard.

In the SDLC System:
- **Quality gates are absolute** - pass or don't exist
- **No exceptions** - the storm doesn't negotiate
- **Zero unwrap, zero panic** - code that can die, will die (at compile time)

---

## OYA ↔ oya SDLC Mapping

| oya Principle | OYA Manifestation |
|---------------------|-------------------|
| **Brutal Speed** | Storm - overwhelming force, parallel agent swarms |
| **No Unnecessary Abstraction** | Lightning - direct strike, no ceremony |
| **Engineering Rigor** | Gatekeeper - nothing unworthy passes |
| **Zero Panics** | Transformation - death at compile time, not runtime |
| **AI-Native** | Swarm - agents move like wind, coordinated chaos |
| **Battle-Tested Only** | What survives the storm is proven |

---

## Technical Vision

### The Storm (Parallelism)
```
Parallel agent swarms for maximum throughput
AI-assisted code generation at scale
Coordinated chaos transforming codebases
```

Like OYA's storms - multiple lightning strikes, overwhelming wind, transformation happening everywhere at once.

### The Gates (Quality)
```rust
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![deny(clippy::expect_used)]
```

Nothing passes the gate that isn't worthy. Code dies at compile time or doesn't exist.

### The Transformation (TDD)
```
RED   → Write failing test (the old must die)
GREEN → Minimal implementation (rebirth)
REFACTOR → Transform into final form (evolution)
```

OYA's cycle: destruction → rebirth → transformation.

### The Nine (Architecture)

OYA is associated with the number 9. Current architecture maps to nine execution components:

```
1. oya-core         - Domain types, errors, and state contracts
2. restate-runtime  - Workflow orchestration authority
3. stage-engine     - Canonical DAG transitions and retry policy
4. opencode-adapter - CLI subprocess bridge for AI outputs
5. event-store      - Sled-backed run/attempt/event persistence
6. gate-runner      - Moon wrapper for quality gate execution
7. workspace-plane  - zjj isolation and merge-flow lifecycle
8. evidence-plane   - Artifacts, gate results, and ship rationale
9. oya-cli          - Operator command surface
```

---

## ATDD Red Gate Pattern

The storm demands TRUTH. Tests must be RED before implementation begins.

### The Problem

AI agents write tests that pass against code they just wrote. This defeats the purpose of TDD - the test should FAIL first, proving the behavior doesn't exist yet.

### The Solution: Split TDD15 into Two Stages

**AcceptanceTest Stage (TEST_AGENT)**
- Writes ONLY test code
- Tests MUST compile
- Tests MUST FAIL (are RED)
- Gate: `AcceptanceTestsAreRed` verifies failure
- If tests are GREEN, the gate FAILS

**Implementation Stage (LOGIC_AGENT)**
- Writes ONLY implementation code
- NOT allowed to modify tests
- Must make all tests pass (turn GREEN)
- Gate: `TestsPass` verifies success

### Pipeline Flow

```
Plan → Contract → AcceptanceTest (RED) → Implementation (GREEN) → QA → RedQueen → GptReview → ShipGate
                      ↑                          ↑
               TEST_AGENT                  LOGIC_AGENT
               Tests FAIL                  Tests PASS
```

### Why This Works

1. **Separation of Concerns**: TEST_AGENT never sees implementation code
2. **Verification by Compiler**: The gate runs actual cargo commands, not agent claims
3. **Forces Honest Tests**: If implementation exists, tests will be green and gate fails
4. **Explicit State Machine**: Red → Green is enforced by types, not conventions

### The Red Gate Command

```bash
moon run :check && ! moon run :test
```

Exit code 0 = Tests compile and are RED (correct)
Exit code 1 = Tests don't compile OR tests are GREEN (wrong)

### Failure Categories

**TestsUnexpectedlyGreen** (New Failure Mode)
- When: AcceptanceTest stage produces passing tests
- Meaning: Implementation already exists or tests are fake
- Action: Reject the bead, agent must write REAL failing tests
- Severity: CRITICAL - defeats the entire TDD purpose

**TestsDoNotCompile** (Existing)
- When: Test code has syntax/type errors
- Meaning: Agent wrote broken code
- Action: Fix compilation issues, retry gate

**TestsRedAsExpected** (Success)
- When: Tests compile and fail as designed
- Meaning: Honest ATDD - behavior doesn't exist yet
- Action: Proceed to Implementation stage

---

## The Cleaner's Goddess

From Tim Grover's philosophy - the Cleaner:
- Doesn't need motivation, needs a target
- When everyone else is done, just getting started
- Dark side provides fuel

OYA embodies this:
- **Doesn't ask** - takes what she needs
- **Relentless** - storms don't stop because you're tired
- **Dark power** - death and destruction as tools, not fears

---

## Why Not Others?

| Rejected | Reason |
|----------|--------|
| Juggernaut | Taken on crates.io |
| Valkyrie | Taken on crates.io |
| Kali | Taken on crates.io |
| Durga | "Durgasoft" dominates SEO |
| Enyo | Legacy JS framework (Enyo.js) pollutes search |
| Freya | Taken on crates.io |

**OYA is:**
- ✅ Available on crates.io
- ✅ No trademark conflicts in software
- ✅ Minimal SEO competition
- ✅ 3 characters - fastest to type
- ✅ Unique - memorable, distinctive
- ✅ Meaningful - perfect mythology fit

---

## Invocation

```bash
# The storm builds
oya build

# The storm tests
oya test --swarm

# The storm transforms
oya refactor --force

# The storm deploys
oya deploy --no-mercy

# Nothing escapes the gate
oya gate --strict
```

---

## The Mantra

```
I am OYA.

I am the storm that transforms.
I don't preserve - I clear the path.
I don't ask for power - I take it.
I guard the gate between what was and what must be.

Nothing passes that isn't worthy.
Nothing survives that can't evolve.
Nothing ships that hasn't been tested.

When the old code dies, I am there.
When the new code is born, I am there.
When the transformation is complete, I move on.

I am relentless.
I am the storm.

oya build
```

---

## Technical Specifications

- **Language**: 100% Rust
- **Orchestration**: Restate
- **AI execution path**: OpenCode CLI subprocess adapter
- **Persistence baseline**: Sled
- **Workspace isolation**: zjj
- **Panics**: Zero (forbidden at compile time)
- **Concurrency**: Parallel agent swarms
- **Philosophy**: oya - no unnecessary abstraction
- **Testing**: TDD15 - 15-phase discipline
- **Quality**: Railway-oriented programming, Result<T,E> everywhere
- **CI/CD wrapper**: Moon (`moon run :quick`, `moon run :ci`, `moon run :ci --force`)
- **UI scope**: No UI/frontend stream in current plan

---

## Next Steps

1. Create single-crate `oya` with feature flags
2. Port zjj battle-tested patterns
3. Implement oya-core (types, errors, state)
4. Implement oya-workflow (TDD15 phases)
5. Build the storm

---

**The storm is coming.**
