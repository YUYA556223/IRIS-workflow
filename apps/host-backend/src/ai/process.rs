use std::path::PathBuf;
use std::process::Stdio;

use anyhow::Context;
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{Child, ChildStdout, Command};

use super::stream::StreamEvent;

/// `claude` CLI を 1 回起動する際のオプション。
#[derive(Debug, Clone)]
pub struct SpawnOptions {
    pub prompt: String,
    /// 既存セッションID。`resume_session = true` なら `--resume` で続き、
    /// `false` なら新規セッションのID指定として `--session-id` を使う。
    pub session_id: Option<String>,
    pub resume_session: bool,
    /// `plan` / `acceptEdits` / `bypassPermissions`。
    pub permission_mode: Option<String>,
    pub allowed_tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    pub add_dirs: Vec<PathBuf>,
    pub model: Option<String>,
    /// `--mcp-config <path>` で読み込ませる JSON ファイル (任意)。
    pub mcp_config: Option<PathBuf>,
    /// `--permission-prompt-tool mcp__<server>__<tool>` を指定 (任意)。
    /// 設定時は Claude が許可要する tool 実行直前にこの MCP tool を呼ぶ。
    pub permission_prompt_tool: Option<String>,
    /// 任意の追加引数 (デバッグ用)。
    pub extra_args: Vec<String>,
}

impl Default for SpawnOptions {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            session_id: None,
            resume_session: false,
            permission_mode: Some("plan".to_owned()),
            allowed_tools: Vec::new(),
            disallowed_tools: Vec::new(),
            add_dirs: Vec::new(),
            model: None,
            mcp_config: None,
            permission_prompt_tool: None,
            extra_args: Vec::new(),
        }
    }
}

/// 起動した `claude` 子プロセスのハンドル。stdout の NDJSON ストリームから
/// 順次 `StreamEvent` を取り出す。
pub struct ClaudeProcessHandle {
    child: Child,
    lines: Lines<BufReader<ChildStdout>>,
}

impl ClaudeProcessHandle {
    /// 子プロセスを spawn する。`claude` 実行ファイルは PATH から自動解決
    /// (Windows では `claude.exe` を想定。`.cmd`/`.bat` シムは未サポート)。
    pub fn spawn(opts: &SpawnOptions) -> anyhow::Result<Self> {
        let mut cmd = Command::new("claude");
        cmd.arg("--print").arg(&opts.prompt);
        cmd.arg("--output-format").arg("stream-json");
        cmd.arg("--verbose"); // stream-json は verbose 必須

        if let Some(sid) = &opts.session_id {
            if opts.resume_session {
                cmd.arg("--resume").arg(sid);
            } else {
                cmd.arg("--session-id").arg(sid);
            }
        }
        if let Some(pm) = &opts.permission_mode {
            cmd.arg("--permission-mode").arg(pm);
        }
        if !opts.allowed_tools.is_empty() {
            cmd.arg("--allowed-tools").arg(opts.allowed_tools.join(","));
        }
        if !opts.disallowed_tools.is_empty() {
            cmd.arg("--disallowed-tools").arg(opts.disallowed_tools.join(","));
        }
        for d in &opts.add_dirs {
            cmd.arg("--add-dir").arg(d);
        }
        if let Some(m) = &opts.model {
            cmd.arg("--model").arg(m);
        }
        if let Some(cfg) = &opts.mcp_config {
            cmd.arg("--mcp-config").arg(cfg);
        }
        if let Some(tool) = &opts.permission_prompt_tool {
            cmd.arg("--permission-prompt-tool").arg(tool);
        }
        for extra in &opts.extra_args {
            cmd.arg(extra);
        }

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.kill_on_drop(true);

        tracing::debug!(?cmd, "spawning claude");
        let mut child = cmd.spawn().context("spawn claude CLI")?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("claude stdout not piped"))?;
        let lines = BufReader::new(stdout).lines();
        Ok(Self { child, lines })
    }

    /// 次の NDJSON 行を読み、`StreamEvent` にパースして返す。
    ///
    /// - 空行はスキップ
    /// - JSON パース失敗時は警告ログを出してその行を読み飛ばし続行
    /// - 子プロセスの stdout EOF で `Ok(None)`
    pub async fn next_event(&mut self) -> anyhow::Result<Option<StreamEvent>> {
        loop {
            let line = self.lines.next_line().await?;
            let Some(line) = line else { return Ok(None) };
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<StreamEvent>(&line) {
                Ok(ev) => return Ok(Some(ev)),
                Err(e) => {
                    tracing::warn!(error = %e, raw = %line, "failed to parse claude event");
                    continue;
                }
            }
        }
    }

    /// プロセスを kill する。
    pub async fn kill(&mut self) -> anyhow::Result<()> {
        self.child.start_kill().context("start_kill claude")?;
        let _ = self.child.wait().await;
        Ok(())
    }

    /// 子プロセスの終了を待つ。stdout が EOF になってから呼ぶ想定。
    pub async fn wait(mut self) -> anyhow::Result<std::process::ExitStatus> {
        let status = self.child.wait().await.context("wait claude")?;
        Ok(status)
    }
}
