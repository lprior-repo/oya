# ============================================================================
# BEAD: src-wzz - Codify planner + rust-contract + red-queen self-build workflow in AGENTS.md
# ============================================================================

id: "src-wzz"
title: "documentation: Codify planner + rust-contract + red-queen self-build workflow"
type: task
priority: 2
effort_estimate: "30min"
labels: [documentation, agnents-md, self-build, workflow]

# ============================================================================
# SECTION 0: CLARIFICATION MARKERS
# ============================================================================

clarification_status: "RESOLVED"

resolved_clarifications:
  - question: "What is the self-build workflow that needs to be codified?"
    answer: "The workflow defined in AGENTS.md line 16 that uses planner, rust-contract, and red-queen for implementation beads"
    decided_by: "code analysis"
    date: "2026-02-27"

  - question: "What does 'codify' mean in this context?"
    answer: "Add explicit documentation and rules explaining the workflow, similar to other entries in AGENTS.md"
    decided_by: "prior analysis"
    date: "2026-02-27"

open_clarifications: []

assumptions:
  - assumption: "The self-build workflow in line 16 is already implemented but needs better documentation"
    validation_method: "Verify that oya lifecycle uses planner/rust-contract/red-queen"
    risk_if_wrong: "Documentation may describe workflow that doesn't actually exist"
  - assumption: "AGENTS.md is the single source of truth for workflow rules"
    validation_method: "Check that no other workflow documentation exists"
    risk_if_wrong: "Documentation may be split across multiple files"

# ============================================================================
# SECTION 1: EARS REQUIREMENTS
# ============================================================================

ears_requirements:
  ubiquitous:
    - "THE SYSTEM SHALL document the self-build workflow in AGENTS.md"
    - "THE SYSTEM SHALL reference planner, rust-contract, and red-queen skills"
    - "THE SYSTEM SHALL explain the workflow steps clearly"

  event_driven:
    - trigger: "WHEN a developer implements an implementation bead for oya"
      shall: "THE SYSTEM SHALL follow the self-build workflow documented in AGENTS.md"
    - trigger: "WHEN AGENTS.md is updated"
      shall: "THE SYSTEM SHALL maintain JSON format and valid structure"

  unwanted:
    - condition: "IF the self-build workflow is ambiguous or unclear"
      shall_not: "THE SYSTEM SHALL NOT leave developers guessing about the correct process"
      because: "Unclear workflows lead to inconsistent implementation practices"
    - condition: "IF a skill is referenced in the workflow"
      shall_not: "THE SYSTEM SHALL NOT reference skills without their purpose documented"
      because: "Developers need to understand why each skill is used"

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
      - field: "AGENTS.md"
        type: "File path /home/lewis/src/oya/AGENTS.md"
        constraints: "Exists and is valid JSONL format"
        example_valid: "Existing AGENTS.md at /home/lewis/src/oya/AGENTS.md"
        example_invalid: "Non-existent or malformed file"
    system_state:
      - "AGENTS.md exists and is valid JSONL"
      - "planner, rust-contract, red-queen skills are defined"

  postconditions:
    state_changes:
      - "AGENTS.md contains explicit documentation of self-build workflow"
      - "Workflow documentation includes skill purposes and step ordering"
      - "No existing entries in AGENTS.md are modified"
    return_guarantees:
      - field: "documentation quality"
        guarantee: "Self-build workflow is clearly explained and actionable"
      - field: "file validity"
        guarantee: "AGENTS.md remains valid JSONL format"
    side_effects:
      - "AGENTS.md file is modified on disk"

  invariants:
    - "AGENTS.md entries remain in JSONL format (one JSON object per line)"
    - "No existing workflow entries are modified or removed"
    - "All skill references match skill names defined in skill loading rules"

# ============================================================================
# SECTION 2.5: RESEARCH REQUIREMENTS
# ============================================================================

