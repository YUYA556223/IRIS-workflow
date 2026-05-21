use async_trait::async_trait;
use uuid::Uuid;

use crate::workflow::ExecutionResult;

use super::StorageResult;

#[async_trait]
pub trait ExecutionRepo: Send + Sync + 'static {
    async fn save(&self, exec: ExecutionResult) -> StorageResult<()>;
    async fn list(
        &self,
        workflow_id: Option<&str>,
        limit: usize,
    ) -> StorageResult<Vec<ExecutionResult>>;
    async fn get(&self, id: Uuid) -> StorageResult<Option<ExecutionResult>>;
}
