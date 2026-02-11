# World-Class Beads Ticket Template

## The Philosophy

> **If GPT-4 or a competent high school senior cannot implement this ticket perfectly on their first attempt, the ticket is incomplete.**

Every bead must be so rigorously specified that implementation becomes mechanical. The AI has no choice but to succeed because every edge case, failure mode, and test is explicitly enumerated.

---

## Research-Backed Principles (2025-2026)

This template incorporates findings from:
- [Anthropic Claude 4.x Best Practices](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-4-best-practices)
- [GitHub Spec-Driven Development](https://github.com/github/spec-kit)
- [Martin Fowler's Spec-Driven Development Analysis](https://martinfowler.com/articles/exploring-gen-ai/sdd-3-tools.html)
- [ThoughtWorks Engineering Practices 2025](https://www.thoughtworks.com/en-us/insights/blog/agile-engineering-practices/spec-driven-development-unpacking-2025-new-engineering-practices)

Key insight: **Specifications become the source of truth. Code is just the expression of specifications in a particular language.**

---

## Template Structure

```yaml
# ============================================================================
# BEAD: [ID] - [Title]
# ============================================================================

id: "intent-cli-XXXX"
title: "[Component]: [Action verb] [specific thing]"
type: feature | bug | task | epic | chore
priority: 0 (critical) | 1 (high) | 2 (medium) | 3 (low) | 4 (backlog)
effort_estimate: "15min | 30min | 1hr | 2hr | 4hr"  # Max 4hr per bead
labels: [component, category, methodology]

# ============================================================================
# SECTION 0: CLARIFICATION MARKERS (Anti-Assumption Gate)
# ============================================================================
# From GitHub Spec Kit: "Use [NEEDS CLARIFICATION: specific question] for
# any ambiguities rather than making assumptions"
#
# AI agents hallucinate when they ASSUME. Force them to ASK.
# Every ambiguity must be explicitly flagged BEFORE implementation.

clarification_status: "RESOLVED" | "HAS_OPEN_QUESTIONS"

resolved_clarifications:
  # Questions that WERE ambiguous but are now answered
  - question: "[What was unclear?]"
    answer: "[The definitive answer]"
    decided_by: "[Who decided - human or prior research]"
    date: "[When resolved]"

open_clarifications:
  # Questions that MUST be answered before implementation can begin
  # If this section is non-empty, the bead is NOT READY for implementation
  - question: "[NEEDS CLARIFICATION: specific question]"
    context: "[Why this matters for implementation]"
    options:
      - "[Option A and its implications]"
      - "[Option B and its implications]"
    default_if_unresolved: "[What to do if no answer comes]"

assumptions:
  # Explicit assumptions made - AI must validate these, not blindly accept
  - assumption: "[What we're assuming is true]"
    validation_method: "[How to verify this assumption]"
    risk_if_wrong: "[What breaks if assumption is false]"

# ============================================================================
# SECTION 1: EARS REQUIREMENTS (What must happen)
# ============================================================================
# EARS = Easy Approach to Requirements Syntax
# Every requirement MUST use one of these 6 patterns:

ears_requirements:
  # Pattern 1: UBIQUITOUS - Always true, no conditions
  ubiquitous:
    - "THE SYSTEM SHALL [behavior that is always true]"
    - "THE SYSTEM SHALL [another universal behavior]"

  # Pattern 2: EVENT-DRIVEN - Trigger-response pairs
  event_driven:
    - trigger: "WHEN [specific user action or system event]"
      shall: "THE SYSTEM SHALL [specific response]"
    - trigger: "WHEN [another trigger]"
      shall: "THE SYSTEM SHALL [another response]"

  # Pattern 3: STATE-DRIVEN - Behavior during specific states
  state_driven:
    - state: "WHILE [system/user/resource is in state X]"
      shall: "THE SYSTEM SHALL [behavior during that state]"

  # Pattern 4: OPTIONAL - Conditional on configuration/roles
  optional:
    - condition: "WHERE [feature flag / user role / config option]"
      shall: "THE SYSTEM SHALL [conditional behavior]"

  # Pattern 5: UNWANTED - Things that must NEVER happen (Inversion!)
  unwanted:
    - condition: "IF [bad state or input]"
      shall_not: "THE SYSTEM SHALL NOT [forbidden behavior]"
      because: "[Why this would be catastrophic]"

  # Pattern 6: COMPLEX - State + Event combinations
  complex:
    - state: "WHILE [in state X]"
      trigger: "WHEN [event Y occurs]"
      shall: "THE SYSTEM SHALL [combined behavior]"

# ============================================================================
# SECTION 2: KIRK CONTRACTS (Design by Contract)
# ============================================================================
# KIRK = Knowledge-Informed Requirements & Kontract
# Every behavior has preconditions, postconditions, and invariants

contracts:
  preconditions:
    # What MUST be true before this code runs
    auth_required: true | false
    required_inputs:
      - field: "field_name"
        type: "String | Int | Bool | List | Object"
        constraints: "[validation rules]"
        example_valid: "[concrete valid value]"
        example_invalid: "[concrete invalid value]"
    system_state:
      - "[State that must exist before execution]"

  postconditions:
    # What MUST be true after this code runs
    state_changes:
      - "[Specific change to system state]"
      - "[Another state change]"
    return_guarantees:
      - field: "response.field"
        guarantee: "[What is guaranteed about this field]"
      - field: "exit_code"
        guarantee: "0 on success, 3 on invalid input, 4 on missing resource"
    side_effects:
      - "[Any IO, network, file, or external effects]"

  invariants:
    # What must ALWAYS be true, before and after
    - "[Invariant 1 - e.g., 'Passwords never appear in logs']"
    - "[Invariant 2 - e.g., 'All timestamps are ISO8601']"
    - "[Invariant 3 - e.g., 'Exit codes match AGENTS.md spec']"

# ============================================================================
# SECTION 2.5: RESEARCH REQUIREMENTS (Investigate Before Implementing)
# ============================================================================
# From Anthropic: "ALWAYS read and understand relevant files before proposing
# code edits. Do not speculate about code you have not inspected."
#
# AI must PROVE it has done the research before writing any code.
# This section defines what must be investigated and documented.

research_requirements:
  # Files that MUST be read before implementation
  files_to_read:
    - path: "[exact file path]"
      what_to_extract: "[What information to gather]"
      document_in: "research_notes.md"

  # Patterns to find in codebase (grep/search)
  patterns_to_find:
    - pattern: "[regex or string to search]"
      purpose: "[Why this matters]"
      expected_locations: "[Where you expect to find it]"

  # Existing implementations to study
  prior_art:
    - feature: "[Similar feature already in codebase]"
      location: "[file:line]"
      what_to_learn: "[What patterns to copy]"

  # External documentation to consult
  external_docs:
    - url: "[Documentation URL]"
      section: "[Specific section needed]"
      extract: "[What information to gather]"

  # Questions the research must answer
  research_questions:
    - question: "[What must be answered by research]"
      answered: false
      answer: "[To be filled after research]"

  # Research gate - AI cannot proceed until this is done
  research_complete_when:
    - "[ ] All files_to_read have been opened and key info extracted"
    - "[ ] All patterns_to_find have been searched"
    - "[ ] All prior_art has been examined"
    - "[ ] All research_questions have answers documented"

# ============================================================================
# SECTION 3: INVERSION ANALYSIS (What could go wrong)
# ============================================================================
# Charlie Munger: "Invert, always invert"
# Define failure modes BEFORE implementation

inversions:
  security_failures:
    - failure: "[Security vulnerability that could occur]"
      prevention: "[How the code MUST prevent this]"
      test_for_it: "[Specific test case]"

  usability_failures:
    - failure: "[UX problem that could occur]"
      prevention: "[How to prevent it]"
      test_for_it: "[Specific test case]"

  data_integrity_failures:
    - failure: "[Data corruption/loss scenario]"
      prevention: "[How to prevent it]"
      test_for_it: "[Specific test case]"

  integration_failures:
    - failure: "[What could break downstream systems]"
      prevention: "[How to prevent it]"
      test_for_it: "[Specific test case]"

# ============================================================================
# SECTION 4: ATDD ACCEPTANCE TESTS (Tests FIRST, code second)
# ============================================================================
# ATDD = Acceptance Test-Driven Development
# Define the EXACT tests that prove the feature works
# NO MOCKS. NO FAKE DATA. REAL END-TO-END TESTS.

acceptance_tests:
  # Happy path tests (must all pass for bead to close)
  happy_paths:
    - name: "test_[descriptive_name]"
      given: "[Exact precondition state - real data]"
      when: "[Exact action taken - real command/call]"
      then:
        - "[Exact assertion 1 - real output]"
        - "[Exact assertion 2]"
      real_input: |
        [Actual input data - not placeholder]
      expected_output: |
        [Actual expected output - not placeholder]

  # Error path tests (every failure mode must be tested)
  error_paths:
    - name: "test_[error_scenario]"
      given: "[Precondition that leads to error]"
      when: "[Action that triggers error]"
      then:
        - "Exit code is [specific code]"
        - "Error message contains '[specific text]'"
        - "No side effects occurred"
      real_input: |
        [Actual invalid input]
      expected_error: |
        [Actual error response]

  # Edge case tests (boundary conditions)
  edge_cases:
    - name: "test_[edge_case]"
      scenario: "[Description of boundary condition]"
      input: "[Exact edge case input]"
      expected: "[Exact expected behavior]"

  # Contract verification tests (prove contracts hold)
  contract_tests:
    - name: "test_precondition_[name]"
      verifies: "[Which precondition]"
      test: "[How to verify it]"
    - name: "test_postcondition_[name]"
      verifies: "[Which postcondition]"
      test: "[How to verify it]"
    - name: "test_invariant_[name]"
      verifies: "[Which invariant]"
      test: "[How to verify it]"

# ============================================================================
# SECTION 5: END-TO-END TEST SPECIFICATION
# ============================================================================
# SOUP TO NUTS: Full pipeline testing with REAL data
# This is the Martin Fowler "walking skeleton" test

e2e_tests:
  pipeline_test:
    name: "test_full_pipeline_[feature]"
    description: "Complete end-to-end test from raw input to final output"

    # Step 1: Setup (real state, real data)
    setup:
      files_to_create:
        - path: "[exact file path]"
          content: |
            [exact file content - real data]
      environment:
        - "[Environment variable]=[value]"
      precondition_commands:
        - "[Command to run before test]"

    # Step 2: Execute (real command)
    execute:
      command: "[Exact command to run]"
      stdin: |
        [Exact stdin if any]
      timeout_ms: 5000

    # Step 3: Verify (real assertions)
    verify:
      exit_code: 0
      stdout_contains:
        - "[Exact string that must appear]"
      stdout_matches_json:
        field: "expected.value"
        type: "string"
        pattern: "[regex pattern]"
      files_created:
        - path: "[file that should exist]"
          contains: "[content verification]"
      files_not_modified:
        - "[file that should not change]"
      side_effects:
        - "[Verifiable side effect]"

    # Step 4: Cleanup
    cleanup:
      commands:
        - "[Cleanup command]"
      files_to_delete:
        - "[temp file path]"

  # Additional E2E scenarios
  e2e_scenarios:
    - name: "e2e_[scenario_name]"
      description: "[What this proves]"
      steps:
        - action: "[Step 1]"
          verify: "[Verification]"
        - action: "[Step 2]"
          verify: "[Verification]"

# ============================================================================
# SECTION 5.5: VERIFICATION CHECKPOINTS (Quality Gates)
# ============================================================================
# From Anthropic: "Have the model write tests in a structured format and keep
# track of them in a structured format (e.g., tests.json)"
#
# AI must pass each gate before proceeding to the next phase.
# This prevents "just make the test pass" shortcuts.

verification_checkpoints:
  # Gate 0: Research Complete
  gate_0_research:
    name: "Research Gate"
    must_pass_before: "Writing any code"
    checks:
      - "[ ] All research_requirements files have been read"
      - "[ ] All research_questions have documented answers"
      - "[ ] All assumptions have been validated"
      - "[ ] All clarifications have been resolved"
    evidence_required:
      - "Research notes with file contents summarized"
      - "Answers to all research questions"

  # Gate 1: Tests Written
  gate_1_tests:
    name: "Test Gate"
    must_pass_before: "Writing implementation code"
    checks:
      - "[ ] All acceptance tests written"
      - "[ ] All error path tests written"
      - "[ ] All E2E tests written"
      - "[ ] Tests are in tests.json structured format"
    evidence_required:
      - "Tests exist in codebase"
      - "Tests fail with expected 'not implemented' errors"

  # Gate 2: Implementation Complete
  gate_2_implementation:
    name: "Implementation Gate"
    must_pass_before: "Declaring task complete"
    checks:
      - "[ ] All tests pass"
      - "[ ] No unwrap() or expect() calls"
      - "[ ] Exit codes match specification"
      - "[ ] moon run :ci passes"
    evidence_required:
      - "Test output showing all pass"
      - "CI output showing green"

  # Gate 3: Integration Verified
  gate_3_integration:
    name: "Integration Gate"
    must_pass_before: "Closing bead"
    checks:
      - "[ ] E2E tests pass with real data"
      - "[ ] No regressions in existing tests"
      - "[ ] Manual verification complete"
    evidence_required:
      - "E2E test output"
      - "Manual verification notes"

  # Structured test tracking (Claude 4.x best practice)
  tests_json:
    format: |
      {
        "tests": [
          {"id": 1, "name": "test_name", "status": "not_started|failing|passing"},
          ...
        ],
        "total": N,
        "passing": 0,
        "failing": 0,
        "not_started": N
      }
    location: "[where to save tests.json]"

# ============================================================================
# SECTION 6: IMPLEMENTATION TASK LIST
# ============================================================================
# From Anthropic: "If you intend to call multiple tools and there are no
# dependencies between the tool calls, make all independent calls in parallel"
#
# Explicit, ordered, atomic tasks with PARALLELIZATION MARKERS
# Each task should be completable in <30 minutes

implementation_tasks:
  # Phase 0: Research (must complete before any other phase)
  phase_0_research:
    parallelizable: true  # These can all run in parallel
    tasks:
      - task: "Read [file1] and extract [patterns]"
        parallel_group: "research"
        file: "[exact file path]"
        done_when: "Key patterns documented in research notes"

      - task: "Search for [pattern] in codebase"
        parallel_group: "research"
        command: "grep -r '[pattern]' src/"
        done_when: "All occurrences documented"

      - task: "Review prior art: [similar feature]"
        parallel_group: "research"
        file: "[exact file path]"
        done_when: "Patterns to copy identified"

  phase_1_tests_first:
    parallelizable: true  # Test writing can be parallel
    gate_required: "gate_0_research"
    tasks:
      - task: "Write test: test_[name]"
        parallel_group: "tests"
        file: "[exact file path]"
        what: "[Exact test to write]"
        done_when: "Test exists and FAILS (red phase)"

      - task: "Write test: test_[name2]"
        parallel_group: "tests"
        file: "[exact file path]"
        what: "[Exact test to write]"
        done_when: "Test exists and FAILS (red phase)"

  phase_2_implementation:
    parallelizable: false  # These are sequential - order matters!
    gate_required: "gate_1_tests"
    tasks:
      - task: "Implement [function/module]"
        depends_on: null  # First task
        file: "[exact file path]"
        what: "[Exact implementation]"
        patterns_to_use:
          - "Result<T, Error> for all fallible operations"
          - "? operator for error propagation"
          - "[Other required patterns]"
        done_when: "Function compiles, some tests pass"

      - task: "Implement [second function]"
        depends_on: "Implement [function/module]"  # Must complete first
        file: "[exact file path]"
        what: "[Exact implementation]"
        done_when: "All phase_1 tests PASS (green phase)"

  phase_3_integration:
    parallelizable: false
    gate_required: "gate_2_implementation"
    tasks:
      - task: "Wire up [component] to [system]"
        file: "[exact file path]"
        what: "[Exact integration work]"
        done_when: "E2E test passes"

  phase_4_verification:
    parallelizable: true  # Verification steps can run in parallel
    gate_required: "gate_3_integration"
    tasks:
      - task: "Run moon run :ci"
        parallel_group: "verification"
        done_when: "All tests pass, no clippy warnings"

      - task: "Manual verification"
        parallel_group: "verification"
        commands:
          - "[Command to run]"
        expected: "[Expected output]"

  # Parallelization guidance for AI
  parallelization_rules:
    - "Tasks in same parallel_group CAN run simultaneously"
    - "Tasks with depends_on MUST wait for dependency"
    - "Gates MUST pass before next phase begins"
    - "When parallelizable: false, execute in listed order"

# ============================================================================
# SECTION 7: FAILURE MODES & DEBUGGING GUIDE
# ============================================================================
# Where to look when things go wrong

failure_modes:
  - symptom: "[Observable problem]"
    likely_cause: "[What probably went wrong]"
    where_to_look:
      - file: "[file path]"
        line_range: "[start-end]"
        what_to_check: "[Specific thing to verify]"
    fix_pattern: "[How to fix it]"

  - symptom: "[Another problem]"
    likely_cause: "[Cause]"
    where_to_look:
      - file: "[file path]"
        function: "[function name]"
        what_to_check: "[What to verify]"
    fix_pattern: "[Fix approach]"

debugging_commands:
  - scenario: "[When X happens]"
    run: "[Command to debug]"
    look_for: "[What to look for in output]"

# ============================================================================
# SECTION 7.5: ANTI-HALLUCINATION RULES (Ground Truth Enforcement)
# ============================================================================
# From Anthropic: "Never speculate about code you have not opened. If the user
# references a specific file, you MUST read the file before answering."
#
# AI must PROVE it has seen the code before modifying it.
# These rules prevent the #1 cause of AI implementation failures.

anti_hallucination:
  # Rule 1: Read Before Write
  read_before_write:
    - file: "[file to be modified]"
      must_read_first: true
      key_sections_to_understand:
        - "[Function/struct that will be modified]"
        - "[Import patterns used]"
        - "[Error handling patterns used]"

  # Rule 2: Verify Existence Before Reference
  verify_before_reference:
    - type: "[Type/function/constant being referenced]"
      expected_location: "[where it should be defined]"
      verify_command: "grep -n '[type_name]' [file]"

  # Rule 3: No Invented APIs
  apis_that_exist:
    - api: "[Actual API/function to use]"
      signature: "[Exact function signature]"
      import_from: "[Module path]"
    - api: "[Another API]"
      signature: "[Signature]"
      import_from: "[Module]"

  apis_that_do_not_exist:
    - "[Common hallucinated API that looks plausible but doesn't exist]"
    - "[Another non-existent API AI might invent]"

  # Rule 4: Concrete Examples Only
  no_placeholder_values:
    - "Do NOT use placeholder values like 'example.com'"
    - "Do NOT use 'lorem ipsum' or 'test' data"
    - "Use REAL values from the codebase or specification"

  # Rule 5: Git as Ground Truth
  git_verification:
    before_claiming_done: |
      git status  # Verify changes are staged
      git diff    # Verify changes match specification
      moon run :test  # Verify tests pass

# ============================================================================
# SECTION 7.6: CONTEXT WINDOW SURVIVAL (Long-Running Task Support)
# ============================================================================
# From Anthropic: "For tasks spanning multiple context windows, use structured
# state files and git for state tracking."
#
# AI must be able to resume from where it left off if context is cleared.
# All progress must be externalized to files, not held in context.

context_survival:
  # Progress tracking file
  progress_file:
    path: ".bead-progress/[bead-id]/progress.txt"
    format: |
      # Bead: [id] - [title]
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

  # Test status tracking (structured JSON)
  tests_status_file:
    path: ".bead-progress/[bead-id]/tests.json"
    update_frequency: "After each test run"

  # Research notes (persistent)
  research_notes_file:
    path: ".bead-progress/[bead-id]/research.md"
    contains:
      - "Files read and key findings"
      - "Patterns discovered"
      - "Prior art examined"
      - "Answers to research questions"

  # Git checkpoints
  git_checkpoints:
    frequency: "After each completed task"
    message_format: "[bead-id] checkpoint: [task completed]"
    purpose: "Allow rollback if next step fails"

  # Recovery instructions (for fresh context window)
  recovery_instructions: |
    If context window is cleared, start new session with:
    1. cat .bead-progress/[bead-id]/progress.txt
    2. cat .bead-progress/[bead-id]/tests.json
    3. git log --oneline -10
    4. Continue from where progress.txt indicates

# ============================================================================
# SECTION 8: COMPLETION CRITERIA
# ============================================================================
# Bead is ONLY complete when ALL of these are true

completion_checklist:
  tests:
    - "[ ] All acceptance tests written and passing"
    - "[ ] All error path tests written and passing"
    - "[ ] All edge case tests written and passing"
    - "[ ] E2E pipeline test passing with real data"
    - "[ ] No mocks or fake data in any test"

  code:
    - "[ ] Implementation uses Result<T, Error> throughout"
    - "[ ] Zero unwrap() or expect() calls"
    - "[ ] All preconditions validated"
    - "[ ] All postconditions guaranteed"
    - "[ ] All invariants maintained"

  ci:
    - "[ ] moon run :ci passes"
    - "[ ] No clippy warnings"
    - "[ ] No compiler warnings"

  documentation:
    - "[ ] Close reason documents what was done"
    - "[ ] Any new CLI flags documented in --help"

# ============================================================================
# SECTION 9: CONTEXT & REFERENCES
# ============================================================================
# Everything the implementer needs to know

context:
  related_files:
    - path: "[file path]"
      relevance: "[Why this file matters]"

  similar_implementations:
    - "[Reference to similar code in codebase]"

  external_references:
    - "[Link to relevant documentation]"

  codebase_patterns:
    - pattern: "[Pattern name]"
      example_location: "[Where to see it used]"
      how_to_apply: "[How to apply here]"

# ============================================================================
# SECTION 10: AI IMPLEMENTATION HINTS (Claude 4.x Optimized)
# ============================================================================
# From Anthropic Claude 4.x Best Practices Guide
# Explicit guidance for AI implementers with model-specific optimizations

ai_hints:
  # Claude 4.x responds well to explicit, clear instructions
  do:
    - "Use functional patterns: map, and_then, ?"
    - "Return Result<T, Error> from all fallible functions"
    - "Use exhaustive pattern matching"
    - "Follow existing code conventions in [file]"
    - "READ files before modifying them (anti-hallucination)"
    - "VERIFY types/functions exist before referencing them"
    - "Use parallel tool calls when tasks are independent"
    - "Update progress.txt after each completed task"
    - "Commit to git after each completed task"

  do_not:
    - "Do NOT use unwrap() or expect()"
    - "Do NOT use panic!, todo!, or unimplemented!"
    - "Do NOT modify clippy configuration"
    - "Do NOT use raw cargo commands (use moon)"
    - "Do NOT use raw git commands (use jj)"
    - "Do NOT speculate about code you haven't read"
    - "Do NOT invent APIs that don't exist"
    - "Do NOT use placeholder values"
    - "Do NOT over-engineer beyond what's specified"

  # Claude 4.x specific: Avoid the word "think" (use alternatives)
  language_guidance:
    avoid: ["think", "thinking"]
    use_instead: ["consider", "evaluate", "analyze", "determine"]

  # Claude 4.x specific: Be explicit about action vs suggestion
  action_guidance: |
    When implementing, TAKE ACTION rather than suggesting changes.
    Don't say "you could change X to Y" - just CHANGE X to Y.
    Be direct: "I will now modify..." not "I could modify..."

  # Claude 4.x specific: Parallel execution guidance
  parallel_execution: |
    When reading multiple files for research, read them ALL in parallel.
    When writing multiple independent tests, write them ALL in parallel.
    Only serialize when there are true dependencies.

  # Claude 4.x specific: Incremental progress
  incremental_progress: |
    Focus on completing ONE task fully before moving to the next.
    After each task: update progress.txt, run tests, commit if passing.
    Don't try to implement everything at once.

  code_patterns:
    - name: "[Pattern]"
      use_when: "[When to use]"
      example: |
        [Code example]

  # Constitutional principles (project-level invariants)
  constitution:
    - "Zero unwrap law: NEVER use .unwrap() or .expect()"
    - "Functional first: Prefer map/and_then over if-else chains"
    - "Moon only: NEVER use raw cargo commands"
    - "JJ only: NEVER use raw git commands"
    - "Test first: Tests MUST exist before implementation"
    - "Real data only: NO mocks, NO fake data in tests"
```

---

## Example: Complete Bead Specification

```yaml
# ============================================================================
# BEAD: intent-cli-EXAMPLE - Add --verbose flag to analyze command
# ============================================================================

id: "intent-cli-a1b2"
title: "analyze: Add --verbose flag for detailed quality breakdown"
type: feature
priority: 2
effort_estimate: "1hr"
labels: [analyze, cli-flags, quality]

# ============================================================================
# SECTION 1: EARS REQUIREMENTS
# ============================================================================

ears_requirements:
  ubiquitous:
    - "THE SYSTEM SHALL return valid JSON for all analyze outputs"
    - "THE SYSTEM SHALL include quality_score in all analyze responses"

  event_driven:
    - trigger: "WHEN user runs 'intent analyze spec.cue --verbose'"
      shall: "THE SYSTEM SHALL include per-behavior quality breakdown"
    - trigger: "WHEN user runs 'intent analyze spec.cue' (no --verbose)"
      shall: "THE SYSTEM SHALL return summary scores only"

  unwanted:
    - condition: "IF spec file does not exist"
      shall_not: "THE SYSTEM SHALL NOT return exit code 0"
      because: "Silent failures hide problems from CI/CD pipelines"
    - condition: "IF --verbose provided with non-JSON output"
      shall_not: "THE SYSTEM SHALL NOT mix text and JSON in stdout"
      because: "Breaks machine parsing"

# ============================================================================
# SECTION 2: KIRK CONTRACTS
# ============================================================================

contracts:
  preconditions:
    auth_required: false
    required_inputs:
      - field: "spec_path"
        type: "String"
        constraints: "Must be valid file path ending in .cue"
        example_valid: "examples/user-api.cue"
        example_invalid: "nonexistent.cue"
      - field: "--verbose"
        type: "Bool"
        constraints: "Optional flag, defaults to false"
        example_valid: "--verbose"
        example_invalid: "--verbose=yes"  # Wrong format
    system_state:
      - "Spec file exists and is valid CUE"

  postconditions:
    state_changes: []  # analyze is read-only
    return_guarantees:
      - field: "success"
        guarantee: "true if analysis completed, false otherwise"
      - field: "data.quality_score"
        guarantee: "Integer 0-100"
      - field: "data.breakdown"
        guarantee: "Present only when --verbose, contains per-behavior scores"
      - field: "exit_code"
        guarantee: "0 on success, 3 on invalid file, 4 on missing file"
    side_effects: []  # None - pure read operation

  invariants:
    - "Output is always valid JSON"
    - "Exit codes match AGENTS.md specification"
    - "No partial/truncated output on any code path"

# ============================================================================
# SECTION 3: INVERSION ANALYSIS
# ============================================================================

inversions:
  usability_failures:
    - failure: "Verbose output is too large to parse"
      prevention: "Limit breakdown to top-level behaviors only"
      test_for_it: "test_verbose_output_size_reasonable"
    - failure: "--verbose without --json produces unusable mixed output"
      prevention: "Always output JSON when --verbose is set"
      test_for_it: "test_verbose_implies_json_format"

  integration_failures:
    - failure: "CI/CD scripts break on new output format"
      prevention: "summary fields unchanged, breakdown is additive only"
      test_for_it: "test_backwards_compatible_with_existing_scripts"

# ============================================================================
# SECTION 4: ATDD ACCEPTANCE TESTS
# ============================================================================

acceptance_tests:
  happy_paths:
    - name: "test_analyze_verbose_includes_breakdown"
      given: "Valid spec file exists at examples/user-api.cue"
      when: "User runs 'gleam run -- analyze examples/user-api.cue --verbose'"
      then:
        - "Exit code is 0"
        - "Response contains 'success': true"
        - "Response contains 'data.breakdown' array"
        - "Each breakdown item has 'behavior' and 'score' fields"
      real_input: |
        gleam run -- analyze examples/user-api.cue --verbose
      expected_output: |
        {
          "success": true,
          "action": "analyze",
          "data": {
            "quality_score": 87,
            "breakdown": [
              {"behavior": "create-user", "score": 95},
              {"behavior": "login", "score": 80}
            ]
          }
        }

    - name: "test_analyze_without_verbose_no_breakdown"
      given: "Valid spec file exists"
      when: "User runs 'gleam run -- analyze examples/user-api.cue'"
      then:
        - "Exit code is 0"
        - "Response contains 'data.quality_score'"
        - "Response does NOT contain 'data.breakdown'"
      real_input: |
        gleam run -- analyze examples/user-api.cue
      expected_output: |
        {
          "success": true,
          "action": "analyze",
          "data": {
            "quality_score": 87
          }
        }

  error_paths:
    - name: "test_analyze_missing_file_exits_4"
      given: "File does not exist"
      when: "User runs 'gleam run -- analyze nonexistent.cue --verbose'"
      then:
        - "Exit code is 4"
        - "Response contains 'success': false"
        - "Response contains error with 'not_found' code"
      real_input: |
        gleam run -- analyze nonexistent.cue --verbose 2>/dev/null; echo "exit:$?"
      expected_error: |
        {"success":false,"errors":[{"code":"not_found","message":"File not found: nonexistent.cue"}]}
        exit:4

  edge_cases:
    - name: "test_verbose_with_empty_spec"
      scenario: "Spec file exists but has no behaviors"
      input: "Empty spec file with only metadata"
      expected: "quality_score: 0, breakdown: []"

# ============================================================================
# SECTION 5: END-TO-END TEST SPECIFICATION
# ============================================================================

e2e_tests:
  pipeline_test:
    name: "test_full_analyze_verbose_pipeline"
    description: "Complete flow: create spec -> analyze verbose -> verify breakdown"

    setup:
      files_to_create:
        - path: "/tmp/test-spec.cue"
          content: |
            spec: {
              name: "Test API"
              features: [{
                name: "Auth"
                behaviors: [{
                  name: "login"
                  intent: "User logs in"
                  request: {method: "POST", path: "/login"}
                  response: {status: 200}
                }]
              }]
            }

    execute:
      command: "gleam run -- analyze /tmp/test-spec.cue --verbose"
      timeout_ms: 10000

    verify:
      exit_code: 0
      stdout_matches_json:
        - path: "success"
          value: true
        - path: "data.quality_score"
          type: "integer"
        - path: "data.breakdown"
          type: "array"
          min_length: 1

    cleanup:
      files_to_delete:
        - "/tmp/test-spec.cue"

# ============================================================================
# SECTION 6: IMPLEMENTATION TASK LIST
# ============================================================================

implementation_tasks:
  phase_1_tests_first:
    - task: "Write test: analyze_verbose_includes_breakdown_test"
      file: "test/integration_e2e_test.gleam"
      what: |
        pub fn analyze_verbose_includes_breakdown_test() {
          let result = execute_cli("gleam run -- analyze examples/user-api.cue --verbose")
          result.exit_code |> should.equal(0)
          result.output |> should.contain("breakdown")
        }
      done_when: "Test exists and FAILS"

    - task: "Write test: analyze_no_verbose_no_breakdown_test"
      file: "test/integration_e2e_test.gleam"
      what: |
        pub fn analyze_no_verbose_no_breakdown_test() {
          let result = execute_cli("gleam run -- analyze examples/user-api.cue")
          result.exit_code |> should.equal(0)
          result.output |> should_not_contain("breakdown")
        }
      done_when: "Test exists and FAILS"

  phase_2_implementation:
    - task: "Add --verbose flag to analyze command parser"
      file: "src/intent.gleam"
      what: "Add 'verbose' flag parsing in analyze_command function"
      patterns_to_use:
        - "case verbose { True -> ... False -> ... }"
      done_when: "Flag is parsed but not yet used"

    - task: "Implement breakdown generation in quality_analyzer"
      file: "src/intent/quality_analyzer.gleam"
      what: "Add analyze_with_breakdown function that returns per-behavior scores"
      patterns_to_use:
        - "list.map to iterate behaviors"
        - "Result<BreakdownReport, Error> return type"
      done_when: "Function exists, tests still fail"

    - task: "Wire verbose flag to breakdown generation"
      file: "src/intent.gleam"
      what: "In analyze command, call analyze_with_breakdown when verbose=True"
      done_when: "All phase_1 tests PASS"

  phase_4_verification:
    - task: "Run moon run :ci"
      done_when: "All tests pass, no warnings"

    - task: "Manual verification"
      commands:
        - "gleam run -- analyze examples/user-api.cue --verbose | jq .data.breakdown"
      expected: "Array of behavior scores"

# ============================================================================
# SECTION 7: FAILURE MODES
# ============================================================================

failure_modes:
  - symptom: "Test fails with 'breakdown not found in output'"
    likely_cause: "Verbose flag not being passed to analyzer"
    where_to_look:
      - file: "src/intent.gleam"
        function: "analyze_command"
        what_to_check: "Is verbose variable being used in the case expression?"
    fix_pattern: "Ensure verbose flag flows through to json_output call"

  - symptom: "Exit code is 0 but output is empty"
    likely_cause: "Early return before JSON output"
    where_to_look:
      - file: "src/intent.gleam"
        function: "analyze_command"
        what_to_check: "Check all code paths reach json_output.output()"
    fix_pattern: "Add explicit json_output call in error branch"

debugging_commands:
  - scenario: "When output format is wrong"
    run: "gleam run -- analyze examples/user-api.cue --verbose 2>&1 | head -20"
    look_for: "Check if there's non-JSON text before the JSON object"

# ============================================================================
# SECTION 8: COMPLETION CRITERIA
# ============================================================================

completion_checklist:
  tests:
    - "[ ] test_analyze_verbose_includes_breakdown passes"
    - "[ ] test_analyze_no_verbose_no_breakdown passes"
    - "[ ] test_analyze_missing_file_exits_4 passes"
    - "[ ] E2E pipeline test passes with real spec file"
    - "[ ] No mocks - all tests use real gleam run commands"

  code:
    - "[ ] No unwrap() or expect() in new code"
    - "[ ] All new functions return Result types"
    - "[ ] Exit codes follow AGENTS.md specification"

  ci:
    - "[ ] moon run :ci passes"
    - "[ ] No clippy warnings"

# ============================================================================
# SECTION 9: CONTEXT
# ============================================================================

context:
  related_files:
    - path: "src/intent.gleam"
      relevance: "Main CLI entry point, where flags are parsed"
    - path: "src/intent/quality_analyzer.gleam"
      relevance: "Quality analysis logic"
    - path: "test/integration_e2e_test.gleam"
      relevance: "Where E2E tests live"

  similar_implementations:
    - "See lint command for flag handling pattern"
    - "See doctor command for breakdown report format"

  codebase_patterns:
    - pattern: "Flag parsing"
      example_location: "src/intent.gleam:1200 (lint command)"
      how_to_apply: "Use case expression on flag boolean"

# ============================================================================
# SECTION 10: AI HINTS
# ============================================================================

ai_hints:
  do:
    - "Use list.map for iterating behaviors"
    - "Use json.encode for building output"
    - "Follow existing quality_analyzer patterns"

  do_not:
    - "Do NOT use unwrap() on Option or Result"
    - "Do NOT add new dependencies"
    - "Do NOT modify existing test files except to add new tests"

  code_patterns:
    - name: "Optional field in JSON"
      use_when: "breakdown should only appear when verbose"
      example: |
        case verbose {
          True -> [#("breakdown", json.array(breakdowns, encode_breakdown))]
          False -> []
        }
        |> list.append(base_fields)
        |> json.object()
```

---

## Quick Reference: The 16 Sections

| Section | Purpose | Must Answer |
|---------|---------|-------------|
| **0. Clarifications** | Anti-assumption gate | All ambiguities resolved? |
| **1. EARS** | What must happen | All 6 patterns covered? |
| **2. KIRK Contracts** | Pre/post/invariants | What's guaranteed? |
| **2.5 Research** | What to investigate | Files/patterns to read? |
| **3. Inversions** | What could go wrong | Every failure mode? |
| **4. ATDD Tests** | Acceptance criteria | Tests before code? |
| **5. E2E Tests** | Full pipeline proof | Real data, no mocks? |
| **5.5 Verification** | Quality gates | All gates defined? |
| **6. Task List** | Implementation steps | Parallel/sequential marked? |
| **7. Failure Modes** | Debugging guide | Where to look? |
| **7.5 Anti-Hallucination** | Ground truth rules | Read-before-write enforced? |
| **7.6 Context Survival** | Long-running support | Progress files defined? |
| **8. Completion** | Definition of done | All boxes checked? |
| **9. Context** | Background info | Related files? |
| **10. AI Hints** | Claude 4.x guidance | Constitution clear? |

---

## The Litmus Tests (Extended)

Before submitting a bead, verify:

### Specification Quality
1. **GPT-4 Test**: Could GPT-4 implement this without asking clarifying questions?
2. **High School Senior Test**: Could a competent CS student implement this?
3. **Clarification Test**: Are ALL ambiguities explicitly resolved (Section 0)?
4. **EARS Coverage Test**: Are all 6 EARS patterns considered?

### Test Quality
5. **90% Coverage Test**: Are there tests for 90%+ of code paths?
6. **No Mocks Test**: Are ALL tests using real data and real commands?
7. **Soup-to-Nuts Test**: Is there an E2E test proving the full pipeline works?
8. **Inversion Test**: Have you defined what must NOT happen?

### Task Quality
9. **30-Minute Task Test**: Is every task completable in under 30 minutes?
10. **Parallelization Test**: Are parallel vs sequential tasks explicitly marked?
11. **Gate Test**: Are verification checkpoints defined between phases?

### AI-Readiness
12. **Research Test**: Are all files-to-read and patterns-to-find specified?
13. **Anti-Hallucination Test**: Are read-before-write rules specified?
14. **Context Survival Test**: Can AI resume from progress.txt if context clears?
15. **API Existence Test**: Are actual APIs listed (not invented ones)?

If any answer is "no", the bead is incomplete.

---

## Creating a Bead from This Template

```bash
# 1. Copy template
cp .beads/BEAD_TEMPLATE.md /tmp/new-bead.md

# 2. Fill in all 10 sections
# (This is the hard part - be thorough!)

# 3. Create the bead
br create "Component: Action description" \
  -t feature \
  -p 2 \
  -d "$(cat /tmp/new-bead.md)"

# 4. Verify with viewer
bv --show <bead-id>
```

---

## Remember

> "Weeks of coding can save you hours of planning."
>
> This template exists because **ambiguous tickets create ambiguous implementations**.
>
> The time spent specifying a bead is always less than the time spent debugging a poorly-specified one.
