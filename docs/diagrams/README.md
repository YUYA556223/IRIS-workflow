# docs/diagrams/

draw.io (diagrams.net) 形式のアーキテクチャ図を管理します。

## ファイル一覧

| ファイル | 内容 |
| --- | --- |
| `infrastructure.drawio` | 全体のインフラ図 (ユーザデバイス ↔ VPN ↔ ホストPC 内サービス) |
| `workflow-execution.drawio` | ワークフロー実行のシーケンス (lane 形式) |

## 編集方法

### A. draw.io デスクトップアプリ (推奨)

[draw.io](https://www.drawio.com/) をインストールし、`.drawio` ファイルを開いて編集 → 保存。

### B. VS Code 拡張

VS Code に [`Draw.io Integration`](https://marketplace.visualstudio.com/items?itemName=hediet.vscode-drawio) 拡張をインストールすると、`.drawio` ファイルを直接編集できます (XML エディタとビジュアルエディタを切替可能)。

### C. ブラウザ

[app.diagrams.net](https://app.diagrams.net/) で開いて編集。「File > Open from > Device」で `.drawio` を選択。

## SVG / PNG への書き出し (drawio CLI)

```powershell
# CLI インストール
pnpm add -g @hediet/drawio-desktop-cli  # もしくは drawio desktop の同梱 CLI を使う

# SVG 書き出し
drawio -x -f svg -o docs/diagrams/infrastructure.svg docs/diagrams/infrastructure.drawio
drawio -x -f svg -o docs/diagrams/workflow-execution.svg docs/diagrams/workflow-execution.drawio

# PNG 書き出し (300dpi)
drawio -x -f png --scale 2 -o docs/diagrams/infrastructure.png docs/diagrams/infrastructure.drawio
```

または draw.io アプリの「File > Export As > SVG / PNG」から GUI 操作でも可。

## アイコン・シェイプの参考

このリポジトリの `.drawio` で使用しているシェイプ:

- `mxgraph.mockup.containers.iPhone` — iPhone モックアップ
- `mxgraph.networking.workstation` — Windows PC タワー
- `mxgraph.networking.server` — サーバラックアイコン
- `mxgraph.bootstrap.window` — Web ブラウザウィンドウ
- `cylinder3` — PostgreSQL データベース (組み込み)
- `cloud` — Tailscale VPN メッシュ (組み込み)
- `hexagon` — MQTT broker (組み込み)
- `note` — YAML ワークフロー定義ファイル (組み込み)

他に使えるアイコンライブラリ:

- **Material Design Icons** (`Shapes > Software > Material Design`)
- **Font Awesome** (`Shapes > Misc > Font Awesome`)
- **AWS / Azure / GCP** (各種クラウド) — `Shapes > Networking > AWS17` 等
- **Cisco** — `Shapes > Networking > Cisco 19` 等

drawio アプリの左下「More Shapes」から追加ライブラリを有効化できます。
