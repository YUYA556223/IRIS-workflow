# 07. デバイス・ウィジェット統合

## 目的

iPhone / Windows PC / IoT デバイスを IRIS-workflow にどう接続し、出力 (ウィジェット・通知・動作) をどう実現するかを定義します。

## デバイスレジストリ

すべてのデバイスは `host-backend` に登録される:

```
POST /devices/register
{
  "kind": "ios" | "windows" | "iot-mqtt" | "browser",
  "name": "ysato-iphone",
  "capabilities": ["widget", "notification", "voice-in", "sdui"],
  "auth": { "type": "tailscale-tag" | "shared-secret" | "tls-cert" }
}
```

レジストリは PostgreSQL の `devices` テーブルに保存 (JSONB に capabilities)。Tailscale tag (`tag:iris-device`) を持ち、ACL でホストへの到達のみ許可。

## iOS (Flutter `apps/mobile`)

### 構成

```
apps/mobile/
├── lib/                   # Flutter (Dart)
│   ├── main.dart
│   ├── features/
│   │   ├── workflows/     # 一覧・実行ログ
│   │   ├── devices/       # ペアリング
│   │   └── sdui/          # SDUI レンダラ呼び出し
│   └── services/
│       └── host_client.dart  # Dio + WS
└── ios/
    ├── Runner/            # Flutter ホストアプリ
    └── IRISWidget/        # Swift WidgetKit ターゲット (※追加)
```

### ホーム画面ウィジェット

- **Swift WidgetKit** で実装 (Flutter のみでは不可能)
- Flutter アプリと共有 App Group を使ってデータ (SDUI スキーマ + bindings) を渡す
- バックグラウンド更新: BGAppRefreshTask + Push Notification 起動
- Tailscale が iOS で稼働中であれば、Widget 内から直接ホストにフェッチ可能

### 通知

- APNs 経由でホストから push
- ペイロードに SDUI スキーマ ID を含め、タップで Flutter アプリに deep link

### 音声トリガ

- 初期: Shortcuts.app からのカスタムインテント
- 将来: Siri / Voice Control 統合

## Windows (Flutter `apps/desktop`)

### 構成

```
apps/desktop/
├── lib/                   # Flutter
│   ├── main.dart
│   └── features/
│       ├── tray/          # タスクトレイ常駐
│       ├── workflows/
│       └── sdui/
└── windows/
    ├── runner/            # Flutter ホスト
    └── iris_widget_plugin/  # WinUI 3 ウィジェット (※追加)
```

### タスクトレイ + ポップアップ

- `tray_manager` パッケージで常駐
- ホスト localhost (もしくは Tailscale 経由) と WebSocket 接続
- 通知は `flutter_local_notifications` + Windows Toast

### Windows 11 ウィジェットボード

- WinUI 3 + Microsoft.Windows.Widgets SDK で実装
- パッケージは MSIX で署名・配布が必須 (P9 で対応)
- ホスト → ウィジェット間は名前付きパイプ or HTTP loopback

## IoT デバイス (MQTT)

### プロトコル

```
home/<device>/cmd    # ホスト → デバイス
home/<device>/state  # デバイス → ホスト
home/<device>/event  # デバイス → ホスト (一過性イベント)
```

JSON ペイロード。スキーマは `packages/proto/device.schema.json` で定義。

### サポート想定デバイス (初期)

| デバイス | 接続 |
| --- | --- |
| スマート電球 (Hue, Matter) | bridge 経由 MQTT |
| スマートプラグ (Tapo, Tuya) | ローカル API → ホスト内で MQTT に橋渡し |
| 自作 ESP32 デバイス | 直接 MQTT |
| HomePod / Echo | 音声出力のみ、Shortcuts/Routine 経由 |

### MQTT Broker

- 開発: `infra/docker/docker-compose.yml` の `eclipse-mosquitto`
- 本番想定: ホスト常駐の `rumqttd` (Rust 製ブローカ、host-backend に組み込み可能)

## 通知の統合フロー

```
ワークフロー action: notify
    │
    ▼
Device Gateway
    │
    ├─► iOS: APNs push (FCM 不使用、Apple Push API 直接)
    ├─► Windows: WS push → flutter_local_notifications
    └─► IoT: MQTT topic publish
```

## ペアリングフロー (案)

1. ユーザがデバイスで IRIS アプリを起動
2. アプリが Tailscale 接続を確認 → ホストの `/pair/init` を叩く
3. ホストが 6 桁コードを web-console と デバイスに表示
4. ユーザが web-console でコードを承認 → デバイス登録完了

## 次に詰めること

- App Group の identifier / Bundle ID 命名規則 (`jp.memorylab.iris.*`)
- WinUI 3 ウィジェットの初期実装難易度・配布署名
- Apple Developer 契約 (APNs 利用に必須)
- MQTT 認証 (mTLS or ユーザ名/パスワード)
- 自分用 IoT を実際に1つ通すユースケース選定
