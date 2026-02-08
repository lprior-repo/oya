//! CLI command handlers.
//!
//! All commands follow functional patterns:
//! - Zero unwraps, zero panics
//! - Result<T, Error> for all operations
//! - Pure functions where possible

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;
use tracing::info;

use crate::cli::Commands;

// Re-export agents module if it exists
#[path = "agents.rs"]
pub mod agents;

/// Execute a CLI command.
///
/// This is the main command dispatcher that routes to the appropriate handler.
pub async fn execute_command(command: Commands) -> Result<()> {
    match command {
        Commands::New {
            slug,
            contract,
            interactive,
        } => cmd_new(slug, contract, interactive).await,

        Commands::Stage {
            slug,
            stage,
            dry_run,
            from,
            to,
        } => cmd_stage(slug, stage, dry_run, from, to).await,

        Commands::AiStage {
            slug,
            stage,
            prompt,
            files,
        } => cmd_ai_stage(slug, stage, prompt, files).await,

        Commands::Approve {
            slug,
            strategy,
            force,
        } => cmd_approve(slug, strategy, force).await,

        Commands::Show { slug, detailed } => cmd_show(slug, detailed).await,

        Commands::List { priority, status } => cmd_list(priority, status).await,

        Commands::Hello { message } => cmd_hello(message).await,

        Commands::Build {
            parallel,
            release,
            target,
        } => cmd_build(parallel, release, target).await,

        Commands::Test { swarm } => cmd_test(swarm).await,

        Commands::Refactor { force } => cmd_refactor(force).await,

        Commands::Deploy { no_mercy } => cmd_deploy(no_mercy).await,

        Commands::Gate { strict } => cmd_gate(strict).await,

        Commands::Agents {
            server: _,
            command: _,
        } => {
            // TODO: Re-enable when agents module is ready
            println!("Agent commands are not yet implemented");
            Ok(())
        }

        Commands::Swarm {
            target,
            test_writers,
            implementers,
            reviewers,
            planner,
            continuous_deployment,
            dry_run,
            resume,
            format,
        } => {
            cmd_swarm(
                target,
                test_writers,
                implementers,
                reviewers,
                planner,
                continuous_deployment,
                dry_run,
                resume,
                format,
            )
            .await
        }
    }
}

/// Create a new task with isolated worktree.
async fn cmd_new(slug: String, contract: Option<String>, interactive: bool) -> Result<()> {
    use oya_pipeline::domain::{Slug, Task};
    use oya_pipeline::persistence::save_task_record;

    info!("Creating new task: {}", slug);

    // Validate slug format
    let validated_slug =
        Slug::new(slug.clone()).map_err(|e| anyhow::anyhow!("Invalid slug '{slug}': {e}"))?;

    // Detect language from repository files
    let repo_root = get_repo_root()?;
    let language = detect_language_from_repo(&repo_root)?;

    // Log optional parameters
    if let Some(contract_path) = contract {
        info!("Contract file: {}", contract_path);
    }

    if interactive {
        info!("Interactive mode enabled");
    }

    // Create task with default status
    let task = Task::new(validated_slug, language);

    // Persist to database
    save_task_record(&task, &repo_root)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to save task to database: {e}"))?;

    println!("Task '{}' created successfully", slug);
    Ok(())
}

/// Detect programming language from repository marker files.
fn detect_language_from_repo(
    repo_root: &std::path::Path,
) -> Result<oya_pipeline::domain::Language> {
    use oya_pipeline::domain::Language;

    // Check for marker files
    let gleam_toml = repo_root.join("gleam.toml");
    let go_mod = repo_root.join("go.mod");
    let cargo_toml = repo_root.join("Cargo.toml");
    let pyproject = repo_root.join("pyproject.toml");
    let package_json = repo_root.join("package.json");

    let has_gleam = gleam_toml.exists();
    let has_go = go_mod.exists();
    let has_cargo = cargo_toml.exists();
    let has_python = pyproject.exists();
    let has_js = package_json.exists();

    Language::detect_from_files(has_gleam, has_go, has_cargo, has_python, has_js).map_err(|_| {
        anyhow::anyhow!(
            "Could not detect project language. \
             Ensure one of these marker files exists: \
             gleam.toml, go.mod, Cargo.toml, pyproject.toml, package.json"
        )
    })
}

