use crate::types::{HnswEfConstruction, HnswM, MaxRelatedChunks};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Configuration for knowledge graph construction parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConfig {
    /// Maximum number of related chunks to link per document (1-1000)
    pub max_related_chunks: MaxRelatedChunks,
    /// HNSW graph connectivity parameter (4-64)
    pub hnsw_m: HnswM,
    /// HNSW graph construction effort level (50-1000)
    pub hnsw_ef_construction: HnswEfConstruction,
}

#[allow(dead_code)] // Public API - exported for library users, not used internally
impl GraphConfig {
    /// Create GraphConfig with default values
    pub fn new() -> Self {
        Self {
            max_related_chunks: MaxRelatedChunks::default(),
            hnsw_m: HnswM::default(),
            hnsw_ef_construction: HnswEfConstruction::default(),
        }
    }

    /// Load configuration from a YAML file with validation
    pub fn load_from_file(path: &Path) -> Result<Self> {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct GraphConfigRaw {
            max_related_chunks: usize,
            hnsw_m: usize,
            hnsw_ef_construction: usize,
        }

        let content = fs::read_to_string(path)?;
        let raw_config: GraphConfigRaw = serde_yaml::from_str(&content)?;

        Ok(Self {
            max_related_chunks: MaxRelatedChunks::new(raw_config.max_related_chunks)?,
            hnsw_m: HnswM::new(raw_config.hnsw_m)?,
            hnsw_ef_construction: HnswEfConstruction::new(raw_config.hnsw_ef_construction)?,
        })
    }

    /// Create config with custom values (validates them)
    pub fn with_params(
        max_related_chunks: usize,
        hnsw_m: usize,
        hnsw_ef_construction: usize,
    ) -> Result<Self> {
        Ok(Self {
            max_related_chunks: MaxRelatedChunks::new(max_related_chunks)?,
            hnsw_m: HnswM::new(hnsw_m)?,
            hnsw_ef_construction: HnswEfConstruction::new(hnsw_ef_construction)?,
        })
    }

    /// Get max_related_chunks as usize
    pub fn get_max_related_chunks(&self) -> usize {
        self.max_related_chunks.get()
    }

    /// Get hnsw_m as usize
    pub fn get_hnsw_m(&self) -> usize {
        self.hnsw_m.get()
    }

    /// Get hnsw_ef_construction as usize
    pub fn get_hnsw_ef_construction(&self) -> usize {
        self.hnsw_ef_construction.get()
    }
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for category detection rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryConfig {
    /// Default category to use if no rules match
    pub default_category: String,
    /// List of category detection rules (evaluated in order)
    pub rules: Vec<CategoryRule>,
}

/// A single category detection rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryRule {
    /// The category name to assign if this rule matches
    pub category: String,
    /// Criteria for matching documents
    #[serde(flatten)]
    pub criteria: MatchCriteria,
}

/// Matching criteria for a rule (filename, content, or path patterns)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatchCriteria {
    /// Case-insensitive patterns to match against filename (without extension)
    pub filename: Option<Vec<String>>,
    /// Case-insensitive patterns to match against content (substring match)
    pub content: Option<Vec<String>>,
    /// Patterns to match against file path (substring match)
    pub path: Option<Vec<String>>,
}

impl CategoryConfig {
    /// Load configuration from a YAML file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: CategoryConfig = serde_yaml::from_str(&content)?;

        // Validate that default_category is set
        if config.default_category.is_empty() {
            anyhow::bail!("Config error: default_category is required and must not be empty");
        }

        // Validate that all categories are lowercase alphanumeric
        if !is_valid_category_name(&config.default_category) {
            anyhow::bail!(
                "Config error: default_category '{}' is not lowercase alphanumeric",
                config.default_category
            );
        }

        for rule in &config.rules {
            if !is_valid_category_name(&rule.category) {
                anyhow::bail!(
                    "Config error: category '{}' is not lowercase alphanumeric",
                    rule.category
                );
            }

            // Validate that rule has at least one non-empty criterion
            if !Self::has_valid_criteria(&rule.criteria) {
                anyhow::bail!(
                    "Config error: rule for category '{}' has no criteria (all are None or empty)",
                    rule.category
                );
            }
        }

