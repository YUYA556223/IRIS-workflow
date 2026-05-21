use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// デバイス識別子 (UUID v4)。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct DeviceId(pub Uuid);

impl DeviceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for DeviceId {
    fn default() -> Self {
        Self::new()
    }
}

/// デバイス種別。Postgres には TEXT として格納される (sqlx::Type)。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, sqlx::Type)]
#[serde(rename_all = "kebab-case")]
#[sqlx(type_name = "TEXT", rename_all = "kebab-case")]
pub enum DeviceKind {
    Ios,
    Windows,
    IotMqtt,
    Browser,
}

/// デバイスが提供する機能フラグ。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    /// ホーム画面やトレイへのウィジェット表示が可能。
    Widget,
    /// 通知 (toast / push) を表示できる。
    Notification,
    /// 音声入力をホストへ送れる。
    VoiceIn,
    /// Server-driven UI レンダラを持つ。
    Sdui,
    /// MQTT publish 可能。
    MqttPub,
    /// MQTT subscribe 可能。
    MqttSub,
}

/// 登録済みデバイス。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: DeviceId,
    pub kind: DeviceKind,
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub registered_at: DateTime<Utc>,
}

/// デバイス登録リクエスト (`POST /devices`)。
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterDevice {
    pub kind: DeviceKind,
    pub name: String,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
}

impl RegisterDevice {
    pub fn into_device(self) -> Device {
        Device {
            id: DeviceId::new(),
            kind: self.kind,
            name: self.name,
            capabilities: self.capabilities,
            registered_at: Utc::now(),
        }
    }
}
