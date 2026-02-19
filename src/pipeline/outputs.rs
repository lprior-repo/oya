use std::path::PathBuf;

use oya::types::{truncate_clean, Gate, StageName as Stage, StageResult};
use restate_sdk::prelude::*;

use crate::orchestrator_types::{
    set_json_state, stage_attempt_key, write_orchestrator_state, GateEventSummary,
    SkillOutputEvent, StageEnvelopeEvent, StageResultEvent,
};
use crate::runtime_tools::{execute_gate, GateEvidence};
use crate::stage_executor::format_gate_command_output;

use super::state::{
    deterministic_timestamp_or_error, parse_rfc3339_deterministic, PipelineRunInput, PipelineState,
    StageArtifacts, StageAttemptRecord,
};
use super::OyaError;

pub(crate) struct RecordStageOutputsInput<'a> {
    pub(crate) input: &'a PipelineRunInput,
    pub(crate) attempt_record: &'a StageAttemptRecord,
    pub(crate) stage_result: &'a StageResult,
    pub(crate) stage_prompt: &'a str,
    pub(crate) stage_started_at: chrono::DateTime<chrono::Utc>,
    pub(crate) repo_root: &'a PathBuf,
}

pub(crate) struct StageEnvelopeInput<'a> {
    pub(crate) input: &'a PipelineRunInput,
    pub(crate) attempt_record: &'a StageAttemptRecord,
    pub(crate) prompt_key: &'a str,
    pub(crate) stage_result_key: &'a str,
    pub(crate) skill_output_key: &'a str,
    pub(crate) gate_events: &'a [GateEventSummary],
    pub(crate) event_ts: &'a str,
    pub(crate) stage_result: &'a StageResult,
}

pub(crate) async fn record_stage_outputs(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    request: RecordStageOutputsInput<'_>,
) -> Result<StageArtifacts, OyaError> {
    update_orchestrator_after_stage(ctx, state, request.stage_result, request.stage_prompt).await?;
    let prompt_key = format!("prompt_{}_{}", state.current_stage.as_str(), state.attempt);
    ctx.set(&prompt_key, request.stage_prompt.to_string());
    let stage_result_key = stage_attempt_key(&state.current_stage, state.attempt, "result");
    set_stage_result_json(ctx, &stage_result_key, request.stage_result)?;
    let skill_output_key = stage_attempt_key(&state.current_stage, state.attempt, "skill_output");
    set_skill_output_json(ctx, &state.current_stage, &skill_output_key, request.stage_result)?;
    let gate_events = record_gate_events(ctx, state, request.repo_root)?;
    let event_ts = deterministic_timestamp_or_error(ctx).await?;
    set_stage_envelope(
        ctx,
        state,
        StageEnvelopeInput {
            input: request.input,
            attempt_record: request.attempt_record,
            prompt_key: &prompt_key,
            stage_result_key: &stage_result_key,
            skill_output_key: &skill_output_key,
            gate_events: &gate_events,
            event_ts: &event_ts,
            stage_result: request.stage_result,
        },
    )?;
    let duration = (chrono::Utc::now() - request.stage_started_at).num_milliseconds().max(0) as u64;
    Ok(StageArtifacts {
        stage_duration_ms: duration,
        event_at: parse_rfc3339_deterministic(&event_ts),
    })
}

async fn update_orchestrator_after_stage(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    stage_result: &StageResult,
    stage_prompt: &str,
) -> Result<(), OyaError> {
    state.orchestrator.last_prompt = stage_prompt.to_string();
    state.orchestrator.last_output = truncate_clean(&stage_result.output.to_string(), 6000);
    state.orchestrator.last_failure = if stage_result.passed {
        String::new()
    } else {
        truncate_clean(&stage_result.output.to_string(), 6000)
    };
    state.orchestrator.updated_at = deterministic_timestamp_or_error(ctx).await?;
    write_orchestrator_state(ctx, &state.orchestrator)
}

fn set_stage_result_json(
    ctx: &WorkflowContext<'_>,
    key: &str,
    stage_result: &StageResult,
) -> Result<(), OyaError> {
    set_json_state(
        ctx,
        key,
        &StageResultEvent {
            passed: stage_result.passed,
            failure_category: stage_result
                .failure_category
                .as_ref()
                .map(|value| format!("{:?}", value)),
            next_stage: stage_result.next_stage.as_ref().map(|value| value.as_str().to_string()),
            output: truncate_clean(&stage_result.output.to_string(), 6000),
        },
    )
}

fn set_skill_output_json(
    ctx: &WorkflowContext<'_>,
    stage: &Stage,
    key: &str,
    stage_result: &StageResult,
) -> Result<(), OyaError> {
    let stage_log = truncate_clean(&stage_result.output.to_string(), 12000);
    set_json_state(
        ctx,
        key,
        &SkillOutputEvent {
            success: stage_result.passed,
            exit_code: if stage_result.passed { 0 } else { 1 },
            full_log: stage_log.clone(),
            feedback: stage_result
                .failure_category
                .as_ref()
                .map_or(String::new(), |value| format!("{:?}", value)),
            contract_document: (stage == &Stage::Contract).then_some(stage_log.clone()),
            implementation_code: (stage == &Stage::Tdd15).then_some(stage_log.clone()),
            test_results: (stage == &Stage::Qa || stage == &Stage::RedQueen)
                .then_some(stage_log.clone()),
            adversarial_report: (stage == &Stage::RedQueen).then_some(stage_log),
        },
    )
}

fn record_gate_events(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    repo_root: &PathBuf,
) -> Result<Vec<GateEventSummary>, OyaError> {
    state
        .current_stage
        .gates()
        .into_iter()
        .map(|gate| record_single_gate_event(ctx, state, repo_root, gate))
        .collect()
}

fn record_single_gate_event(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    repo_root: &PathBuf,
    gate: Gate,
) -> Result<GateEventSummary, OyaError> {
    let gate_evidence: GateEvidence = execute_gate(gate.clone(), repo_root)?;
    let gate_key =
        stage_attempt_key(&state.current_stage, state.attempt, &format!("gate_{}", gate.as_str()));
    set_json_state(
        ctx,
        &gate_key,
        &StageResultEvent {
            passed: gate_evidence.passed,
            failure_category: None,
            next_stage: None,
            output: format_gate_command_output(
                gate_evidence.command.as_str(),
                gate_evidence.exit_code,
                truncate_clean(&gate_evidence.output, 4000).as_str(),
            ),
        },
    )?;
    Ok(GateEventSummary {
        gate: gate.as_str().to_string(),
        state_key: gate_key,
        artifact_id: String::new(),
        passed: gate_evidence.passed,
        exit_code: gate_evidence.exit_code,
    })
}

fn set_stage_envelope(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    input: StageEnvelopeInput<'_>,
) -> Result<(), OyaError> {
    let stage_event_key = stage_attempt_key(&state.current_stage, state.attempt, "event");
    set_json_state(
        ctx,
        &stage_event_key,
        &StageEnvelopeEvent {
            run_id: input.input.run_id.clone(),
            bead_id: input.input.bead_id.clone(),
            stage: state.current_stage.as_str().to_string(),
            attempt: state.attempt,
            status: if input.stage_result.passed {
                "passed".to_string()
            } else {
                "failed".to_string()
            },
            input_key: input.attempt_record.stage_input_key.clone(),
            prompt_key: input.prompt_key.to_string(),
            result_key: input.stage_result_key.to_string(),
            skill_output_key: input.skill_output_key.to_string(),
            gate_events: input.gate_events.to_vec(),
            recorded_at: input.event_ts.to_string(),
        },
    )
}