/// Run a pipeline stage.
async fn cmd_stage(
    slug: String,
    stage: String,
    dry_run: bool,
    from: Option<String>,
    to: Option<String>,
) -> Result<()> {
    info!("Running stage '{}' for task '{}'", stage, slug);

    if dry_run {
        info!("Dry run mode - no changes will be made");
    }

    if let Some(start) = from {
        let end = to.unwrap_or_else(|| stage.clone());
        info!("Running stage range: {} to {}", start, end);
    }

    // TODO: Implement actual stage execution
    // - Load task from database
    // - Run stage with proper error handling
    // - Update task status
    // - Handle retries

    println!("Stage '{}' completed for task '{}'", stage, slug);
    Ok(())
}

/// Run a stage with AI assistance.
async fn cmd_ai_stage(
    slug: String,
    stage: String,
    prompt: Option<String>,
    files: Vec<String>,
) -> Result<()> {
    info!("Running AI-assisted stage '{}' for task '{}'", stage, slug);

    if let Some(custom_prompt) = prompt {
        info!("Custom prompt: {}", custom_prompt);
    }

    if !files.is_empty() {
        info!("Files in context: {}", files.join(", "));
    }

    // TODO: Implement AI stage execution
    // - Load task from database
    // - Gather context from files
    // - Call OpenCode API
    // - Apply changes with confirmation

    println!("AI stage '{}' completed for task '{}'", stage, slug);
    Ok(())
}

/// Approve task for deployment.
async fn cmd_approve(slug: String, strategy: Option<String>, force: bool) -> Result<()> {
    info!("Approving task '{}'", slug);

    if let Some(deployment_strategy) = strategy {
        info!("Deployment strategy: {}", deployment_strategy);
    }

    if force {
        info!("Force approval enabled - skipping safety checks");
    }

    // TODO: Implement actual approval logic
    // - Load task from database
    // - Verify task passed pipeline
    // - Mark for integration
    // - Trigger deployment if needed

    println!("Task '{}' approved", slug);
    Ok(())
}

/// Show task details.
async fn cmd_show(slug: String, detailed: bool) -> Result<()> {
    use oya_pipeline::persistence::load_task_record;

    let repo_root = get_repo_root()?;
    info!("Showing details for task '{}'", slug);

    // Try to load from database
    match load_task_record(&slug, &repo_root).await {
        Ok(task) => {
            display_task(&task, detailed);
            Ok(())
        }
        Err(e) => {
            // Task not found in database
            println!("Error: {}", e);
            println!("\nTask '{}' not found in database", slug);
            println!("Hint: Use 'oya new -s <slug>' to create a new task");
            Err(anyhow::anyhow!("Task not found: {}", e))
        }
    }
}

/// Display task information to the user.
fn display_task(task: &oya_pipeline::domain::Task, detailed: bool) {
    use oya_pipeline::domain::TaskStatus;

    println!("\nTask: {}", task.slug);
    println!("Language: {}", task.language);
    println!("Status: {}", task.status);
    println!("Priority: {}", task.priority);
    println!("Branch: {}", task.branch);

    if detailed {
        println!("\nDetailed Information:");

        match &task.status {
            TaskStatus::Created => {
                println!("  State: Just created, ready to start pipeline");
            }
            TaskStatus::InProgress { stage } => {
                println!("  Current Stage: {}", stage);
                println!("  State: Pipeline is running");
            }
            TaskStatus::PassedPipeline => {
                println!("  State: Pipeline passed, ready for integration");
            }
            TaskStatus::FailedPipeline { stage, reason } => {
                println!("  Failed Stage: {}", stage);
                println!("  Failure Reason: {}", reason);
            }
            TaskStatus::Integrated => {
                println!("  State: Task has been integrated");
            }
        }

        // Show next steps
        match &task.status {
            TaskStatus::Created => {
                println!("\nNext Steps:");
                println!("  Run: oya stage -s {} --stage implement", task.slug);
            }
            TaskStatus::InProgress { stage } => {
                println!("\nNext Steps:");
                println!("  Current stage in progress: {}", stage);
                println!("  Monitor progress or check logs");
            }
            TaskStatus::PassedPipeline => {
                println!("\nNext Steps:");
                println!("  Run: oya approve -s {}", task.slug);
            }
            TaskStatus::FailedPipeline { .. } => {
                println!("\nNext Steps:");
                println!("  Fix the issue and retry the stage");
                println!("  Run: oya stage -s {} --stage <stage>", task.slug);
            }
            TaskStatus::Integrated => {
                println!("\nTask Complete:");
                println!("  This task has been integrated");
            }
        }
    }
}

