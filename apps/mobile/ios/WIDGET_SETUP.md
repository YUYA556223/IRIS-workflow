# iOS WidgetKit / APNs セットアップ (P5.1)

iOS 実機ビルドに必要な手順をまとめる。Mac + Xcode + Apple Developer
アカウントが必須。Windows からはここまでの **コードのみ** が用意済み。

## 1. App Group の作成 (Apple Developer 管理画面)

1. [developer.apple.com](https://developer.apple.com/) → Certificates, Identifiers & Profiles → Identifiers → App Groups
2. `+` ボタンで新規作成、ID は **`group.jp.memorylab.iris`** とする
3. Runner と IRISWidget 両方の App ID にこの App Group を有効化

## 2. APNs 認証キー (.p8) の作成

1. Keys → `+` → Apple Push Notifications service (APNs) を有効化して作成
2. ダウンロードした `AuthKey_XXXXXXXXXX.p8` を host-backend が読み込める場所に保存
3. host-backend 側に APNs 配信用の実装を追加するときに使う (P5.1 後続作業)

## 3. Xcode プロジェクトに Widget Extension を追加

1. `apps/mobile/ios/Runner.xcworkspace` を Xcode で開く
2. Project navigator → Runner → Targets → `+` → iOS App → **Widget Extension**
   - Product Name: `IRISWidget`
   - Bundle Identifier: `jp.memorylab.iris.IRISWidget`
   - Activate Scheme: チェックなし
3. 生成された `IRISWidget/IRISWidget.swift` を削除し、本リポジトリの
   `apps/mobile/ios/IRISWidget/IRISWidget.swift` を **drag-in** で置き換える
4. 同フォルダの `Info.plist` と `IRISWidget.entitlements` も組み込む
5. Target `IRISWidget` → Signing & Capabilities:
   - App Groups → `group.jp.memorylab.iris` をチェック

## 4. Runner ターゲットの設定

1. Target `Runner` → Signing & Capabilities:
   - App Groups → `group.jp.memorylab.iris` をチェック
   - Push Notifications を追加
   - Background Modes → Remote notifications を有効化
2. Build Phases → Compile Sources に
   `IRISWidgetBridge.swift` が含まれていることを確認
3. Entitlements ファイルが `Runner/Runner.entitlements` を指していること

## 5. Flutter 側

- 通知許可要求とトークン取得は `lib/ios_widget/widget_bridge.dart` を介する
- 起動直後に `WidgetBridge.requestNotificationPermission()` → `getDeviceToken()`
- 取得したトークンを host-backend に `POST /devices` (kind=ios) で登録する
  (現状 host-backend 側に APNs 実配信ロジックは未実装。P5.1 後続)

## 6. ホーム画面ウィジェットの動作確認

1. アプリを実機で起動 (Tailscale 経由で host-backend に接続できること)
2. ワークフロー実行 → Flutter 側 `WidgetBridge.updateWidget()` を呼ぶ
3. ホーム画面の「+」→ IRIS-workflow ウィジェットを追加
4. App Group に書き込まれた payload (title / body / updatedAt) が表示される

## 7. APNs 配信 (host-backend 側、TODO)

現状 host-backend には APNs HTTP/2 配信実装が無い。実装する場合:

1. `apns2` クレートまたは `reqwest` で直接 HTTP/2 を叩く
2. 認証は p8 鍵 + JWT (kid = Key ID, iss = Team ID)
3. `/devices/:id` に登録された iOS デバイスの token に対し、`alert + sound +
   mutable-content` payload を送る
4. デバイスは notification を受けて `WidgetBridge` 経由で App Group を更新

## 既知の制約

- iOS Simulator では APNs が動かない (実機必須)
- WidgetKit Preview は Xcode 上でしか動かない
- Widget extension の Bundle ID は親 (Runner) の prefix を継承すること
