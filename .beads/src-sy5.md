# ============================================================================
# BEAD: src-sy5 - observability: add durable step-level telemetry and unwind signals
# ============================================================================

id: "src-sy5"
title: "observability: add durable step-level telemetry and unwind signals"
type: feature
priority: 1
effort_estimate: "4hr"
labels: [observability, telemetry, opentelemetry, lifecycle, compensation]

# ============================================================================
# SECTION 0: CLARIFICATION MARKERS
# ============================================================================

clarification_status: "HAS_OPEN_QUESTIONS"

resolved_clarifications:
  - question: "What telemetry should be captured for each lifecycle step?"
    answer: "Step name, status, duration, started_at, finished_at, error details, and compensation signals"
    decided_by: "prior research"
    date: "2026-02-27"

open_clarifications:
  - question: "Should telemetry be emitted via OpenTelemetry directly or logged and collected by OpenObserve?"
    context: "AGENTS.md specifies OTEL_EXPORTER_OTLP_ENDPOINT for OpenObserve integration"
    options:
      - "Direct OTEL API calls (more control, more complex)"
      - "Structured logging collected by OpenObserve (simpler, existing integration)"
    default_if_unresolved: "Use structured logging with OpenObserve collection (existing pattern)"

  - question: "What format should unwind/compensation signals use?"
    context: "Need to define a clear signal format for step failures that trigger compensation"
    options:
      - "Use existing CompensationDiagnostic type extended with telemetry fields"
      - "Create new UnwindSignal type separate from diagnostics"
    default_if_unresolved: "Extend existing CompensationDiagnostic type"

assumptions:
  - assumption: "OTEL service name 'oya-orchestrator' is configured per AGENTS.md"
    validation_method: "Check OTEL_SERVICE_NAME environment variable usage in codebase"
    risk_if_wrong: "Telemetry won't be associated with correct service in OpenObserve"
  - assumption: "OpenTelemetry endpoint is at http://localhost:4318 per AGENTS.md"
    validation_method: "Verify OTEL_EXPORTER_OTLP_ENDPOINT usage"
    risk_if_wrong: "Telemetry won't be sent to OpenObserve"

# ============================================================================
# SECTION 1: EARS REQUIREMENTS
# ============================================================================

ears_requirements:
  ubiquitous:
    - "THE SYSTEM SHALL emit telemetry for every lifecycle step execution"
    - "THE SYSTEM SHALL capture step name, status, and duration for each step"
    - "THE SYSTEM SHALL send telemetry to OpenObserve via OTEL endpoint"

  event_driven:
    - trigger: "WHEN a lifecycle step starts"
      shall: "THE SYSTEM SHALL emit a 'step_started' telemetry event with step name and timestamp"
    - trigger: "WHEN a lifecycle step completes successfully"
      shall: "THE SYSTEM SHALL emit a 'step_completed' telemetry event with duration"
    - trigger: "WHEN a lifecycle step fails"
      shall: "THE SYSTEM SHALL emit a 'step_failed' telemetry event with error details"
    - trigger: "WHEN compensation for a step begins"
      shall: "THE SYSTEM SHALL emit an 'unwind_started' telemetry signal"
    - trigger: "WHEN compensation for a step completes"
      shall: "THE SYSTEM SHALL emit an 'unwind_completed' telemetry signal"

  state_driven:
    - state: "WHILE lifecycle execution is in progress"
      shall: "THE SYSTEM SHALL aggregate step-level telemetry into lifecycle-span context"

  optional:
    - condition: "WHERE RUST_LOG environment variable is set to debug"
      shall: "THE SYSTEM SHALL log telemetry events to stdout for debugging"

  unwanted:
    - condition: "IF a telemetry emission fails"
      shall_not: "THE SYSTEM SHALL NOT fail the entire lifecycle execution"
      because: "Telemetry failures should not break the actual work"
    - condition: "IF step execution is very fast (< 1ms)"
      shall_not: "THE SYSTEM SHALL NOT lose precision in duration measurements"
      because: "Fast steps are still important to measure"

  complex: []

# ============================================================================
# SECTION 2: KIRK CONTRACTS
# ============================================================================

