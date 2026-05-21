use chrono::Utc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::ai::PermissionPromptInput;
use crate::domain::{DeliveryTarget, Notification, SduiSpec, Widget, WidgetId};

use super::event::{DeliveryEnvelope, DeliveryEvent};

/// 配信ハブ。
///
/// 内部は単一の `tokio::sync::broadcast` チャネル。API ハンドラが `publish` し、
/// WS セッション側が `subscribe()` で受信。受信側がデバイス ID に応じて
/// `DeliveryTarget::matches` でフィルタする。
///
/// broadcast の容量を超えると古いメッセージから drop されるため、
/// 通知のような一過性データには適している (永続化が必要なものは別途 DB 経由)。
pub struct DeliveryHub {
    tx: broadcast::Sender<DeliveryEnvelope>,
}

impl DeliveryHub {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    /// 新規 receiver を発行する (各 WS セッションがこれを呼ぶ)。
    pub fn subscribe(&self) -> broadcast::Receiver<DeliveryEnvelope> {
        self.tx.subscribe()
    }

    /// 任意の envelope を発行。配信されたサブスクライバ数を返す
    /// (誰も購読していなければ 0 を返し、エラーにはしない)。
    pub fn publish(&self, envelope: DeliveryEnvelope) -> usize {
        match self.tx.send(envelope) {
            Ok(n) => n,
            Err(_) => 0,
        }
    }

    // ---------- 高レベル API ----------

    pub fn dispatch_notification(&self, notification: Notification) -> usize {
        let target = notification.target.clone();
        self.publish(DeliveryEnvelope::new(
            target,
            DeliveryEvent::NotificationDelivered { notification },
        ))
    }

    pub fn publish_widget_created(&self, widget: &Widget) -> usize {
        self.publish(DeliveryEnvelope::new(
            widget.target.clone(),
            DeliveryEvent::WidgetCreated {
                widget: widget.clone(),
            },
        ))
    }

    pub fn publish_widget_updated(&self, widget: &Widget) -> usize {
        self.publish(DeliveryEnvelope::new(
            widget.target.clone(),
            DeliveryEvent::WidgetUpdated {
                widget_id: widget.id,
                bindings: widget.bindings.clone(),
                updated_at: widget.updated_at,
            },
        ))
    }

    pub fn publish_widget_deleted(&self, widget_id: WidgetId, target: DeliveryTarget) -> usize {
        self.publish(DeliveryEnvelope::new(
            target,
            DeliveryEvent::WidgetDeleted { widget_id },
        ))
    }

    pub fn publish_sdui_updated(&self, spec: &SduiSpec) -> usize {
        self.publish(DeliveryEnvelope::new(
            // SDUI 更新は SDUI capability を持つ全デバイスへ
            DeliveryTarget::Capability {
                capability: crate::domain::Capability::Sdui,
            },
            DeliveryEvent::SduiUpdated {
                spec_id: spec.id.clone(),
                spec: spec.clone(),
            },
        ))
    }

    pub fn host_ping(&self) -> usize {
        self.publish(DeliveryEnvelope::new(
            DeliveryTarget::All,
            DeliveryEvent::HostPing { at: Utc::now() },
        ))
    }

    /// MCP 由来の許可要求をデバイスへ配信する。
    /// 受け取ったデバイスは UI を出して `POST /permission/respond` で答える。
    pub fn publish_permission_request(
        &self,
        request_id: Uuid,
        input: &PermissionPromptInput,
    ) -> usize {
        self.publish(DeliveryEnvelope::new(
            // 通知 capability 持ちのデバイスへ送る (= 承認 UI を出せる端末)
            DeliveryTarget::Capability {
                capability: crate::domain::Capability::Notification,
            },
            DeliveryEvent::PermissionRequested {
                request_id,
                tool_name: input.tool_name.clone(),
                tool_input: input.tool_input.clone(),
                session_id: input.session_id.clone(),
            },
        ))
    }
}
