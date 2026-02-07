//! Gleam language stage execution.

use std::path::Path;

use crate::{
    error::{Error, Result},
    file_discovery::find_gleam_files,
    process::{command_exists, run_command},
};

/// Execute a Gleam pipeline stage.
pub fn execute_gleam_stage(stage_name: &str, cwd: &Path) -> Result<()> {
    match stage_name {
        "implement" => gleam_implement(cwd),
        "unit-test" => gleam_unit_test(cwd),
        "coverage" => gleam_coverage(cwd),
        "lint" => gleam_lint(cwd),
        "static" => gleam_static(cwd),
        "integration" => gleam_integration(cwd),
        "security" => gleam_security(cwd),
        "review" => gleam_review(cwd),
        "accept" => gleam_accept(cwd),
        other => Err(Error::UnknownStage {
            name: other.to_string(),
        }),
    }
}

/// Get preview for Gleam stage.
pub fn get_gleam_preview(stage_name: &str) -> (String, u64) {
    match stage_name {
        "implement" => ("gleam build".to_string(), 5000),
        "unit-test" => ("gleam test".to_string(), 10000),
        "coverage" => ("find . -name '*_test.gleam'".to_string(), 2000),
        "lint" => ("gleam format --check .".to_string(), 3000),
        "static" => ("gleam check".to_string(), 5000),
        "integration" => ("gleam test".to_string(), 15000),
        "security" => ("gleam deps download".to_string(), 5000),
        "review" => ("grep -r TODO/FIXME".to_string(), 2000),
        "accept" => (
            "gleam build && gleam test && gleam format --check".to_string(),
            20000,
        ),
        _ => ("unknown".to_string(), 0),
    }
}

fn gleam_implement(cwd: &Path) -> Result<()> {
    check_gleam_exists()?;
    run_command("gleam", &["build"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Gleam", "implement", "Code does not compile"))
}

fn gleam_unit_test(cwd: &Path) -> Result<()> {
    check_gleam_exists()?;
    run_command("gleam", &["test"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Gleam", "unit-test", "Tests failed"))
}

fn gleam_coverage(cwd: &Path) -> Result<()> {
    // Use memoized file discovery to find test files
    let test_files = find_gleam_files(cwd)?
        .into_iter()
        .filter(|p| {
            // Functional pattern: map_or instead of unwrap_or
            let file_name = p.file_name().and_then(|n| n.to_str()).map_or("", |s| s);
            file_name.ends_with("_test.gleam") || file_name.starts_with("test_")
        })
        .collect::<Vec<_>>();

    if test_files.is_empty() {
        return Err(Error::stage_failed(
            "Gleam",
            "coverage",
            "No test files found",
        ));
    }

    // Convert paths to strings for find command
    let _file_paths: Vec<String> = test_files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    let result = run_command("find", &[".", "-name", "*.gleam"], cwd)?;

    result.check_success().map_err(|_| {
        Error::stage_failed(
            "Gleam",
            "coverage",
            format!("find failed with code: {}", result.exit_code),
        )
    })
}

fn gleam_lint(cwd: &Path) -> Result<()> {
    run_command("gleam", &["format", "--check", "."], cwd)?
        .check_success()
        .map_err(|_| {
            Error::stage_failed(
                "Gleam",
                "lint",
                "Code formatting issues. Run: gleam format .",
            )
        })
}

fn gleam_static(cwd: &Path) -> Result<()> {
    run_command("gleam", &["check"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Gleam", "static", "Type checking failed"))
}

fn gleam_integration(cwd: &Path) -> Result<()> {
    run_command("gleam", &["test"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Gleam", "integration", "Integration tests failed"))
}

fn gleam_security(cwd: &Path) -> Result<()> {
    run_command("gleam", &["deps", "download"], cwd)?
        .check_success()
        .map_err(|_| Error::stage_failed("Gleam", "security", "Dependency validation failed"))
}

fn gleam_review(cwd: &Path) -> Result<()> {
    // Use memoized file discovery to get Gleam files
    let gleam_files = find_gleam_files(cwd)?;

    if gleam_files.is_empty() {
        return Ok(()); // No Gleam files to review
    }

    // Convert paths to strings for grep
    let file_paths: Vec<String> = gleam_files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    let mut args: Vec<String> = vec!["-r".to_string(), r"TODO\|FIXME\|XXX\|HACK".to_string()];
    args.extend(file_paths.iter().map(|p| p.to_string()));
    args.push(".".to_string());

    let args_slice: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let result = run_command("grep", &args_slice, cwd)?;

    match result.exit_code {
        0 => Err(Error::stage_failed(
            "Gleam",
            "review",
            "TODO/FIXME/XXX/HACK markers found",
        )),
        1 => Ok(()), // No matches found - good
        code => Err(Error::stage_failed(
            "Gleam",
            "review",
            format!("grep failed with code: {code}"),
        )),
    }
}

fn gleam_accept(cwd: &Path) -> Result<()> {
    gleam_implement(cwd)?;
    gleam_unit_test(cwd)?;
    gleam_lint(cwd)
}

fn check_gleam_exists() -> Result<()> {
    if !command_exists("gleam")? {
        return Err(Error::CommandNotFound {
            cmd: "gleam".to_string(),
        });
    }
    Ok(())
}
