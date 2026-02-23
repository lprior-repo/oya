# Martin Fowler Test Plan: Queue Lock and Merge-Decision Types

**Bead ID:** src-49q
**Approach:** Given-When-Then scenarios for all acceptance criteria and failure modes

---

## Happy Path Tests

### Queue Item Parsing

- `given_valid_fields_when_creating_queue_item_then_succeeds_with_wrapped_newtypes`
- `given_priority_1_when_creating_queue_item_then_succeeds`
- `given_priority_10_when_creating_queue_item_then_succeeds`
- `given_40_hex_sha_when_creating_queue_item_then_succeeds`

### Session Lock Management

- `given_valid_inputs_when_creating_session_lock_then_succeeds`
- `given_valid_lock_when_checking_expiration_before_expiry_then_false`
- `given_valid_lock_when_checking_expiration_at_expiry_then_true`
- `given_valid_lock_when_verifying_ownership_with_matching_token_then_true`

### Queue Selection

- `given_single_ready_item_when_selecting_then_returns_ready_decision`
- `given_multiple_items_by_priority_when_selecting_then_returns_highest_priority`
- `given_ready_item_with_lock_held_when_selecting_then_returns_ready`
- `given_completed_merge_when_selecting_then_returns_merged_decision`

### Merge Decision

- `given_lock_held_and_dependencies_ready_when_deriving_then_returns_merge`
- `given_no_lock_when_deriving_then_returns_requeue_with_lock_unavailable`
- `given_dependencies_not_ready_when_deriving_then_returns_requeue_with_dependency_pending`

---

## Error Path Tests

### Queue Item Parsing - Missing Fields

- `given_missing_id_when_creating_queue_item_then_returns_missing_field_error`
- `given_missing_bead_id_when_creating_queue_item_then_returns_missing_field_error`
- `given_missing_workspace_when_creating_queue_item_then_returns_missing_field_error`
- `given_missing_priority_when_creating_queue_item_then_returns_missing_field_error`
- `given_missing_freshness_base_rev_when_creating_queue_item_then_returns_missing_field_error`

### Queue Item Parsing - Empty Fields

- `given_empty_id_when_creating_queue_item_then_returns_missing_field_error`
- `given_whitespace_only_bead_id_when_creating_queue_item_then_returns_missing_field_error`
- `given_empty_workspace_when_creating_queue_item_then_returns_missing_field_error`

### Queue Item Parsing - Invalid Priority

- `given_priority_zero_when_creating_queue_item_then_returns_invalid_state_error`
- `given_priority_eleven_when_creating_queue_item_then_returns_invalid_state_error`
- `given_priority_twenty_when_creating_queue_item_then_returns_invalid_state_error`

### Queue Item Parsing - Invalid SHA

- `given_sha_39_chars_when_creating_queue_item_then_returns_invalid_state_error`
- `given_sha_41_chars_when_creating_queue_item_then_returns_invalid_state_error`
- `given_sha_with_non_hex_chars_when_creating_queue_item_then_returns_invalid_state_error`
- `given_sha_with_spaces_when_creating_queue_item_then_returns_invalid_state_error`

### Queue Item Parsing - Control Characters

- `given_id_with_control_chars_when_creating_queue_item_then_returns_invalid_state_error`
- `given_bead_id_with_null_byte_when_creating_queue_item_then_returns_invalid_state_error`

### Session Lock - Invalid Inputs

- `given_empty_token_when_creating_session_lock_then_returns_missing_field_error`
- `given_zero_ttl_when_creating_session_lock_then_returns_invalid_state_error`
- `given_ttl_resulting_in_expiry_before_acquisition_when_creating_session_lock_then_returns_invalid_state_error`

---

## Edge Case Tests

### Priority Boundaries

- `given_priority_boundary_1_when_creating_queue_item_then_succeeds`
- `given_priority_boundary_10_when_creating_queue_item_then_succeeds`
- `given_priority_boundary_0_when_creating_queue_item_then_fails`
- `given_priority_boundary_11_when_creating_queue_item_then_fails`

### SHA Boundaries

- `given_sha_exactly_40_hex_chars_when_creating_queue_item_then_succeeds`
- `given_sha_39_chars_when_creating_queue_item_then_fails`
- `given_sha_41_chars_when_creating_queue_item_then_fails`
- `given_sha_all_lowercase_hex_when_creating_queue_item_then_succeeds`
- `given_sha_all_uppercase_hex_when_creating_queue_item_then_succeeds`

### Lock Expiration

