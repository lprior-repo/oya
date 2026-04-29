use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoverageError {
    #[error("Failed to read file at {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to read directory at {path}: {source}")]
    ReadDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Malformed YAML at {path}: {source}")]
    MalformedYaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("Invalid spec shape at {path}: {detail}")]
    InvalidSpecShape { path: PathBuf, detail: String },
    #[error("Duplicate behavior id '{id}' in {path}")]
    DuplicateBehaviorId { path: PathBuf, id: String },
    #[error("Duplicate edge case id '{id}' in {path}")]
    DuplicateEdgeCaseId { path: PathBuf, id: String },
    #[error("Malformed scenario reference at {path}: {detail}")]
    MalformedReference { path: PathBuf, detail: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecCoverage {
    pub spec_id: String,
    pub total_behaviors: usize,
    pub covered_behaviors: usize,
    pub total_edge_cases: usize,
    pub covered_edge_cases: usize,
    pub coverage_percentage: f64,
    pub missing_behaviors: Vec<String>,
    pub missing_edge_cases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub specs: Vec<SpecCoverage>,
    pub overall_coverage: f64,
    pub total_behaviors: usize,
    pub total_edge_cases: usize,
    pub covered_behaviors: usize,
    pub covered_edge_cases: usize,
    pub common_gaps: Vec<String>,
}

pub struct CoverageAnalyzer {
    specs_dir: PathBuf,
    scenarios_dir: PathBuf,
}

impl CoverageAnalyzer {
    #[must_use]
    pub fn new(specs_dir: &Path, scenarios_dir: &Path) -> Self {
        Self { specs_dir: specs_dir.to_path_buf(), scenarios_dir: scenarios_dir.to_path_buf() }
    }

    /// Analyze scenario coverage.
    ///
    /// # Errors
    /// Returns an error if finding files or reading content fails.
    pub fn analyze(&self) -> Result<CoverageReport, CoverageError> {
        let mut spec_coverage = Vec::new();

        for spec_file in self.find_spec_files()? {
            if let Some(coverage) = self.analyze_spec(&spec_file)? {
                spec_coverage.push(coverage);
            }
        }

        let (total_behaviors, covered_behaviors) = if spec_coverage.is_empty() {
            (0, 0)
        } else {
            let t: usize = spec_coverage.iter().map(|s| s.total_behaviors).sum();
            let c: usize = spec_coverage.iter().map(|s| s.covered_behaviors).sum();
            (t, c)
        };

        let total_edge_cases: usize = spec_coverage.iter().map(|s| s.total_edge_cases).sum();
        let covered_edge_cases: usize = spec_coverage.iter().map(|s| s.covered_edge_cases).sum();

        let mut gap_counts: HashMap<String, usize> = HashMap::new();
        for spec in &spec_coverage {
            for behavior in &spec.missing_behaviors {
                *gap_counts.entry(behavior.clone()).or_insert(0) += 1;
            }
            for edge_case in &spec.missing_edge_cases {
                *gap_counts.entry(edge_case.clone()).or_insert(0) += 1;
            }
        }

        let mut sorted_gaps: Vec<_> = gap_counts.into_iter().collect();
        sorted_gaps.sort_by_key(|b| std::cmp::Reverse(b.1));
        let common_gaps: Vec<String> = sorted_gaps.into_iter().take(10).map(|(s, _)| s).collect();

        Ok(CoverageReport {
            specs: spec_coverage,
            overall_coverage: if total_behaviors > 0 {
                #[allow(clippy::cast_precision_loss)]
                {
                    covered_behaviors as f64 / total_behaviors as f64 * 100.0
                }
            } else {
                0.0
            },
            total_behaviors,
            total_edge_cases,
            covered_behaviors,
            covered_edge_cases,
            common_gaps,
        })
    }

    fn find_spec_files(&self) -> Result<Vec<PathBuf>, CoverageError> {
        let mut specs = Self::collect_yaml_files(&self.specs_dir)?;
        specs.sort();
        Ok(specs)
    }

    fn collect_yaml_files(root: &Path) -> Result<Vec<PathBuf>, CoverageError> {
        let mut files = Vec::new();
        if !root.exists() {
            return Ok(files);
        }

        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir)
                .map_err(|source| CoverageError::ReadDir { path: dir.clone(), source })?
            {
                let path = entry
                    .map_err(|source| CoverageError::ReadDir { path: dir.clone(), source })?
                    .path();
                if path.is_dir() {
                    stack.push(path);
                } else if path
                    .extension()
                    .and_then(std::ffi::OsStr::to_str)
                    .is_some_and(|ext| ext == "yaml" || ext == "yml")
                {
                    files.push(path);
                }
            }
        }

        Ok(files)
    }

    fn normalize_spec_ref(value: &str) -> String {
        let normalized = value.trim().replace('\\', "/");
        let name =
            normalized.rsplit('/').next().map_or(normalized.as_str(), std::convert::identity);
        let stem = name
            .strip_suffix(".yaml")
            .or_else(|| name.strip_suffix(".yml"))
            .map_or(name, std::convert::identity);

        stem.strip_prefix("spec-").map_or(stem, std::convert::identity).to_lowercase()
    }

    #[allow(clippy::too_many_lines)]
    fn analyze_spec(&self, spec_path: &Path) -> Result<Option<SpecCoverage>, CoverageError> {
        let spec_path_buf = spec_path.to_path_buf();
        let spec_content = fs::read_to_string(spec_path)
            .map_err(|source| CoverageError::ReadFile { path: spec_path_buf.clone(), source })?;
        let yaml: Value = serde_yaml::from_str(&spec_content).map_err(|source| {
            CoverageError::MalformedYaml { path: spec_path_buf.clone(), source }
        })?;

        if yaml.get("specification").is_none() {
            return Ok(None);
        }

        let spec_id = yaml
            .get("specification")
            .and_then(|value| value.get("identity"))
            .and_then(|value| value.get("id"))
            .and_then(serde_yaml::Value::as_str)
            .ok_or_else(|| CoverageError::InvalidSpecShape {
                path: spec_path_buf.clone(),
                detail: "missing specification.identity.id".to_string(),
            })?
            .trim()
            .to_string();

        if spec_id.is_empty() {
            return Err(CoverageError::InvalidSpecShape {
                path: spec_path_buf,
                detail: "specification.identity.id must be a non-empty string".to_string(),
            });
        }

        let mut behavior_ids: HashSet<String> = HashSet::new();
        let mut edge_case_ids: HashSet<String> = HashSet::new();
        let Some(specification) = yaml.get("specification") else {
            return Ok(None);
        };

        let behaviors = specification
            .get("behaviors")
            .and_then(serde_yaml::Value::as_sequence)
            .ok_or_else(|| CoverageError::InvalidSpecShape {
                path: spec_path_buf.clone(),
                detail: "specification.behaviors must be an array".to_string(),
            })?;

        for behavior in behaviors {
            let behavior_map =
                behavior.as_mapping().ok_or_else(|| CoverageError::InvalidSpecShape {
                    path: spec_path_buf.clone(),
                    detail: "each behavior must be an object".to_string(),
                })?;

            let behavior_id = behavior_map
                .get(Value::String("id".to_string()))
                .and_then(serde_yaml::Value::as_str)
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| CoverageError::InvalidSpecShape {
                    path: spec_path_buf.clone(),
                    detail: "each behavior must include a non-empty string id".to_string(),
                })?
                .to_string();

            if !behavior_ids.insert(behavior_id.clone()) {
                return Err(CoverageError::DuplicateBehaviorId {
                    path: spec_path_buf.clone(),
                    id: behavior_id,
                });
            }

            if let Some(edge_cases_value) =
                behavior_map.get(Value::String("edge_cases".to_string()))
            {
                let edge_cases = edge_cases_value.as_sequence().ok_or_else(|| {
                    CoverageError::InvalidSpecShape {
                        path: spec_path_buf.clone(),
                        detail: "behavior.edge_cases must be an array when provided".to_string(),
                    }
                })?;

                for edge_case in edge_cases {
                    let edge_map =
                        edge_case.as_mapping().ok_or_else(|| CoverageError::InvalidSpecShape {
                            path: spec_path_buf.clone(),
                            detail: "each edge case must be an object".to_string(),
                        })?;

                    let edge_case_id = edge_map
                        .get(Value::String("id".to_string()))
                        .and_then(serde_yaml::Value::as_str)
                        .map(str::trim)
                        .filter(|id| !id.is_empty())
                        .ok_or_else(|| CoverageError::InvalidSpecShape {
                            path: spec_path_buf.clone(),
                            detail: "each edge case must include a non-empty string id".to_string(),
                        })?
                        .to_string();

                    if !edge_case_ids.insert(edge_case_id.clone()) {
                        return Err(CoverageError::DuplicateEdgeCaseId {
                            path: spec_path_buf.clone(),
                            id: edge_case_id,
                        });
                    }
                }
            }
        }

        let mut scenario_behavior_ids: HashSet<String> = HashSet::new();
        let mut scenario_edge_case_ids: HashSet<String> = HashSet::new();

        for (scenario, scenario_path) in self.find_scenarios_for_spec(&spec_id)? {
            let steps =
                scenario.get("steps").and_then(serde_yaml::Value::as_sequence).or_else(|| {
                    scenario
                        .get("scenario")
                        .and_then(|inner| inner.get("steps"))
                        .and_then(serde_yaml::Value::as_sequence)
                });

            if let Some(steps) = steps {
                for step in steps {
                    if let Some(assertions_value) = step.get("assertions") {
                        let assertions = assertions_value.as_sequence().ok_or_else(|| {
                            CoverageError::InvalidSpecShape {
                                path: scenario_path.clone(),
                                detail: "scenario step assertions must be an array".to_string(),
                            }
                        })?;

                        for assertion in assertions {
                            if let Some(behavior_ref_value) = assertion.get("behavior_ref") {
                                let behavior_ref = behavior_ref_value
                                    .as_str()
                                    .map(str::trim)
                                    .filter(|reference| !reference.is_empty())
                                    .ok_or_else(|| CoverageError::MalformedReference {
                                        path: scenario_path.clone(),
                                        detail: "behavior_ref must be a non-empty string"
                                            .to_string(),
                                    })?;

                                scenario_behavior_ids.insert(behavior_ref.to_string());
                            }

                            if let Some(edge_case_ref_value) = assertion.get("edge_case_ref") {
                                let edge_case_ref = edge_case_ref_value
                                    .as_str()
                                    .map(str::trim)
                                    .filter(|reference| !reference.is_empty())
                                    .ok_or_else(|| CoverageError::MalformedReference {
                                        path: scenario_path.clone(),
                                        detail: "edge_case_ref must be a non-empty string"
                                            .to_string(),
                                    })?;

                                scenario_edge_case_ids.insert(edge_case_ref.to_string());
                            }
                        }
                    }
                }
            }
        }

        let covered_behaviors = behavior_ids.intersection(&scenario_behavior_ids).count();
        let covered_edge_cases = edge_case_ids.intersection(&scenario_edge_case_ids).count();

        let mut missing_behaviors: Vec<String> =
            behavior_ids.difference(&scenario_behavior_ids).cloned().collect();
        missing_behaviors.sort();

        let mut missing_edge_cases: Vec<String> =
            edge_case_ids.difference(&scenario_edge_case_ids).cloned().collect();
        missing_edge_cases.sort();

        Ok(Some(SpecCoverage {
            spec_id,
            total_behaviors: behavior_ids.len(),
            covered_behaviors,
            total_edge_cases: edge_case_ids.len(),
            covered_edge_cases,
            coverage_percentage: if behavior_ids.is_empty() {
                0.0
            } else {
                #[allow(clippy::cast_precision_loss)]
                {
                    covered_behaviors as f64 / behavior_ids.len() as f64 * 100.0
                }
            },
            missing_behaviors,
            missing_edge_cases,
        }))
    }

    fn find_scenarios_for_spec(
        &self,
        spec_id: &str,
    ) -> Result<Vec<(Value, PathBuf)>, CoverageError> {
        let mut scenarios = Vec::new();
        let normalized_spec_id = Self::normalize_spec_ref(spec_id);

        for path in Self::collect_yaml_files(&self.scenarios_dir)? {
            let content = fs::read_to_string(&path)
                .map_err(|source| CoverageError::ReadFile { path: path.clone(), source })?;
            let yaml = serde_yaml::from_str::<Value>(&content)
                .map_err(|source| CoverageError::MalformedYaml { path: path.clone(), source })?;

            if let Some(scenario) = yaml.get("scenario") {
                if let Some(ref_str) = scenario.get("spec_ref").and_then(Value::as_str) {
                    let normalized_ref = Self::normalize_spec_ref(ref_str);
                    if normalized_ref == normalized_spec_id {
                        scenarios.push((yaml, path.clone()));
                    }
                }
            }
        }
        Ok(scenarios)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let dir = std::env::temp_dir().join(format!("oya-coverage-{label}-{nanos}"));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn write_file(path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(path, content)?;
        Ok(())
    }

    fn spec_with_edge_cases() -> &'static str {
        r#"
specification:
  identity:
    id: spec-coverage
    version: 1.0.0
  behaviors:
    - id: behavior-1
      description: behavior
      edge_cases:
        - id: edge-1
          when: edge
          then:
            - "fails"
"#
    }

    fn scenario_with_refs(spec_ref: &str) -> String {
        format!(
            r#"
scenario:
  spec_ref: {spec_ref}
  steps:
    - assertions:
        - behavior_ref: behavior-1
          edge_case_ref: edge-1
"#
        )
    }

    #[test]
    fn coverage_analyzer_initializes_with_empty_paths() {
        let _ = CoverageAnalyzer::new(Path::new("."), Path::new("."));
    }

    #[test]
    fn given_matching_behavior_ref_when_analyzing_then_behavior_is_counted_as_covered(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("behavior")?;
        let specs = root.join("specs");
        let scenarios = root.join("scenarios");
        fs::create_dir_all(&specs)?;
        fs::create_dir_all(&scenarios)?;

        write_file(&specs.join("spec.yaml"), spec_with_edge_cases())?;
        write_file(&scenarios.join("scenario.yaml"), &scenario_with_refs("spec-coverage"))?;

        let report = CoverageAnalyzer::new(&specs, &scenarios).analyze()?;

        assert_eq!(report.specs.len(), 1);
        assert_eq!(report.specs[0].covered_behaviors, 1);
        assert!(report.specs[0].missing_behaviors.is_empty());
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn given_matching_edge_case_ref_when_analyzing_then_edge_case_is_counted_as_covered(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("edge-case")?;
        let specs = root.join("specs");
        let scenarios = root.join("scenarios");
        fs::create_dir_all(&specs)?;
        fs::create_dir_all(&scenarios)?;

        write_file(&specs.join("spec.yaml"), spec_with_edge_cases())?;
        write_file(&scenarios.join("scenario.yaml"), &scenario_with_refs("spec-coverage"))?;

        let report = CoverageAnalyzer::new(&specs, &scenarios).analyze()?;

        assert_eq!(report.specs[0].covered_edge_cases, 1);
        assert!(report.specs[0].missing_edge_cases.is_empty());
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn given_spec_without_identity_id_when_analyzing_then_it_returns_error(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("missing-id")?;
        let specs = root.join("specs");
        let scenarios = root.join("scenarios");
        fs::create_dir_all(&specs)?;
        fs::create_dir_all(&scenarios)?;

        write_file(
            &specs.join("spec.yaml"),
            r#"
specification:
  identity:
    version: 1.0.0
  behaviors: []
"#,
        )?;

        let result = CoverageAnalyzer::new(&specs, &scenarios).analyze();
        assert!(result.is_err());
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn given_duplicate_behavior_ids_when_analyzing_then_it_returns_typed_duplicate_error(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("duplicate-behavior")?;
        let specs = root.join("specs");
        let scenarios = root.join("scenarios");
        fs::create_dir_all(&specs)?;
        fs::create_dir_all(&scenarios)?;

        write_file(
            &specs.join("spec.yaml"),
            r#"
specification:
  identity:
    id: spec-coverage
  behaviors:
    - id: behavior-1
    - id: behavior-1
"#,
        )?;

        let result = CoverageAnalyzer::new(&specs, &scenarios).analyze();
        assert!(matches!(result, Err(CoverageError::DuplicateBehaviorId { .. })));
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn given_duplicate_edge_case_ids_when_analyzing_then_it_returns_typed_duplicate_error(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("duplicate-edge")?;
        let specs = root.join("specs");
        let scenarios = root.join("scenarios");
        fs::create_dir_all(&specs)?;
        fs::create_dir_all(&scenarios)?;

        write_file(
            &specs.join("spec.yaml"),
            r#"
specification:
  identity:
    id: spec-coverage
  behaviors:
    - id: behavior-1
      edge_cases:
        - id: edge-1
        - id: edge-1
"#,
        )?;

        let result = CoverageAnalyzer::new(&specs, &scenarios).analyze();
        assert!(matches!(result, Err(CoverageError::DuplicateEdgeCaseId { .. })));
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn given_non_array_behaviors_shape_when_analyzing_then_it_returns_invalid_shape_error(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("invalid-shape")?;
        let specs = root.join("specs");
        let scenarios = root.join("scenarios");
        fs::create_dir_all(&specs)?;
        fs::create_dir_all(&scenarios)?;

        write_file(
            &specs.join("spec.yaml"),
            r#"
specification:
  identity:
    id: spec-coverage
  behaviors: {}
"#,
        )?;

        let result = CoverageAnalyzer::new(&specs, &scenarios).analyze();
        assert!(matches!(result, Err(CoverageError::InvalidSpecShape { .. })));
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn given_non_string_behavior_reference_when_analyzing_then_it_returns_malformed_reference_error(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("malformed-reference")?;
        let specs = root.join("specs");
        let scenarios = root.join("scenarios");
        fs::create_dir_all(&specs)?;
        fs::create_dir_all(&scenarios)?;

        write_file(&specs.join("spec.yaml"), spec_with_edge_cases())?;
        write_file(
            &scenarios.join("scenario.yaml"),
            r#"
scenario:
  spec_ref: spec-coverage
  steps:
    - assertions:
        - behavior_ref: 42
"#,
        )?;

        let result = CoverageAnalyzer::new(&specs, &scenarios).analyze();
        assert!(matches!(result, Err(CoverageError::MalformedReference { .. })));
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn given_nested_spec_and_scenario_dirs_when_analyzing_then_discovery_is_recursive(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("recursive-discovery")?;
        let specs = root.join("specs").join("nested");
        let scenarios = root.join("scenarios").join("deep").join("inner");
        fs::create_dir_all(&specs)?;
        fs::create_dir_all(&scenarios)?;

        write_file(&specs.join("spec.yaml"), spec_with_edge_cases())?;
        write_file(&scenarios.join("scenario.yaml"), &scenario_with_refs("spec-coverage"))?;

        let report =
            CoverageAnalyzer::new(&root.join("specs"), &root.join("scenarios")).analyze()?;

        assert_eq!(report.specs.len(), 1);
        assert_eq!(report.specs[0].covered_behaviors, 1);
        assert_eq!(report.specs[0].covered_edge_cases, 1);
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn given_mixed_spec_ref_formats_when_analyzing_then_refs_are_normalized(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir("normalized-ref")?;
        let specs = root.join("specs");
        let scenarios = root.join("scenarios");
        fs::create_dir_all(&specs)?;
        fs::create_dir_all(&scenarios)?;

        write_file(
            &specs.join("spec.yaml"),
            r#"
specification:
  identity:
    id: spec-flow-wasm-v1
  behaviors:
    - id: behavior-1
      edge_cases: []
"#,
        )?;
        write_file(
            &scenarios.join("scenario.yaml"),
            &scenario_with_refs("specs/flow-wasm-v1.yaml"),
        )?;

        let report = CoverageAnalyzer::new(&specs, &scenarios).analyze()?;

        assert_eq!(report.specs.len(), 1);
        assert_eq!(report.specs[0].covered_behaviors, 1);
        fs::remove_dir_all(root)?;
        Ok(())
    }
}
