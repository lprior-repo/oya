# Contract Specification: Queue Lock and Merge-Decision Types

**Bead ID:** src-49q
**Purpose:** Define typed domain contracts for queue lock management and merge decision selection with compile-time safety and exhaustive matching.

---

## Context

### Feature
Define compile-safe contracts for queue lock ownership, merge decision selection, and deterministic candidate selection from queue snapshots.

### Domain Terms
- **Queue Snapshot**: Immutable collection of queue items at a point in time
- **Queue Item**: Serializable record with `id`, `bead_id`, `workspace`, `priority`, `freshness_base_rev`, `state`
- **Selection Decision**: Outcome of selecting next merge candidate (`Ready`, `Blocked`, `Stale`, `Conflict`, `Merged`)
- **Session Lock**: Token-based lock with ownership and expiration (`token`, `acquired_at`, `expires_at`)
- **Merge Decision**: Action to take for a queue item (`Merge`, `Requeue`, `Block`)

### Assumptions
- Queue records come from external serialized sources (files, network)
- Lock expiration is checked against epoch seconds in UTC
- At most one queue item can be in `Merging` state globally
- Lock reclamation is single-winner when multiple workers detect expiry
- Same queue snapshot (same items, same order) always selects same next item

### Open Questions
- None

---

## Preconditions

### Queue Item Parsing
- Input record must have all required fields: `id`, `bead_id`, `workspace`, `priority`, `freshness_base_rev`, `state`
- `priority` must be numeric and in range `1..=10`
- `freshness_base_rev` must be exactly 40 hexadecimal characters
- String fields must be non-empty after trimming
- String fields must not contain forbidden control characters (except `\n`, `\r`, `\t`)

### Lock Acquisition
- Worker must provide a non-empty lock token
- TTL must be > 0 seconds
- `expires_at` must be > `acquired_at`

### Lock Release
- Caller must hold the lock (token matches owner)
- `now_epoch_seconds` must be valid (non-zero, reasonable)

### Queue Selection
- Queue snapshot must be valid (all items pass parsing)
- At most one item can be in `Merging` state globally
- Lock state must be available for querying

---

## Postconditions

### Queue Item Parsing Success
- Returns `QueueItem` with all fields validated as newtypes
- All newtypes wrap validated values (e.g., `NonZeroPriority(5)`, `FullSha("aaaa...")`)
- Input is not modified

### Queue Item Parsing Failure
- Returns `ValidationError` with field-level context
- `MissingField` variant indicates which field is missing/empty
- `InvalidState` variant indicates constraint violation with reason
- No partial state is created

### Lock Acquisition Success
- Returns `SessionLock` with `token`, `acquired_at`, `expires_at`
- Lock is now considered owned by `token`
- `expires_at` = `acquired_at` + `ttl_seconds`
- No other lock exists for the resource

### Lock Acquisition Failure
- Returns `ValidationError::InvalidState` if TTL is 0
- Returns `ValidationError::MissingField` if token is empty
- Returns `ValidationError::InvalidState` if `expires_at` ≤ `acquired_at`

### Lock Release Success
- Lock is removed from tracking
- Resource is now available for acquisition
- Returns `Ok(())`

### Lock Release Failure
- Returns `ValidationError::InvalidState` if caller does not own lock
- Returns `ValidationError::InvalidState` if lock is not found

### Queue Selection Success
- Returns `SelectionDecision` with exhaustive variant
- `Ready` → contains `QueueItem` ready to merge
- `Blocked` → contains `BlockReason` (lock unavailable or dependencies pending)
- `Stale` → contains `StaleReason` (base revision advanced or conflict detected)
- `Conflict` → contains conflicting `QueueBeadId`
- `Merged` → contains completed `QueueBeadId` and `QueuePosition`
- Decision is deterministic for same queue snapshot and state

### Queue Selection Failure
- Returns `ValidationError` if any queue item fails parsing
- Returns `ValidationError` if queue snapshot is invalid

