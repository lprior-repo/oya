// BDD-style tests for CLI validation
//
// Given-When-Then format:
// Given: Preconditions and context
// When: Action or event
// Then: Expected outcome

use std::process::Command;

/// Helper struct to capture command execution results
struct CommandResult {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

/// Helper function to run oya command and capture output
fn run_oya_command(args: &[&str]) -> CommandResult {
    let output = Command::new("cargo")
        .args(["run", "--release", "--bin", "oya", "--"])
        .args(args)
        .output()
        .expect("Failed to execute oya command");

    CommandResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(999),
    }
}

#[cfg(test)]
mod slug_validation {
    use super::*;

    mod empty_slug {
        use super::*;

        #[test]
        fn given_empty_slug_when_creating_task_then_rejects_with_exit_code_2() {
            // Given: An empty slug is provided
            let slug = "";

            // When: Creating a new task with empty slug
            let result = run_oya_command(&["new", "--slug", slug]);

            // Then: Command should fail with exit code 2 (argument error)
            assert_eq!(result.exit_code, 2, "Empty slug should exit with code 2");

            // And: Error message should be clear
            assert!(
                result.stderr.contains("Error: Slug cannot be empty"),
                "Error message should mention empty slug"
            );

            // And: Helpful hint should be provided
            assert!(result.stderr.contains("Hint:"), "Should provide usage hint");
        }

        #[test]
        fn given_whitespace_only_slug_when_creating_task_then_rejects() {
            // Given: A slug containing only whitespace
            let slug = "   ";

            // When: Creating a new task with whitespace slug
            let result = run_oya_command(&["new", "--slug", slug]);

            // Then: Command should fail
            assert_eq!(result.exit_code, 2);

            // And: Error message should mention empty slug
            assert!(
                result.stderr.contains("Error: Slug cannot be empty"),
                "Whitespace-only slug should be treated as empty"
            );
        }
    }

    mod path_traversal {
        use super::*;

        #[test]
        fn given_slug_with_double_dots_when_creating_task_then_rejects_with_exit_code_2() {
            // Given: A slug containing path traversal sequence
            let slug = "../etc/passwd";

            // When: Creating a new task with path traversal attempt
            let result = run_oya_command(&["new", "--slug", slug]);

            // Then: Command should fail with exit code 2 (validation error)
            assert_eq!(
                result.exit_code, 2,
                "Path traversal should exit with code 2"
            );

            // And: Error message should mention path separators
            assert!(
                result
                    .stderr
                    .contains("Error: Slug cannot contain path separators or traversal sequences"),
                "Error should mention path traversal rejection"
            );

            // And: Hint should provide valid example
            assert!(
                result.stderr.contains("my-feature") || result.stderr.contains("task-123"),
                "Hint should show valid slug format"
            );
        }

        #[test]
        fn given_slug_with_forward_slash_when_creating_task_then_rejects() {
            // Given: A slug containing forward slash
            let slug = "some/nested/path";

            // When: Creating a new task with slash in slug
            let result = run_oya_command(&["new", "--slug", slug]);

            // Then: Command should fail with exit code 2
            assert_eq!(result.exit_code, 2);

            // And: Error should mention path separators
            assert!(
                result.stderr.contains("path separators"),
                "Error should mention path separators"
            );
        }

        #[test]
        fn given_valid_slug_when_creating_task_then_passes_validation() {
            // Given: A valid slug format
            let slug = "my-feature-123";

            // When: Creating a new task with valid slug
            let result = run_oya_command(&["new", "--slug", slug]);

            // Then: Validation should pass (command still unimplemented, but that's different)
            // The command should fail with exit code 1 (not implemented), not exit code 2 (validation)
            assert_ne!(result.exit_code, 2, "Valid slug should not fail validation");

            // And: Should not mention slug validation error
            assert!(
                !result.stderr.contains("Slug cannot contain"),
                "Valid slug should not trigger validation error"
            );
        }
    }
}

#[cfg(test)]
mod exit_codes {
    use super::*;

    mod command_execution {
        use super::*;

        #[test]
        fn given_list_command_when_executed_then_exits_with_code_0() {
            // Given: The list command is implemented
            // When: Running list command
            let result = run_oya_command(&["list"]);

            // Then: Should exit with code 0 (success)
            assert_eq!(
                result.exit_code, 0,
                "Implemented command should exit with code 0"
            );
        }

        #[test]
        fn given_show_command_with_missing_task_when_executed_then_exits_with_code_2() {
            // Given: The show command is implemented but task is missing
            // When: Running show command
            let result = run_oya_command(&["show", "--slug", "nonexistent-task"]);

            // Then: Should exit with code 2 (not found/validation error)
            assert_eq!(result.exit_code, 2);
        }
    }
}

#[cfg(test)]
mod help_text {
    use super::*;

    #[test]
    fn given_help_command_when_executed_then_shows_examples() {
        // Given: The user requests help
        // When: Running help command
        let result = run_oya_command(&["--help"]);

        // Then: Output should contain example usage
        assert!(
            result.stdout.contains("oya new --slug") || result.stdout.contains("Examples:"),
            "Help should show usage examples"
        );

        // And: Should show stage command example
        assert!(
            result.stdout.contains("stage"),
            "Help should show stage command"
        );

        // And: Should show approve command example
        assert!(
            result.stdout.contains("approve"),
            "Help should show approve command"
        );
    }
}