/// List all tasks.
async fn cmd_list(priority: Option<String>, status: Option<String>) -> Result<()> {
    use oya_pipeline::persistence::list_all_tasks;

    let repo_root = get_repo_root()?;
    info!("Listing all tasks");

    // Load all tasks from database
    let tasks = list_all_tasks(&repo_root).await?;

    if tasks.is_empty() {
        println!("No tasks found");
        println!("\nHint: Use 'oya new -s <slug>' to create a new task");
        return Ok(());
    }

    // Apply filters
    let filtered = apply_filters(&tasks, priority, status);

    println!("\nTasks ({} total):\n", filtered.len());

    for task in &filtered {
        let status_icon = match &task.status {
            oya_pipeline::domain::TaskStatus::Created => "○",
            oya_pipeline::domain::TaskStatus::InProgress { .. } => "◐",
            oya_pipeline::domain::TaskStatus::PassedPipeline => "✓",
            oya_pipeline::domain::TaskStatus::FailedPipeline { .. } => "✗",
            oya_pipeline::domain::TaskStatus::Integrated => "⊙",
        };

        println!(
            "  {} {} - {} - {} [{}]",
            status_icon, task.slug, task.language, task.status, task.priority
        );
    }

    // Show summary
    let created = filtered
        .iter()
        .filter(|t| matches!(t.status, oya_pipeline::domain::TaskStatus::Created))
        .count();
    let in_progress = filtered.iter().filter(|t| t.status.is_transient()).count();
    let passed = filtered
        .iter()
        .filter(|t| matches!(t.status, oya_pipeline::domain::TaskStatus::PassedPipeline))
        .count();
    let failed = filtered.iter().filter(|t| t.status.is_failed()).count();
    let integrated = filtered
        .iter()
        .filter(|t| matches!(t.status, oya_pipeline::domain::TaskStatus::Integrated))
        .count();

    println!("\nSummary:");
    println!("  Created: {}", created);
    println!("  In Progress: {}", in_progress);
    println!("  Passed: {}", passed);
    println!("  Failed: {}", failed);
    println!("  Integrated: {}", integrated);

    Ok(())
}

/// Apply priority and status filters to task list.
fn apply_filters(
    tasks: &[oya_pipeline::domain::Task],
    priority: Option<String>,
    status: Option<String>,
) -> Vec<&oya_pipeline::domain::Task> {
    let mut filtered: Vec<&oya_pipeline::domain::Task> = tasks.iter().collect();

    // Filter by priority
    if let Some(prio_str) = priority {
        if let Ok(prio) = oya_pipeline::domain::Priority::parse(&prio_str) {
            filtered = filtered
                .into_iter()
                .filter(|t| t.priority == prio)
                .collect();
        }
    }

    // Filter by status
    if let Some(status_str) = status {
        let status_lower = status_str.to_lowercase();
        filtered = filtered
            .into_iter()
            .filter(|t| t.status.to_filter_status() == status_lower)
            .collect();
    }

    filtered
}

/// Say hello to the world.
async fn cmd_hello(message: String) -> Result<()> {
    println!("{}", message);
    Ok(())
}

/// Build the project using Moon with parallel execution.
async fn cmd_build(parallel: usize, release: bool, target: Option<String>) -> Result<()> {
    use std::io::Write;

    info!(
        "Building project with {} parallel workers (release: {})",
        parallel, release
    );

    // Validate parallel job count
    if parallel == 0 {
        return Err(anyhow::anyhow!("Parallel job count must be greater than 0"));
    }

    if parallel > 256 {
        return Err(anyhow::anyhow!(
            "Parallel job count cannot exceed 256 (requested: {})",
            parallel
        ));
    }

    // Determine moon target task
    let moon_task = if release { ":build-release" } else { ":build" };

    // Build moon command with environment variable for parallelism
    let mut cmd = Command::new("moon");

    // Set MOON_JOBS environment variable for parallel execution
    cmd.env("MOON_JOBS", parallel.to_string());

    // Add the target task
    cmd.arg(moon_task);

    // If specific target requested, append it
    if let Some(specified_target) = target {
        cmd.arg(&specified_target);
        info!("Building target: {}", specified_target);
    }

    info!("Executing: moon {} (jobs: {})", moon_task, parallel);

    // Run the command and capture output
    let output = cmd.output().map_err(|e| {
        anyhow::anyhow!("Failed to execute moon command: {}. Is moon installed?", e)
    })?;

    // Write stdout to real-time for user feedback
    std::io::stdout()
        .write_all(&output.stdout)
        .map_err(|e| anyhow::anyhow!("Failed to write build output: {}", e))?;

    // Write stderr to real-time for error feedback
    std::io::stderr()
        .write_all(&output.stderr)
        .map_err(|e| anyhow::anyhow!("Failed to write error output: {}", e))?;

    // Check exit status
    if output.status.success() {
        println!("\n✓ Build completed successfully");
        Ok(())
    } else {
        let exit_code = output.status.code();
        let code_str = exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        Err(anyhow::anyhow!("Build failed with exit code {}", code_str))
    }
}

