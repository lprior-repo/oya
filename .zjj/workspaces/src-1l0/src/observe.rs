use crate::ObserveArgs;
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub(crate) fn run(args: ObserveArgs) -> Result<()> {
    let path = bridge_events_path()?;
    print_recent_events(&path, &args)?;
    if args.follow {
        follow_events(path, args)?;
    }
    Ok(())
}

fn bridge_events_path() -> Result<PathBuf> {
    let repo_root = std::env::current_dir().context("resolve current directory")?;
    Ok(repo_root.join(".oya").join("bridge").join("events.jsonl"))
}

fn print_recent_events(path: &PathBuf, args: &ObserveArgs) -> Result<()> {
    let lines = read_lines(path)?;
    let filtered = filter_events(lines, &args.run_id)?;
    let limit = usize::try_from(args.limit).map_err(|_| anyhow::anyhow!("invalid limit value"))?;
    let start = filtered.len().saturating_sub(limit);
    for line in &filtered[start..] {
        println!("{}", line);
    }
    Ok(())
}

fn follow_events(path: PathBuf, args: ObserveArgs) -> Result<()> {
    let mut seen = read_lines(&path)?.len();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(args.interval));
        let lines = read_lines(&path)?;
        if lines.len() <= seen {
            continue;
        }
        let new_lines = lines[seen..].to_vec();
        seen = lines.len();
        for line in filter_events(new_lines, &args.run_id)? {
            println!("{}", line);
        }
    }
}

fn read_lines(path: &PathBuf) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("read bridge events file {}", path.display()))?;
    Ok(content.lines().map(std::string::ToString::to_string).collect())
}

fn filter_events(lines: Vec<String>, run_id: &Option<String>) -> Result<Vec<String>> {
    match run_id {
        None => Ok(lines),
        Some(needle) => lines
            .into_iter()
            .map(|line| {
                matches_run_id(line.as_str(), needle)
                    .with_context(|| format!("bridge filter parse failure: {}", line))
                    .map(|is_match| (line, is_match))
            })
            .collect::<Result<Vec<_>>>()
            .map(|pairs| {
                pairs
                    .into_iter()
                    .filter_map(|(line, is_match)| if is_match { Some(line) } else { None })
                    .collect()
            }),
    }
}

fn matches_run_id(line: &str, run_id: &str) -> Result<bool> {
    let value: Value = serde_json::from_str(line).context("parse event json")?;
    Ok(value.get("run_id").and_then(Value::as_str) == Some(run_id))
}
