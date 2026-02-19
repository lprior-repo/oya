mod command_exec;
mod failure_summary;
mod gates;
mod http;
mod workspace;

pub(crate) use command_exec::run_opencode;
pub(crate) use failure_summary::summarize_failure_output;
pub(crate) use gates::{execute_gate, gate_failure_outcome, GateEvidence};
#[cfg(test)]
pub(crate) use gates::{
    parse_command_parts, parse_gate_command, GateCommand, MoonTask, ParsedCommandParts,
};
pub(crate) use http::{
    build_http_client, fetch_opencode_text, opencode_config, opencode_endpoint_url,
    poller_http_client_settings, workflow_http_client_settings, OpenCodeConfig, OpenCodeEndpoint,
};
pub(crate) use workspace::{prepare_stage_workspace, WorkspacePrepRequest};
