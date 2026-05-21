//! ワークフロー (DAG) エンジン。
//!
//! 「入力 (トリガ) → AI 中心の処理 → デバイス出力」というコンセプトの実体。
//! YAML で記述したワークフローをロードし、DAG として実行する。
//!
//! - `dsl`      : YAML/JSON にデシリアライズ可能なワークフロー型
//! - `dag`      : トポロジカルソート (Kahn のアルゴリズム) + サイクル検出
//! - `template` : `{{ node_id.field }}` プレースホルダ展開
//! - `loader`   : ディレクトリから YAML を一括ロード
//! - `store`    : ロード済みワークフローの in-memory レジストリ
//! - `executor` : ノードを順次実行し、`ClaudeService` / `DeliveryHub` 等にディスパッチ
//!
//! 仕様の概念は `docs/concept/05-workflow-dsl.md` を参照。

pub mod dag;
pub mod dsl;
pub mod executor;
pub mod loader;
pub mod store;
pub mod template;

pub use dsl::{Edge, Node, NodeType, Trigger, Workflow};
pub use executor::{
    ExecutionResult, ExecutionStatus, NodeExecution, NodeStatus, WorkflowExecutor,
};
pub use loader::load_dir;
pub use store::WorkflowStore;
