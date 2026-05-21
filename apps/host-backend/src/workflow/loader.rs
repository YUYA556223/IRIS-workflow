use std::path::Path;

use anyhow::Context;

use super::dsl::Workflow;

/// ディレクトリ配下の `*.yaml` / `*.yml` を全て `Workflow` としてパースして返す。
///
/// - パース失敗ファイルは個別にログを残し、他のファイルのロードは継続
/// - ディレクトリが存在しない場合は空 Vec を返す (エラーにしない)
pub fn load_dir(dir: &Path) -> anyhow::Result<Vec<Workflow>> {
    if !dir.exists() {
        tracing::warn!(path = %dir.display(), "workflows directory does not exist");
        return Ok(Vec::new());
    }

    let mut workflows = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !matches!(ext, "yaml" | "yml") {
            continue;
        }

        match parse_file(&path) {
            Ok(wf) => {
                tracing::info!(
                    path = %path.display(),
                    workflow_id = %wf.id,
                    nodes = wf.nodes.len(),
                    "workflow loaded"
                );
                workflows.push(wf);
            }
            Err(e) => {
                tracing::error!(path = %path.display(), error = %e, "failed to parse workflow");
            }
        }
    }
    Ok(workflows)
}

fn parse_file(path: &Path) -> anyhow::Result<Workflow> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let wf: Workflow =
        serde_yaml::from_str(&content).with_context(|| format!("yaml parse {}", path.display()))?;
    Ok(wf)
}
