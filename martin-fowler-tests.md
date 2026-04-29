# Martin Fowler Test Plan: Oya Core & Frontend Boundary

## Happy Path Tests
- `test_statedb_batch_write_commits_workflow_and_journal_atomically`
  - Given: A valid workflow state and journal entry
  - When: `execute_batch_write` is invoked
  - Then: Both entries exist in their respective Fjall keyspaces

- `test_opencode_adapter_returns_success_for_valid_prompt`
  - Given: A valid prompt and running OpenCode server
  - When: The adapter sends the request
  - Then: The response is successful and parsed correctly

- `test_frontend_defaults_use_canonical_ports`
  - Given: A default configuration instantiation in the frontend
  - When: Examining the configuration
  - Then: Ingress is 909, Admin is 9070, Service is 9180

- `test_oya_contracts_dto_serialization_round_trips`
  - Given: A `LifecycleStatusSnapshot` instance
  - When: Serializing to JSON and back
  - Then: The deserialized instance exactly matches the original

## Error Path Tests
- `test_opencode_adapter_sanitizes_raw_stack_traces_via_predicate`
  - Given: An OpenCode response containing a stack trace or provider error
  - When: Parsed by the adapter
  - Then: Returns `Error::OpenCodeAdapterError("opencode model not found or unavailable")`

- `test_statedb_batch_write_fails_gracefully_on_io_error`
  - Given: A readonly or full filesystem
  - When: A batch write is attempted
  - Then: Returns `Error::PersistenceError` and neither keyspace is updated

- `test_atomic_sequence_key_generation_maintains_order`
  - Given: High concurrency writes for a single bead
  - When: Multiple keys are generated
  - Then: Keys are strictly monotonically increasing without collisions

## Edge Case Tests
- `test_opencode_output_is_error_handles_empty_response`
  - Given: An empty string response
  - When: Evaluated by `opencode_output_is_error`
  - Then: Returns false (or handles gracefully without panicking)

- `test_opencode_output_is_error_handles_malformed_json`
  - Given: Invalid JSON from the OpenCode server
  - When: Evaluated by `opencode_output_is_error`
  - Then: Gracefully handled, does not panic

- `test_statedb_handles_large_journal_payloads_without_memory_exhaustion`
  - Given: A 10MB journal payload
  - When: Written via `execute_batch_write`
  - Then: Write succeeds without OOM, properly chunked or handled by Fjall

## Contract Verification Tests
- `test_precondition_oya_contracts_compiles_to_wasm_without_fjall`
  - Given: The `oya-contracts` crate
  - When: Compiling with target `wasm32-unknown-unknown`
  - Then: Compilation succeeds (verified via CI)

- `test_postcondition_journal_keys_are_strictly_monotonic`
  - Given: A sequence of generated keys
  - When: Iterating through them
  - Then: `key[i] < key[i+1]` is always true

- `test_invariant_no_raw_opencode_errors_in_workflow_state`
  - Given: An error response from OpenCode
  - When: The workflow state is updated
  - Then: The raw error string does not exist anywhere in the serialized workflow state

## Given-When-Then Scenarios

### Scenario 1: OpenCode Server Error Sanitization
**Given**: An OpenCode server that returns a raw stack trace `ProviderModelNotFoundError: model not found at line 42`
**When**: The OpenCode server adapter receives this response
**Then**:
- `opencode_output_is_error` evaluates to true
- The raw stack trace is discarded
- The adapter returns a sanitized `Error::OpenCodeAdapterError("opencode model not found or unavailable")`
- The frontend only sees the sanitized message via `LifecycleStatusSnapshot`

### Scenario 2: Atomic State Db Batch Write
**Given**: A `StateDb` instance initialized with `workflows` and `journal` keyspaces
**When**: A batch write is executed containing a `WorkflowState` update and an `EffectJournalEntry`
**Then**:
- Both updates are applied atomically to Fjall
- A simulated system crash during the batch write does not result in partial state (only one keyspace updated)
- The journal key used is monotonically increasing

### Scenario 3: DTO Serialization at the Boundary
**Given**: Oya Core needs to emit a memory snapshot to the frontend
**When**: Core constructs a `MemorySnapshot` (from `oya-contracts`) and serializes it over HTTP
**Then**:
- The frontend successfully deserializes the exact `MemorySnapshot` type
- No backend-specific implementation details (like Fjall guards or Restate contexts) are exposed or required to deserialize the payload