# ============================================================================
# BEAD: src-1hu - OyaService get_lifecycle should return terminal not-found status for unknown key
# ============================================================================

id: "src-1hu"
title: "handlers: Return terminal not-found status for unknown lifecycle keys"
type: bug
priority: 2
effort_estimate: "1hr"
labels: [handlers, OyaService, get_lifecycle, not-found, error-handling]

# ============================================================================
# SECTION 0: CLARIFICATION MARKERS
# ============================================================================

clarification_status: "RESOLVED"

resolved_clarifications:
  - question: "What does 'terminal not-found status' mean?"
    answer: "Return a terminal error (TerminalError) with a clear 'not_found' code/message when lifecycle key doesn't exist"
    decided_by: "code analysis"
    date: "2026-02-27"

  - question: "Where is the bug in current get_lifecycle implementation?"
    answer: "At line 267-285 in handlers.rs, get_lifecycle tries runtime, workflow, and raw status but doesn't explicitly return terminal error when all fail - it returns a default snapshot"
    decided_by: "code analysis"
    date: "2026-02-27"

open_clarifications: []

assumptions:
  - assumption: "The issue is that get_lifecycle returns a default/empty snapshot instead of an error for unknown keys"
    validation_method: "Verify current behavior with test"
    risk_if_wrong: "Fix may target wrong part of code"
  - assumption: "TerminalError should be used for not-found status"
    validation_method: "Check other handler error patterns in restate_oya module"
    risk_if_wrong: "May use wrong error type"

# ============================================================================
# SECTION 1: EARS REQUIREMENTS
# ============================================================================

ears_requirements:
  ubiquitous:
    - "THE SYSTEM SHALL return terminal error when lifecycle key is not found"
    - "THE SYSTEM SHALL include clear error message indicating not-found status"
    - "THE SYSTEM SHALL not return empty/default snapshot for unknown keys"

  event_driven:
    - trigger: "WHEN OyaService.get_lifecycle is called with unknown key"
      shall: "THE SYSTEM SHALL return Err(HandlerError) with TerminalError containing not_found message"
    - trigger: "WHEN runtime status is None"
      shall: "THE SYSTEM SHALL check workflow status before returning default"
    - trigger: "WHEN workflow status is None"
      shall: "THE SYSTEM SHALL check raw status before returning default"
    - trigger: "WHEN all status sources (runtime, workflow, raw) return None/empty"
      shall: "THE SYSTEM SHALL return terminal not-found error"

  unwanted:
    - condition: "IF lifecycle key doesn't exist"
      shall_not: "THE SYSTEM SHALL NOT return Ok() with empty/default snapshot"
      because: "Silent success for non-existent lifecycle masks failures and confuses clients"
    - condition: "IF all status lookups fail"
      shall_not: "THE SYSTEM SHALL NOT create synthetic snapshot with done: false"
      because: "Synthetic snapshots look like in-progress lifecycles that don't exist"

  state_driven: []

  optional: []

  complex: []

# ============================================================================
# SECTION 2: KIRK CONTRACTS
# ============================================================================

contracts:
  preconditions:
    auth_required: false
    required_inputs:
      - field: "key"
        type: "String"
        constraints: "Non-empty string representing lifecycle key"
        example_valid: "\"src-1hu\""
        example_invalid: "\"\""
    system_state:
      - "RUNTIME_LIFECYCLE_STATUS HashMap exists"
      - "OyaService and Oya clients are available"

  postconditions:
    state_changes:
      - "No state changes - get_lifecycle is a read-only operation"
    return_guarantees:
      - field: "Result::Ok"
        guarantee: "Returns Ok(LifecycleStatusSnapshot) ONLY when lifecycle exists and status is available"
      - field: "Result::Err"
        guarantee: "Returns Err(HandlerError) with TerminalError when lifecycle not found"
      - field: "error message"
        guarantee: "Includes 'not_found' and the key that wasn't found"
    side_effects: []

  invariants:
    - "get_lifecycle always returns either valid snapshot or terminal error, never empty default"
    - "All three status sources (runtime, workflow, raw) are checked before returning error"
    - "TerminalError is used for not-found status (consistent with other handler errors)"

