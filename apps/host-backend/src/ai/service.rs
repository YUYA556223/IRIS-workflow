use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::{mpsc, Semaphore};

use super::process::{ClaudeProcessHandle, SpawnOptions};
use super::stream::StreamEvent;

/// 1 回の `claude` 実行の集約結果。
#[derive(Debug, Clone, Serialize)]
pub struct ClaudeRunResult {
    pub session_id: Option<String>,
    pub result: Option<String>,
    pub total_cost_usd: Option<f64>,
    pub num_turns: Option<u32>,
    pub duration_ms: Option<u64>,
    pub is_error: bool,
    pub exit_code: Option<i32>,
    /// 受け取った生イベント列 (デバッグ・ワークフロー記録用)。
    pub events: Vec<StreamEvent>,
}

/// Claude Code 呼び出しの高レベル API。
///
/// - `Semaphore` で同時実行数を制限 (default = CPU/2)
/// - wall-clock タイムアウトでハングしたプロセスを救出
/// - 1 回の呼び出しで全イベントを集約し `ClaudeRunResult` を返す
///
/// ストリーミング配信 (SSE / WS) が必要になったら別メソッドを追加する想定。
pub struct ClaudeService {
    sem: Arc<Semaphore>,
    timeout: Duration,
}

impl ClaudeService {
    pub fn new(max_concurrency: usize, timeout: Duration) -> Self {
        Self {
            sem: Arc::new(Semaphore::new(max_concurrency.max(1))),
            timeout,
        }
    }

    /// プロンプトを 1 本実行し、最終結果を返す。
    /// セマフォ獲得 → spawn → イベント集約 → wait の流れ。
    pub async fn run(&self, opts: SpawnOptions) -> anyhow::Result<ClaudeRunResult> {
        let _permit = self
            .sem
            .acquire()
            .await
            .map_err(|e| anyhow::anyhow!("claude semaphore closed: {e}"))?;

        match tokio::time::timeout(self.timeout, Self::run_inner(opts)).await {
            Ok(res) => res,
            Err(_) => Err(anyhow::anyhow!(
                "claude run timed out after {:?}",
                self.timeout
            )),
        }
    }

    /// プロンプトを 1 本実行し、`StreamEvent` を逐次 channel 経由で配信する。
    /// セマフォは Receiver が drop されるまで保持。Receiver が drop されたら
    /// プロセスは即 kill される。
    pub async fn run_stream(
        &self,
        opts: SpawnOptions,
    ) -> anyhow::Result<mpsc::Receiver<StreamEvent>> {
        let permit = Arc::clone(&self.sem)
            .acquire_owned()
            .await
            .map_err(|e| anyhow::anyhow!("claude semaphore closed: {e}"))?;
        let mut handle = ClaudeProcessHandle::spawn(&opts)?;
        let (tx, rx) = mpsc::channel::<StreamEvent>(32);

        tokio::spawn(async move {
            let _permit = permit; // ストリーム終了まで permit を保持
            loop {
                match handle.next_event().await {
                    Ok(Some(ev)) => {
                        if tx.send(ev).await.is_err() {
                            // Receiver drop → ストリーム購読打ち切り。プロセスを kill。
                            let _ = handle.kill().await;
                            return;
                        }
                    }
                    Ok(None) => return,
                    Err(e) => {
                        tracing::error!(error = %e, "claude stream parse error");
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn run_inner(opts: SpawnOptions) -> anyhow::Result<ClaudeRunResult> {
        let mut handle = ClaudeProcessHandle::spawn(&opts)?;

        let mut events = Vec::new();
        let mut session_id = opts.session_id.clone();
        let mut result_text = None;
        let mut total_cost_usd = None;
        let mut num_turns = None;
        let mut duration_ms = None;
        let mut is_error = false;

        while let Some(ev) = handle.next_event().await? {
            if session_id.is_none() {
                if let Some(sid) = &ev.session_id {
                    session_id = Some(sid.clone());
                }
            }
            if ev.is_result() {
                result_text = ev.result.clone();
                total_cost_usd = ev.total_cost_usd;
                num_turns = ev.num_turns;
                duration_ms = ev.duration_ms;
                is_error = ev.errored();
            } else if ev.errored() {
                is_error = true;
            }
            events.push(ev);
        }

        let status = handle.wait().await?;
        if !status.success() {
            tracing::warn!(?status, events_len = events.len(), "claude exited non-zero");
        }

        Ok(ClaudeRunResult {
            session_id,
            result: result_text,
            total_cost_usd,
            num_turns,
            duration_ms,
            is_error,
            exit_code: status.code(),
            events,
        })
    }
}
