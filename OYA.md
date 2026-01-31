# OYA - SDLC Factory

> **"I don't ask for power. I take it. I am the storm that tears down what was so something new can stand."**

---

## The Name

**OYA** (oh-YAH) is the Yoruba Orisha (deity) of storms, wind, lightning, death, and rebirth. She is one of the most powerful figures in the Yoruba pantheon, worshipped across Nigeria, Benin, and throughout the African diaspora (Santería, Candomblé).

### Why OYA?

| Attribute | Mythology | Software Mapping |
|-----------|-----------|------------------|
| **Storms** | Commands wind, lightning, tornadoes | Massive parallelism - 100 concurrent agents |
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

This is the SDLC Factory philosophy:
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

In the SDLC Factory:
- **Quality gates are absolute** - pass or don't exist
- **No exceptions** - the storm doesn't negotiate
- **Zero unwrap, zero panic** - code that can die, will die (at compile time)

---

## OYA ↔ Brutalist SDLC Mapping

| Brutalist Principle | OYA Manifestation |
|---------------------|-------------------|
| **Brutal Speed** | Storm - overwhelming force, 100 concurrent beads |
| **No Unnecessary Abstraction** | Lightning - direct strike, no ceremony |
| **Engineering Rigor** | Gatekeeper - nothing unworthy passes |
| **Zero Panics** | Transformation - death at compile time, not runtime |
| **AI-Native** | Swarm - agents move like wind, coordinated chaos |
| **Battle-Tested Only** | What survives the storm is proven |

---

## Technical Vision

### The Storm (Parallelism)
```
100 concurrent beads
~100k LOC/hour generation capacity
AI agent swarms executing in parallel
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

OYA is associated with the number 9. The architecture:

```
1. oya-core       - Foundation types, errors, state machine
2. oya-workflow   - Intra-bead workflow engine
3. oya-events     - Inter-bead event sourcing
4. oya-reconciler - K8s-style reconciliation
5. oya-tdd15      - TDD15 phase definitions
6. oya-intent     - EARS/KIRK requirement decomposition
7. oya-zjj        - Workspace isolation
8. oya-docs       - Documentation indexing
9. oya-cli        - Unified command interface
```

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
oya build --parallel 100

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
- **Panics**: Zero (forbidden at compile time)
- **Concurrency**: 100+ parallel beads
- **Throughput**: ~100k LOC/hour
- **Philosophy**: Brutalist - no unnecessary abstraction
- **Testing**: TDD15 - 15-phase discipline
- **Quality**: Railway-oriented programming, Result<T,E> everywhere

---

## Next Steps

1. Create single-crate `oya` with feature flags
2. Port zjj battle-tested patterns
3. Implement oya-core (types, errors, state)
4. Implement oya-workflow (TDD15 phases)
5. Build the storm

---

**The storm is coming.**
