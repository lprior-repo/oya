//! CLI command handlers.
//!
//! All commands follow functional patterns:
//! - Zero unwraps, zero panics
//! - Result<T, Error> for all operations
//! - Pure functions where possible

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::path::PathBuf;

use anyhow::Result;
use tracing::info;

use crate::cli::{AgentCommands, Commands};

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

        Commands::Agents { server, command } => cmd_agents(server, command).await,
    }
}

/// Create a new task with isolated worktree.
async fn cmd_new(slug: String, contract: Option<String>, interactive: bool) -> Result<()> {
    info!("Creating new task: {}", slug);

    if let Some(contract_path) = contract {
        info!("Contract file: {}", contract_path);
    }

    if interactive {
        info!("Interactive mode enabled");
    }

    // TODO: Implement actual task creation
    // - Validate slug
    // - Detect language
    // - Create workspace using zjj
    // - Initialize in database

    println!("Task '{}' created successfully", slug);
    Ok(())
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
    use oya_pipeline::domain::Priority;
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
    let in_progress = filtered
        .iter()
        .filter(|t| t.status.is_transient())
        .count();
    let passed = filtered
        .iter()
        .filter(|t| matches!(t.status, oya_pipeline::domain::TaskStatus::PassedPipeline))
        .count();
    let failed = filtered
        .iter()
        .filter(|t| t.status.is_failed())
        .count();
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

/// Manage agent pool.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_filters_priority() {
        use oya_pipeline::domain::{Language, Priority, Slug, Task, TaskStatus};

        let tasks = vec![
            Task::new(Slug::new("task1").unwrap(), Language::Rust).with_priority(Priority::P1),
            Task::new(Slug::new("task2").unwrap(), Language::Rust).with_priority(Priority::P2),
            Task::new(Slug::new("task3").unwrap(), Language::Rust).with_priority(Priority::P1),
        ];

        let filtered = apply_filters(&tasks, Some("P1".to_string()), None);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_apply_filters_status() {
        use oya_pipeline::domain::{Language, Slug, Task, TaskStatus};

        let tasks = vec![
            Task::new(Slug::new("task1").unwrap(), Language::Rust)
                .with_status(TaskStatus::Created),
            Task::new(Slug::new("task2").unwrap(), Language::Rust)
                .with_status(TaskStatus::PassedPipeline),
            Task::new(Slug::new("task3").unwrap(), Language::Rust)
                .with_status(TaskStatus::Created),
        ];

        let filtered = apply_filters(&tasks, None, Some("open".to_string()));
        assert_eq!(filtered.len(), 2);
    }
}
