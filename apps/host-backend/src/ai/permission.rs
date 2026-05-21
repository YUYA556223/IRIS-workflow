//! Claude Code の `--permission-prompt-tool` を受ける Rust 側ハブ。
//!
//! プロトコル: Claude が許可を要する tool を実行する直前、設定済みの MCP tool が
//! 呼ばれる。tool は以下の JSON を受け取り、応答する:
//!
//! 入力: `{ "tool_name": "Bash", "tool_input": { ... } }`
//! 応答: `{ "behavior": "allow"|"deny", "updatedInput": {...}?, "message": "..."? }`
//!
//! 本モジュールはこの応答を生成する登録簿 (registry) を提供する:
//!  1. MCP server bridge (別 process) が host-backend の `/permission/request` を叩く
//!  2. host-backend は `PermissionRegistry::request()` を呼び、`DeliveryHub` 経由で
//!     全デバイスへ `PermissionRequested` イベントを push
//!  3. デバイスがユーザに承認を求め、`POST /permission/respond` で結果を返す
//!  4. 待っていた MCP server bridge にレスポンスを返す
//!
//! 完全な MCP server バイナリ (`apps/iris-mcp-permission/`) はフォローアップ。
//! 現状の API は HTTP のみで完結し、CLI bridge は外付け可能な形になっている。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

/// MCP permission-prompt-tool 入力。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPromptInput {
    pub tool_name: String,
    #[serde(default)]
    pub tool_input: serde_json::Value,
    /// (オプション) Claude セッション ID。どのワークフロー実行か追跡したい時に。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// MCP permission-prompt-tool 応答。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPromptOutput {
    /// `"allow"` または `"deny"`。
    pub behavior: PermissionBehavior,
    /// `behavior=allow` で入力を書き換える場合 (例: rm を ls に変換)。
    #[serde(default, rename = "updatedInput", skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,
    /// ユーザへ表示する任意メッセージ。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    Allow,
    Deny,
}

/// in-flight な permission request を追跡するレジストリ。
pub struct PermissionRegistry {
    pending: Mutex<HashMap<Uuid, oneshot::Sender<PermissionPromptOutput>>>,
    timeout: Duration,
}

impl PermissionRegistry {
    pub fn new(timeout: Duration) -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            timeout,
        }
    }

    /// 新規 permission request を発行する。`PendingHandle` を返し、呼び出し側は
    /// その `wait()` でユーザ応答を受け取る。タイムアウトは構築時の設定値。
    pub async fn open(&self) -> PendingHandle {
        let id = Uuid::new_v4();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        PendingHandle {
            id,
            rx,
            timeout: self.timeout,
        }
    }

    /// デバイスからの応答を受け取り、待機中の sender に渡す。
    /// 該当 ID が無ければ `false`。
    pub async fn respond(&self, id: Uuid, resp: PermissionPromptOutput) -> bool {
        let sender = self.pending.lock().await.remove(&id);
        if let Some(s) = sender {
            let _ = s.send(resp);
            true
        } else {
            false
        }
    }
}

/// 開いた permission request のハンドル。`wait()` でユーザの応答を待つ。
pub struct PendingHandle {
    pub id: Uuid,
    rx: oneshot::Receiver<PermissionPromptOutput>,
    timeout: Duration,
}

impl PendingHandle {
    pub async fn wait(self) -> anyhow::Result<PermissionPromptOutput> {
        match tokio::time::timeout(self.timeout, self.rx).await {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(_)) => anyhow::bail!("permission sender dropped"),
            Err(_) => anyhow::bail!("permission request timed out after {:?}", self.timeout),
        }
    }
}

/// API レスポンスとして配信するメタデータ (`/permission/request` の戻り)。
#[derive(Debug, Clone, Serialize)]
pub struct PermissionRequestMeta {
    pub request_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub input: PermissionPromptInput,
}

impl PermissionRequestMeta {
    pub fn new(id: Uuid, input: PermissionPromptInput) -> Self {
        Self {
            request_id: id,
            created_at: Utc::now(),
            input,
        }
    }
}

/// Shared registry handle (AppState 経由で配布)。
pub type PermissionRegistryHandle = Arc<PermissionRegistry>;
