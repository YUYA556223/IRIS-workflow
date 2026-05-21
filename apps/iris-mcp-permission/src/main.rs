//! MCP stdio server: Claude Code の `--permission-prompt-tool` をホストの
//! `/permission/request` HTTP API へ橋渡しする最小実装。
//!
//! プロトコル: JSON-RPC 2.0 over stdio (newline-delimited)。
//! 対応メソッド:
//!  - `initialize`         : サーバ capability を返す
//!  - `tools/list`         : `prompt` ツール定義を返す
//!  - `tools/call`         : `prompt` を呼ぶと、引数を host-backend に転送
//!  - `notifications/*`    : 無視
//!  - その他               : `-32601 method not found`
//!
//! 設定例 (Claude Code の `--mcp-config <file>` に渡す JSON):
//! ```json
//! {
//!   "mcpServers": {
//!     "iris-permission": {
//!       "command": "iris-mcp-permission",
//!       "env": { "IRIS_BACKEND_URL": "http://127.0.0.1:8787" }
//!     }
//!   }
//! }
//! ```
//!
//! その上で Claude Code 起動時に
//! `--permission-prompt-tool mcp__iris-permission__prompt` を渡す。

use std::io::IsTerminal;

use anyhow::Result;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "iris-mcp-permission";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const TOOL_NAME: &str = "prompt";

#[tokio::main]
async fn main() -> Result<()> {
    // stdout を JSON-RPC に使うので、log は stderr へ。
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .compact()
        .init();

    if std::io::stdin().is_terminal() {
        eprintln!(
            "{} {} — this is an MCP stdio server. Run via Claude Code's --mcp-config, not interactively.",
            SERVER_NAME, SERVER_VERSION
        );
        return Ok(());
    }

    let backend_url = std::env::var("IRIS_BACKEND_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8787".to_owned());
    tracing::info!(%backend_url, "iris-mcp-permission starting");

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = reader.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, raw = %line, "invalid json");
                continue;
            }
        };

        let id = req.get("id").cloned();
        let method = req
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_owned();

        // Notification (no id) — replies are skipped per JSON-RPC.
        let is_notification = id.is_none() || id.as_ref().map_or(false, |v| v.is_null());

        let response = match handle(&method, &req, &http, &backend_url).await {
            Ok(Some(result)) => Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result,
            })),
            Ok(None) => None, // intentionally no reply (notification)
            Err(MethodError::NotFound) => Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("method not found: {}", method) }
            })),
            Err(MethodError::Internal(e)) => {
                tracing::error!(error = %e, method, "method failed");
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32603, "message": e.to_string() }
                }))
            }
        };

        if let Some(resp) = response {
            if is_notification {
                continue;
            }
            let mut bytes = serde_json::to_vec(&resp)?;
            bytes.push(b'\n');
            stdout.write_all(&bytes).await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}

enum MethodError {
    NotFound,
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for MethodError {
    fn from(e: anyhow::Error) -> Self {
        MethodError::Internal(e)
    }
}

async fn handle(
    method: &str,
    req: &Value,
    http: &reqwest::Client,
    backend_url: &str,
) -> Result<Option<Value>, MethodError> {
    match method {
        "initialize" => Ok(Some(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION }
        }))),
        "tools/list" => Ok(Some(json!({
            "tools": [{
                "name": TOOL_NAME,
                "description": "Ask the IRIS host to approve or deny a Claude Code tool invocation.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "tool_name":  { "type": "string", "description": "Tool Claude wants to use (Bash, Edit, ...)" },
                        "tool_input": { "type": "object", "description": "Original tool input args" }
                    },
                    "required": ["tool_name", "tool_input"]
                }
            }]
        }))),
        "tools/call" => {
            let params = req.get("params").cloned().unwrap_or_else(|| json!({}));
            let tool = params
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("");
            if tool != TOOL_NAME {
                return Err(MethodError::Internal(anyhow::anyhow!(
                    "unknown tool: {}",
                    tool
                )));
            }
            let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            tracing::info!(?arguments, "forwarding permission request to host");
            let resp = http
                .post(format!("{}/permission/request", backend_url))
                .json(&arguments)
                .send()
                .await
                .map_err(|e| MethodError::Internal(e.into()))?
                .error_for_status()
                .map_err(|e| MethodError::Internal(e.into()))?;
            let body: Value = resp
                .json()
                .await
                .map_err(|e| MethodError::Internal(e.into()))?;
            // MCP tools return a list of content items. The Claude Code
            // permission-prompt-tool expects the JSON response as a text
            // content item.
            Ok(Some(json!({
                "content": [
                    { "type": "text", "text": serde_json::to_string(&body).unwrap() }
                ]
            })))
        }
        // 通知系は受けて無視
        m if m.starts_with("notifications/") => Ok(None),
        _ => Err(MethodError::NotFound),
    }
}
