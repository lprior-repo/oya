use serde_json::json;

mod common;

#[tokio::test]
async fn test_pipeline_prompt_includes_bead_state() {
    let bead_id = "src-1l6n";
    let bead_state = json!({
        "id": bead_id,
        "title": "ruby pokemon pipeline test",
        "description": "Verify OIA implement flow pulls bead state from Restate pipeline handler",
        "status": "in_progress"
    });

    let prompt_result = oya::restate_oia::pipeline_prompt(bead_id, bead_state.clone());
    assert!(prompt_result.is_ok());
    let prompt = match prompt_result {
        Ok(value) => value,
        Err(_) => return,
    };

    let prompt_text = prompt.into_inner();

    assert!(prompt_text.contains("Implement bead src-1l6n"));
    assert!(prompt_text.contains("Bead State:"));
    assert!(prompt_text.contains("ruby pokemon pipeline test"));
    assert!(prompt_text.contains("Verify OIA implement flow"));
    assert!(prompt_text.contains("moon run :check"));
}

#[tokio::test]
async fn test_pipeline_prompt_validates_bead_state_json() {
    let bead_state = json!({
        "id": "test-123",
        "title": "Test Task"
    });

    let result = oya::restate_oia::pipeline_prompt("test-123", bead_state);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_bead_sync_request_structure() {
    let bead_id = "src-1l6n".to_string();
    let bead_status = "in_progress".to_string();
    let bead_state = json!({
        "id": &bead_id,
        "title": "test",
        "status": &bead_status
    });

    let request = oya::restate_oia::BeadSyncRequest {
        bead_id: bead_id.clone(),
        bead_status: bead_status.clone(),
        bead_state: bead_state.clone(),
    };

    assert_eq!(request.bead_id, bead_id);
    assert_eq!(request.bead_status, bead_status);
    assert_eq!(request.bead_state, bead_state);
}

#[tokio::test]
async fn test_pipeline_request_structure() {
    let request =
        oya::restate_oia::PipelineRequest { model: Some("openai/gpt-5.3-codex".to_string()) };

    assert!(request.model.is_some());
}

#[tokio::test]
async fn test_start_request_with_bead_state() {
    let bead_id = "src-1l6n".to_string();
    let bead_status = "in_progress".to_string();
    let bead_state = json!({
        "id": &bead_id,
        "title": "test",
        "status": &bead_status
    });

    let request = oya::restate_oia::StartRequest {
        prompt: "test prompt".to_string(),
        model: Some("test-model".to_string()),
        bead_id: Some(bead_id.clone()),
        bead_status: Some(bead_status.clone()),
        bead_state: Some(bead_state.clone()),
    };

    assert_eq!(request.bead_id, Some(bead_id));
    assert_eq!(request.bead_status, Some(bead_status));
    assert_eq!(request.bead_state, Some(bead_state));
}
