# Contract Specification: Oya Run Orchestration

## Context
- **Feature**: Run Orchestration (Bead/Stage Lifecycle)
- **Domain Terms**:
    - **Run**: A single execution workflow for a Bead (Unit of work).
    - **Bead**: The item being worked on (Issue/Task).
    - **Stage**: A distinct phase of work (Contract, TDD, QA, etc.).
    - **Gate**: A check that must pass to transition states.
    - **Artifact**: Output produced by a stage.

## Invariants
1.  **Identity**: Every Run has a unique, immutable RunID.
2.  **Monotonic Time**: `created_at` <= `updated_at`. `started_at` <= `completed_at`.
3.  **State Consistency**:
    - A `Completed` stage attempt must have a `completed_at` timestamp.
    - A `Shipped` run implies all required stages passed.
    - A `Failed` run implies at least one stage failed or a critical error occurred.
4.  **Stage Progression**: Stages execute in a defined order (Pipeline).

## Error Taxonomy
- `DomainError::InvalidStateTransition`: Attempting to move from a terminal state (e.g., Failed) to Active.
- `DomainError::MissingArtifact`: Required artifact not found for stage completion.
- `DomainError::GateCheckFailed`: Criteria not met.
- `DomainError::StaleData`: Update applied to old version.

## Functional Core Signatures
- `fn start_run(bead_id: BeadId, now: DateTime<Utc>) -> Result<Run, DomainError>`
- `fn complete_stage(run: Run, stage: Stage, result: StageResult, now: DateTime<Utc>) -> Result<Run, DomainError>`
- `fn fail_stage(run: Run, stage: Stage, error: FailureCategory, now: DateTime<Utc>) -> Result<Run, DomainError>`
- `fn ship_run(run: Run, decision: ShipDecision, now: DateTime<Utc>) -> Result<Run, DomainError>`

## Types (Value Objects)
- `RunId`: Wrapper around ULID.
- `BeadId`: Wrapper around String (or specific format).
- `Run`: The Aggregate Root.
