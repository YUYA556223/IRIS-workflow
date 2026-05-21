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
    /// - `action`: `"builtin/notify"` / `"builtin/widget-update"` / `"builtin/sdui-upsert"` / `"builtin/mqtt-publish"`
    /// - `transform`: `"builtin/pass-through"` / `"builtin/now"` 等
    /// - `workflow`: 不要 (`with.workflow_id` で対象を指定)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub using: Option<String>,
    /// パラメータ。値中の `{{ node_id.path }}` は実行時にバインディング展開される。
    #[serde(default)]
    pub with: serde_json::Value,
    /// 条件付き実行 (P3.4)。テンプレート展開後の文字列が truthy のときのみ実行。
    /// falsy だった場合はノードは `Skipped` で完了し、**下流はそのまま継続実行される**
    /// (失敗ではないので taint しない)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// 失敗時の自動リトライ。指定なしなら 1 回試行のみ。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryPolicy>,
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
    /// サブワークフロー呼び出し。`with.workflow_id` で対象を指定。
    Workflow,
}

/// 失敗時の自動リトライポリシー (P3.4)。
///
/// `max_attempts` 回まで再試行し、毎回 `delay_ms` (constant) または
/// `delay_ms * 2^(attempt-1)` (exponential) のディレイを挟む。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_delay_ms")]
    pub delay_ms: u64,
    #[serde(default)]
    pub backoff: BackoffStrategy,
}

fn default_max_attempts() -> u32 {
    3
}
fn default_delay_ms() -> u64 {
    500
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BackoffStrategy {
    #[default]
    Constant,
    Exponential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
}