contracts:
  preconditions:
    auth_required: false
    required_inputs:
      - field: "step_name"
        type: "String"
        constraints: "Non-empty string identifying the lifecycle step"
        example_valid: "\"moon_ci\""
        example_invalid: "\"\""
      - field: "step_status"
        type: "LifecycleStepStatus"
        constraints: "One of Pending, Running, Succeeded, Failed"
        example_valid: "LifecycleStepStatus::Succeeded"
        example_invalid: "invalid status"
    system_state:
      - "OTEL_EXPORTER_OTLP_ENDPOINT environment variable may be configured"
      - "OpenObserve service may be running (optional - telemetry degrades gracefully if unavailable)"

  postconditions:
    state_changes:
      - "Telemetry data sent to OpenObserve (if endpoint configured and available)"
      - "Step execution duration recorded"
    return_guarantees:
      - field: "telemetry emission"
        guarantee: "Always returns Ok(()) even if OpenObserve is unavailable"
      - field: "duration measurement"
        guarantee: "Duration in milliseconds, precision to at least 1ms"
    side_effects:
      - "Network call to OTEL endpoint (if configured)"
      - "Optional log output to stdout (if RUST_LOG=debug)"

  invariants:
    - "Telemetry emission never panics - always degrades gracefully"
    - "Duration is measured as finished_at - started_at in milliseconds"
    - "Step names in telemetry match step names in lifecycle definitions"
    - "Unwind/compensation signals are emitted in reverse order of step execution"

# ============================================================================
# SECTION 2.5: RESEARCH REQUIREMENTS
# ============================================================================

research_requirements:
  files_to_read:
    - path: "src/lifecycle/workflow/mod.rs"
      what_to_extract: "How run_lifecycle executes steps and tracks progress"
      document_in: "research_notes.md"
    - path: "src/lifecycle/types/mod.rs"
      what_to_extract: "LifecycleStep, LifecycleStepStatus, and CompensationDiagnostic types"
      document_in: "research_notes.md"
    - path: "AGENTS.md"
      what_to_extract: "OTEL configuration and OpenObserve setup instructions"
      document_in: "research_notes.md"
    - path: "src/restate_oya/handlers.rs"
      what_to_extract: "How lifecycle progress updates are currently handled"
      document_in: "research_notes.md"

  patterns_to_find:
    - pattern: "LifecycleProgressUpdate"
      purpose: "Find how progress updates flow through the system"
      expected_locations: "src/lifecycle/workflow/mod.rs, src/restate_oya/handlers.rs"
    - pattern: "OTEL.*telemetry|tracing.*instrument"
      purpose: "Find existing telemetry/tracing patterns in the codebase"
      expected_locations: "src/main.rs, src/lib.rs"

  prior_art:
    - feature: "Existing CompensationDiagnostic type"
      location: "src/lifecycle/types/mod.rs"
      what_to_learn: "Current structure for compensation diagnostics"

  external_docs:
    - url: "https://opentelemetry.io/docs/reference/specification/trace/semantic_conventions/"
      section: "General Semantic Attributes"
      extract: "Standard attributes for telemetry events"
    - url: "https://github.com/openobserve/openobserve"
      section: "Getting Started"
      extract: "How to send data to OpenObserve via OTEL"

  research_questions:
    - question: "Does the codebase already use any telemetry or tracing libraries?"
      answered: false
      answer: "[To be filled after research]"
    - question: "How is LifecycleProgressUpdate currently structured - can we add telemetry there?"
      answered: false
      answer: "[To be filled after research]"
    - question: "What is the CompensationDiagnostic type structure - can it be extended?"
      answered: false
      answer: "[To be filled after research]"
    - question: "Are there existing OTEL environment variables being read anywhere?"
      answered: false
      answer: "[To be filled after research]"

  research_complete_when:
    - "[ ] All files_to_read have been opened and key info extracted"
    - "[ ] All patterns_to_find have been searched"
    - "[ ] All prior_art has been examined"
    - "[ ] All research_questions have answers documented"

# ============================================================================
# SECTION 3: INVERSION ANALYSIS
# ============================================================================

