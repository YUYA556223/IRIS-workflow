use serde::{Deserialize, Serialize};

use super::device::{Capability, Device, DeviceId, DeviceKind};

/// 通知やウィジェット更新の配信先を表す。
///
/// - `All`        : 接続中の全デバイス
/// - `Device`     : 特定デバイス1つ
/// - `Kind`       : OS/フォームファクタ単位 (例: iOS 全機)
/// - `Capability` : 特定 capability を持つデバイス群 (例: `widget` 持ち)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DeliveryTarget {
    All,
    Device { id: DeviceId },
    Kind { kind: DeviceKind },
    Capability { capability: Capability },
}

impl DeliveryTarget {
    /// 与えられたデバイスがこのターゲットにマッチするかを判定する。
    pub fn matches(&self, device: &Device) -> bool {
        match self {
            Self::All => true,
            Self::Device { id } => device.id == *id,
            Self::Kind { kind } => device.kind == *kind,
            Self::Capability { capability } => device.capabilities.contains(capability),
        }
    }
}
