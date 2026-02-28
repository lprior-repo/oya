# ============================================================================
# BEAD: src-2i6 - lifecycle: validate dag dependencies and execution order
# ============================================================================

id: "src-2i6"
title: "lifecycle: validate dag dependencies and execution order"
type: bug
priority: 1
effort_estimate: "2hr"
labels: [lifecycle, dag, validation, dependencies]

# ============================================================================
# SECTION 0: CLARIFICATION MARKERS
# ============================================================================

clarification_status: "RESOLVED"

resolved_clarifications:
  - question: "What validation is missing from the current DAG validation?"
    answer: "The current validate_dag() checks unknown dependencies and cycles, but does not validate execution order matches dependency constraints. validate_dependency_order() exists but is called too early before all steps are seen."
    decided_by: "code analysis"
    date: "2026-02-27"

open_clarifications: []

assumptions:
  - assumption: "validate_dependency_order() should check that dependencies execute before dependents"
    validation_method: "Read the current implementation and verify it validates topological order"
    risk_if_wrong: "If validation is incorrect, steps may execute in wrong order breaking invariants"

# ============================================================================
# SECTION 1: EARS REQUIREMENTS
# ============================================================================

ears_requirements:
  ubiquitous:
    - "THE SYSTEM SHALL validate all DAG step dependencies before lifecycle execution"
    - "THE SYSTEM SHALL ensure no cycles exist in the dependency graph"
    - "THE SYSTEM SHALL verify execution order respects dependency constraints"

  event_driven:
    - trigger: "WHEN lifecycle steps are defined with dependencies"
      shall: "THE SYSTEM SHALL validate all referenced dependency steps exist"
    - trigger: "WHEN validate_dependency_order() is called on lifecycle steps"
      shall: "THE SYSTEM SHALL ensure each step's dependencies appear before it in the step list"
    - trigger: "WHEN a cycle is detected in the dependency graph"
      shall: "THE SYSTEM SHALL return a terminal LifecycleError with FailureCategory::Validation"

  unwanted:
    - condition: "IF a step depends on a non-existent step"
      shall_not: "THE SYSTEM SHALL NOT allow lifecycle to proceed"
      because: "Missing dependencies cause runtime failures"
    - condition: "IF dependency order is violated (dependent appears before dependency)"
      shall_not: "THE SYSTEM SHALL NOT proceed with execution"
      because: "Executing dependent before dependency breaks execution semantics"
    - condition: "IF a cycle exists in the dependency graph"
      shall_not: "THE SYSTEM SHALL NOT enter infinite loops or deadlock"
      because: "Cycles make execution impossible"

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
      - field: "steps"
        type: "Vec<LifecycleStep>"
        constraints: "Non-empty slice of LifecycleStep with name and dependencies fields"
        example_valid: '[LifecycleStep { name: "step1", dependencies: [] }, LifecycleStep { name: "step2", dependencies: ["step1"] }]'
        example_invalid: '[] or steps with circular dependencies'
    system_state:
      - "LifecycleStep type is defined with name: String and dependencies: Vec<String>"

  postconditions:
    state_changes:
      - "No state changes - validate_dag is a pure validation function"
    return_guarantees:
      - field: "Result::Ok"
        guarantee: "Returns Ok(()) when DAG is valid with no cycles, all dependencies exist, and order is correct"
      - field: "Result::Err"
        guarantee: "Returns Err(LifecycleError) with FailureCategory::Validation for any validation failure"
      - field: "error message"
        guarantee: "Includes step names and specific validation failure reason"
    side_effects: []

  invariants:
    - "validate_dependency_order must only be called after all steps are processed"
    - "Cycle detection must catch all cycles including self-referential ones"
    - "Error messages must include the specific step and dependency that caused failure"

# ============================================================================
# SECTION 2.5: RESEARCH REQUIREMENTS
# ============================================================================