inversions:
  security_failures:
    - failure: "Telemetry data includes sensitive information (API keys, passwords)"
      prevention: "Redact or exclude sensitive fields from telemetry payload"
      test_for_it: "test_telemetry_excludes_sensitive_data"

  usability_failures:
    - failure: "Telemetry is sent but not visible in OpenObserve UI"
      prevention: "Verify OTEL configuration and service name match OpenObserve expectations"
      test_for_it: "test_telemetry_visible_in_openobserve"
    - failure: "Duration measurements overflow for very long-running steps"
      prevention: "Use u64 for milliseconds, document that very long steps may wrap"
      test_for_it: "test_duration_handles_long_running_steps"

  data_integrity_failures:
    - failure: "Telemetry events are sent out of order or lost"
      prevention: "Use synchronous emission or buffer with flush on lifecycle completion"
      test_for_it: "test_telemetry_events_in_correct_order"
    - failure: "Unwind signals not emitted for failed steps"
      prevention: "Ensure all failure paths trigger unwind signal emission"
      test_for_it: "test_unwind_signals_emitted_on_step_failure"

  integration_failures:
    - failure: "OpenObserve unavailable causes lifecycle to fail"
      prevention: "Wrap all telemetry emission in try-catch, never fail on telemetry errors"
      test_for_it: "test_lifecycle_succeeds_when_openobserve_unavailable"

# ============================================================================
# SECTION 4: ATDD ACCEPTANCE TESTS
# ============================================================================

acceptance_tests:
  happy_paths:
    - name: "test_step_started_emits_telemetry"
      given: "Lifecycle step 'moon_ci' starts execution"
      when: "Step progress update is applied"
      then:
        - "Telemetry event 'step_started' is emitted"
        - "Event contains step name 'moon_ci'"
        - "Event contains timestamp"
      real_input: |
        let update = LifecycleProgressUpdate::Step {
            step: "moon_ci".to_owned(),
            status: LifecycleStepStatus::Running,
            message: Some("running moon ci".to_owned()),
            started_at: Some("2026-02-27T10:00:00Z".to_owned()),
            finished_at: None,
            duration_ms: None,
        };
        emit_step_telemetry(&update).await.unwrap();
      expected_output: "Telemetry event sent to OpenObserve (or logged)"

    - name: "test_step_completed_emits_duration"
      given: "Lifecycle step completes successfully after 5 seconds"
      when: "Step progress update with finished_at is applied"
      then:
        - "Telemetry event 'step_completed' is emitted"
        - "Event contains duration_ms = 5000"
        - "Event contains status 'succeeded'"
      real_input: |
        let update = LifecycleProgressUpdate::Step {
            step: "moon_ci".to_owned(),
            status: LifecycleStepStatus::Succeeded,
            started_at: Some("2026-02-27T10:00:00Z".to_owned()),
            finished_at: Some("2026-02-27T10:00:05Z".to_owned()),
            duration_ms: Some(5000),
            ..Default::default()
        };
      expected_output: "Telemetry event with duration"

    - name: "test_unwind_signal_emitted_on_step_failure"
      given: "Lifecycle step fails and triggers compensation"
      when: "Compensation for step begins"
      then:
        - "Telemetry event 'unwind_started' is emitted"
        - "Event contains step name being compensated"
        - "Event contains original failure reason"
      real_input: |
        emit_unwind_signal("moon_ci", &compensation_diagnostic).await.unwrap();
      expected_output: "Unwind telemetry event"

  error_paths:
    - name: "test_telemetry_failure_does_not_break_lifecycle"
      given: "OpenObserve endpoint is unavailable or network fails"
      when: "Step progress update tries to emit telemetry"
      then:
        - "emit_step_telemetry returns Ok(())"
        - "No panic or error propagates"
        - "Lifecycle execution continues normally"
      real_input: |
        // Simulate network failure by using invalid endpoint
        std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://invalid:9999");
        emit_step_telemetry(&update).await.unwrap();
      expected_error: "None - telemetry failures are swallowed gracefully"

    - name: "test_invalid_step_name_handled_gracefully"
      given: "Step progress update with empty or invalid step name"
      when: "Telemetry emission is attempted"
      then:
        - "Returns Ok(())"
        - "Logs warning but doesn't panic"
      real_input: |
        let update = LifecycleProgressUpdate::Step {
            step: "".to_owned(),
            status: LifecycleStepStatus::Running,
            ..Default::default()
        };
        emit_step_telemetry(&update).await.unwrap();
      expected_error: "None"

  edge_cases:
    - name: "test_zero_duration_step"
      scenario: "Step completes in < 1ms"
      input: "started_at and finished_at are identical or within 1ms"
      expected: "duration_ms is 0 or 1, no precision loss"

    - name: "test_multiple_steps_telemetry_aggregation"
      scenario: "Multiple steps execute in sequence"
      input: "Series of step updates for different step names"
      expected: "Each step emits its own telemetry events, events are in order"

  contract_tests:
    - name: "test_precondition_non_empty_step_name"
      verifies: "step_name is non-empty"
      test: "Call with empty step name, verify returns Ok(()) but logs warning"

    - name: "test_postcondition_duration_calculated_correctly"
      verifies: "duration_ms equals finished_at - started_at in milliseconds"
      test: "Set known timestamps, verify duration calculation"

    - name: "test_invariant_telemetry_never_panics"
      verifies: "Telemetry emission never causes panic"
      test: "Try various invalid inputs, verify no panics occur"

