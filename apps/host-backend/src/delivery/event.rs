use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use uuid::Uuid;

use crate::domain::{
    DeliveryTarget, Notification, NotificationId, SduiSpec, SduiSpecId, Widget, WidgetId,
};

/// デバイスへ送られる配信イベント。WebSocket では JSON でシリアライズされる。
///
/// `#[serde(tag = "type")]` で内部タグ付け。`#[serde(flatten)]` を併用して
/// ペイロード型のフィールドをトップレベルに展開している。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DeliveryEvent {
    /// 新規ウィジェットが作成された (デバイスは UI 一覧に追加する)。
    WidgetCreated {
        #[serde(flatten)]
        widget: Widget,
    },
    /// ウィジェットの bindings が更新された。
    WidgetUpdated {
        widget_id: WidgetId,
        bindings: serde_json::Value,
        updated_at: DateTime<Utc>,
    },
    /// ウィジェットが削除された。
    WidgetDeleted { widget_id: WidgetId },
    /// SDUI 仕様が更新/新規追加された (ホットスワップ用)。
    SduiUpdated {
        spec_id: SduiSpecId,
        spec: SduiSpec,
    },
    /// 通知の配信。
    NotificationDelivered {
        #[serde(flatten)]
        notification: Notification,
    },
    /// 通知 ID 単体での取り消し (delivered なものに対しても可)。
    NotificationCancelled { notification_id: NotificationId },
    /// Claude Code から MCP 経由で許可を求められている。
    /// デバイスは UI を出してユーザに尋ね、`POST /permission/respond` を返す。
    PermissionRequested {
        request_id: Uuid,
        tool_name: String,
        tool_input: serde_json::Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
    },
    /// ホストからのハートビート (一定間隔)。デバイス側で疎通確認に使う。
    HostPing { at: DateTime<Utc> },
}

/// 配信先 (target) を添えた配信イベント。
/// `DeliveryHub` の broadcast チャネル上を流れる単位。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryEnvelope {
    pub target: DeliveryTarget,
    pub event: DeliveryEvent,
    pub at: DateTime<Utc>,
}

impl DeliveryEnvelope {
    pub fn new(target: DeliveryTarget, event: DeliveryEvent) -> Self {
        Self {
            target,
            event,
            at: Utc::now(),
        }
    }
}
