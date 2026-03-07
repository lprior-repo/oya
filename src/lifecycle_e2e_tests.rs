#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use crate::lifecycle::types::{BeadData, BeadId, Model};

    fn create_test_bead() -> BeadData {
        let bead_id = BeadId::parse("test-001").expect("valid bead id");
        BeadData::from_bead_id(bead_id)
    }

    fn test_model() -> Model {
        Model::default_model()
    }

    #[tokio::test]
    async fn e2e_test_discovery_finds_ready_beads() {
        let result = discover_ready_beads().await;
        assert!(result.is_ok(), "Discovery should find ready beads without error");
        
        let beads = result.expect("discovery result");
        assert!(!beads.is_empty(), "Should discover at least one ready bead");
        
        for bead in &beads {
            assert!(!bead.bead_id.as_str().is_empty(), "Bead ID should not be empty");
            assert!(!bead.workspace.as_str().is_empty(), "Workspace name should not be empty");
        }
    }

    #[tokio::test]
    async fn e2e_test_discovery_respects_dependencies() {
        let result = discover_ready_beads().await;
        assert!(result.is_ok());
        
        let beads = result.expect("discovery result");
        
        for bead in &beads {
            let has_unmet_deps = check_unmet_dependencies(bead.bead_id.as_str()).await;
            assert!(!has_unmet_deps, "Ready bead should have no unmet dependencies");
        }
    }

    #[tokio::test]
    async fn e2e_test_quality_gate_enforces_threshold() {
        let bead = create_test_bead();
        
        let quality_score = calculate_quality_score(bead.bead_id.as_str()).await;
        assert!(quality_score.is_ok(), "Quality calculation should succeed");
        
        let score = quality_score.expect("quality score");
        
        if score < 70 {
            let gate_result = attempt_gate_transition(&bead, "planning").await;
            assert!(
                gate_result.is_err(),
                "Gate transition should fail for score < 70"
            );
        } else {
            let gate_result = attempt_gate_transition(&bead, "planning").await;
            assert!(
                gate_result.is_ok(),
                "Gate transition should succeed for score >= 70"
            );
        }
    }

    #[tokio::test]
    async fn e2e_test_planning_generates_lifecycle_steps() {
        let bead = create_test_bead();
        let model = test_model();
        
        let steps = plan_lifecycle_steps(&bead, &model).await;
        assert!(steps.is_ok(), "Planning should create lifecycle steps");
        
        let lifecycle_steps = steps.expect("lifecycle steps");
        assert!(!lifecycle_steps.is_empty(), "Should have at least one lifecycle step");
        
        let step_names: Vec<&str> = lifecycle_steps.iter().map(|s| s.name.as_str()).collect();
        
        assert!(
            step_names.contains(&"mark_in_progress"),
            "Should include mark_in_progress step"
        );
        assert!(
            step_names.contains(&"workspace_add"),
            "Should include workspace_add step"
        );
        assert!(
            step_names.contains(&"opencode"),
            "Should include opencode step"
        );
        assert!(
            step_names.contains(&"moon_ci"),
            "Should include moon_ci step"
        );
    }

    #[tokio::test]
    async fn e2e_test_planning_validates_dag() {
        let bead = create_test_bead();
        let model = test_model();
        
        let steps = plan_lifecycle_steps(&bead, &model).await;
        assert!(steps.is_ok());
        
        let lifecycle_steps = steps.expect("lifecycle steps");
        
        let dag_valid = validate_lifecycle_dag(&lifecycle_steps).await;
        assert!(dag_valid.is_ok(), "Lifecycle DAG should be valid");
        assert!(dag_valid.expect("dag validation"), "No circular dependencies in steps");
    }

    #[tokio::test]
    async fn e2e_test_full_pipeline_discovery_to_planning() {
        let discovery_result = discover_ready_beads().await;
        assert!(discovery_result.is_ok(), "Discovery phase should succeed");
        
        let beads = discovery_result.expect("discovered beads");
        assert!(!beads.is_empty(), "Should discover ready beads");
        
        let first_bead = &beads[0];
        
        let quality_result = calculate_quality_score(first_bead.bead_id.as_str()).await;
        assert!(quality_result.is_ok(), "Quality calculation should succeed");
        
        let score = quality_result.expect("quality score");
        
        if score >= 70 {
            let gate_result = attempt_gate_transition(first_bead, "planning").await;
            assert!(
                gate_result.is_ok(),
                "Gate transition should succeed for valid score"
            );
            
            let model = test_model();
            let planning_result = plan_lifecycle_steps(first_bead, &model).await;
            assert!(planning_result.is_ok(), "Planning phase should succeed");
            
            let steps = planning_result.expect("lifecycle steps");
            assert!(!steps.is_empty(), "Should generate lifecycle steps");
            
            let dag_result = validate_lifecycle_dag(&steps).await;
            assert!(dag_result.is_ok(), "DAG validation should succeed");
        }
    }

    #[tokio::test]
    async fn e2e_test_pipeline_creates_beads_in_database() {
        let initial_count = count_beads_in_database().await;
        assert!(initial_count.is_ok());
        
        let before = initial_count.expect("initial bead count");
        
        let bead = create_test_bead();
        let execution_result = execute_failing_lifecycle(&bead).await;
        drop(execution_result);
        
        let after_count = count_beads_in_database().await;
        assert!(after_count.is_ok());
        
        let after = after_count.expect("final bead count");
        
        assert!(after > before, "Bead count should increase after execution");
    }

    #[tokio::test]
    async fn e2e_test_pipeline_environment_cleanup() {
        let bead = create_test_bead();
        let workspace_path = bead.workspace_path.clone();
        
        let execution_result = execute_failing_lifecycle(&bead).await;
        drop(execution_result);
        
        let cleanup_result = verify_workspace_cleanup(&workspace_path).await;
        assert!(cleanup_result.is_ok(), "Workspace should be cleaned up after execution");
    }

    #[tokio::test]
    async fn e2e_test_quality_score_color_progression() {
        let score = 45u32;
        let color = get_quality_score_color(score).await;
        assert_eq!(color.expect("color"), "red", "Score 45 should be red");
        
        let score = 60u32;
        let color = get_quality_score_color(score).await;
        assert_eq!(color.expect("color"), "yellow", "Score 60 should be yellow");
        
        let score = 75u32;
        let color = get_quality_score_color(score).await;
        assert_eq!(color.expect("color"), "green", "Score 75 should be green");
        
        let score = 95u32;
        let color = get_quality_score_color(score).await;
        assert_eq!(color.expect("color"), "bright-green", "Score 95 should be bright green");
    }

    async fn discover_ready_beads() -> Result<Vec<BeadData>, Box<dyn std::error::Error>> {
        Err("discover_ready_beads not implemented".into())
    }

    async fn check_unmet_dependencies(_bead_id: &str) -> bool {
        false
    }

    async fn calculate_quality_score(_bead_id: &str) -> Result<u32, Box<dyn std::error::Error>> {
        Err("calculate_quality_score not implemented".into())
    }

    async fn attempt_gate_transition(
        _bead: &BeadData,
        _phase: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("attempt_gate_transition not implemented".into())
    }

    async fn plan_lifecycle_steps(
        _bead: &BeadData,
        _model: &Model,
    ) -> Result<Vec<crate::lifecycle::workflow::steps::LifecycleStep>, Box<dyn std::error::Error>> {
        Err("plan_lifecycle_steps not implemented".into())
    }

    async fn validate_lifecycle_dag(
        _steps: &[crate::lifecycle::workflow::steps::LifecycleStep],
    ) -> Result<bool, Box<dyn std::error::Error>> {
        Err("validate_lifecycle_dag not implemented".into())
    }

    async fn count_beads_in_database() -> Result<usize, Box<dyn std::error::Error>> {
        Err("count_beads_in_database not implemented".into())
    }

    async fn execute_failing_lifecycle(
        _bead: &BeadData,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("execute_failing_lifecycle not implemented".into())
    }

    async fn verify_workspace_cleanup(_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        Err("verify_workspace_cleanup not implemented".into())
    }

    async fn get_quality_score_color(_score: u32) -> Result<&'static str, Box<dyn std::error::Error>> {
        Err("get_quality_score_color not implemented".into())
    }
}