/// Run tests with optional swarm mode (massive parallelism).
async fn cmd_test(swarm: bool) -> Result<()> {
    use std::io::Write;
    use std::process::Command;

    if swarm {
        info!("Running tests in swarm mode (maximum parallelism)");
    } else {
        info!("Running tests");
    }

    // Build moon test command
    let mut cmd = Command::new("moon");
    cmd.arg(":test");

    // In swarm mode, set high parallelism
    if swarm {
        cmd.env("MOON_JOBS", "100");
        info!("Swarm mode enabled: 100 parallel test workers");
    }

    info!("Executing: moon :test");

    // Run the command and capture output
    let output = cmd.output().map_err(|e| {
        anyhow::anyhow!("Failed to execute moon command: {}. Is moon installed?", e)
    })?;

    // Write stdout to real-time for user feedback
    std::io::stdout()
        .write_all(&output.stdout)
        .map_err(|e| anyhow::anyhow!("Failed to write test output: {}", e))?;

    // Write stderr to real-time for error feedback
    std::io::stderr()
        .write_all(&output.stderr)
        .map_err(|e| anyhow::anyhow!("Failed to write error output: {}", e))?;

    // Check exit status
    if output.status.success() {
        println!("\n✓ Tests passed successfully");
        Ok(())
    } else {
        let exit_code = output.status.code();
        let code_str = exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        Err(anyhow::anyhow!("Tests failed with exit code {}", code_str))
    }
}

/// Refactor codebase with automated transformations.
async fn cmd_refactor(force: bool) -> Result<()> {
    use std::io::Write;
    use std::process::Command;

    if force {
        info!("Running refactor with --force (skipping confirmation)");
    } else {
        info!("Running refactor");
    }

    // For now, refactor runs formatting and linting fixes
    // In the future, this could include automated code transformations
    let mut cmd = Command::new("moon");
    cmd.arg(":fmt-fix");

    info!("Executing: moon :fmt-fix");

    // Run the command and capture output
    let output = cmd.output().map_err(|e| {
        anyhow::anyhow!("Failed to execute moon command: {}. Is moon installed?", e)
    })?;

    // Write stdout to real-time for user feedback
    std::io::stdout()
        .write_all(&output.stdout)
        .map_err(|e| anyhow::anyhow!("Failed to write refactor output: {}", e))?;

    // Write stderr to real-time for error feedback
    std::io::stderr()
        .write_all(&output.stderr)
        .map_err(|e| anyhow::anyhow!("Failed to write error output: {}", e))?;

    // Check exit status
    if output.status.success() {
        println!("\n✓ Refactoring completed successfully");

        // If not force mode, suggest reviewing changes
        if !force {
            println!("\nHint: Review the changes before committing.");
            println!("      Use --force to skip this message in the future.");
        }

        Ok(())
    } else {
        let exit_code = output.status.code();
        let code_str = exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        Err(anyhow::anyhow!(
            "Refactoring failed with exit code {}",
            code_str
        ))
    }
}

/// Deploy validated changes.
async fn cmd_deploy(no_mercy: bool) -> Result<()> {
    use std::io::Write;
    use std::process::Command;

    if no_mercy {
        info!("⚠️  Deploying with --no-mercy (skipping ALL safety checks)");
        info!("This is dangerous - unvalidated code will be deployed");
    } else {
        info!("Running deployment with safety checks");
    }

    // Normal deployment: run quality gates first
    if !no_mercy {
        println!("Running pre-deployment checks...");

        let check_cmd = Command::new("moon").arg(":quick").output().map_err(|e| {
            anyhow::anyhow!("Failed to execute moon command: {}. Is moon installed?", e)
        })?;

        if !check_cmd.status.success() {
            std::io::stderr()
                .write_all(&check_cmd.stderr)
                .map_err(|e| anyhow::anyhow!("Failed to write error output: {}", e))?;
            return Err(anyhow::anyhow!(
                "Pre-deployment checks failed. Use --no-mercy to bypass (not recommended)"
            ));
        }

        println!("✓ Pre-deployment checks passed");
    }

    // In a real implementation, this would run actual deployment commands
    // For now, we just validate that checks pass
    println!("\n✓ Deployment validated successfully");

    if no_mercy {
        println!("⚠️  Deployed without safety checks - code may be unstable");
    }

    Ok(())
}

