//! Pipeline-related Tauri commands
//!
//! Provides commands for:
//! - Getting pipeline stage definitions
//! - Running individual stages
//! - Getting pipeline state
//!
//! These commands use Tauri's native IPC for ~10μs round-trip latency.

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use oya_shared::{PipelineState, StageEvent, StageInfo, StageStatus};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};

/// Get the standard pipeline stages
///
/// Returns the 9-stage pipeline definition with dependencies.
/// This is a fast, synchronous call with no I/O.
#[tauri::command]
pub fn get_pipeline_stages() -> Vec<StageInfo> {
    PipelineState::standard().stages
}

/// Get current pipeline state for a task
///
/// Returns the full pipeline state including stage statuses.
/// Uses in-memory state if available, otherwise returns fresh state.
#[tauri::command]
pub async fn get_pipeline_state(
    task_id: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<PipelineState> {
    // Try to get from cache
    if let Some(pipeline) = state.get_pipeline_state(&task_id).await {
        return Ok(pipeline);
    }

    // Return fresh standard pipeline
    let pipeline = PipelineState::standard();
    state.set_pipeline_state(task_id, pipeline.clone()).await;
    Ok(pipeline)
}

/// Run a single pipeline stage
///
/// Executes the specified stage and emits progress events.
/// Returns the stage result after completion.
#[tauri::command]
pub async fn run_stage(
    task_id: String,
    stage_name: String,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> AppResult<StageEvent> {
    // Get current pipeline state
    let mut pipeline = state
        .get_pipeline_state(&task_id)
        .await
        .unwrap_or_else(PipelineState::standard);

    // Validate stage exists
    if pipeline.find_stage(&stage_name).is_none() {
        return Err(AppError::Pipeline(format!("Unknown stage: {stage_name}")));
    }

    // Check dependencies
    let stages_snapshot = pipeline.stages.clone();
    let stage = pipeline
        .find_stage(&stage_name)
        .ok_or_else(|| AppError::Pipeline(format!("Stage not found: {stage_name}")))?;

    if !stage.can_run(&stages_snapshot) {
        return Err(AppError::Pipeline(format!(
            "Stage '{stage_name}' cannot run - dependencies not satisfied"
        )));
    }

    // Mark stage as running
    pipeline.update_stage(&stage_name, StageStatus::Running, None, None);
    state
        .set_pipeline_state(task_id.clone(), pipeline.clone())
        .await;

    // Emit started event
    let started = StageEvent::started(&stage_name);
    let _ = app.emit("stage-event", &started);

    // Execute stage (simulated for now - will wire to actual pipeline executor)
    let start = Instant::now();
    let result = execute_stage_mock(&stage_name).await;
    let duration_ms = start.elapsed().as_millis() as u64;

    // Update state based on result
    let event = match result {
        Ok(()) => {
            pipeline.update_stage(&stage_name, StageStatus::Passed, Some(duration_ms), None);
            StageEvent::passed(&stage_name, duration_ms)
        }
        Err(e) => {
            let error_msg = e.to_string();
            pipeline.update_stage(
                &stage_name,
                StageStatus::Failed,
                Some(duration_ms),
                Some(error_msg.clone()),
            );

            // Skip dependent stages
            skip_dependents(&mut pipeline, &stage_name);

            StageEvent::failed(&stage_name, duration_ms, error_msg)
        }
    };

    // Save updated state
    state.set_pipeline_state(task_id, pipeline).await;

    // Emit completion event
    let _ = app.emit("stage-event", &event);

    Ok(event)
}

/// Run all pending stages in sequence
///
/// Executes stages in dependency order, stopping on first failure
/// if using StopOnFirst strategy.
#[tauri::command]
pub async fn run_pipeline(
    task_id: String,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> AppResult<PipelineState> {
    let mut pipeline = state
        .get_pipeline_state(&task_id)
        .await
        .unwrap_or_else(PipelineState::standard);

    // Run stages in order
    for stage_name in get_execution_order() {
        // Check if stage can run
        let stages_snapshot = pipeline.stages.clone();
        if let Some(stage) = pipeline.find_stage(&stage_name) {
            if !stage.can_run(&stages_snapshot) {
                continue;
            }
        } else {
            continue;
        }

        // Mark running
        pipeline.update_stage(&stage_name, StageStatus::Running, None, None);
        state
            .set_pipeline_state(task_id.clone(), pipeline.clone())
            .await;
        let _ = app.emit("stage-event", &StageEvent::started(&stage_name));

        // Execute
        let start = Instant::now();
        let result = execute_stage_mock(&stage_name).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(()) => {
                pipeline.update_stage(&stage_name, StageStatus::Passed, Some(duration_ms), None);
                let _ = app.emit("stage-event", &StageEvent::passed(&stage_name, duration_ms));
            }
            Err(e) => {
                let error_msg = e.to_string();
                pipeline.update_stage(
                    &stage_name,
                    StageStatus::Failed,
                    Some(duration_ms),
                    Some(error_msg.clone()),
                );
                let _ = app.emit(
                    "stage-event",
                    &StageEvent::failed(&stage_name, duration_ms, &error_msg),
                );

                // Skip dependents and stop
                skip_dependents(&mut pipeline, &stage_name);
                break;
            }
        }

        state
            .set_pipeline_state(task_id.clone(), pipeline.clone())
            .await;
    }

    Ok(pipeline)
}

/// Reset pipeline state for a task
#[tauri::command]
pub async fn reset_pipeline(
    task_id: String,
    state: State<'_, Arc<AppState>>,
) -> AppResult<PipelineState> {
    let pipeline = PipelineState::standard();
    state.set_pipeline_state(task_id, pipeline.clone()).await;
    Ok(pipeline)
}

// Helper: Get stage execution order (topological sort respecting dependencies)
fn get_execution_order() -> Vec<String> {
    vec![
        "implement".to_string(),
        "unit-test".to_string(),
        "lint".to_string(),
        "coverage".to_string(),
        "static".to_string(),
        "integration".to_string(),
        "security".to_string(),
        "review".to_string(),
        "accept".to_string(),
    ]
}

// Helper: Skip all stages that depend on a failed stage
fn skip_dependents(pipeline: &mut PipelineState, failed_stage: &str) {
    let dependents: Vec<String> = pipeline
        .stages
        .iter()
        .filter(|s| s.depends_on.contains(&failed_stage.to_string()))
        .map(|s| s.name.clone())
        .collect();

    for dep_name in dependents {
        pipeline.update_stage(&dep_name, StageStatus::Skipped, None, None);
        // Recursively skip further dependents
        skip_dependents(pipeline, &dep_name);
    }
}

// Mock stage execution - will be replaced with actual pipeline executor
async fn execute_stage_mock(stage_name: &str) -> Result<(), AppError> {
    // Simulate execution time (50-200ms per stage)
    let delay = match stage_name {
        "implement" => 100,
        "unit-test" => 150,
        "coverage" => 200,
        "lint" => 50,
        "static" => 100,
        "integration" => 200,
        "security" => 150,
        "review" => 50,
        "accept" => 25,
        _ => 100,
    };

    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

    // For demo, all stages pass
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_execution_order() {
        let order = get_execution_order();
        assert_eq!(order[0], "implement");
        assert_eq!(order.len(), 9);
    }

    #[test]
    fn test_skip_dependents() {
        let mut pipeline = PipelineState::standard();

        // Skip implement's dependents
        skip_dependents(&mut pipeline, "implement");

        // unit-test depends on implement
        assert!(matches!(
            pipeline.find_stage("unit-test").map(|s| s.status),
            Some(StageStatus::Skipped)
        ));

        // lint also depends on implement
        assert!(matches!(
            pipeline.find_stage("lint").map(|s| s.status),
            Some(StageStatus::Skipped)
        ));
    }
}