# ============================================================================
# SECTION 5: END-TO-END TEST SPECIFICATION
# ============================================================================

e2e_tests:
  pipeline_test:
    name: "test_full_lifecycle_with_telemetry"
    description: "Complete flow: run lifecycle -> verify telemetry emitted to OpenObserve"

    setup:
      files_to_create: []
      environment:
        - "OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318"
        - "RUST_LOG=debug"
      precondition_commands:
        - "systemctl --user start observability.service"

    execute:
      command: "oya lifecycle --bead src-sy5 --repo lprior-repo/oya"
      timeout_ms: 60000

    verify:
      exit_code: 0
      stdout_contains:
        - "step_started: mark_in_progress"
        - "step_started: moon_ci"
        - "step_completed: moon_ci"
        - "duration_ms"
      side_effects:
        - "Telemetry visible in OpenObserve UI at http://localhost:5080"

    cleanup:
      commands:
        - "systemctl --user stop observability.service"
      files_to_delete: []

  e2e_scenarios:
    - name: "e2e_telemetry_survives_lifecycle_failure"
      description: "Prove telemetry is emitted even when lifecycle fails midway"
      steps:
        - action: "Run lifecycle with a bead that fails during moon_ci step"
          verify: "Telemetry shows step_started for moon_ci and step_failed"
        - action: "Check for unwind signals"
          verify: "Unwind telemetry events are emitted in reverse order"

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
      - "[ ] Existing telemetry patterns understood"
      - "[ ] Clarification on OTEL vs logging approach resolved"
    evidence_required:
      - "Research notes documenting existing telemetry usage (if any)"
      - "Answers to how to emit telemetry (OTEL API vs logging)"
      - "Understanding of LifecycleProgressUpdate flow"

  gate_1_tests:
    name: "Test Gate"
    must_pass_before: "Writing implementation code"
    checks:
      - "[ ] All acceptance tests written"
      - "[ ] All error path tests written"
      - "[ ] All edge case tests written"
      - "[ ] Tests use real telemetry emission (no mocks)"
    evidence_required:
      - "Tests exist in src/lifecycle/telemetry_tests.rs"
      - "Tests compile (may fail due to missing implementation)"

  gate_2_implementation:
    name: "Implementation Gate"
    must_pass_before: "Declaring task complete"
    checks:
      - "[ ] All tests pass"
      - "[ ] No unwrap() or expect() calls"
      - "[ ] Telemetry emission integrated into LifecycleProgressUpdate handling"
      - "[ ] Unwind signals emitted during compensation"
      - "[ ] moon run :ci passes"
    evidence_required:
      - "Test output showing all pass"
      - "CI output showing green"
      - "Telemetry visible in OpenObserve (optional but preferred)"

  gate_3_integration:
    name: "Integration Gate"
    must_pass_before: "Closing bead"
    checks:
      - "[ ] E2E test with real lifecycle run passes"
      - "[ ] Telemetry visible in OpenObserve UI"
      - "[ ] No regressions in existing lifecycle tests"
      - "[ ] Manual verification complete"
    evidence_required:
      - "E2E test output"
      - "Screenshot or evidence of OpenObserve showing telemetry"
      - "Manual verification notes"

  tests_json:
    format: |
      {
        "tests": [
          {"id": 1, "name": "test_step_started_emits_telemetry", "status": "not_started|failing|passing"},
          {"id": 2, "name": "test_step_completed_emits_duration", "status": "not_started|failing|passing"},
          {"id": 3, "name": "test_unwind_signal_emitted_on_step_failure", "status": "not_started|failing|passing"}
        ],
        "total": N,
        "passing": 0,
        "failing": 0,
        "not_started": N
      }
    location: ".bead-progress/src-sy5/tests.json"

