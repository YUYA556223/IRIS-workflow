use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

use crate::{
    domain::{SduiSpec, SduiSpecId},
    error::{AppError, AppResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sdui-specs", get(list).post(upsert))
        .route("/sdui-specs/:id", get(fetch).delete(remove))
}

async fn list(State(s): State<AppState>) -> AppResult<Json<Vec<SduiSpec>>> {
    Ok(Json(s.sdui.list().await?))
}

async fn upsert(
    State(s): State<AppState>,
    Json(spec): Json<SduiSpec>,
) -> AppResult<(StatusCode, Json<SduiSpec>)> {
    if spec.id.0.trim().is_empty() {
        return Err(AppError::BadRequest("spec id must not be empty".into()));
    }
    s.sdui.upsert(spec.clone()).await?;
    s.delivery.publish_sdui_updated(&spec);
    tracing::info!(spec_id = %spec.id, "sdui spec upserted");
    Ok((StatusCode::CREATED, Json(spec)))
}

async fn fetch(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<SduiSpec>> {
    let sid = SduiSpecId::new(id);
    s.sdui
        .get(&sid)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

async fn remove(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    let sid = SduiSpecId::new(id);
    if s.sdui.delete(&sid).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
