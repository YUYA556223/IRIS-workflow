# 05. ワークフロー DSL

## 目的

IRIS-workflow のワークフローを **人間が読みやすく、AI が生成しやすい** YAML DSL として定義します。

## 設計原則

- **DAG ベース**: ノードとエッジで処理の流れを表現
- **宣言的**: 「何をやるか」を書き、実行順序はエンジンが解決
- **AI ネイティブ**: ノード単位で Claude Code エージェントを呼べる
- **変数バインディング**: 前のノードの出力 → 次のノードへ参照渡し

## 最小例

```yaml
id: morning-briefing
name: 朝のブリーフィング
trigger:
  type: cron
  schedule: "30 6 * * 1-5"   # 平日 06:30

nodes:
  - id: fetch_calendar
    type: transform
    using: builtin/calendar
    with:
      provider: google
      range: today

  - id: summarize
    type: ai
    using: claude-code
    with:
      prompt: |
        以下の予定とニュースから、3分で読めるブリーフィングを作って:
        {{ fetch_calendar.output }}
      allowed_tools: [Read, WebFetch]
      session: new

  - id: render_widget
    type: action
    using: builtin/sdui
    with:
      target: iphone-home-widget
      template: briefing-card
      data:
        title: 今日のサマリ
        body: "{{ summarize.output.text }}"
        actions: "{{ summarize.output.actions }}"

edges:
  - from: fetch_calendar
    to: summarize
  - from: summarize
    to: render_widget
```

## ノード種別

| Type | 説明 | 例 |
| --- | --- | --- |
| `trigger` | (暗黙) ワークフロー入口。`trigger:` フィールドで定義 | cron / webhook / voice / fs-watch / mqtt |
| `ai` | Claude Code (またはローカルLLM) を呼び出す | summarize, classify, plan |
| `transform` | データ整形・組み込みAPI呼び出し | calendar / weather / jq / regex |
| `action` | デバイス側 / 外部APIへ出力 | sdui / notify / mqtt / http / slack |
| `branch` | 条件分岐 (将来) | if / switch |
| `parallel` | 明示的並列 (将来) | join 戦略は all / any |

## 変数バインディング

- ノード出力は `{{ <node_id>.output.<path> }}` で参照
- `Mustache`-like テンプレートを採用 (実装は `tinytemplate` か `tera`)
- 型は JSON 互換 (string / number / bool / array / object)

## トリガ種別

```yaml
trigger:
  type: cron
  schedule: "0 9 * * *"

# または
trigger:
  type: webhook
  path: /hooks/zoom-end

# または
trigger:
  type: voice
  hotword: "Hey IRIS"

# または
trigger:
  type: fs-watch
  path: ~/Documents/inbox

# または
trigger:
  type: mqtt
  topic: home/sensor/motion
```

## ストレージ

ワークフロー定義は **ファイルとしてリポジトリ管理可能** にする:

```
~/.iris-workflow/
└── workflows/
    ├── morning-briefing.yaml
    ├── meeting-followup.yaml
    └── deep-focus.yaml
```

(PostgreSQL には実行ログ・実行状態のみ保存し、定義そのものは YAML ファイルがソースオブトゥルース。PostgreSQL の JSONB に実行時パラメータと結果を構造化保存し、LISTEN/NOTIFY で実行イベントを backend 内ストリームに流す)

## 検証

- JSONSchema を `packages/proto/workflow.schema.json` に定義
- ロード時に schema validation
- web-console の Monaco エディタにスキーマを食わせて補完

## 実装状況 (P3 + P3.1〜3.3)

実装済 (`apps/host-backend/src/workflow/` + `src/triggers/` + `src/storage/executions.rs`):

- ✅ YAML / JSON のパース (`Workflow` / `Trigger` / `Node` / `Edge`)
- ✅ Kahn のトポロジカルソート + サイクル検出
- ✅ `{{ <node_id>.path }}` / `{{ trigger.path }}` の文字列・配列・オブジェクト再帰展開
- ✅ ノード種別ディスパッチ:
  - `ai` — `claude-code` (`ClaudeService` 経由)
  - `action` — `builtin/notify`, `builtin/widget-update`, `builtin/sdui-upsert`, `builtin/broadcast-target`
  - `transform` — `builtin/pass-through`, `builtin/now`
- ✅ ファイル直読み (`IRIS_WORKFLOWS_DIR`)
- ✅ REST: `GET/POST /workflows`, `GET/DELETE /workflows/:id`, `POST /workflows/:id/run`
- ✅ **P3.1**: `TriggerHub` で cron (6-7 フィールド) / webhook (`POST /hooks/*path`) / fs-watch (notify crate) 自動起動
- ✅ **P3.2**: `workflow_executions` (Postgres + メモリ) で履歴永続化、`GET /executions(/:id)`, `/workflows/:id/executions`
- ✅ **P3.3**: `JoinSet` による波形並列実行 + 失敗下流伝播 (BFS で `tainted` マーキング)
- ✅ E2E: webhook/cron/fs-watch/並列/履歴/失敗伝播の全 6 観点 (`scripts/test-p3-full.mjs`)

未実装 (Phase 後送り):

- ⏳ 分岐 (`branch`) / 明示的並列 (`parallel`) ノード種別 — 現状は edge 構造のみで暗黙並列
- ⏳ Secret store 参照 / リトライポリシ / サブワークフロー

## 次に詰めること

- 分岐・ループ・サブワークフローの記法
- 認証情報 (API キー等) を YAML に書かず、Secret store から参照する仕組み
- ワークフロー間連携 (あるワークフローの完了が別のトリガになる)
- 失敗時のリトライポリシ記法
