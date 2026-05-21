//! Claude Code ブリッジ。
//!
//! `claude` CLI を `tokio::process::Command` で spawn し、`--output-format stream-json`
//! で得られる NDJSON イベントを `StreamEvent` にパースしてストリーミング処理する。
//!
//! - `stream`  : イベント型定義 (system_init / assistant / user / result / ...)
//! - `process` : ClaudeProcessHandle (spawn / next_event / kill / wait)
//! - `service` : ClaudeService (Semaphore による同時実行制御 + run)
//!
//! 詳細設計は `docs/concept/04-claude-code-bridge.md` を参照。

pub mod permission;
pub mod process;
pub mod service;
pub mod stream;

pub use permission::{
    PendingHandle, PermissionBehavior, PermissionPromptInput, PermissionPromptOutput,
    PermissionRegistry, PermissionRegistryHandle, PermissionRequestMeta,
};
pub use process::{ClaudeProcessHandle, SpawnOptions};
pub use service::{ClaudeRunResult, ClaudeService};
pub use stream::StreamEvent;
