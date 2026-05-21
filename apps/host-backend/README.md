# host-backend

IRIS-workflow ホスト常駐デーモン (Rust + Axum)。

## モジュール構成

```
src/
├── main.rs              # bin: tracing 初期化 → Config → AppState → axum::serve
├── lib.rs               # 全モジュールの公開と build_app(state)
├── config.rs            # Config (環境変数経由のロード)
├── telemetry.rs         # tracing-subscriber 初期化
├── error.rs             # AppError + IntoResponse, AppResult
├── state.rs             # AppState (Arc<dyn Repo>, Arc<DeliveryHub>, Arc<ClaudeService>, Arc<Config>)
│
├── domain/              # ★純粋なドメイン型 (IO 依存なし)
│   ├── device.rs        # Device / DeviceId / DeviceKind / Capability / RegisterDevice
│   ├── widget.rs        # Widget / WidgetId / CreateWidget / UpdateWidgetBindings
│   ├── notification.rs  # Notification / DispatchNotification / Priority
│   ├── sdui.rs          # SduiSpec / SduiSpecId / Component (タグ付き enum)
│   └── target.rs        # DeliveryTarget (All / Device / Kind / Capability)
│
├── storage/             # ★永続化抽象。trait + メモリ実装 + Postgres 実装
│   ├── mod.rs           # StorageError / StorageResult
│   ├── devices.rs       # DeviceRepo trait
│   ├── widgets.rs       # WidgetRepo trait
│   ├── sdui.rs          # SduiRepo trait
│   ├── memory.rs        # Memory{Device|Widget|Sdui}Repo (DashMap)
│   └── postgres.rs      # Pg{Device|Widget|Sdui}Repo (sqlx + JSONB)
│
├── delivery/            # ★配信ハブ。WS 経由で「即届ける」系の中枢
│   ├── event.rs         # DeliveryEvent / DeliveryEnvelope
│   └── hub.rs           # DeliveryHub: broadcast::Sender + dispatch_* / publish_*
│
├── ai/                  # ★Claude Code ブリッジ
│   ├── stream.rs        # StreamEvent (system/assistant/user/result)
│   ├── process.rs       # ClaudeProcessHandle (spawn + NDJSON parse + kill)
│   └── service.rs       # ClaudeService (Semaphore + timeout + run → ClaudeRunResult)
│
├── workflow/            # ★DAG ワークフローエンジン
│   ├── dsl.rs           # Workflow / Trigger / Node / NodeType / Edge
│   ├── dag.rs           # Kahn のトポロジカルソート + サイクル検出
│   ├── template.rs      # {{ node.path }} プレースホルダ展開
│   ├── loader.rs        # ディレクトリから YAML 一括ロード
│   ├── store.rs         # WorkflowStore (DashMap)
│   └── executor.rs      # WorkflowExecutor: 波形 (JoinSet) 並列実行 + 失敗下流伝播
│
├── triggers/            # ★Trigger Hub (cron / fs-watch / webhook ルックアップ)
│   └── mod.rs           # TriggerHub: sync() で WorkflowStore に合わせて再登録
│
└── api/                 # ★HTTP / WebSocket ルータ
    ├── mod.rs           # Router 合成 (CORS + Trace)
    ├── health.rs        # GET  /health
    ├── devices.rs       # GET/POST/DELETE /devices, /devices/:id
    ├── widgets.rs       # GET/POST/DELETE /widgets, /widgets/:id, PATCH .../bindings
    ├── sdui.rs          # GET/POST/DELETE /sdui-specs, /sdui-specs/:id
    ├── notifications.rs # POST /notifications
    ├── ai.rs            # POST /ai/prompt
    ├── workflows.rs     # GET/POST/DELETE /workflows, POST /workflows/:id/run
    ├── executions.rs    # GET /executions, /executions/:id, /workflows/:id/executions
    ├── webhooks.rs      # POST /hooks/*path (動的ルーティング)
    └── ws.rs            # GET /ws (WebSocket upgrade + subscribe)

migrations/
├── 20260521120000_init.sql          # devices / sdui_specs / widgets スキーマ
└── 20260521130000_executions.sql    # workflow_executions (JSONB の nodes + trigger_data)

example-workflows/
├── hello-world.yaml          # AI 出力 → 通知配信
└── widget-flow.yaml          # AI 出力 → ウィジェット bindings 更新
```

