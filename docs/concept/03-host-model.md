# 03. ホストPC型 + VPN モデル

## 目的

IRIS-workflow が「クラウドではなくユーザのPCをホストとする」設計を採用した理由と、その帰結を整理します。

## なぜホスト型か

| 理由 | 説明 |
| --- | --- |
| **Claude Code 活用** | Anthropic API ではなくユーザ既存の `claude login` セッションを使いたい。CLI を subprocess で叩くにはローカル実行が前提 |
| **プライバシ** | ワークフロー履歴・トリガデータ・ファイルがすべて自分のPCにとどまる |
| **コスト** | クラウドホスティングのランニングコストゼロ |
| **レイテンシ** | デバイスから VPN 経由で最短経路、外部往復なし |
| **拡張性** | ローカルファイルシステム・既存PC環境 (Docker, ローカルLLM, etc.) と素直に統合できる |

トレードオフ:

- **可用性**: ホストPCがオフラインだとシステム全停止 → 起床時のトリガ等に影響
- **モバイル化**: 出先で動かしたい場合、PCがスリープしていれば動かない (Wake-on-LAN, スマートプラグ等の併用案あり)
- **デバイス数の上限**: 個人用途では問題ないが、家族 4-5 名で 20 デバイス級は要検証

→ MVP では「自分1人 + 数台のデバイス」のスコープに絞る。

## VPN (Tailscale を第一候補)

### なぜ Tailscale か

- WireGuard ベースで NAT 越え自動
- mDNS 不要、`hostname.tailnet.ts.net` で名前解決
- 認証 (Google, Microsoft, etc.) とユーザ管理がアカウントレベル
- ACL を JSON で記述可能 → コードで管理できる
- 無料枠が個人用途には十分

### 代替: WireGuard 自前

完全自己ホストしたい場合の代替。設定 YAML をリポジトリにコミットできる利点があるが、運用負荷大。

### tailnet 設計案

```
tagOwners:
  tag:iris-host:    [autogroup:admin]
  tag:iris-device:  [autogroup:admin]

acls:
  - action: accept
    src:   [tag:iris-device]
    dst:   [tag:iris-host:3000, tag:iris-host:8080]  # web-console + backend
  - action: accept
    src:   [tag:iris-host]
    dst:   [tag:iris-device:*]                       # ホスト→デバイス push
```

詳細は [`infra/tailscale/README.md`](../../infra/tailscale/README.md)。

## オフライン挙動

- ホストPC稼働中・VPN 切断: デバイスは古いキャッシュを表示し続け、再接続で同期
- ホストPC オフライン: トリガは記録されず、デバイス UI は最後の表示で固定
- ローカルだけで動作するワークフロー (PC内完結) は VPN 不要

## 次に詰めること

- macOS をホストにする日が来るか (Windows 専用設計を避けてOS抽象化レイヤを薄く持つ)
- ホストPCを「常時起動」に保つ運用ノウハウ (BIOS Wake-on-LAN, Windowsの自動起動)
- 「家族で1つのホスト」モデルの認可設計 (ユーザ毎ワークフロー / 共有ワークフロー)
- ホストPC変更時のマイグレーション (`pg_dump` ベースのバックアップ + YAML をどう移すか)