# ============================================================================
# SECTION 2.5: RESEARCH REQUIREMENTS
# ============================================================================

research_requirements:
  files_to_read:
    - path: "src/restate_oya/handlers.rs"
      what_to_extract: "Current get_lifecycle implementation and how it handles missing status"
      document_in: "research_notes.md"
    - path: "src/restate_oya/types.rs"
      what_to_extract: "LifecycleStatusSnapshot structure and default values"
      document_in: "research_notes.md"

  patterns_to_find:
    - pattern: "TerminalError::new"
      purpose: "Find how other handlers create terminal errors"
      expected_locations: "src/restate_oya/handlers.rs"
    - pattern: "parse_lifecycle_status_snapshot"
      purpose: "Find how raw status is parsed and what it returns for empty status"
      expected_locations: "src/restate_oya/handlers.rs"

  prior_art:
    - feature: "Other handler methods (get_state, get_bead)"
      location: "src/restate_oya/handlers.rs:249-265"
      what_to_learn: "How other handlers return errors for missing keys"

  external_docs: []

  research_questions:
    - question: "What does parse_lifecycle_status_snapshot return when raw status is empty?"
      answered: false
      answer: "[To be filled after research]"
    - question: "Do other handlers (get_state, get_bead) explicitly handle not-found with TerminalError?"
      answered: false
      answer: "[To be filled after research]"
    - question: "Is RUNTIME_LIFECYCLE_STATUS HashMap check correct (it returns Option)?"
      answered: false
      answer: "[To be filled after research]"

  research_complete_when:
    - "[x] All files_to_read have been opened and key info extracted"
    - "[ ] All patterns_to_find have been searched"
    - "[ ] All prior_art has been examined"
    - "[ ] All research_questions have answers documented"

# ============================================================================
# SECTION 3: INVERSION ANALYSIS
# ============================================================================

inversions:
  security_failures: []

  usability_failures:
    - failure: "Client receives Ok() response for non-existent lifecycle and tries to poll it"
      prevention: "Always return terminal error for unknown keys"
      test_for_it: "test_unknown_key_returns_terminal_error"
    - failure: "Error message doesn't indicate which key wasn't found"
      prevention: "Include key in error message"
      test_for_it: "test_error_message_includes_key"

  data_integrity_failures:
    - failure: "Empty snapshot with done: false is returned for unknown keys"
      prevention: "Check all status sources before creating snapshot, return error if all fail"
      test_for_it: "test_unknown_key_does_not_return_empty_snapshot"

  integration_failures:
    - failure: "Fixing get_lifecycle breaks existing valid lifecycle queries"
      prevention: "Add tests for valid keys to ensure they still return Ok()"
      test_for_it: "test_existing_key_returns_valid_status"

# ============================================================================
# SECTION 4: ATDD ACCEPTANCE TESTS
# ============================================================================

