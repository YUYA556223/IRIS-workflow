use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;

use crate::{
    ai::{ClaudeService, PermissionRegistry, PermissionRegistryHandle},
    config::Config,
    delivery::DeliveryHub,
    mqtt::MqttBus,
    storage::{
        memory::{MemoryDeviceRepo, MemoryExecutionRepo, MemorySduiRepo, MemoryWidgetRepo},
        postgres::{PgDeviceRepo, PgExecutionRepo, PgSduiRepo, PgWidgetRepo},
        DeviceRepo, ExecutionRepo, SduiRepo, WidgetRepo,
    },
    triggers::TriggerHub,
    workflow::{WorkflowExecutor, WorkflowStore},
};

/// アプリ全体の共有状態。`axum::extract::State` で各ハンドラに注入される。
#[derive(Clone)]
pub struct AppState {
    pub devices: Arc<dyn DeviceRepo>,
    pub widgets: Arc<dyn WidgetRepo>,
    pub sdui: Arc<dyn SduiRepo>,
    pub executions: Arc<dyn ExecutionRepo>,
    pub delivery: Arc<DeliveryHub>,
    pub claude: Arc<ClaudeService>,
    pub workflows: Arc<WorkflowStore>,
    pub executor: Arc<WorkflowExecutor>,
    pub triggers: Arc<TriggerHub>,
    pub permission: PermissionRegistryHandle,
    pub mqtt: Option<Arc<MqttBus>>,
    pub config: Arc<Config>,
}

impl AppState {
    fn build_runtime(config: &Config) -> (Arc<DeliveryHub>, Arc<ClaudeService>) {
        let delivery = Arc::new(DeliveryHub::new(config.delivery_capacity));
        let claude = Arc::new(ClaudeService::new(
            config.ai_concurrency,
            Duration::from_secs(config.ai_timeout_secs),
        ));
        (delivery, claude)
    }

    #[allow(clippy::too_many_arguments)]
    fn build_executor(
        claude: Arc<ClaudeService>,
        delivery: Arc<DeliveryHub>,
        widgets: Arc<dyn WidgetRepo>,
        sdui: Arc<dyn SduiRepo>,
        executions: Arc<dyn ExecutionRepo>,
        workflows: Arc<WorkflowStore>,
        mqtt: Option<Arc<MqttBus>>,
    ) -> Arc<WorkflowExecutor> {
        Arc::new(WorkflowExecutor::new(
            claude, delivery, widgets, sdui, executions, workflows, mqtt,
        ))
    }

    /// 全リポジトリをメモリ実装で構築する (テスト用 / DATABASE_URL 未設定時)。
    pub fn new_in_memory(config: Config, mqtt: Option<Arc<MqttBus>>) -> Self {
        let (delivery, claude) = Self::build_runtime(&config);
        let widgets: Arc<dyn WidgetRepo> = Arc::new(MemoryWidgetRepo::new());
        let sdui: Arc<dyn SduiRepo> = Arc::new(MemorySduiRepo::new());
        let executions: Arc<dyn ExecutionRepo> = Arc::new(MemoryExecutionRepo::new());
        let workflows = Arc::new(WorkflowStore::new());
        let executor = Self::build_executor(
            claude.clone(),
            delivery.clone(),
            widgets.clone(),
            sdui.clone(),
            executions.clone(),
            workflows.clone(),
            mqtt.clone(),
        );
        let triggers = Arc::new(TriggerHub::new(
            workflows.clone(),
            executor.clone(),
            mqtt.clone(),
        ));
        let permission: PermissionRegistryHandle = Arc::new(PermissionRegistry::new(
            Duration::from_secs(config.permission_timeout_secs),
        ));
        Self {
            devices: Arc::new(MemoryDeviceRepo::new()),
            widgets,
            sdui,
            executions,
            delivery,
            claude,
            workflows,
            executor,
            triggers,
            permission,
            mqtt,
            config: Arc::new(config),
        }
    }

    /// 全リポジトリを PostgreSQL 実装で構築する。
    pub fn new_with_pool(config: Config, pool: PgPool, mqtt: Option<Arc<MqttBus>>) -> Self {
        let (delivery, claude) = Self::build_runtime(&config);
        let widgets: Arc<dyn WidgetRepo> = Arc::new(PgWidgetRepo::new(pool.clone()));
        let sdui: Arc<dyn SduiRepo> = Arc::new(PgSduiRepo::new(pool.clone()));
        let executions: Arc<dyn ExecutionRepo> = Arc::new(PgExecutionRepo::new(pool.clone()));
        let workflows = Arc::new(WorkflowStore::new());
        let executor = Self::build_executor(
            claude.clone(),
            delivery.clone(),
            widgets.clone(),
            sdui.clone(),
            executions.clone(),
            workflows.clone(),
            mqtt.clone(),
        );
        let triggers = Arc::new(TriggerHub::new(
            workflows.clone(),
            executor.clone(),
            mqtt.clone(),
        ));
        let permission: PermissionRegistryHandle = Arc::new(PermissionRegistry::new(
            Duration::from_secs(config.permission_timeout_secs),
        ));
        Self {
            devices: Arc::new(PgDeviceRepo::new(pool)),
            widgets,
            sdui,
            executions,
            delivery,
            claude,
            workflows,
            executor,
            triggers,
            permission,
            mqtt,
            config: Arc::new(config),
        }
    }
}
