use std::convert::Infallible;
use std::path::PathBuf;

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::post,
    Json, Router,
};
use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    ai::{ClaudeRunResult, SpawnOptions},
    error::AppResult,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ai/prompt", post(prompt))
        .route("/ai/prompt/stream", post(prompt_stream))
}

#[derive(Debug, Deserialize)]
pub struct PromptRequest {
    pub prompt: String,
    #[serde(default)]
    pub session_id: Option<String>,
    /// `true` なら既存セッションを `--resume` で継続、`false` なら新規。
    #[serde(default)]
    pub resume_session: bool,
    /// 既定 `"plan"` (読取のみ)。`"acceptEdits"` や `"bypassPermissions"` も指定可。
    #[serde(default)]
    pub permission_mode: Option<String>,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub disallowed_tools: Vec<String>,
    #[serde(default)]
    pub add_dirs: Vec<PathBuf>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub mcp_config: Option<PathBuf>,
    #[serde(default)]
    pub permission_prompt_tool: Option<String>,
}

impl From<PromptRequest> for SpawnOptions {
    fn from(r: PromptRequest) -> Self {
        Self {
            prompt: r.prompt,
            session_id: r.session_id,
            resume_session: r.resume_session,
            permission_mode: r.permission_mode.or_else(|| Some("plan".to_owned())),
            allowed_tools: r.allowed_tools,
            disallowed_tools: r.disallowed_tools,
            add_dirs: r.add_dirs,
            model: r.model,
            mcp_config: r.mcp_config,
            permission_prompt_tool: r.permission_prompt_tool,
            extra_args: Vec::new(),
        }
    }
}

/// 同じ入力を `ClaudeService.run_stream()` に流し、SSE で逐次配信する。
///
/// ```text
/// curl -N -H 'Content-Type: application/json' \
///   -d '{"prompt":"Hello"}' \
///   http://127.0.0.1:8787/ai/prompt/stream
/// ```
async fn prompt_stream(
    State(s): State<AppState>,
    Json(req): Json<PromptRequest>,
) -> AppResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let opts: SpawnOptions = req.into();
    tracing::info!(prompt_len = opts.prompt.len(), "ai/prompt/stream invoked");
    let rx = s.claude.run_stream(opts).await?;
    let stream = ReceiverStream::new(rx).map(|ev| {
        let json = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".to_owned());
        let kind = ev.kind.clone();
        Ok::<_, Infallible>(Event::default().event(kind).data(json))
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

async fn prompt(
    State(s): State<AppState>,
    Json(req): Json<PromptRequest>,
) -> AppResult<Json<ClaudeRunResult>> {
    tracing::info!(
        prompt_len = req.prompt.len(),
        resume = req.resume_session,
        has_session_id = req.session_id.is_some(),
        "ai/prompt invoked"
    );
    let opts: SpawnOptions = req.into();
    let result = s.claude.run(opts).await?;
    tracing::info!(
        session_id = ?result.session_id,
        num_turns = ?result.num_turns,
        cost_usd = ?result.total_cost_usd,
        is_error = result.is_error,
        "ai/prompt completed"
    );
    Ok(Json(result))
}
