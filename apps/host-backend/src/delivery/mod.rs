//! デバイス向け配信レイヤ。
//!
//! - `DeliveryEvent` : デバイスへ送るペイロード (通知 / ウィジェット更新 / ハートビート etc.)
//! - `DeliveryEnvelope` : event + target (どのデバイス群に届けるか)
//! - `DeliveryHub` : 内部 broadcast チャネル。API ハンドラがここに publish し、
//!   WebSocket セッション (`api/ws.rs`) が subscribe してデバイスへ送る。

pub mod event;
pub mod hub;

pub use event::{DeliveryEnvelope, DeliveryEvent};
pub use hub::DeliveryHub;
