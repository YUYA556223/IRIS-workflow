// IRIS-workflow: Flutter ↔ WidgetKit ブリッジ (P5.1 scaffold).
//
// 役割:
//  1. Flutter から `iris.widget/bridge` MethodChannel 経由で payload を受け取り、
//     App Group の UserDefaults に書き込んで `WidgetCenter.reloadAllTimelines()`
//     を呼ぶ。
//  2. APNs device token を取得して同 channel 経由で Flutter に返す
//     (Flutter からホスト host-backend へ送信し APNs push の宛先に使う)。
//
// AppDelegate から `IRISWidgetBridge.attach(controller:)` を呼ぶこと。

import Foundation
import Flutter
import UserNotifications
import UIKit
import WidgetKit

private let appGroupId = "group.jp.memorylab.iris"
private let defaultsKey = "iris.widget.payload"
private let channelName = "iris.widget/bridge"

@objc public final class IRISWidgetBridge: NSObject {
    public static let shared = IRISWidgetBridge()

    private var channel: FlutterMethodChannel?
    private var pendingTokenResult: FlutterResult?

    public func attach(controller: FlutterViewController) {
        let channel = FlutterMethodChannel(
            name: channelName,
            binaryMessenger: controller.binaryMessenger
        )
        channel.setMethodCallHandler { [weak self] call, result in
            self?.handle(call: call, result: result)
        }
        self.channel = channel
    }

    private func handle(call: FlutterMethodCall, result: @escaping FlutterResult) {
        switch call.method {
        case "updateWidget":
            guard let args = call.arguments as? [String: Any] else {
                result(FlutterError(code: "bad_args", message: "expected map", details: nil))
                return
            }
            updateWidget(payload: args)
            result(true)

        case "requestNotificationPermission":
            requestNotificationPermission(result: result)

        case "getDeviceToken":
            // 既に取得済なら即返却、未取得なら APNs 登録を促す
            pendingTokenResult = result
            DispatchQueue.main.async {
                UIApplication.shared.registerForRemoteNotifications()
            }

        default:
            result(FlutterMethodNotImplemented)
        }
    }

    private func updateWidget(payload: [String: Any]) {
        guard let defaults = UserDefaults(suiteName: appGroupId) else {
            NSLog("IRIS: app group %@ not configured", appGroupId)
            return
        }
        if let data = try? JSONSerialization.data(withJSONObject: payload, options: []),
           let s = String(data: data, encoding: .utf8) {
            defaults.set(s, forKey: defaultsKey)
            WidgetCenter.shared.reloadAllTimelines()
        }
    }

    private func requestNotificationPermission(result: @escaping FlutterResult) {
        let center = UNUserNotificationCenter.current()
        center.requestAuthorization(options: [.alert, .sound, .badge]) { granted, _ in
            DispatchQueue.main.async { result(granted) }
        }
    }

    /// AppDelegate.application(_:didRegisterForRemoteNotificationsWithDeviceToken:) からの委譲。
    @objc public func didRegisterDeviceToken(_ deviceToken: Data) {
        let token = deviceToken.map { String(format: "%02x", $0) }.joined()
        pendingTokenResult?(token)
        pendingTokenResult = nil
        channel?.invokeMethod("onDeviceToken", arguments: token)
    }

    /// AppDelegate.application(_:didFailToRegisterForRemoteNotificationsWithError:) からの委譲。
    @objc public func didFailToRegisterForRemoteNotifications(error: Error) {
        pendingTokenResult?(FlutterError(
            code: "apns_register_failed",
            message: error.localizedDescription,
            details: nil
        ))
        pendingTokenResult = nil
    }
}