research_requirements:
  files_to_read:
    - path: "AGENTS.md"
      what_to_extract: "Current self-build workflow entry and structure of other workflow entries"
      document_in: "research_notes.md"
    - path: "docs/BEADS.md"
      what_to_extract: "Existing documentation about beads and workflows"
      document_in: "research_notes.md"

  patterns_to_find:
    - pattern: "\"workflow\""
      purpose: "Find all workflow entries in AGENTS.md to understand format"
      expected_locations: "AGENTS.md"

  prior_art:
    - feature: "Existing workflow entries in AGENTS.md"
      location: "AGENTS.md:16"
      what_to_learn: "How workflows are documented - steps format, structure"

  external_docs:
    - url: "file:///home/lewis/.claude/skills/planner/SKILL.md"
      section: "Design Principle"
      extract: "Understanding of planner skill purpose and workflow"

  research_questions:
    - question: "What is the exact format of workflow entries in AGENTS.md?"
      answered: false
      answer: "[To be filled after research]"
    - question: "Are there any existing references to planner, rust-contract, or red-queen outside of line 16?"
      answered: false
      answer: "[To be filled after research]"
    - question: "Should the new documentation add a new entry or expand existing line 16?"
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
    - failure: "Workflow documentation is too abstract and lacks concrete examples"
      prevention: "Include specific command examples and step-by-step guidance"
      test_for_it: "test_workflow_documentation_includes_examples"
    - failure: "Developers don't know when to use self-build vs regular bead workflow"
      prevention: "Explicitly document when self-build workflow applies"
      test_for_it: "test_when_to_use_self_build_documented"

  data_integrity_failures:
    - failure: "AGENTS.md becomes invalid JSONL after update"
      prevention: "Validate JSON format before committing changes"
      test_for_it: "test_agents_md_remains_valid_jsonl"

  integration_failures:
    - failure: "New documentation conflicts with existing skill loading rules"
      prevention: "Ensure new entries align with existing skill definitions"
      test_for_it: "test_workflow_references_known_skills"

# ============================================================================
# SECTION 4: ATDD ACCEPTANCE TESTS
# ============================================================================

acceptance_tests:
  happy_paths:
    - name: "test_self_build_workflow_documented"
      given: "AGENTS.md contains existing self-build workflow entry"
      when: "New documentation is added to AGENTS.md"
      then:
        - "Self-build workflow is clearly explained"
        - "Planner skill purpose is documented"
        - "Rust-contract skill purpose is documented"
        - "Red-queen skill purpose is documented"
        - "Workflow steps are in order"
      real_input: |
        {"kind":"guide","id":"self-build-workflow","text":"..."}
      expected_output: "AGENTS.md with comprehensive self-build workflow documentation"

    - name: "test_agents_md_remains_valid_jsonl"
      given: "AGENTS.md is valid JSONL"
      when: "Documentation is added"
      then:
        - "File is valid JSONL"
        - "All lines parse as valid JSON"
      real_input: |
        cat AGENTS.md | while read line; do
            echo "$line" | jq . > /dev/null 2>&1 || exit 1
        done
      expected_output: "All lines parse successfully"

  error_paths: []

  edge_cases:
    - name: "test_documentation_preserves_existing_entries"
      scenario: "Adding documentation should not modify existing entries"
      input: "Read AGENTS.md, add new line, verify previous lines unchanged"
      expected: "All existing entries remain identical"

  contract_tests:
    - name: "test_precondition_agents_md_exists"
      verifies: "AGENTS.md file exists and is readable"
      test: "Open AGENTS.md, verify it's a valid file"

    - name: "test_postcondition_format_maintained"
      verifies: "JSONL format is maintained"
      test: "Parse each line as JSON, verify all succeed"

    - name: "test_invariant_no_existing_entries_modified"
      verifies: "Existing entries are unchanged"
      test: "Compare AGENTS.md before and after, verify only new lines added"

# ============================================================================
# SECTION 5: END-TO-END TEST SPECIFICATION
# ============================================================================

