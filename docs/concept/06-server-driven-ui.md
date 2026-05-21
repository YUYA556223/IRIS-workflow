# 06. Server-driven UI (SDUI)

## 目的

ウィジェットや通知の **見た目と挙動をサーバ (ホスト) が定義し**、デバイスはレンダリングと入力イベントの送信に徹するモデルを定義します。

## なぜ SDUI か

- ワークフローを変更すれば、再デプロイなしに iPhone/Win のウィジェットが即更新される
- Claude Code が動的にウィジェット仕様を生成できる (AI 駆動 UI)
- 1 つの仕様から iOS / Windows / Web へ展開可能 (レンダラ次第)

## スキーマ概念

```jsonc
{
  "id": "briefing-card-v1",
  "type": "ComponentTree",
  "root": {
    "type": "VStack",
    "spacing": 8,
    "children": [
      { "type": "Text", "value": "{{ title }}", "style": "title" },
      { "type": "Text", "value": "{{ body }}", "style": "body" },
      {
        "type": "HStack",
        "spacing": 4,
        "children": [
          {
            "type": "Button",
            "label": "{{ actions[0].label }}",
            "onTap": { "type": "Event", "name": "action.invoke", "payload": { "id": "{{ actions[0].id }}" } }
          }
        ]
      }
    ]
  },
  "bindings": {
    "title": "string",
    "body": "string",
    "actions": "array<{ id: string, label: string }>"
  }
}
```

## コアコンポーネント (最小セット)

- `VStack` / `HStack` / `ZStack`
- `Text`
- `Image`
- `Button`
- `Spacer`
- `Divider`
- `List`
- `Toggle`
- `ProgressBar`

将来追加: `Chart`, `Map`, `Animation`, `Video`.

## イベント

```jsonc
{ "type": "Event", "name": "action.invoke", "payload": { ... } }
```

デバイス側ではタップ・トグル変更等を host-backend に送信し、host 側でハンドラ (= 別のワークフローもしくは特定 action) を起動。

## 配信プロトコル

- 初回ロード: `GET /sdui/<id>` (REST)
- リアルタイム更新: WebSocket subscription (`/ws/sdui` で `subscribe <id>` → サーバから patch を push)
- パッチ形式: JSON Patch (RFC 6902) または全置換 (容量小さい場合)

## レンダラの責務分担

| プラットフォーム | レンダラ | 備考 |
| --- | --- | --- |
| Flutter (iOS app / Win app 本体) | `packages/sdui-renderer-flutter` | フル機能 |
| iOS WidgetKit (ホーム画面) | Swift で SDUI サブセットを描画 | WidgetKit の制約 (StaticConfiguration, Timeline) に従う |
| Win WinUI 3 widget | C# / WinRT で SDUI サブセットを描画 | XAML へマッピング |
| Next.js web-console (プレビュー用) | React コンポーネント | 開発時にウィジェットの見た目を確認 |

ホーム画面ウィジェット (iOS/Windows) は OS の制約上、頻繁更新やインタラクションが限定されるため、SDUI のうち **静的レンダ可能なサブセット** だけを使う。

## アクセシビリティ

- すべての Text/Button に `a11y_label` フィールドを許可
- カラースキームは tokens で抽象化し、ダーク/ライトモード対応

## 次に詰めること

- スキーマのバージョニング (互換性破壊時の戦略)
- レンダラ拡張性 (カスタムコンポーネント登録の仕組み)
- AI が生成したスキーマの検証 (生成 → JSONSchema check → ホットスワップ)
- 既存 SDUI 実装 (Server-Driven UI from Airbnb / Lottie 等) の調査
