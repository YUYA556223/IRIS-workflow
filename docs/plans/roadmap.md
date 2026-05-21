# Roadmap (人間向けサマリ)

詳細は [`implementation-plan.xlsx`](./implementation-plan.xlsx) を参照してください (生成: `node scripts/gen-plan.mjs`)。

## フェーズ一覧

| Phase | ゴール | 完了基準 |
| --- | --- | --- |
| **P0 — Bootstrap** | リポジトリ骨格 + ドキュメント + 計画書 | 構造確認、`cargo check` / `pnpm install` 成功 |
| **P1 — Host Backend MVP** | Axum 起動 + PostgreSQL + 最小 API (`/health`, `/workflows`) | curl で API が応答する |
| **P2 — Claude Code Bridge** | `ClaudeProcessHandle` で対話成立 | 単発プロンプト → 応答ストリーム取得 |
| **P3 — Workflow Engine MVP** | YAML 1本 (cron → ai → notify) が End-to-End 動作 | 朝のブリーフィングが iPhone 通知に届く |
| **P4 — Web Console MVP** | ワークフロー一覧 + 実行ログ閲覧 | ブラウザでワークフロー操作可能 |
| **P5 — Flutter Mobile MVP** | iOS アプリ + 静的ウィジェット | 実機で通知受信、ホーム画面に表示 |
| **P6 — VPN + Multi-device** | Tailscale 越しにマルチデバイス接続 | iPhone 4G 経由でホストに到達 |
| **P7 — Server-driven UI** | サーバが生成した UI をデバイスがレンダ | ワークフロー結果に応じて Widget が動的更新 |
| **P8 — IoT (MQTT)** | MQTT デバイス制御 | スマート照明をワークフローから制御 |
| **P9 — Packaging** | Windows MSIX + Flutter リリースビルド | インストーラーから一式セットアップ可能 |

## マイルストーン

- **M1 (P3 完了)**: 「自分の中で1本だけ毎日動くワークフロー」が実現
- **M2 (P6 完了)**: 「iPhoneのウィジェットが自分のホストPCから直接更新される」が実現
- **M3 (P9 完了)**: 「家族や知人に渡せる」レベルのパッケージ
