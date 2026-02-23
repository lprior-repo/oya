# Archived Design Contracts

These design contracts were moved from `src/lib.rs` as they describe functions that don't exist or are stubs.

---

## Design Contract: `test-trace-final`

### Purpose and goals
- Define a stable final-stage trace contract for planning, trace collection,
  report evaluation, and final gate validation.
- Guarantee reproducible outcomes for identical inputs by enforcing strict validation,
  stable stage ordering, and explicit decision derivation.
- Preserve auditability through structured diagnostics and monotonic event timestamps.

### Key functions to implement
- `build_test_trace_final_plan(input: &TestTraceFinalInput) -> Result<TestTraceFinalPlan, TestTraceFinalError>`
- `collect_test_trace_final_observation(plan: &TestTraceFinalPlan) -> Result<TestTraceFinalObservation, TestTraceFinalError>`
- `evaluate_test_trace_final_report(observation: &TestTraceFinalObservation) -> Result<TestTraceFinalReport, TestTraceFinalError>`
- `derive_test_trace_final_decision(report: &TestTraceFinalReport) -> TestTraceFinalDecision`
- `validate_test_trace_final_report(report: &TestTraceFinalReport) -> Result<(), TestTraceFinalError>`

### Acceptance criteria
- Plan creation rejects empty fields, over-limit inputs, and invalid control characters.
- Observation collection emits ordered checks with non-empty diagnostics and valid timestamps.
- Report evaluation preserves contract stage order and enforces monotonic timestamps.
- Final decision is derived only from trace/check outcomes and matches validation results.
- Re-running with equivalent inputs yields equivalent report structure and decisions.

---

## Design Contract: `src-2nw`

### Purpose and goals
- Fix critical determinism bug in Restate workflow execution by ensuring `spawn_blocking`
  operations are properly journaled and not re-executed on workflow replay.
- Maintain Restate's determinism guarantee by separating non-stable operations
  from stable journaling in the workflow execution context.
- Ensure workflow state consistency across executions and replays by following the
  correct execution pattern for blocking operations.

### Key functions to implement
- `execute_stage_real(ctx: &WorkflowContext<'_>, request: StageExecutionRequest, repo_root: PathBuf) -> Result<(StageResult, String, Vec<GateResultData>), OyaError>`
  - Fixed implementation with `spawn_blocking` outside `ctx.run()`
- `execute_stage_blocking(input: StageBlockingInput) -> Result<StageExecution, OyaError>`
  - Existing synchronous blocking execution (no changes needed)
- `test_execute_stage_real_stable_replay()`
  - New test to verify spawn_blocking is not called on replay

### Acceptance criteria
- `spawn_blocking` is called OUTSIDE of `ctx.run()` (non-stable part)
- Only result mapping is inside `ctx.run()` (stable journaling)
- Error handling properly separates OyaError (outer) from HandlerError (inner)
- Test added that verifies spawn_blocking is not called on replay
- Documentation explains the determinism pattern with proper doc comments
- `moon run :clippy` passes with no unwrap/expect/panic violations
- `moon run :test` passes with all tests green
- Code review confirms no other functions have this anti-pattern

---

## Design Contract: `src-23s`

### Purpose and goals
- Verify that zjj workspace isolation is properly disabled by default
- Ensure zjj commands execute in the current working directory without creating
  isolated workspaces when not explicitly requested
- Validate that the default behavior provides direct command execution
  for backward compatibility and simple use cases

### Key functions to implement
- `verify_zjj_default_disabled() -> Result<(), ZjjVerificationError>`
  - Checks that zjj operates in current directory by default
- `test_zjj_no_workspace_creation()`
  - Test to verify no workspace directories are created implicitly
- `test_zjj_commands_in_current_dir()`
  - Test to verify commands execute in the current working directory
- `validate_zjj_default_config() -> Result<(), ZjjVerificationError>`
  - Validates that default configuration has workspace isolation disabled

### Acceptance criteria
- zjj commands execute in current directory by default without workspace creation
- No implicit workspace directories are created when using basic zjj commands
- Default configuration explicitly disables workspace isolation
- Commands like `zjj status`, `zjj list`, `zjj help` work in current directory
- Tests verify both the absence of workspace creation and correct command behavior
- Error handling works correctly when workspace isolation is not enabled
- Documentation clearly states the default behavior and how to enable workspaces

---

## Design Contract: `src-1k3.1`

### Purpose and goals
- Remediate retry-exhausted scenarios caused by opencode plugin module resolution failures
- Ensure `Cannot find module '@opencode-ai/plugin'` errors are classified as ProviderUnavailable
- Enable proper recovery path when node_modules resolution fails in opencode cache

### Key functions to implement
- `classify_opencode_plugin_error(stderr: &str) -> Option<FailureCategory>`
  - Detects plugin resolution failures and maps to ProviderUnavailable
- `detect_opencode_module_resolution_failure(stderr: &str) -> bool`
  - Identifies ResolveMessage patterns with @opencode-ai/plugin references
- `remediate_plugin_unavailable(error: &OyaError) -> Result<RemediationAction, OyaError>`
  - Returns appropriate recovery action for provider unavailability

### Acceptance criteria
- `Cannot find module '@opencode-ai/plugin'` classified as ProviderUnavailable (not RateLimited)
- ResolveMessage patterns correctly parsed and matched
- Non-retryable failure category triggers immediate remediation path
- Retry-exhausted beads spawn remediation children with correct failure context
- Plugin resolution failures do not trigger infinite retry loops

---

## Design Contract: `src-1k3.1 src-1ml.1 src-1oy.1 src-23s.1 src-23s.2 src-23s.3`