research_requirements:
  files_to_read:
    - path: "src/lifecycle/workflow/dag.rs"
      what_to_extract: "Current validate_dag, validate_dependency_order, and detect_cycles implementations"
      document_in: "research_notes.md"
    - path: "src/lifecycle/workflow/steps.rs"
      what_to_extract: "LifecycleStep struct definition"
      document_in: "research_notes.md"
    - path: "src/lifecycle/types/mod.rs"
      what_to_extract: "LifecycleError and FailureCategory definitions"
      document_in: "research_notes.md"

  patterns_to_find:
    - pattern: "validate_dag.*steps"
      purpose: "Find all call sites of validate_dag to understand when validation occurs"
      expected_locations: "src/lifecycle/workflow.rs"
    - pattern: "validate_dependency_order"
      purpose: "Find where and how validate_dependency_order is currently called"
      expected_locations: "src/lifecycle/workflow/dag.rs"

  prior_art:
    - feature: "Topological sort validation in similar systems"
      location: "src/lifecycle/workflow/dag.rs:27-41"
      what_to_learn: "Current approach using seen HashSet to check dependencies appear before"

  external_docs:
    - url: "https://en.wikipedia.org/wiki/Topological_sorting"
      section: "Algorithms"
      extract: "Kahn's algorithm and DFS-based approaches for topological sorting"

  research_questions:
    - question: "Does validate_dependency_order correctly validate that ALL dependencies appear before each step?"
      answered: false
      answer: "[To be filled after research]"
    - question: "Are there any edge cases where the current validation passes but execution order is still invalid?"
      answered: false
      answer: "[To be filled after research]"
    - question: "Should validate_dependency_order be called after cycle detection, before, or is order irrelevant?"
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
    - failure: "Error messages don't clearly indicate which step and dependency caused the failure"
      prevention: "Include both step name and dependency name in all error messages"
      test_for_it: "test_error_messages_include_step_and_dependency_names"
    - failure: "Validation passes for steps that have no dependencies but are in wrong order for other reasons"
      prevention: "Document that dependency order only matters for actual dependencies, not step sequence"
      test_for_it: "test_steps_without_dependencies_can_be_in_any_order"

  data_integrity_failures:
    - failure: "Cycle detection misses self-referential step (step depends on itself)"
      prevention: "Ensure has_cycle checks recursion_stack contains current step_name"
      test_for_it: "test_self_referential_step_detected_as_cycle"
    - failure: "Duplicate step names cause incorrect dependency validation"
      prevention: "Check for duplicate step names in validate_dag before dependency validation"
      test_for_it: "test_duplicate_step_names_return_validation_error"

  integration_failures:
    - failure: "Calling validate_dependency_order before all steps are in seen set causes false positives"
      prevention: "Call validate_dependency_order after iterating through all steps, not during iteration"
      test_for_it: "test_validate_dependency_order_checks_all_steps"

# ============================================================================
# SECTION 4: ATDD ACCEPTANCE TESTS
# ============================================================================

