//! Factory CLI - Contract-driven CI/CD Pipeline
//!
//! Main entry point for the factory command line tool.

mod cli;

use std::time::Instant;

use clap::Parser;
use factory_core::{
    audit,
    domain::{filter_stages, get_stage, Slug, Task, TaskStatus},
    persistence::{list_all_tasks, load_task_record, save_task_record},
    repo::{detect_language, detect_repo_root},
    stages::{execute_stage, execute_stages_dry_run},
    worktree::create_worktree,
    Result,
};

use crate::cli::{Cli, Commands, HELP_TEXT};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New {
            slug,
            contract,
            interactive,
        } => execute_new(&slug, contract.as_deref(), interactive),

        Commands::Stage {
            slug,
            stage,
            dry_run,
            from,
            to,
        } => execute_stage_command(&slug, &stage, dry_run, from.as_deref(), to.as_deref()),

        Commands::Approve {
            slug,
            strategy,
            force,
        } => execute_approve(&slug, strategy.as_deref(), force),

        Commands::Show { slug, detailed } => execute_show(&slug, detailed),

        Commands::List { priority, status } => execute_list(priority.as_deref(), status.as_deref()),

        Commands::Help { topic } => {
            show_help(topic.as_deref());
            Ok(())
        }
    }
}

fn execute_new(slug: &str, contract: Option<&str>, interactive: bool) -> Result<()> {
    let validated_slug = Slug::new(slug)?;
    let repo_root = detect_repo_root()?;
    let lang = detect_language(&repo_root)?;

    let wt = create_worktree(slug, lang, &repo_root)?;

    let task = Task::new(validated_slug, lang, wt.path.clone());
    save_task_record(&task, &repo_root)?;

    // Log task creation to audit trail
    let _ = audit::log_task_created(&repo_root, slug, lang.as_str(), &wt.branch);

    let contract_info = contract.map_or(String::new(), |c| format!("\nContract: {c}"));
    let interactive_info = if interactive {
        "\nInteractive: enabled"
    } else {
        ""
    };

    println!(
        "Created: {}\nBranch:  {}\nLanguage: {}{}{}",
        wt.path.display(),
        wt.branch,
        lang.as_str(),
        contract_info,
        interactive_info
    );

    Ok(())
}

fn execute_stage_command(
    slug: &str,
    stage_name: &str,
    dry_run: bool,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<()> {
    let _ = Slug::new(slug)?;
    let repo_root = detect_repo_root()?;
    let task = load_task_record(slug, &repo_root)?;

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
        execute_single_stage(slug, &stage.name, &task, &repo_root)?;
    }

    Ok(())
}

fn execute_single_stage(
    slug: &str,
    stage_name: &str,
    task: &Task,
    repo_root: &std::path::Path,
) -> Result<()> {
    let _ = get_stage(stage_name)?;

    // Log stage start
    let _ = audit::log_stage_started(repo_root, slug, stage_name, 1);

    let start_time = Instant::now();
    let result = execute_stage(stage_name, task.language, &task.worktree_path);

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

fn execute_approve(slug: &str, strategy: Option<&str>, force: bool) -> Result<()> {
    let _ = Slug::new(slug)?;
    let repo_root = detect_repo_root()?;
    let task = load_task_record(slug, &repo_root)?;

    if !force {
        check_at_least_one_stage_passed(&repo_root, slug)?;
    }

    let strategy_str = strategy.unwrap_or("immediate");

    let approved_task = task.with_status(TaskStatus::Integrated);
    save_task_record(&approved_task, &repo_root)?;

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
        Err(factory_core::Error::InvalidRecord {
            reason: "Cannot approve task: no stages have been passed. Run at least one stage before approving.".to_string(),
        })
    }
}

fn execute_show(slug: &str, detailed: bool) -> Result<()> {
    let _ = Slug::new(slug)?;
    let repo_root = detect_repo_root()?;
    let task = load_task_record(slug, &repo_root)?;

    if detailed {
        println!("Task: {slug}");
        println!("Status: {}", task.status);
        println!("Branch: {}", task.branch);
        println!("Worktree: {}", task.worktree_path.display());
        println!("Language: {}", task.language.as_str());
    } else {
        println!("{slug}: {}", task.status);
    }

    Ok(())
}

fn execute_list(priority: Option<&str>, status: Option<&str>) -> Result<()> {
    let repo_root = detect_repo_root()?;
    let tasks = list_all_tasks(&repo_root)?;

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

fn show_help(topic: Option<&str>) {
    match topic {
        None => println!("{HELP_TEXT}"),
        Some(t) => println!("Help for: {t}"),
    }
}
