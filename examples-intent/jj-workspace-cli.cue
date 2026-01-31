package jj_workspace

import "github.com/intent-cli/intent/schema:intent"

spec: intent.#Spec & {
	name: "JJ Workspace Manager CLI"

	description: """
		A command-line tool for managing JJ version control workspaces and
		Zellij terminal sessions. Supports workspace creation, tab management,
		and synchronization with Zellij.
		"""

	audience: "DevOps engineers and developers"

	success_criteria: [
		"Users can create, list, and delete workspaces",
		"Users can focus specific workspace tabs in Zellij",
		"Exit codes indicate success/failure states clearly",
		"Error messages provide actionable guidance",
	}

	config: {
		base_url:   ""  // CLI tools don't have base_url
		timeout_ms: 30000  // 30 second default for operations
		headers:    {}
	}

	features: [
		{
			name: "Workspace Management"

			description: """
				Create, list, and delete JJ workspaces. Each workspace
				corresponds to a directory in the user's file system.
				"""

			commands: [
				{
					name:   "workspace-add"
					intent: "Create a new JJ workspace and open in Zellij"

					usage: "jj workspace add <name> [--no-hooks] [--template]"

					flags: {
						"--no-hooks":    "Skip running post-create hooks",
						"--template":    "Use a template directory",
						"--no-open":     "Create workspace without opening Zellij",
					}

					exit_code: 0

					success_example: {
						output: "Workspace 'my-project' created at ~/dev/my-project"
						exit_code: 0
					}

					error_examples: [
						{
							condition: "workspace already exists"
							exit_code: 1
							error: "Workspace 'my-project' already exists"
						},
						{
							condition: "invalid name"
							exit_code: 2
							error: "Invalid workspace name: contains invalid characters"
						},
					}
				},
				{
					name:   "workspace-list"
					intent: "List all available workspaces"

					usage: "jj workspace list"

					exit_code: 0

					success_example: {
						output: "my-project\nanother-project\n"
						exit_code: 0
					}
				},
				{
					name:   "workspace-delete"
					intent: "Delete a workspace"

					usage: "jj workspace delete <name>"

					exit_code: 0

					error_example: {
						condition: "workspace not found"
						exit_code: 1
						error: "Workspace 'nonexistent' not found"
					}
				},
			}
		},
		{
			name: "Tab Management"

			description: """
				Manage Zellij tabs within workspaces. Users can focus
				specific tabs, list all tabs, and sync tab state.
				"""

			commands: [
				{
					name:   "tab-focus"
					intent: "Focus a specific tab in Zellij"

					usage: "jj tab focus <name>"

					exit_code: 0

					error_example: {
						condition: "tab not found"
						exit_code: 1
						error: "Tab 'main' not found in workspace 'my-project'"
					}
				},
				{
					name:   "tab-list"
					intent: "List all tabs in the current workspace"

					usage: "jj tab list"

					exit_code: 0
				},
			}
		},
		{
			name: "Synchronization"

			description: """
				Synchronize workspace state with remote repository and
				update Zellij tabs to reflect latest changes.
				"""

			commands: [
				{
					name:   "sync"
					intent: "Synchronize workspace with remote"

					usage: "jj sync"

					exit_code: 0

					success_example: {
						output: "Synced 3 changes from remote"
						exit_code: 0
					}

					error_example: {
						condition: "no remote configured"
						exit_code: 1
						error: "No remote configured for this workspace"
					}
				},
			}
		},
		{
			name: "Diagnostics"

			description: """
				Health check and troubleshooting commands for workspace
				and Zellij integration.
				"""

			commands: [
				{
					name:   "doctor"
					intent: "Check workspace health and configuration"

					usage: "jj doctor"

					exit_code: 0

					success_example: {
						output: "✓ Workspace configuration is valid\n✓ Zellij connection active"
						exit_code: 0
					}

					error_example: {
						condition: "zellij not running"
						exit_code: 3
						error: "Zellij daemon not running - start with 'zellij'"
					}
				},
				{
					name:   "dashboard"
					intent: "Show workspace overview dashboard"

					usage: "jj dashboard"

					exit_code: 0
				},
			}
		},
	}

	rules: [
		{
			name:        "exit-code-semantics"
			description: "Exit codes follow standard conventions"

			when: {exit_code: "!= 0"}

			check: {
				exit_code_semantics: {
					"0":  "success",
					"1":  "generic error",
					"2":  "usage error (invalid flags, missing args)",
					"3":  "blocked (prerequisite failed, resource unavailable)",
				}
			}
		},
		{
			name:        "error-to-stderr"
			description: "All error messages go to stderr, success to stdout"

			when: {exit_code: "!= 0"}

			check: {
				errors_use_stderr: true
				success_output_format: "JSON (with --json flag) or human-readable"
			}
		},
	}

	anti_patterns: [
		{
			name:        "success-to-stderr"
			description: "Success output should go to stdout, not stderr"

			bad_example: {
				command: "jj workspace add myproj"
				stderr: "Created workspace"  // WRONG
				stdout: ""
			}

			good_example: {
				command: "jj workspace add myproj"
				stderr: ""  // Empty on success
				stdout: "Workspace 'myproj' created"  // OR JSON output
			}
		},
		{
			name:        "inconsistent-exit-codes"
			description: "Same error type should always return same exit code"

			bad_example: {
				command: "jj workspace add myproj"
				exit_code: 1  // First attempt
				command: "jj workspace add myproj"
				exit_code: 2  // Same error, different code!
			}

			good_example: {
				command: "jj workspace add myproj"
				exit_code: 1  // Always 1 for "workspace exists"
			}
		},
	}

	ai_hints: {
		implementation: {
			suggested_stack: ["Rust", "Clap", "anyhow", "zellij-client"]
		}

		entities: {
			workspace: {
				fields: {
					name: "string, 1-64 chars, alphanumeric and hyphens only"
					path: "string, absolute path to workspace directory"
					template: "string | null, template directory used"
					created_at: "datetime, when workspace was created"
				}
			}
			}

		security: {
			authentication: "Not applicable - local CLI tool"
			authorization: "File system permissions apply"
			rate_limiting: "Not applicable - local CLI tool"
		}

		pitfalls: [
			"Don't block on interactive prompts without --tty flag",
			"Don't assume Zellij is always running - check with doctor command",
			"Don't mix JSON and human-readable output in same response",
			"Don't use exit code 0 for errors with --json flag (use JSON error response)",
		}
	}
}
