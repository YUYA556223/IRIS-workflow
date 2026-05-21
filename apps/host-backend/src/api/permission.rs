//! Permission prompt API (P2.1).
//!
//! 役割:
//!  - `/permission/request` : MCP bridge から呼ばれる入口。デバイスに承認依頼を
//!    push し、`PermissionRegistry` で応答を待って返す。
//!  - `/permission/respond` : デバイスから承認結果を受け取り、待機中の bridge へ
//!    `oneshot` で渡す。

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ai::{PermissionPromptInput, PermissionPromptOutput},
    error::{AppError, AppResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/permission/request", post(request))
        .route("/permission/respond", post(respond))
}

/// MCP bridge → host-backend。デバイスに承認依頼を出し、応答を待って返す。
async fn request(
    State(s): State<AppState>,
    Json(input): Json<PermissionPromptInput>,
) -> AppResult<Json<PermissionPromptOutput>> {
    let pending = s.permission.open().await;
    tracing::info!(
        request_id = %pending.id,
        tool_name = %input.tool_name,
        "permission requested"
    );
    let receivers = s.delivery.publish_permission_request(pending.id, &input);
    tracing::debug!(
        request_id = %pending.id,
        receivers,
        "permission request broadcast"
    );

    let resp = pending
        .wait()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    tracing::info!(
        tool_name = %input.tool_name,
        behavior = ?resp.behavior,
        "permission resolved"
    );
    Ok(Json(resp))
}

#[derive(Debug, Deserialize)]
pub struct RespondRequest {
    pub request_id: Uuid,
    #[serde(flatten)]
    pub response: PermissionPromptOutput,
}

#[derive(Debug, Serialize)]
pub struct RespondResponse {
    pub accepted: bool,
}

/// デバイス → host-backend。承認結果を待機中の bridge に伝える。
async fn respond(
    State(s): State<AppState>,
    Json(req): Json<RespondRequest>,
) -> AppResult<(StatusCode, Json<RespondResponse>)> {
    let accepted = s.permission.respond(req.request_id, req.response).await;
    let status = if accepted {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    };
    Ok((status, Json(RespondResponse { accepted })))
}
