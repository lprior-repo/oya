use oya_contracts::{
    CompensationDiagnostic, LifecycleGateSnapshot, LifecycleStatusSnapshot, LifecycleStepSnapshot,
};

#[test]
fn status_dto_roundtrip_frontend_consumes_shared_contracts() {
    let snapshot = LifecycleStatusSnapshot {
        bead_id: Some("demo".to_owned()),
        steps: vec![LifecycleStepSnapshot {
            step: "verify".to_owned(),
            status: "succeeded".to_owned(),
            message: Some("green".to_owned()),
            details: None,
            started_at: Some("2026-04-29T00:00:00Z".to_owned()),
            finished_at: Some("2026-04-29T00:00:01Z".to_owned()),
            duration_ms: Some(1_000),
        }],
        gates: vec![LifecycleGateSnapshot {
            gate_id: "fmt".to_owned(),
            status: "passed".to_owned(),
            message: None,
        }],
        discipline_gates: Vec::new(),
        state: None,
        pr_url: None,
        done: true,
        success: Some(true),
        message: Some("completed".to_owned()),
        compensation_diagnostics: vec![CompensationDiagnostic {
            compensation_type: "none".to_owned(),
            target: "demo".to_owned(),
            success: true,
            error: None,
        }],
    };

    let encoded = serde_json::to_string(&snapshot).map_err(|error| error.to_string());
    let Ok(json) = encoded else { panic_free_test_failure("serialize shared status dto") };
    let decoded =
        serde_json::from_str::<LifecycleStatusSnapshot>(&json).map_err(|error| error.to_string());
    let Ok(roundtrip) = decoded else { panic_free_test_failure("deserialize shared status dto") };

    assert_eq!(roundtrip, snapshot);
}

fn panic_free_test_failure(context: &str) -> ! {
    assert!(false, "{context}");
    std::process::abort();
}