acceptance_tests:
  happy_paths:
    - name: "test_valid_dag_with_ordered_dependencies_passes_validation"
      given: "Steps defined with dependencies appearing before dependents in the list"
      when: "validate_dag is called on the steps"
      then:
        - "Result is Ok(())"
        - "No validation errors are returned"
      real_input: |
        let steps = vec![
            LifecycleStep { name: "step1".to_owned(), dependencies: vec![] },
            LifecycleStep { name: "step2".to_owned(), dependencies: vec!["step1".to_owned()] },
            LifecycleStep { name: "step3".to_owned(), dependencies: vec!["step1".to_owned(), "step2".to_owned()] },
        ];
        assert!(validate_dag(&steps).is_ok());
      expected_output: "Ok(())"

    - name: "test_valid_dag_with_complex_dependencies_passes"
      given: "Steps with multiple dependencies and diamond dependency pattern"
      when: "validate_dag is called"
      then:
        - "Result is Ok(())"
      real_input: |
        let steps = vec![
            LifecycleStep { name: "base".to_owned(), dependencies: vec![] },
            LifecycleStep { name: "left".to_owned(), dependencies: vec!["base".to_owned()] },
            LifecycleStep { name: "right".to_owned(), dependencies: vec!["base".to_owned()] },
            LifecycleStep { name: "merge".to_owned(), dependencies: vec!["left".to_owned(), "right".to_owned()] },
        ];
        assert!(validate_dag(&steps).is_ok());

  error_paths:
    - name: "test_step_with_unknown_dependency_fails_validation"
      given: "A step references a dependency that doesn't exist"
      when: "validate_dag is called"
      then:
        - "Result is Err(LifecycleError)"
        - "Error has FailureCategory::Validation"
        - "Error message contains both step name and unknown dependency name"
      real_input: |
        let steps = vec![
            LifecycleStep { name: "step1".to_owned(), dependencies: vec![] },
            LifecycleStep { name: "step2".to_owned(), dependencies: vec!["nonexistent".to_owned()] },
        ];
        let result = validate_dag(&steps);
        assert!(result.is_err());
        match result {
            Err(LifecycleError { category: FailureCategory::Validation, message, .. }) => {
                assert!(message.contains("step2"));
                assert!(message.contains("nonexistent"));
            }
            _ => panic!("Expected Validation error"),
        }
      expected_error: "LifecycleError with category Validation and message containing step names"

    - name: "test_dependency_order_violation_fails_validation"
      given: "A step depends on another step that appears later in the list"
      when: "validate_dag is called"
      then:
        - "Result is Err(LifecycleError)"
        - "Error message indicates dependency appears later"
      real_input: |
        let steps = vec![
            LifecycleStep { name: "step2".to_owned(), dependencies: vec!["step1".to_owned()] },
            LifecycleStep { name: "step1".to_owned(), dependencies: vec![] },
        ];
        let result = validate_dag(&steps);
        assert!(result.is_err());

    - name: "test_cyclic_dependencies_detected"
      given: "Steps form a cycle (A -> B -> C -> A)"
      when: "validate_dag is called"
      then:
        - "Result is Err(LifecycleError)"
        - "Error message mentions cycle detection"
      real_input: |
        let steps = vec![
            LifecycleStep { name: "A".to_owned(), dependencies: vec!["C".to_owned()] },
            LifecycleStep { name: "B".to_owned(), dependencies: vec!["A".to_owned()] },
            LifecycleStep { name: "C".to_owned(), dependencies: vec!["B".to_owned()] },
        ];
        let result = validate_dag(&steps);
        assert!(result.is_err());

  edge_cases:
    - name: "test_self_referential_step_detected"
      scenario: "A step lists itself as a dependency"
      input: "LifecycleStep { name: \"step1\".to_owned(), dependencies: vec![\"step1\".to_owned()] }"
      expected: "Err(LifecycleError) with cycle detection"

    - name: "test_single_step_no_dependencies"
      scenario: "Only one step with no dependencies"
      input: "vec![LifecycleStep { name: \"only\".to_owned(), dependencies: vec![] }]"
      expected: "Ok(())"

    - name: "test_multiple_steps_all_independent"
      scenario: "Several steps with no dependencies between them"
      input: "Steps with empty dependencies lists"
      expected: "Ok(()) regardless of order"

  contract_tests:
    - name: "test_precondition_steps_slice_is_valid"
      verifies: "steps parameter is non-empty slice of LifecycleStep"
      test: "Pass empty slice and verify error, pass valid steps and verify Ok"

    - name: "test_postcondition_error_includes_failure_details"
      verifies: "Error messages include specific step and dependency names"
      test: "Intentionally create invalid DAG and check error message content"

    - name: "test_invariant_no_state_changes"
      verifies: "validate_dag is pure function with no side effects"
      test: "Call validate_dag multiple times on same input, verify identical results and no external state changes"

