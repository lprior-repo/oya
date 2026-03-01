package bead

#EnhancedBead: {
	id: "src-2i6"
	title: "lifecycle: validate dag dependencies and execution order"
	type: "bug"
	priority: 1
	effort_estimate: "2hr"
	labels: ["lifecycle", "dag", "validation", "dependencies"]

	clarifications: {
		clarification_status: "RESOLVED"
		resolved_clarifications: [{ question: "What validation is missing from current DAG validation?", answer: "The current validate_dag() checks unknown dependencies and cycles, but does not validate execution order matches dependency constraints. validate_dependency_order() exists but is called too early before all steps are seen.", decided_by: "code analysis", date: "2026-02-27" }]
		assumptions: [{ assumption: "validate_dependency_order() should check that dependencies execute before dependents", validation_method: "Read current implementation and verify it validates topological order", risk_if_wrong: "If validation is incorrect, steps may execute in wrong order breaking invariants" }]
	}

	ears_requirements: {
		ubiquitous: ["THE SYSTEM SHALL validate all DAG step dependencies before lifecycle execution", "THE SYSTEM SHALL ensure no cycles exist in the dependency graph", "THE SYSTEM SHALL verify execution order respects dependency constraints"]
		event_driven: [
			{ trigger: "WHEN lifecycle steps are defined with dependencies", shall: "THE SYSTEM SHALL validate all referenced dependency steps exist" },
			{ trigger: "WHEN validate_dependency_order() is called on lifecycle steps", shall: "THE SYSTEM SHALL ensure each step dependencies appear before it in the step list" },
			{ trigger: "WHEN a cycle is detected in the dependency graph", shall: "THE SYSTEM SHALL return a terminal LifecycleError with FailureCategory::Validation" },
		]
		unwanted: [
			{ condition: "IF a step depends on a non-existent step", shall_not: "THE SYSTEM SHALL NOT allow lifecycle to proceed", because: "Missing dependencies cause runtime failures" },
			{ condition: "IF dependency order is violated (dependent appears before dependency)", shall_not: "THE SYSTEM SHALL NOT proceed with execution", because: "Executing dependent before dependency breaks execution semantics" },
			{ condition: "IF a cycle exists in the dependency graph", shall_not: "THE SYSTEM SHALL NOT enter infinite loops or deadlock", because: "Cycles make execution impossible" },
		]
	}

	contracts: {
		preconditions: {
			auth_required: false
			required_inputs: [{ field: "steps", type: "Vec<LifecycleStep>", constraints: "Non-empty slice of LifecycleStep with name and dependencies fields", example_valid: "[LifecycleStep { name: \"step1\", dependencies: [] }, LifecycleStep { name: \"step2\", dependencies: [\"step1\"] }]", example_invalid: "[] or steps with circular dependencies" }]
			system_state: ["LifecycleStep type is defined with name: String and dependencies: Vec<String>"]
		}
		postconditions: {
			state_changes: ["No state changes - validate_dag is a pure validation function"]
			return_guarantees: [{ field: "Result::Ok", guarantee: "Returns Ok(()) when DAG is valid with no cycles, all dependencies exist, and order is correct" }, { field: "Result::Err", guarantee: "Returns Err(LifecycleError) with FailureCategory::Validation for any validation failure" }, { field: "error message", guarantee: "Includes step names and specific validation failure reason" }]
		}
		invariants: ["validate_dependency_order must only be called after all steps are processed", "Cycle detection must catch all cycles including self-referential ones", "Error messages must include the specific step and dependency that caused failure"]
	}

	research_requirements: {
		files_to_read: [
			{ path: "src/lifecycle/workflow/dag.rs", what_to_extract: "Current validate_dag, validate_dependency_order, and detect_cycles implementations", document_in: "research_notes.md" },
			{ path: "src/lifecycle/workflow/steps.rs", what_to_extract: "LifecycleStep struct definition", document_in: "research_notes.md" },
			{ path: "src/lifecycle/types/mod.rs", what_to_extract: "LifecycleError and FailureCategory definitions", document_in: "research_notes.md" },
		]
		research_complete_when: ["[x] All files_to_read have been opened and key info extracted", "[ ] All research_questions have answers documented"]
	}

	inversions: {
		usability_failures: [
			{ failure: "Error messages don't clearly indicate which step and dependency caused failure", prevention: "Include both step name and dependency name in all error messages", test_for_it: "test_error_messages_include_step_and_dependency_names" },
			{ failure: "Validation passes for steps that have no dependencies but are in wrong order for other reasons", prevention: "Document that dependency order only matters for actual dependencies, not step sequence", test_for_it: "test_steps_without_dependencies_can_be_in_any_order" },
		]
		data_integrity_failures: [
			{ failure: "Cycle detection misses self-referential step (step depends on itself)", prevention: "Ensure has_cycle checks if recursion_stack contains current step_name before recursing", test_for_it: "test_self_referential_step_detected_as_cycle" },
			{ failure: "Duplicate step names cause incorrect dependency validation", prevention: "Check for duplicate step names in validate_dag before dependency validation", test_for_it: "test_duplicate_step_names_return_validation_error" },
		]
		integration_failures: [{ failure: "Calling validate_dependency_order before all steps are in seen set causes false positives", prevention: "Call validate_dependency_order after iterating through all steps, not during iteration", test_for_it: "test_validate_dependency_order_checks_all_steps" }]
	}

	acceptance_tests: {
		happy_paths: [{ name: "test_valid_dag_with_ordered_dependencies_passes_validation", given: "Steps defined with dependencies appearing before dependents in the list", when: "validate_dag is called on the steps", then: ["Result is Ok(())", "No validation errors are returned"] }, { name: "test_valid_dag_with_complex_dependencies_passes", given: "Steps with multiple dependencies and diamond dependency pattern", when: "validate_dag is called", then: ["Result is Ok(())"] }]
		error_paths: [
			{ name: "test_step_with_unknown_dependency_fails_validation", given: "A step references a dependency that doesn't exist", when: "validate_dag is called", then: ["Result is Err(LifecycleError)", "Error has FailureCategory::Validation", "Error message contains both step name and unknown dependency name"] },
			{ name: "test_dependency_order_violation_fails_validation", given: "A step depends on another step that appears later in the list", when: "validate_dag is called", then: ["Result is Err(LifecycleError)", "Error message indicates dependency appears later"] },
			{ name: "test_cyclic_dependencies_detected", given: "Steps form a cycle (A -> B -> C -> A)", when: "validate_dag is called", then: ["Result is Err(LifecycleError)", "Error message mentions cycle detection"] },
		]
	}

	implementation_tasks: {
		phase_0_research: {
			parallelizable: true
			tasks: [{ task: "Read src/lifecycle/workflow/dag.rs and extract current validation logic", file: "src/lifecycle/workflow/dag.rs", done_when: "Current validate_dag, validate_dependency_order, detect_cycles understood" }]
		}
		phase_1_tests_first: {
			parallelizable: true
			gate_required: "gate_0_research"
			tasks: [{ task: "Write test: test_valid_dag_with_ordered_dependencies_passes_validation", file: "src/lifecycle/workflow/dag_tests.rs", what: "Test for valid DAG with dependencies in correct order", done_when: "Test exists and FAILS (red phase)" }, { task: "Write test: test_step_with_unknown_dependency_fails_validation", file: "src/lifecycle/workflow/dag_tests.rs", what: "Test that unknown dependencies return validation error", done_when: "Test exists and FAILS (red phase)" }]
		}
		phase_2_implementation: {
			parallelizable: false
			gate_required: "gate_1_tests"
			tasks: [
				{ task: "Fix validate_dependency_order logic to check all steps after full iteration", file: "src/lifecycle/workflow/dag.rs", what: "Move validate_dependency_order call to after step collection, or modify logic to check all dependencies exist in steps list", done_when: "Function compiles, tests start passing" },
				{ task: "Ensure cycle detection handles self-referential steps correctly", file: "src/lifecycle/workflow/dag.rs", what: "Verify has_cycle checks if current step is in recursion_stack before recursing", done_when: "Self-referential test passes" },
				{ task: "Improve error messages to include specific step and dependency names", file: "src/lifecycle/workflow/dag.rs", what: "Update all error return statements to format messages with step.name and dep name", done_when: "Error message assertions pass" },
			]
		}
	}

	failure_modes: {
		failure_modes: [
			{ symptom: "Test fails with unexpected Ok result for invalid DAG", likely_cause: "validate_dependency_order not checking dependencies appear before dependents", where_to_look: [{ file: "src/lifecycle/workflow/dag.rs", line_range: "27-41", what_to_check: "Does validate_dependency_order iterate through steps and check each dep is in seen set?" }], fix_pattern: "Ensure validate_dependency_order is called AFTER all steps are processed into seen set" },
			{ symptom: "Self-referential step not detected as cycle", likely_cause: "has_cycle doesn't check if current step_name is already in recursion_stack", where_to_look: [{ file: "src/lifecycle/workflow/dag.rs", function: "has_cycle", what_to_check: "Is there a check for recursion_stack.contains(dep_name) before recursing?" }], fix_pattern: "Add recursion_stack.contains check before recursing on dependencies" },
		]
	}

	anti_hallucination: {
		read_before_write: [{ file: "src/lifecycle/workflow/dag.rs", must_read_first: true, key_sections_to_understand: ["validate_dag function (lines 10-25)", "validate_dependency_order function (lines 27-41)", "detect_cycles function (lines 43-59)", "has_cycle function (lines 61-87)"] }]
		apis_that_exist: [{ api: "validate_dag", signature: "fn validate_dag(steps: &[LifecycleStep]) -> Result<(), LifecycleError>", import_from: "crate::lifecycle::workflow::dag" }, { api: "LifecycleError::terminal", signature: "fn terminal(category: FailureCategory, message: String) -> LifecycleError", import_from: "crate::lifecycle::types" }]
		no_placeholder_values: ["Do NOT use placeholder step names like step_a, step_b - use real lifecycle step names from codebase", "Do NOT use generic error messages - include actual step names from tests"]
		git_verification: { before_claiming_done: "git status  # Verify changes are staged\ngit diff    # Verify changes match specification\nmoon run :test  # Verify all tests pass" }
	}

	completion_checklist: {
		tests: ["[ ] test_valid_dag_with_ordered_dependencies_passes_validation passes", "[ ] test_step_with_unknown_dependency_fails_validation passes", "[ ] test_dependency_order_violation_fails_validation passes", "[ ] test_cyclic_dependencies_detected passes", "[ ] test_self_referential_step_detected passes", "[ ] All existing lifecycle tests still pass"]
		code: ["[ ] No unwrap() or expect() in new code", "[ ] All validation functions return Result types", "[ ] validate_dependency_order correctly validates execution order", "[ ] Error messages include specific step and dependency names"]
		ci: ["[ ] moon run :ci passes", "[ ] No clippy warnings", "[ ] No compiler warnings"]
	}

	ai_hints: {
		do: ["Read current validate_dependency_order implementation carefully before making changes", "Understand why it called at line 24 of validate_dag and if that correct", "Use Result<T, LifecycleError> for all validation functions", "Follow existing error formatting patterns in file", "Use functional patterns: map, and_then, ?", "Update progress.txt after each completed task", "Commit to git after each completed task"]
		do_not: ["Do NOT use unwrap() or expect()", "Do NOT panic!, todo!, or unimplemented!", "Do NOT modify function signatures of existing validation functions", "Do NOT remove cycle detection - it critical", "Do NOT add unnecessary complexity - keep the validation simple and clear"]
	}

	context: {
		related_files: [{ path: "src/lifecycle/workflow/dag.rs", relevance: "Contains validate_dag, validate_dependency_order, and cycle detection logic" }, { path: "src/lifecycle/workflow/steps.rs", relevance: "Defines LifecycleStep struct with name and dependencies fields" }, { path: "src/lifecycle/types/mod.rs", relevance: "Defines LifecycleError and FailureCategory for validation errors" }]
		similar_implementations: ["See how other validation functions in codebase handle error reporting"]
		codebase_patterns: [{ pattern: "Validation error with step names", example_location: "src/lifecycle/workflow/dag.rs:16-19", how_to_apply: "Use format! to include both step.name and dep in error messages" }]
	}
}
