use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::{
    ai::{ClaudeService, SpawnOptions},
    delivery::DeliveryHub,
    domain::{notification::DispatchNotification, target::DeliveryTarget, SduiSpec, WidgetId},
    storage::{executions::ExecutionRepo, SduiRepo, WidgetRepo},
};

use super::{
    dag::topo_sort,
    dsl::{Node, NodeType, Workflow},
    template::{render_value, TemplateContext},
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[sqlx(type_name = "TEXT", rename_all = "kebab-case")]
pub enum ExecutionStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum NodeStatus {
    Success,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecution {
    pub node_id: String,
    pub kind: NodeType,
    pub status: NodeStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub output: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub execution_id: Uuid,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    /// 起動データ (REST body / cron fired_at / webhook payload / fs event)。
    #[serde(default)]
    pub trigger_data: Value,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub nodes: Vec<NodeExecution>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// ワークフローを 1 回実行するエンジン。
///
/// ノードは「波 (wave)」単位で並列実行する:
///  - 初期波: in-degree = 0 のノード全て
///  - 各ノード完了後、後続の in-degree を -1。0 になったら次波に追加
///  - ノードが失敗した場合は、依存する全ての下流ノードを `Skipped` とマーク
pub struct WorkflowExecutor {
    claude: Arc<ClaudeService>,
    delivery: Arc<DeliveryHub>,
    widgets: Arc<dyn WidgetRepo>,
    sdui: Arc<dyn SduiRepo>,
    executions: Arc<dyn ExecutionRepo>,
}

impl WorkflowExecutor {
    pub fn new(
        claude: Arc<ClaudeService>,
        delivery: Arc<DeliveryHub>,
        widgets: Arc<dyn WidgetRepo>,
        sdui: Arc<dyn SduiRepo>,
        executions: Arc<dyn ExecutionRepo>,
    ) -> Self {
        Self {
            claude,
            delivery,
            widgets,
            sdui,
            executions,
        }
    }

    /// `trigger_data` は起動時の文脈データ (`{{ trigger.* }}` で参照可)。
    /// 実行終了後に `ExecutionRepo` へ自動保存される (保存失敗はログのみ)。
    pub async fn execute(&self, workflow: &Workflow, trigger_data: Value) -> ExecutionResult {
        let execution_id = Uuid::new_v4();
        let started_at = Utc::now();

        // 1. 検証 (topo_sort はサイクル検出も兼ねる)
        let topo_order = match topo_sort(workflow) {
            Ok(o) => o,
            Err(e) => {
                let result = ExecutionResult {
                    execution_id,
                    workflow_id: workflow.id.clone(),
                    status: ExecutionStatus::Failed,
                    trigger_data: trigger_data.clone(),
                    started_at,
                    finished_at: Utc::now(),
                    nodes: Vec::new(),
                    error: Some(format!("topology: {}", e)),
                };
                self.persist(&result).await;
                return result;
            }
        };

        // 2. in-degree と隣接リスト
        let mut in_degree: HashMap<String, usize> = workflow
            .nodes
            .iter()
            .map(|n| (n.id.clone(), 0_usize))
            .collect();
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        for edge in &workflow.edges {
            *in_degree.entry(edge.to.clone()).or_insert(0) += 1;
            adjacency
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
        }

        let mut outputs: HashMap<String, Value> = HashMap::new();
        let mut tainted: HashSet<String> = HashSet::new();
        let mut completed: HashMap<String, NodeExecution> = HashMap::new();
        let mut overall_failed = false;

        // 初期波: in_degree=0 を YAML 順に
        let mut current_wave: Vec<String> = workflow
            .nodes
            .iter()
            .filter(|n| in_degree.get(&n.id).copied().unwrap_or(0) == 0)
            .map(|n| n.id.clone())
            .collect();

        while !current_wave.is_empty() {
            // 3. 波ごとに JoinSet で並列実行
            let mut joinset: JoinSet<NodeOutcome> = JoinSet::new();
            // この波で同期的に決着するもの (template fail / tainted) も完了集合に積む
            for node_id in &current_wave {
                let node = match workflow.find_node(node_id) {
                    Some(n) => n.clone(),
                    None => continue,
                };

                if tainted.contains(node_id) {
                    let now = Utc::now();
                    completed.insert(
                        node.id.clone(),
                        NodeExecution {
                            node_id: node.id.clone(),
                            kind: node.kind,
                            status: NodeStatus::Skipped,
                            started_at: now,
                            finished_at: now,
                            output: Value::Null,
                            error: None,
                        },
                    );
                    continue;
                }

                // テンプレート展開 (現在の outputs にのみ依存。同波の peer 出力は参照不可)
                let ctx = TemplateContext {
                    trigger: &trigger_data,
                    outputs: &outputs,
                };
                let rendered = match render_value(&node.with, &ctx) {
                    Ok(v) => v,
                    Err(e) => {
                        let now = Utc::now();
                        overall_failed = true;
                        taint_downstream(&node.id, &adjacency, &mut tainted);
                        completed.insert(
                            node.id.clone(),
                            NodeExecution {
                                node_id: node.id.clone(),
                                kind: node.kind,
                                status: NodeStatus::Failed,
                                started_at: now,
                                finished_at: now,
                                output: Value::Null,
                                error: Some(format!("template: {}", e)),
                            },
                        );
                        continue;
                    }
                };

                // spawn (各タスクは Arc を独立にクローン)
                let claude = self.claude.clone();
                let delivery = self.delivery.clone();
                let widgets = self.widgets.clone();
                let sdui = self.sdui.clone();
                joinset.spawn(async move {
                    let started_at = Utc::now();
                    let result = match node.kind {
                        NodeType::Ai => run_ai_node(&node, rendered, claude.as_ref()).await,
                        NodeType::Action => {
                            run_action_node(
                                &node,
                                rendered,
                                delivery.as_ref(),
                                widgets.as_ref(),
                                sdui.as_ref(),
                            )
                            .await
                        }
                        NodeType::Transform => run_transform_node(&node, rendered).await,
                    };
                    NodeOutcome {
                        node_id: node.id.clone(),
                        kind: node.kind,
                        started_at,
                        finished_at: Utc::now(),
                        result,
                    }
                });
            }

            // 4. 波内の全結果を回収
            while let Some(join_res) = joinset.join_next().await {
                match join_res {
                    Ok(outcome) => {
                        let node_id = outcome.node_id.clone();
                        match outcome.result {
                            Ok(output) => {
                                outputs.insert(node_id.clone(), output.clone());
                                completed.insert(
                                    node_id.clone(),
                                    NodeExecution {
                                        node_id,
                                        kind: outcome.kind,
                                        status: NodeStatus::Success,
                                        started_at: outcome.started_at,
                                        finished_at: outcome.finished_at,
                                        output,
                                        error: None,
                                    },
                                );
                            }
                            Err(e) => {
                                tracing::warn!(node_id = %node_id, error = %e, "node failed");
                                overall_failed = true;
                                taint_downstream(&node_id, &adjacency, &mut tainted);
                                completed.insert(
                                    node_id.clone(),
                                    NodeExecution {
                                        node_id,
                                        kind: outcome.kind,
                                        status: NodeStatus::Failed,
                                        started_at: outcome.started_at,
                                        finished_at: outcome.finished_at,
                                        output: Value::Null,
                                        error: Some(format!("{:#}", e)),
                                    },
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "join_next failed");
                        overall_failed = true;
                    }
                }
            }

            // 5. 次波構築: この波で完了したノードの後続の in-degree を -1
            let mut next_wave: Vec<String> = Vec::new();
            for nid in &current_wave {
                if let Some(succs) = adjacency.get(nid) {
                    for succ in succs {
                        if let Some(deg) = in_degree.get_mut(succ) {
                            if *deg > 0 {
                                *deg -= 1;
                                if *deg == 0 {
                                    next_wave.push(succ.clone());
                                }
                            }
                        }
                    }
                }
            }
            current_wave = next_wave;
        }

        // 6. NodeExecution を topo 順に整列 (出力の決定性のため)
        let nodes_vec: Vec<NodeExecution> = topo_order
            .iter()
            .filter_map(|id| completed.remove(id))
            .collect();

        let result = ExecutionResult {
            execution_id,
            workflow_id: workflow.id.clone(),
            status: if overall_failed {
                ExecutionStatus::Failed
            } else {
                ExecutionStatus::Success
            },
            trigger_data,
            started_at,
            finished_at: Utc::now(),
            nodes: nodes_vec,
            error: None,
        };
        self.persist(&result).await;
        result
    }

    async fn persist(&self, result: &ExecutionResult) {
        if let Err(e) = self.executions.save(result.clone()).await {
            tracing::error!(
                execution_id = %result.execution_id,
                error = %e,
                "failed to persist execution"
            );
        }
    }
}

/// spawned タスクから返す戻り値。
struct NodeOutcome {
    node_id: String,
    kind: NodeType,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    result: anyhow::Result<Value>,
}

/// `start` の下流すべてを tainted にマークする (BFS)。
fn taint_downstream(
    start: &str,
    adjacency: &HashMap<String, Vec<String>>,
    tainted: &mut HashSet<String>,
) {
    let mut stack: Vec<String> = vec![start.to_owned()];
    while let Some(id) = stack.pop() {
        if let Some(succs) = adjacency.get(&id) {
            for succ in succs {
                if tainted.insert(succ.clone()) {
                    stack.push(succ.clone());
                }
            }
        }
    }
}

// =============================================================
// Free-standing node-type dispatchers (Arc 経由で spawn できるよう)
// =============================================================

async fn run_ai_node(
    node: &Node,
    with: Value,
    claude: &ClaudeService,
) -> anyhow::Result<Value> {
    let params: AiParams = serde_json::from_value(with).context("ai node params")?;
    let opts = SpawnOptions {
        prompt: params.prompt,
        session_id: params.session_id,
        resume_session: params.resume_session,
        permission_mode: params.permission_mode.or_else(|| Some("plan".to_owned())),
        allowed_tools: params.allowed_tools,
        disallowed_tools: params.disallowed_tools,
        add_dirs: Vec::new(),
        model: params.model,
        mcp_config: None,
        permission_prompt_tool: None,
        extra_args: Vec::new(),
    };
    let result = claude.run(opts).await?;
    if result.is_error {
        anyhow::bail!("claude reported error: {:?}", result.result);
    }
    tracing::info!(
        node_id = %node.id,
        session_id = ?result.session_id,
        cost_usd = ?result.total_cost_usd,
        "ai node completed"
    );
    Ok(serde_json::json!({
        "text": result.result,
        "session_id": result.session_id,
        "cost_usd": result.total_cost_usd,
        "num_turns": result.num_turns,
        "duration_ms": result.duration_ms,
    }))
}

async fn run_action_node(
    node: &Node,
    with: Value,
    delivery: &DeliveryHub,
    widgets: &dyn WidgetRepo,
    sdui: &dyn SduiRepo,
) -> anyhow::Result<Value> {
    let using = node
        .using
        .as_deref()
        .context("action node missing 'using'")?;
    match using {
        "builtin/notify" => {
            let req: DispatchNotification =
                serde_json::from_value(with).context("notify params")?;
            let notif = req.into_notification();
            let receivers = delivery.dispatch_notification(notif.clone());
            Ok(serde_json::json!({
                "notification_id": notif.id,
                "receivers": receivers,
            }))
        }
        "builtin/widget-update" => {
            let params: WidgetUpdateParams =
                serde_json::from_value(with).context("widget-update params")?;
            let widget = widgets
                .update_bindings(params.widget_id, params.bindings)
                .await?;
            delivery.publish_widget_updated(&widget);
            Ok(serde_json::json!({
                "widget_id": widget.id,
                "updated_at": widget.updated_at,
            }))
        }
        "builtin/sdui-upsert" => {
            let spec: SduiSpec = serde_json::from_value(with).context("sdui-upsert params")?;
            sdui.upsert(spec.clone()).await?;
            delivery.publish_sdui_updated(&spec);
            Ok(serde_json::json!({ "spec_id": spec.id }))
        }
        "builtin/broadcast-target" => {
            let params: BroadcastParams =
                serde_json::from_value(with).context("broadcast params")?;
            let receivers =
                delivery.dispatch_notification(crate::domain::notification::Notification {
                    id: crate::domain::notification::NotificationId::new(),
                    target: params.target,
                    title: params.title,
                    body: params.body,
                    priority: Default::default(),
                    data: params.data,
                    created_at: Utc::now(),
                });
            Ok(serde_json::json!({ "receivers": receivers }))
        }
        other => anyhow::bail!("unknown action '{}'", other),
    }
}

async fn run_transform_node(node: &Node, with: Value) -> anyhow::Result<Value> {
    let using = node.using.as_deref().unwrap_or("builtin/pass-through");
    match using {
        "builtin/pass-through" => {
            if let Value::Object(ref m) = with {
                if let Some(d) = m.get("data") {
                    return Ok(d.clone());
                }
            }
            Ok(with)
        }
        "builtin/now" => Ok(serde_json::json!({
            "iso": Utc::now().to_rfc3339(),
            "ts": Utc::now().timestamp(),
        })),
        other => anyhow::bail!("unknown transform '{}'", other),
    }
}

// =============================================================
// Node parameter shapes (deserialize after template rendering)
// =============================================================

#[derive(Debug, Deserialize)]
struct AiParams {
    prompt: String,
    #[serde(default)]
    permission_mode: Option<String>,
    #[serde(default)]
    allowed_tools: Vec<String>,
    #[serde(default)]
    disallowed_tools: Vec<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    resume_session: bool,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WidgetUpdateParams {
    widget_id: WidgetId,
    #[serde(default)]
    bindings: Value,
}

#[derive(Debug, Deserialize)]
struct BroadcastParams {
    target: DeliveryTarget,
    title: String,
    body: String,
    #[serde(default)]
    data: Option<Value>,
}
