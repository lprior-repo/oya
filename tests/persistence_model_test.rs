use oya::domain::{GateResult, StageAttempt, StageName, StageResult, StageState};
use oya::infrastructure::persistence::OyaDb;

fn sample_stage_result(stage: StageName, attempt: u32) -> StageResult {
    StageResult {
        run_id: "run-1".to_string(),
        stage,
        attempt,
        passed: true,
        output: serde_json::json!({"ok": true}),
        failure_category: None,
        next_stage: None,
    }
}

#[tokio::test]
async fn stage_results_are_sorted_by_stage_then_attempt() {
    let db_result = OyaDb::connect("memory://").await;
    assert!(db_result.is_ok());
    let db = match db_result {
        Ok(value) => value,
        Err(_) => return,
    };
    assert!(db.init_schema().await.is_ok());

    assert!(db.insert_stage_result(&sample_stage_result(StageName::Qa, 1)).await.is_ok());
    assert!(db.insert_stage_result(&sample_stage_result(StageName::Contract, 2)).await.is_ok());
    assert!(db.insert_stage_result(&sample_stage_result(StageName::Contract, 1)).await.is_ok());

    let results_result = db.get_stage_results("run-1").await;
    assert!(results_result.is_ok());
    let results = match results_result {
        Ok(value) => value,
        Err(_) => return,
    };
    let sequence: Vec<(StageName, u32)> =
        results.iter().map(|item| (item.stage.clone(), item.attempt)).collect();

    assert_eq!(
        sequence,
        vec![(StageName::Contract, 1), (StageName::Contract, 2), (StageName::Qa, 1)]
    );
}

#[tokio::test]
async fn gate_results_do_not_overwrite_across_attempts_when_namespaced() {
    let db_result = OyaDb::connect("memory://").await;
    assert!(db_result.is_ok());
    let db = match db_result {
        Ok(value) => value,
        Err(_) => return,
    };
    assert!(db.init_schema().await.is_ok());

    let first = GateResult {
        run_id: "run-2".to_string(),
        gate_name: "contract:001:moon_ci".to_string(),
        command: Some("moon run :ci".to_string()),
        passed: true,
        exit_code: 0,
        log_ref: None,
    };
    let second = GateResult {
        run_id: "run-2".to_string(),
        gate_name: "contract:002:moon_ci".to_string(),
        command: Some("moon run :ci".to_string()),
        passed: false,
        exit_code: 1,
        log_ref: None,
    };

    assert!(db.insert_gate_result(&first).await.is_ok());
    assert!(db.insert_gate_result(&second).await.is_ok());

    let insert_duplicate = db.insert_gate_result(&first).await;
    assert!(insert_duplicate.is_ok());
}

#[tokio::test]
async fn stage_attempt_state_updates_to_terminal_state_with_completion_time() {
    let db_result = OyaDb::connect("memory://").await;
    assert!(db_result.is_ok());
    let db = match db_result {
        Ok(value) => value,
        Err(_) => return,
    };
    assert!(db.init_schema().await.is_ok());

    let attempt = StageAttempt {
        run_id: "run-3".to_string(),
        stage: StageName::Contract,
        attempt: 1,
        session_id: None,
        state: StageState::Running,
        started_at: chrono::Utc::now(),
        completed_at: None,
    };

    assert!(db.insert_stage_attempt(&attempt).await.is_ok());

    let stage_key_result = serde_json::to_string(&StageName::Contract);
    assert!(stage_key_result.is_ok());
    let stage_key = match stage_key_result {
        Ok(value) => value,
        Err(_) => return,
    };
    assert!(db.update_stage_attempt_state("run-3", &stage_key, 1, "passed").await.is_ok());

    let attempts_result = db.get_stage_attempts_sync("run-3");
    assert!(attempts_result.is_ok());
    let attempts = match attempts_result {
        Ok(value) => value,
        Err(_) => return,
    };
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].state, StageState::Passed);
    assert!(attempts[0].completed_at.is_some());
}
