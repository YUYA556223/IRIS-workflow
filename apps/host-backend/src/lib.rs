//! IRIS-workflow host backend ライブラリクレート。
//!
//! バイナリ (`main.rs`) はこのライブラリを薄くラップする。テストや
//! 将来の埋め込み用途 (例: ホストアプリ自身にバンドル) のためライブラリ化している。

pub mod ai;
pub mod api;
pub mod config;
pub mod delivery;
pub mod domain;
pub mod error;
pub mod state;
pub mod storage;
pub mod telemetry;
pub mod triggers;
pub mod workflow;

pub use config::Config;
pub use error::{AppError, AppResult};
pub use state::AppState;

/// `AppState` から完全な Axum Router を構築する。
pub fn build_app(state: AppState) -> axum::Router {
    api::router(state)
}
