#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! Pipeline domain and persistence for OYA tasks.
//!
//! This crate defines task lifecycle types and persistence helpers for the
//! CLI and orchestrator layers.

mod domain;
mod error;
mod persistence;
mod plan;
mod stages;

pub use domain::{Language, Priority, Slug, Task, TaskStatus};
pub use error::{Error, Result};
pub use persistence::{list_all_tasks, load_task_record, save_task_record, update_task_status};
pub use plan::{
    PipelineStageStatus, StageReport, apply_stage_plan, approve_task, pipeline_report,
    plan_task_stages, resolve_stage_range, run_full_pipeline, run_task_pipeline, stage_range,
};
pub use stages::{Stage, pipeline_stage_edges, validate_stage_sequence};
