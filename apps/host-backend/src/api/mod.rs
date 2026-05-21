//! HTTP / WebSocket エンドポイント。
//!
//! 各サブモジュールが `Router<AppState>` を返し、本モジュールがマージして
//! 最終的な `Router<()>` を組み立てる。

use axum::Router;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::state::AppState;

pub mod ai;
pub mod devices;
pub mod executions;
pub mod health;
pub mod notifications;
pub mod permission;
pub mod sdui;
pub mod webhooks;
pub mod widgets;
pub mod workflows;
pub mod ws;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .merge(devices::router())
        .merge(widgets::router())
        .merge(notifications::router())
        .merge(sdui::router())
        .merge(ws::router())
        .merge(ai::router())
        .merge(workflows::router())
        .merge(executions::router())
        .merge(webhooks::router())
        .merge(permission::router())
        .layer(TraceLayer::new_for_http())
        // 開発時の web-console (localhost:3000) からのアクセスを許可。
        // 本番 (host PC + tailnet) では Origin 制限を入れる予定。
        .layer(CorsLayer::permissive())
        .with_state(state)
}