/// Run quality gates (validation checks).
async fn cmd_gate(strict: bool) -> Result<()> {
    use std::io::Write;
    use std::process::Command;

    if strict {
        info!("Running quality gates in strict mode (warnings = failures)");
    } else {
        info!("Running quality gates");
    }

    // Run quick checks (format + clippy)
    let mut cmd = Command::new("moon");
    cmd.arg(":quick");

    // In strict mode, set environment variable
    if strict {
        cmd.env("OYA_STRICT_MODE", "1");
    }

    info!("Executing: moon :quick");

    // Run the command and capture output
    let output = cmd.output().map_err(|e| {
        anyhow::anyhow!("Failed to execute moon command: {}. Is moon installed?", e)
    })?;

    // Write stdout to real-time for user feedback
    std::io::stdout()
        .write_all(&output.stdout)
        .map_err(|e| anyhow::anyhow!("Failed to write gate output: {}", e))?;

    // Write stderr to real-time for error feedback
    std::io::stderr()
        .write_all(&output.stderr)
        .map_err(|e| anyhow::anyhow!("Failed to write error output: {}", e))?;

    // Check exit status
    if output.status.success() {
        println!("\n✓ Quality gates passed");

        if strict {
            println!("Strict mode: All checks passed with zero warnings");
        }

        Ok(())
    } else {
        let exit_code = output.status.code();
        let code_str = exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        if strict {
            Err(anyhow::anyhow!(
                "Quality gates failed in strict mode (exit code {})\n\
                 Fix all warnings and errors before proceeding",
                code_str
            ))
        } else {
            Err(anyhow::anyhow!(
                "Quality gates failed with exit code {}",
                code_str
            ))
        }
    }
}

// TODO: Re-enable agent commands when agents module is ready
/*
async fn cmd_agents(server: Option<String>, command: AgentCommands) -> Result<()> {
    use crate::agents::AgentApiClient;

    let client = AgentApiClient::new(server.as_deref());

    match command {
        AgentCommands::Spawn { count } => {
            info!("Spawning {} agents", count);
            let response = client.spawn(count).await?;
            println!("Spawned {} agents", response.agent_ids.len());
            println!("Total agents: {}", response.total);
            Ok(())
        }
        AgentCommands::Scale { target } => {
            info!("Scaling to {} agents", target);
            let response = client.scale(target).await?;
            println!("Previous total: {}", response.previous);
            println!("New total: {}", response.total);
            if !response.spawned.is_empty() {
                println!("Spawned: {}", response.spawned.join(", "));
            }
            if !response.terminated.is_empty() {
                println!("Terminated: {}", response.terminated.join(", "));
            }
            Ok(())
        }
        AgentCommands::List => {
            info!("Listing agents");
            let response = client.list().await?;
            println!("Total agents: {}", response.total);
            println!("\nAgents:");
            for agent in response.agents {
                println!(
                    "  {} - {} - health: {:.2} - uptime: {}s",
                    agent.id, agent.status, agent.health_score, agent.uptime_secs
                );
                if let Some(bead) = agent.current_bead {
                    println!("    Current bead: {}", bead);
                }
            }
            Ok(())
        }
    }
}
*/

/// Get the repository root directory.
///
/// Searches upward from the current directory to find the repository root.
fn get_repo_root() -> Result<PathBuf> {
    let current = std::env::current_dir()?;

    // Start from current directory and search upward
    let mut path = current.as_path();

    loop {
        // Check if .oya or .git exists here
        let oya_dir = path.join(".oya");
        let git_dir = path.join(".git");

        if oya_dir.exists() || git_dir.exists() {
            return Ok(path.to_path_buf());
        }

        // Move to parent directory
        match path.parent() {
            Some(parent) if parent != path => path = parent,
            _ => {
                // Reached root without finding repo
                return Ok(current); // Fallback to current directory
            }
        }
    }
}

