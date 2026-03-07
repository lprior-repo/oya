package bead

#EnhancedBead: {
	id: "src-1ji"
	title: "test: Verify lifecycle fix with comprehensive test coverage"
	type: "task"
	priority: 2
	effort_estimate: "1hr"
	labels: ["testing", "lifecycle", "regression-test", "e2e"]

	clarifications: {
		clarification_status: "HAS_OPEN_QUESTIONS"
		resolved_clarifications: [{ question: "What is 'lifecycle fix' that needs testing?", answer: "The fix is related to get_lifecycle returning terminal not-found status for unknown keys (src-1hu)", decided_by: "assumption based on bead name", date: "2026-02-27" }]
		open_clarifications: [
			{
				question: "[NEEDS CLARIFICATION: Does this bead test only src-1hu fix or are there other recent lifecycle changes to test?]",
				context: "Bead name suggests testing a specific fix",
				options: ["Test only src-1hu (get_lifecycle not-found status)", "Test all recent lifecycle changes (review git history for scope)"],
				default_if_unresolved: "Test only src-1hu fix and validate no regressions in general lifecycle",
			},
			{
				question: "[NEEDS CLARIFICATION: What level of test coverage is expected - unit tests, integration tests, or E2E?]",
				context: "Test bead should define scope of testing approach",
				options: ["Unit tests only (fast, isolated)", "Integration tests (test handlers and workflow together)", "Full E2E tests (run actual lifecycle via API)"],
				default_if_unresolved: "All three levels: unit + integration + E2E",
			},
		]
		assumptions: [{
			assumption: "The fix being tested is src-1hu (get_lifecycle not-found status)",
			validation_method: "Verify src-1hu bead exists and relates to lifecycle",
			risk_if_wrong: "Testing wrong fix",
		}]
	}

	ears_requirements: {
		ubiquitous: ["THE SYSTEM SHALL have comprehensive test coverage for lifecycle fix", "THE SYSTEM SHALL verify the fix works correctly", "THE SYSTEM SHALL verify no regressions in existing lifecycle functionality"]
		event_driven: [{ trigger: "WHEN src-1hu fix is implemented", shall: "THE SYSTEM SHALL have tests that verify not-found status is returned" }, { trigger: "WHEN test suite is run", shall: "THE SYSTEM SHALL execute all lifecycle tests and report results" }]
		unwanted: [
			{ condition: "IF lifecycle fix is not properly tested", shall_not: "THE SYSTEM SHALL NOT allow PR merge with only the fix", because: "Untested fixes can cause regressions and bugs in production" },
			{ condition: "IF tests are incomplete or missing", shall_not: "THE SYSTEM SHALL NOT consider the bead complete", because: "Incomplete test coverage masks edge cases and failures" },
		]
	}

	contracts: {
		preconditions: {
			auth_required: false
			required_inputs: [
				{ field: "src-1hu fix", type: "Implemented code", constraints: "get_lifecycle returns TerminalError for unknown keys", example_valid: "Implementation exists and compiles", example_invalid: "Fix not yet implemented" }
			]
			system_state: ["Oya runtime is available (oya init)", "Lifecycle tests compile"]
		}
		postconditions: {
			state_changes: ["No state changes - this is a test-only bead"]
			return_guarantees: [{ field: "test coverage", guarantee: "Tests cover fix, happy paths, error paths, and regressions" }, { field: "test results", guarantee: "All tests pass, no regressions detected" }]
			side_effects: ["Test suite is executed", "Test results are reported"]
		}
		invariants: ["Tests are isolated (no dependencies on external services unless E2E)", "Tests verify both positive (fix works) and negative (no regressions) cases", "Test execution is repeatable and deterministic"]
	}

	research_requirements: {
		files_to_read: [
			{ path: ".beads/src-1hu.md", what_to_extract: "What the lifecycle fix does and what it should verify", document_in: "research_notes.md" },
			{ path: "src/lifecycle/workflow/mod.rs", what_to_extract: "Existing lifecycle test patterns and coverage", document_in: "research_notes.md" },
			{ path: "src/restate_oya/handlers_tests.rs", what_to_extract: "Existing handler test patterns", document_in: "research_notes.md" },
		]
		patterns_to_find: [{ pattern: "#\\[test\\].*lifecycle", purpose: "Find existing lifecycle tests to understand coverage", expected_locations: "src/lifecycle/, src/restate_oya/" }]
		research_questions: [
			{ question: "What specific behavior does src-1hu fix change?", answered: false },
			{ question: "What existing lifecycle tests should be run to check for regressions?", answered: false },
			{ question: "Are there existing E2E tests for lifecycle that should still pass?", answered: false },
		]
		research_complete_when: ["[x] All files_to_read have been opened and key info extracted", "[ ] All patterns_to_find have been searched", "[ ] All research_questions have answers documented"]
	}

	inversions: {
		usability_failures: [
			{ failure: "Tests pass but don't actually verify the fix", prevention: "Ensure tests explicitly check for TerminalError and not-found message", test_for_it: "test_fix_actually_verifies_terminal_error" },
			{ failure: "Tests don't cover important edge cases", prevention: "Review test coverage for all error paths and edge cases", test_for_it: "test_coverage_includes_all_paths" },
		]
		data_integrity_failures: [{ failure: "Tests rely on external state that may be inconsistent", prevention: "Make tests deterministic and isolated, use mocks where appropriate for non-E2E", test_for_it: "test_isolation_and_determinism" }]
		integration_failures: [{ failure: "New tests break existing test suite", prevention: "Run full test suite to verify no conflicts", test_for_it: "test_full_suite_runs_without_conflicts" }]
	}

	acceptance_tests: {
		happy_paths: [
			{
				name: "test_get_lifecycle_unknown_key_returns_terminal_error",
				given: "src-1hu fix is implemented",
				when: "Test suite is run",
				then: ["test_unknown_key_returns_terminal_error passes", "Error is TerminalError type", "Error message contains 'not_found' and key"]
				real_input: "moon run :test test_unknown_key_returns_terminal_error"
				expected_output: "test result: ok",
			},
			{
				name: "test_existing_lifecycle_still_works",
				given: "A valid lifecycle exists",
				when: "get_lifecycle is called with valid key",
				then: ["Returns Ok(LifecycleStatusSnapshot)", "Snapshot contains valid step data"]
				real_input: "moon run :test test_existing_key_returns_valid_status"
				expected_output: "test result: ok",
			},
			{ name: "test_full_lifecycle_suite_passes", given: "All lifecycle tests exist", when: "Complete test suite is run", then: ["All lifecycle tests pass", "No regressions detected"], real_input: "moon run :test lifecycle restate_oya", expected_output: "all tests passed" },
		]
		error_paths: [{
				name: "test_fix_is_actually_tested",
				given: "src-1hu fix exists",
				when: "Test suite is reviewed",
				then: ["Tests explicitly verify TerminalError is returned", "Tests check error message contains 'not_found'"]
				real_input: "grep -A 5 \"test_unknown_key_returns_terminal_error\" src/restate_oya/handlers_tests.rs",
				expected_output: "Test code that verifies TerminalError and message",
			}]
	}

	verification_checkpoints: {
		gate_0_research: {
			name: "Research Gate"
			must_pass_before: "Running tests"
			checks: ["[ ] All research_requirements files have been read", "[ ] All research_questions have documented answers", "[ ] src-1hu fix scope understood", "[ ] Existing test coverage documented"]
			evidence_required: ["Research notes documenting src-1hu fix", "List of existing lifecycle tests", "Understanding of what needs to be tested"]
		}
		gate_1_tests: {
			name: "Test Gate"
			must_pass_before: "Declaring task complete"
			checks: ["[ ] Fix verification tests exist", "[ ] Regression tests run and pass", "[ ] E2E tests run and pass", "[ ] Test coverage is comprehensive"]
			evidence_required: ["Test execution output showing all pass", "Coverage report (if available)", "E2E test output"]
		}
		gate_2_implementation: {
			name: "Implementation Gate"
			must_pass_before: "Declaring task complete"
			checks: ["[ ] All new tests pass", "[ ] All existing tests pass", "[ ] moon run :ci passes"]
			evidence_required: ["Test output showing all pass", "CI output showing green"]
		}
	}

	implementation_tasks: {
		phase_0_research: {
			parallelizable: true
			tasks: [
				{ task: "Read .beads/src-1hu.md to understand fix scope", file: ".beads/src-1hu.md", done_when: "Fix behavior and expected test coverage documented" },
				{ task: "Find existing lifecycle tests", file: "src/lifecycle/, src/restate_oya/", done_when: "List of existing tests documented" },
			]
		}
		phase_1_tests_first: {
			parallelizable: true
			gate_required: "gate_0_research"
			tasks: [
				{ task: "Verify fix tests exist from src-1hu", file: "src/restate_oya/handlers_tests.rs", what: "Check that tests from src-1hu bead are present", done_when: "Tests exist or documented as missing" }
				{ task: "Run fix-specific tests", commands: ["moon run :test test_unknown_key_returns_terminal_error test_unknown_key_does_not_return_ok"], expected: "All fix tests pass", done_when: "Tests pass or failures documented" }
				{ task: "Run full lifecycle test suite", commands: ["moon run :test lifecycle restate_oya"], expected: "All existing tests pass", done_when: "Test suite runs, results documented" },
			]
		}
		phase_2_implementation: {
			parallelizable: false
			gate_required: "gate_1_tests"
			tasks: [{ task: "Write test report documenting fix verification", file: ".bead-progress/src-1ji/test_report.md", what: "Document which tests pass, which fail (if any), and coverage assessment", done_when: "Test report created" }]
		}
		phase_3_integration: {
			parallelizable: false
			gate_required: "gate_2_implementation"
			tasks: [
				{ task: "Run E2E API test", commands: ["oya init", "curl -s http://localhost:909/OyaService/get_lifecycle -X POST -H 'Content-Type: application/json' -d '{\"key\":\"test-e2e-unknown\"}'"], expected: "Response with not_found error", done_when: "E2E test completes" }
				{ task: "Run existing handler tests", commands: ["moon run :test restate_oya"], expected: "All existing handler tests pass", done_when: "All existing tests pass" },
			]
		}
		phase_4_verification: {
			parallelizable: true
			gate_required: "gate_3_integration"
			tasks: [{ task: "Run moon run :ci", done_when: "All tests pass, no clippy warnings" }, { task: "Review test report for coverage", commands: ["cat .bead-progress/src-1ji/test_report.md"], expected: "Comprehensive test coverage documented" }]
		}
	}

	failure_modes: {
		failure_modes: [
			{ symptom: "Fix tests don't exist", likely_cause: "src-1hu bead wasn't implemented or tests were not written", where_to_look: [{ file: "src/restate_oya/handlers_tests.rs", what_to_check: "Do tests with names from src-1hu exist?" }], fix_pattern: "Implement tests per src-1hu bead or document that tests exist elsewhere" },
			{ symptom: "Existing lifecycle tests fail after fix", likely_cause: "Fix introduced regression in lifecycle logic", where_to_look: [{ file: "src/restate_oya/handlers.rs", function: "get_lifecycle", what_to_check: "Did fix change behavior for valid keys?" }], fix_pattern: "Review fix logic, ensure it only affects unknown key paths" },
		]
	}

	anti_hallucination: {
		read_before_write: [{ file: ".beads/src-1hu.md", must_read_first: true, key_sections_to_understand: ["Section 1: EARS Requirements", "Section 4: ATDD Tests", "Expected behavior of the fix"] }]
		apis_that_exist: [{ api: "moon run :test", signature: "moon run :test [test_names]", import_from: "moon CLI" }]
		apis_that_do_not_exist: ["Any custom test runner - use moon only"]
		no_placeholder_values: ["Do NOT use placeholder test names - use actual test names from codebase", "Do NOT assume test results without running them"]
		git_verification: { before_claiming_done: "git status  # Verify no code changes (test-only bead)\ncat .bead-progress/src-1ji/test_report.md  # Verify test report is complete\nmoon run :test  # Verify all tests pass" }
	}

	context_survival: {
		progress_file: { path: ".bead-progress/src-1ji/progress.txt", format: "# Bead: src-1ji - Test lifecycle fix\n# Started: [timestamp]\n# Last updated: [timestamp]\n\n## Current Phase\n[phase_name]\n\n## Completed Tasks\n- [x] [task 1]\n\n## Current Task\n- [ ] [current task] (IN PROGRESS)\n    - [sub-step completed]\n    - [sub-step in progress]\n\n## Test Results Summary\n- [Fix Tests]: [pass/fail]\n- [Existing Tests]: [pass/fail]\n- [E2E Tests]: [pass/fail]\n- [Coverage]: [assessment]\n\n## Next Steps (if context clears)\n1. Read this file\n2. Review test report\n3. Continue from \"Current Task\"" }
		tests_status_file: { path: ".bead-progress/src-1ji/tests.json", update_frequency: "After each test run" }
		research_notes_file: { path: ".bead-progress/src-1ji/research.md", contains: ["src-1hu fix scope and expected behavior", "List of existing lifecycle tests", "Test coverage assessment"] }
		recovery_instructions: "If context window is cleared, start new session with:\n1. cat .bead-progress/src-1ji/progress.txt\n2. cat .bead-progress/src-1ji/tests.json\n3. cat .bead-progress/src-1ji/research.md\n4. git log --oneline -10\n5. Continue from where progress.txt indicates"
	}

	completion_checklist: {
		tests: ["[ ] Fix-specific tests pass (from src-1hu)", "[ ] Existing lifecycle tests pass (no regressions)", "[ ] E2E API test passes", "[ ] Test report documents coverage and results"]
		code: ["[ ] No code changes (test-only bead)", "[ ] Test report is comprehensive", "[ ] Any gaps in coverage are documented"]
		ci: ["[ ] moon run :test passes for lifecycle and restate_oya", "[ ] moon run :ci passes (if applicable)"]
	}

	context: {
		related_files: [{ path: ".beads/src-1hu.md", relevance: "Fix being tested" }, { path: "src/restate_oya/handlers_tests.rs", relevance: "Tests for the fix" }, { path: "src/lifecycle/", relevance: "Existing lifecycle tests to check for regressions" }]
		similar_implementations: ["Existing test patterns in src/lifecycle and src/restate_oya"]
		codebase_patterns: [{ pattern: "Test naming and structure", example_location: "src/restate_oya/handlers_tests.rs", how_to_apply: "Follow existing naming and structure conventions" }]
	}

	ai_hints: {
		do: ["Read src-1hu bead to understand fix scope", "Run all fix-specific tests and document results", "Run full lifecycle test suite to check for regressions", "Run E2E tests to verify end-to-end behavior", "Document test results comprehensively in test_report.md", "Update progress.txt after each completed task"]
		do_not: ["Do NOT modify production code (test-only bead)", "Do NOT skip existing tests - run full suite", "Do NOT assume test results without running them", "Do NOT make code changes to fix issues found (create separate bead)"]
		code_patterns: [{ name: "Test report format", use_when: "Documenting test results", example: "# Test Report: src-1ji\n  ## Fix Tests\n  - test_unknown_key_returns_terminal_error: PASS\n  ## Regression Tests\n  - test_existing_lifecycle_runs: PASS\n  ## E2E Tests\n  - API not_found test: PASS" }]
		constitution: ["No code changes: This is a test-only bead", "Comprehensive testing: Run fix tests + regressions + E2E", "Document everything: Test results, coverage, issues found"]
	}
}
