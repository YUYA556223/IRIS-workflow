// IRIS-workflow: Dart 側 WidgetKit ブリッジ (P5.1 scaffold).
//
// MethodChannel `iris.widget/bridge` を介して iOS 側の `IRISWidgetBridge`
// (Swift) を呼び出す。実装本体は iOS にあり、Android / Win / macOS では
// no-op で安全に動作する。

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

class WidgetBridge {
  static const _channel = MethodChannel('iris.widget/bridge');

  /// ホスト画面ウィジェット (WidgetKit) のペイロードを更新する。
  ///
  /// iOS でのみ有効。それ以外のプラットフォームでは no-op で `false` を返す。
  static Future<bool> updateWidget({
    required String title,
    required String body,
    String? updatedAt,
  }) async {
    if (!defaultTargetPlatform.isIos) return false;
    try {
      final ok = await _channel.invokeMethod<bool>('updateWidget', {
        'title': title,
        'body': body,
        if (updatedAt != null) 'updatedAt': updatedAt,
      });
      return ok ?? false;
    } catch (e) {
      debugPrint('WidgetBridge.updateWidget failed: $e');
      return false;
    }
  }

  /// 通知の許可を要求 (初回のみダイアログ)。
  static Future<bool> requestNotificationPermission() async {
    if (!defaultTargetPlatform.isIos) return false;
    try {
      final ok = await _channel.invokeMethod<bool>('requestNotificationPermission');
      return ok ?? false;
    } catch (e) {
      debugPrint('WidgetBridge.requestNotificationPermission failed: $e');
      return false;
    }
  }

  /// APNs device token を取得して文字列で返す (失敗時 null)。
  /// 取得後は `host-backend` の `/devices` に渡して push 配信先に登録する。
  static Future<String?> getDeviceToken() async {
    if (!defaultTargetPlatform.isIos) return null;
    try {
      return await _channel.invokeMethod<String>('getDeviceToken');
    } catch (e) {
      debugPrint('WidgetBridge.getDeviceToken failed: $e');
      return null;
    }
  }
}

extension on TargetPlatform {
  bool get isIos => this == TargetPlatform.iOS;
}