        Ok(config)
    }

    /// Check if criteria has at least one non-empty, non-None criterion
    fn has_valid_criteria(criteria: &MatchCriteria) -> bool {
        // Check filename: must be Some with non-empty vec containing non-empty strings
        let has_filename = criteria
            .filename
            .as_ref()
            .is_some_and(|v| !v.is_empty() && v.iter().any(|s| !s.trim().is_empty()));

        // Check content: must be Some with non-empty vec containing non-empty strings
        let has_content = criteria
            .content
            .as_ref()
            .is_some_and(|v| !v.is_empty() && v.iter().any(|s| !s.trim().is_empty()));

        // Check path: must be Some with non-empty vec containing non-empty strings
        let has_path = criteria
            .path
            .as_ref()
            .is_some_and(|v| !v.is_empty() && v.iter().any(|s| !s.trim().is_empty()));

        // At least one criterion must be valid
        has_filename || has_content || has_path
    }

    /// Detect category for a document using these rules
    pub fn detect_category(&self, filename: &str, content: &str, path: &str) -> String {
        // Evaluate rules in order
        for rule in &self.rules {
            if self.matches_rule(filename, content, path, &rule.criteria) {
                return rule.category.clone();
            }
        }
        // Return default if no rules match
        self.default_category.clone()
    }

    /// Check if a document matches a rule's criteria
    fn matches_rule(
        &self,
        filename: &str,
        content: &str,
        path: &str,
        criteria: &MatchCriteria,
    ) -> bool {
        // Check filename patterns (case-insensitive)
        if let Some(patterns) = &criteria.filename {
            let fname_lower = Path::new(filename)
                .file_stem()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            if patterns
                .iter()
                .any(|p| fname_lower.contains(&p.to_lowercase()))
            {
                return true;
            }
        }

        // Check content patterns (case-insensitive, substring)
        if let Some(patterns) = &criteria.content {
            let content_lower = content.to_lowercase();
            if patterns
                .iter()
                .any(|p| content_lower.contains(&p.to_lowercase()))
            {
                return true;
            }
        }

        // Check path patterns (case-sensitive substring)
        if let Some(patterns) = &criteria.path {
            if patterns.iter().any(|p| path.contains(p)) {
                return true;
            }
        }

        false
    }
}

