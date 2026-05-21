use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
    workflow::{ExecutionResult, Workflow},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/workflows", get(list).post(upsert))
        .route("/workflows/:id", get(fetch).delete(remove))
        .route("/workflows/:id/run", post(run))
}

async fn list(State(s): State<AppState>) -> AppResult<Json<Vec<Workflow>>> {
    Ok(Json(s.workflows.list()))
}

async fn upsert(
    State(s): State<AppState>,
    Json(wf): Json<Workflow>,
) -> AppResult<(StatusCode, Json<Workflow>)> {
    if wf.id.trim().is_empty() {
        return Err(AppError::BadRequest("workflow id must not be empty".into()));
    }
    if wf.nodes.is_empty() {
        return Err(AppError::BadRequest("workflow must have at least one node".into()));
    }
    s.workflows.upsert(wf.clone());
    s.triggers.sync().await;
    tracing::info!(workflow_id = %wf.id, nodes = wf.nodes.len(), "workflow upserted");
    Ok((StatusCode::CREATED, Json(wf)))
}

async fn fetch(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Workflow>> {
    s.workflows.get(&id).map(Json).ok_or(AppError::NotFound)
}

async fn remove(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    if s.workflows.delete(&id) {
        s.triggers.sync().await;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

async fn run(
    State(s): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<Value>>,
) -> AppResult<Json<ExecutionResult>> {
    let workflow = s.workflows.get(&id).ok_or(AppError::NotFound)?;
    let trigger_data = body.map(|Json(v)| v).unwrap_or(Value::Null);
    tracing::info!(workflow_id = %id, "workflow run requested");
    let result = s.executor.clone().execute(workflow, trigger_data).await;
    Ok(Json(result))
}
