use serde::{Deserialize, Serialize};

/// ワークフロー定義 (YAML / JSON 共通)。
///
/// 例:
/// ```yaml
/// id: morning-briefing
/// name: 朝のブリーフィング
/// trigger:
///   type: manual
/// nodes:
///   - id: greet
///     type: ai
///     using: claude-code
///     with:
///       prompt: "Say hello"
/// edges: []
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub trigger: Trigger,
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub edges: Vec<Edge>,
}

impl Workflow {
    pub fn find_node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }
}

/// ワークフロー起動契機。MVP では `manual` のみ実装、cron/webhook は P3.1 で。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Trigger {
    /// REST `POST /workflows/:id/run` から起動。
    Manual,
    /// Cron スケジュール (e.g. "0 9 * * 1-5")。実行は P3.1。
    Cron { schedule: String },
    /// Webhook (path で match)。実行は P3.1。
    Webhook { path: String },
    /// ファイル監視。実行は P3.1。
    FsWatch { path: String },
    /// MQTT トピックサブスクライブ。実行は P8。
    Mqtt { topic: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: NodeType,
    /// プロバイダ識別子。
    /// - `ai`: `"claude-code"` 等
    /// - `action`: `"builtin/notify"` / `"builtin/widget-update"` / `"builtin/sdui-upsert"`
    /// - `transform`: `"builtin/pass-through"` / `"builtin/now"` 等
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub using: Option<String>,
    /// パラメータ。値中の `{{ node_id.path }}` は実行時にバインディング展開される。
    #[serde(default)]
    pub with: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum NodeType {
    /// LLM 呼び出し (デフォルトプロバイダ: Claude Code)。
    Ai,
    /// データ整形・組み込み API 呼び出し。
    Transform,
    /// デバイスへの出力 (通知 / Widget / IoT)。
    Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
}