### レイヤ依存

```
api  →  state ─┬─►  storage (trait)
               └─►  delivery (concrete) ◄── api (publish)

domain  ←── all layers (no IO, no deps within crate)
```

`storage` は trait 化されているため、テスト用にモック差し替え可能。
`delivery` は `tokio::sync::broadcast` を内部に持つ concrete 構造体で、`Arc` 共有。

## エンドポイント一覧

| Method | Path | 用途 | 配信 |
|---|---|---|---|
| GET | `/health` | ヘルスチェック | — |
| GET | `/devices` | 登録済みデバイス一覧 | — |
| POST | `/devices` | デバイス登録 | — |
| GET | `/devices/:id` | デバイス取得 | — |
| DELETE | `/devices/:id` | デバイス削除 | — |
| GET | `/sdui-specs` | SDUI 仕様一覧 | — |
| POST | `/sdui-specs` | SDUI 仕様 upsert | `SduiUpdated` を SDUI capability 持ちに |
| GET | `/sdui-specs/:id` | SDUI 仕様取得 | — |
| DELETE | `/sdui-specs/:id` | SDUI 仕様削除 | — |
| GET | `/widgets` | ウィジェット一覧 | — |
| POST | `/widgets` | ウィジェット作成 (SDUI 参照整合性チェック) | `WidgetCreated` |
| GET | `/widgets/:id` | ウィジェット取得 | — |
| PATCH | `/widgets/:id/bindings` | bindings 更新 | `WidgetUpdated` |
| DELETE | `/widgets/:id` | ウィジェット削除 | `WidgetDeleted` |
| POST | `/notifications` | 通知ディスパッチ | `NotificationDelivered` |
| POST | `/ai/prompt` | Claude Code 呼び出し (集約) | — |
| POST | `/ai/prompt/stream` | Claude Code 呼び出し (SSE streaming) | — |
| POST | `/permission/request` | MCP bridge → ホスト。ユーザの承認待ち | デバイスへ `PermissionRequested` push |
| POST | `/permission/respond` | デバイス → ホスト。承認/拒否を bridge に伝達 | — |
| GET | `/workflows` | ワークフロー一覧 | — |
| POST | `/workflows` | ワークフロー upsert | — |
| GET | `/workflows/:id` | ワークフロー取得 | — |
| DELETE | `/workflows/:id` | ワークフロー削除 | — |
| POST | `/workflows/:id/run` | ワークフロー実行 (任意 body = trigger data) | ノード内の action が `DeliveryHub` 経由で配信 |
| GET | `/executions` | 実行履歴一覧 (`?workflow_id=` `?limit=` クエリ) | — |
| GET | `/executions/:id` | 実行詳細 (UUID) | — |
| GET | `/workflows/:id/executions` | 特定ワークフローの履歴 (`?limit=`) | — |
| POST | `/hooks/*path` | Webhook 起動 (workflow.trigger.webhook.path に match) | action ノード経由で配信 |
| GET | `/ws` | WebSocket upgrade | (購読側) |

### POST /ai/prompt のリクエスト/レスポンス

リクエスト:

```json
{
  "prompt": "Summarize today's events ...",
  "session_id": null,            // 既存セッションがあれば指定 (省略可)
  "resume_session": false,       // true なら --resume、false なら --session-id (新規)
  "permission_mode": "plan",     // 既定。"acceptEdits" / "bypassPermissions" も可
  "allowed_tools": ["Read", "Grep"],
  "disallowed_tools": ["Bash(rm:*)"],
  "add_dirs": ["C:/Users/.../sandbox"],
  "model": null                  // 省略時 Claude のデフォルト
}
```

レスポンス (`ClaudeRunResult`):

