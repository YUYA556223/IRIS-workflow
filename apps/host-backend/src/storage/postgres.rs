//! PostgreSQL 実装。`DATABASE_URL` が設定された時に `AppState` で選択される。
//!
//! 設計:
//! - 行型 (`*Row`) は DB スキーマに 1:1 で対応し、`From` でドメイン型へ変換する。
//!   ドメイン型は IO 中立に保ち、Postgres 依存はこのファイルに隔離。
//! - JSONB 列は `sqlx::types::Json<T>` で型付き、または `serde_json::Value` で動的。
//! - 一意制約違反は `StorageError::Conflict` に翻訳。

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{types::Json, PgPool};
use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::{
    Capability, Component, DeliveryTarget, Device, DeviceId, DeviceKind, SduiSpec, SduiSpecId,
    Widget, WidgetId,
};
use crate::workflow::{ExecutionResult, ExecutionStatus, NodeExecution};

use super::{DeviceRepo, ExecutionRepo, SduiRepo, StorageError, StorageResult, WidgetRepo};

fn db_err(e: sqlx::Error) -> StorageError {
    if let sqlx::Error::Database(ref de) = e {
        if de.is_unique_violation() {
            return StorageError::Conflict(de.to_string());
        }
    }
    StorageError::Other(anyhow::Error::from(e))
}

// ===================== Devices =====================

#[derive(sqlx::FromRow)]
struct DeviceRow {
    id: Uuid,
    kind: DeviceKind,
    name: String,
    capabilities: Json<Vec<Capability>>,
    registered_at: DateTime<Utc>,
}

impl From<DeviceRow> for Device {
    fn from(r: DeviceRow) -> Self {
        Self {
            id: DeviceId(r.id),
            kind: r.kind,
            name: r.name,
            capabilities: r.capabilities.0,
            registered_at: r.registered_at,
        }
    }
}

pub struct PgDeviceRepo {
    pool: PgPool,
}

