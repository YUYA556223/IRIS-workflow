# 02. 全体アーキテクチャ

## 目的

IRIS-workflow のコンポーネント構成と、コンポーネント間のデータフローを定義します。

## 全体図

詳細は [`docs/diagrams/infrastructure.mmd`](../diagrams/infrastructure.mmd) を参照 (Mermaid)。要点は以下:

```
[User Devices]                 [VPN]                  [Host PC]
  iPhone     ── Tailscale ───►  ┃  ───► host-backend (Rust/Axum)
  Win PC      ───────────────►  ┃        ├─ Workflow Engine (DAG)
  IoT (MQTT)  ───────────────►  ┃        ├─ Claude Code Bridge ──► claude CLI (subprocess)
                                ┃        ├─ Device Gateway (MQTT, WS)
                                ┃        ├─ SDUI Service
                                ┃        ├─ Trigger Hub (cron, webhook, voice)
                                ┃        └─ Storage (PostgreSQL + YAML on disk)
                                ┃
                                ┗──► web-console (Next.js, served by host) ──► host-backend
```

## コアコンポーネント

### Host Backend (`apps/host-backend`)

Rust + Axum で書かれた **ローカルデーモン**。すべての中核ロジックがここに集約されます。
実装の詳細・ファイル構造は [`apps/host-backend/README.md`](../../apps/host-backend/README.md) を参照。

| サブシステム | 状態 | 役割 |
| --- | --- | --- |
| `domain/` | P1 ✅ | 純粋なドメイン型 (Device / Widget / Notification / SDUI / DeliveryTarget)。IO 依存なし |
| `storage/` | P1 ✅ memory / P1.5 ✅ postgres | DeviceRepo / WidgetRepo / SduiRepo の trait。メモリ実装は DashMap、Postgres 実装は sqlx + JSONB。`DATABASE_URL` で切替 |
| `delivery/` | P1 ✅ | `DeliveryHub`: `tokio::sync::broadcast` を中核に、通知・ウィジェット更新・SDUI 更新をデバイスへ push |
| `api/` | P1 ✅ | REST + WebSocket エンドポイント。CORS + Tracing middleware |
| `ai/` | P2 ✅ | Claude Code subprocess を管理 (stream-json NDJSON)。`ClaudeService` が Semaphore + timeout でラップ |
| `workflow/` | P3 ✅ + P3.3 ✅ | DAG エンジン。YAML ロード + ai/action/transform ノード実行 + テンプレート展開 + `JoinSet` 並列実行 + 失敗下流伝播 |
| 実行履歴 | P3.2 ✅ | `workflow_executions` (Postgres + メモリ)、`/executions` API |
| `triggers/` | P3.1 ✅ | cron / webhook (`POST /hooks/*path`) / fs-watch (notify) — `TriggerHub.sync()` で再構築 |
| MCP permission | P2.1 | `rmcp` で permission-prompt-tool 受け、デバイスへ承認 push |
| `devices/` (MQTT) | P8 | IoT 接続用 MQTT クライアント (rumqttd 統合) |

なお Server-driven UI は専用モジュールを切らず、`domain::sdui` (型) + `delivery` (配信) + `api::sdui` (REST) の3点に分散させている (P7 で必要があれば `sdui/` モジュールを切る)。

### Web Console (`apps/web-console`)

Next.js (App Router) 製の管理 UI。

- ワークフロー一覧 / 実行ログ閲覧
- DSL エディタ (Monaco) と GUI ビジュアル編集
- デバイスペアリングフロー
- shadcn/ui ベースのコンポーネント

ローカルではホスト同居 (同じPCで起動)。リモート (他PC) からは Tailscale 経由で `http://<host>.tailnet:3000` にアクセス。

### Flutter Mobile / Desktop (`apps/mobile`, `apps/desktop`)

- 共通: Riverpod + Dio で host-backend と通信
- iOS: Swift WidgetKit ターゲットを Flutter プロジェクトに同梱。Platform Channel でデータ受渡し
- Windows: タスクトレイ常駐 + WinUI 3 ウィジェットプラグイン
- SDUI レンダラを `packages/sdui-renderer-flutter` から取り込む

### 共有パッケージ

| パッケージ | 内容 |
| --- | --- |
| `packages/proto` | JSONSchema / OpenAPI / Protobuf 共有定義 |
| `packages/sdk-ts` | TypeScript SDK (Next.js 側から host-backend を叩く) |
| `packages/sdk-dart` | Dart SDK (Flutter 側から host-backend を叩く) |
| `packages/sdui-renderer-flutter` | SDUI スキーマを Flutter widget に展開するレンダラ |

## データフロー (ワークフロー1本の実行)

```
1. Trigger Hub がトリガを検知 (cron, webhook, etc.)
   └─► WorkflowExecution レコード作成 (storage)
2. Workflow Engine が DAG をロードし、ノードを順次実行
   ├─ ai ノード → Claude Code Bridge → claude CLI subprocess
   │       └─ stream-json をパースして結果を変数バインディング
   ├─ transform ノード → 変数の整形 (Rhai? jq?)
   └─ action ノード → Device Gateway → MQTT/WS/HTTP
3. SDUI Service が必要なウィジェット更新をブロードキャスト
4. デバイス側 (Flutter) が更新を受け取り、レンダリング
5. 実行ログを storage に追記、web-console で閲覧可能
```

## トラスト境界

- **ホスト内部** (信頼): backend ↔ Claude Code ↔ PostgreSQL
- **VPN 内部** (信頼、ただし認証必須): host ↔ デバイス (Tailscale ACL で制御)
- **VPN 外部** (非信頼): 外部 API、Webhook 提供元

特に Claude Code は許可されたディレクトリと許可ツールのみ触れるよう、`--permission-mode plan` をデフォルトとし、書き込み系は明示的に承認を求める設計とする。詳細は [`04-claude-code-bridge.md`](./04-claude-code-bridge.md)。

## 次に詰めること

- WebSocket vs Server-Sent Events vs gRPC streaming のどれを SDUI 配信に使うか
- ワークフロー定義 YAML の Git 同期 (バージョン管理ストラテジ)
- 認証 (デバイス → host) の方式: TLS client cert / shared secret / OIDC