# ============================================================================
# SECTION 5: END-TO-END TEST SPECIFICATION
# ============================================================================

e2e_tests:
  pipeline_test:
    name: "test_full_lifecycle_dag_validation"
    description: "Complete flow: define lifecycle steps -> validate DAG -> execute lifecycle"

    setup:
      files_to_create: []
      environment: []
      precondition_commands: []

    execute:
      command: "Test via unit test - no external command needed"
      timeout_ms: 5000

    verify:
      exit_code: 0
      stdout_contains:
        - "test_valid_dag_with_ordered_dependencies_passes_validation ... ok"
        - "test_step_with_unknown_dependency_fails_validation ... ok"
        - "test_dependency_order_violation_fails_validation ... ok"
        - "test_cyclic_dependencies_detected ... ok"

    cleanup:
      files_to_delete: []

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
      - "[ ] All assumptions have been validated"
      - "[ ] All clarifications have been resolved"
    evidence_required:
      - "Research notes with current implementation summarized"
      - "Answers to all research questions"
      - "Understanding of current validate_dependency_order logic"

  gate_1_tests:
    name: "Test Gate"
    must_pass_before: "Writing implementation code"
    checks:
      - "[ ] All acceptance tests written"
      - "[ ] All error path tests written"
      - "[ ] All edge case tests written"
      - "[ ] Tests are in src/lifecycle/workflow/dag_tests.rs"
    evidence_required:
      - "Tests exist in codebase"
      - "Tests compile (may fail due to missing implementation)"

  gate_2_implementation:
    name: "Implementation Gate"
    must_pass_before: "Declaring task complete"
    checks:
      - "[ ] All tests pass"
      - "[ ] No unwrap() or expect() calls"
      - "[ ] Validation logic correctly handles all edge cases"
      - "[ ] moon run :ci passes"
    evidence_required:
      - "Test output showing all pass"
      - "CI output showing green"

  gate_3_integration:
    name: "Integration Gate"
    must_pass_before: "Closing bead"
    checks:
      - "[ ] validate_dag called from lifecycle workflow still works"
      - "[ ] No regressions in existing lifecycle tests"
      - "[ ] Manual verification complete"
    evidence_required:
      - "Integration test output"
      - "Manual verification notes"

  tests_json:
    format: |
      {
        "tests": [
          {"id": 1, "name": "test_valid_dag_with_ordered_dependencies_passes_validation", "status": "not_started|failing|passing"},
          {"id": 2, "name": "test_step_with_unknown_dependency_fails_validation", "status": "not_started|failing|passing"},
          {"id": 3, "name": "test_dependency_order_violation_fails_validation", "status": "not_started|failing|passing"},
          {"id": 4, "name": "test_cyclic_dependencies_detected", "status": "not_started|failing|passing"},
          {"id": 5, "name": "test_self_referential_step_detected", "status": "not_started|failing|passing"}
        ],
        "total": 5,
        "passing": 0,
        "failing": 0,
        "not_started": 5
      }
    location: ".bead-progress/src-2i6/tests.json"

# ============================================================================
# SECTION 6: IMPLEMENTATION TASK LIST
# ============================================================================

