use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: SocketAddr,
    /// `DeliveryHub` の broadcast チャネル容量。
    pub delivery_capacity: usize,
    /// 設定時は Postgres バックエンドを採用。未設定ならメモリ実装。
    pub database_url: Option<String>,
    /// Claude Code (`claude` CLI) の同時実行上限。
    pub ai_concurrency: usize,
    /// Claude Code 呼び出しの最大 wall-clock タイムアウト (秒)。
    pub ai_timeout_secs: u64,
    /// ワークフロー定義 YAML を読み込むディレクトリ。未設定ならロードなし。
    pub workflows_dir: Option<PathBuf>,
    /// permission-prompt 応答待ちのタイムアウト (秒)。
    pub permission_timeout_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env()
    }
}

impl Config {
    pub fn from_env() -> Self {
        let bind = std::env::var("IRIS_BIND")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 8787)));

        let delivery_capacity = std::env::var("IRIS_DELIVERY_CAPACITY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(256);

        let database_url = std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty());

        let ai_concurrency = std::env::var("IRIS_AI_CONCURRENCY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| (num_cpus_or_default(4) / 2).max(1));

        let ai_timeout_secs = std::env::var("IRIS_AI_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(600);

        let workflows_dir = std::env::var("IRIS_WORKFLOWS_DIR")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);

        let permission_timeout_secs = std::env::var("IRIS_PERMISSION_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120);

        Self {
            bind,
            delivery_capacity,
            database_url,
            ai_concurrency,
            ai_timeout_secs,
            workflows_dir,
            permission_timeout_secs,
        }
    }
}

fn num_cpus_or_default(default: usize) -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(default)
}
