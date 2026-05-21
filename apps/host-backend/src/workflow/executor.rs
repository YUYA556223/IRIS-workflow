use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Context;
use chrono::{DateTime, Utc};
use futures::Future;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::{
    ai::{ClaudeService, SpawnOptions},
    delivery::DeliveryHub,
    domain::{notification::DispatchNotification, SduiSpec, WidgetId},
    mqtt::MqttBus,
    storage::{executions::ExecutionRepo, SduiRepo, WidgetRepo},
};

use super::{
    dag::topo_sort,
    dsl::{BackoffStrategy, Node, NodeType, Workflow},
    store::WorkflowStore,
    template::{render_string, render_value, TemplateContext},
};

/// サブワークフロー再帰の安全限界。これを超えると即 Failed。
const MAX_SUBWORKFLOW_DEPTH: usize = 16;

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
    /// このノードで実行された試行回数 (リトライ込み)。
    #[serde(default = "default_attempts")]
    pub attempts: u32,
}

fn default_attempts() -> u32 {
    1
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
/// 波形 (wave) 並列実行 + 失敗下流 taint + 条件付き実行 (`when`) +
/// リトライ (`retry`) + サブワークフロー (`NodeType::Workflow`) を全てサポート。
pub struct WorkflowExecutor {
    claude: Arc<ClaudeService>,
    delivery: Arc<DeliveryHub>,
    widgets: Arc<dyn WidgetRepo>,
    sdui: Arc<dyn SduiRepo>,
    executions: Arc<dyn ExecutionRepo>,
    workflows: Arc<WorkflowStore>,
    mqtt: Option<Arc<MqttBus>>,
}

impl WorkflowExecutor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        claude: Arc<ClaudeService>,
        delivery: Arc<DeliveryHub>,
        widgets: Arc<dyn WidgetRepo>,
        sdui: Arc<dyn SduiRepo>,
        executions: Arc<dyn ExecutionRepo>,
        workflows: Arc<WorkflowStore>,
        mqtt: Option<Arc<MqttBus>>,
    ) -> Self {
        Self {
            claude,
            delivery,
            widgets,
            sdui,
            executions,
            workflows,
            mqtt,
        }
    }

    /// 公開エントリ。トップレベル実行 (depth=0)。
    /// 実行終了後に `ExecutionRepo` へ自動保存される。
    pub fn execute(
        self: Arc<Self>,
        workflow: Arc<Workflow>,
        trigger_data: Value,
    ) -> Pin<Box<dyn Future<Output = ExecutionResult> + Send>> {
        self.execute_at_depth(workflow, trigger_data, 0)
    }

    /// 再帰可能な内部実装。Box::pin して dyn 化することで、サブワークフローでの
    /// 自己再帰時に型が無限サイズになるのを防いでいる。
    fn execute_at_depth(
        self: Arc<Self>,
        workflow: Arc<Workflow>,
        trigger_data: Value,
        depth: usize,
    ) -> Pin<Box<dyn Future<Output = ExecutionResult> + Send>> {
        Box::pin(async move {
            let execution_id = Uuid::new_v4();
            let started_at = Utc::now();

            // 0. 再帰深度チェック
            if depth > MAX_SUBWORKFLOW_DEPTH {
                let result = ExecutionResult {
                    execution_id,
                    workflow_id: workflow.id.clone(),
                    status: ExecutionStatus::Failed,
                    trigger_data: trigger_data.clone(),
                    started_at,
                    finished_at: Utc::now(),
                    nodes: Vec::new(),
                    error: Some(format!(
                        "sub-workflow depth exceeded {}",
                        MAX_SUBWORKFLOW_DEPTH
                    )),
                };
                self.persist(&result).await;
                return result;
            }

            // 1. 検証 (topo_sort)
            let topo_order = match topo_sort(&workflow) {
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

            // 2. in-degree と隣接リスト + ノード ID → &Node の O(1) ルックアップ表
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
            let nodes_by_id: HashMap<&str, &Node> = workflow
                .nodes
                .iter()
                .map(|n| (n.id.as_str(), n))
                .collect();

            let mut outputs: HashMap<String, Value> = HashMap::new();
            let mut tainted: HashSet<String> = HashSet::new();
            let mut completed: HashMap<String, NodeExecution> = HashMap::new();
            let mut overall_failed = false;

            let mut current_wave: Vec<String> = workflow
                .nodes
                .iter()
                .filter(|n| in_degree.get(&n.id).copied().unwrap_or(0) == 0)
                .map(|n| n.id.clone())
                .collect();

            while !current_wave.is_empty() {
                let mut joinset: JoinSet<NodeOutcome> = JoinSet::new();

                for node_id in &current_wave {
                    let Some(node_ref) = nodes_by_id.get(node_id.as_str()) else {
                        continue;
                    };
                    let node = (*node_ref).clone();

                    if tainted.contains(node_id) {
                        completed.insert(node.id.clone(), skipped_node(&node));
                        continue;
                    }

                    let ctx = TemplateContext {
                        trigger: &trigger_data,
                        outputs: &outputs,
                    };

                    // `when` 評価。falsy なら Skipped で下流継続 (taint しない)。
                    if let Some(cond_template) = &node.when {
                        match render_string(cond_template, &ctx) {
                            Ok(s) => {
                                if !is_truthy(&s) {
                                    completed.insert(node.id.clone(), skipped_node(&node));
                                    // 下流が `{{ <node>.x }}` を参照しても "" になるよう Null を残す。
                                    outputs.insert(node.id.clone(), Value::Null);
                                    continue;
                                }
                            }
                            Err(e) => {
                                overall_failed = true;
                                taint_downstream(&node.id, &adjacency, &mut tainted);
                                completed.insert(
                                    node.id.clone(),
                                    pre_dispatch_failed(&node, format!("when: {}", e)),
                                );
                                continue;
                            }
                        }
                    }

                    let rendered = match render_value(&node.with, &ctx) {
                        Ok(v) => v,
                        Err(e) => {
                            overall_failed = true;
                            taint_downstream(&node.id, &adjacency, &mut tainted);
                            completed.insert(
                                node.id.clone(),
                                pre_dispatch_failed(&node, format!("template: {}", e)),
                            );
                            continue;
                        }
                    };

                    let executor_arc = Arc::clone(&self);
                    let retry = node.retry.clone();
                    joinset.spawn(async move {
                        let started_at = Utc::now();
                        let max_attempts = retry.as_ref().map(|r| r.max_attempts.max(1)).unwrap_or(1);
                        let delay_ms = retry.as_ref().map(|r| r.delay_ms).unwrap_or(0);
                        let backoff = retry.as_ref().map(|r| r.backoff).unwrap_or_default();

                        // `rendered` は最終 attempt で move、それ以前は clone。
                        // max_attempts==1 (リトライなし) の場合は 1 回も clone しない。
                        let mut buf = Some(rendered);
                        let mut last_err: Option<anyhow::Error> = None;
                        let mut attempts: u32 = 0;
                        for attempt in 1..=max_attempts {
                            attempts = attempt;
                            let with = if attempt == max_attempts {
                                buf.take().expect("present until last attempt")
                            } else {
                                buf.as_ref().expect("present").clone()
                            };
                            let result = run_node_on(&executor_arc, &node, with, depth).await;
                            match result {
                                Ok(output) => {
                                    return NodeOutcome {
                                        node_id: node.id.clone(),
                                        kind: node.kind,
                                        started_at,
                                        finished_at: Utc::now(),
                                        result: Ok(output),
                                        attempts,
                                    };
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        node_id = %node.id,
                                        attempt,
                                        max_attempts,
                                        error = %e,
                                        "node attempt failed"
                                    );
                                    last_err = Some(e);
                                    if attempt < max_attempts {
                                        let wait = match backoff {
                                            BackoffStrategy::Constant => delay_ms,
                                            BackoffStrategy::Exponential => {
                                                delay_ms.saturating_mul(2u64.saturating_pow(attempt - 1))
                                            }
                                        };
                                        if wait > 0 {
                                            tokio::time::sleep(
                                                std::time::Duration::from_millis(wait),
                                            )
                                            .await;
                                        }
                                    }
                                }
                            }
                        }
                        NodeOutcome {
                            node_id: node.id.clone(),
                            kind: node.kind,
                            started_at,
                            finished_at: Utc::now(),
                            result: Err(last_err
                                .unwrap_or_else(|| anyhow::anyhow!("unknown error"))),
                            attempts,
                        }
                    });
                }

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
                                            attempts: outcome.attempts,
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
                                            attempts: outcome.attempts,
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

                // 次波構築
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
        })
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

struct NodeOutcome {
    node_id: String,
    kind: NodeType,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    result: anyhow::Result<Value>,
    attempts: u32,
}

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

/// `when` 評価ロジック。trimmed (case-insensitive) で
/// `""` / `"false"` / `"no"` / `"0"` / `"null"` / `"off"` のいずれかなら falsy。
fn is_truthy(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    !["false", "no", "0", "null", "off"]
        .iter()
        .any(|f| trimmed.eq_ignore_ascii_case(f))
}

/// Pre-dispatch で「実行はせず完了状態に置く」ノードの NodeExecution を作る共通形。
fn skipped_node(node: &Node) -> NodeExecution {
    let now = Utc::now();
    NodeExecution {
        node_id: node.id.clone(),
        kind: node.kind,
        status: NodeStatus::Skipped,
        started_at: now,
        finished_at: now,
        output: Value::Null,
        error: None,
        attempts: 0,
    }
}

fn pre_dispatch_failed(node: &Node, error: String) -> NodeExecution {
    let now = Utc::now();
    NodeExecution {
        node_id: node.id.clone(),
        kind: node.kind,
        status: NodeStatus::Failed,
        started_at: now,
        finished_at: now,
        output: Value::Null,
        error: Some(error),
        attempts: 0,
    }
}

/// 単一ノードを実行する。`executor` 経由でサブ依存 (Arc 群) にアクセスする。
async fn run_node_on(
    executor: &Arc<WorkflowExecutor>,
    node: &Node,
    with: Value,
    depth: usize,
) -> anyhow::Result<Value> {
    match node.kind {
        NodeType::Ai => run_ai_node(node, with, &executor.claude).await,
        NodeType::Action => {
            run_action_node(
                node,
                with,
                &executor.delivery,
                executor.widgets.as_ref(),
                executor.sdui.as_ref(),
                executor.mqtt.as_ref(),
            )
            .await
        }
        NodeType::Transform => run_transform_node(node, with).await,
        NodeType::Workflow => {
            run_workflow_node(
                node,
                with,
                executor.clone(),
                executor.workflows.clone(),
                depth,
            )
            .await
        }
    }
}

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
    mqtt: Option<&Arc<MqttBus>>,
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
            // notify とフィールドが完全に同じになったため alias。priority は default 扱い。
            let req: DispatchNotification =
                serde_json::from_value(with).context("broadcast params")?;
            let notif = req.into_notification();
            let receivers = delivery.dispatch_notification(notif.clone());
            Ok(serde_json::json!({
                "notification_id": notif.id,
                "receivers": receivers,
            }))
        }
        "builtin/fail" => {
            let params: FailParams =
                serde_json::from_value(with).unwrap_or(FailParams { reason: None });
            anyhow::bail!(
                "builtin/fail invoked: {}",
                params.reason.unwrap_or_else(|| "intentional".into())
            )
        }
        "builtin/mqtt-publish" => {
            let bus = mqtt.ok_or_else(|| {
                anyhow::anyhow!("mqtt-publish requires IRIS_MQTT_BROKER to be configured")
            })?;
            let params: MqttPublishParams =
                serde_json::from_value(with).context("mqtt-publish params")?;
            let payload_bytes: Vec<u8> = match params.payload {
                Value::String(s) => s.into_bytes(),
                Value::Null => Vec::new(),
                other => serde_json::to_vec(&other)?,
            };
            bus.publish(&params.topic, payload_bytes.clone(), params.retain)
                .await?;
            Ok(serde_json::json!({
                "topic": params.topic,
                "bytes": payload_bytes.len(),
            }))
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

async fn run_workflow_node(
    node: &Node,
    with: Value,
    executor: Arc<WorkflowExecutor>,
    workflows: Arc<WorkflowStore>,
    depth: usize,
) -> anyhow::Result<Value> {
    let params: WorkflowNodeParams =
        serde_json::from_value(with).context("workflow node params")?;
    let sub_wf: Arc<Workflow> = workflows
        .get(&params.workflow_id)
        .ok_or_else(|| anyhow::anyhow!("sub-workflow not found: '{}'", params.workflow_id))?;
    tracing::info!(
        node_id = %node.id,
        sub_workflow = %params.workflow_id,
        depth,
        "invoking sub-workflow"
    );
    let sub_result = executor
        .execute_at_depth(sub_wf, params.trigger_data, depth + 1)
        .await;
    if matches!(sub_result.status, ExecutionStatus::Failed) {
        anyhow::bail!(
            "sub-workflow '{}' failed: {}",
            params.workflow_id,
            sub_result.error.clone().unwrap_or_default()
        );
    }
    Ok(serde_json::to_value(&sub_result)?)
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
struct FailParams {
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorkflowNodeParams {
    workflow_id: String,
    #[serde(default)]
    trigger_data: Value,
}

#[derive(Debug, Deserialize)]
struct MqttPublishParams {
    pub topic: String,
    /// 文字列ならそのまま、それ以外なら JSON にシリアライズして送る。
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub retain: bool,
}
