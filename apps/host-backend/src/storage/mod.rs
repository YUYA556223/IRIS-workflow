//! ストレージ層: ドメイン型の永続化を抽象化する。
//!
//! P1 ではメモリ実装 (`memory`) を採用し、P1.5 で PostgreSQL 実装に差し替える。
//! 各リポジトリは `Arc<dyn Repo>` で `AppState` に保持される。

pub mod devices;
pub mod executions;
pub mod memory;
pub mod postgres;
pub mod sdui;
pub mod widgets;

pub use devices::DeviceRepo;
pub use executions::ExecutionRepo;
pub use sdui::SduiRepo;
pub use widgets::WidgetRepo;

/// ストレージ操作で発生し得るエラー。
///
/// メモリ実装では基本的に発生しないが、Postgres 実装で `sqlx::Error`
/// などをラップする受け皿として用意する。
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type StorageResult<T> = Result<T, StorageError>;
