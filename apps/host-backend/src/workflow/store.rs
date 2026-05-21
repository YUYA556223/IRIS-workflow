use std::path::Path;

use dashmap::DashMap;

use super::dsl::Workflow;

/// ロード済みワークフローの in-memory レジストリ。
///
/// YAML ファイル群が source of truth で、`POST /workflows` で上書きされた
/// 場合はメモリのみに保持される (将来: ファイルへ書き戻すオプション)。
#[derive(Default)]
pub struct WorkflowStore {
    inner: DashMap<String, Workflow>,
}

impl WorkflowStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(&self) -> Vec<Workflow> {
        let mut v: Vec<_> = self.inner.iter().map(|e| e.value().clone()).collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }

    pub fn get(&self, id: &str) -> Option<Workflow> {
        self.inner.get(id).map(|e| e.value().clone())
    }

    pub fn upsert(&self, wf: Workflow) {
        self.inner.insert(wf.id.clone(), wf);
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
            self.inner.insert(wf.id.clone(), wf);
        }
        Ok(count)
    }
}
