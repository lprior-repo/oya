// BDD-style tests for CLI validation
//
// Given-When-Then format:
// Given: Preconditions and context
// When: Action or event
// Then: Expected outcome

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::process::Command;

/// Helper struct to capture command execution results
struct CommandResult {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

/// Helper function to run oya command and capture output
fn run_oya_command(args: &[&str]) -> Result<CommandResult, Box<dyn std::error::Error>> {
    let output = Command::new("cargo")
        .args(["run", "--release", "--bin", "oya", "--"])
        .args(args)
        .output()?;

    Ok(CommandResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(999),
    })
}

#[cfg(test)]
mod slug_validation {
    use super::*;

    mod empty_slug {
        use super::*;

        #[test]
        fn given_empty_slug_when_creating_task_then_rejects_with_exit_code_2(
        ) -> Result<(), Box<dyn std::error::Error>> {
            // Given: An empty slug is provided
            let slug = "";

            // When: Creating a new task with empty slug
            let result = run_oya_command(&["new", "--slug", slug])?;

            // Then: Command should fail with exit code 2 (argument error)
            assert_eq!(result.exit_code, 2, "Empty slug should exit with code 2");

            // And: Error message should be clear
            assert!(
                result.stderr.contains("Error: Slug cannot be empty"),
                "Error message should mention empty slug"
            );

            // And: Helpful hint should be provided
            assert!(result.stderr.contains("Hint:"), "Should provide usage hint");
            Ok(())
        }

        #[test]
        fn given_whitespace_only_slug_when_creating_task_then_rejects(
        ) -> Result<(), Box<dyn std::error::Error>> {
            // Given: A slug containing only whitespace
            let slug = "   ";

            // When: Creating a new task with whitespace slug
            let result = run_oya_command(&["new", "--slug", slug])?;

            // Then: Command should fail
            assert_eq!(result.exit_code, 2);

            // And: Error message should mention empty slug
            assert!(
                result.stderr.contains("Error: Slug cannot be empty"),
                "Whitespace-only slug should be treated as empty"
            );
            Ok(())
        }
    }

    mod path_traversal {
        use super::*;

        #[test]
        fn given_slug_with_double_dots_when_creating_task_then_rejects_with_exit_code_2(
        ) -> Result<(), Box<dyn std::error::Error>> {
            // Given: A slug containing path traversal sequence
            let slug = "../etc/passwd";

            // When: Creating a new task with path traversal attempt
            let result = run_oya_command(&["new", "--slug", slug])?;

            // Then: Command should fail with exit code 2 (validation error)
            assert_eq!(
                result.exit_code, 2,
                "Path traversal should exit with code 2"
            );

            // And: Error message should mention invalid characters
            assert!(
                result
                    .stderr
                    .contains("Error: Slug cannot contain invalid characters"),
                "Error should mention invalid characters rejection"
            );

            // And: Hint should provide valid example
            assert!(
                result.stderr.contains("my-feature") || result.stderr.contains("task-123"),
                "Hint should show valid slug format"
            );
            Ok(())
        }

        #[test]
        fn given_slug_with_forward_slash_when_creating_task_then_rejects(
        ) -> Result<(), Box<dyn std::error::Error>> {
            // Given: A slug containing forward slash
            let slug = "some/nested/path";

            // When: Creating a new task with slash in slug
            let result = run_oya_command(&["new", "--slug", slug])?;

            // Then: Command should fail with exit code 2
            assert_eq!(result.exit_code, 2);

            // And: Error should mention invalid characters
            assert!(
                result.stderr.contains("invalid characters"),
                "Error should mention invalid characters"
            );
            Ok(())
        }

