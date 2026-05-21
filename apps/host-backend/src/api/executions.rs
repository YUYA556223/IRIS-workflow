use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
    workflow::ExecutionResult,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/executions", get(list))
        .route("/executions/:id", get(fetch))
        .route("/workflows/:id/executions", get(list_for_workflow))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    100
}

async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<ExecutionResult>>> {
    let limit = q.limit.min(1000);
    Ok(Json(
        s.executions.list(q.workflow_id.as_deref(), limit).await?,
    ))
}

async fn fetch(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ExecutionResult>> {
    s.executions
        .get(id)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

async fn list_for_workflow(
    State(s): State<AppState>,
    Path(workflow_id): Path<String>,
    Query(q): Query<LimitQuery>,
) -> AppResult<Json<Vec<ExecutionResult>>> {
    let limit = q.limit.min(1000);
    Ok(Json(s.executions.list(Some(&workflow_id), limit).await?))
}