e2e_tests:
  pipeline_test:
    name: "test_full_documentation_update"
    description: "Complete flow: read AGENTS.md -> add documentation -> validate -> commit"

    setup:
      files_to_create: []
      environment: []
      precondition_commands:
        - "cd /home/lewis/src/oya && git diff AGENTS.md > /tmp/original_agents.md"

    execute:
      command: "Edit AGENTS.md to add self-build workflow documentation"
      timeout_ms: 5000

    verify:
      exit_code: 0
      stdout_contains: []
      files_not_modified:
        - "/tmp/original_agents.md (should not be modified)"
      files_created:
        - path: "/home/lewis/src/oya/AGENTS.md"
          contains: "self-build workflow"
          contains: "planner"
          contains: "rust-contract"
          contains: "red-queen"

    cleanup:
      files_to_delete: ["/tmp/original_agents.md"]

  e2e_scenarios: []

# ============================================================================
# SECTION 5.5: VERIFICATION CHECKPOINTS
# ============================================================================

verification_checkpoints:
  gate_0_research:
    name: "Research Gate"
    must_pass_before: "Writing documentation"
    checks:
      - "[ ] All research_requirements files have been read"
      - "[ ] All research_questions have documented answers"
      - "[ ] AGENTS.md format and structure understood"
    evidence_required:
      - "Research notes documenting AGENTS.md structure"
      - "Understanding of current self-build workflow entry"

  gate_1_tests:
    name: "Test Gate"
    must_pass_before: "Writing documentation code"
    checks:
      - "[ ] All acceptance tests written"
      - "[ ] Tests validate JSONL format"
      - "[ ] Tests verify content is added correctly"
    evidence_required:
      - "Tests exist in tests/agents_md_tests.rs or shell script"
      - "Tests verify documentation structure"

  gate_2_implementation:
    name: "Implementation Gate"
    must_pass_before: "Declaring task complete"
    checks:
      - "[ ] All tests pass"
      - "[ ] AGENTS.md updated with comprehensive documentation"
      - "[ ] JSONL format validated"
    evidence_required:
      - "Test output showing all pass"
      - "jq validation of AGENTS.md succeeds"

  gate_3_integration:
    name: "Integration Gate"
    must_pass_before: "Closing bead"
    checks:
      - "[ ] AGENTS.md is valid JSONL"
      - "[ ] Documentation follows existing AGENTS.md style"
      - "[ ] No conflicts with existing entries"
      - "[ ] Manual verification complete"
    evidence_required:
      - "jq validation output"
      - "Manual review notes"

  tests_json:
    format: |
      {
        "tests": [
          {"id": 1, "name": "test_self_build_workflow_documented", "status": "not_started|failing|passing"},
          {"id": 2, "name": "test_agents_md_remains_valid_jsonl", "status": "not_started|failing|passing"}
        ],
        "total": 2,
        "passing": 0,
        "failing": 0,
        "not_started": 2
      }
    location: ".bead-progress/src-wzz/tests.json"

# ============================================================================
# SECTION 6: IMPLEMENTATION TASK LIST
# ============================================================================