        #[test]
        fn given_valid_slug_when_creating_task_then_passes_validation(
        ) -> Result<(), Box<dyn std::error::Error>> {
            // Given: A valid slug format
            let slug = "my-feature-123";

            // When: Creating a new task with valid slug
            let result = run_oya_command(&["new", "--slug", slug])?;

            // Then: Validation should pass (command still unimplemented, but that's different)
            // The command should fail with exit code 1 (not implemented), not exit code 2 (validation)
            assert_ne!(result.exit_code, 2, "Valid slug should not fail validation");

            // And: Should not mention slug validation error
            assert!(
                !result.stderr.contains("Slug cannot contain"),
                "Valid slug should not trigger validation error"
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod exit_codes {
    use super::*;

    mod command_execution {
        use super::*;

        #[test]
        fn given_list_command_when_executed_then_exits_with_code_0(
        ) -> Result<(), Box<dyn std::error::Error>> {
            // Given: The list command is implemented
            // When: Running list command
            let result = run_oya_command(&["list"])?;

            // Then: Should exit with code 0 (success)
            assert_eq!(
                result.exit_code, 0,
                "Implemented command should exit with code 0"
            );
            Ok(())
        }

        #[test]
        fn given_show_command_with_missing_task_when_executed_then_exits_with_code_2(
        ) -> Result<(), Box<dyn std::error::Error>> {
            // Given: The show command is implemented but task is missing
            // When: Running show command
            let result = run_oya_command(&["show", "--slug", "nonexistent-task"])?;

            // Then: Should exit with code 2 (not found/validation error)
            assert_eq!(result.exit_code, 2);
            Ok(())
        }
    }
}

#[cfg(test)]
mod help_text {
    use super::*;

    #[test]
    fn given_help_command_when_executed_then_shows_examples(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Given: The user requests help
        // When: Running help command
        let result = run_oya_command(&["--help"])?;

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
        Ok(())
    }
}

#[cfg(test)]
mod command_matrix {
    use super::*;

    mod list_command {
        use super::*;

        #[test]
        fn given_list_command_with_json_flag_then_outputs_json(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["list", "--json"])?;
            assert_eq!(result.exit_code, 0);
            assert!(
                result.stdout.contains("\"tasks\"") || result.stdout.contains("\"total\""),
                "JSON output should contain tasks or total field"
            );
            Ok(())
        }

        #[test]
        fn given_list_command_with_unknown_flag_then_rejects(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["list", "--unknown-flag"])?;
            assert_ne!(result.exit_code, 0, "Unknown flag should be rejected");
            assert!(
                result.stderr.contains("error") || result.stderr.contains("unexpected"),
                "Should show error for unknown flag"
            );
            Ok(())
        }

        #[test]
        fn given_list_command_with_invalid_root_then_fails_gracefully(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["list", "--root", "/nonexistent/path/12345"])?;
            assert!(
                result.exit_code == 0 || result.exit_code == 1,
                "Should handle nonexistent root gracefully (exit 0 with empty or 1 with error)"
            );
            Ok(())
        }
    }

    mod show_command {
        use super::*;

        #[test]
        fn given_null_byte_in_slug_then_cannot_execute_command(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let slug_with_null = "test\x00task";
            let output = Command::new("cargo")
                .args([
                    "run",
                    "--release",
                    "--bin",
                    "oya",
                    "--",
                    "new",
                    "--slug",
                    slug_with_null,
                ])
                .output();

            match output {
                Ok(_) => {}
                Err(e) => {
                    assert!(
                        e.to_string().contains("nul byte")
                            || e.to_string().contains("InvalidInput"),
                        "Null byte should be rejected at command level: {e}"
                    );
                }
            }
            Ok(())
        }

        #[test]
        fn given_show_command_with_special_chars_slug_then_rejects_or_fails(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["show", "--slug", "../../../etc/passwd"])?;
            assert_ne!(result.exit_code, 0, "Path traversal should be rejected");
            Ok(())
        }

        #[test]
        fn given_show_command_without_slug_arg_then_fails() -> Result<(), Box<dyn std::error::Error>>
        {
            let result = run_oya_command(&["show"])?;
            assert_ne!(result.exit_code, 0, "Missing slug should fail");
            Ok(())
        }
    }

    mod stage_command {
        use super::*;

