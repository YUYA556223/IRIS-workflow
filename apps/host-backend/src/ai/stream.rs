use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// `claude --output-format stream-json` の各行に対応する汎用イベント型。
///
/// 既知フィールドを名前付きで取り出し、未知フィールドは `rest` に保持する。
/// この設計により、Claude Code 側で新しいイベントタイプ・フィールドが
/// 追加されても破綻せずに通り抜けられる。
///
/// 代表的な `type` 値:
/// - `"system"` (`subtype: "init"`): セッション開始通知
/// - `"assistant"`: アシスタントメッセージ (`rest["message"]` に内容)
/// - `"user"`: ツール結果メッセージ (`rest["message"]` に内容)
/// - `"result"` (`subtype: "success" | "error_*"`): 最終結果。`result`,
///   `total_cost_usd`, `num_turns`, `duration_ms` を含む。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StreamEvent {
    #[serde(rename = "type")]
    pub kind: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// `result` イベントの最終テキスト出力。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_turns: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,

    /// その他のフィールド (`message`, `model`, `tools`, ...) を verbatim 保持。
    #[serde(flatten)]
    pub rest: HashMap<String, serde_json::Value>,
}

impl StreamEvent {
    pub fn is_result(&self) -> bool {
        self.kind == "result"
    }

    pub fn errored(&self) -> bool {
        self.is_error.unwrap_or(false) || self.kind == "error"
    }
}
