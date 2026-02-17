#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use crate::orchestration::{
    ApproverMode, BeadId, FailureCategory, Run, RunState, ShipDecision, StageAttempt, StageName,
    StageResult, StageState,
};
use crate::persistence::{OyaDb, OyaDbError};
use chrono::Utc;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipelineConfig {
    pub max_attempts_per_stage: u32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_attempts_per_stage: 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PipelineOutcome {
    Shipped { run_id: String },
    Failed { run_id: String, reason: String },
}

#[derive(Clone, Debug)]
pub struct StageExecution {
    pub passed: bool,
    pub output: serde_json::Value,
    pub failure_category: Option<FailureCategory>,
}

impl StageExecution {
    pub fn pass(output: serde_json::Value) -> Self {
        Self {
            passed: true,
            output,
            failure_category: None,
        }
    }

    pub fn fail(output: serde_json::Value, failure_category: FailureCategory) -> Self {
        Self {
            passed: false,
            output,
            failure_category: Some(failure_category),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DagNode {
    pub stage: StageName,
    pub depends_on: Vec<StageName>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipelineDag {
    pub nodes: Vec<DagNode>,
}

impl PipelineDag {
    pub fn default_eight_step() -> Self {
        Self {
            nodes: vec![
                DagNode {
                    stage: StageName::Contract,
                    depends_on: vec![],
                },
                DagNode {
                    stage: StageName::DesignDag,
                    depends_on: vec![StageName::Contract],
                },
                DagNode {
                    stage: StageName::Implement,
                    depends_on: vec![StageName::DesignDag],
                },
                DagNode {
                    stage: StageName::Tdd15,
                    depends_on: vec![StageName::Implement],
                },
                DagNode {
                    stage: StageName::Qa,
                    depends_on: vec![StageName::Tdd15],
                },
                DagNode {
                    stage: StageName::RedQueen,
                    depends_on: vec![StageName::Qa],
                },
                DagNode {
                    stage: StageName::GptReview,
                    depends_on: vec![StageName::RedQueen],
                },
                DagNode {
                    stage: StageName::ShipGate,
                    depends_on: vec![StageName::GptReview],
                },
            ],
        }
    }

    fn node_for(&self, stage: &StageName) -> Option<&DagNode> {
        self.nodes.iter().find(|node| &node.stage == stage)
    }

    pub fn terminal_nodes(&self) -> Vec<StageName> {
        let depended_on: HashSet<StageName> = self
            .nodes
            .iter()
            .flat_map(|node| node.depends_on.clone())
            .collect();

        self.nodes
            .iter()
            .filter(|node| !depended_on.contains(&node.stage))
            .map(|node| node.stage.clone())
            .collect()
    }

    pub fn recursive_order(&self, terminal: &StageName) -> Result<Vec<StageName>, OyaDbError> {
        fn visit(
            dag: &PipelineDag,
            stage: &StageName,
            visiting: &mut HashSet<StageName>,
            visited: &mut HashSet<StageName>,
            order: &mut Vec<StageName>,
        ) -> Result<(), OyaDbError> {
            if visited.contains(stage) {
                return Ok(());
            }
            if visiting.contains(stage) {
                return Err(OyaDbError::Serialization(format!(
                    "cycle detected at {}",
                    stage.as_str()
                )));
            }

            let node = dag.node_for(stage).ok_or_else(|| {
                OyaDbError::Serialization(format!("missing DAG node for {}", stage.as_str()))
            })?;

            visiting.insert(stage.clone());
            for dependency in &node.depends_on {
                visit(dag, dependency, visiting, visited, order)?;
            }
            visiting.remove(stage);
            visited.insert(stage.clone());
            order.push(stage.clone());
            Ok(())
        }

        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        visit(self, terminal, &mut visiting, &mut visited, &mut order)?;
        Ok(order)
    }
}

fn stage_as_key(stage: &StageName) -> Result<String, OyaDbError> {
    serde_json::to_string(stage).map_err(|e| OyaDbError::Serialization(e.to_string()))
}

async fn run_stage_with_retry<F>(
    db: &OyaDb,
    run_id: &str,
    stage: StageName,
    context: &str,
    config: &PipelineConfig,
    mut previous_result: Option<StageResult>,
    executor: &mut F,
) -> Result<Option<String>, OyaDbError>
where
    F: FnMut(StageName, u32, &str, Option<&StageResult>) -> StageExecution,
{
    let mut attempt = 1u32;
    loop {
        let stage_attempt = StageAttempt {
            run_id: run_id.to_string(),
            stage: stage.clone(),
            attempt,
            session_id: None,
            state: StageState::Running,
            started_at: Utc::now(),
            completed_at: None,
        };
        db.insert_stage_attempt(&stage_attempt).await?;

        let execution = executor(stage.clone(), attempt, context, previous_result.as_ref());
        let stage_result = StageResult {
            run_id: run_id.to_string(),
            stage: stage.clone(),
            attempt,
            passed: execution.passed,
            output: execution.output,
            failure_category: execution.failure_category,
            next_stage: stage.next(),
        };

        db.insert_stage_result(&stage_result).await?;

        let stage_key = stage_as_key(&stage)?;
        let attempt_state = if stage_result.passed {
            "passed"
        } else {
            "failed"
        };
        db.update_stage_attempt_state(run_id, &stage_key, attempt, attempt_state)
            .await?;

        if stage_result.passed {
            return Ok(None);
        }

        if attempt >= config.max_attempts_per_stage {
            return Ok(Some(format!(
                "stage {} failed after {} attempts",
                stage.as_str(),
                config.max_attempts_per_stage
            )));
        }

        previous_result = Some(stage_result);
        attempt += 1;
    }
}

pub async fn run_pipeline<F>(
    db: &OyaDb,
    bead_id: BeadId,
    context: &str,
    config: PipelineConfig,
    mut executor: F,
) -> Result<PipelineOutcome, OyaDbError>
where
    F: FnMut(StageName, u32, &str, Option<&StageResult>) -> StageExecution,
{
    let dag = PipelineDag::default_eight_step();
    let started_run = Run::new(bead_id)
        .start()
        .map_err(|e| OyaDbError::Serialization(e.to_string()))?;

    let run_id = started_run.id.as_str().to_string();
    db.insert_bead_run(&started_run).await?;

    let mut executed: HashSet<StageName> = HashSet::new();

    for terminal in dag.terminal_nodes() {
        for stage in dag.recursive_order(&terminal)? {
            if executed.contains(&stage) {
                continue;
            }

            if let Some(reason) = run_stage_with_retry(
                db,
                &run_id,
                stage.clone(),
                context,
                &config,
                None,
                &mut executor,
            )
            .await?
            {
                let failed = started_run.fail(reason.clone());
                db.update_run_state(&run_id, &failed.state).await?;
                return Ok(PipelineOutcome::Failed { run_id, reason });
            }

            executed.insert(stage);
        }
    }

    let shipped_state = RunState::Shipped {
        completed_at: Utc::now(),
    };
    db.update_run_state(&run_id, &shipped_state).await?;

    let decision = ShipDecision {
        run_id: run_id.clone(),
        shipped: true,
        rationale: "all eight DAG stages passed".to_string(),
        approver_mode: ApproverMode::Auto,
        timestamp: Utc::now(),
    };
    db.insert_ship_decision(&decision).await?;

    Ok(PipelineOutcome::Shipped { run_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn memory_db() -> Result<OyaDb, OyaDbError> {
        let db = OyaDb::connect("memory://").await?;
        db.init_schema().await?;
        Ok(db)
    }

    #[test]
    fn given_default_dag_when_listing_nodes_then_contains_all_eight_steps() {
        let dag = PipelineDag::default_eight_step();
        assert_eq!(dag.nodes.len(), 8);
    }

    #[test]
    fn given_default_dag_when_resolving_order_then_recursive_order_is_deterministic(
    ) -> Result<(), OyaDbError> {
        let dag = PipelineDag::default_eight_step();
        let order = dag.recursive_order(&StageName::ShipGate)?;
        assert_eq!(
            order,
            vec![
                StageName::Contract,
                StageName::DesignDag,
                StageName::Implement,
                StageName::Tdd15,
                StageName::Qa,
                StageName::RedQueen,
                StageName::GptReview,
                StageName::ShipGate,
            ]
        );
        Ok::<(), OyaDbError>(())
    }

    #[tokio::test]
    async fn given_all_stages_pass_when_running_pipeline_then_run_is_shipped(
    ) -> Result<(), OyaDbError> {
        let db = memory_db().await?;

        let outcome = run_pipeline(
            &db,
            BeadId::new("bead-1"),
            "context",
            PipelineConfig::default(),
            |_stage, _attempt, _ctx, _previous| {
                StageExecution::pass(serde_json::json!({"status": "ok"}))
            },
        )
        .await?;

        let run_id = match outcome {
            PipelineOutcome::Shipped { run_id } => run_id,
            PipelineOutcome::Failed { reason, .. } => {
                return Err(OyaDbError::Serialization(format!(
                    "unexpected failure: {reason}"
                )));
            }
        };

        let results = db.get_stage_results(&run_id).await?;
        assert_eq!(results.len(), 8);
        assert!(results.iter().all(|result| result.passed));

        let observed: HashSet<StageName> =
            results.iter().map(|result| result.stage.clone()).collect();
        let expected: HashSet<StageName> = [
            StageName::Contract,
            StageName::DesignDag,
            StageName::Implement,
            StageName::Tdd15,
            StageName::Qa,
            StageName::RedQueen,
            StageName::GptReview,
            StageName::ShipGate,
        ]
        .into_iter()
        .collect();
        assert_eq!(observed, expected);
        Ok(())
    }

    #[tokio::test]
    async fn given_contract_fails_once_when_running_pipeline_then_retry_occurs_and_ships(
    ) -> Result<(), OyaDbError> {
        let db = memory_db().await?;
        let mut failed_once = false;

        let outcome = run_pipeline(
            &db,
            BeadId::new("bead-2"),
            "context",
            PipelineConfig::default(),
            |stage, _attempt, _ctx, _previous| {
                if stage == StageName::Contract && !failed_once {
                    failed_once = true;
                    StageExecution::fail(
                        serde_json::json!({"status": "compile error"}),
                        FailureCategory::CompileFailed,
                    )
                } else {
                    StageExecution::pass(serde_json::json!({"status": "ok"}))
                }
            },
        )
        .await?;

        match outcome {
            PipelineOutcome::Shipped { .. } => Ok(()),
            PipelineOutcome::Failed { reason, .. } => Err(OyaDbError::Serialization(format!(
                "unexpected failure: {reason}"
            ))),
        }
    }

    #[tokio::test]
    async fn given_stage_keeps_failing_when_running_pipeline_then_run_fails_after_max_attempts(
    ) -> Result<(), OyaDbError> {
        let db = memory_db().await?;

        let outcome = run_pipeline(
            &db,
            BeadId::new("bead-3"),
            "context",
            PipelineConfig {
                max_attempts_per_stage: 2,
            },
            |_stage, _attempt, _ctx, _previous| {
                StageExecution::fail(
                    serde_json::json!({"status": "test failed"}),
                    FailureCategory::TestFailed,
                )
            },
        )
        .await?;

        match outcome {
            PipelineOutcome::Failed { reason, .. } => {
                assert!(reason.contains("failed after 2 attempts"));
                Ok(())
            }
            PipelineOutcome::Shipped { .. } => Err(OyaDbError::Serialization(
                "pipeline shipped unexpectedly".to_string(),
            )),
        }
    }
}
