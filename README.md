# IRIS-workflow

> **Input → AI-centric Workflow → Action** をユーザPCホスト上で完結させるパーソナル・オートメーション基盤。

`IRIS-workflow` は、日常のあらゆる動作 (時刻・音声・センサ・Webhook ほか) をトリガにして、AI 中心のワークフローを実行し、その結果を **iPhone のウィジェット / Windows のウィジェット・通知 / IoT デバイスの動作** などに出力する個人向けプラットフォームです。

## コンセプトひと言まとめ

```
何らかの入力 → AI が組み立てるワークフロー → デバイスのウィジェットや動きとして結実
```

- **ホストPC型**: クラウドではなくユーザのメインPC上で Rust 製ローカルデーモン (Axum) がワークフローを実行
- **中央AIは Claude Code**: Anthropic API ではなく `claude` CLI をサブプロセスで駆動し、ユーザの既存認証を活用
- **VPNでデバイス接続**: iPhone / 他PC / IoT は Tailscale 等のメッシュVPNでホストに到達
- **Server-driven UI**: ウィジェットのレイアウトと挙動はホスト側で定義、デバイスはレンダラに徹する

詳細は [`docs/concept/`](./docs/concept/) を参照してください。

## モノレポ構成

| パス | 説明 |
| --- | --- |
| `apps/host-backend/` | Rust + Axum ローカルデーモン (ワークフローエンジン、Claude Code ブリッジ、デバイスゲートウェイ、PostgreSQL 接続) |
| `apps/web-console/` | Next.js 製のワークフロービルダー Web UI |
| `apps/mobile/` | Flutter iOS アプリ (WidgetKit 統合) |
| `apps/desktop/` | Flutter Windows アプリ (タスクトレイ + WinUI ウィジェット) |
| `packages/proto/` | OpenAPI / JSONSchema による共有スキーマ |
| `packages/sdk-ts/` | TypeScript SDK (Next.js から利用) |
| `packages/sdk-dart/` | Dart SDK (Flutter から利用) |
| `packages/sdui-renderer-flutter/` | Server-driven UI の Flutter レンダラ |
| `infra/` | Docker Compose / Tailscale / パッケージング設定 |
| `docs/` | コンセプト文書・図・実装計画書 |
| `scripts/` | 開発用ユーティリティスクリプト |

## クイックスタート

### 前提ツール

| ツール | 推奨バージョン |
| --- | --- |
| Rust toolchain | 1.80+ (stable) |
| Node.js | 20+ |
| pnpm | 9+ |
| Flutter SDK | 3.32+ (Dart 3.5+) |
| Git | 2.40+ |
| Claude Code | 最新 (`claude login` 済み) |
| (任意) Tailscale CLI | 最新 |

### セットアップ

```powershell
# 依存インストール
pnpm install
cargo fetch

# ホスト一式を起動 (Rust backend + Next.js web-console を並行起動)
.\scripts\dev-host.ps1
```

### 実装計画書 (Excel)

```powershell
node scripts/gen-plan.mjs
# → docs/plans/implementation-plan.xlsx が更新される
```

### インフラ図のレンダ

```powershell
npx -p @mermaid-js/mermaid-cli mmdc -i docs/diagrams/infrastructure.mmd -o docs/diagrams/infrastructure.svg
```

## ロードマップ

`docs/plans/implementation-plan.xlsx` に Phase / Task / Risk / Milestone の詳細あり。ハイレベルは [`docs/plans/roadmap.md`](./docs/plans/roadmap.md) を参照。

| Phase | ゴール |
| --- | --- |
| P0 | リポジトリ骨格 + ドキュメント (現在のフェーズ) |
| P1 | Host Backend MVP (Axum + PostgreSQL + 最小API) |
| P2 | Claude Code ブリッジ |
| P3 | Workflow Engine MVP (DAG + YAML DSL) |
| P4 | Web Console MVP |
| P5 | Flutter Mobile MVP (iOS) |
| P6 | VPN + マルチデバイス連携 |
| P7 | Server-driven UI |
| P8 | IoT (MQTT) |
| P9 | パッケージング |

## ライセンス

未定 (内部開発中)。
