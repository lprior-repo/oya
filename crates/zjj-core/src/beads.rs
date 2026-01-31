#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::arithmetic_side_effects)]

use std::path::Path;

use chrono::{DateTime, Utc};
use im::HashMap;
use itertools::Itertools;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use tap::Pipe;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BeadsError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Issue not found: {0}")]
    NotFound(String),

    #[error("Invalid filter: {0}")]
    InvalidFilter(String),

    #[error("Query failed: {0}")]
    QueryFailed(String),

    #[error("Path error: {0}")]
    PathError(String),
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumString,
    Display,
    Serialize,
    Deserialize,
    Hash,
)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IssueStatus {
    #[strum(to_string = "open")]
    Open,

    #[strum(to_string = "in_progress")]
    InProgress,

    #[strum(to_string = "blocked")]
    Blocked,

    #[strum(to_string = "deferred")]
    Deferred,

    #[strum(to_string = "closed")]
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display, Serialize, Deserialize, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IssueType {
    #[strum(to_string = "bug")]
    Bug,

    #[strum(to_string = "feature")]
    Feature,

    #[strum(to_string = "task")]
    Task,

    #[strum(to_string = "epic")]
    Epic,

    #[strum(to_string = "chore")]
    Chore,

    #[strum(to_string = "merge-request")]
    MergeRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
    P4,
}

impl Priority {
    #[must_use]
    pub const fn from_u32(n: u32) -> Option<Self> {
        match n {
            0 => Some(Self::P0),
            1 => Some(Self::P1),
            2 => Some(Self::P2),
            3 => Some(Self::P3),
            4 => Some(Self::P4),
            _ => None,
        }
    }

    #[must_use]
    pub const fn to_u32(&self) -> u32 {
        match self {
            Self::P0 => 0,
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
            Self::P4 => 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadIssue {
    pub id: String,
    pub title: String,
    pub status: IssueStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<IssueType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_by: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<DateTime<Utc>>,
}

impl BeadIssue {
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        self.status == IssueStatus::Blocked
            || self.blocked_by.as_ref().is_some_and(|v| !v.is_empty())
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.status == IssueStatus::Open || self.status == IssueStatus::InProgress
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeadsSummary {
    pub total: usize,
    pub open: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub deferred: usize,
    pub closed: usize,
}

impl BeadsSummary {
    #[must_use]
    #[allow(clippy::arithmetic_side_effects)]
    pub fn from_issues(issues: &[BeadIssue]) -> Self {
        issues.iter().fold(Self::default(), |mut acc, issue| {
            acc.total += 1;
            match issue.status {
                IssueStatus::Open => acc.open += 1,
                IssueStatus::InProgress => acc.in_progress += 1,
                IssueStatus::Blocked => acc.blocked += 1,
                IssueStatus::Deferred => acc.deferred += 1,
                IssueStatus::Closed => acc.closed += 1,
            }
            acc
        })
    }

    #[must_use]
    #[allow(clippy::arithmetic_side_effects)]
    pub const fn active(&self) -> usize {
        self.open + self.in_progress
    }

    #[must_use]
    pub const fn has_blockers(&self) -> bool {
        self.blocked > 0
    }
}

#[derive(Debug, Clone, Default)]
pub struct BeadFilter {
    pub status: Vec<IssueStatus>,
    pub issue_type: Vec<IssueType>,
    pub priority_min: Option<Priority>,
    pub priority_max: Option<Priority>,
    pub labels: Vec<String>,
    pub assignee: Option<String>,
    pub parent: Option<String>,
    pub has_parent: bool,
    pub blocked_only: bool,
    pub search_text: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl BeadFilter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_status(mut self, status: IssueStatus) -> Self {
        self.status.push(status);
        self
    }

    #[must_use]
    pub fn with_statuses(mut self, statuses: impl IntoIterator<Item = IssueStatus>) -> Self {
        self.status.extend(statuses);
        self
    }

    #[must_use]
    pub fn with_type(mut self, issue_type: IssueType) -> Self {
        self.issue_type.push(issue_type);
        self
    }

    #[must_use]
    pub const fn with_priority_range(mut self, min: Priority, max: Priority) -> Self {
        self.priority_min = Some(min);
        self.priority_max = Some(max);
        self
    }

    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    #[must_use]
    pub fn with_assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }

    #[must_use]
    pub fn with_parent(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    #[must_use]
    pub const fn blocked_only(mut self) -> Self {
        self.blocked_only = true;
        self
    }

    #[must_use]
    pub fn with_search(mut self, text: impl Into<String>) -> Self {
        self.search_text = Some(text.into());
        self
    }

    #[must_use]
    pub const fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    #[must_use]
    pub const fn offset(mut self, n: usize) -> Self {
        self.offset = Some(n);
        self
    }
}

#[derive(Debug, Clone, Copy, EnumString, Display, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum BeadSort {
    #[strum(to_string = "priority")]
    Priority,

    #[strum(to_string = "created")]
    Created,

    #[strum(to_string = "updated")]
    Updated,

    #[strum(to_string = "closed")]
    Closed,

    #[strum(to_string = "status")]
    Status,

    #[strum(to_string = "title")]
    Title,

    #[strum(to_string = "id")]
    Id,
}

#[derive(Debug, Clone, Copy, EnumString, Display, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum SortDirection {
    #[strum(to_string = "asc")]
    Asc,

    #[strum(to_string = "desc")]
    Desc,
}

#[derive(Debug, Clone)]
pub struct BeadQuery {
    pub filter: BeadFilter,
    pub sort: BeadSort,
    pub direction: SortDirection,
    pub include_closed: bool,
}

impl Default for BeadQuery {
    fn default() -> Self {
        Self {
            filter: BeadFilter::new(),
            sort: BeadSort::Priority,
            direction: SortDirection::Desc,
            include_closed: false,
        }
    }
}

impl BeadQuery {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn filter(mut self, filter: BeadFilter) -> Self {
        self.filter = filter;
        self
    }

    #[must_use]
    pub const fn sort_by(mut self, sort: BeadSort) -> Self {
        self.sort = sort;
        self
    }

    #[must_use]
    pub const fn direction(mut self, direction: SortDirection) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub const fn include_closed(mut self, include: bool) -> Self {
        self.include_closed = include;
        self
    }
}

pub fn query_beads(workspace_path: &Path) -> std::result::Result<Vec<BeadIssue>, BeadsError> {
    let beads_db = workspace_path.join(".beads/beads.db");

    if !beads_db.exists() {
        return Ok(Vec::new());
    }

    let conn = Connection::open(&beads_db)
        .map_err(|e| BeadsError::DatabaseError(format!("Failed to open beads.db: {e}")))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, title, status, priority, type, description, labels, assignee,
                    parent, depends_on, blocked_by, created_at, updated_at, closed_at
             FROM issues ORDER BY priority, created_at DESC",
        )
        .map_err(|e| BeadsError::QueryFailed(format!("Failed to prepare query: {e}")))?;

    let rows = stmt
        .query_map([], |row| {
            let status_str: String = row.get(2)?;
            let status = status_str.parse().unwrap_or(IssueStatus::Open);

            let priority_str: Option<String> = row.get(3)?;
            let priority = priority_str
                .and_then(|p| p.strip_prefix('P').and_then(|n| n.parse().ok()))
                .and_then(Priority::from_u32);

            let issue_type_str: Option<String> = row.get(4)?;
            let issue_type = issue_type_str.and_then(|s| s.parse().ok());

            let labels_str: Option<String> = row.get(6)?;
            let labels = labels_str.map(|s| s.split(',').map(String::from).collect());

            let depends_on_str: Option<String> = row.get(9)?;
            let depends_on = depends_on_str.map(|s| s.split(',').map(String::from).collect());

            let blocked_by_str: Option<String> = row.get(10)?;
            let blocked_by = blocked_by_str.map(|s| s.split(',').map(String::from).collect());

            let created_at_str: Option<String> = row.get(11)?;
            let created_at = created_at_str
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let updated_at_str: Option<String> = row.get(12)?;
            let updated_at = updated_at_str
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let closed_at_str: Option<String> = row.get(13)?;
            let closed_at = closed_at_str
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            Ok(BeadIssue {
                id: row.get(0)?,
                title: row.get(1)?,
                status,
                priority,
                issue_type,
                description: row.get(5)?,
                labels,
                assignee: row.get(7)?,
                parent: row.get(8)?,
                depends_on,
                blocked_by,
                created_at,
                updated_at,
                closed_at,
            })
        })
        .map_err(|e| BeadsError::QueryFailed(format!("Failed to execute query: {e}")))?;

    rows.collect::<std::result::Result<Vec<BeadIssue>, _>>()
        .map_err(|e| BeadsError::QueryFailed(format!("Failed to collect results: {e}")))
}

#[must_use]
pub fn filter_issues(issues: &[BeadIssue], filter: &BeadFilter) -> Vec<BeadIssue> {
    issues
        .iter()
        .filter(|issue| matches_filter(issue, filter))
        .cloned()
        .collect()
}

fn matches_filter(issue: &BeadIssue, filter: &BeadFilter) -> bool {
    (filter.status.is_empty() || filter.status.contains(&issue.status))
        && (filter.issue_type.is_empty()
            || issue
                .issue_type
                .as_ref()
                .is_some_and(|t| filter.issue_type.contains(t)))
        && (filter
            .priority_min
            .is_none_or(|min| issue.priority.is_none_or(|p| p >= min)))
        && (filter
            .priority_max
            .is_none_or(|max| issue.priority.is_none_or(|p| p <= max)))
        && (filter.labels.is_empty()
            || issue
                .labels
                .as_ref()
                .is_some_and(|issue_labels| filter.labels.iter().all(|l| issue_labels.contains(l))))
        && (filter
            .assignee
            .as_ref()
            .is_none_or(|assignee| issue.assignee.as_ref().is_some_and(|a| a == assignee)))
        && (filter
            .parent
            .as_ref()
            .is_none_or(|parent| issue.parent.as_ref().is_some_and(|p| p == parent)))
        && (!filter.has_parent || issue.parent.is_some())
        && (!filter.blocked_only || issue.is_blocked())
        && filter.search_text.as_ref().is_none_or(|text| {
            let text_lower = text.to_lowercase();
            issue.title.to_lowercase().contains(&text_lower)
                || issue
                    .description
                    .as_ref()
                    .is_some_and(|d| d.to_lowercase().contains(&text_lower))
        })
}

use std::cmp::Reverse;

#[must_use]
pub fn sort_issues(
    issues: &[BeadIssue],
    sort: BeadSort,
    direction: SortDirection,
) -> Vec<BeadIssue> {
    match sort {
        BeadSort::Priority => match direction {
            SortDirection::Asc => issues
                .iter()
                .sorted_by_key(|i| (i.priority.map_or(5, |p| p.to_u32()), i.updated_at))
                .cloned()
                .collect(),
            SortDirection::Desc => issues
                .iter()
                .sorted_by_key(|i| {
                    (
                        Reverse(i.priority.map_or(5, |p| p.to_u32())),
                        Reverse(i.updated_at),
                    )
                })
                .cloned()
                .collect(),
        },
        BeadSort::Created => match direction {
            SortDirection::Asc => issues
                .iter()
                .sorted_by_key(|i| i.created_at)
                .cloned()
                .collect(),
            SortDirection::Desc => issues
                .iter()
                .sorted_by_key(|i| Reverse(i.created_at))
                .cloned()
                .collect(),
        },
        BeadSort::Updated => match direction {
            SortDirection::Asc => issues
                .iter()
                .sorted_by_key(|i| i.updated_at)
                .cloned()
                .collect(),
            SortDirection::Desc => issues
                .iter()
                .sorted_by_key(|i| Reverse(i.updated_at))
                .cloned()
                .collect(),
        },
        BeadSort::Closed => match direction {
            SortDirection::Asc => issues
                .iter()
                .sorted_by_key(|i| i.closed_at)
                .cloned()
                .collect(),
            SortDirection::Desc => issues
                .iter()
                .sorted_by_key(|i| Reverse(i.closed_at))
                .cloned()
                .collect(),
        },
        BeadSort::Status => match direction {
            SortDirection::Asc => issues
                .iter()
                .sorted_by_key(|i| i.status)
                .cloned()
                .collect(),
            SortDirection::Desc => issues
                .iter()
                .sorted_by_key(|i| Reverse(i.status))
                .cloned()
                .collect(),
        },
        BeadSort::Title => match direction {
            SortDirection::Asc => issues
                .iter()
                .sorted_by_key(|i| i.title.to_lowercase())
                .cloned()
                .collect(),
            SortDirection::Desc => issues
                .iter()
                .sorted_by_key(|i| Reverse(i.title.to_lowercase()))
                .cloned()
                .collect(),
        },
        BeadSort::Id => match direction {
            SortDirection::Asc => issues
                .iter()
                .sorted_by_key(|i| i.id.to_lowercase())
                .cloned()
                .collect(),
            SortDirection::Desc => issues
                .iter()
                .sorted_by_key(|i| Reverse(i.id.to_lowercase()))
                .cloned()
                .collect(),
        },
    }
}

#[must_use]
pub fn paginate(
    issues: &[BeadIssue],
    offset: Option<usize>,
    limit: Option<usize>,
) -> Vec<BeadIssue> {
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(issues.len());
    issues.iter().skip(offset).take(limit).cloned().collect()
}

#[must_use]
pub fn apply_query(issues: &[BeadIssue], query: &BeadQuery) -> Vec<BeadIssue> {
    issues
        .pipe(|i| filter_issues(i, &query.filter))
        .pipe(|i| sort_issues(&i, query.sort, query.direction))
        .pipe(|i| paginate(&i, query.filter.offset, query.filter.limit))
}

#[must_use]
pub fn summarize(issues: &[BeadIssue]) -> BeadsSummary {
    BeadsSummary::from_issues(issues)
}

#[must_use]
pub fn find_blockers(issues: &[BeadIssue]) -> Vec<BeadIssue> {
    let blocked_ids: std::collections::HashSet<_> = issues
        .iter()
        .filter(|i| i.is_blocked())
        .flat_map(|i| i.blocked_by.iter().flatten())
        .cloned()
        .collect();

    issues
        .iter()
        .filter(|i| blocked_ids.contains(&i.id))
        .cloned()
        .collect()
}

#[must_use]
pub fn find_blocked(issues: &[BeadIssue]) -> Vec<BeadIssue> {
    issues.iter().filter(|i| i.is_blocked()).cloned().collect()
}

#[must_use]
pub fn get_dependency_graph(issues: &[BeadIssue]) -> HashMap<String, Vec<String>> {
    issues
        .iter()
        .filter_map(|issue| {
            issue
                .depends_on
                .as_ref()
                .map(|deps| deps.iter().map(move |dep| (dep.clone(), issue.id.clone())))
        })
        .flatten()
        .into_group_map()
        .into_iter()
        .collect()
}

#[must_use]
pub fn group_by_status(issues: &[BeadIssue]) -> HashMap<IssueStatus, Vec<BeadIssue>> {
    issues
        .iter()
        .map(|issue| (issue.status, issue.clone()))
        .into_group_map()
        .into_iter()
        .collect()
}

#[must_use]
pub fn group_by_type(issues: &[BeadIssue]) -> HashMap<Option<IssueType>, Vec<BeadIssue>> {
    issues
        .iter()
        .map(|issue| (issue.issue_type.clone(), issue.clone()))
        .into_group_map()
        .into_iter()
        .collect()
}

#[must_use]
pub fn find_ready(issues: &[BeadIssue]) -> Vec<BeadIssue> {
    let blocked_ids: std::collections::HashSet<_> = issues
        .iter()
        .filter(|i| i.is_blocked())
        .flat_map(|i| i.blocked_by.iter().flatten())
        .cloned()
        .collect();

    issues
        .iter()
        .filter(|i| i.is_open() && !blocked_ids.contains(&i.id))
        .cloned()
        .collect()
}

#[must_use]
#[allow(clippy::arithmetic_side_effects, clippy::cast_possible_wrap)]
pub fn find_stale(issues: &[BeadIssue], days: u64) -> Vec<BeadIssue> {
    let cutoff = Utc::now() - chrono::Duration::days(days as i64);

    issues
        .iter()
        .filter(|i| i.updated_at < cutoff && i.status != IssueStatus::Closed)
        .cloned()
        .collect()
}

#[must_use]
pub fn find_potential_duplicates(
    issues: &[BeadIssue],
    threshold: usize,
) -> Vec<(BeadIssue, Vec<BeadIssue>)> {
    let issues_vec: Vec<BeadIssue> = issues.to_vec();

    issues_vec
        .iter()
        .enumerate()
        .filter(|(i, _)| *i < issues_vec.len().saturating_sub(1))
        .filter_map(|(i, issue)| {
            #[allow(clippy::arithmetic_side_effects)]
            let similar: Vec<BeadIssue> = issues_vec
                .iter()
                .skip(i + 1)
                .filter(|other| {
                    let self_words: std::collections::HashSet<_> =
                        issue.title.split_whitespace().collect();
                    let other_words: std::collections::HashSet<_> =
                        other.title.split_whitespace().collect();
                    self_words.intersection(&other_words).count() >= threshold
                })
                .cloned()
                .collect();

            if similar.is_empty() {
                None
            } else {
                Some((issue.clone(), similar))
            }
        })
        .collect()
}

pub fn get_issue(issues: &[BeadIssue], id: &str) -> Option<BeadIssue> {
    issues.iter().find(|i| i.id == id).cloned()
}

#[must_use]
pub fn get_issues_by_id(issues: &[BeadIssue], ids: &[String]) -> Vec<BeadIssue> {
    let id_set: std::collections::HashSet<_> = ids.iter().collect();
    issues
        .iter()
        .filter(|i| id_set.contains(&i.id))
        .cloned()
        .collect()
}

#[must_use]
pub fn calculate_critical_path(issues: &[BeadIssue]) -> Vec<BeadIssue> {
    fn dfs(
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        path: &mut Vec<BeadIssue>,
        visited: &mut std::collections::HashSet<String>,
        all_issues: &[BeadIssue],
    ) {
        if visited.contains(node) {
            return;
        }
        visited.insert(node.to_string());

        if let Some(issue) = all_issues.iter().find(|i| i.id == node) {
            path.push(issue.clone());
        }

        if let Some(deps) = graph.get(node) {
            for dep in deps {
                dfs(dep, graph, path, visited, all_issues);
            }
        }
    }

    let graph = get_dependency_graph(issues);

    let mut all_paths = Vec::new();

    for issue in issues {
        let mut path = Vec::new();
        let mut visited = std::collections::HashSet::new();
        dfs(&issue.id, &graph, &mut path, &mut visited, issues);
        if !path.is_empty() {
            all_paths.push(path);
        }
    }

    all_paths
        .into_iter()
        .max_by_key(std::vec::Vec::len)
        .unwrap_or_default()
}

#[must_use]
pub fn to_ids(issues: &[BeadIssue]) -> Vec<String> {
    issues.iter().map(|i| i.id.clone()).collect()
}

#[must_use]
pub fn to_titles(issues: &[BeadIssue]) -> Vec<String> {
    issues.iter().map(|i| i.title.clone()).collect()
}

#[must_use]
pub fn extract_labels(issues: &[BeadIssue]) -> Vec<String> {
    issues
        .iter()
        .filter_map(|i| i.labels.as_ref())
        .flatten()
        .unique()
        .cloned()
        .collect()
}

#[must_use]
pub fn count_by_status(issues: &[BeadIssue]) -> HashMap<IssueStatus, usize> {
    issues
        .iter()
        .map(|issue| issue.status)
        .counts()
        .into_iter()
        .collect()
}

#[must_use]
pub fn any_match(issues: &[BeadIssue], filter: &BeadFilter) -> bool {
    issues.iter().any(|i| matches_filter(i, filter))
}

#[must_use]
pub fn all_match(issues: &[BeadIssue], filter: &BeadFilter) -> bool {
    issues.iter().all(|i| matches_filter(i, filter))
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, clippy::redundant_clone)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_issue_is_blocked() {
        let blocked = BeadIssue {
            id: "test".to_string(),
            title: "Test".to_string(),
            status: IssueStatus::Blocked,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: Some(vec!["other".to_string()]),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        };

        let unblocked = BeadIssue {
            id: "test2".to_string(),
            title: "Test2".to_string(),
            status: IssueStatus::Open,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        };

        assert!(blocked.is_blocked());
        assert!(!unblocked.is_blocked());
    }

    #[test]
    fn test_bead_issue_is_open() {
        let open = BeadIssue {
            id: "test".to_string(),
            title: "Test".to_string(),
            status: IssueStatus::Open,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        };

        let in_progress = BeadIssue {
            id: "test2".to_string(),
            title: "Test2".to_string(),
            status: IssueStatus::InProgress,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        };

        let closed = BeadIssue {
            id: "test3".to_string(),
            title: "Test3".to_string(),
            status: IssueStatus::Closed,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: Some(Utc::now()),
        };

        assert!(open.is_open());
        assert!(in_progress.is_open());
        assert!(!closed.is_open());
    }

    #[test]
    fn test_beads_summary_from_issues() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Open".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "In Progress".to_string(),
                status: IssueStatus::InProgress,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "3".to_string(),
                title: "Blocked".to_string(),
                status: IssueStatus::Blocked,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "4".to_string(),
                title: "Closed".to_string(),
                status: IssueStatus::Closed,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: Some(Utc::now()),
            },
        ];

        let summary = BeadsSummary::from_issues(&issues);

        assert_eq!(summary.total, 4);
        assert_eq!(summary.open, 1);
        assert_eq!(summary.in_progress, 1);
        assert_eq!(summary.blocked, 1);
        assert_eq!(summary.closed, 1);
        assert_eq!(summary.active(), 2);
        assert!(summary.has_blockers());
    }

    #[test]
    fn test_bead_filter_new() {
        let filter = BeadFilter::new();
        assert!(filter.status.is_empty());
        assert!(filter.issue_type.is_empty());
        assert!(filter.labels.is_empty());
    }

    #[test]
    fn test_bead_filter_chaining() {
        let filter = BeadFilter::new()
            .with_status(IssueStatus::Open)
            .with_status(IssueStatus::InProgress)
            .with_type(IssueType::Bug)
            .with_label("urgent")
            .with_priority_range(Priority::P0, Priority::P2)
            .limit(10);

        assert_eq!(filter.status.len(), 2);
        assert_eq!(filter.issue_type.len(), 1);
        assert_eq!(filter.labels.len(), 1);
        assert_eq!(filter.limit, Some(10));
    }

    #[test]
    fn test_query_beads_empty_path() {
        let result = query_beads(std::path::Path::new("/nonexistent"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_filter_issues_by_status() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Open Issue".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Closed Issue".to_string(),
                status: IssueStatus::Closed,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: Some(Utc::now()),
            },
        ];

        let filter = BeadFilter::new().with_status(IssueStatus::Open);
        let filtered = filter_issues(&issues, &filter);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "1");
    }

    #[test]
    fn test_sort_issues_by_priority() {
        let issues = vec![
            BeadIssue {
                id: "p3".to_string(),
                title: "P3".to_string(),
                status: IssueStatus::Open,
                priority: Some(Priority::P3),
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "p0".to_string(),
                title: "P0".to_string(),
                status: IssueStatus::Open,
                priority: Some(Priority::P0),
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "p2".to_string(),
                title: "P2".to_string(),
                status: IssueStatus::Open,
                priority: Some(Priority::P2),
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let sorted = sort_issues(&issues, BeadSort::Priority, SortDirection::Desc);

        assert_eq!(sorted[0].id, "p0");
        assert_eq!(sorted[1].id, "p2");
        assert_eq!(sorted[2].id, "p3");
    }

    #[test]
    fn test_paginate() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Issue 1".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Issue 2".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "3".to_string(),
                title: "Issue 3".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let page = paginate(&issues, Some(1), Some(1));

        assert_eq!(page.len(), 1);
        assert_eq!(page[0].id, "2");
    }

    #[test]
    fn test_find_blockers() {
        let issues = vec![
            BeadIssue {
                id: "blocker".to_string(),
                title: "Blocker".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "blocked".to_string(),
                title: "Blocked".to_string(),
                status: IssueStatus::Blocked,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: Some(vec!["blocker".to_string()]),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "unrelated".to_string(),
                title: "Unrelated".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let blockers = find_blockers(&issues);

        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].id, "blocker");
    }

    #[test]
    fn test_find_blocked() {
        let issues = vec![
            BeadIssue {
                id: "open".to_string(),
                title: "Open".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "blocked".to_string(),
                title: "Blocked".to_string(),
                status: IssueStatus::Blocked,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: Some(vec!["other".to_string()]),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let blocked = find_blocked(&issues);

        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].id, "blocked");
    }

    #[test]
    fn test_get_issue() {
        let issues = vec![
            BeadIssue {
                id: "zjj-001".to_string(),
                title: "Issue 1".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "zjj-002".to_string(),
                title: "Issue 2".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let found = get_issue(&issues, "zjj-001");
        let not_found = get_issue(&issues, "nonexistent");

        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "zjj-001");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_to_ids() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Issue 1".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Issue 2".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let ids = to_ids(&issues);

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"1".to_string()));
        assert!(ids.contains(&"2".to_string()));
    }

    #[test]
    fn test_extract_labels() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Issue 1".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: Some(vec!["urgent".to_string(), "bug".to_string()]),
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Issue 2".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: Some(vec!["urgent".to_string(), "feature".to_string()]),
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let labels = extract_labels(&issues);

        assert_eq!(labels.len(), 3);
    }

    #[test]
    fn test_group_by_status() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Open".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Another Open".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "3".to_string(),
                title: "Closed".to_string(),
                status: IssueStatus::Closed,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: Some(Utc::now()),
            },
        ];

        let grouped = group_by_status(&issues);

        assert_eq!(grouped.get(&IssueStatus::Open).map(|v| v.len()), Some(2));
        assert_eq!(grouped.get(&IssueStatus::Closed).map(|v| v.len()), Some(1));
    }

    #[test]
    fn test_priority_to_u32() {
        assert_eq!(Priority::P0.to_u32(), 0);
        assert_eq!(Priority::P1.to_u32(), 1);
        assert_eq!(Priority::P2.to_u32(), 2);
        assert_eq!(Priority::P3.to_u32(), 3);
        assert_eq!(Priority::P4.to_u32(), 4);
    }

    #[test]
    fn test_priority_from_u32() {
        assert_eq!(Priority::from_u32(0), Some(Priority::P0));
        assert_eq!(Priority::from_u32(1), Some(Priority::P1));
        assert_eq!(Priority::from_u32(2), Some(Priority::P2));
        assert_eq!(Priority::from_u32(3), Some(Priority::P3));
        assert_eq!(Priority::from_u32(4), Some(Priority::P4));
        assert_eq!(Priority::from_u32(5), None);
    }

    #[test]
    fn test_bead_query_default() {
        let query = BeadQuery::new();
        assert_eq!(query.sort, BeadSort::Priority);
        assert_eq!(query.direction, SortDirection::Desc);
        assert!(!query.include_closed);
    }

    #[test]
    fn test_bead_query_chaining() {
        let query = BeadQuery::new()
            .filter(BeadFilter::new().with_status(IssueStatus::Open))
            .sort_by(BeadSort::Created)
            .direction(SortDirection::Asc)
            .include_closed(true);

        assert_eq!(query.sort, BeadSort::Created);
        assert_eq!(query.direction, SortDirection::Asc);
        assert!(query.include_closed);
    }

    #[test]
    fn test_apply_query() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Open Bug".to_string(),
                status: IssueStatus::Open,
                priority: Some(Priority::P0),
                issue_type: Some(IssueType::Bug),
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Open Feature".to_string(),
                status: IssueStatus::Open,
                priority: Some(Priority::P1),
                issue_type: Some(IssueType::Feature),
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "3".to_string(),
                title: "Closed Bug".to_string(),
                status: IssueStatus::Closed,
                priority: Some(Priority::P2),
                issue_type: Some(IssueType::Bug),
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: Some(Utc::now()),
            },
        ];

        let query = BeadQuery::new()
            .filter(BeadFilter::new().with_type(IssueType::Bug))
            .sort_by(BeadSort::Priority)
            .direction(SortDirection::Desc);

        let result = apply_query(&issues, &query);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "1");
        assert_eq!(result[1].id, "3");
    }

    #[test]
    fn test_any_match() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Open".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Closed".to_string(),
                status: IssueStatus::Closed,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: Some(Utc::now()),
            },
        ];

        let open_filter = BeadFilter::new().with_status(IssueStatus::Open);
        let bug_filter = BeadFilter::new().with_type(IssueType::Bug);

        assert!(any_match(&issues, &open_filter));
        assert!(!any_match(&issues, &bug_filter));
    }

    #[test]
    fn test_all_match() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Open".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Open Too".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let open_filter = BeadFilter::new().with_status(IssueStatus::Open);
        let closed_filter = BeadFilter::new().with_status(IssueStatus::Closed);

        assert!(all_match(&issues, &open_filter));
        assert!(!all_match(&issues, &closed_filter));
    }

    #[test]
    fn test_count_by_status() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Open".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Another Open".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "3".to_string(),
                title: "Closed".to_string(),
                status: IssueStatus::Closed,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: Some(Utc::now()),
            },
        ];

        let counts = count_by_status(&issues);

        assert_eq!(counts.get(&IssueStatus::Open), Some(&2));
        assert_eq!(counts.get(&IssueStatus::Closed), Some(&1));
    }

    #[test]
    fn test_find_stale() {
        let recent = BeadIssue {
            id: "recent".to_string(),
            title: "Recent".to_string(),
            status: IssueStatus::Open,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        };

        let stale = BeadIssue {
            id: "stale".to_string(),
            title: "Stale".to_string(),
            status: IssueStatus::Open,
            priority: None,
            issue_type: None,
            description: None,
            labels: None,
            assignee: None,
            parent: None,
            depends_on: None,
            blocked_by: None,
            created_at: Utc::now() - chrono::Duration::days(30),
            updated_at: Utc::now() - chrono::Duration::days(30),
            closed_at: None,
        };

        let issues = vec![recent.clone(), stale.clone()];

        let stale_issues = find_stale(&issues, 7);

        assert_eq!(stale_issues.len(), 1);
        assert_eq!(stale_issues[0].id, "stale");
    }

    #[test]
    fn test_find_ready() {
        let issues = vec![
            BeadIssue {
                id: "ready".to_string(),
                title: "Ready".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "blocked".to_string(),
                title: "Blocked".to_string(),
                status: IssueStatus::Blocked,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: Some(vec!["other".to_string()]),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "in-progress".to_string(),
                title: "In Progress".to_string(),
                status: IssueStatus::InProgress,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let ready = find_ready(&issues);

        assert_eq!(ready.len(), 2);
        assert!(ready.iter().any(|i| i.id == "ready"));
        assert!(ready.iter().any(|i| i.id == "in-progress"));
        assert!(!ready.iter().any(|i| i.id == "blocked"));
    }

    #[test]
    fn test_get_issues_by_id() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Issue 1".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Issue 2".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "3".to_string(),
                title: "Issue 3".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let ids = vec!["1".to_string(), "3".to_string()];
        let result = get_issues_by_id(&issues, &ids);

        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|i| i.id == "1"));
        assert!(result.iter().any(|i| i.id == "3"));
    }

    #[test]
    fn test_get_dependency_graph() {
        let issues = vec![
            BeadIssue {
                id: "1".to_string(),
                title: "Issue 1".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: Some(vec!["2".to_string()]),
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
            BeadIssue {
                id: "2".to_string(),
                title: "Issue 2".to_string(),
                status: IssueStatus::Open,
                priority: None,
                issue_type: None,
                description: None,
                labels: None,
                assignee: None,
                parent: None,
                depends_on: None,
                blocked_by: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                closed_at: None,
            },
        ];

        let graph = get_dependency_graph(&issues);

        assert!(graph
            .get("2")
            .map(|v| v.contains(&"1".to_string()))
            .unwrap_or(false));
    }
}
