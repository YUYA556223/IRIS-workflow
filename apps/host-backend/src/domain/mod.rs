//! ドメイン型: IO に依存しない純粋なエンティティ・値オブジェクトを定義する。

pub mod device;
pub mod notification;
pub mod sdui;
pub mod target;
pub mod widget;

pub use device::{Capability, Device, DeviceId, DeviceKind, RegisterDevice};
pub use notification::{DispatchNotification, Notification, NotificationId, NotificationPriority};
pub use sdui::{Component, SduiSpec, SduiSpecId};
pub use target::DeliveryTarget;
pub use widget::{UpdateWidgetBindings, Widget, WidgetId};
