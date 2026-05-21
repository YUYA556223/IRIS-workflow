use anyhow::Context as _;
use host_backend::{build_app, mqtt::MqttBus, telemetry, AppState, Config};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    telemetry::init_tracing();

    let config = Config::from_env();
    let bind = config.bind;

    // MQTT 接続 (任意)。state 構築前に確立する。
    let mqtt: Option<Arc<MqttBus>> = if let Some(url) = &config.mqtt_broker {
        tracing::info!(broker = %url, "connecting to MQTT broker");
        match MqttBus::connect(url, &config.mqtt_client_id).await {
            Ok(bus) => Some(bus),
            Err(e) => {
                tracing::error!(error = %e, "mqtt connection failed; continuing without MQTT");
                None
            }
        }
    } else {
        None
    };

    let state = match &config.database_url {
        Some(url) => {
            tracing::info!("DATABASE_URL set, connecting to PostgreSQL");
            let pool = PgPoolOptions::new()
                .max_connections(16)
                .acquire_timeout(std::time::Duration::from_secs(5))
                .connect(url)
                .await
                .context("connect to PostgreSQL")?;

            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .context("apply migrations")?;
            tracing::info!("migrations applied");

            AppState::new_with_pool(config.clone(), pool, mqtt.clone())
        }
        None => {
            tracing::warn!("DATABASE_URL not set, using in-memory storage (data lost on restart)");
            AppState::new_in_memory(config.clone(), mqtt.clone())
        }
    };

    // ワークフロー定義のディレクトリロード (任意)
    if let Some(dir) = state.config.workflows_dir.clone() {
        match state.workflows.load_dir(&dir) {
            Ok(n) => tracing::info!(count = n, path = %dir.display(), "workflows loaded"),
            Err(e) => tracing::error!(error = %e, path = %dir.display(), "failed to load workflows"),
        }
    }

    // ロード済みワークフローに基づいてトリガを登録
    state.triggers.sync().await;

    let app = build_app(state);

    tracing::info!(%bind, "host-backend listening");
    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