        #[test]
        fn given_stage_command_without_slug_then_fails() -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["stage"])?;
            assert_ne!(result.exit_code, 0, "Missing slug should fail");
            Ok(())
        }

        #[test]
        fn given_stage_command_without_stage_then_fails() -> Result<(), Box<dyn std::error::Error>>
        {
            let result = run_oya_command(&["stage", "--slug", "test-task"])?;
            assert_ne!(result.exit_code, 0, "Missing stage should fail");
            Ok(())
        }

        #[test]
        fn given_stage_command_with_invalid_stage_then_rejects(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&[
                "stage",
                "--slug",
                "test-task",
                "--stage",
                "invalid-stage-xyz",
            ])?;
            assert!(
                result.exit_code != 0 || result.stdout.contains("unknown"),
                "Invalid stage should be rejected or show unknown"
            );
            Ok(())
        }
    }

    mod approve_command {
        use super::*;

        #[test]
        fn given_approve_command_without_slug_then_fails() -> Result<(), Box<dyn std::error::Error>>
        {
            let result = run_oya_command(&["approve"])?;
            assert_ne!(result.exit_code, 0, "Missing slug should fail");
            Ok(())
        }

        #[test]
        fn given_approve_command_with_nonexistent_slug_then_fails_gracefully(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["approve", "--slug", "nonexistent-task-xyz-123"])?;
            assert_ne!(result.exit_code, 0, "Nonexistent task should fail");
            Ok(())
        }
    }

    mod workspace_command {
        use super::*;

        #[test]
        fn given_workspace_command_without_subcommand_then_shows_help_or_fails(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["workspace"])?;
            assert!(
                result.stdout.contains("Commands:") || result.exit_code != 0,
                "Should show help or fail without subcommand"
            );
            Ok(())
        }

        #[test]
        fn given_workspace_list_command_then_succeeds_or_fails_gracefully(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["workspace", "list"])?;
            assert!(
                result.exit_code == 0 || result.exit_code == 1,
                "Should succeed or fail gracefully"
            );
            Ok(())
        }

        #[test]
        fn given_workspace_status_without_name_then_fails() -> Result<(), Box<dyn std::error::Error>>
        {
            let result = run_oya_command(&["workspace", "status"])?;
            assert_ne!(result.exit_code, 0, "Missing workspace name should fail");
            Ok(())
        }

        #[test]
        fn given_workspace_unknown_subcommand_then_fails() -> Result<(), Box<dyn std::error::Error>>
        {
            let result = run_oya_command(&["workspace", "unknown-subcommand"])?;
            assert_ne!(result.exit_code, 0, "Unknown subcommand should fail");
            Ok(())
        }
    }
}

#[cfg(test)]
mod no_args {
    use super::*;

    #[test]
    fn given_no_args_then_shows_usage_or_help() -> Result<(), Box<dyn std::error::Error>> {
        let result = run_oya_command(&[])?;
        assert_eq!(
            result.exit_code, 0,
            "No args should succeed with usage info"
        );
        assert!(
            result.stdout.contains("OYA") || result.stdout.contains("help"),
            "Should show OYA name or help suggestion"
        );
        Ok(())
    }
}

#[cfg(test)]
mod bdd_workflows {
    use super::*;

    mod task_lifecycle {
        use super::*;

        #[test]
        fn given_operator_lists_tasks_when_no_tasks_exist_then_shows_empty_list(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["list"])?;
            assert_eq!(result.exit_code, 0, "List should succeed");
            assert!(
                result.stdout.contains("Tasks")
                    || result.stdout.contains("tasks")
                    || result.stdout.contains("total"),
                "Should show task count"
            );
            Ok(())
        }