/// Validate that a category name is lowercase alphanumeric
fn is_valid_category_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_valid_category_names() {
        assert!(is_valid_category_name("meta"));
        assert!(is_valid_category_name("tutorial"));
        assert!(is_valid_category_name("api-reference"));
        assert!(is_valid_category_name("type_system"));
        assert!(is_valid_category_name("ref123"));
    }

    #[test]
    fn test_detect_category_by_filename() {
        let config = CategoryConfig {
            default_category: "concept".to_string(),
            rules: vec![CategoryRule {
                category: "meta".to_string(),
                criteria: MatchCriteria {
                    filename: Some(vec!["readme".to_string()]),
                    ..Default::default()
                },
            }],
        };
        assert_eq!(config.detect_category("readme.md", "", ""), "meta");
    }

    // === VALIDATION TESTS ===

    #[test]
    fn test_reject_rule_with_all_none_criteria() -> anyhow::Result<()> {
        let config_yaml = r#"
default_category: "concept"
rules:
  - category: "api"
"#;

        let temp = TempDir::new()?;
        let config_path = temp.path().join("config.yaml");
        fs::write(&config_path, config_yaml)?;

        let result = CategoryConfig::load_from_file(&config_path);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("no criteria"));
            assert!(err_msg.contains("api"));
        }
        Ok(())
    }

    #[test]
    fn test_reject_rule_with_all_empty_arrays() -> anyhow::Result<()> {
        let config_yaml = r#"
default_category: "concept"
rules:
  - category: "api"
    filename: []
    content: []
    path: []
"#;

        let temp = TempDir::new()?;
        let config_path = temp.path().join("config.yaml");
        fs::write(&config_path, config_yaml)?;

        let result = CategoryConfig::load_from_file(&config_path);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("no criteria"));
            assert!(err_msg.contains("api"));
        }
        Ok(())
    }

    #[test]
    fn test_reject_rule_with_empty_strings_only() -> anyhow::Result<()> {
        let config_yaml = r#"
default_category: "concept"
rules:
  - category: "tutorial"
    filename: [""]
    content: [""]
    path: [""]
"#;

        let temp = TempDir::new()?;
        let config_path = temp.path().join("config.yaml");
        fs::write(&config_path, config_yaml)?;

        let result = CategoryConfig::load_from_file(&config_path);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("no criteria"));
        }
        Ok(())
    }

    #[test]
    fn test_reject_rule_with_whitespace_only_strings() -> anyhow::Result<()> {
        let config_yaml = r#"
default_category: "concept"
rules:
  - category: "reference"
    filename: ["   "]
    content: [" "]
"#;

        let temp = TempDir::new()?;
        let config_path = temp.path().join("config.yaml");
        fs::write(&config_path, config_yaml)?;

        let result = CategoryConfig::load_from_file(&config_path);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("no criteria"));
        }
        Ok(())
    }

    #[test]
    fn test_accept_rule_with_single_filename_criterion() -> anyhow::Result<()> {
        let config_yaml = r#"
default_category: "concept"
rules:
  - category: "api"
    filename: ["reference"]
"#;

        let temp = TempDir::new()?;
        let config_path = temp.path().join("config.yaml");
        fs::write(&config_path, config_yaml)?;

        let result = CategoryConfig::load_from_file(&config_path);
        assert!(result.is_ok());
        let config = result?;
        assert_eq!(config.rules.len(), 1);
        assert_eq!(config.rules[0].category, "api");
        Ok(())
    }

    #[test]
    fn test_accept_rule_with_single_content_criterion() -> anyhow::Result<()> {
        let config_yaml = r#"
default_category: "concept"
rules:
  - category: "tutorial"
    content: ["example"]
"#;

        let temp = TempDir::new()?;
        let config_path = temp.path().join("config.yaml");
        fs::write(&config_path, config_yaml)?;

        let result = CategoryConfig::load_from_file(&config_path);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_accept_rule_with_single_path_criterion() -> anyhow::Result<()> {
        let config_yaml = r#"
default_category: "concept"
rules:
  - category: "guide"
    path: ["/docs/guides/"]
"#;

        let temp = TempDir::new()?;
        let config_path = temp.path().join("config.yaml");
        fs::write(&config_path, config_yaml)?;

        let result = CategoryConfig::load_from_file(&config_path);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_accept_rule_with_multiple_criteria() -> anyhow::Result<()> {
        let config_yaml = r#"
default_category: "concept"
rules:
  - category: "api"
    filename: ["reference", "swagger"]
    content: ["endpoint", "method"]
    path: ["/api/"]
"#;

        let temp = TempDir::new()?;
        let config_path = temp.path().join("config.yaml");
        fs::write(&config_path, config_yaml)?;

        let result = CategoryConfig::load_from_file(&config_path);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_accept_rule_with_empty_criteria_but_some_populated() -> anyhow::Result<()> {
        let config_yaml = r#"
default_category: "concept"
rules:
  - category: "meta"
    filename: []
    content: ["metadata", "header"]
"#;

        let temp = TempDir::new()?;
        let config_path = temp.path().join("config.yaml");
        fs::write(&config_path, config_yaml)?;

        let result = CategoryConfig::load_from_file(&config_path);
        assert!(result.is_ok());
        let config = result?;
        assert_eq!(config.rules[0].category, "meta");
        Ok(())
    }

    #[test]
    fn test_multiple_rules_all_valid() -> anyhow::Result<()> {
        let config_yaml = r#"
default_category: "concept"
rules:
  - category: "api"
    filename: ["reference"]
  - category: "tutorial"
    content: ["example"]
  - category: "guide"
    path: ["/guides/"]
"#;

        let temp = TempDir::new()?;
        let config_path = temp.path().join("config.yaml");
        fs::write(&config_path, config_yaml)?;

        let result = CategoryConfig::load_from_file(&config_path);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_multiple_rules_one_invalid() -> anyhow::Result<()> {
        let config_yaml = r#"
default_category: "concept"
rules:
  - category: "api"
    filename: ["reference"]
  - category: "broken"
  - category: "guide"
    path: ["/guides/"]
"#;

        let temp = TempDir::new()?;
        let config_path = temp.path().join("config.yaml");
        fs::write(&config_path, config_yaml)?;

        let result = CategoryConfig::load_from_file(&config_path);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("broken"));
        }
        Ok(())
    }

    #[test]
    fn test_has_valid_criteria_with_all_none() {
        let criteria = MatchCriteria::default();
        assert!(!CategoryConfig::has_valid_criteria(&criteria));
    }

    #[test]
    fn test_has_valid_criteria_with_empty_vecs() {
        let criteria = MatchCriteria {
            filename: Some(vec![]),
            content: Some(vec![]),
            path: Some(vec![]),
        };
        assert!(!CategoryConfig::has_valid_criteria(&criteria));
    }

    #[test]
    fn test_has_valid_criteria_with_one_valid_filename() {
        let criteria = MatchCriteria {
            filename: Some(vec!["api".to_string()]),
            content: None,
            path: None,
        };
        assert!(CategoryConfig::has_valid_criteria(&criteria));
    }

    #[test]
    fn test_has_valid_criteria_with_whitespace_strings() {
        let criteria = MatchCriteria {
            filename: Some(vec!["  ".to_string()]),
            content: Some(vec![" ".to_string()]),
            path: None,
        };
        assert!(!CategoryConfig::has_valid_criteria(&criteria));
    }
}

#[cfg(test)]
mod graph_config_tests {
    use super::*;
    use tempfile::TempDir;

    // === VALID CONFIGURATION TESTS ===

    #[test]
    fn test_graph_config_default() {
        let config = GraphConfig::new();
        assert_eq!(config.get_max_related_chunks(), 20);
        assert_eq!(config.get_hnsw_m(), 16);
        assert_eq!(config.get_hnsw_ef_construction(), 200);
    }

    #[test]
    fn test_graph_config_default_trait() {
        let config = GraphConfig::default();
        assert_eq!(config.get_max_related_chunks(), 20);
        assert_eq!(config.get_hnsw_m(), 16);
        assert_eq!(config.get_hnsw_ef_construction(), 200);
    }

    #[test]
    fn test_graph_config_load_valid_yaml() -> anyhow::Result<()> {
        let yaml_content = r#"
max_related_chunks: 25
hnsw_m: 20
hnsw_ef_construction: 300
"#;
        let temp = TempDir::new()?;
        let config_path = temp.path().join("graph_config.yaml");
        fs::write(&config_path, yaml_content)?;

        let config = GraphConfig::load_from_file(&config_path)?;
        assert_eq!(config.get_max_related_chunks(), 25);
        assert_eq!(config.get_hnsw_m(), 20);
        assert_eq!(config.get_hnsw_ef_construction(), 300);
        Ok(())
    }

    #[test]
    fn test_graph_config_with_params_valid() -> anyhow::Result<()> {
        let config = GraphConfig::with_params(50, 32, 400)?;
        assert_eq!(config.get_max_related_chunks(), 50);
        assert_eq!(config.get_hnsw_m(), 32);
        assert_eq!(config.get_hnsw_ef_construction(), 400);
        Ok(())
    }

    #[test]
    fn test_graph_config_with_params_min_values() -> anyhow::Result<()> {
        let config = GraphConfig::with_params(1, 4, 50)?;
        assert_eq!(config.get_max_related_chunks(), 1);
        assert_eq!(config.get_hnsw_m(), 4);
        assert_eq!(config.get_hnsw_ef_construction(), 50);
        Ok(())
    }

    #[test]
    fn test_graph_config_with_params_max_values() -> anyhow::Result<()> {
        let config = GraphConfig::with_params(1000, 64, 1000)?;
        assert_eq!(config.get_max_related_chunks(), 1000);
        assert_eq!(config.get_hnsw_m(), 64);
        assert_eq!(config.get_hnsw_ef_construction(), 1000);
        Ok(())
    }

    // === max_related_chunks VALIDATION TESTS ===

    #[test]
    fn test_reject_max_related_chunks_zero() {
        let result = GraphConfig::with_params(0, 16, 200);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("max_related_chunks"));
            assert!(err_msg.contains("at least 1"));
        }
    }

    #[test]
    fn test_reject_max_related_chunks_too_large() {
        let result = GraphConfig::with_params(1001, 16, 200);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("max_related_chunks"));
            assert!(err_msg.contains("at most 1000"));
        }
    }

    #[test]
    fn test_reject_max_related_chunks_way_too_large() {
        let result = GraphConfig::with_params(1_000_000, 16, 200);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("max_related_chunks"));
        }
    }

    #[test]
    fn test_load_yaml_max_related_chunks_zero() -> anyhow::Result<()> {
        let yaml_content = r#"
max_related_chunks: 0
hnsw_m: 16
hnsw_ef_construction: 200
"#;
        let temp = TempDir::new()?;
        let config_path = temp.path().join("graph_config.yaml");
        fs::write(&config_path, yaml_content)?;

        let result = GraphConfig::load_from_file(&config_path);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("max_related_chunks"));
        }
        Ok(())
    }

    // === hnsw_m VALIDATION TESTS ===

    #[test]
    fn test_reject_hnsw_m_too_small() {
        let result = GraphConfig::with_params(20, 3, 200);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("hnsw_m"));
            assert!(err_msg.contains("at least 4"));
        }
    }

    #[test]
    fn test_reject_hnsw_m_zero() {
        let result = GraphConfig::with_params(20, 0, 200);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("hnsw_m"));
        }
    }

    #[test]
    fn test_reject_hnsw_m_too_large() {
        let result = GraphConfig::with_params(20, 65, 200);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("hnsw_m"));
            assert!(err_msg.contains("at most 64"));
        }
    }

    #[test]
    fn test_reject_hnsw_m_way_too_large() {
        let result = GraphConfig::with_params(20, 256, 200);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("hnsw_m"));
        }
    }

    #[test]
    fn test_load_yaml_hnsw_m_too_small() -> anyhow::Result<()> {
        let yaml_content = r#"
max_related_chunks: 20
hnsw_m: 2
hnsw_ef_construction: 200
"#;
        let temp = TempDir::new()?;
        let config_path = temp.path().join("graph_config.yaml");
        fs::write(&config_path, yaml_content)?;

        let result = GraphConfig::load_from_file(&config_path);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("hnsw_m"));
        }
        Ok(())
    }

    // === hnsw_ef_construction VALIDATION TESTS ===

    #[test]
    fn test_reject_hnsw_ef_construction_too_small() {
        let result = GraphConfig::with_params(20, 16, 49);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("hnsw_ef_construction"));
            assert!(err_msg.contains("at least 50"));
        }
    }

    #[test]
    fn test_reject_hnsw_ef_construction_zero() {
        let result = GraphConfig::with_params(20, 16, 0);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("hnsw_ef_construction"));
        }
    }

    #[test]
    fn test_reject_hnsw_ef_construction_too_large() {
        let result = GraphConfig::with_params(20, 16, 1001);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("hnsw_ef_construction"));
            assert!(err_msg.contains("at most 1000"));
        }
    }

    #[test]
    fn test_reject_hnsw_ef_construction_way_too_large() {
        let result = GraphConfig::with_params(20, 16, 10000);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("hnsw_ef_construction"));
        }
    }

    #[test]
    fn test_load_yaml_hnsw_ef_construction_too_small() -> anyhow::Result<()> {
        let yaml_content = r#"
max_related_chunks: 20
hnsw_m: 16
hnsw_ef_construction: 25
"#;
        let temp = TempDir::new()?;
        let config_path = temp.path().join("graph_config.yaml");
        fs::write(&config_path, yaml_content)?;

        let result = GraphConfig::load_from_file(&config_path);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("hnsw_ef_construction"));
        }
        Ok(())
    }

    // === MULTIPLE PARAMETER FAILURES ===

    #[test]
    fn test_reject_all_parameters_invalid() {
        let result = GraphConfig::with_params(0, 2, 25);
        assert!(result.is_err());
        // Should report the first validation failure (max_related_chunks)
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("max_related_chunks"));
        }
    }

    #[test]
    fn test_load_yaml_multiple_invalid_parameters() -> anyhow::Result<()> {
        let yaml_content = r#"
max_related_chunks: 2000
hnsw_m: 100
hnsw_ef_construction: 10000
"#;
        let temp = TempDir::new()?;
        let config_path = temp.path().join("graph_config.yaml");
        fs::write(&config_path, yaml_content)?;

        let result = GraphConfig::load_from_file(&config_path);
        assert!(result.is_err());
        // Should report the first validation failure
        if let Err(e) = result {
            let err_msg = e.to_string();
            // ConfigError provides specific error messages
            assert!(err_msg.contains("max_related_chunks") || err_msg.contains("too large"));
        }
        Ok(())
    }

    // === EDGE CASES AND BOUNDARY TESTS ===

    #[test]
    fn test_boundary_max_related_chunks_low() -> anyhow::Result<()> {
        // Test boundaries near minimum
        for value in 1..=5 {
            let config = GraphConfig::with_params(value, 16, 200)?;
            assert_eq!(config.get_max_related_chunks(), value);
        }
        Ok(())
    }

    #[test]
    fn test_boundary_max_related_chunks_high() -> anyhow::Result<()> {
        // Test boundaries near maximum
        for value in 996..=1000 {
            let config = GraphConfig::with_params(value, 16, 200)?;
            assert_eq!(config.get_max_related_chunks(), value);
        }
        Ok(())
    }

    #[test]
    fn test_boundary_hnsw_m_low() -> anyhow::Result<()> {
        // Test boundaries near minimum
        for value in 4..=8 {
            let config = GraphConfig::with_params(20, value, 200)?;
            assert_eq!(config.get_hnsw_m(), value);
        }
        Ok(())
    }

    #[test]
    fn test_boundary_hnsw_m_high() -> anyhow::Result<()> {
        // Test boundaries near maximum
        for value in 60..=64 {
            let config = GraphConfig::with_params(20, value, 200)?;
            assert_eq!(config.get_hnsw_m(), value);
        }
        Ok(())
    }

    #[test]
    fn test_boundary_hnsw_ef_construction_low() -> anyhow::Result<()> {
        // Test boundaries near minimum
        for value in 50..=55 {
            let config = GraphConfig::with_params(20, 16, value)?;
            assert_eq!(config.get_hnsw_ef_construction(), value);
        }
        Ok(())
    }

    #[test]
    fn test_boundary_hnsw_ef_construction_high() -> anyhow::Result<()> {
        // Test boundaries near maximum
        for value in 995..=1000 {
            let config = GraphConfig::with_params(20, 16, value)?;
            assert_eq!(config.get_hnsw_ef_construction(), value);
        }
        Ok(())
    }

    // === LOAD FROM FILE EDGE CASES ===

    #[test]
    fn test_load_yaml_missing_file() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let config_path = temp.path().join("nonexistent.yaml");
        let result = GraphConfig::load_from_file(&config_path);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_load_yaml_malformed() -> anyhow::Result<()> {
        let yaml_content = r#"
max_related_chunks: "not_a_number"
hnsw_m: 16
hnsw_ef_construction: 200
"#;
        let temp = TempDir::new()?;
        let config_path = temp.path().join("graph_config.yaml");
        fs::write(&config_path, yaml_content)?;

        let result = GraphConfig::load_from_file(&config_path);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_load_yaml_all_defaults_work() {
        // Verify that defaults can be loaded
        let config = GraphConfig::default();
        assert!(config.get_max_related_chunks() >= 1 && config.get_max_related_chunks() <= 1000);
        assert!(config.get_hnsw_m() >= 4 && config.get_hnsw_m() <= 64);
        assert!(
            config.get_hnsw_ef_construction() >= 50 && config.get_hnsw_ef_construction() <= 1000
        );
    }

    #[test]
    fn test_clone_preserves_validation() -> anyhow::Result<()> {
        let config = GraphConfig::with_params(100, 32, 500)?;
        let cloned = config.clone();
        assert_eq!(cloned.max_related_chunks.get(), 100);
        assert_eq!(cloned.hnsw_m.get(), 32);
        assert_eq!(cloned.hnsw_ef_construction.get(), 500);
        Ok(())
    }

    #[test]
    fn test_serialization_round_trip() -> anyhow::Result<()> {
        let config = GraphConfig::with_params(50, 24, 350)?;
        let yaml = serde_yaml::to_string(&config)?;
        let deserialized: GraphConfig = serde_yaml::from_str(&yaml)?;
        assert_eq!(
            config.get_max_related_chunks(),
            deserialized.max_related_chunks
        );
        assert_eq!(config.get_hnsw_m(), deserialized.hnsw_m);
        assert_eq!(
            config.get_hnsw_ef_construction(),
            deserialized.hnsw_ef_construction
        );
        Ok(())
    }
}
