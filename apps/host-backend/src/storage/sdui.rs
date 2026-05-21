use async_trait::async_trait;

use crate::domain::{SduiSpec, SduiSpecId};

use super::StorageResult;

#[async_trait]
pub trait SduiRepo: Send + Sync + 'static {
    async fn list(&self) -> StorageResult<Vec<SduiSpec>>;
    async fn get(&self, id: &SduiSpecId) -> StorageResult<Option<SduiSpec>>;
    async fn upsert(&self, spec: SduiSpec) -> StorageResult<()>;
    async fn delete(&self, id: &SduiSpecId) -> StorageResult<bool>;
}
