//! メモリ実装。プロセス再起動で消える。P1.5 で PostgreSQL 実装に置き換える。

use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use uuid::Uuid;

use crate::domain::{
    Capability, Device, DeviceId, DeviceKind, SduiSpec, SduiSpecId, Widget, WidgetId,
};
use crate::workflow::ExecutionResult;

use super::{DeviceRepo, ExecutionRepo, SduiRepo, StorageError, StorageResult, WidgetRepo};

/// メモリ保持の上限。これを超えると古いものから捨てる。
const MEMORY_EXECUTION_CAP: usize = 1000;

#[derive(Default)]
pub struct MemoryDeviceRepo {
    inner: DashMap<DeviceId, Device>,
}

impl MemoryDeviceRepo {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl DeviceRepo for MemoryDeviceRepo {
    async fn list(&self) -> StorageResult<Vec<Device>> {
        Ok(self.inner.iter().map(|e| e.value().clone()).collect())
    }

    async fn get(&self, id: DeviceId) -> StorageResult<Option<Device>> {
        Ok(self.inner.get(&id).map(|e| e.value().clone()))
    }

    async fn insert(&self, device: Device) -> StorageResult<()> {
        self.inner.insert(device.id, device);
        Ok(())
    }

    async fn delete(&self, id: DeviceId) -> StorageResult<bool> {
        Ok(self.inner.remove(&id).is_some())
    }

    async fn find_by_kind(&self, kind: DeviceKind) -> StorageResult<Vec<Device>> {
        Ok(self
            .inner
            .iter()
            .filter(|e| e.value().kind == kind)
            .map(|e| e.value().clone())
            .collect())
    }

    async fn find_by_capability(&self, capability: Capability) -> StorageResult<Vec<Device>> {
        Ok(self
            .inner
            .iter()
            .filter(|e| e.value().capabilities.contains(&capability))
            .map(|e| e.value().clone())
            .collect())
    }
}

#[derive(Default)]
pub struct MemoryWidgetRepo {
    inner: DashMap<WidgetId, Widget>,
}

impl MemoryWidgetRepo {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl WidgetRepo for MemoryWidgetRepo {
    async fn list(&self) -> StorageResult<Vec<Widget>> {
        Ok(self.inner.iter().map(|e| e.value().clone()).collect())
    }

    async fn get(&self, id: WidgetId) -> StorageResult<Option<Widget>> {
        Ok(self.inner.get(&id).map(|e| e.value().clone()))
    }

    async fn insert(&self, widget: Widget) -> StorageResult<()> {
        self.inner.insert(widget.id, widget);
        Ok(())
    }

    async fn update_bindings(
        &self,
        id: WidgetId,
        bindings: serde_json::Value,
    ) -> StorageResult<Widget> {
        let mut entry = self.inner.get_mut(&id).ok_or(StorageError::NotFound)?;
        entry.bindings = bindings;
        entry.updated_at = Utc::now();
        Ok(entry.value().clone())
    }

    async fn delete(&self, id: WidgetId) -> StorageResult<bool> {
        Ok(self.inner.remove(&id).is_some())
    }
}

#[derive(Default)]
pub struct MemorySduiRepo {
    inner: DashMap<SduiSpecId, SduiSpec>,
}

impl MemorySduiRepo {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Default)]
pub struct MemoryExecutionRepo {
    /// 新しい順に積む (push_front 相当)。
    inner: Mutex<Vec<ExecutionResult>>,
}

impl MemoryExecutionRepo {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ExecutionRepo for MemoryExecutionRepo {
    async fn save(&self, exec: ExecutionResult) -> StorageResult<()> {
        let mut v = self
            .inner
            .lock()
            .map_err(|_| StorageError::Other(anyhow::anyhow!("mutex poisoned")))?;
        v.insert(0, exec);
        if v.len() > MEMORY_EXECUTION_CAP {
            v.truncate(MEMORY_EXECUTION_CAP);
        }
        Ok(())
    }

    async fn list(
        &self,
        workflow_id: Option<&str>,
        limit: usize,
    ) -> StorageResult<Vec<ExecutionResult>> {
        let v = self
            .inner
            .lock()
            .map_err(|_| StorageError::Other(anyhow::anyhow!("mutex poisoned")))?;
        let out: Vec<_> = v
            .iter()
            .filter(|e| workflow_id.map_or(true, |id| e.workflow_id == id))
            .take(limit)
            .cloned()
            .collect();
        Ok(out)
    }

    async fn get(&self, id: Uuid) -> StorageResult<Option<ExecutionResult>> {
        let v = self
            .inner
            .lock()
            .map_err(|_| StorageError::Other(anyhow::anyhow!("mutex poisoned")))?;
        Ok(v.iter().find(|e| e.execution_id == id).cloned())
    }
}

#[async_trait]
impl SduiRepo for MemorySduiRepo {
    async fn list(&self) -> StorageResult<Vec<SduiSpec>> {
        Ok(self.inner.iter().map(|e| e.value().clone()).collect())
    }

    async fn get(&self, id: &SduiSpecId) -> StorageResult<Option<SduiSpec>> {
        Ok(self.inner.get(id).map(|e| e.value().clone()))
    }

    async fn upsert(&self, spec: SduiSpec) -> StorageResult<()> {
        self.inner.insert(spec.id.clone(), spec);
        Ok(())
    }

    async fn delete(&self, id: &SduiSpecId) -> StorageResult<bool> {
        Ok(self.inner.remove(id).is_some())
    }
}
