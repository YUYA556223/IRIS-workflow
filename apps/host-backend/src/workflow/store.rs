use std::path::Path;
use std::sync::Arc;

use dashmap::DashMap;

use super::dsl::Workflow;

/// ロード済みワークフローの in-memory レジストリ。
///
/// 内部は `Arc<Workflow>` で保持し、`get` / `list` は cheap clone を返す。
/// これにより cron/fs/mqtt/webhook 起動のたびに発生していた `Workflow` の
/// deep clone を排除できる。
#[derive(Default)]
pub struct WorkflowStore {
    inner: DashMap<String, Arc<Workflow>>,
}

impl WorkflowStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(&self) -> Vec<Arc<Workflow>> {
        let mut v: Vec<_> = self.inner.iter().map(|e| e.value().clone()).collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }

    pub fn get(&self, id: &str) -> Option<Arc<Workflow>> {
        self.inner.get(id).map(|e| e.value().clone())
    }

    pub fn upsert(&self, wf: Workflow) {
        self.inner.insert(wf.id.clone(), Arc::new(wf));
    }

    pub fn delete(&self, id: &str) -> bool {
        self.inner.remove(id).is_some()
    }

    /// 指定ディレクトリの YAML を全ロードして取り込む。既存ものは上書きされる。
    /// 戻り値はロード数。ディレクトリ未存在時は 0。
    pub fn load_dir(&self, dir: &Path) -> anyhow::Result<usize> {
        let workflows = super::loader::load_dir(dir)?;
        let count = workflows.len();
        for wf in workflows {
            self.inner.insert(wf.id.clone(), Arc::new(wf));
        }
        Ok(count)
    }
}