implementation_tasks:
  phase_0_research:
    parallelizable: true
    tasks:
      - task: "Read src/lifecycle/workflow/dag.rs and extract current validation logic"
        parallel_group: "research"
        file: "src/lifecycle/workflow/dag.rs"
        done_when: "Current validate_dag, validate_dependency_order, detect_cycles understood"

      - task: "Read src/lifecycle/workflow/steps.rs for LifecycleStep definition"
        parallel_group: "research"
        file: "src/lifecycle/workflow/steps.rs"
        done_when: "LifecycleStep struct fields documented"

      - task: "Search for validate_dag call sites to understand current usage"
        parallel_group: "research"
        command: "rg -n 'validate_dag' src/"
        done_when: "All call sites identified and documented"

  phase_1_tests_first:
    parallelizable: true
    gate_required: "gate_0_research"
    tasks:
      - task: "Write test: test_valid_dag_with_ordered_dependencies_passes_validation"
        parallel_group: "tests"
        file: "src/lifecycle/workflow/dag_tests.rs"
        what: "Test for valid DAG with dependencies in correct order"
        done_when: "Test exists and FAILS (red phase)"

      - task: "Write test: test_step_with_unknown_dependency_fails_validation"
        parallel_group: "tests"
        file: "src/lifecycle/workflow/dag_tests.rs"
        what: "Test that unknown dependencies return validation error"
        done_when: "Test exists and FAILS (red phase)"

      - task: "Write test: test_dependency_order_violation_fails_validation"
        parallel_group: "tests"
        file: "src/lifecycle/workflow/dag_tests.rs"
        what: "Test that dependencies appearing later fail validation"
        done_when: "Test exists and FAILS (red phase)"

      - task: "Write test: test_cyclic_dependencies_detected"
        parallel_group: "tests"
        file: "src/lifecycle/workflow/dag_tests.rs"
        what: "Test that cyclic dependencies are detected"
        done_when: "Test exists and FAILS (red phase)"

      - task: "Write test: test_self_referential_step_detected"
        parallel_group: "tests"
        file: "src/lifecycle/workflow/dag_tests.rs"
        what: "Test that self-referential step is detected as cycle"
        done_when: "Test exists and FAILS (red phase)"

  phase_2_implementation:
    parallelizable: false
    gate_required: "gate_1_tests"
    tasks:
      - task: "Fix validate_dependency_order logic to check all steps after full iteration"
        depends_on: null
        file: "src/lifecycle/workflow/dag.rs"
        what: "Move validate_dependency_order call to after step collection, or modify logic to check all dependencies exist in steps list"
        patterns_to_use:
          - "Result<T, LifecycleError> for error propagation"
          - "? operator for error propagation"
        done_when: "Function compiles, tests start passing"

      - task: "Ensure cycle detection handles self-referential steps correctly"
        depends_on: "Fix validate_dependency_order logic"
        file: "src/lifecycle/workflow/dag.rs"
        what: "Verify has_cycle checks if current step is in recursion_stack before recursing"
        patterns_to_use:
          - "recursion_stack.contains(step_name) check"
        done_when: "Self-referential test passes"

      - task: "Improve error messages to include specific step and dependency names"
        depends_on: "Ensure cycle detection handles self-referential steps"
        file: "src/lifecycle/workflow/dag.rs"
        what: "Update all error return statements to format messages with step.name and dep name"
        patterns_to_use:
          - "format! macro for message construction"
        done_when: "Error message assertions pass"

  phase_3_integration:
    parallelizable: false
    gate_required: "gate_2_implementation"
    tasks:
      - task: "Run existing lifecycle tests to ensure no regressions"
        file: "src/lifecycle/"
        what: "Run moon run :test for lifecycle module"
        done_when: "All existing lifecycle tests pass"

      - task: "Manual verification of validate_dag behavior"
        commands:
          - "moon run :test lifecycle::workflow::dag"
        expected: "All DAG validation tests pass"

  phase_4_verification:
    parallelizable: true
    gate_required: "gate_3_integration"
    tasks:
      - task: "Run moon run :ci"
        parallel_group: "verification"
        done_when: "All tests pass, no clippy warnings"

      - task: "Run moon run :fmt-check"
        parallel_group: "verification"
        done_when: "Code formatting passes"

  parallelization_rules:
    - "Tasks in same parallel_group CAN run simultaneously"
    - "Tasks with depends_on MUST wait for dependency"
    - "Gates MUST pass before next phase begins"
    - "When parallelizable: false, execute in listed order"