acceptance_tests:
  happy_paths:
    - name: "test_existing_key_returns_valid_status"
      given: "Lifecycle with key 'src-1hu' exists and is in progress"
      when: "OyaService.get_lifecycle is called with key 'src-1hu'"
      then:
        - "Result is Ok(LifecycleStatusSnapshot)"
        - "Snapshot has valid step data"
        - "No error is returned"
      real_input: |
        let snapshot = oya_service.get_lifecycle(ctx, Json::new(KeyRequest { key: "src-1hu".to_owned() })).await.unwrap();
        assert!(snapshot.steps.len() > 0);
      expected_output: "Ok(LifecycleStatusSnapshot) with step data"

  error_paths:
    - name: "test_unknown_key_returns_terminal_error"
      given: "No lifecycle exists with key 'nonexistent-key'"
      when: "OyaService.get_lifecycle is called with key 'nonexistent-key'"
      then:
        - "Result is Err(HandlerError)"
        - "Error is TerminalError"
        - "Error message contains 'not_found'"
        - "Error message contains 'nonexistent-key'"
      real_input: |
        let result = oya_service.get_lifecycle(ctx, Json::new(KeyRequest { key: "nonexistent-key".to_owned() })).await;
        assert!(result.is_err());
        match result {
            Err(HandlerError::Terminal(e)) => {
                assert!(e.to_string().contains("not_found"));
                assert!(e.to_string().contains("nonexistent-key"));
            }
            _ => panic!("Expected TerminalError"),
        }
      expected_error: "Err(HandlerError::Terminal(Error)) with not_found message"

    - name: "test_unknown_key_does_not_return_ok"
      given: "Lifecycle key doesn't exist"
      when: "get_lifecycle is called"
      then:
        - "Result is NOT Ok()"
        - "No empty/default LifecycleStatusSnapshot is returned"
      real_input: |
        let result = get_lifecycle(&ctx, req).await;
        assert!(matches!(result, Err(_)));
      expected_error: "Err result"

  edge_cases:
    - name: "test_empty_key_string"
      scenario: "Key is empty string"
      input: "KeyRequest { key: \"\".to_owned() }"
      expected: "TerminalError with not_found message"

    - name: "test_runtime_status_none_workflow_status_none_raw_empty"
      scenario: "All three status sources return None or empty"
      input: "Key that doesn't exist in runtime, workflow, and raw status"
      expected: "TerminalError with not_found message"

  contract_tests:
    - name: "test_precondition_non_empty_key"
      verifies: "key is non-empty"
      test: "Call with empty key, verify returns TerminalError"

    - name: "test_postcondition_terminal_error_for_unknown"
      verifies: "TerminalError is returned for unknown keys"
      test: "Verify error type is TerminalError for unknown key"

    - name: "test_invariant_no_empty_snapshot_for_unknown"
      verifies: "Empty snapshot is never returned for unknown keys"
      test: "Verify Ok result is NEVER returned for unknown key"

# ============================================================================
# SECTION 5: END-TO-END TEST SPECIFICATION
# ============================================================================

e2e_tests:
  pipeline_test:
    name: "test_full_get_lifecycle_unknown_key"
    description: "Complete flow: call get_lifecycle with unknown key -> verify terminal error"

    setup:
      files_to_create: []
      environment: []
      precondition_commands:
        - "oya init"

    execute:
      command: "curl -s http://localhost:909/OyaService/get_lifecycle -X POST -H 'Content-Type: application/json' -d '{\"key\":\"nonexistent-xyz123\"}'"
      timeout_ms: 5000

    verify:
      exit_code: 0
      stdout_matches_json:
        - path: "error"
          type: "object"
      stdout_contains:
        - "not_found"
        - "nonexistent-xyz123"
      side_effects: []

    cleanup: []

  e2e_scenarios: []

# ============================================================================
# SECTION 5.5: VERIFICATION CHECKPOINTS
# ============================================================================

verification_checkpoints:
  gate_0_research:
    name: "Research Gate"
    must_pass_before: "Writing any code"
    checks:
      - "[ ] All research_requirements files have been read"
      - "[ ] All research_questions have documented answers"
      - "[ ] Current get_lifecycle implementation understood"
      - "[ ] How parse_lifecycle_status_snapshot works understood"
    evidence_required:
      - "Research notes documenting current get_lifecycle logic"
      - "Answers to research questions"
      - "Understanding of status lookup order"

  gate_1_tests:
    name: "Test Gate"
    must_pass_before: "Writing implementation code"
    checks:
      - "[ ] All acceptance tests written"
      - "[ ] All error path tests written"
      - "[ ] All edge case tests written"
      - "[ ] Tests verify TerminalError type and error messages"
    evidence_required:
      - "Tests exist in src/restate_oya/handlers_tests.rs"
      - "Tests compile (may fail due to missing implementation)"

  gate_2_implementation:
    name: "Implementation Gate"
    must_pass_before: "Declaring task complete"
    checks:
      - "[ ] All tests pass"
      - "[ ] No unwrap() or expect() calls"
      - "[ ] get_lifecycle returns TerminalError for unknown keys"
      - "[ ] Error messages include key and 'not_found'"
      - "[ ] moon run :ci passes"
    evidence_required:
      - "Test output showing all pass"
      - "CI output showing green"

  gate_3_integration:
    name: "Integration Gate"
    must_pass_before: "Closing bead"
    checks:
      - "[ ] Existing lifecycle lookups still work"
      - "[ ] No regressions in handler tests"
      - "[ ] E2E test with curl passes"
      - "[ ] Manual verification complete"
    evidence_required:
      - "Integration test output"
      - "E2E curl output showing not_found error"
      - "Manual verification notes"

  tests_json:
    format: |
      {
        "tests": [
          {"id": 1, "name": "test_existing_key_returns_valid_status", "status": "not_started|failing|passing"},
          {"id": 2, "name": "test_unknown_key_returns_terminal_error", "status": "not_started|failing|passing"},
          {"id": 3, "name": "test_unknown_key_does_not_return_ok", "status": "not_started|failing|passing"}
        ],
        "total": 3,
        "passing": 0,
        "failing": 0,
        "not_started": 3
      }
    location: ".bead-progress/src-1hu/tests.json"

