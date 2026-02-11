# Bead Operations Implementation Summary

## Completed Implementation

Three bead operation functions have been implemented in the IPC Worker Actor:

1. **`handle_start_bead`** - Transitions a bead from non-terminal state to Running
2. **`handle_cancel_bead`** - Transitions a bead from any non-terminal state to Cancelled
3. **`handle_retry_bead`** - Resets a Failed bead to Ready state for re-execution

## Architecture

The implementation follows the shell/async pattern used in the codebase:

- **Async Shell Layer**: `handle_start_bead`, `handle_cancel_bead`, `handle_retry_bead` are async methods on `IpcWorkerActorDef`
- **Actor Handle Integration**: These handlers are called from `Actor::handle` method with early returns
- **Functional Core**: The functional core `handle_guest_message` returns "handled asynchronously" error for these commands (since they're intercepted earlier)

## State Transition Validations

### execute_start_bead
- **Valid Transitions**: Pending/Ready/Dispatched/Assigned → Running
- **Idempotent**: Running → Running (no-op, returns success)
- **Invalid**: Completed/Failed/Cancelled → Running (returns InvalidStateTransition error)

### execute_cancel_bead
- **Valid Transitions**: Pending/Ready/Dispatched/Assigned/Running → Cancelled
- **Idempotent**: Cancelled → Cancelled (no-op, returns success)
- **Invalid**: Completed/Failed → Cancelled (returns InvalidStateTransition error)

### execute_retry_bead
- **Valid Transitions**: Failed → Ready (with retry_count increment)
- **Invalid**: All other states → Error (only Failed beads can be retried)

## Error Handling

All functions use functional patterns:
- Zero unwrap(), expect(), panic!
- Railway-Oriented Programming with Result<T, ActorError>
- Proper error mapping from PersistenceError to ActorError
- Idempotent operations return success without modification

## Integration Points

### Persistence Layer
- Uses `OrchestratorStore` methods: `get_bead()`, `update_bead_state()`
- These are async methods, requiring `.await` in handlers

### EventBus (Future Work)
- Event publishing is deferred due to type conversion requirements:
  - String bead_id → oya_events::BeadId (ULID)
  - persistence::BeadState → oya_events::BeadState
- This integration should be added in a future pass

## Files Modified

1. `crates/orchestrator/src/actors/ipc_worker.rs`:
   - Added async handler methods: `handle_start_bead`, `handle_cancel_bead`, `handle_retry_bead`
   - Added message interception in `Actor::handle` method
   - Updated functional core to return "handled asynchronously" for bead commands

## Contract & Test Documents

Created contract and test plan documents:
1. `.agents/contract-bead-operations.md` - Design by contract specification
2. `.agents/martin-fowler-tests-bead-operations.md` - Martin Fowler test plan with Given-When-Then scenarios

## Next Steps

To complete the implementation:
1. Add EventBus integration with proper type conversions
2. Add retry_count increment persistence (requires new store method)
3. Implement tests from the Martin Fowler test plan
4. Add worker assignment cleanup on cancel