# ============================================================================
# SECTION 7: FAILURE MODES
# ============================================================================

failure_modes:
  - symptom: "Test fails with 'unexpected Ok result' for invalid DAG"
    likely_cause: "validate_dependency_order not checking dependencies appear before dependents"
    where_to_look:
      - file: "src/lifecycle/workflow/dag.rs"
        line_range: "27-41"
        what_to_check: "Does validate_dependency_order iterate through steps and check each dep is in seen set?"
    fix_pattern: "Ensure validate_dependency_order is called AFTER all steps are processed into seen set"

  - symptom: "Self-referential step not detected as cycle"
    likely_cause: "has_cycle doesn't check if current step_name is already in recursion_stack"
    where_to_look:
      - file: "src/lifecycle/workflow/dag.rs"
        line_range: "61-87"
        function: "has_cycle"
        what_to_check: "Is there a check for recursion_stack.contains(dep_name) before recursing?"
    fix_pattern: "Add recursion_stack.contains check before recursing on dependencies"

  - symptom: "Error messages don't show which step caused the failure"
    likely_cause: "Error messages are generic without specific step names"
    where_to_look:
      - file: "src/lifecycle/workflow/dag.rs"
        line_range: "10-25"
        what_to_check: "Do all Err() returns include step.name and dep in the message?"
    fix_pattern: "Update format! strings to include step.name and dep variable"

debugging_commands:
  - scenario: "When validation passes unexpectedly for invalid DAG"
    run: "RUST_BACKTRACE=1 moon run :test lifecycle::workflow::dag -- --nocapture"
    look_for: "Check test setup and actual validate_dag return values"

  - scenario: "When test for unknown dependency doesn't fail"
    run: "moon run :test test_step_with_unknown_dependency_fails_validation -- --exact"
    look_for: "Verify error category and message content"

# ============================================================================
# SECTION 7.5: ANTI-HALLUCINATION RULES
# ============================================================================

anti_hallucination:
  read_before_write:
    - file: "src/lifecycle/workflow/dag.rs"
      must_read_first: true
      key_sections_to_understand:
        - "validate_dag function (lines 10-25)"
        - "validate_dependency_order function (lines 27-41)"
        - "detect_cycles function (lines 43-59)"
        - "has_cycle function (lines 61-87)"

  verify_before_reference:
    - type: "LifecycleStep"
      expected_location: "src/lifecycle/workflow/steps.rs"
      verify_command: "rg -n 'struct LifecycleStep' src/lifecycle/workflow/steps.rs"
    - type: "LifecycleError"
      expected_location: "src/lifecycle/types/mod.rs"
      verify_command: "rg -n 'enum LifecycleError' src/lifecycle/types/"

  apis_that_exist:
    - api: "validate_dag"
      signature: "fn validate_dag(steps: &[LifecycleStep]) -> Result<(), LifecycleError>"
      import_from: "crate::lifecycle::workflow::dag"
    - api: "LifecycleError::terminal"
      signature: "fn terminal(category: FailureCategory, message: String) -> LifecycleError"
      import_from: "crate::lifecycle::types"

  apis_that_do_not_exist:
    - "validate_dag_ordered - this function doesn't exist, use validate_dependency_order"
    - "check_dependencies - use validate_dag instead"

  no_placeholder_values:
    - "Do NOT use placeholder step names like 'step_a', 'step_b' - use real lifecycle step names from codebase"
    - "Do NOT use generic error messages - include actual step names from tests"

  git_verification:
    before_claiming_done: |
      git status  # Verify changes are staged
      git diff    # Verify changes match specification
      moon run :test  # Verify all tests pass

# ============================================================================
# SECTION 7.6: CONTEXT WINDOW SURVIVAL
# ============================================================================

