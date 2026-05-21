use async_trait::async_trait;

use crate::domain::{Widget, WidgetId};

use super::StorageResult;

#[async_trait]
pub trait WidgetRepo: Send + Sync + 'static {
    async fn list(&self) -> StorageResult<Vec<Widget>>;
    async fn get(&self, id: WidgetId) -> StorageResult<Option<Widget>>;
    async fn insert(&self, widget: Widget) -> StorageResult<()>;
    async fn update_bindings(
        &self,
        id: WidgetId,
        bindings: serde_json::Value,
    ) -> StorageResult<Widget>;
    async fn delete(&self, id: WidgetId) -> StorageResult<bool>;
}