```json
{
  "session_id": "f63457b1-...",
  "result": "最終アシスタントテキスト",
  "total_cost_usd": 0.07539,
  "num_turns": 1,
  "duration_ms": 1915,
  "is_error": false,
  "exit_code": 0,
  "events": [ /* 全ての StreamEvent (system_init, assistant, result, ...) */ ]
}
```

### WebSocket プロトコル

クライアントは `GET /ws` で WS に upgrade した後、最初に hello を送信:

```json
{ "type": "hello", "device_id": "<uuid>" }
```

サーバはこれを受けて `devices` レポジトリでデバイスを検証し、`DeliveryHub.subscribe()`
で broadcast チャネルを購読。以後、配信イベントを JSON で push する。
クライアントから動作イベントを送る場合:

```json
{ "type": "event", "name": "action.invoke", "payload": { "id": "..." } }
```

サーバ→クライアント (例):

```json
{ "type": "widget-updated", "widget_id": "...", "bindings": {...}, "updated_at": "..." }
{ "type": "notification-delivered", "id": "...", "title": "...", "body": "...", "priority": "high", "target": {"type":"all"}, "created_at": "..." }
{ "type": "host-ping", "at": "..." }
```

`#[serde(tag = "type")]` + `#[serde(flatten)]` でペイロードがトップレベルに展開される。

## 設定 (環境変数)

| 変数 | 既定 | 説明 |
|---|---|---|
| `IRIS_BIND` | `127.0.0.1:8787` | バインドアドレス |
| `IRIS_DELIVERY_CAPACITY` | `256` | `DeliveryHub` の broadcast 容量 |
| `DATABASE_URL` | (未設定 = メモリ) | 設定時に Postgres バックエンドを使用。例: `postgres://iris:iris_dev_password@127.0.0.1:5432/iris` |
| `IRIS_AI_CONCURRENCY` | `CPU/2` | `claude` 同時実行数の上限 |
| `IRIS_AI_TIMEOUT_SECS` | `600` | `claude` 呼び出しの wall-clock タイムアウト |
| `IRIS_WORKFLOWS_DIR` | (未設定) | 起動時にこのディレクトリの `*.yaml` を全ロード |
| `IRIS_PERMISSION_TIMEOUT_SECS` | `120` | `/permission/request` の承認待ちタイムアウト |
| `RUST_LOG` | `info,host_backend=debug,tower_http=info` | tracing フィルタ |

## ローカル実行

メモリモード (最速):

```powershell
cargo run -p host-backend
```

Postgres モード (永続化):

```powershell
docker compose -f infra/docker/docker-compose.yml up -d postgres
$env:DATABASE_URL = 'postgres://iris:iris_dev_password@127.0.0.1:5432/iris'
cargo run -p host-backend
```

起動時に `migrations/` の SQL が自動適用される。

```powershell
# 別ウィンドウで動作確認
curl http://127.0.0.1:8787/health
node scripts/test-ws.mjs   # WS スモークテスト

# Claude Code 呼び出し
curl -X POST http://127.0.0.1:8787/ai/prompt `
  -H 'Content-Type: application/json' `
  -d '{"prompt":"Hello, reply briefly.","permission_mode":"plan"}'
```

## ワークフロー実行例

ホスト起動 (Postgres + 例ワークフロー込み):

```powershell
docker compose -f infra/docker/docker-compose.yml up -d postgres
$env:DATABASE_URL = 'postgres://iris:iris_dev_password@127.0.0.1:5432/iris'
$env:IRIS_WORKFLOWS_DIR = 'apps/host-backend/example-workflows'
cargo run -p host-backend
```

別ウィンドウで:

```powershell
# E2E スモークテスト (デバイス登録 → WS subscribe → workflow run → 通知受信)
node scripts/test-workflow.mjs
```

期待出力:

```
workflows: [ 'hello-world', 'widget-flow' ]
device:    <uuid>
running hello-world workflow...
execution status: success
nodes:
  - greet    [ai]     success ({"text":"Hello from IRIS-workflow!", ...})
  - announce [action] success ({"notification_id":"...","receivers":1})
ws recv: notification-delivered → Workflow says
OK: workflow execution propagated to WS notification
```