- `given_lock_just_acquired_when_checking_expiration_then_false`
- `given_lock_exactly_at_expiry_when_checking_expiration_then_true`
- `given_lock_one_second_past_expiry_when_checking_expiration_then_true`
- `given_lock_far_past_expiry_when_checking_expiration_then_true`

### Empty Queue

- `given_empty_queue_when_selecting_then_returns_idle_or_blocked`
- `given_empty_queue_with_no_lock_when_selecting_then_returns_idle`
- `given_empty_queue_with_lock_held_when_selecting_then_returns_blocked`

### Single Item Queue

- `given_single_ready_item_when_selecting_then_returns_ready`
- `given_single_stale_item_when_selecting_then_returns_stale`
- `given_single_conflict_item_when_selecting_then_returns_conflict`

---

## Contract Verification Tests

### Precondition Verification

- `test_precondition_queue_item_missing_required_fields_fails`
- `test_precondition_queue_item_priority_out_of_range_fails`
- `test_precondition_queue_item_sha_not_40_chars_fails`
- `test_precondition_queue_item_sha_not_hex_fails`
- `test_precondition_lock_token_empty_fails`
- `test_precondition_lock_ttl_zero_fails`
- `test_precondition_lock_expires_before_acquired_fails`

### Postcondition Verification

- `test_postcondition_queue_item_all_fields_wrapped_in_newtypes`
- `test_postcondition_session_lock_expires_at_is_acquired_plus_ttl`
- `test_postcondition_selection_decision_is_exhaustive_matchable`
- `test_postcondition_merge_decision_is_exhaustive_matchable`

### Invariant Verification

- `test_invariant_priority_always_1_to_10`
- `test_invariant_sha_always_exactly_40_hex_chars`
- `test_invariant_lock_expires_after_acquired`
- `test_invariant_lock_expiration_monotonic`
- `test_invariant_only_one_merge_item_global`
- `test_invariant_selection_deterministic_for_same_snapshot`
- `test_invariant_merge_decision_matching_exhaustive`

---

## Given-When-Then Scenarios

### Scenario 1: Valid Queue Item Creation

**Given:**
- id = "queue-item-1"
- bead_id = "src-abc123"
- workspace = "/path/to/workspace"
- priority = 5
- freshness_base_rev = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
- state = "Ready"

**When:** `QueueItem::try_new(id, bead_id, workspace, priority, freshness_base_rev, state)` is called

**Then:**
- Returns `Ok(QueueItem)`
- `item.id.as_str()` returns "queue-item-1"
- `item.bead_id.as_str()` returns "src-abc123"
- `item.priority.as_u8()` returns 5
- `item.sha.as_str()` returns 40-char SHA
- `item.freshness_base_rev.as_str()` returns 40-char SHA
- All fields are wrapped in newtypes

---

### Scenario 2: Invalid Priority (Zero) - AC1: Field-Scoped Parse Error

**Given:**
- id = "queue-item-1"
- bead_id = "src-abc123"
- workspace = "/path/to/workspace"
- priority = 0
- freshness_base_rev = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
- state = "Ready"

**When:** `QueueItem::try_new(id, bead_id, workspace, priority, freshness_base_rev, state)` is called

**Then:**
- Returns `Err(ValidationError)`
- Error is `ValidationError::InvalidState("priority must be > 0")`
- No `QueueItem` is created
- Error message includes field name "priority"

---

### Scenario 3: Invalid Priority (> 10) - AC1: Field-Scoped Parse Error

**Given:**
- id = "queue-item-1"
- bead_id = "src-abc123"
- workspace = "/path/to/workspace"
- priority = 11
- freshness_base_rev = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
- state = "Ready"

**When:** `QueueItem::try_new(id, bead_id, workspace, priority, freshness_base_rev, state)` is called

**Then:**
- Returns `Err(ValidationError)`
- Error is `ValidationError::InvalidState("priority must be <= 10")`
- No `QueueItem` is created
- Error message includes field name "priority"

---

### Scenario 4: Invalid SHA Length (39 chars) - AC1: Field-Scoped Parse Error

**Given:**
- id = "queue-item-1"
- bead_id = "src-abc123"
- workspace = "/path/to/workspace"
- priority = 5
- freshness_base_rev = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" (39 chars)
- state = "Ready"

**When:** `QueueItem::try_new(id, bead_id, workspace, priority, freshness_base_rev, state)` is called

**Then:**
- Returns `Err(ValidationError)`
- Error is `ValidationError::InvalidState("sha must be 40 characters")`
- No `QueueItem` is created
- Error message indicates constraint violation

---

### Scenario 5: Invalid SHA (Non-hex) - AC1: Field-Scoped Parse Error

