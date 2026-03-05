package bead

#EnhancedBead: {
	id: "src-golden"
	title: "cli: Add golden-retriever CLI tool for fetching project resources"
	type: "feature"
	priority: 2
	effort_estimate: "2hr"
	labels: ["cli", "tool", "golden-retriever", "project-management"]

	clarifications: {
		clarification_status: "RESOLVED"
		resolved_clarifications: [
			{
				question: "What should the golden-retriever CLI tool do?"
				answer: "A CLI tool that fetches and retrieves project-related resources - config files, status information, bead details, and other useful data"
				decided_by: "feature definition"
				date: "2026-03-05"
			},
			{
				question: "Where should the CLI tool be implemented?"
				answer: "As a new binary in the oya crate, similar to the main oya CLI"
				decided_by: "architecture alignment"
				date: "2026-03-05"
			},
		]
		assumptions: [{
			assumption: "The golden-retriever tool should integrate with existing oya infrastructure"
			validation_method: "Review existing CLI implementation patterns"
			risk_if_wrong: "May need significant refactoring if architecture differs"
		}]
	}

	ears_requirements: {
		ubiquitous: [
			"THE SYSTEM SHALL provide a golden-retriever binary executable"
			"THE SYSTEM SHALL support --help flag showing available commands"
			"THE SYSTEM SHALL use clap for argument parsing following oya patterns"
		]
		event_driven: [
			{
				trigger: "WHEN user runs 'golden-retriever fetch <resource>'"
				shall: "THE SYSTEM SHALL fetch and display the requested resource"
			},
			{
				trigger: "WHEN user runs 'golden-retriever status'"
				shall: "THE SYSTEM SHALL display current project status and active beads"
			},
			{
				trigger: "WHEN user runs 'golden-retriever config'"
				shall: "THE SYSTEM SHALL display current configuration"
			},
		]
		unwanted: [
			{
				condition: "IF resource not found"
				shall_not: "THE SYSTEM SHALL NOT panic or crash"
				because: "CLI tools must handle errors gracefully with clear messages"
			},
		]
	}

	contracts: {
		preconditions: {
			auth_required: false
			required_inputs: [
				{
					field: "command"
					type: "String"
					constraints: "One of: fetch, status, config"
					example_valid: "status"
					example_invalid: "bark"
				},
			]
			system_state: ["oya project structure exists", ".beads directory present"]
		}
		postconditions: {
			state_changes: ["Read-only operations - no state modifications"]
			return_guarantees: [
				{
					field: "exit_code"
					guarantee: "Returns 0 on success, non-zero on error"
				},
				{
					field: "output"
					guarantee: "Prints requested information to stdout or error to stderr"
				},
			]
		}
	}

	error_taxonomy: {
		categories: [
			{
				name: "ResourceNotFound"
				description: "Requested resource does not exist"
				example: "Bead ID not found in .beads directory"
				recovery: "Display helpful error with available resources"
			},
			{
				name: "InvalidCommand"
				description: "Unknown command provided"
				example: "User provides 'bark' instead of 'fetch'"
				recovery: "Display usage help with valid commands"
			},
		]
	}

	illegal_states: {
		empty_output: {
			description: "Command succeeds but produces no output"
			prevention: "All commands must produce meaningful output or explicit confirmation"
		}
	}

	test_scenarios: {
		happy_path: [
			{
				name: "fetch status"
				given: "Valid oya project"
				when: "Running 'golden-retriever status'"
				then: "Displays current project status"
			},
		]
		error_cases: [
			{
				name: "invalid command"
				given: "Any state"
				when: "Running 'golden-retriever bark'"
				then: "Shows error and usage help"
			},
		]
	}

	implementation_notes: {
		files_to_create: ["src/bin/golden-retriever.rs"]
		patterns_to_follow: ["Use clap derive macros like main oya CLI", "Follow existing error handling patterns"]
		dependencies: ["clap", "anyhow or thiserror"]
	}
}
