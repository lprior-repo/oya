//! Health check endpoints: GET /api/health, GET /api/system/health

use super::super::actors::AppState;
use super::super::error::Result;
use axum::{extract::State, response::Json};
use serde::Serialize;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    status: String,
    version: String,
}

fn build_health_response() -> HealthResponse {
    HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// GET /api/health - Health check endpoint
pub async fn health_check(State(_state): State<AppState>) -> Result<Json<HealthResponse>> {
    Ok(Json(build_health_response()))
}

/// GET /api/system/health - System health check endpoint
pub async fn system_health_check(State(_state): State<AppState>) -> Result<Json<HealthResponse>> {
    Ok(Json(build_health_response()))
}
