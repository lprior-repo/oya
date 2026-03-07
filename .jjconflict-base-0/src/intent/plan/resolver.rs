#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub ordered_tasks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolverError {
    CycleDetected { task_id: String },
    UnknownDependency { task_id: String, dependency: String },
    EmptyTaskList,
    UnreachableTask { task_id: String },
}

impl std::fmt::Display for ResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolverError::CycleDetected { task_id } => {
                write!(f, "cycle detected involving task '{}'", task_id)
            }
            ResolverError::UnknownDependency { task_id, dependency } => {
                write!(f, "task '{}' has unknown dependency '{}'", task_id, dependency)
            }
            ResolverError::EmptyTaskList => write!(f, "task list is empty"),
            ResolverError::UnreachableTask { task_id } => {
                write!(f, "task '{}' is not reachable from root", task_id)
            }
        }
    }
}

impl std::error::Error for ResolverError {}

pub fn resolve_execution_plan(tasks: &[Task]) -> Result<ExecutionPlan, ResolverError> {
    if tasks.is_empty() {
        return Err(ResolverError::EmptyTaskList);
    }

    let task_map: HashMap<&str, &Task> =
        tasks.iter().map(|task| (task.id.as_str(), task)).collect();

    validate_dependencies(&task_map)?;
    detect_cycles(&task_map)?;
    validate_reachability(&task_map)?;

    let ordered_tasks = topological_sort(&task_map)?;

    Ok(ExecutionPlan { ordered_tasks })
}

fn validate_dependencies(task_map: &HashMap<&str, &Task>) -> Result<(), ResolverError> {
    for task in task_map.values() {
        for dep in &task.dependencies {
            if !task_map.contains_key(dep.as_str()) {
                return Err(ResolverError::UnknownDependency {
                    task_id: task.id.clone(),
                    dependency: dep.clone(),
                });
            }
        }
    }
    Ok(())
}

fn detect_cycles(task_map: &HashMap<&str, &Task>) -> Result<(), ResolverError> {
    let mut visited = HashSet::<&str>::new();
    let mut recursion_stack = HashSet::<&str>::new();

    for task_id in task_map.keys() {
        if !visited.contains(task_id)
            && has_cycle(task_id, task_map, &mut visited, &mut recursion_stack)?
        {
            return Err(ResolverError::CycleDetected { task_id: task_id.to_string() });
        }
    }
    Ok(())
}

fn has_cycle<'a>(
    task_id: &'a str,
    task_map: &HashMap<&'a str, &'a Task>,
    visited: &mut HashSet<&'a str>,
    recursion_stack: &mut HashSet<&'a str>,
) -> Result<bool, ResolverError> {
    visited.insert(task_id);
    recursion_stack.insert(task_id);

    let task = task_map.get(task_id).ok_or_else(|| ResolverError::UnknownDependency {
        task_id: task_id.to_string(),
        dependency: String::new(),
    })?;

    for dep in &task.dependencies {
        let dep_id = dep.as_str();
        if !visited.contains(dep_id) {
            if has_cycle(dep_id, task_map, visited, recursion_stack)? {
                return Ok(true);
            }
        } else if recursion_stack.contains(dep_id) {
            return Ok(true);
        }
    }

    recursion_stack.remove(task_id);
    Ok(false)
}

fn validate_reachability(task_map: &HashMap<&str, &Task>) -> Result<(), ResolverError> {
    let root_tasks: Vec<&str> = task_map
        .keys()
        .filter(|&&id| task_map.get(id).map(|task| task.dependencies.is_empty()).unwrap_or(false))
        .copied()
        .collect();

    let mut reachable = HashSet::<&str>::new();
    for root in root_tasks {
        collect_reachable(root, task_map, &mut reachable)?;
    }

    for task_id in task_map.keys() {
        if !reachable.contains(task_id) {
            return Err(ResolverError::UnreachableTask { task_id: task_id.to_string() });
        }
    }

    Ok(())
}

