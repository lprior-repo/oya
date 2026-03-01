#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::lifecycle::types::{FailureCategory, LifecycleError};

use super::steps::LifecycleStep;

pub fn validate_dag(steps: &[LifecycleStep]) -> Result<(), LifecycleError> {
    let step_names: std::collections::HashSet<&str> =
        steps.iter().map(|step| step.name.as_str()).collect();
    for step in steps {
        for dep in &step.dependencies {
            if !step_names.contains(dep.as_str()) {
                return Err(LifecycleError::terminal(
                    FailureCategory::Validation,
                    format!("step `{}` has unknown dependency `{}`", step.name, dep),
                ));
            }
        }
    }
    detect_cycles(steps)?;
    validate_dependency_order(steps)
}

fn validate_dependency_order(steps: &[LifecycleStep]) -> Result<(), LifecycleError> {
    let mut seen = std::collections::HashSet::new();
    for step in steps {
        for dep in &step.dependencies {
            if !seen.contains(dep.as_str()) {
                return Err(LifecycleError::terminal(
                    FailureCategory::Validation,
                    format!("step `{}` depends on `{}` which appears later", step.name, dep),
                ));
            }
        }
        seen.insert(step.name.as_str());
    }
    Ok(())
}

fn detect_cycles(steps: &[LifecycleStep]) -> Result<(), LifecycleError> {
    let step_map: std::collections::HashMap<&str, &LifecycleStep> =
        steps.iter().map(|step| (step.name.as_str(), step)).collect();
    let mut visited = std::collections::HashSet::<&str>::new();
    let mut recursion_stack = std::collections::HashSet::<&str>::new();
    for step in steps {
        if !visited.contains(step.name.as_str())
            && has_cycle(step.name.as_str(), &step_map, &mut visited, &mut recursion_stack)?
        {
            return Err(LifecycleError::terminal(
                FailureCategory::Validation,
                format!("cycle detected in lifecycle step graph involving `{}`", step.name),
            ));
        }
    }
    Ok(())
}

fn has_cycle<'a>(
    step_name: &'a str,
    step_map: &std::collections::HashMap<&'a str, &'a LifecycleStep>,
    visited: &mut std::collections::HashSet<&'a str>,
    recursion_stack: &mut std::collections::HashSet<&'a str>,
) -> Result<bool, LifecycleError> {
    visited.insert(step_name);
    recursion_stack.insert(step_name);
    let step = step_map.get(step_name).ok_or_else(|| {
        LifecycleError::terminal(
            FailureCategory::Validation,
            format!("internal error: step `{step_name}` not found in map"),
        )
    })?;
    for dep in &step.dependencies {
        let dep_name = dep.as_str();
        if !visited.contains(dep_name) {
            if has_cycle(dep_name, step_map, visited, recursion_stack)? {
                return Ok(true);
            }
        } else if recursion_stack.contains(dep_name) {
            return Ok(true);
        }
    }
    recursion_stack.remove(step_name);
    Ok(false)
}
