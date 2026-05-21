//! トリガハブ: 各ワークフローの `trigger` 設定に基づき、自動的に
//! `WorkflowExecutor.execute()` を呼び出す。
//!
//! - `Manual`  : 自動起動なし (REST `POST /workflows/:id/run` のみ)
//! - `Cron`    : cron 式 (6-7 フィールド: sec min hour dom mon dow [year]) に従い起動
//! - `Webhook` : `POST /hooks/<path>` でマッチして起動 (API ハンドラから lookup)
//! - `FsWatch` : ファイル/ディレクトリ変更で起動 (notify)
//! - `Mqtt`    : (P8) MQTT トピック購読で起動
//!
//! `sync()` を呼ぶと現在の `WorkflowStore` 内容に合わせて全トリガを再登録する。
//! ワークフローの upsert/delete 後にも呼ぶ想定。

use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use cron::Schedule;
use notify::Watcher;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::workflow::{Trigger, Workflow, WorkflowExecutor, WorkflowStore};

/// 内部状態。`sync()` で全置換する。
struct State {
    cron: HashMap<String, JoinHandle<()>>,
    /// FS タスクは watcher を tuple で同梱保持し、ハブを drop すると停止する。
    fs: HashMap<String, FsTask>,
    /// webhook path -> workflow id
    webhook: HashMap<String, String>,
}

struct FsTask {
    /// 保持するだけで `Drop` でファイル監視が止まる。
    _watcher: notify::RecommendedWatcher,
    handle: JoinHandle<()>,
}

impl Drop for FsTask {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub struct TriggerHub {
    workflows: Arc<WorkflowStore>,
    executor: Arc<WorkflowExecutor>,
    state: Mutex<State>,
}

impl TriggerHub {
    pub fn new(workflows: Arc<WorkflowStore>, executor: Arc<WorkflowExecutor>) -> Self {
        Self {
            workflows,
            executor,
            state: Mutex::new(State {
                cron: HashMap::new(),
                fs: HashMap::new(),
                webhook: HashMap::new(),
            }),
        }
    }

    /// 全 cron/fs タスクを停止し、現在の `WorkflowStore` 内容に基づいて再登録する。
    /// ワークフローの upsert/delete 後に呼ぶ想定。
    pub async fn sync(&self) {
        let mut state = self.state.lock().await;

        // 既存停止
        for (_, h) in state.cron.drain() {
            h.abort();
        }
        state.fs.clear(); // Drop で watcher 停止 + task abort
        state.webhook.clear();

        let workflows = self.workflows.list();
        let mut cron_count = 0;
        let mut fs_count = 0;
        let mut webhook_count = 0;

        for wf in workflows {
            match &wf.trigger {
                Trigger::Manual => {}
                Trigger::Cron { schedule } => match spawn_cron(self.executor.clone(), wf.clone(), schedule) {
                    Ok(h) => {
                        state.cron.insert(wf.id.clone(), h);
                        cron_count += 1;
                    }
                    Err(e) => tracing::error!(workflow_id = %wf.id, error = %e, "cron schedule invalid"),
                },
                Trigger::Webhook { path } => {
                    let normalized = path.trim_start_matches('/').to_owned();
                    state.webhook.insert(normalized, wf.id.clone());
                    webhook_count += 1;
                }
                Trigger::FsWatch { path } => {
                    match spawn_fs_watch(self.executor.clone(), wf.clone(), path) {
                        Ok(task) => {
                            state.fs.insert(wf.id.clone(), task);
                            fs_count += 1;
                        }
                        Err(e) => tracing::error!(workflow_id = %wf.id, error = %e, "fs-watch failed"),
                    }
                }
                Trigger::Mqtt { .. } => {
                    tracing::debug!(workflow_id = %wf.id, "mqtt trigger registered (P8 todo)");
                }
            }
        }

        tracing::info!(
            cron = cron_count,
            fs = fs_count,
            webhook = webhook_count,
            "triggers synced"
        );
    }

    /// 指定 webhook path にマッチするワークフロー id を返す (path 先頭の `/` は無視)。
    pub async fn lookup_webhook(&self, path: &str) -> Option<String> {
        let normalized = path.trim_start_matches('/');
        let state = self.state.lock().await;
        state.webhook.get(normalized).cloned()
    }
}

// ============== helpers ==============

fn spawn_cron(
    executor: Arc<WorkflowExecutor>,
    workflow: Workflow,
    schedule_str: &str,
) -> anyhow::Result<JoinHandle<()>> {
    let schedule = Schedule::from_str(schedule_str)?;
    let schedule_label = schedule_str.to_owned();
    let workflow_id = workflow.id.clone();

    let handle = tokio::spawn(async move {
        loop {
            let now = Utc::now();
            let Some(next) = schedule.upcoming(Utc).next() else {
                tracing::warn!(workflow_id = %workflow.id, "cron schedule produced no upcoming time, stopping");
                return;
            };
            let dur = match (next - now).to_std() {
                Ok(d) => d,
                Err(_) => {
                    // 過去時刻 — 少し休んで次へ
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };
            tracing::debug!(workflow_id = %workflow.id, ?dur, next = %next, "cron next fire");
            tokio::time::sleep(dur).await;

            let trigger_data = serde_json::json!({
                "trigger": "cron",
                "schedule": schedule_label,
                "fired_at": Utc::now().to_rfc3339(),
            });
            let result = executor.execute(&workflow, trigger_data).await;
            tracing::info!(
                workflow_id = %workflow.id,
                execution_id = %result.execution_id,
                status = ?result.status,
                "cron-triggered execution finished"
            );
        }
    });

    tracing::info!(%workflow_id, schedule = %schedule_str, "cron trigger registered");
    Ok(handle)
}

fn spawn_fs_watch(
    executor: Arc<WorkflowExecutor>,
    workflow: Workflow,
    path: &str,
) -> anyhow::Result<FsTask> {
    let watch_path = Path::new(path).to_path_buf();
    if !watch_path.exists() {
        anyhow::bail!("fs-watch path does not exist: {}", watch_path.display());
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel::<notify::Event>(100);

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(ev) = res {
            // 同期コールバックなので blocking_send。
            let _ = tx.blocking_send(ev);
        }
    })?;
    watcher.watch(&watch_path, notify::RecursiveMode::Recursive)?;

    let workflow_id = workflow.id.clone();
    let handle = tokio::spawn(async move {
        while let Some(ev) = rx.recv().await {
            let trigger_data = serde_json::json!({
                "trigger": "fs-watch",
                "kind": format!("{:?}", ev.kind),
                "paths": ev.paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                "fired_at": Utc::now().to_rfc3339(),
            });
            let result = executor.execute(&workflow, trigger_data).await;
            tracing::info!(
                workflow_id = %workflow.id,
                execution_id = %result.execution_id,
                status = ?result.status,
                "fs-triggered execution finished"
            );
        }
    });

    tracing::info!(%workflow_id, path = %watch_path.display(), "fs-watch trigger registered");
    Ok(FsTask {
        _watcher: watcher,
        handle,
    })
}