# ============================================================================
# SECTION 6: IMPLEMENTATION TASK LIST
# ============================================================================

implementation_tasks:
  phase_0_research:
    parallelizable: true
    tasks:
      - task: "Read src/restate_oya/handlers.rs get_lifecycle implementation"
        parallel_group: "research"
        file: "src/restate_oya/handlers.rs"
        done_when: "Current implementation lines 267-285 understood"

      - task: "Find TerminalError patterns in other handlers"
        parallel_group: "research"
        file: "src/restate_oya/handlers.rs"
        done_when: "How to create TerminalError documented"

      - task: "Understand parse_lifecycle_status_snapshot behavior"
        parallel_group: "research"
        file: "src/restate_oya/handlers.rs"
        done_when: "What it returns for empty status documented"

  phase_1_tests_first:
    parallelizable: true
    gate_required: "gate_0_research"
    tasks:
      - task: "Write test: test_unknown_key_returns_terminal_error"
        parallel_group: "tests"
        file: "src/restate_oya/handlers_tests.rs"
        what: "Test that unknown key returns TerminalError with not_found message"
        done_when: "Test exists and FAILS (red phase)"

      - task: "Write test: test_unknown_key_does_not_return_ok"
        parallel_group: "tests"
        file: "src/restate_oya/handlers_tests.rs"
        what: "Test that unknown key does NOT return Ok result"
        done_when: "Test exists and FAILS (red phase)"

      - task: "Write test: test_existing_key_returns_valid_status"
        parallel_group: "tests"
        file: "src/restate_oya/handlers_tests.rs"
        what: "Regression test - ensure existing keys still work"
        done_when: "Test exists and PASSES (green phase - no changes yet)"

  phase_2_implementation:
    parallelizable: false
    gate_required: "gate_1_tests"
    tasks:
      - task: "Modify get_lifecycle to check all status sources and return error if all fail"
        depends_on: null
        file: "src/restate_oya/handlers.rs"
        what: "Add check: if runtime is None AND workflow is None AND raw is empty, return TerminalError"
        patterns_to_use:
          - "Result<T, HandlerError> for error propagation"
          - "TerminalError::new for terminal errors"
        done_when: "Function compiles, unknown key tests start passing"

      - task: "Ensure error message includes key and 'not_found'"
        depends_on: "Modify get_lifecycle to check all status sources"
        file: "src/restate_oya/handlers.rs"
        what: "Update TerminalError message to include key parameter"
        patterns_to_use:
          - "format! macro for message construction"
        done_when: "Error message assertions pass"

  phase_3_integration:
    parallelizable: false
    gate_required: "gate_2_implementation"
    tasks:
      - task: "Run E2E test with curl"
        commands:
          - "oya init"
          - "curl -s http://localhost:909/OyaService/get_lifecycle -X POST -H 'Content-Type: application/json' -d '{\"key\":\"nonexistent-test\"}'"
        expected: "Response with not_found error"

      - task: "Run existing handler tests"
        file: "src/restate_oya/"
        what: "Run moon run :test for restate_oya module"
        done_when: "All existing handler tests pass"

  phase_4_verification:
    parallelizable: true
    gate_required: "gate_3_integration"
    tasks:
      - task: "Run moon run :ci"
        parallel_group: "verification"
        done_when: "All tests pass, no clippy warnings"

      - task: "Manual verification of get_lifecycle behavior"
        parallel_group: "verification"
        commands:
          - "curl -s http://localhost:909/OyaService/get_lifecycle -X POST -H 'Content-Type: application/json' -d '{\"key\":\"src-1hu\"}'"
        expected: "Ok response for existing key"

  parallelization_rules:
    - "Tasks in same parallel_group CAN run simultaneously"
    - "Tasks with depends_on MUST wait for dependency"
    - "Gates MUST pass before next phase begins"
    - "When parallelizable: false, execute in listed order"