# ============================================================================
# SECTION 6: IMPLEMENTATION TASK LIST
# ============================================================================

implementation_tasks:
  phase_0_research:
    parallelizable: true
    tasks:
      - task: "Read src/lifecycle/workflow/mod.rs and find LifecycleProgressUpdate usage"
        parallel_group: "research"
        file: "src/lifecycle/workflow/mod.rs"
        done_when: "Understand how progress updates flow"

      - task: "Read AGENTS.md for OTEL configuration details"
        parallel_group: "research"
        file: "AGENTS.md"
        done_when: "OTEL endpoint and service name documented"

      - task: "Search for existing telemetry/tracing in codebase"
        parallel_group: "research"
        command: "rg -i 'otel|telemetry|tracing::instrument' src/"
        done_when: "Existing patterns identified"

  phase_1_tests_first:
    parallelizable: true
    gate_required: "gate_0_research"
    tasks:
      - task: "Write test: test_step_started_emits_telemetry"
        parallel_group: "tests"
        file: "src/lifecycle/telemetry_tests.rs"
        what: "Test that step_started event is emitted"
        done_when: "Test exists and FAILS (red phase)"

      - task: "Write test: test_step_completed_emits_duration"
        parallel_group: "tests"
        file: "src/lifecycle/telemetry_tests.rs"
        what: "Test that step_completed event includes duration"
        done_when: "Test exists and FAILS (red phase)"

      - task: "Write test: test_telemetry_failure_does_not_break_lifecycle"
        parallel_group: "tests"
        file: "src/lifecycle/telemetry_tests.rs"
        what: "Test that telemetry failures are handled gracefully"
        done_when: "Test exists and FAILS (red phase)"

  phase_2_implementation:
    parallelizable: false
    gate_required: "gate_1_tests"
    tasks:
      - task: "Create telemetry module with emission functions"
        depends_on: null
        file: "src/lifecycle/telemetry.rs"
        what: "Create module with emit_step_telemetry and emit_unwind_signal functions"
        patterns_to_use:
          - "Result<T, TelemetryError> for errors"
          - "? operator for error propagation"
          - "Graceful degradation on network failures"
        done_when: "Module compiles, tests start passing"

      - task: "Integrate telemetry emission into progress update handler"
        depends_on: "Create telemetry module"
        file: "src/lifecycle/workflow/mod.rs"
        what: "Call emit_step_telemetry in the progress callback"
        patterns_to_use:
          - "Async emission with .await"
        done_when: "Telemetry emitted for all step updates"

      - task: "Add unwind signal emission during compensation"
        depends_on: "Integrate telemetry emission"
        file: "src/lifecycle/workflow/mod.rs"
        what: "Call emit_unwind_signal for each compensated step in reverse order"
        patterns_to_use:
          - "Reverse iteration over completed steps"
        done_when: "Unwind signals emitted on failures"

      - task: "Configure OTEL service name from AGENTS.md"
        depends_on: "Add unwind signal emission"
        file: "src/lifecycle/telemetry.rs"
        what: "Read OTEL_SERVICE_NAME env var, default to 'oya-orchestrator'"
        patterns_to_use:
          - "std::env::var with fallback"
        done_when: "Service name correctly set"

  phase_3_integration:
    parallelizable: false
    gate_required: "gate_2_implementation"
    tasks:
      - task: "Run existing lifecycle tests to ensure no regressions"
        file: "src/lifecycle/"
        what: "Run moon run :test for lifecycle module"
        done_when: "All existing lifecycle tests pass"

      - task: "Run E2E test with OpenObserve"
        commands:
          - "systemctl --user start observability.service"
          - "oya lifecycle --bead src-sy5 --repo lprior-repo/oya"
          - "Open http://localhost:5080 and verify telemetry visible"
        expected: "Telemetry events visible in OpenObserve UI"

  phase_4_verification:
    parallelizable: true
    gate_required: "gate_3_integration"
    tasks:
      - task: "Run moon run :ci"
        parallel_group: "verification"
        done_when: "All tests pass, no clippy warnings"

      - task: "Manual verification of telemetry in OpenObserve"
        parallel_group: "verification"
        commands:
          - "curl http://localhost:5080"
        expected: "OpenObserve UI accessible and shows oya-orchestrator telemetry"

  parallelization_rules:
    - "Tasks in same parallel_group CAN run simultaneously"
    - "Tasks with depends_on MUST wait for dependency"
    - "Gates MUST pass before next phase begins"
    - "When parallelizable: false, execute in listed order"

