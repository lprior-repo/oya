# OYA UI MacroMarkdown - Canvas-First Workflow UX

## Product intent

Build a workflow interface that feels simple, trustworthy, and fast to debug.

The UI must optimize for three outcomes:

1. I can see what the AI wrote.
2. I can see what it is doing.
3. I can drag/drop and visualize.

Core principle:

> Workflow document, execution journal, and canvas rendering are one coherent truth source.

---

## Information architecture

The product has **three primary screens** and **one secondary prototype entry point**.

## 1) Canvas / Editor (primary)

The default and most important screen.

- Renders workflow as a **DAG**.
- Nodes = steps.
- Edges = dependencies.
- Layout = automatic left-to-right by dependency order.
- Canvas is code-driven: source definition is parsed, graph computed.
- During execution, nodes and edges reflect live status.
- Clicking a node (live or historical) opens inspector.

### Canvas behaviors

- Infinite canvas, free pan/zoom.
- Left-to-right is convention and visual default.
- No forced manual layout constraints for users.
- Node status colors during execution:
  - idle/pending = neutral
  - running = blue
  - succeeded = green
  - failed = red
- Data flow animates across edges while active.
- Global run bar shows: `Running step X of Y - <step name>`.

---

## 2) Inspector Panel

Opens from the right when a node is clicked during or after execution.

### Inspector header fields

- step name
- step type
- status
- start time
- end time
- duration
- attempt number

### Inspector body

- Tab 1: **Input** (exact JSON that entered the step)
- Tab 2: **Output**
  - exact JSON output if success
  - full error plus stack trace if failed

### Inspector UX requirements

- Syntax highlighting for JSON and stack traces.
- Search box for payload search.
- Copy button for raw payload/error.
- No transformed or summarized payload unless explicitly toggled.

---

## 3) Execution History

A table of all past runs for the current workflow.

### Columns

- execution ID
- status
- start time
- duration
- steps completed
- steps failed

### Interactions

- Click a row -> open execution on canvas in frozen mode.
- Frozen mode preserves exact node states and I/O.
- Clicking nodes in frozen mode still opens inspector.

---

## Secondary entry point: Prototype mode

Prototype mode is a convenience path, not the primary authoring path.

- Users can drag nodes from a palette and sketch quickly.
- System generates a code skeleton from this sketch.
- Intended for onboarding and rough prototyping.
- Production path remains code-first.

---

## Non-negotiable behaviors

## 10-second failed-run diagnosis

Within 10 seconds of opening a failed execution, the user can:

1. identify the red failed node,
2. see its error message,
3. open the exact input that caused failure,

with no multi-screen navigation burden.

## Canvas never lies

- Canvas state is derived directly from execution journal.
- No drifting intermediate view model.
- What user sees is exactly what happened.

## Full-fidelity replay

- Historical execution rehydrates exact run state.
- Node-level input/output must match recorded journal entries exactly.

---

## Visual design rules

1. Progressive disclosure first.
2. Busyness must earn its place.
3. Dense, information-rich nodes over large empty cards.
4. Keep always-visible controls minimal.
5. Put detail in inspector/history, not permanent chrome.

---

## Minimal persistent UI shell

Always visible:

- workflow name
- run control
- save/export control
- global execution status line
- canvas

On-demand:

- inspector panel
- history screen
- prototype palette

---

## Node card requirements

Each node should show enough to understand flow without opening panels.

- label and type
- current status
- key config hint
- brief output preview when available (3-5 lines + `...`)

Deep payload inspection stays in inspector tabs.

---

## Data contracts for UI fidelity

UI-level execution record should support:

- execution metadata (id, status, start, end, duration)
- per-step metadata (status, attempt, timing)
- per-step exact input payload
- per-step exact output payload or error+stack
- deterministic node identity mapping to graph

Historical replay requires preserving enough snapshot context to render exactly what the user saw at runtime.

---

## Acceptance criteria

1. User can open workflow and understand topology at a glance.
2. Running workflow clearly shows active step and flow progression.
3. Failed run triage (red node -> error -> causative input) is achievable in <=10 seconds.
4. History row click reliably restores a frozen, inspectable past run.
5. Canvas representation stays journal-faithful with no drift.
6. Prototype mode can generate a valid starter code skeleton.

---

## v0 prompt usage note

Use this document as the UI product contract for v0 mockups.

When prompting v0, ask for:

1. Canvas screen with live execution overlays.
2. Inspector panel with Input/Output tabs and searchable JSON.
3. Execution history table and frozen replay state.
4. Prototype mode palette screen generating skeleton output.
