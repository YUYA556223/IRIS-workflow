use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::sdui::SduiSpecId;
use super::target::DeliveryTarget;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct WidgetId(pub Uuid);

impl WidgetId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for WidgetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for WidgetId {
    fn default() -> Self {
        Self::new()
    }
}

/// ウィジェットは「どの SDUI 仕様を、どのデバイス群に、どの bindings で表示するか」を束ねる。
///
/// `bindings` は SDUI 仕様の `bindings` 宣言に対応する動的データ。
/// 更新するとサブスクライブ中のデバイスへ patch がブロードキャストされる。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub id: WidgetId,
    pub name: String,
    pub sdui_spec_id: SduiSpecId,
    pub target: DeliveryTarget,
    pub bindings: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// ウィジェット作成リクエスト (`POST /widgets`)。
#[derive(Debug, Clone, Deserialize)]
pub struct CreateWidget {
    pub name: String,
    pub sdui_spec_id: SduiSpecId,
    pub target: DeliveryTarget,
    #[serde(default)]
    pub bindings: serde_json::Value,
}

impl CreateWidget {
    pub fn into_widget(self) -> Widget {
        let now = Utc::now();
        Widget {
            id: WidgetId::new(),
            name: self.name,
            sdui_spec_id: self.sdui_spec_id,
            target: self.target,
            bindings: self.bindings,
            created_at: now,
            updated_at: now,
        }
    }
}

/// ウィジェット bindings 更新リクエスト (`PATCH /widgets/:id/bindings`)。
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWidgetBindings {
    pub bindings: serde_json::Value,
}