---

## Invariants

### Queue Item Invariants
- `priority` is always in range `1..=10`
- `freshness_base_rev` is always exactly 40 hex characters
- `id` is always non-empty and control-character-clean
- `bead_id` is always non-empty and control-character-clean
- `workspace` is always non-empty and control-character-clean
- `state` is a valid state enum value

### Session Lock Invariants
- `token` is always non-empty and control-character-clean
- `expires_at` is always > `acquired_at`
- `ttl_seconds` is always > 0
- Lock expiration is monotonic (once expired, stays expired)

### Selection Invariants
- At most one `QueueItem` can be in `Merging` state globally
- Same queue snapshot + same state → same selection (deterministic)
- `MergeDecision` matching is compile-time exhaustive (no `_` wildcard)
- All queue items in snapshot are validated before selection
- Selection preserves priority ordering (higher priority selected first)

---

## Error Taxonomy

All queue/lock operations return `Result<T, ValidationError>`.

### ValidationError Variants

```rust
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    /// A required field is missing or empty
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// A field contains a placeholder value
    #[error("Placeholder value in {0}: {1}")]
    PlaceholderValue(String, String),

    /// An invariant violation occurred
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// An exit code is out of valid range
    #[error("Invalid exit code: {0}")]
    InvalidExitCode(i32),

    /// Evidence is inconsistent with claims
    #[error("Inconsistent evidence: {0}")]
    InconsistentEvidence(String),
}
```

### Error Variant Usage

| Error Variant | When to Use | Example Message |
|----------------|-------------|------------------|
| `MissingField` | Field is `None`, `""`, or whitespace-only | `"Missing required field: priority"` |
| `InvalidState` | Constraint violation, out-of-range, or invariant breach | `"Invalid state: priority must be in 1..=10"` |
| `PlaceholderValue` | Field contains placeholder text | `"Placeholder value in bead_id: todo"` |

### Field-Scoped Parse Errors

Parsing failures include the field name in the error:

- `"Missing required field: priority"` → `priority` field is missing/empty
- `"Invalid state: priority must be in 1..=10"` → `priority` is out of range
- `"Invalid state: sha must be 40 characters"` → `freshness_base_rev` has wrong length
- `"Invalid state: sha must be hexadecimal"` → `freshness_base_rev` has non-hex chars
- `"Missing required field: queue_item_id"` → `id` field is missing

---

## Contract Signatures

### Queue Item Parsing

```rust
/// Parse a raw serialized queue record into a validated QueueItem
///
/// # Returns
/// - `Ok(QueueItem)` with all fields wrapped in validated newtypes
/// - `Err(ValidationError)` with field-scoped parse diagnostics
pub fn parse_queue_record(
    raw: &SerializedQueueRecord,
) -> Result<QueueItem, ValidationError>;

/// Try to create a QueueItem from raw field values (parse boundary)
///
/// # Returns
/// - `Ok(QueueItem)` if all fields validate
/// - `Err(ValidationError)` with field name in message
impl QueueItem {
    pub fn try_new(
        id: &str,
        bead_id: &str,
        workspace: &str,
        priority: u8,
        freshness_base_rev: &str,
        state: &str,
    ) -> Result<Self, ValidationError>;
}
```

### Priority Validation

```rust
/// Validated priority: 1-10 inclusive (non-zero)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NonZeroPriority(u8);

impl TryFrom<u8> for NonZeroPriority {
    type Error = ValidationError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == 0 {
            return Err(ValidationError::InvalidState(
                "priority must be > 0".to_string(),
            ));
        }
        if value > 10 {
            return Err(ValidationError::InvalidState(
                "priority must be <= 10".to_string(),
            ));
        }
        Ok(Self(value))
    }
}
```

### Freshness SHA Validation

