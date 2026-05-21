# iris_mobile (Flutter)

IRIS-workflow iOS / Flutter アプリ。Riverpod + Dio + web_socket_channel。

## 画面

| Tab | 機能 |
|---|---|
| Workflows | 一覧 + 手動 Run |
| Executions | 履歴 + 詳細ボトムシート (ノード別 status / output / error) |
| Live | WS で push されるイベントログ |
| Settings | host URL 設定 / デバイス再登録 |

起動時に `iris-mobile` デバイスを自動登録し、device_id を `shared_preferences`
に保存。WS で `Capability::Notification` 持ちとして配信を受ける。

## 開発

```powershell
# Postgres + host-backend を起動した状態で:
cd apps/mobile
flutter pub get
flutter run -d windows    # まずは Windows desktop で動作確認
# 実機 iOS の場合は Xcode + Apple Developer アカウントが必要 (P5.1)
```

シミュレータでない実機 iOS から接続する場合は、Settings 画面で host URL を
Tailscale 経由のものに変更する (`http://<hostname>.tailnet.ts.net:8787`)。

## 既知の TODO (P5.1)

- iOS Push Notifications (APNs) 統合 — 現状はアプリ起動中のみ WS 経由で受信
- WidgetKit (ホーム画面ウィジェット) — `packages/sdui-renderer-flutter` 経由
- iOS Shortcut カスタムインテント (Siri トリガ)