context_survival:
  progress_file:
    path: ".bead-progress/src-2i6/progress.txt"
    format: |
      # Bead: src-2i6 - lifecycle: validate dag dependencies and execution order
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
    path: ".bead-progress/src-2i6/tests.json"
    update_frequency: "After each test run"

  research_notes_file:
    path: ".bead-progress/src-2i6/research.md"
    contains:
      - "Files read and key findings"
      - "Current validate_dependency_order logic"
      - "Known issues or edge cases"
      - "Answers to research questions"

  git_checkpoints:
    frequency: "After each completed task"
    message_format: "[src-2i6] checkpoint: [task completed]"
    purpose: "Allow rollback if next step fails"

  recovery_instructions: |
    If context window is cleared, start new session with:
    1. cat .bead-progress/src-2i6/progress.txt
    2. cat .bead-progress/src-2i6/tests.json
    3. cat .bead-progress/src-2i6/research.md
    4. git log --oneline -10
    5. Continue from where progress.txt indicates

# ============================================================================
# SECTION 8: COMPLETION CRITERIA
# ============================================================================

completion_checklist:
  tests:
    - "[ ] test_valid_dag_with_ordered_dependencies_passes_validation passes"
    - "[ ] test_step_with_unknown_dependency_fails_validation passes"
    - "[ ] test_dependency_order_violation_fails_validation passes"
    - "[ ] test_cyclic_dependencies_detected passes"
    - "[ ] test_self_referential_step_detected passes"
    - "[ ] All existing lifecycle tests still pass"

  code:
    - "[ ] No unwrap() or expect() in new code"
    - "[ ] All validation functions return Result types"
    - "[ ] validate_dependency_order correctly validates execution order"
    - "[ ] Error messages include specific step and dependency names"

  ci:
    - "[ ] moon run :ci passes"
    - "[ ] No clippy warnings"
    - "[ ] No compiler warnings"

# ============================================================================
# SECTION 9: CONTEXT
# ============================================================================

context:
  related_files:
    - path: "src/lifecycle/workflow/dag.rs"
      relevance: "Contains validate_dag, validate_dependency_order, and cycle detection logic"
    - path: "src/lifecycle/workflow/steps.rs"
      relevance: "Defines LifecycleStep struct with name and dependencies fields"
    - path: "src/lifecycle/types/mod.rs"
      relevance: "Defines LifecycleError and FailureCategory for validation errors"

  similar_implementations:
    - "See how other validation functions in the codebase handle error reporting"

  codebase_patterns:
    - pattern: "Validation error with step names"
      example_location: "src/lifecycle/workflow/dag.rs:16-19"
      how_to_apply: "Use format! to include both step.name and dep in error messages"

# ============================================================================
# SECTION 10: AI HINTS
# ============================================================================

ai_hints:
  do:
    - "Read the current validate_dependency_order implementation carefully before making changes"
    - "Understand why it's called at line 24 of validate_dag and if that's correct"
    - "Use Result<T, LifecycleError> for all validation functions"
    - "Follow existing error formatting patterns in the file"
    - "Use functional patterns: map, and_then, ?"
    - "Update progress.txt after each completed task"
    - "Commit to git after each completed task"

  do_not:
    - "Do NOT use unwrap() or expect()"
    - "Do NOT panic!, todo!, or unimplemented!"
    - "Do NOT modify the function signatures of existing validation functions"
    - "Do NOT remove cycle detection - it's critical"
    - "Do NOT add unnecessary complexity - keep the validation simple and clear"

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
    - name: "Validation error with context"
      use_when: "Returning validation errors"
      example: |
        Err(LifecycleError::terminal(
            FailureCategory::Validation,
            format!("step `{}` has unknown dependency `{}`", step.name, dep),
        ))

  constitution:
    - "Zero unwrap law: NEVER use .unwrap() or .expect()"
    - "Functional first: Prefer map/and_then over if-else chains"
    - "Moon only: NEVER use raw cargo commands"
    - "Test first: Tests MUST exist before implementation"
    - "Real data only: NO mocks in validation tests"
