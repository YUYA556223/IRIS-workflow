use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::target::DeliveryTarget;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct NotificationId(pub Uuid);

impl NotificationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for NotificationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationPriority {
    Low,
    #[default]
    Normal,
    High,
    /// バナー表示 + 音 + 振動など、最大のアテンションを要する通知。
    Critical,
}

/// 配信される通知 (1回限り、ephemeral)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: NotificationId,
    pub target: DeliveryTarget,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub priority: NotificationPriority,
    /// 任意の追加データ (deep-link, action ID 等)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// 通知ディスパッチリクエスト (`POST /notifications`)。
#[derive(Debug, Clone, Deserialize)]
pub struct DispatchNotification {
    pub target: DeliveryTarget,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub priority: NotificationPriority,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

impl DispatchNotification {
    pub fn into_notification(self) -> Notification {
        Notification {
            id: NotificationId::new(),
            target: self.target,
            title: self.title,
            body: self.body,
            priority: self.priority,
            data: self.data,
            created_at: Utc::now(),
        }
    }
}
