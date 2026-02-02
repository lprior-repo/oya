//! Stage Gate component for pipeline visualization
//!
//! Displays the 9-stage pipeline with real-time status updates.
//! Uses Tauri IPC for communication with the backend.

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::models::pipeline::{PipelineState, StageInfo};
use crate::state::tauri_bridge;

/// Individual stage indicator component
#[component]
pub fn StageIndicator(stage: StageInfo, on_click: Option<Callback<String>>) -> impl IntoView {
    let stage_name = stage.name.clone();
    let stage_name_click = stage_name.clone();
    let status = stage.status;
    let gate = stage.gate.clone();
    let duration = stage.duration_ms;
    let error = stage.error.clone();

    let handle_click = move |_| {
        if let Some(callback) = on_click {
            callback.run(stage_name_click.clone());
        }
    };

    view! {
        <div
            class=format!("stage-indicator {}", status.css_class())
            style=format!(
                "border-color: {}; background-color: {}20;",
                status.color(),
                status.color()
            )
            on:click=handle_click
        >
            <div class="stage-icon" style=format!("color: {}", status.color())>
                {status.icon()}
            </div>
            <div class="stage-info">
                <div class="stage-name">{stage_name}</div>
                <div class="stage-gate">{gate}</div>
                {duration.map(|ms| view! {
                    <div class="stage-duration">{format!("{}ms", ms)}</div>
                })}
                {error.map(|e| view! {
                    <div class="stage-error" style="color: #ef4444;">{e}</div>
                })}
            </div>
            <div class="stage-status">{status.to_string()}</div>
        </div>
    }
}

/// Progress bar showing overall pipeline completion
#[component]
pub fn PipelineProgress(state: PipelineState) -> impl IntoView {
    let percent = state.completion_percent();
    let passed = state.passed_count();
    let failed = state.failed_count();
    let total = state.stages.len();

    // Determine overall color based on state
    let bar_color = if state.first_failure.is_some() {
        "#ef4444" // red for failure
    } else if state.all_passed {
        "#22c55e" // green for all passed
    } else {
        "#3b82f6" // blue for in progress
    };

    view! {
        <div class="pipeline-progress">
            <div class="progress-header">
                <span class="progress-label">"Pipeline Progress"</span>
                <span class="progress-stats">
                    {format!("{}/{} passed", passed, total)}
                    {if failed > 0 {
                        format!(", {} failed", failed)
                    } else {
                        String::new()
                    }}
                </span>
            </div>
            <div class="progress-bar-container">
                <div
                    class="progress-bar"
                    style=format!(
                        "width: {}%; background-color: {};",
                        percent,
                        bar_color
                    )
                />
            </div>
            <div class="progress-percent">{format!("{}%", percent)}</div>
        </div>
    }
}

