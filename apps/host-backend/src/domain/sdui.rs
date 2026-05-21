use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// SDUI 仕様の ID。ワークフローや AI が生成・参照する人間可読 ID を許容する
/// (例: `"briefing-card-v1"`)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct SduiSpecId(pub String);

impl SduiSpecId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for SduiSpecId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<String> for SduiSpecId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SduiSpecId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

/// Server-driven UI 仕様の最上位。
/// 詳細は `docs/concept/06-server-driven-ui.md` と
/// `packages/proto/sdui.schema.json` を参照。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SduiSpec {
    pub id: SduiSpecId,
    /// 現状は `"ComponentTree"` 固定。将来 `"Theme"` などを増やす余地のため文字列のまま。
    #[serde(rename = "type")]
    pub kind: String,
    pub root: Component,
    /// bindings 名 → 型ヒント (例: `"title": "string"`)。レンダラ側で型チェック可能。
    #[serde(default)]
    pub bindings: HashMap<String, String>,
}

/// UI コンポーネントツリーのノード。
///
/// `#[serde(tag = "type")]` により JSON では `{"type": "VStack", ...}` 形式になる。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Component {
    VStack {
        #[serde(default)]
        spacing: f32,
        #[serde(default)]
        children: Vec<Component>,
    },
    HStack {
        #[serde(default)]
        spacing: f32,
        #[serde(default)]
        children: Vec<Component>,
    },
    ZStack {
        #[serde(default)]
        children: Vec<Component>,
    },
    Text {
        value: String,
        #[serde(default)]
        style: String,
    },
    Image {
        src: String,
        #[serde(default)]
        alt: String,
    },
    Button {
        label: String,
        /// タップ時のイベントペイロード。`{"type":"Event","name":"...","payload":...}` を想定。
        #[serde(rename = "onTap", default)]
        on_tap: serde_json::Value,
    },
    Spacer,
    Divider,
    List {
        #[serde(default)]
        items: Vec<Component>,
    },
    Toggle {
        label: String,
        value: bool,
    },
    ProgressBar {
        value: f32,
    },
}
