use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::Serialize;

use crate::{
    domain::{DispatchNotification, Notification},
    error::AppResult,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/notifications", post(dispatch))
}

#[derive(Serialize)]
pub struct DispatchResponse {
    pub notification: Notification,
    /// 配信時点で WS にぶら下がっていたデバイス数。0 でもエラーではない
    /// (オフラインのデバイスがあとで再接続したら別チャネルで取りに来る)。
    pub receivers: usize,
}

async fn dispatch(
    State(s): State<AppState>,
    Json(req): Json<DispatchNotification>,
) -> AppResult<(StatusCode, Json<DispatchResponse>)> {
    let notification = req.into_notification();
    let receivers = s.delivery.dispatch_notification(notification.clone());
    tracing::info!(
        notification_id = %notification.id,
        receivers,
        priority = ?notification.priority,
        "notification dispatched"
    );
    Ok((
        StatusCode::ACCEPTED,
        Json(DispatchResponse {
            notification,
            receivers,
        }),
    ))
}
