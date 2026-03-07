package bead

#EnhancedBead: {
	id: "src-sy5"
	title: "observability: add durable step-level telemetry and unwind signals"
	type: "feature"
	priority: 1
	effort_estimate: "4hr"
	labels: ["observability", "telemetry", "opentelemetry", "lifecycle", "compensation"]

	clarifications: {
		clarification_status: "HAS_OPEN_QUESTIONS"
		resolved_clarifications: [{ question: "What telemetry should be captured for each lifecycle step?", answer: "Step name, status, duration, started_at, finished_at, error details, and compensation signals", decided_by: "prior research", date: "2026-02-27" }]
		open_clarifications: [{ question: "[NEEDS CLARIFICATION: Should telemetry be emitted via OpenTelemetry directly or logged and collected by OpenObserve?]", context: "AGENTS.md specifies OTEL_EXPORTER_OTLP_ENDPOINT for OpenObserve integration", options: ["Direct OTEL API calls (more control, more complex)", "Structured logging collected by OpenObserve (simpler, existing integration)"], default_if_unresolved: "Use structured logging with OpenObserve collection (existing pattern)" }]
		assumptions: [{ assumption: "OTEL service name oya-orchestrator is configured per AGENTS.md", validation_method: "Check OTEL_SERVICE_NAME environment variable usage in codebase", risk_if_wrong: "Telemetry wont be associated with correct service in OpenObserve" }, { assumption: "OpenTelemetry endpoint is at http://localhost:4318 per AGENTS.md", validation_method: "Verify OTEL_EXPORTER_OTLP_ENDPOINT usage", risk_if_wrong: "Telemetry wont be sent to OpenObserve" }]
	}

	ears_requirements: {
		ubiquitous: ["THE SYSTEM SHALL emit telemetry for every lifecycle step execution", "THE SYSTEM SHALL capture step name, status, and duration for each step", "THE SYSTEM SHALL send telemetry to OpenObserve via OTEL endpoint"]
		event_driven: [{ trigger: "WHEN a lifecycle step starts", shall: "THE SYSTEM SHALL emit a step_started telemetry event with step name and timestamp" }, { trigger: "WHEN a lifecycle step completes successfully", shall: "THE SYSTEM SHALL emit a step_completed telemetry event with duration" }, { trigger: "WHEN a lifecycle step fails", shall: "THE SYSTEM SHALL emit a step_failed telemetry event with error details" }, { trigger: "WHEN compensation for a step begins", shall: "THE SYSTEM SHALL emit an unwind_started telemetry signal" }, { trigger: "WHEN compensation for a step completes", shall: "THE SYSTEM SHALL emit an unwind_completed telemetry signal" }]
		state_driven: [{ state: "WHILE lifecycle execution is in progress", shall: "THE SYSTEM SHALL aggregate step-level telemetry into lifecycle-span context" }]
		unwanted: [{ condition: "IF a telemetry emission fails", shall_not: "THE SYSTEM SHALL NOT fail the entire lifecycle execution", because: "Telemetry failures should not break actual work" }, { condition: "IF step execution is very fast (< 1ms)", shall_not: "THE SYSTEM SHALL NOT lose precision in duration measurements", because: "Fast steps are still important to measure" }]
	}

	contracts: {
		preconditions: {
			auth_required: false
			required_inputs: [{ field: "step_name", type: "String", constraints: "Non-empty string identifying lifecycle step", example_valid: "moon_ci", example_invalid: "" }]
			system_state: ["OTEL_EXPORTER_OTLP_ENDPOINT environment variable may be configured", "OpenObserve service may be running (optional - telemetry degrades gracefully if unavailable)"]
		}
		postconditions: {
			state_changes: ["Telemetry data sent to OpenObserve (if endpoint configured and available)", "Step execution duration recorded"]
			return_guarantees: [{ field: "telemetry emission", guarantee: "Always returns Ok(()) even if OpenObserve is unavailable" }, { field: "duration measurement", guarantee: "Duration in milliseconds, precision to at least 1ms" }]
			side_effects: ["Network call to OTEL endpoint (if configured)", "Optional log output to stdout (if RUST_LOG=debug)"]
		}
		invariants: ["Telemetry emission never panics - always degrades gracefully", "Duration is measured as finished_at - started_at in milliseconds", "Step names in telemetry match step names in lifecycle definitions", "Unwind/compensation signals are emitted in reverse order of step execution"]
	}

	research_requirements: {
		files_to_read: [{ path: "src/lifecycle/workflow/mod.rs", what_to_extract: "How run_lifecycle executes steps and tracks progress", document_in: "research_notes.md" }, { path: "src/lifecycle/types/mod.rs", what_to_extract: "LifecycleStep, LifecycleStepStatus, and CompensationDiagnostic types", document_in: "research_notes.md" }, { path: "AGENTS.md", what_to_extract: "OTEL configuration and OpenObserve setup instructions", document_in: "research_notes.md" }, { path: "src/restate_oya/handlers.rs", what_to_extract: "How lifecycle progress updates are currently handled", document_in: "research_notes.md" }]
		research_questions: [{ question: "Does codebase already use any telemetry or tracing libraries?", answered: false }, { question: "How is LifecycleProgressUpdate currently structured - can we add telemetry there?", answered: false }, { question: "What is CompensationDiagnostic type structure - can it be extended?", answered: false }, { question: "Are there existing OTEL environment variables being read anywhere?", answered: false }]
		research_complete_when: ["[x] All files_to_read have been opened and key info extracted", "[ ] All research_questions have answers documented"]
	}

	inversions: {
		security_failures: [{ failure: "Telemetry data includes sensitive information (API keys, passwords)", prevention: "Redact or exclude sensitive fields from telemetry payload", test_for_it: "test_telemetry_excludes_sensitive_data" }]
		usability_failures: [{ failure: "Telemetry is sent but not visible in OpenObserve UI", prevention: "Verify OTEL configuration and service name match OpenObserve expectations", test_for_it: "test_telemetry_visible_in_openobserve" }, { failure: "Duration measurements overflow for very long-running steps", prevention: "Use u64 for milliseconds, document that very long steps may wrap", test_for_it: "test_duration_handles_long_running_steps" }]
		data_integrity_failures: [{ failure: "Telemetry events are sent out of order or lost", prevention: "Use synchronous emission or buffer with flush on lifecycle completion", test_for_it: "test_telemetry_events_in_correct_order" }, { failure: "Unwind signals not emitted for failed steps", prevention: "Ensure all failure paths trigger unwind signal emission", test_for_it: "test_unwind_signals_emitted_on_step_failure" }]
		integration_failures: [{ failure: "OpenObserve unavailable causes lifecycle to fail", prevention: "Wrap all telemetry emission in try-catch, never fail on telemetry errors", test_for_it: "test_lifecycle_succeeds_when_openobserve_unavailable" }]
	}

	acceptance_tests: {
		happy_paths: [{ name: "test_step_started_emits_telemetry", given: "Lifecycle step moon_ci starts execution", when: "Step progress update is applied", then: ["Telemetry event step_started is emitted", "Event contains step name moon_ci", "Event contains timestamp"] }]
		error_paths: [{ name: "test_telemetry_failure_does_not_break_lifecycle", given: "OpenObserve endpoint is unavailable or network fails", when: "Step progress update tries to emit telemetry", then: ["emit_step_telemetry returns Ok(())", "No panic or error propagates", "Lifecycle execution continues normally"] }]
	}

	verification_checkpoints: {
		gate_0_research: { name: "Research Gate", must_pass_before: "Writing any code", checks: ["[ ] All research_requirements files have been read", "[ ] All research_questions have documented answers", "[ ] Existing telemetry patterns understood", "[ ] Clarification on OTEL vs logging approach resolved"], evidence_required: ["Research notes documenting existing telemetry usage (if any)", "Answers to how to emit telemetry (OTEL API vs logging)", "Understanding of LifecycleProgressUpdate flow"] }
		gate_1_tests: { name: "Test Gate", must_pass_before: "Writing implementation code", checks: ["[ ] All acceptance tests written", "[ ] All error path tests written", "[ ] Tests use real telemetry emission (no mocks)"], evidence_required: ["Tests exist in src/lifecycle/telemetry_tests.rs", "Tests compile (may fail due to missing implementation)"] }
		gate_2_implementation: { name: "Implementation Gate", must_pass_before: "Declaring task complete", checks: ["[ ] All tests pass", "[ ] No unwrap() or expect() calls", "[ ] Telemetry emission integrated into LifecycleProgressUpdate handling", "[ ] Unwind signals emitted during compensation", "[ ] moon run :ci passes"], evidence_required: ["Test output showing all pass", "CI output showing green", "Telemetry visible in OpenObserve (optional but preferred)"] }
	}

	implementation_tasks: {
		phase_0_research: { parallelizable: true, tasks: [{ task: "Read src/lifecycle/workflow/mod.rs and find LifecycleProgressUpdate usage", file: "src/lifecycle/workflow/mod.rs", done_when: "Understand how progress updates flow" }, { task: "Read AGENTS.md for OTEL configuration details", file: "AGENTS.md", done_when: "OTEL endpoint and service name documented" }] }
		phase_1_tests_first: { parallelizable: true, gate_required: "gate_0_research", tasks: [{ task: "Write test: test_step_started_emits_telemetry", file: "src/lifecycle/telemetry_tests.rs", what: "Test that step_started event is emitted", done_when: "Test exists and FAILS (red phase)" }] }
		phase_2_implementation: { parallelizable: false, gate_required: "gate_1_tests", tasks: [{ task: "Create telemetry module with emission functions", file: "src/lifecycle/telemetry.rs", what: "Create module with emit_step_telemetry and emit_unwind_signal functions", done_when: "Module compiles, tests start passing" }, { task: "Integrate telemetry emission into progress update handler", file: "src/lifecycle/workflow/mod.rs", what: "Call emit_step_telemetry in progress callback", done_when: "Telemetry emitted for all step updates" }, { task: "Add unwind signal emission during compensation", file: "src/lifecycle/workflow/mod.rs", what: "Call emit_unwind_signal for each compensated step in reverse order", done_when: "Unwind signals emitted on failures" }] }
		phase_4_verification: { parallelizable: true, gate_required: "gate_2_implementation", tasks: [{ task: "Run moon run :ci", done_when: "All tests pass, no clippy warnings" }, { task: "Manual verification of telemetry in OpenObserve", done_when: "Telemetry visible in OpenObserve UI" }] }
	}

	failure_modes: {
		failure_modes: [{ symptom: "Tests fail with telemetry emission failed error", likely_cause: "Telemetry functions are returning errors instead of Ok(())", where_to_look: [{ file: "src/lifecycle/telemetry.rs", function: "emit_step_telemetry", what_to_check: "Are all error cases caught and converted to Ok(())?" }], fix_pattern: "Wrap all emission logic in try-catch, always return Ok(())" }, { symptom: "Telemetry not visible in OpenObserve", likely_cause: "OTEL endpoint not configured or wrong service name", where_to_look: [{ file: "src/lifecycle/telemetry.rs", what_to_check: "Is OTEL_EXPORTER_OTLP_ENDPOINT being read? Is service name oya-orchestrator?" }], fix_pattern: "Verify OTEL endpoint and service name configuration" }]
	}

	anti_hallucination: {
		read_before_write: [{ file: "src/lifecycle/workflow/mod.rs", must_read_first: true, key_sections_to_understand: ["LifecycleProgressUpdate enum definition", "run_lifecycle_with_progress function", "Progress callback handling"] }]
		apis_that_exist: [{ api: "std::env::var", signature: "fn var(key: &str) -> Result<String, VarError>", import_from: "std::env" }]
		no_placeholder_values: ["Do NOT use placeholder step names like test_step - use real lifecycle step names", "Do NOT use fake OTEL endpoints - use real localhost:4318 from AGENTS.md"]
		git_verification: { before_claiming_done: "git status  # Verify changes are staged\ngit diff    # Verify changes match specification\nmoon run :test  # Verify all tests pass" }
	}

	completion_checklist: { tests: ["[ ] test_step_started_emits_telemetry passes", "[ ] test_step_completed_emits_duration passes", "[ ] test_unwind_signal_emitted_on_step_failure passes", "[ ] test_telemetry_failure_does_not_break_lifecycle passes", "[ ] E2E test with OpenObserve passes", "[ ] All existing lifecycle tests still pass"], code: ["[ ] No unwrap() or expect() in new code", "[ ] Telemetry module created and integrated", "[ ] Unwind signals emitted during compensation", "[ ] OTEL service name configured correctly"], ci: ["[ ] moon run :ci passes", "[ ] No clippy warnings", "[ ] No compiler warnings"] }

	ai_hints: { do: ["Decide on OTEL vs logging approach during research phase", "Read AGENTS.md carefully for correct OTEL endpoint configuration", "Make telemetry emission never fail lifecycle - always return Ok(())", "Use functional patterns: map, and_then, ?", "Update progress.txt after each completed task", "Commit to git after each completed task"], do_not: ["Do NOT use unwrap() or expect() for telemetry operations", "Do NOT let telemetry failures break lifecycle execution", "Do NOT create new dependencies without checking existing Cargo.toml", "Do NOT assume OpenObserve is always available - handle unavailability gracefully"] }
}