**Given:**
- id = "queue-item-1"
- bead_id = "src-abc123"
- workspace = "/path/to/workspace"
- priority = 5
- freshness_base_rev = "gggggggggggggggggggggggggggggggggggggggg"
- state = "Ready"

**When:** `QueueItem::try_new(id, bead_id, workspace, priority, freshness_base_rev, state)` is called

**Then:**
- Returns `Err(ValidationError)`
- Error is `ValidationError::InvalidState("sha must be hexadecimal")`
- No `QueueItem` is created
- Error message indicates constraint violation

---

### Scenario 6: Missing Field (Empty bead_id) - AC1: Field-Scoped Parse Error

**Given:**
- id = "queue-item-1"
- bead_id = ""
- workspace = "/path/to/workspace"
- priority = 5
- freshness_base_rev = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
- state = "Ready"

**When:** `QueueItem::try_new(id, bead_id, workspace, priority, freshness_base_rev, state)` is called

**Then:**
- Returns `Err(ValidationError)`
- Error is `ValidationError::MissingField("bead_id")`
- No `QueueItem` is created
- Error message explicitly names the missing field

---

### Scenario 7: Valid Session Lock Creation

**Given:**
- token = "worker-1"
- acquired_at = 1000
- ttl_seconds = 60

**When:** `SessionLock::try_new(token, acquired_at, ttl_seconds)` is called

**Then:**
- Returns `Ok(SessionLock)`
- `lock.token.as_str()` returns "worker-1"
- `lock.acquired_at` returns 1000
- `lock.expires_at` returns 1060
- Lock is not expired at time 1050

---

### Scenario 8: Invalid Session Lock (Zero TTL)

**Given:**
- token = "worker-1"
- acquired_at = 1000
- ttl_seconds = 0

**When:** `SessionLock::try_new(token, acquired_at, ttl_seconds)` is called

**Then:**
- Returns `Err(ValidationError)`
- Error is `ValidationError::InvalidState("ttl_seconds must be > 0")`
- No `SessionLock` is created

---

### Scenario 9: Invalid Session Lock (Empty Token)

**Given:**
- token = ""
- acquired_at = 1000
- ttl_seconds = 60

**When:** `SessionLock::try_new(token, acquired_at, ttl_seconds)` is called

**Then:**
- Returns `Err(ValidationError)`
- Error is `ValidationError::MissingField("lock_token")`
- No `SessionLock` is created

---

### Scenario 10: Lock Expiration

**Given:**
- A session lock with acquired_at=1000, ttl_seconds=60 (expires_at=1060)

**When:** `lock.is_expired(now_epoch_seconds)` is called with various times

**Then:**
- `is_expired(1000)` returns `false` (just acquired)
- `is_expired(1050)` returns `false` (10 seconds before expiry)
- `is_expired(1059)` returns `false` (1 second before expiry)
- `is_expired(1060)` returns `true` (exactly at expiry)
- `is_expired(1061)` returns `true` (1 second past expiry)
- `is_expired(2000)` returns `true` (far past expiry)

---

### Scenario 11: Lock Ownership Verification

**Given:**
- A session lock with token="worker-1"

**When:** `lock.is_owned_by(candidate_token)` is called

**Then:**
- `is_owned_by("worker-1")` returns `true`
- `is_owned_by("worker-2")` returns `false`
- `is_owned_by("")` returns `false`

---

### Scenario 12: Queue Selection - Ready Item

**Given:**
- Queue snapshot with one ready item (priority=5, valid SHA)
- No current lock held
- now_epoch_seconds = 1000
- main_revision matches item's freshness_base_rev

**When:** `select_next_merge_candidate(queue_snapshot, None, now_epoch_seconds, main_revision)` is called

**Then:**
- Returns `Ok(SelectionDecision::Ready { queue_item: ... })`
- Ready decision contains the queue item
- No lock is required for Ready decision

---

### Scenario 13: Queue Selection - Blocked by Lock

**Given:**
- Queue snapshot with one ready item (priority=5)
- Current lock held by "worker-1" acquired_at=1000, expires_at=1100
- now_epoch_seconds = 1050
- main_revision matches item's freshness_base_rev

**When:** `select_next_merge_candidate(queue_snapshot, Some(&lock), now_epoch_seconds, main_revision)` is called

**Then:**
- Returns `Ok(SelectionDecision::Blocked { reason: ... })`
- Blocked reason contains `BlockReason::LockUnavailable { owner: Some("worker-1"), expires_at: Some(1100) }`
- No item is selected

---

### Scenario 14: Queue Selection - Stale Base Revision