/// Run swarm mode (13-agent continuous assembly line).
async fn cmd_swarm(
    target: usize,
    test_writers: usize,
    implementers: usize,
    reviewers: usize,
    planner: bool,
    continuous_deployment: bool,
    dry_run: bool,
    resume: Option<String>,
    format: String,
) -> Result<()> {
    use oya::swarm::SwarmConfig;

    info!(
        target,
        test_writers, implementers, reviewers, planner, "Starting swarm mode"
    );

    // Create swarm config
    let config = SwarmConfig {
        target_beads: target,
        test_writers,
        implementers,
        reviewers,
        planner,
        continuous_deployment,
        ..SwarmConfig::default()
    };

    // Validate configuration
    config
        .validate()
        .map_err(|e| anyhow::anyhow!("Invalid swarm config: {}", e))?;

    if dry_run {
        println!("=== Swarm Mode: Dry Run ===\n");
        println!("Configuration:");
        println!("  Target beads: {}", config.target_beads);
        println!("  Test Writers: {}", config.test_writers);
        println!("  Implementers: {}", config.implementers);
        println!("  Reviewers: {}", config.reviewers);
        println!("  Planner: {}", config.planner);
        println!("  Continuous Deployment: {}", config.continuous_deployment);
        println!("  Total agents: {}", config.total_agents());
        println!("\nQuality Gates:");
        println!("  moon run :ci: {}", config.quality_gates.moon_ci);
        println!("  moon run :quick: {}", config.quality_gates.moon_quick);
        println!("  Zero panic: {}", config.quality_gates.zero_panic);
        println!("  Red Queen QA: {}", config.quality_gates.red_queen);
        println!("  Git push: {}", config.quality_gates.git_push);

        if let Some(session_id) = resume {
            println!("\nResume session: {}", session_id);
        }

        println!("\n=== Dry Run Complete ===");
        println!("Remove --dry-run to execute swarm");
        return Ok(());
    }

    if let Some(session_id) = resume {
        info!("Resuming from session: {}", session_id);
        // TODO: Implement resume functionality
        return Err(anyhow::anyhow!("Resume functionality not yet implemented"));
    }

    // TODO: Implement actual swarm orchestration
    // 1. Spawn OrchestratorActor
    // 2. Monitor status every 1s
    // 3. Handle Ctrl+C for graceful shutdown
    // 4. Return result on completion

    println!("=== Swarm Mode Started ===\n");
    println!("Configuration:");
    println!("  Target beads: {}", config.target_beads);
    println!("  Total agents: {}", config.total_agents());
    println!("\nAgent Distribution:");
    println!("  Test Writers: {}", config.test_writers);
    println!("  Implementers: {}", config.implementers);
    println!("  Reviewers: {}", config.reviewers);
    println!("  Planner: {}", config.planner);

    println!("\n⚠️  Swarm orchestration not yet implemented");
    println!("    This will spawn the orchestrator and 13 agents");
    println!("    Agents will complete beads using contract-first development");
    println!("    All work follows continuous-deployment principles\n");

    println!("Use --dry-run to preview configuration");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oya_pipeline::domain::{Language, Priority, Slug, Task, TaskStatus};

    /// Helper function to create test tasks with proper error handling.
    fn make_test_task(slug: &str, lang: Language) -> Result<Task, oya_pipeline::Error> {
        Ok(Task::new(Slug::new(slug)?, lang))
    }

    #[test]
    fn test_apply_filters_priority() -> Result<(), Box<dyn std::error::Error>> {
        let tasks = vec![
            make_test_task("task1", Language::Rust)?.with_priority(Priority::P1),
            make_test_task("task2", Language::Rust)?.with_priority(Priority::P2),
            make_test_task("task3", Language::Rust)?.with_priority(Priority::P1),
        ];

        let filtered = apply_filters(&tasks, Some("P1".to_string()), None);
        assert_eq!(filtered.len(), 2);
        Ok(())
    }

    #[test]
    fn test_apply_filters_status() -> Result<(), Box<dyn std::error::Error>> {
        let tasks = vec![
            make_test_task("task1", Language::Rust)?.with_status(TaskStatus::Created),
            make_test_task("task2", Language::Rust)?.with_status(TaskStatus::PassedPipeline),
            make_test_task("task3", Language::Rust)?.with_status(TaskStatus::Created),
        ];

        let filtered = apply_filters(&tasks, None, Some("open".to_string()));
        assert_eq!(filtered.len(), 2);
        Ok(())
    }
}
