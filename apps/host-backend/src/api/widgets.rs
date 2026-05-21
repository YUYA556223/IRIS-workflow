use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
    Json, Router,
};

use crate::{
    domain::{widget::CreateWidget, UpdateWidgetBindings, Widget, WidgetId},
    error::{AppError, AppResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/widgets", get(list).post(create))
        .route("/widgets/:id", get(fetch).delete(remove))
        .route("/widgets/:id/bindings", patch(update_bindings))
}

async fn list(State(s): State<AppState>) -> AppResult<Json<Vec<Widget>>> {
    Ok(Json(s.widgets.list().await?))
}

async fn create(
    State(s): State<AppState>,
    Json(req): Json<CreateWidget>,
) -> AppResult<(StatusCode, Json<Widget>)> {
    // SDUI spec の存在チェック (referential integrity)
    if s.sdui.get(&req.sdui_spec_id).await?.is_none() {
        return Err(AppError::BadRequest(format!(
            "sdui spec '{}' not found",
            req.sdui_spec_id
        )));
    }

    let widget = req.into_widget();
    s.widgets.insert(widget.clone()).await?;
    let receivers = s.delivery.publish_widget_created(&widget);
    tracing::info!(
        widget_id = %widget.id,
        name = %widget.name,
        receivers,
        "widget created"
    );
    Ok((StatusCode::CREATED, Json(widget)))
}

async fn fetch(
    State(s): State<AppState>,
    Path(id): Path<WidgetId>,
) -> AppResult<Json<Widget>> {
    s.widgets
        .get(id)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

async fn update_bindings(
    State(s): State<AppState>,
    Path(id): Path<WidgetId>,
    Json(req): Json<UpdateWidgetBindings>,
) -> AppResult<Json<Widget>> {
    let widget = s.widgets.update_bindings(id, req.bindings).await?;
    let receivers = s.delivery.publish_widget_updated(&widget);
    tracing::info!(widget_id = %id, receivers, "widget bindings updated");
    Ok(Json(widget))
}

async fn remove(
    State(s): State<AppState>,
    Path(id): Path<WidgetId>,
) -> AppResult<StatusCode> {
    let widget = s.widgets.get(id).await?.ok_or(AppError::NotFound)?;
    let _ = s.widgets.delete(id).await?;
    s.delivery.publish_widget_deleted(id, widget.target);
    tracing::info!(widget_id = %id, "widget deleted");
    Ok(StatusCode::NO_CONTENT)
}