**Given:**
- Queue snapshot with one item (priority=5, freshness_base_rev="aaaa...")
- No current lock
- now_epoch_seconds = 1000
- main_revision = "bbbb..." (different from item's freshness_base_rev)

**When:** `select_next_merge_candidate(queue_snapshot, None, now_epoch_seconds, main_revision)` is called

**Then:**
- Returns `Ok(SelectionDecision::Stale { reason: ... })`
- Stale reason contains `StaleReason::BaseRevisionAdvanced`
- No item is selected

---

### Scenario 15: Merge Decision - Proceed with Merge

**Given:**
- queue_position = 1
- lock = Some(LockToken("worker-1"))
- dependencies_ready = true

**When:** `derive_merge_decision(queue_position, lock, dependencies_ready)` is called

**Then:**
- Returns `MergeDecision::Merge { queue_position: 1, lock: LockToken("worker-1") }`

---

### Scenario 16: Merge Decision - Requeue (Lock Unavailable)

**Given:**
- queue_position = 1
- lock = None
- dependencies_ready = true

**When:** `derive_merge_decision(queue_position, lock, dependencies_ready)` is called

**Then:**
- Returns `MergeDecision::Requeue { reason: MergeBlockReason::LockUnavailable, queue_position: 2 }`

---

### Scenario 17: Merge Decision - Requeue (Dependency Pending)

**Given:**
- queue_position = 1
- lock = Some(LockToken("worker-1"))
- dependencies_ready = false

**When:** `derive_merge_decision(queue_position, lock, dependencies_ready)` is called

**Then:**
- Returns `MergeDecision::Requeue { reason: MergeBlockReason::DependencyPending, queue_position: 2 }`

---

### Scenario 18: Deterministic Selection - AC5: Same Queue → Same Item

**Given:**
- Queue snapshot with items: [id="A", priority=10], [id="B", priority=5]
- No current lock
- now_epoch_seconds = 1000
- main_revision matches all items

**When:** `select_next_merge_candidate` is called twice with identical inputs

**Then:**
- First call returns `SelectionDecision::Ready { queue_item: id="A" }`
- Second call returns `SelectionDecision::Ready { queue_item: id="A" }`
- Both selections are identical

---

### Scenario 19: Lock Owner Release - AC4: Only Owner Can Release

**Given:**
- Session lock owned by "worker-1"

**When:** Lock release is attempted by "worker-1"

**Then:**
- Release succeeds with `Ok(())`
- Lock is removed from tracking

---

### Scenario 20: Lock Non-Owner Release Fails - AC4: Only Owner Can Release

**Given:**
- Session lock owned by "worker-1"

**When:** Lock release is attempted by "worker-2"

**Then:**
- Release fails with `Err(ValidationError::InvalidState("caller does not own lock"))`
- Lock remains held by "worker-1"

---

### Scenario 21: Lock Reclamation by First Worker - AC4: Expired Lock Reclaim Is Single-Winner

**Given:**
- Expired lock owned by "worker-1" (expires_at=1000)
- Worker-2 detects expiry at now=1000
- Worker-3 detects expiry at now=1001

**When:** Both workers attempt to reclaim the lock

**Then:**
- First worker to attempt reclamation succeeds
- Second worker's attempt fails (lock already claimed)
- Only one winner exists

---

### Scenario 22: At Most One Merging Item - AC3: Single Merge Global

**Given:**
- Queue snapshot with multiple items
- One item is currently in `Merging` state

**When:** `select_next_merge_candidate` is called

**Then:**
- Returns `SelectionDecision::Blocked { reason: LockUnavailable { owner: Some(...), expires_at: ... } }`
- No additional item is selected
- At most one merging item is active

---

### Scenario 23: Exhaustive Merge Decision Matching - AC2: Compile-Enforced

**Given:**
- All variants of `MergeDecision`: `Merge`, `Requeue`, `Block`

**When:** Rust compiler type-checks match expression

**Then:**
- Match must handle all three variants explicitly
- Using `_` wildcard causes compile error
- Missing variant causes compile error

---

### Scenario 24: Exhaustive Selection Decision Matching - AC2: Compile-Enforced

**Given:**
- All variants of `SelectionDecision`: `Ready`, `Blocked`, `Stale`, `Conflict`, `Merged`

**When:** Rust compiler type-checks match expression

**Then:**
- Match must handle all five variants explicitly
- Using `_` wildcard causes compile error
- Missing variant causes compile error

---

## Test Names Summary

### Happy Path (8 tests)
1. `given_valid_fields_when_creating_queue_item_then_succeeds_with_wrapped_newtypes`
2. `given_priority_1_when_creating_queue_item_then_succeeds`
3. `given_priority_10_when_creating_queue_item_then_succeeds`
4. `given_40_hex_sha_when_creating_queue_item_then_succeeds`
5. `given_valid_inputs_when_creating_session_lock_then_succeeds`
6. `given_valid_lock_when_checking_expiration_before_expiry_then_false`
7. `given_single_ready_item_when_selecting_then_returns_ready_decision`
8. `given_lock_held_and_dependencies_ready_when_deriving_then_returns_merge`

### Error Path (20 tests)
1. `given_missing_id_when_creating_queue_item_then_returns_missing_field_error`
2. `given_missing_bead_id_when_creating_queue_item_then_returns_missing_field_error`
3. `given_missing_workspace_when_creating_queue_item_then_returns_missing_field_error`
4. `given_missing_priority_when_creating_queue_item_then_returns_missing_field_error`
5. `given_missing_freshness_base_rev_when_creating_queue_item_then_returns_missing_field_error`
6. `given_empty_id_when_creating_queue_item_then_returns_missing_field_error`
7. `given_whitespace_only_bead_id_when_creating_queue_item_then_returns_missing_field_error`
8. `given_empty_workspace_when_creating_queue_item_then_returns_missing_field_error`
9. `given_priority_zero_when_creating_queue_item_then_returns_invalid_state_error`
10. `given_priority_eleven_when_creating_queue_item_then_returns_invalid_state_error`
11. `given_priority_twenty_when_creating_queue_item_then_returns_invalid_state_error`
12. `given_sha_39_chars_when_creating_queue_item_then_returns_invalid_state_error`
13. `given_sha_41_chars_when_creating_queue_item_then_returns_invalid_state_error`
14. `given_sha_with_non_hex_chars_when_creating_queue_item_then_returns_invalid_state_error`
15. `given_sha_with_spaces_when_creating_queue_item_then_returns_invalid_state_error`
16. `given_id_with_control_chars_when_creating_queue_item_then_returns_invalid_state_error`
17. `given_bead_id_with_null_byte_when_creating_queue_item_then_returns_invalid_state_error`
18. `given_empty_token_when_creating_session_lock_then_returns_missing_field_error`
19. `given_zero_ttl_when_creating_session_lock_then_returns_invalid_state_error`
20. `given_ttl_resulting_in_expiry_before_acquisition_when_creating_session_lock_then_returns_invalid_state_error`

### Edge Case (14 tests)
1. `given_priority_boundary_1_when_creating_queue_item_then_succeeds`
2. `given_priority_boundary_10_when_creating_queue_item_then_succeeds`
3. `given_priority_boundary_0_when_creating_queue_item_then_fails`
4. `given_priority_boundary_11_when_creating_queue_item_then_fails`
5. `given_sha_exactly_40_hex_chars_when_creating_queue_item_then_succeeds`
6. `given_sha_39_chars_when_creating_queue_item_then_fails`
7. `given_sha_41_chars_when_creating_queue_item_then_fails`
8. `given_sha_all_lowercase_hex_when_creating_queue_item_then_succeeds`
9. `given_sha_all_uppercase_hex_when_creating_queue_item_then_succeeds`
10. `given_lock_just_acquired_when_checking_expiration_then_false`
11. `given_lock_exactly_at_expiry_when_checking_expiration_then_true`
12. `given_empty_queue_when_selecting_then_returns_idle_or_blocked`
13. `given_single_ready_item_when_selecting_then_returns_ready`
14. `given_single_stale_item_when_selecting_then_returns_stale`

### Contract Verification (7 tests)
1. `test_precondition_queue_item_missing_required_fields_fails`
2. `test_precondition_queue_item_priority_out_of_range_fails`
3. `test_precondition_queue_item_sha_not_40_chars_fails`
4. `test_postcondition_queue_item_all_fields_wrapped_in_newtypes`
5. `test_postcondition_selection_decision_is_exhaustive_matchable`
6. `test_invariant_priority_always_1_to_10`
7. `test_invariant_selection_deterministic_for_same_snapshot`

**Total Tests: 49**

---

## Exit Criteria

All tests must:

1. **Compile** - Test code compiles without warnings
2. **Fail Initially** - Tests for new behavior fail before implementation
3. **Pass After Implementation** - All tests pass after code is written
4. **Be Explicit** - No `_` wildcard matches in enum handling (compile-enforced exhaustive)
5. **Be Deterministic** - Given same inputs, always produce same outputs
6. **Be Field-Scoped** - Parse errors include field name in message
7. **Validate Invariants** - Every invariant has a corresponding test
8. **Cover All Error Variants** - Each `ValidationError` variant has ≥1 test
