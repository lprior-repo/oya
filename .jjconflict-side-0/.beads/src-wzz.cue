package bead

// Bead: src-wzz - Codify planner + rust-contract + red-queen self-build workflow
// Type: task | Priority: 2 | Effort: 30min

#EnhancedBead: {
	id: "src-wzz"
	title: "documentation: Codify planner + rust-contract + red-queen self-build workflow"
	type: "task"
	priority: 2
	effort_estimate: "30min"
	labels: ["documentation", "agents-md", "self-build", "workflow"]

	clarifications: {
		clarification_status: "RESOLVED"
		resolved_clarifications: [
			{ question: "What is self-build workflow that needs to be codified?", answer: "The workflow defined in AGENTS.md line 16 that uses planner, rust-contract, and red-queen for implementation beads", decided_by: "code analysis", date: "2026-02-27" },
			{ question: "What does 'codify' mean in this context?", answer: "Add explicit documentation and rules explaining workflow, similar to other entries in AGENTS.md", decided_by: "prior analysis", date: "2026-02-27" },
		]
		assumptions: [{ assumption: "The self-build workflow in line 16 is already implemented but needs better documentation", validation_method: "Verify that oya lifecycle uses planner/rust-contract/red-queen", risk_if_wrong: "Documentation may describe workflow that doesn't actually exist" }]
	}

	ears_requirements: {
		ubiquitous: ["THE SYSTEM SHALL document self-build workflow in AGENTS.md", "THE SYSTEM SHALL reference planner, rust-contract, and red-queen skills", "THE SYSTEM SHALL explain workflow steps clearly"]
		event_driven: [{ trigger: "WHEN a developer implements an implementation bead for oya", shall: "THE SYSTEM SHALL follow self-build workflow documented in AGENTS.md" }]
		unwanted: [{ condition: "IF self-build workflow is ambiguous or unclear", shall_not: "THE SYSTEM SHALL NOT leave developers guessing about correct process", because: "Unclear workflows lead to inconsistent implementation practices" }]
	}

	contracts: {
		preconditions: {
			auth_required: false
			required_inputs: [{ field: "AGENTS.md", type: "File path /home/lewis/src/oya/AGENTS.md", constraints: "Exists and is valid JSONL format", example_valid: "Existing AGENTS.md at /home/lewis/src/oya/AGENTS.md", example_invalid: "Non-existent or malformed file" }]
			system_state: ["AGENTS.md exists and is valid JSONL", "planner, rust-contract, red-queen skills are defined"]
		}
		postconditions: {
			state_changes: ["AGENTS.md contains explicit documentation of self-build workflow", "Workflow documentation includes skill purposes and step ordering", "No existing entries in AGENTS.md are modified"]
			return_guarantees: [{ field: "documentation quality", guarantee: "Self-build workflow is clearly explained and actionable" }, { field: "file validity", guarantee: "AGENTS.md remains valid JSONL format" }]
			side_effects: ["AGENTS.md file is modified on disk"]
		}
		invariants: ["AGENTS.md entries remain in JSONL format (one JSON object per line)", "No existing workflow entries are modified or removed", "All skill references match skill names defined in skill loading rules"]
	}

	research_requirements: {
		files_to_read: [
			{ path: "AGENTS.md", what_to_extract: "Current self-build workflow entry and structure of other workflow entries", document_in: "research_notes.md" },
			{ path: "docs/BEADS.md", what_to_extract: "Existing documentation about beads and workflows", document_in: "research_notes.md" },
		]
		patterns_to_find: [{ pattern: "\"workflow\"", purpose: "Find all workflow entries in AGENTS.md to understand format", expected_locations: "AGENTS.md" }]
		research_questions: [
			{ question: "What is exact format of workflow entries in AGENTS.md?", answered: false },
			{ question: "Are there any existing references to planner, rust-contract, or red-queen outside of line 16?", answered: false },
			{ question: "Should new documentation add a new entry or expand existing line 16?", answered: false },
		]
		research_complete_when: ["[x] All files_to_read have been opened and key info extracted", "[ ] All patterns_to_find have been searched", "[ ] All research_questions have answers documented"]
	}

	inversions: {
		usability_failures: [
			{ failure: "Workflow documentation is too abstract and lacks concrete examples", prevention: "Include specific command examples and step-by-step guidance", test_for_it: "test_workflow_documentation_includes_examples" },
			{ failure: "Developers don't know when to use self-build vs regular bead workflow", prevention: "Explicitly document when self-build workflow applies", test_for_it: "test_when_to_use_self_build_documented" },
		]
		data_integrity_failures: [{ failure: "AGENTS.md becomes invalid JSONL after update", prevention: "Validate JSON format before committing changes", test_for_it: "test_agents_md_remains_valid_jsonl" }]
		integration_failures: [{ failure: "New documentation conflicts with existing skill loading rules", prevention: "Ensure new entries align with existing skill definitions", test_for_it: "test_workflow_references_known_skills" }]
	}

	acceptance_tests: {
		happy_paths: [{
				name: "test_self_build_workflow_documented",
				given: "AGENTS.md contains existing self-build workflow entry",
				when: "New documentation is added to AGENTS.md",
				then: ["Self-build workflow is clearly explained", "Planner skill purpose is documented", "Rust-contract skill purpose is documented", "Red-queen skill purpose is documented", "Workflow steps are in order"]
				real_input: '{"kind":"guide","id":"self-build-workflow","text":"..."}'
				expected_output: "AGENTS.md with comprehensive self-build workflow documentation",
		}]
		error_paths: []
	}

	verification_checkpoints: {
		gate_0_research: {
			name: "Research Gate"
			must_pass_before: "Writing documentation"
			checks: ["[ ] All research_requirements files have been read", "[ ] All research_questions have documented answers", "[ ] AGENTS.md format and structure understood"]
			evidence_required: ["Research notes documenting AGENTS.md structure", "Answers to how to emit telemetry (OTEL API vs logging)", "Understanding of current self-build workflow entry"]
		}
		gate_1_tests: {
			name: "Test Gate"
			must_pass_before: "Writing documentation code"
			checks: ["[ ] All acceptance tests written", "[ ] Tests validate JSONL format", "[ ] Tests verify content is added correctly"]
			evidence_required: ["Tests exist in tests/agents_md_tests.rs or shell script", "Tests verify documentation structure"]
		}
		gate_2_implementation: {
			name: "Implementation Gate"
			must_pass_before: "Declaring task complete"
			checks: ["[ ] All tests pass", "[ ] AGENTS.md updated with comprehensive documentation", "[ ] JSONL format validated"]
			evidence_required: ["Test output showing all pass", "jq validation of AGENTS.md succeeds"]
		}
	}

	implementation_tasks: {
		phase_0_research: {
			parallelizable: true
			tasks: [
				{ task: "Read AGENTS.md and extract current self-build workflow entry", file: "AGENTS.md", done_when: "Current line 16 entry documented and understood" }
				{ task: "Understand workflow entry format in AGENTS.md", file: "AGENTS.md", done_when: "Format of workflow entries (kind, id, text, steps) understood" }
			]
		}
		phase_1_tests_first: {
			parallelizable: true
			gate_required: "gate_0_research"
			tasks: [{ task: "Write test: test_agents_md_remains_valid_jsonl", file: "tests/agents_md_tests.rs or scripts/validate_agents.sh", what: "Shell script or test that validates each line of AGENTS.md is valid JSON", done_when: "Test exists and PASSES (green phase - AGENTS.md currently valid)" }]
		}
		phase_2_implementation: {
			parallelizable: false
			gate_required: "gate_1_tests"
			tasks: [
				{ task: "Add comprehensive self-build workflow documentation to AGENTS.md", file: "AGENTS.md", what: "Add new guide entry explaining self-build workflow with planner, rust-contract, red-queen", done_when: "AGENTS.md contains new documentation entry" }
				{ task: "Validate AGENTS.md remains valid JSONL", file: "AGENTS.md", what: "Run jq validation on each line to ensure format is correct", done_when: "All lines parse as valid JSON" }
			]
		}
		phase_4_verification: {
			parallelizable: true
			gate_required: "gate_2_implementation"
			tasks: [{ task: "Run moon run :ci", done_when: "All tests pass, no clippy warnings" }, { task: "Verify AGENTS.md with jq", done_when: "All lines parse successfully" }]
		}
	}

	anti_hallucination: {
		read_before_write: [{ file: "AGENTS.md", must_read_first: true, key_sections_to_understand: ["Line 16: existing self-build workflow entry", "Format of workflow entries (kind, steps)"] }]
		apis_that_exist: [{ api: "jq command-line tool", signature: "jq [expression] [file]", import_from: "System shell" }]
		no_placeholder_values: ["Do NOT use placeholder skill names - use actual skill names: planner, rust-contract, red-queen", "Do NOT use generic workflow descriptions - use specific step names from actual workflow"]
		git_verification: { before_claiming_done: "git status  # Verify changes are staged\ngit diff AGENTS.md    # Verify changes match specification\ncat AGENTS.md | while read line; do echo \"$line\" | jq . > /dev/null 2>&1 || exit 1; done && echo \"All lines valid\"" }
	}

	context_survival: {
		progress_file: { path: ".bead-progress/src-wzz/progress.txt", format: "# Bead: src-wzz - Codify planner + rust-contract + red-queen self-build workflow\n# Started: [timestamp]\n# Last updated: [timestamp]\n\n## Current Phase\n[phase_name]" }
		research_notes_file: { path: ".bead-progress/src-wzz/research.md", contains: ["AGENTS.md current structure", "Existing self-build workflow entry", "Format requirements for new documentation"] }
		recovery_instructions: "If context window is cleared, start new session with:\n1. cat .bead-progress/src-wzz/progress.txt\n2. git log --oneline -10\n3. Continue from where progress.txt indicates"
	}

	completion_checklist: {
		tests: ["[ ] test_self_build_workflow_documented passes", "[ ] test_agents_md_remains_valid_jsonl passes"]
		code: ["[ ] AGENTS.md updated with comprehensive self-build workflow documentation", "[ ] Documentation includes planner skill purpose", "[ ] Documentation includes rust-contract skill purpose", "[ ] Documentation includes red-queen skill purpose", "[ ] Documentation includes workflow steps in order"]
		ci: ["[ ] AGENTS.md is valid JSONL (jq validation passes)", "[ ] No clippy warnings (if any Rust tests added)"]
	}

	context: {
		related_files: [{ path: "AGENTS.md", relevance: "Target file for documentation updates" }, { path: "docs/BEADS.md", relevance: "Related documentation about beads and workflows" }]
		similar_implementations: ["Existing workflow entries in AGENTS.md for format reference"]
		codebase_patterns: [{ pattern: "AGENTS.md JSONL format", example_location: "AGENTS.md:1-46", how_to_apply: "Each line is a complete JSON object with kind, id, and text fields" }]
	}

	ai_hints: {
		do: ["Read AGENTS.md carefully before making any changes", "Follow existing JSONL format exactly", "Make documentation clear and actionable for developers", "Include skill purposes and workflow steps", "Update progress.txt after each completed task", "Commit to git after each completed task"]
		do_not: ["Do NOT modify existing entries in AGENTS.md", "Do NOT break JSONL format", "Do NOT use placeholder or vague descriptions", "Do NOT add entries for skills that don't exist"]
		code_patterns: [{ name: "AGENTS.md JSONL line format", use_when: "Adding new entries to AGENTS.md", example: '{"kind":"guide","id":"self-build-workflow","text":"Complete description of workflow"}' }]
		constitution: ["JSONL format: Each line must be valid JSON", "No modifications to existing entries", "Clear and actionable documentation"]
	}
}
