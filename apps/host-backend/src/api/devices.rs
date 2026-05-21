use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

use crate::{
    domain::{Device, DeviceId, RegisterDevice},
    error::{AppError, AppResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/devices", get(list).post(register))
        .route("/devices/:id", get(fetch).delete(remove))
}

async fn list(State(s): State<AppState>) -> AppResult<Json<Vec<Device>>> {
    Ok(Json(s.devices.list().await?))
}

async fn register(
    State(s): State<AppState>,
    Json(req): Json<RegisterDevice>,
) -> AppResult<(StatusCode, Json<Device>)> {
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("name must not be empty".into()));
    }
    let device = req.into_device();
    s.devices.insert(device.clone()).await?;
    tracing::info!(device_id = %device.id, name = %device.name, "device registered");
    Ok((StatusCode::CREATED, Json(device)))
}

async fn fetch(
    State(s): State<AppState>,
    Path(id): Path<DeviceId>,
) -> AppResult<Json<Device>> {
    s.devices
        .get(id)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

async fn remove(
    State(s): State<AppState>,
    Path(id): Path<DeviceId>,
) -> AppResult<StatusCode> {
    if s.devices.delete(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
