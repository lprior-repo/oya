# Contract Specification

## Context
- Feature: schema: add cue queue and lock validation artifacts
- Domain terms:
  - QueueItem: bead queued for merge with id, bead_id, priority, sha, freshness_base_rev
  - SessionLock: lock token with acquired_at, expires_at, ttl_seconds
  - CUE schema: declarative validation schema for structured data
- Assumptions:
  - Queue items are serialized as JSON
  - Schema validation happens before queue processing

## Preconditions
- [x] Queue schema file exists and is loadable (cue/queue_schema.cue)
- [x] Lock schema file exists and is loadable (cue/lock_schema.cue)
- [x] Artifacts are serialized as JSON compatible payloads

## Postconditions
- [x] Valid records pass cue vet
- [x] Invalid records produce actionable field-level failure output

## Invariants
- [x] Schema version is explicit
- [x] All required keys present for each record kind (bead_id, priority, sha)

## Error Taxonomy
- `ValidationError::MissingField` - required field absent
- `ValidationError::InvalidState` - field fails validation (bounds, format)
- Schema validation errors - CUE-level validation failures

## Contract Signatures
```rust
// Schema validation functions
pub fn validate_queue_item(item: &QueueItem) -> Result<(), ValidationError>
pub fn validate_session_lock(lock: &SessionLock) -> Result<(), ValidationError>
```

## Non-goals
- [ ] Runtime schema registration (compile-time only)
- [ ] Schema versioning (single version)
