use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Component {
    pub name: String,
    pub required_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Behavior {
    pub name: String,
    pub covers_components: Vec<String>,
    pub covers_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoverageGap {
    pub component_name: String,
    pub missing_fields: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoverageResult {
    pub coverage_score: f64,
    pub gaps: Vec<CoverageGap>,
    pub covered_components: Vec<String>,
    pub total_components: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CoverageError {
    EmptyInput,
    InvalidComponent(String),
    InvalidBehavior(String),
}

impl std::fmt::Display for CoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoverageError::EmptyInput => write!(f, "Input cannot be empty"),
            CoverageError::InvalidComponent(name) => write!(f, "Invalid component: {}", name),
            CoverageError::InvalidBehavior(name) => write!(f, "Invalid behavior: {}", name),
        }
    }
}

impl std::error::Error for CoverageError {}

fn collect_covered_items(behaviors: &[Behavior]) -> (HashSet<String>, HashSet<String>) {
    let covered_components: HashSet<String> =
        behaviors.iter().flat_map(|b| b.covers_components.iter().cloned()).collect();
    let covered_fields: HashSet<String> =
        behaviors.iter().flat_map(|b| b.covers_fields.iter().cloned()).collect();
    (covered_components, covered_fields)
}

fn find_coverage_gaps(
    components: &[Component],
    covered_components: &HashSet<String>,
    covered_fields: &HashSet<String>,
) -> Vec<CoverageGap> {
    let mut gaps = Vec::new();
    for component in components {
        let component_covered = covered_components.contains(&component.name);
        if !component_covered {
            gaps.push(CoverageGap {
                component_name: component.name.clone(),
                missing_fields: component.required_fields.clone(),
                reason: format!("Component '{}' is not covered by any behavior", component.name),
            });
        } else {
            let missing_fields: Vec<String> = component
                .required_fields
                .iter()
                .filter(|field| !covered_fields.contains(*field))
                .cloned()
                .collect();
            if !missing_fields.is_empty() {
                gaps.push(CoverageGap {
                    component_name: component.name.clone(),
                    missing_fields: missing_fields.clone(),
                    reason: format!(
                        "Component '{}' is covered but missing {} required field(s)",
                        component.name,
                        missing_fields.len()
                    ),
                });
            }
        }
    }
    gaps
}

fn calculate_coverage_score(
    components: &[Component],
    covered_components: &HashSet<String>,
    gaps: &[CoverageGap],
) -> f64 {
    let covered_count = components.iter().filter(|c| covered_components.contains(&c.name)).count();
    let coverage_score = if components.is_empty() {
        100.0
    } else {
        (covered_count as f64 / components.len() as f64) * 100.0
    };
    let has_missing_required_fields = gaps.iter().any(|gap| !gap.missing_fields.is_empty());
    if has_missing_required_fields && coverage_score >= 100.0 {
        99.9
    } else {
        coverage_score
    }
}

pub fn analyze_coverage(
    behaviors: Vec<Behavior>,
    components: Vec<Component>,
) -> Result<CoverageResult, CoverageError> {
    if behaviors.is_empty() && components.is_empty() {
        return Err(CoverageError::EmptyInput);
    }
    if components.is_empty() {
        return Ok(CoverageResult {
            coverage_score: 100.0,
            gaps: vec![],
            covered_components: vec![],
            total_components: 0,
        });
    }
    let (covered_components, covered_fields) = collect_covered_items(&behaviors);
    let gaps = find_coverage_gaps(&components, &covered_components, &covered_fields);
    let coverage_score = calculate_coverage_score(&components, &covered_components, &gaps);
    let covered_component_names: Vec<String> = components
        .iter()
        .filter(|c| covered_components.contains(&c.name))
        .map(|c| c.name.clone())
        .collect();
    Ok(CoverageResult {
        coverage_score,
        gaps,
        covered_components: covered_component_names,
        total_components: components.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_inputs_returns_error() -> Result<(), Box<dyn std::error::Error>> {
        let result = analyze_coverage(vec![], vec![]);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_no_components_returns_full_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let behaviors = vec![Behavior {
            name: "test_behavior".to_string(),
            covers_components: vec![],
            covers_fields: vec![],
        }];

        let result = analyze_coverage(behaviors, vec![])?;
        assert_eq!(result.coverage_score, 100.0);
        assert_eq!(result.gaps.len(), 0);
        Ok(())
    }

    #[test]
    fn test_full_coverage_no_gaps() -> Result<(), Box<dyn std::error::Error>> {
        let behaviors = vec![Behavior {
            name: "behavior1".to_string(),
            covers_components: vec!["ComponentA".to_string()],
            covers_fields: vec!["field1".to_string(), "field2".to_string()],
        }];

        let components = vec![Component {
            name: "ComponentA".to_string(),
            required_fields: vec!["field1".to_string(), "field2".to_string()],
        }];

        let result = analyze_coverage(behaviors, components)?;
        assert_eq!(result.coverage_score, 100.0);
        assert_eq!(result.gaps.len(), 0);
        assert_eq!(result.covered_components.len(), 1);
        Ok(())
    }

    #[test]
    fn test_uncovered_component_creates_gap() -> Result<(), Box<dyn std::error::Error>> {
        let behaviors = vec![Behavior {
            name: "behavior1".to_string(),
            covers_components: vec!["ComponentA".to_string()],
            covers_fields: vec![],
        }];

        let components = vec![
            Component { name: "ComponentA".to_string(), required_fields: vec![] },
            Component {
                name: "ComponentB".to_string(),
                required_fields: vec!["field1".to_string()],
            },
        ];

        let result = analyze_coverage(behaviors, components)?;
        assert_eq!(result.coverage_score, 50.0);
        assert_eq!(result.gaps.len(), 1);
        assert_eq!(result.gaps[0].component_name, "ComponentB");
        Ok(())
    }

    #[test]
    fn test_missing_required_fields_reduces_score() -> Result<(), Box<dyn std::error::Error>> {
        let behaviors = vec![Behavior {
            name: "behavior1".to_string(),
            covers_components: vec!["ComponentA".to_string()],
            covers_fields: vec!["field1".to_string()],
        }];

        let components = vec![Component {
            name: "ComponentA".to_string(),
            required_fields: vec!["field1".to_string(), "field2".to_string()],
        }];

        let result = analyze_coverage(behaviors, components)?;
        assert!(result.coverage_score < 100.0);
        assert_eq!(result.gaps.len(), 1);
        assert_eq!(result.gaps[0].missing_fields, vec!["field2"]);
        Ok(())
    }

    #[test]
    fn test_multiple_behaviors_cover_component() -> Result<(), Box<dyn std::error::Error>> {
        let behaviors = vec![
            Behavior {
                name: "behavior1".to_string(),
                covers_components: vec!["ComponentA".to_string()],
                covers_fields: vec!["field1".to_string()],
            },
            Behavior {
                name: "behavior2".to_string(),
                covers_components: vec!["ComponentA".to_string()],
                covers_fields: vec!["field2".to_string()],
            },
        ];

        let components = vec![Component {
            name: "ComponentA".to_string(),
            required_fields: vec!["field1".to_string(), "field2".to_string()],
        }];

        let result = analyze_coverage(behaviors, components)?;
        assert_eq!(result.coverage_score, 100.0);
        assert_eq!(result.gaps.len(), 0);
        Ok(())
    }
}