fn collect_reachable<'a>(
    task_id: &'a str,
    task_map: &HashMap<&'a str, &'a Task>,
    reachable: &mut HashSet<&'a str>,
) -> Result<(), ResolverError> {
    if reachable.contains(task_id) {
        return Ok(());
    }

    reachable.insert(task_id);

    let task = task_map.get(task_id).ok_or_else(|| ResolverError::UnknownDependency {
        task_id: task_id.to_string(),
        dependency: String::new(),
    })?;

    for dep in &task.dependencies {
        collect_reachable(dep.as_str(), task_map, reachable)?;
    }

    for (potential_dependent_id, potential_dependent) in task_map {
        if potential_dependent.dependencies.contains(&task_id.to_string()) {
            collect_reachable(potential_dependent_id, task_map, reachable)?;
        }
    }

    Ok(())
}

fn topological_sort(task_map: &HashMap<&str, &Task>) -> Result<Vec<String>, ResolverError> {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

    for task_id in task_map.keys() {
        in_degree.insert(task_id, 0);
        adjacency.insert(task_id, Vec::new());
    }

    for task in task_map.values() {
        for dep in &task.dependencies {
            if let Some(dependent_list) = adjacency.get_mut(dep.as_str()) {
                dependent_list.push(task.id.as_str());
            }
            if let Some(degree) = in_degree.get_mut(task.id.as_str()) {
                *degree += 1;
            }
        }
    }

    let mut queue: Vec<&str> =
        in_degree.iter().filter(|(_, &degree)| degree == 0).map(|(&id, _)| id).collect();

    let mut result = Vec::new();

    while let Some(current) = queue.pop() {
        result.push(current.to_string());

        if let Some(neighbors) = adjacency.get(current) {
            for &neighbor in neighbors {
                if let Some(degree) = in_degree.get_mut(neighbor) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push(neighbor);
                    }
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_task_list() {
        let result = resolve_execution_plan(&[]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ResolverError::EmptyTaskList));
    }

    #[test]
    fn test_single_task_no_dependencies() {
        let tasks = vec![Task { id: "task-1".to_string(), dependencies: vec![] }];
        let result = resolve_execution_plan(&tasks);
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.ordered_tasks, vec!["task-1"]);
    }

    #[test]
    fn test_simple_dependency_chain() {
        let tasks = vec![
            Task { id: "task-3".to_string(), dependencies: vec!["task-2".to_string()] },
            Task { id: "task-2".to_string(), dependencies: vec!["task-1".to_string()] },
            Task { id: "task-1".to_string(), dependencies: vec![] },
        ];
        let result = resolve_execution_plan(&tasks);
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.ordered_tasks, vec!["task-1", "task-2", "task-3"]);
    }

    #[test]
    fn test_unknown_dependency() {
        let tasks =
            vec![Task { id: "task-1".to_string(), dependencies: vec!["nonexistent".to_string()] }];
        let result = resolve_execution_plan(&tasks);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ResolverError::UnknownDependency { .. }));
    }

    #[test]
    fn test_cycle_detection() {
        let tasks = vec![
            Task { id: "task-1".to_string(), dependencies: vec!["task-2".to_string()] },
            Task { id: "task-2".to_string(), dependencies: vec!["task-1".to_string()] },
        ];
        let result = resolve_execution_plan(&tasks);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ResolverError::CycleDetected { .. }));
    }

    #[test]
    fn test_multiple_root_tasks() {
        let tasks = vec![
            Task { id: "task-1".to_string(), dependencies: vec![] },
            Task { id: "task-2".to_string(), dependencies: vec![] },
            Task {
                id: "task-3".to_string(),
                dependencies: vec!["task-1".to_string(), "task-2".to_string()],
            },
        ];
        let result = resolve_execution_plan(&tasks);
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert!(plan.ordered_tasks.contains(&"task-1".to_string()));
        assert!(plan.ordered_tasks.contains(&"task-2".to_string()));
        let task1_pos = plan.ordered_tasks.iter().position(|x| x == "task-1").unwrap();
        let task2_pos = plan.ordered_tasks.iter().position(|x| x == "task-2").unwrap();
        let task3_pos = plan.ordered_tasks.iter().position(|x| x == "task-3").unwrap();
        assert!(task3_pos > task1_pos);
        assert!(task3_pos > task2_pos);
    }
}
