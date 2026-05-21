# 04. Claude Code ブリッジ設計

## 目的

中央 AI エンジンとして使う **Claude Code (CLI)** を、Rust ホストバックエンドからどう駆動するかを定義します。

## 基本方針

> Anthropic API を直接叩かず、ユーザの `claude` CLI をサブプロセスとして起動し、stream-json で対話する。

これにより、ユーザの既存認証 (`~/.claude/`) と MCP server 設定 (`~/.claude.json` 等) をそのまま活用できます。

## アーキテクチャ

```
WorkflowEngine
    │
    ▼
AgentNode
    │
    ▼
ClaudeProcessHandle ───spawn (tokio::process::Command)───► claude CLI
    │  stdin (NDJSON)                                       │
    │  stdout (stream-json)                                 │
    │  stderr (logs)                                        │
    │                                                       │
    └─◄────── permission prompts ◄── MCP server (rmcp) ◄────┘
                                       │
                                       ▼
                              push to user device (iPhone/Win)
                              for approval
```

## 呼び出し方

```bash
claude -p "<prompt>" \
  --output-format stream-json \
  --input-format stream-json \
  --include-partial-messages \
  --verbose \
  --session-id <uuid> \
  --permission-mode plan \
  --permission-prompt-tool mcp__iris-permission__prompt \
  --add-dir <workflow_sandbox>/ \
  --allowed-tools "Read,Grep,Bash(git status:*)"
```

Rust 側は `tokio::process::Command` で spawn し、stdout を `LinesCodec` で行単位に分割、各行を `serde_json::from_str` で `StreamEvent` (`system_init` / `assistant_delta` / `tool_use` / `result` / `error`) にデシリアライズします。

## セッション管理

- **粒度**: ワークフロー実行 1 本 = 1 セッション (UUID は Rust 側で生成)
- **ノード間継続**: 同一ワークフロー内の連続するエージェントノードは `--resume <session-id>` で会話継続
- **長期常駐**: 採用しない (コンテキスト肥大とトークン浪費を避けるため)
- **ログ**: セッション ID と `total_cost_usd` / `num_turns` / `total_tokens` を PostgreSQL の `workflow_executions` テーブルに保存 (JSONB カラムで構造化ログも格納)

## 権限制御

| 設定 | 用途 |
| --- | --- |
| `--permission-mode plan` | デフォルト。読み取りのみ、書き込みは確認必須 |
| `--permission-mode acceptEdits` | 明示的に「自動編集ノード」と宣言された時だけ |
| `--add-dir <path>` | ワークフロー専用 sandbox ディレクトリだけを公開 |
| `--allowed-tools` | ホワイトリスト指定 |
| `--disallowed-tools` | `Bash(rm:*)`, `Bash(curl:*)` 等を恒久遮断 |
| `--permission-prompt-tool` | 危険操作の承認を MCP server に委譲 |

MCP server (`rmcp` クレートで自前実装) は Claude Code から JSON-RPC で承認要求を受け、それを **iPhone や Win トレイの通知としてユーザにプッシュ**、ユーザがタップで承認した結果を JSON-RPC で返します。

## 同時実行制御

- グローバル: `tokio::sync::Semaphore` で同時実行数を制限 (デフォルト = `num_cpus / 2`)
- ワークフロー内並列ノード: `tokio::task::JoinSet` で並列実行
- レート制限・429 検知: exponential backoff (再試行は冪等ノードのみ)

## エラー回復・タイムアウト

| 状況 | 対処 |
| --- | --- |
| ノード wall-clock 超過 | `tokio::time::timeout` (デフォルト 10 分) → Cancel + ノード失敗 |
| Claude プロセス異常終了 | exit code + stderr 監視 → リトライ (1 回) → 失敗マーク |
| コンテキスト爆発 | `result` イベントで `num_turns` / `total_cost_usd` 監視、閾値で打ち切り |
| 中断 | `tokio::process::Child::kill()` でプロセスグループごと kill (Win では `windows` crate、Unix では `nix`) |

## 採用クレート

| 用途 | クレート |
| --- | --- |
| Process management | `tokio` (`tokio::process`) |
| NDJSON | `tokio-util::codec::LinesCodec` |
| JSON | `serde`, `serde_json` |
| MCP server | `rmcp` (公式 Rust SDK) |
| ログ | `tracing`, `tracing-subscriber` |
| プロセス kill (cross-platform) | `nix` (Unix) + `windows` (Win) — feature flag で切替 |

## ファイル構成 (`apps/host-backend/src/ai/`)

```
ai/
├── mod.rs              # 公開API
├── process.rs          # ClaudeProcessHandle (spawn, stream, cancel)
├── session.rs          # SessionId, --resume 管理
├── stream.rs           # stream-json イベント型 + parser
├── permission.rs       # MCP server (rmcp) for permission-prompt-tool
└── pool.rs             # Semaphore + 同時実行制御
```

## 次に詰めること

- Claude Code バージョン依存性のテスト戦略 (CLI 仕様変更追従)
- MCP permission tool のレスポンス UI (iPhone widget / Win トレイ通知) を SDUI で表現できるか
- ローカル LLM (Ollama 等) を「同じインターフェース」で差し替えできる抽象化レイヤの設計
- Claude Code の Subagent 機能をワークフローノードとして表現するか、別概念にするか