        #[test]
        fn given_operator_requests_help_then_all_commands_documented(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["--help"])?;
            assert_eq!(result.exit_code, 0, "Help should succeed");
            assert!(result.stdout.contains("list"), "Should document list");
            assert!(result.stdout.contains("show"), "Should document show");
            assert!(result.stdout.contains("new"), "Should document new");
            assert!(result.stdout.contains("stage"), "Should document stage");
            assert!(result.stdout.contains("approve"), "Should document approve");
            assert!(
                result.stdout.contains("workspace"),
                "Should document workspace"
            );
            Ok(())
        }

        #[test]
        fn given_operator_provides_invalid_command_then_clear_error_message(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["invalid-command"])?;
            assert_ne!(result.exit_code, 0, "Invalid command should fail");
            assert!(
                result.stderr.contains("error") || result.stderr.contains("unrecognized"),
                "Should show clear error message"
            );
            Ok(())
        }
    }

    mod exit_code_contracts {
        use super::*;

        #[test]
        fn given_successful_command_then_exit_code_0() -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["list"])?;
            assert_eq!(result.exit_code, 0, "Successful list should return 0");
            Ok(())
        }

        #[test]
        fn given_validation_error_then_exit_code_2() -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["new", "--slug", ""])?;
            assert_eq!(result.exit_code, 2, "Validation error should return 2");
            Ok(())
        }

        #[test]
        fn given_not_found_error_then_exit_code_2() -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["show", "--slug", "nonexistent-task-xyz"])?;
            assert_eq!(result.exit_code, 2, "Not found should return 2");
            Ok(())
        }
    }

    mod json_output {
        use super::*;

        #[test]
        fn given_json_flag_then_outputs_valid_json() -> Result<(), Box<dyn std::error::Error>> {
            let result = run_oya_command(&["list", "--json"])?;
            assert_eq!(result.exit_code, 0, "JSON output should succeed");
            assert!(
                result.stdout.starts_with('{') || result.stdout.contains("\"tasks\""),
                "Should output valid JSON structure"
            );
            Ok(())
        }
    }

    mod idempotency {
        use super::*;

        #[test]
        fn given_list_command_run_multiple_times_then_same_result(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let result1 = run_oya_command(&["list"])?;
            let result2 = run_oya_command(&["list"])?;
            assert_eq!(
                result1.exit_code, result2.exit_code,
                "Exit codes should be consistent"
            );
            assert_eq!(
                result1.stdout, result2.stdout,
                "Output should be idempotent"
            );
            Ok(())
        }

        #[test]
        fn given_list_json_run_multiple_times_then_same_json(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let result1 = run_oya_command(&["list", "--json"])?;
            let result2 = run_oya_command(&["list", "--json"])?;
            assert_eq!(
                result1.exit_code, result2.exit_code,
                "Exit codes should be consistent"
            );
            assert_eq!(
                result1.stdout, result2.stdout,
                "JSON output should be idempotent"
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod adversarial_input {
    use super::*;

    #[test]
    fn given_very_long_slug_then_rejects_or_handles_gracefully(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let long_slug = "a".repeat(10000);
        let result = run_oya_command(&["new", "--slug", &long_slug])?;
        assert!(
            result.exit_code != 0 || result.stderr.contains("Error"),
            "Very long slug should be rejected or produce error"
        );
        Ok(())
    }

    #[test]
    fn given_unicode_slug_then_rejects_or_handles() -> Result<(), Box<dyn std::error::Error>> {
        let result = run_oya_command(&["new", "--slug", "日本語-タスク"])?;
        assert_ne!(result.exit_code, 0, "Unicode slug should be rejected");
        Ok(())
    }

    #[test]
    fn given_sql_injection_attempt_in_slug_then_rejects() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = run_oya_command(&["new", "--slug", "'; DROP TABLE tasks; --"])?;
        assert_ne!(result.exit_code, 0, "SQL injection should be rejected");
        Ok(())
    }

    #[test]
    fn given_control_characters_in_slug_then_rejects() -> Result<(), Box<dyn std::error::Error>> {
        let result = run_oya_command(&["new", "--slug", "test\x1btask"])?;
        assert_ne!(result.exit_code, 0, "Control characters should be rejected");
        Ok(())
    }

    #[test]
    fn given_command_with_double_dash_then_rejects_unknown(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = run_oya_command(&["list", "--", "--unknown"])?;
        assert_ne!(
            result.exit_code, 0,
            "Unknown args after -- should be rejected"
        );
        Ok(())
    }
}
