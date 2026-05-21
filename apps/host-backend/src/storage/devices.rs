use async_trait::async_trait;

use crate::domain::{Capability, Device, DeviceId, DeviceKind};

use super::StorageResult;

#[async_trait]
pub trait DeviceRepo: Send + Sync + 'static {
    async fn list(&self) -> StorageResult<Vec<Device>>;
    async fn get(&self, id: DeviceId) -> StorageResult<Option<Device>>;
    async fn insert(&self, device: Device) -> StorageResult<()>;
    async fn delete(&self, id: DeviceId) -> StorageResult<bool>;

    /// 指定 kind / capability に該当する全デバイスを返す。
    /// `DeliveryHub` がブロードキャスト時にフィルタする際に使う。
    async fn find_by_kind(&self, kind: DeviceKind) -> StorageResult<Vec<Device>>;
    async fn find_by_capability(&self, capability: Capability) -> StorageResult<Vec<Device>>;
}