### Purpose and goals
- Define deterministic, auditable contracts for opencode failure classification,
  polling/parse hygiene, and zjj default workspace behavior.
- Ensure provider/module-resolution failures are classified correctly and routed to
  non-retry remediation paths.
- Guarantee safe parsing and validation boundaries so malformed or oversized input
  cannot silently pass into orchestration decisions.
- Preserve default zjj execution in the current directory unless explicit workspace
  isolation is requested.

### Key functions to implement
- `classify_opencode_error(stderr: &str) -> Option<FailureCategory>`
- `parse_opencode_output(raw: &str) -> Result<OpencodeRunOutput, OpencodeParseError>`
- `parse_opencode_sse_events(raw_chunk: &str, max_events: usize) -> Result<Vec<String>, OpsMonitorError>`
- `build_opencode_poll_snapshot(session_status_json: &str, permission_json: &str, question_json: &str) -> Result<OpencodePollSnapshot, OpsMonitorError>`
- `build_zjj_workspace_name(run_id: &str, stage: &str, attempt: u32) -> Result<String, OpsMonitorError>`
- `is_retryable_failure(category: &FailureCategory) -> bool`

### Acceptance criteria
- Plugin/module-resolution errors containing `@opencode-ai/plugin` classify as
  `FailureCategory::ProviderUnavailable` and never as retryable test/lint categories.
- Poll/snapshot parsing rejects invalid JSON, invalid shapes, forbidden control characters,
  and over-limit payloads with explicit error variants.
- SSE parsing normalizes line endings, extracts only `data:` payloads, enforces payload
  size limits, and returns events in deterministic source order.
- `parse_opencode_output` supports both structured JSON and SSE/text-event extraction,
  while enforcing stdout type/length/content validation.
- Workspace naming rejects empty/invalid segments and zero attempts, producing stable,
  normalized names within configured length bounds.
- Retryability decisions remain limited to code-fixable failures; provider and rate-limit
  conditions remain non-retryable orchestration signals.

---

## Design Contract: `src-1k3.1` (contract)

### Purpose and goals
- Establish deterministic classification for opencode plugin module-resolution failures.
- Route provider-unavailable failures to remediation instead of retry loops.
- Preserve stable orchestration behavior across repeated runs.

### Key functions to implement
- `classify_opencode_error(stderr: &str) -> Option<FailureCategory>`
- `is_retryable_failure(category: &FailureCategory) -> bool`
- `remediate_retry_exhausted_failure(failure: &StageFailure) -> Result<RemediationPlan, OyaError>`

### Acceptance criteria
- Errors containing `Cannot find module '@opencode-ai/plugin'` classify as `FailureCategory::ProviderUnavailable`.
- `FailureCategory::ProviderUnavailable` is non-retryable in retryability decisions.
- Retry-exhausted handling emits remediation plans with preserved run/bead failure context.

---

## Design Contract: `src-1gw`

### Purpose and goals
- Define a deterministic contract-validation stage that turns raw contract input into a validated decision artifact.
- Ensure invalid or unsafe contract payloads are rejected at boundaries with explicit, auditable errors.
- Preserve stable outcomes so equivalent inputs always produce equivalent validation results.

### Key functions to implement
- `build_contract_validation_plan(input: &ContractValidationInput) -> Result<ContractValidationPlan, ContractValidationError>`
- `collect_contract_validation_observation(plan: &ContractValidationPlan) -> Result<ContractValidationObservation, ContractValidationError>`
- `evaluate_contract_validation_report(observation: &ContractValidationObservation) -> Result<ContractValidationReport, ContractValidationError>`
- `derive_contract_validation_decision(report: &ContractValidationReport) -> ContractValidationDecision`
- `validate_contract_validation_report(report: &ContractValidationReport) -> Result<(), ContractValidationError>`

### Acceptance criteria
- Planning rejects empty required fields, over-limit payloads, and forbidden control characters.
- Observation collection records ordered checks with non-empty diagnostics and valid timestamps.
- Report evaluation preserves canonical stage order and enforces monotonic timestamps.
- Final decision is derived exclusively from report outcomes and matches report validation.
- Re-running with equivalent inputs yields equivalent report structure and decision outputs.

---

## Design Contract: `src-2ey`

### Purpose and goals
- Pin moon CI evidence to an exact git revision at collection time to prevent stale evidence
  from bypassing land checks.
- Ensure the ship gate validates that moon evidence revision matches current HEAD before
  allowing merge operations.
- Detect and reject revision mismatches with explicit, auditable error messages.

### Key functions to implement
- `collect_moon_evidence_with_revision(repo_root: &Path) -> Result<MoonEvidence, ShipGateError>`
  - Captures moon output AND current git HEAD revision atomically
- `validate_moon_evidence_revision(evidence: &MoonEvidence, current_head: &str) -> Result<(), ShipGateError>`
  - Compares pinned revision against current HEAD, rejects on mismatch
- `pin_evidence_revision(evidence: &mut MoonEvidence, revision: &str) -> Result<(), ShipGateError>`
  - Sets the revision field on evidence with format validation

### Acceptance criteria
- Moon evidence includes mandatory `revision` field containing full 40-char git SHA
- Revision is captured atomically with moon execution (not before/after separately)
- Ship gate rejects evidence where `evidence.revision != git rev-parse HEAD`
- Revision mismatch returns `ShipGateError::StaleEvidence` with both revisions in message
- Empty or malformed revision fields are rejected at collection time
- Tests verify stale evidence rejection with mocked revision mismatch scenarios
- `moon run :ci` passes with no clippy warnings
