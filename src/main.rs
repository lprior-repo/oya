//! OYA CLI - Main entry point
//!
//! Storm goddess of transformation. 100x developer throughput with AI agent swarms.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

mod cli;

use std::path::Path;
use std::time::Instant;

use clap::Parser;
use oya_pipeline::{
    AIStageExecutor, Result, audit,
    domain::{Slug, Task, TaskStatus, filter_stages, get_stage},
    persistence::{list_all_tasks, load_task_record, save_task_record},
    repo::{detect_language, detect_repo_root},
    stages::{execute_stage, execute_stages_dry_run},
};
use oya_opencode::PhaseInput;

use crate::cli::{Cli, Commands};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New {
            slug,
            contract,
            interactive,
        } => execute_new(&slug, contract.as_deref(), interactive).await,

        Commands::Stage {
            slug,
            stage,
            dry_run,
            from,
            to,
        } => execute_stage_command(&slug, &stage, dry_run, from.as_deref(), to.as_deref()).await,

        Commands::AiStage {
            slug,
            stage,
            prompt,
            files,
        } => execute_ai_stage(&slug, &stage, prompt.as_deref(), &files).await,

        Commands::Approve {
            slug,
            strategy,
            force,
        } => execute_approve(&slug, strategy.as_deref(), force).await,

        Commands::Show { slug, detailed } => execute_show(&slug, detailed).await,

        Commands::List { priority, status } => {
            execute_list(priority.as_deref(), status.as_deref()).await
        }
    }
}

async fn execute_new(slug: &str, contract: Option<&str>, interactive: bool) -> Result<()> {
    let validated_slug = Slug::new(slug)?;
    let repo_root = detect_repo_root()?;
    let lang = detect_language(&repo_root)?;

    let task = Task::new(validated_slug, lang);
    let branch = task.branch.clone();
    save_task_record(&task, &repo_root).await?;

    // Log task creation to audit trail
    let _ = audit::log_task_created(&repo_root, slug, lang.as_str(), &branch);

    let contract_info = contract.map_or(String::new(), |c| format!("\nContract: {c}"));
    let interactive_info = if interactive {
        "\nInteractive: enabled"
    } else {
        ""
    };

    println!(
        "Created: {}\nBranch:  {}\nLanguage: {}{}{}",
        slug,
        branch,
        lang.as_str(),
        contract_info,
        interactive_info
    );

    Ok(())
}