impl PgDeviceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DeviceRepo for PgDeviceRepo {
    async fn list(&self) -> StorageResult<Vec<Device>> {
        let rows = sqlx::query_as::<_, DeviceRow>(
            "SELECT id, kind, name, capabilities, registered_at FROM devices
             ORDER BY registered_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get(&self, id: DeviceId) -> StorageResult<Option<Device>> {
        let row = sqlx::query_as::<_, DeviceRow>(
            "SELECT id, kind, name, capabilities, registered_at FROM devices WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(row.map(Into::into))
    }

    async fn insert(&self, device: Device) -> StorageResult<()> {
        sqlx::query(
            "INSERT INTO devices (id, kind, name, capabilities, registered_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(device.id.0)
        .bind(device.kind)
        .bind(&device.name)
        .bind(Json(&device.capabilities))
        .bind(device.registered_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, id: DeviceId) -> StorageResult<bool> {
        let res = sqlx::query("DELETE FROM devices WHERE id = $1")
            .bind(id.0)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(res.rows_affected() > 0)
    }

    async fn find_by_kind(&self, kind: DeviceKind) -> StorageResult<Vec<Device>> {
        let rows = sqlx::query_as::<_, DeviceRow>(
            "SELECT id, kind, name, capabilities, registered_at FROM devices WHERE kind = $1",
        )
        .bind(kind)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_by_capability(&self, capability: Capability) -> StorageResult<Vec<Device>> {
        // JSONB 配列の containment 検索: capabilities @> '["widget"]'::jsonb
        let needle = serde_json::to_value(vec![capability]).unwrap_or(serde_json::json!([]));
        let rows = sqlx::query_as::<_, DeviceRow>(
            "SELECT id, kind, name, capabilities, registered_at FROM devices
             WHERE capabilities @> $1",
        )
        .bind(needle)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

// ===================== SDUI Specs =====================

#[derive(sqlx::FromRow)]
struct SduiSpecRow {
    id: String,
    kind: String,
    root: Json<Component>,
    bindings: Json<HashMap<String, String>>,
}

impl From<SduiSpecRow> for SduiSpec {
    fn from(r: SduiSpecRow) -> Self {
        Self {
            id: SduiSpecId(r.id),
            kind: r.kind,
            root: r.root.0,
            bindings: r.bindings.0,
        }
    }
}

pub struct PgSduiRepo {
    pool: PgPool,
}

impl PgSduiRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SduiRepo for PgSduiRepo {
    async fn list(&self) -> StorageResult<Vec<SduiSpec>> {
        let rows = sqlx::query_as::<_, SduiSpecRow>(
            "SELECT id, kind, root, bindings FROM sdui_specs ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get(&self, id: &SduiSpecId) -> StorageResult<Option<SduiSpec>> {
        let row = sqlx::query_as::<_, SduiSpecRow>(
            "SELECT id, kind, root, bindings FROM sdui_specs WHERE id = $1",
        )
        .bind(&id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(row.map(Into::into))
    }

    async fn upsert(&self, spec: SduiSpec) -> StorageResult<()> {
        sqlx::query(
            "INSERT INTO sdui_specs (id, kind, root, bindings)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (id) DO UPDATE SET
                kind = EXCLUDED.kind,
                root = EXCLUDED.root,
                bindings = EXCLUDED.bindings",
        )
        .bind(&spec.id.0)
        .bind(&spec.kind)
        .bind(Json(&spec.root))
        .bind(Json(&spec.bindings))
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, id: &SduiSpecId) -> StorageResult<bool> {
        let res = sqlx::query("DELETE FROM sdui_specs WHERE id = $1")
            .bind(&id.0)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(res.rows_affected() > 0)
    }
}

// ===================== Widgets =====================

#[derive(sqlx::FromRow)]
struct WidgetRow {
    id: Uuid,
    name: String,
    sdui_spec_id: String,
    target: Json<DeliveryTarget>,
    bindings: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<WidgetRow> for Widget {
    fn from(r: WidgetRow) -> Self {
        Self {
            id: WidgetId(r.id),
            name: r.name,
            sdui_spec_id: SduiSpecId(r.sdui_spec_id),
            target: r.target.0,
            bindings: r.bindings,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct PgWidgetRepo {
    pool: PgPool,
}

impl PgWidgetRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WidgetRepo for PgWidgetRepo {
    async fn list(&self) -> StorageResult<Vec<Widget>> {
        let rows = sqlx::query_as::<_, WidgetRow>(
            "SELECT id, name, sdui_spec_id, target, bindings, created_at, updated_at
             FROM widgets ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get(&self, id: WidgetId) -> StorageResult<Option<Widget>> {
        let row = sqlx::query_as::<_, WidgetRow>(
            "SELECT id, name, sdui_spec_id, target, bindings, created_at, updated_at
             FROM widgets WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(row.map(Into::into))
    }

    async fn insert(&self, widget: Widget) -> StorageResult<()> {
        sqlx::query(
            "INSERT INTO widgets (id, name, sdui_spec_id, target, bindings, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(widget.id.0)
        .bind(&widget.name)
        .bind(&widget.sdui_spec_id.0)
        .bind(Json(&widget.target))
        .bind(&widget.bindings)
        .bind(widget.created_at)
        .bind(widget.updated_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn update_bindings(
        &self,
        id: WidgetId,
        bindings: serde_json::Value,
    ) -> StorageResult<Widget> {
        let row = sqlx::query_as::<_, WidgetRow>(
            "UPDATE widgets SET bindings = $2, updated_at = now()
             WHERE id = $1
             RETURNING id, name, sdui_spec_id, target, bindings, created_at, updated_at",
        )
        .bind(id.0)
        .bind(bindings)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.map(Into::into).ok_or(StorageError::NotFound)
    }

    async fn delete(&self, id: WidgetId) -> StorageResult<bool> {
        let res = sqlx::query("DELETE FROM widgets WHERE id = $1")
            .bind(id.0)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(res.rows_affected() > 0)
    }
}

// ===================== Workflow Executions =====================

#[derive(sqlx::FromRow)]
struct ExecutionRow {
    id: Uuid,
    workflow_id: String,
    status: ExecutionStatus,
    trigger_data: serde_json::Value,
    nodes: Json<Vec<NodeExecution>>,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    error: Option<String>,
}

impl From<ExecutionRow> for ExecutionResult {
    fn from(r: ExecutionRow) -> Self {
        Self {
            execution_id: r.id,
            workflow_id: r.workflow_id,
            status: r.status,
            trigger_data: r.trigger_data,
            nodes: r.nodes.0,
            started_at: r.started_at,
            finished_at: r.finished_at,
            error: r.error,
        }
    }
}

pub struct PgExecutionRepo {
    pool: PgPool,
}

impl PgExecutionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ExecutionRepo for PgExecutionRepo {
    async fn save(&self, exec: ExecutionResult) -> StorageResult<()> {
        sqlx::query(
            "INSERT INTO workflow_executions
                (id, workflow_id, status, trigger_data, nodes, started_at, finished_at, error)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(exec.execution_id)
        .bind(&exec.workflow_id)
        .bind(exec.status)
        .bind(&exec.trigger_data)
        .bind(Json(&exec.nodes))
        .bind(exec.started_at)
        .bind(exec.finished_at)
        .bind(&exec.error)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn list(
        &self,
        workflow_id: Option<&str>,
        limit: usize,
    ) -> StorageResult<Vec<ExecutionResult>> {
        let limit_i = i64::try_from(limit).unwrap_or(100);
        let rows = if let Some(wid) = workflow_id {
            sqlx::query_as::<_, ExecutionRow>(
                "SELECT id, workflow_id, status, trigger_data, nodes, started_at, finished_at, error
                 FROM workflow_executions
                 WHERE workflow_id = $1
                 ORDER BY started_at DESC
                 LIMIT $2",
            )
            .bind(wid)
            .bind(limit_i)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, ExecutionRow>(
                "SELECT id, workflow_id, status, trigger_data, nodes, started_at, finished_at, error
                 FROM workflow_executions
                 ORDER BY started_at DESC
                 LIMIT $1",
            )
            .bind(limit_i)
            .fetch_all(&self.pool)
            .await
        };
        let rows = rows.map_err(db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get(&self, id: Uuid) -> StorageResult<Option<ExecutionResult>> {
        let row = sqlx::query_as::<_, ExecutionRow>(
            "SELECT id, workflow_id, status, trigger_data, nodes, started_at, finished_at, error
             FROM workflow_executions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(row.map(Into::into))
    }
}