```rust
/// Validated 40-character hexadecimal SHA
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FullSha(String);

impl TryFrom<&str> for FullSha {
    type Error = ValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let trimmed = value.trim();
        if trimmed.len() != 40 {
            return Err(ValidationError::InvalidState(
                "sha must be 40 characters".to_string(),
            ));
        }
        if !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ValidationError::InvalidState(
                "sha must be hexadecimal".to_string(),
            ));
        }
        Ok(Self(trimmed.to_string()))
    }
}
```

### Session Lock Management

```rust
/// Token-based session lock with expiration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionLock {
    pub token: LockToken,
    pub acquired_at: u64,
    pub expires_at: u64,
}

impl SessionLock {
    /// Create a new session lock
    ///
    /// # Returns
    /// - `Ok(SessionLock)` if token non-empty, ttl > 0, expires_at > acquired_at
    /// - `Err(ValidationError)` otherwise
    pub fn try_new(
        token: &str,
        acquired_at: u64,
        ttl_seconds: u64,
    ) -> Result<Self, ValidationError>;

    /// Check if lock is expired at given epoch seconds
    #[must_use]
    pub fn is_expired(&self, now_epoch_seconds: u64) -> bool;

    /// Verify ownership of lock
    #[must_use]
    pub fn is_owned_by(&self, token: &str) -> bool;
}
```

### Selection Decision

```rust
/// Exhaustive decision for queue item selection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionDecision {
    /// Item is ready to merge
    Ready { queue_item: QueueItem },
    /// Item is blocked (lock unavailable or dependencies pending)
    Blocked { reason: BlockReason },
    /// Item is stale (base advanced or conflict detected)
    Stale { reason: StaleReason },
    /// Conflict detected with another bead
    Conflict { bead_id: QueueBeadId },
    /// Item was already merged
    Merged { bead_id: QueueBeadId, queue_position: QueuePosition },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockReason {
    LockUnavailable { owner: Option<String>, expires_at: Option<u64> },
    DependencyPending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaleReason {
    BaseRevisionAdvanced,
    ConflictDetected,
}

/// Select next merge candidate from queue snapshot
///
/// # Returns
/// - `Ok(SelectionDecision)` with exhaustive variant based on queue/state
/// - `Err(ValidationError)` if queue snapshot is invalid
pub fn select_next_merge_candidate(
    queue_snapshot: &[QueueItem],
    current_lock: Option<&SessionLock>,
    now_epoch_seconds: u64,
    main_revision: &FullSha,
) -> Result<SelectionDecision, ValidationError>;
```

### Merge Decision

```rust
/// Exhaustive merge decision (compile-time enforced matching)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum MergeDecision {
    /// Proceed with merge
    Merge { queue_position: QueuePosition, lock: LockToken },
    /// Requeue for later processing
    Requeue { reason: MergeBlockReason, queue_position: QueuePosition },
    /// Block processing indefinitely
    Block { reason: MergeBlockReason },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeBlockReason {
    LockUnavailable,
    DependencyPending,
    QueueConflict,
}

/// Derive merge decision from queue position and lock state
///
/// # Returns
/// - `MergeDecision::Merge` if lock held and dependencies ready
/// - `MergeDecision::Requeue` if lock unavailable or dependencies pending
/// - `MergeDecision::Block` if queue conflict
#[must_use]
pub fn derive_merge_decision(
    queue_position: QueuePosition,
    lock: Option<LockToken>,
    dependencies_ready: bool,
) -> MergeDecision;
```

---

## Non-Goals

- ~~Implement actual lock storage~~ - This is a contract definition; storage implementation is separate
- ~~Implement queue persistence~~ - This contract defines validation and selection logic only
- ~~Implement worker coordination protocol~~ - Only defines types and invariants
- ~~Add authentication/authorization~~ - Lock is token-based, not user-based
- ~~Support priority inversion~~ - Priority is strictly higher-is-better, no dynamic adjustment
- ~~Support partial queue selection~~ - Always selects exactly one candidate or returns none