implementation_tasks:
  phase_0_research:
    parallelizable: true
    tasks:
      - task: "Read AGENTS.md and extract current self-build workflow entry"
        parallel_group: "research"
        file: "AGENTS.md"
        done_when: "Current line 16 entry documented and understood"

      - task: "Understand workflow entry format in AGENTS.md"
        parallel_group: "research"
        file: "AGENTS.md"
        done_when: "Format of workflow entries (kind, id, text, steps) understood"

  phase_1_tests_first:
    parallelizable: true
    gate_required: "gate_0_research"
    tasks:
      - task: "Write test: test_agents_md_remains_valid_jsonl"
        parallel_group: "tests"
        file: "tests/agents_md_tests.rs or scripts/validate_agents.sh"
        what: "Shell script or test that validates each line of AGENTS.md is valid JSON"
        done_when: "Test exists and PASSES (green phase - AGENTS.md currently valid)"

      - task: "Write test: test_self_build_workflow_documented"
        parallel_group: "tests"
        file: "tests/agents_md_tests.rs or scripts/check_self_build_docs.sh"
        what: "Check that self-build workflow documentation exists and contains required keywords"
        done_when: "Test exists and FAILS (red phase - documentation not yet added)"

  phase_2_implementation:
    parallelizable: false
    gate_required: "gate_1_tests"
    tasks:
      - task: "Add comprehensive self-build workflow documentation to AGENTS.md"
        depends_on: null
        file: "AGENTS.md"
        what: "Add new guide entry explaining self-build workflow with planner, rust-contract, red-queen"
        patterns_to_use:
          - "JSONL format (one JSON object per line)"
          - "Kind: guide for documentation entries"
        done_when: "AGENTS.md contains new documentation entry"

      - task: "Validate AGENTS.md remains valid JSONL"
        depends_on: "Add comprehensive self-build workflow documentation"
        file: "AGENTS.md"
        what: "Run jq validation on each line to ensure format is correct"
        patterns_to_use:
          - "cat AGENTS.md | while read line; do echo \"$line\" | jq .; done"
        done_when: "All lines parse as valid JSON"

  phase_3_integration:
    parallelizable: false
    gate_required: "gate_2_implementation"
    tasks:
      - task: "Manual review of documentation clarity"
        file: "AGENTS.md"
        what: "Read new documentation entry, verify it's clear and actionable"
        done_when: "Documentation is clear and follows AGENTS.md style"

      - task: "Run validation test suite"
        commands:
          - "moon run :test agents_md"
          - "or bash scripts/validate_agents.sh"
        expected: "All tests pass"

  phase_4_verification:
    parallelizable: true
    gate_required: "gate_3_integration"
    tasks:
      - task: "Run moon run :ci"
        parallel_group: "verification"
        done_when: "All tests pass, no clippy warnings"

      - task: "Verify AGENTS.md with jq"
        parallel_group: "verification"
        commands:
          - "cat /home/lewis/src/oya/AGENTS.md | jq -s 'length' | xargs -I {} echo 'Parsed {} lines'"
        expected: "All lines parse successfully"

  parallelization_rules:
    - "Tasks in same parallel_group CAN run simultaneously"
    - "Tasks with depends_on MUST wait for dependency"
    - "Gates MUST pass before next phase begins"
    - "When parallelizable: false, execute in listed order"

# ============================================================================
# SECTION 7: FAILURE MODES
# ============================================================================

failure_modes:
  - symptom: "jq validation fails on AGENTS.md"
    likely_cause: "Invalid JSON format in one of the lines"
    where_to_look:
      - file: "AGENTS.md"
        what_to_check: "Are there syntax errors, missing quotes, or malformed JSON?"
    fix_pattern: "Fix JSON syntax errors, ensure proper quoting and commas"

  - symptom: "Documentation test fails saying 'workflow not found'"
    likely_cause: "New documentation entry not added or has wrong format"
    where_to_look:
      - file: "AGENTS.md"
        what_to_check: "Does the new entry have kind: guide and appropriate id?"
    fix_pattern: "Add or fix the documentation entry with correct JSON structure"

debugging_commands:
  - scenario: "When jq validation fails"
    run: "cat AGENTS.md | jq -C 2>&1 | head -50"
    look_for: "Parse error messages indicating which line is invalid"

  - scenario: "When tests fail"
    run: "moon run :test agents_md -- --nocapture"
    look_for: "Specific test failure messages"

# ============================================================================
# SECTION 7.5: ANTI-HALLUCINATION RULES
# ============================================================================

anti_hallucination:
  read_before_write:
    - file: "AGENTS.md"
      must_read_first: true
      key_sections_to_understand:
        - "Line 16: existing self-build workflow entry"
        - "Format of workflow entries (kind, steps)"
        - "Format of skill loading rules"

  verify_before_reference:
    - type: "AGENTS.md workflow format"
      expected_location: "AGENTS.md"
      verify_command: "cat AGENTS.md | head -20"

  apis_that_exist:
    - api: "jq command-line tool"
      signature: "jq [expression] [file]"
      import_from: "System shell"

  apis_that_do_not_exist:
    - "Any custom validation tool - use jq"

  no_placeholder_values:
    - "Do NOT use placeholder skill names - use actual skill names: planner, rust-contract, red-queen"
    - "Do NOT use generic workflow descriptions - use specific step names from actual workflow"

  git_verification:
    before_claiming_done: |
      git status  # Verify changes are staged
      git diff AGENTS.md    # Verify changes match specification
      cat AGENTS.md | while read line; do echo "$line" | jq . > /dev/null 2>&1 || exit 1; done && echo "All lines valid"

