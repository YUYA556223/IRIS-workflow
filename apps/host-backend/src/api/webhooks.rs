use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use serde_json::Value;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
    workflow::ExecutionResult,
};

pub fn router() -> Router<AppState> {
    // axum 0.7: `*path` で path の残り全部を捕捉
    Router::new().route("/hooks/*path", post(handler))
}

/// `POST /hooks/<path>` — workflow.trigger.webhook.path == path のワークフローを起動。
/// リクエスト body を `{{ trigger.* }}` で参照できる trigger_data として渡す。
async fn handler(
    State(s): State<AppState>,
    Path(path): Path<String>,
    body: Option<Json<Value>>,
) -> AppResult<Json<ExecutionResult>> {
    let workflow_id = s
        .triggers
        .lookup_webhook(&path)
        .await
        .ok_or(AppError::NotFound)?;
    let workflow = s.workflows.get(&workflow_id).ok_or(AppError::NotFound)?;
    let trigger_data = body.map(|Json(v)| v).unwrap_or(Value::Null);
    tracing::info!(path = %path, workflow_id = %workflow_id, "webhook hit");
    let result = s.executor.execute(&workflow, trigger_data).await;
    Ok(Json(result))
}