## ノード種別と組み込みプロバイダ

| `type` | `using` | パラメータ | 出力 |
|---|---|---|---|
| `ai` | `claude-code` (既定) | `prompt`, `permission_mode`, `allowed_tools`, `disallowed_tools`, `session_id`, `resume_session`, `model` | `text`, `session_id`, `cost_usd`, `num_turns`, `duration_ms` |
| `action` | `builtin/notify` | `target`, `title`, `body`, `priority`, `data` | `notification_id`, `receivers` |
| `action` | `builtin/widget-update` | `widget_id`, `bindings` | `widget_id`, `updated_at` |
| `action` | `builtin/sdui-upsert` | (SduiSpec フィールド) | `spec_id` |
| `action` | `builtin/broadcast-target` | `target`, `title`, `body`, `data` | `receivers` |
| `transform` | `builtin/pass-through` | `data` (なければ with 全体) | input をそのまま |
| `transform` | `builtin/now` | — | `iso`, `ts` |

`with` 中の文字列フィールド (および配列・オブジェクトの内部) は `{{ <path> }}` プレースホルダで前のノード出力やトリガデータを参照できる:
- `{{ trigger.<key> }}` — `POST /workflows/:id/run` のリクエスト body
- `{{ <node_id>.<key> }}` — 同ワークフロー内の上流ノード出力

## ワークフロー実行モデル (P3.3 並列版)

`WorkflowExecutor.execute()` は波形 (wave) 実行モデル:

1. `topo_sort` でサイクル検出 + 全ノード列挙
2. 各ノードの in-degree 計算
3. 初期波 = in-degree が 0 のノード集合
4. 波内のノードを `tokio::task::JoinSet` で並列 spawn
5. 各タスクは独自に Arc<ClaudeService>/Arc<DeliveryHub>/Arc<dyn Repo> をクローン
6. 波の全タスク完了を `join_next()` で待つ
7. 完了したノードの後続の in-degree を -1、0 になったノードを次波へ
8. **失敗伝播**: ノード失敗時は下流すべてを BFS で tainted に登録 → 次波に来たら `Skipped`
9. 全波完了後、`NodeExecution` を topo 順に整列して返す (出力決定性)

## トリガモデル (P3.1)

`TriggerHub.sync()` でロード済みワークフロー全体を再スキャンし、登録を再構築:

| `trigger.type` | 実装 |
|---|---|
| `manual`  | 何も登録しない (`POST /workflows/:id/run` のみ起動) |
| `cron`    | `cron::Schedule` (6-7 フィールド: `sec min hr dom mon dow`) で次回時刻計算 → tokio task が sleep+execute |
| `webhook` | path をマップに登録、`POST /hooks/<path>` でルックアップ |
| `fs-watch` | `notify::recommended_watcher` + mpsc。任意のイベントで execute (`kind` / `paths` を trigger_data に詰める) |
| `mqtt`    | `rumqttc` の AsyncClient + broadcast バス。topic フィルタ (`+` `#` 対応) で workflow 起動 |

`sync()` はワークフロー upsert/delete のたびに自動呼び出し。既存タスクは abort し、watcher は drop で停止。

## E2E スモークテスト

```powershell
# Postgres + バックエンド起動
docker compose -f infra/docker/docker-compose.yml up -d postgres
$env:DATABASE_URL = 'postgres://iris:iris_dev_password@127.0.0.1:5432/iris'
$env:IRIS_WORKFLOWS_DIR = 'apps/host-backend/example-workflows'
cargo run -p host-backend

# 別ウィンドウで:
node scripts/test-workflow.mjs   # AI → 通知 → WS の最小フロー
node scripts/test-p3-full.mjs    # webhook / 並列 / 履歴 / 失敗伝播 / cron / fs-watch
```