# ============================================================================
# SECTION 7: FAILURE MODES
# ============================================================================

failure_modes:
  - symptom: "Test fails with 'unexpected Ok result' for unknown key"
    likely_cause: "get_lifecycle still returns Ok() for unknown keys after fix"
    where_to_look:
      - file: "src/restate_oya/handlers.rs"
        function: "get_lifecycle"
        what_to_check: "Is there an early return that bypasses the error check?"
    fix_pattern: "Ensure all code paths check status sources and return error if all fail"

  - symptom: "Error message doesn't include key"
    likely_cause: "TerminalError::new call doesn't include key parameter in message"
    where_to_look:
      - file: "src/restate_oya/handlers.rs"
        function: "get_lifecycle"
        what_to_check: "Does TerminalError::new message include format! with key variable?"
    fix_pattern: "Update TerminalError::new to use format!(\"... {} ...\", key)"

  - symptom: "Existing lifecycle lookups now fail"
    likely_cause: "Fix broke the logic for valid keys"
    where_to_look:
      - file: "src/restate_oya/handlers.rs"
        function: "get_lifecycle"
        what_to_check: "Does error check only trigger when ALL three status sources fail?"
    fix_pattern: "Ensure error is returned only if runtime.is_none() AND workflow.is_none() AND raw is empty"

debugging_commands:
  - scenario: "When unknown key still returns Ok"
    run: "RUST_LOG=debug oya init && curl -v http://localhost:909/OyaService/get_lifecycle -X POST -H 'Content-Type: application/json' -d '{\"key\":\"test-unknown\"}'"
    look_for: "Check response body and HTTP status"

  - scenario: "When tests fail"
    run: "RUST_BACKTRACE=1 moon run :test restate_oya -- --nocapture"
    look_for: "Full stack trace of test failure"

# ============================================================================
# SECTION 7.5: ANTI-HALLUCINATION RULES
# ============================================================================

anti_hallucination:
  read_before_write:
    - file: "src/restate_oya/handlers.rs"
      must_read_first: true
      key_sections_to_understand:
        - "get_lifecycle function (lines 267-285)"
        - "parse_lifecycle_status_snapshot function"
        - "TerminalError usage patterns"

  verify_before_reference:
    - type: "TerminalError::new"
      expected_location: "src/restate_oya module"
      verify_command: "rg -n 'TerminalError::new' src/restate_oya/"

  apis_that_exist:
    - api: "TerminalError::new"
      signature: "fn new(message: String) -> TerminalError"
      import_from: "restate_sdk::prelude"

  apis_that_do_not_exist:
    - "NotFoundError - use TerminalError with appropriate message instead"

  no_placeholder_values:
    - "Do NOT use placeholder keys like 'test-key' - use realistic key formats"
    - "Do NOT use generic error messages - include actual key in error"

  git_verification:
    before_claiming_done: |
      git status  # Verify changes are staged
      git diff src/restate_oya/handlers.rs    # Verify changes match specification
      moon run :test  # Verify all tests pass

# ============================================================================
# SECTION 7.6: CONTEXT WINDOW SURVIVAL
# ============================================================================