async fn execute_stage_command(
    slug: &str,
    stage_name: &str,
    dry_run: bool,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<()> {
    let _ = Slug::new(slug)?;
    let repo_root = detect_repo_root()?;
    let task = load_task_record(slug, &repo_root).await?;

    // Get stages to run based on from/to range
    let stages_to_run = match (from, to) {
        (Some(f), Some(t)) => filter_stages(f, t)?,
        (Some(f), None) => filter_stages(f, stage_name)?,
        (None, Some(t)) => filter_stages(stage_name, t)?,
        (None, None) => vec![get_stage(stage_name)?],
    };

    if dry_run {
        let previews = execute_stages_dry_run(&stages_to_run, task.language);
        for preview in previews {
            println!("DRY RUN: {}", preview.name);
            println!("  Command: {}", preview.command);
            println!("  Estimated: {}ms", preview.estimated_duration);
        }
        return Ok(());
    }

    for stage in stages_to_run {
        execute_single_stage(slug, &stage.name, &task, &repo_root).await?;
    }

    Ok(())
}

async fn execute_single_stage(
    slug: &str,
    stage_name: &str,
    task: &Task,
    repo_root: &std::path::Path,
) -> Result<()> {
    let _ = get_stage(stage_name)?;

    // Log stage start
    let _ = audit::log_stage_started(repo_root, slug, stage_name, 1);

    let start_time = Instant::now();
    let result = execute_stage(stage_name, task.language, Path::new("."));

    match result {
        Ok(()) => {
            #[allow(clippy::cast_possible_truncation)]
            let duration_ms = start_time.elapsed().as_millis() as i64;
            let _ = audit::log_stage_passed(repo_root, slug, stage_name, duration_ms);
            println!("\u{2713} {stage_name} passed ({duration_ms}ms)");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

async fn execute_ai_stage(
    slug: &str,
    stage_name: &str,
    prompt: Option<&str>,
    files: &[String],
) -> Result<()> {
    let _ = Slug::new(slug)?;
    let repo_root = detect_repo_root()?;
    let task = load_task_record(slug, &repo_root).await?;

    // Get the stage definition
    let stage = get_stage(stage_name)?;

    // Create AI executor
    let ai_executor = AIStageExecutor::new()?;

    // Check if OpenCode is available
    if !ai_executor.is_available().await {
        return Err(oya_pipeline::Error::InvalidRecord {
            reason: "OpenCode CLI is not available. Please install opencode.".to_string(),
        });
    }

    // Check if this stage can be executed by AI
    if !ai_executor.can_execute(&stage) {
        return Err(oya_pipeline::Error::InvalidRecord {
            reason: format!(
                "Stage '{}' cannot be executed by AI. Supported stages: implement, test, review, refactor, document",
                stage_name
            ),
        });
    }

    // Build phase input from prompt and files
    let input = if prompt.is_some() || !files.is_empty() {
        let mut phase_input = PhaseInput::default();

        if let Some(p) = prompt {
            phase_input.text = Some(p.to_string());
        }

        if !files.is_empty() {
            phase_input.files = files.to_vec();
        }

        Some(phase_input)
    } else {
        None
    };

    // Log stage start
    let _ = audit::log_stage_started(&repo_root, slug, stage_name, 1);

    println!("Executing '{}' with AI assistance...", stage_name);

    // Execute stage with AI
    let start_time = Instant::now();
    let result = ai_executor.execute_stage(&task, &stage, input).await?;

    if result.passed {
        #[allow(clippy::cast_possible_truncation)]
        let duration_ms = start_time.elapsed().as_millis() as i64;
        let _ = audit::log_stage_passed(&repo_root, slug, stage_name, duration_ms);
        println!("\u{2713} {} passed ({} ms)", stage_name, duration_ms);
        println!("\nAI completed the stage successfully.");
    } else {
        let error_msg = result.error.as_deref().unwrap_or("unknown error");
        println!("\u{2717} {} failed: {}", stage_name, error_msg);
        return Err(oya_pipeline::Error::InvalidRecord {
            reason: format!("AI stage execution failed: {}", error_msg),
        });
    }

    Ok(())
}

async fn execute_approve(slug: &str, strategy: Option<&str>, force: bool) -> Result<()> {
    let _ = Slug::new(slug)?;
    let repo_root = detect_repo_root()?;
    let task = load_task_record(slug, &repo_root).await?;

    if !force {
        check_at_least_one_stage_passed(&repo_root, slug)?;
    }

    let strategy_str = strategy.unwrap_or("immediate");

    let approved_task = task.with_status(TaskStatus::Integrated);
    save_task_record(&approved_task, &repo_root).await?;

    // Log task approval to audit trail
    let _ = audit::log_task_approved(&repo_root, slug, strategy_str);

    println!("\u{2713} Approved: {slug}");
    Ok(())
}

fn check_at_least_one_stage_passed(repo_root: &std::path::Path, slug: &str) -> Result<()> {
    let audit_log = audit::read_audit_log(repo_root, slug)?;

    let has_passed_stage = audit_log
        .entries
        .iter()
        .any(|entry| entry.event_type == audit::AuditEventType::StagePassed);

    if has_passed_stage {
        Ok(())
    } else {
        Err(oya_pipeline::Error::InvalidRecord {
            reason: "Cannot approve task: no stages have been passed. Run at least one stage before approving.".to_string(),
        })
    }
}

async fn execute_show(slug: &str, detailed: bool) -> Result<()> {
    let _ = Slug::new(slug)?;
    let repo_root = detect_repo_root()?;
    let task = load_task_record(slug, &repo_root).await?;

    if detailed {
        println!("Task: {slug}");
        println!("Status: {}", task.status);
        println!("Branch: {}", task.branch);
        println!("Language: {}", task.language.as_str());
    } else {
        println!("{slug}: {}", task.status);
    }

    Ok(())
}

async fn execute_list(priority: Option<&str>, status: Option<&str>) -> Result<()> {
    let repo_root = detect_repo_root()?;
    let tasks = list_all_tasks(&repo_root).await?;

    let filtered: Vec<_> = tasks
        .into_iter()
        .filter(|task| {
            // Filter by status
            if let Some(s) = status {
                if task.status.to_filter_status() != s {
                    return false;
                }
            }
            // Filter by priority
            if let Some(p) = priority {
                if task.priority.as_str().to_uppercase() != p.to_uppercase() {
                    return false;
                }
            }
            true
        })
        .collect();

    if filtered.is_empty() {
        println!("No matching tasks");
    } else {
        for task in filtered {
            println!(
                "{} ({}) {} [{}]",
                task.slug,
                task.branch,
                task.status.to_filter_status(),
                task.priority
            );
        }
    }

    Ok(())
}

