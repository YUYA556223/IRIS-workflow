# web-console

IRIS-workflow の管理ダッシュボード (Next.js 16 + Tailwind v4 + TanStack Query)。

## ページ

| ルート | 機能 |
|---|---|
| `/` | ホスト稼働状況 + 最新実行履歴サマリ |
| `/workflows` | ワークフロー一覧 + 手動実行 |
| `/workflows/[id]` | ワークフロー定義 + 実行履歴 |
| `/executions` | 実行履歴一覧 |
| `/executions/[id]` | 実行詳細 (ノード別 status + output + error) |
| `/devices` | 登録デバイス一覧 + capabilities |
| `/live` | WebSocket 経由のリアルタイムイベントフィード |

## 開発

```powershell
docker compose -f infra/docker/docker-compose.yml up -d postgres
$env:DATABASE_URL = 'postgres://iris:iris_dev_password@127.0.0.1:5432/iris'
$env:IRIS_WORKFLOWS_DIR = 'apps/host-backend/example-workflows'
cargo run -p host-backend                # 別ウィンドウ
pnpm --filter web-console dev            # http://localhost:3000
```

`/live` ページはこのタブ自身を `browser` デバイスとして登録し、WebSocket で配信
イベントを受信する。複数タブを開けば複数の browser デバイスが host-backend に
登録される。

## 設定

| 変数 | 既定 | 説明 |
|---|---|---|
| `NEXT_PUBLIC_IRIS_BASE_URL` | `http://127.0.0.1:8787` | host-backend の base URL |

## ビルド

```powershell
pnpm --filter web-console build
pnpm --filter web-console start
```

## 依存

- `@iris/sdk-ts` (workspace) — REST/WS クライアントと型
- `@tanstack/react-query` — データフェッチングと SWR キャッシュ
