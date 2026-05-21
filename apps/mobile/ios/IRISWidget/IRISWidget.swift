// IRIS-workflow Widget Extension (P5.1 scaffold).
//
// このターゲットを Xcode で `Target → + → Widget Extension` として追加してから
// 本ファイルを Source として組み込む。
//
// 設計:
//  - Flutter アプリが App Group (`group.jp.memorylab.iris`) の UserDefaults に
//    最新の bindings を書き込む (`IRISWidgetBridge` 経由)
//  - Widget は TimelineProvider でその値を読み取り、レンダリング
//  - 背景更新は: (a) アプリが書き込むたびに `WidgetCenter.shared.reloadAllTimelines()`
//    を呼ぶ、(b) APNs プッシュ (`mutable-content`) で `NotificationServiceExtension`
//    が App Group へ書き込み → Widget reload
//
// SDUI レンダリング (Phase 2): bindings に加え SDUI スキーマ JSON も App Group
// に置き、Widget 側で簡易レンダラ (Text/HStack/VStack のサブセット) を実装する。
// 現状は title/body の2フィールド固定。

import WidgetKit
import SwiftUI

private let appGroupId = "group.jp.memorylab.iris"
private let defaultsKey = "iris.widget.payload"

struct IRISPayload: Codable {
    var title: String
    var body: String
    var updatedAt: String?

    static let placeholder = IRISPayload(
        title: "IRIS",
        body: "ホストと未接続",
        updatedAt: nil
    )

    static func load() -> IRISPayload {
        guard
            let defaults = UserDefaults(suiteName: appGroupId),
            let raw = defaults.string(forKey: defaultsKey),
            let data = raw.data(using: .utf8),
            let payload = try? JSONDecoder().decode(IRISPayload.self, from: data)
        else {
            return .placeholder
        }
        return payload
    }
}

struct IRISEntry: TimelineEntry {
    let date: Date
    let payload: IRISPayload
}

struct IRISProvider: TimelineProvider {
    func placeholder(in context: Context) -> IRISEntry {
        IRISEntry(date: Date(), payload: .placeholder)
    }

    func getSnapshot(in context: Context, completion: @escaping (IRISEntry) -> Void) {
        completion(IRISEntry(date: Date(), payload: IRISPayload.load()))
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<IRISEntry>) -> Void) {
        let now = Date()
        let entry = IRISEntry(date: now, payload: IRISPayload.load())
        // 5分後に自然再描画 (それ以前は reloadAllTimelines で起こされる)
        let next = Calendar.current.date(byAdding: .minute, value: 5, to: now) ?? now
        completion(Timeline(entries: [entry], policy: .after(next)))
    }
}

struct IRISWidgetView: View {
    let entry: IRISEntry

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(entry.payload.title)
                .font(.headline)
                .lineLimit(1)
            Text(entry.payload.body)
                .font(.body)
                .lineLimit(4)
            Spacer()
            if let updated = entry.payload.updatedAt {
                Text(updated)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(12)
        .containerBackground(for: .widget) { Color(.systemBackground) }
    }
}

@main
struct IRISWidget: Widget {
    let kind: String = "IRISWidget"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: IRISProvider()) { entry in
            IRISWidgetView(entry: entry)
        }
        .configurationDisplayName("IRIS-workflow")
        .description("最新のワークフロー出力を表示します。")
        .supportedFamilies([.systemSmall, .systemMedium])
    }
}

#Preview(as: .systemSmall) {
    IRISWidget()
} timeline: {
    IRISEntry(
        date: Date(),
        payload: IRISPayload(title: "今日の予定", body: "10:00 ミーティング", updatedAt: "13:00")
    )
}