# ============================================================================
# SECTION 7: FAILURE MODES
# ============================================================================

failure_modes:
  - symptom: "Tests fail with 'telemetry emission failed' error"
    likely_cause: "Telemetry functions are returning errors instead of Ok(())"
    where_to_look:
      - file: "src/lifecycle/telemetry.rs"
        function: "emit_step_telemetry"
        what_to_check: "Are all error cases caught and converted to Ok(())?"
    fix_pattern: "Wrap all emission logic in try-catch, always return Ok(())"

  - symptom: "Telemetry not visible in OpenObserve"
    likely_cause: "OTEL endpoint not configured or wrong service name"
    where_to_look:
      - file: "src/lifecycle/telemetry.rs"
        what_to_check: "Is OTEL_EXPORTER_OTLP_ENDPOINT being read? Is service name 'oya-orchestrator'?"
    fix_pattern: "Verify env var reading and service name configuration"

  - symptom: "Unwind signals not emitted for failed steps"
    likely_cause: "Compensation logic doesn't call emit_unwind_signal"
    where_to_look:
      - file: "src/lifecycle/workflow/mod.rs"
        function: "run_lifecycle_with_progress"
        what_to_check: "Does compensation path call emit_unwind_signal for each step?"
    fix_pattern: "Add emit_unwind_signal calls in compensation logic"

debugging_commands:
  - scenario: "When telemetry not appearing in OpenObserve"
    run: "RUST_LOG=debug oya lifecycle --bead <id> --repo <repo> 2>&1 | rg -i telemetry"
    look_for: "Telemetry emission logs and any errors"

  - scenario: "When tests fail unexpectedly"
    run: "RUST_BACKTRACE=1 moon run :test lifecycle::telemetry -- --nocapture"
    look_for: "Full stack trace of test failure"

# ============================================================================
# SECTION 7.5: ANTI-HALLUCINATION RULES
# ============================================================================

anti_hallucination:
  read_before_write:
    - file: "src/lifecycle/workflow/mod.rs"
      must_read_first: true
      key_sections_to_understand:
        - "LifecycleProgressUpdate enum definition"
        - "run_lifecycle_with_progress function"
        - "Progress callback handling"

  verify_before_reference:
    - type: "LifecycleProgressUpdate"
      expected_location: "src/lifecycle/workflow/mod.rs"
      verify_command: "rg -n 'enum LifecycleProgressUpdate' src/lifecycle/"

  apis_that_exist:
    - api: "std::env::var"
      signature: "fn var(key: &str) -> Result<String, VarError>"
      import_from: "std::env"

  apis_that_do_not_exist:
    - "otel::emit_span - check what actual OTEL crate API is available"
    - "tracing::info_span - verify if tracing crate is in dependencies"

  no_placeholder_values:
    - "Do NOT use placeholder step names like 'test_step' - use real lifecycle step names"
    - "Do NOT use fake OTEL endpoints - use real localhost:4318 from AGENTS.md"

  git_verification:
    before_claiming_done: |
      git status  # Verify changes are staged
      git diff    # Verify changes match specification
      moon run :test  # Verify all tests pass
      systemctl --user start observability.service && oya lifecycle --bead <id> && systemctl --user stop observability.service  # Verify E2E

# ============================================================================
# SECTION 7.6: CONTEXT WINDOW SURVIVAL
# ============================================================================