# ============================================================================
# SECTION 7.6: CONTEXT WINDOW SURVIVAL
# ============================================================================

context_survival:
  progress_file:
    path: ".bead-progress/src-wzz/progress.txt"
    format: |
      # Bead: src-wzz - Codify planner + rust-contract + red-queen self-build workflow
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
    path: ".bead-progress/src-wzz/tests.json"
    update_frequency: "After each test run"

  research_notes_file:
    path: ".bead-progress/src-wzz/research.md"
    contains:
      - "AGENTS.md current structure"
      - "Existing self-build workflow entry"
      - "Format requirements for new documentation"

  git_checkpoints:
    frequency: "After each completed task"
    message_format: "[src-wzz] checkpoint: [task completed]"
    purpose: "Allow rollback if next step fails"

  recovery_instructions: |
    If context window is cleared, start new session with:
    1. cat .bead-progress/src-wzz/progress.txt
    2. cat .bead-progress/src-wzz/tests.json
    3. cat .bead-progress/src-wzz/research.md
    4. git log --oneline -10
    5. Continue from where progress.txt indicates

# ============================================================================
# SECTION 8: COMPLETION CRITERIA
# ============================================================================

completion_checklist:
  tests:
    - "[ ] test_self_build_workflow_documented passes"
    - "[ ] test_agents_md_remains_valid_jsonl passes"

  code:
    - "[ ] AGENTS.md updated with comprehensive self-build workflow documentation"
    - "[ ] Documentation includes planner skill purpose"
    - "[ ] Documentation includes rust-contract skill purpose"
    - "[ ] Documentation includes red-queen skill purpose"
    - "[ ] Documentation includes workflow steps in order"

  ci:
    - "[ ] AGENTS.md is valid JSONL (jq validation passes)"
    - "[ ] No clippy warnings (if any Rust tests added)"

# ============================================================================
# SECTION 9: CONTEXT
# ============================================================================

context:
  related_files:
    - path: "AGENTS.md"
      relevance: "Target file for documentation updates"
    - path: "docs/BEADS.md"
      relevance: "Related documentation about beads and workflows"

  similar_implementations:
    - "Existing workflow entries in AGENTS.md for format reference"

  codebase_patterns:
    - pattern: "AGENTS.md JSONL format"
      example_location: "AGENTS.md:1-46"
      how_to_apply: "Each line is a complete JSON object with kind, id, and text fields"

# ============================================================================
# SECTION 10: AI HINTS
# ============================================================================

ai_hints:
  do:
    - "Read AGENTS.md carefully before making any changes"
    - "Follow the existing JSONL format exactly"
    - "Make documentation clear and actionable for developers"
    - "Include skill purposes and workflow steps"
    - "Update progress.txt after each completed task"
    - "Commit to git after each completed task"

  do_not:
    - "Do NOT modify existing entries in AGENTS.md"
    - "Do NOT break JSONL format"
    - "Do NOT use placeholder or vague descriptions"
    - "Do NOT add entries for skills that don't exist"

  language_guidance:
    avoid: ["think", "thinking"]
    use_instead: ["consider", "evaluate", "analyze", "determine"]

  action_guidance: |
    When implementing, TAKE ACTION rather than suggesting changes.
    Don't say "you could add X" - just ADD X.
    Be direct: "I will now add..." not "I could add..."

  parallel_execution: |
    When reading multiple files for research, read them ALL in parallel.
    Only serialize when there are true dependencies (e.g., validate after editing).

  incremental_progress: |
    Focus on completing ONE task fully before moving to the next.
    After each task: update progress.txt, run tests, commit if passing.
    Don't try to implement everything at once.

  code_patterns:
    - name: "AGENTS.md JSONL line format"
      use_when: "Adding new entries to AGENTS.md"
      example: |
        {"kind":"guide","id":"self-build-workflow","text":"Complete description of the workflow"}

  constitution:
    - "JSONL format: Each line must be valid JSON"
    - "No modifications to existing entries"
    - "Clear and actionable documentation"