/// Stage Gate panel showing all stages
#[component]
pub fn StageGate(#[prop(default = "default-task".to_string())] task_id: String) -> impl IntoView {
    // Reactive state for pipeline
    let (pipeline_state, set_pipeline_state) = signal(PipelineState::default());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(Option::<String>::None);

    // Clone task_id for closures
    let task_id_fetch = task_id.clone();
    let task_id_run = task_id.clone();
    let task_id_reset = task_id.clone();

    // Fetch pipeline state on mount
    Effect::new(move |_| {
        let task_id = task_id_fetch.clone();
        spawn_local(async move {
            set_loading.set(true);
            match tauri_bridge::get_pipeline_state(&task_id).await {
                Ok(state) => {
                    set_pipeline_state.set(state);
                    set_error.set(None);
                }
                Err(e) => {
                    set_error.set(Some(e.to_string()));
                }
            }
            set_loading.set(false);
        });
    });

    // Run pipeline handler
    let run_pipeline = move |_| {
        let task_id = task_id_run.clone();
        spawn_local(async move {
            set_loading.set(true);
            match tauri_bridge::run_pipeline(&task_id).await {
                Ok(state) => {
                    set_pipeline_state.set(state);
                    set_error.set(None);
                }
                Err(e) => {
                    set_error.set(Some(e.to_string()));
                }
            }
            set_loading.set(false);
        });
    };

    // Reset pipeline handler
    let reset_pipeline = move |_| {
        let task_id = task_id_reset.clone();
        spawn_local(async move {
            set_loading.set(true);
            match tauri_bridge::reset_pipeline(&task_id).await {
                Ok(state) => {
                    set_pipeline_state.set(state);
                    set_error.set(None);
                }
                Err(e) => {
                    set_error.set(Some(e.to_string()));
                }
            }
            set_loading.set(false);
        });
    };

    // Stage click handler
    let on_stage_click = Callback::new(move |stage_name: String| {
        let task_id = task_id.clone();
        spawn_local(async move {
            match tauri_bridge::run_stage(&task_id, &stage_name).await {
                Ok(event) => {
                    web_sys::console::log_1(
                        &format!("Stage {} completed: {:?}", event.stage, event.status).into(),
                    );
                    // Refresh pipeline state
                    if let Ok(state) = tauri_bridge::get_pipeline_state(&task_id).await {
                        set_pipeline_state.set(state);
                    }
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Stage failed: {}", e).into());
                    set_error.set(Some(e.to_string()));
                }
            }
        });
    });

    view! {
        <div class="stage-gate-panel">
            <div class="stage-gate-header">
                <h2>"Pipeline Stages"</h2>
                <div class="stage-gate-actions">
                    <button
                        class="btn btn-primary"
                        on:click=run_pipeline
                        disabled=move || loading.get()
                    >
                        {move || if loading.get() { "Running..." } else { "Run Pipeline" }}
                    </button>
                    <button
                        class="btn btn-secondary"
                        on:click=reset_pipeline
                        disabled=move || loading.get()
                    >
                        "Reset"
                    </button>
                </div>
            </div>

            {move || error.get().map(|e| view! {
                <div class="stage-gate-error">
                    <span class="error-icon">"!"</span>
                    <span class="error-message">{e}</span>
                </div>
            })}

            <PipelineProgress state=pipeline_state.get() />

            <div class="stage-list">
                {move || {
                    let state = pipeline_state.get();
                    state.stages.into_iter().map(|stage| {
                        let callback = Some(on_stage_click);
                        view! {
                            <StageIndicator
                                stage=stage
                                on_click=callback
                            />
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>

            <style>
                {r#"
                .stage-gate-panel {
                    padding: 1rem;
                    background: #1a1a2e;
                    border-radius: 8px;
                    color: #e0e0e0;
                }

                .stage-gate-header {
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    margin-bottom: 1rem;
                }

                .stage-gate-header h2 {
                    margin: 0;
                    font-size: 1.25rem;
                }

                .stage-gate-actions {
                    display: flex;
                    gap: 0.5rem;
                }

                .btn {
                    padding: 0.5rem 1rem;
                    border: none;
                    border-radius: 4px;
                    cursor: pointer;
                    font-size: 0.875rem;
                    transition: opacity 0.2s;
                }

                .btn:disabled {
                    opacity: 0.5;
                    cursor: not-allowed;
                }

                .btn-primary {
                    background: #3b82f6;
                    color: white;
                }

                .btn-secondary {
                    background: #374151;
                    color: white;
                }

                .stage-gate-error {
                    background: #7f1d1d;
                    border: 1px solid #ef4444;
                    border-radius: 4px;
                    padding: 0.75rem;
                    margin-bottom: 1rem;
                    display: flex;
                    align-items: center;
                    gap: 0.5rem;
                }

                .error-icon {
                    background: #ef4444;
                    color: white;
                    width: 20px;
                    height: 20px;
                    border-radius: 50%;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    font-weight: bold;
                }

                .pipeline-progress {
                    margin-bottom: 1rem;
                }

                .progress-header {
                    display: flex;
                    justify-content: space-between;
                    margin-bottom: 0.5rem;
                    font-size: 0.875rem;
                }

                .progress-bar-container {
                    background: #374151;
                    border-radius: 4px;
                    height: 8px;
                    overflow: hidden;
                }

                .progress-bar {
                    height: 100%;
                    transition: width 0.3s ease;
                }

                .progress-percent {
                    text-align: right;
                    font-size: 0.75rem;
                    margin-top: 0.25rem;
                    color: #9ca3af;
                }

                .stage-list {
                    display: flex;
                    flex-direction: column;
                    gap: 0.5rem;
                }

                .stage-indicator {
                    display: flex;
                    align-items: center;
                    padding: 0.75rem;
                    border: 1px solid;
                    border-radius: 4px;
                    cursor: pointer;
                    transition: transform 0.1s, box-shadow 0.1s;
                }

                .stage-indicator:hover {
                    transform: translateX(4px);
                    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
                }

                .stage-icon {
                    font-size: 1.5rem;
                    margin-right: 1rem;
                    min-width: 2rem;
                    text-align: center;
                }

                .stage-info {
                    flex: 1;
                }

                .stage-name {
                    font-weight: 600;
                    font-size: 0.875rem;
                }

                .stage-gate {
                    font-size: 0.75rem;
                    color: #9ca3af;
                }

                .stage-duration {
                    font-size: 0.75rem;
                    color: #6b7280;
                }

                .stage-error {
                    font-size: 0.75rem;
                    margin-top: 0.25rem;
                }

                .stage-status {
                    font-size: 0.75rem;
                    text-transform: uppercase;
                    font-weight: 500;
                }

                .stage-pending { border-color: #6b7280; }
                .stage-running { border-color: #3b82f6; animation: pulse 1.5s infinite; }
                .stage-passed { border-color: #22c55e; }
                .stage-failed { border-color: #ef4444; }
                .stage-skipped { border-color: #a855f7; }

                @keyframes pulse {
                    0%, 100% { opacity: 1; }
                    50% { opacity: 0.7; }
                }
                "#}
            </style>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_status_colors() {
        assert_eq!(StageStatus::Pending.color(), "#6b7280");
        assert_eq!(StageStatus::Running.color(), "#3b82f6");
        assert_eq!(StageStatus::Passed.color(), "#22c55e");
        assert_eq!(StageStatus::Failed.color(), "#ef4444");
        assert_eq!(StageStatus::Skipped.color(), "#a855f7");
    }

    #[test]
    fn test_stage_status_icons() {
        assert_eq!(StageStatus::Pending.icon(), "○");
        assert_eq!(StageStatus::Running.icon(), "◉");
        assert_eq!(StageStatus::Passed.icon(), "●");
        assert_eq!(StageStatus::Failed.icon(), "✕");
        assert_eq!(StageStatus::Skipped.icon(), "⊘");
    }

    #[test]
    fn test_components_compile() {
        // Verify that all components compile correctly
        let _ = StageGate;
        let _ = StageIndicator;
        let _ = PipelineProgress;
    }
}