context_survival:
  progress_file:
    path: ".bead-progress/src-sy5/progress.txt"
    format: |
      # Bead: src-sy5 - observability: add durable step-level telemetry and unwind signals
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
    path: ".bead-progress/src-sy5/tests.json"
    update_frequency: "After each test run"

  research_notes_file:
    path: ".bead-progress/src-sy5/research.md"
    contains:
      - "Existing telemetry patterns in codebase"
      - "OTEL configuration approach decision"
      - "LifecycleProgressUpdate flow understanding"
      - "Compensation flow understanding"

  git_checkpoints:
    frequency: "After each completed task"
    message_format: "[src-sy5] checkpoint: [task completed]"
    purpose: "Allow rollback if next step fails"

  recovery_instructions: |
    If context window is cleared, start new session with:
    1. cat .bead-progress/src-sy5/progress.txt
    2. cat .bead-progress/src-sy5/tests.json
    3. cat .bead-progress/src-sy5/research.md
    4. git log --oneline -10
    5. Continue from where progress.txt indicates

# ============================================================================
# SECTION 8: COMPLETION CRITERIA
# ============================================================================

completion_checklist:
  tests:
    - "[ ] test_step_started_emits_telemetry passes"
    - "[ ] test_step_completed_emits_duration passes"
    - "[ ] test_unwind_signal_emitted_on_step_failure passes"
    - "[ ] test_telemetry_failure_does_not_break_lifecycle passes"
    - "[ ] E2E test with OpenObserve passes"
    - "[ ] All existing lifecycle tests still pass"

  code:
    - "[ ] No unwrap() or expect() in new code"
    - "[ ] Telemetry module created and integrated"
    - "[ ] Unwind signals emitted during compensation"
    - "[ ] OTEL service name configured correctly"

  ci:
    - "[ ] moon run :ci passes"
    - "[ ] No clippy warnings"
    - "[ ] No compiler warnings"

  verification:
    - "[ ] Telemetry visible in OpenObserve UI"
    - "[ ] Manual verification of step-level telemetry"
    - "[ ] Manual verification of unwind signals"

# ============================================================================
# SECTION 9: CONTEXT
# ============================================================================

context:
  related_files:
    - path: "src/lifecycle/workflow/mod.rs"
      relevance: "Contains lifecycle execution logic and progress handling"
    - path: "AGENTS.md"
      relevance: "Contains OTEL configuration and OpenObserve setup"
    - path: "src/lifecycle/types/mod.rs"
      relevance: "Contains CompensationDiagnostic type for unwind signals"

  similar_implementations:
    - "Search for any existing logging or tracing patterns in the codebase"

  codebase_patterns:
    - pattern: "Progress callback in run_lifecycle_with_progress"
      example_location: "src/lifecycle/workflow/mod.rs"
      how_to_apply: "Call telemetry emission inside the progress callback"

# ============================================================================
# SECTION 10: AI HINTS
# ============================================================================

ai_hints:
  do:
    - "Decide on OTEL vs logging approach during research phase"
    - "Read AGENTS.md carefully for correct OTEL endpoint configuration"
    - "Make telemetry emission never fail the lifecycle - always return Ok(())"
    - "Use functional patterns: map, and_then, ?"
    - "Update progress.txt after each completed task"
    - "Commit to git after each completed task"

  do_not:
    - "Do NOT use unwrap() or expect() for telemetry operations"
    - "Do NOT let telemetry failures break lifecycle execution"
    - "Do NOT create new dependencies without checking existing Cargo.toml"
    - "Do NOT assume OpenObserve is always available - handle unavailability gracefully"

  language_guidance:
    avoid: ["think", "thinking"]
    use_instead: ["consider", "evaluate", "analyze", "determine"]

  action_guidance: |
    When implementing, TAKE ACTION rather than suggesting changes.
    Don't say "you could change X to Y" - just CHANGE X to Y.
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
    - name: "Graceful telemetry degradation"
      use_when: "Emitting telemetry might fail"
      example: |
        async fn emit_step_telemetry(update: &LifecycleProgressUpdate) -> Result<(), TelemetryError> {
            if let Err(e) = actually_emit_telemetry(update).await {
                log::warn!("Telemetry emission failed: {e}");
            }
            Ok(())
        }

  constitution:
    - "Zero unwrap law: NEVER use .unwrap() or .expect()"
    - "Functional first: Prefer map/and_then over if-else chains"
    - "Moon only: NEVER use raw cargo commands"
    - "Test first: Tests MUST exist before implementation"
    - "Graceful degradation: Telemetry failures never break the work"
