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
    apply_stage_plan, approve_task, plan_task_stages, pipeline_report, resolve_stage_range,
    run_full_pipeline, run_task_pipeline, stage_range, PipelineStageStatus, StageReport,
};
pub use stages::{pipeline_stage_edges, Stage, validate_stage_sequence};