`test-p3-full.mjs` の検証項目 (全て pass 済み):
1. `POST /hooks/<path>` で webhook ワークフロー起動、`{{ trigger.* }}` 展開
2. 3 ノード並列実行 (started_at スプレッド < 200ms を確認)
3. `GET /executions`, `/executions/:id`, `/workflows/:id/executions` の永続化往復
4. 失敗伝播: 上流失敗時に依存ノードは `Skipped`、独立ノードは並列実行で生存
5. `cron: "*/3 * * * * *"` 登録 → 6.5 秒で 2 回起動
6. `fs-watch` 登録 → ファイル作成で起動 (`{{ trigger.kind }}` = `Modify(Any)`)

`test-p3-4.mjs` (P3.4) と `test-p8.mjs` (P8) も追加。`when`/`retry`/`secrets`/
`sub-workflow`/`mqtt trigger`/`mqtt-publish` の全観点を E2E でカバー済み:

```powershell
$env:IRIS_MQTT_BROKER = 'tcp://127.0.0.1:1883'
$env:IRIS_SECRET_P3_4_DEMO = 'from-env'
docker compose -f infra/docker/docker-compose.yml up -d
# host-backend を起動した状態で:
node scripts/test-p3-4.mjs
node scripts/test-p8.mjs
```

## Claude Code の Permission Prompt との繋ぎ込み

Claude Code は `--permission-prompt-tool mcp__<server>__<tool>` で許可要求を任意 MCP
ツールへ委譲できる。IRIS では:

1. ホストが `apps/iris-mcp-permission` (stdio MCP server) を spawn 設定に含める
2. Claude Code 起動時に `--mcp-config` で参照させ、`--permission-prompt-tool
   mcp__iris-permission__prompt` を渡す
3. Claude が tool を使う直前、MCP bridge が `POST /permission/request` を叩く
4. ホストは `PermissionRegistry.open()` で pending を作り、`DeliveryHub` 経由で
   `PermissionRequested` イベントを Notification capability 持ちデバイスへ push
5. デバイス UI でユーザが承認 → `POST /permission/respond` で behavior を返す
6. ホストは MCP bridge に応答を返し、Claude へ伝播

Claude Code 起動側のサンプル `mcp-config.json`:

```json
{
  "mcpServers": {
    "iris-permission": {
      "command": "iris-mcp-permission",
      "env": { "IRIS_BACKEND_URL": "http://127.0.0.1:8787" }
    }
  }
}
```

## 拡張ポイント (今後の Phase)

| Phase | 追加するもの | 状態 |
|---|---|---|
| P1   | `domain/` `storage::memory` `delivery/` `api/` 骨格 | ✅ |
| P1.5 | `storage::postgres` (`DATABASE_URL` 切替) | ✅ |
| P2   | `ai/` Claude Code subprocess bridge | ✅ |
| P2.1 | Permission API + MCP bridge (`iris-mcp-permission`) | ✅ |
| P2.2 | `POST /ai/prompt/stream` SSE ストリーミング | ✅ |
| P3   | `workflow/` DAG エンジン (manual トリガ) | ✅ |
| P3.1 | `triggers/` Cron / Webhook / FS-watch | ✅ |
| P3.2 | `workflow_executions` 永続化 + 履歴 API | ✅ |
| P3.3 | 並列ノード実行 + 失敗伝播 | ✅ |
| P4   | Next.js web-console (一覧 / 実行 / 履歴 / Live) | ✅ |
| P5   | Flutter Mobile (Workflows / Executions / Live / Settings) | ✅ |
| P3.4 | 条件分岐 (`when`) / リトライ / Secrets / サブワークフロー | ✅ |
| P5.1 | iOS WidgetKit + APNs (Swift + Flutter ブリッジ) | ✅ scaffold (Mac/Xcode 必須) |
| P8   | MQTT (rumqttc) — `trigger: mqtt` + `builtin/mqtt-publish` | ✅ |
| P9   | Windows MSIX (Flutter desktop) | ✅ config (証明書は別途) |
| —    | host-backend 側 APNs HTTP/2 配信実装 / Store 提出 / Observability | TODO |

## テスト

```powershell
cargo test -p host-backend
```

unit test は `workflow::dag` (topo / cycle 検出) と `workflow::template` (プレースホルダ展開) に同梱。
統合テストは上記 `scripts/test-p3-full.mjs` を参照。
