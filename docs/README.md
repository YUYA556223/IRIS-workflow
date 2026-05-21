# docs/

IRIS-workflow のドキュメント置き場です。詳細仕様より **方向性・概念** を優先しています。

## 構成

```
docs/
├── concept/      # コンセプト文書 (00 が入口)
├── diagrams/     # Mermaid 図のソース
└── plans/        # 実装計画 (Excel と人間向け Markdown)
```

## 読む順番 (おすすめ)

1. [`concept/00-overview.md`](./concept/00-overview.md) — 1ページで全体像
2. [`concept/01-vision.md`](./concept/01-vision.md) — なぜ作るか / ユースケース
3. [`concept/02-architecture.md`](./concept/02-architecture.md) — 全体アーキテクチャ
4. [`concept/03-host-model.md`](./concept/03-host-model.md) — ホストPC型 + VPN
5. [`concept/04-claude-code-bridge.md`](./concept/04-claude-code-bridge.md) — AIエンジン (Claude Code) 連携
6. [`concept/05-workflow-dsl.md`](./concept/05-workflow-dsl.md) — ワークフローDSL
7. [`concept/06-server-driven-ui.md`](./concept/06-server-driven-ui.md) — Server-driven UI
8. [`concept/07-devices-and-widgets.md`](./concept/07-devices-and-widgets.md) — デバイス・ウィジェット統合

実装計画は [`plans/roadmap.md`](./plans/roadmap.md) (人間向け要約) と [`plans/implementation-plan.xlsx`](./plans/implementation-plan.xlsx) (詳細) に分割しています。