context_survival:
  progress_file:
    path: ".bead-progress/src-1hu/progress.txt"
    format: |
      # Bead: src-1hu - OyaService get_lifecycle should return terminal not-found status for unknown key
      # Started: [timestamp]
      # Last updated: [timestamp]

      ## Current Phase
      [phase_name]

      ## Completed Tasks
      - [x] [task 1]
      - [x] [task 2]

      ## Current Task
      - [ ] [current task] (IN PROGRESS)
          - [sub-step completed]
          - [sub-step in progress]

      ## Remaining Tasks
      - [ ] [task 3]
      - [ ] [task 4]

      ## Key Decisions Made
      - [Decision 1]: [Rationale]
      - [Decision 2]: [Rationale]

      ## Blockers/Issues
      - [None | List of blockers]

      ## Next Steps (if context clears)
      1. Read this file
      2. Review git log for recent commits
      3. Continue from "Current Task"

  tests_status_file:
    path: ".bead-progress/src-1hu/tests.json"
    update_frequency: "After each test run"

  research_notes_file:
    path: ".bead-progress/src-1hu/research.md"
    contains:
      - "Current get_lifecycle implementation logic"
      - "How status sources are checked (runtime, workflow, raw)"
      - "TerminalError patterns in other handlers"
      - "parse_lifecycle_status_snapshot behavior"

  git_checkpoints:
    frequency: "After each completed task"
    message_format: "[src-1hu] checkpoint: [task completed]"
    purpose: "Allow rollback if next step fails"

  recovery_instructions: |
    If context window is cleared, start new session with:
    1. cat .bead-progress/src-1hu/progress.txt
    2. cat .bead-progress/src-1hu/tests.json
    3. cat .bead-progress/src-1hu/research.md
    4. git log --oneline -10
    5. Continue from where progress.txt indicates

# ============================================================================
# SECTION 8: COMPLETION CRITERIA
# ============================================================================

completion_checklist:
  tests:
    - "[ ] test_unknown_key_returns_terminal_error passes"
    - "[ ] test_unknown_key_does_not_return_ok passes"
    - "[ ] test_existing_key_returns_valid_status passes"
    - "[ ] E2E test with curl passes"

  code:
    - "[ ] No unwrap() or expect() in new code"
    - "[ ] get_lifecycle returns TerminalError for unknown keys"
    - "[ ] Error messages include key and 'not_found'"
    - "[ ] All three status sources are checked before error"

  ci:
    - "[ ] moon run :ci passes"
    - "[ ] No clippy warnings"
    - "[ ] No compiler warnings"

# ============================================================================
# SECTION 9: CONTEXT
# ============================================================================

context:
  related_files:
    - path: "src/restate_oya/handlers.rs"
      relevance: "Contains get_lifecycle implementation to fix"
    - path: "src/restate_oya/types.rs"
      relevance: "Contains LifecycleStatusSnapshot and KeyRequest types"

  similar_implementations:
    - "See get_state and get_bead handlers for error patterns"

  codebase_patterns:
    - pattern: "TerminalError::new usage"
      example_location: "src/restate_oya/handlers.rs:96"
      how_to_apply: "Use TerminalError::new with descriptive message for not-found"

# ============================================================================
# SECTION 10: AI HINTS
# ============================================================================

ai_hints:
  do:
    - "Read current get_lifecycle implementation carefully"
    - "Understand the order of status checks (runtime -> workflow -> raw)"
    - "Add explicit check: if all three fail, return TerminalError"
    - "Use format! to include key in error message"
    - "Update progress.txt after each completed task"
    - "Commit to git after each completed task"

  do_not:
    - "Do NOT use unwrap() or expect() in get_lifecycle"
    - "Do NOT return Ok() with empty snapshot for unknown keys"
    - "Do NOT modify the status lookup logic for valid keys"
    - "Do NOT change the order of status checks"

  language_guidance:
    avoid: ["think", "thinking"]
    use_instead: ["consider", "evaluate", "analyze", "determine"]

  action_guidance: |
    When implementing, TAKE ACTION rather than suggesting changes.
    Don't say "you could add X" - just ADD X.
    Be direct: "I will now modify..." not "I could modify..."

  parallel_execution: |
    When reading multiple files for research, read them ALL in parallel.
    When writing multiple independent tests, write them ALL in parallel.
    Only serialize when there are true dependencies.

  incremental_progress: |
    Focus on completing ONE task fully before moving to the next.
    After each task: update progress.txt, run tests, commit if passing.
    Don't try to implement everything at once.

  code_patterns:
    - name: "Terminal error with message"
      use_when: "Returning terminal error for not-found"
      example: |
        Err(TerminalError::new(format!("lifecycle not found: {}", key)).into())

  constitution:
    - "Zero unwrap law: NEVER use .unwrap() or .expect()"
    - "Functional first: Prefer map/and_then over if-else chains"
    - "Moon only: NEVER use raw cargo commands"
    - "Test first: Tests MUST exist before implementation"
    - "No silent failures: Unknown keys MUST return errors"
