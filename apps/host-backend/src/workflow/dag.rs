use std::collections::{HashMap, HashSet, VecDeque};

use super::dsl::Workflow;

/// ワークフローをトポロジカルソートしてノードの実行順を返す。
///
/// - エッジで参照されるノードが定義に無い場合はエラー
/// - サイクルが含まれる場合はエラー
/// - 同列ノード (依存関係なし) は YAML の `nodes` 順を保つ
pub fn topo_sort(wf: &Workflow) -> anyhow::Result<Vec<String>> {
    let node_ids: HashSet<&str> = wf.nodes.iter().map(|n| n.id.as_str()).collect();
    if node_ids.len() != wf.nodes.len() {
        anyhow::bail!("workflow contains duplicate node IDs");
    }

    let mut in_degree: HashMap<&str, usize> =
        wf.nodes.iter().map(|n| (n.id.as_str(), 0_usize)).collect();
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

    for edge in &wf.edges {
        if !node_ids.contains(edge.from.as_str()) {
            anyhow::bail!("edge references unknown node: '{}'", edge.from);
        }
        if !node_ids.contains(edge.to.as_str()) {
            anyhow::bail!("edge references unknown node: '{}'", edge.to);
        }
        if edge.from == edge.to {
            anyhow::bail!("self-edge on node '{}'", edge.from);
        }
        *in_degree.entry(edge.to.as_str()).or_insert(0) += 1;
        adjacency
            .entry(edge.from.as_str())
            .or_default()
            .push(edge.to.as_str());
    }

    // 安定性: YAML の nodes 順で初期キューに投入
    let mut queue: VecDeque<&str> = wf
        .nodes
        .iter()
        .filter(|n| in_degree.get(n.id.as_str()).copied().unwrap_or(0) == 0)
        .map(|n| n.id.as_str())
        .collect();

    let mut order = Vec::with_capacity(wf.nodes.len());
    while let Some(id) = queue.pop_front() {
        order.push(id.to_owned());
        if let Some(neighbors) = adjacency.get(id) {
            for n in neighbors {
                if let Some(deg) = in_degree.get_mut(n) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(n);
                    }
                }
            }
        }
    }

    if order.len() != wf.nodes.len() {
        anyhow::bail!("workflow contains a cycle");
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::dsl::{Node, NodeType, Trigger};

    fn mk(id: &str) -> Node {
        Node {
            id: id.to_owned(),
            kind: NodeType::Transform,
            using: None,
            with: serde_json::Value::Null,
        }
    }

    #[test]
    fn linear() {
        let wf = Workflow {
            id: "t".into(),
            name: "t".into(),
            description: None,
            trigger: Trigger::Manual,
            nodes: vec![mk("a"), mk("b"), mk("c")],
            edges: vec![
                super::super::dsl::Edge {
                    from: "a".into(),
                    to: "b".into(),
                },
                super::super::dsl::Edge {
                    from: "b".into(),
                    to: "c".into(),
                },
            ],
        };
        assert_eq!(topo_sort(&wf).unwrap(), vec!["a", "b", "c"]);
    }

    #[test]
    fn detects_cycle() {
        let wf = Workflow {
            id: "t".into(),
            name: "t".into(),
            description: None,
            trigger: Trigger::Manual,
            nodes: vec![mk("a"), mk("b")],
            edges: vec![
                super::super::dsl::Edge {
                    from: "a".into(),
                    to: "b".into(),
                },
                super::super::dsl::Edge {
                    from: "b".into(),
                    to: "a".into(),
                },
            ],
        };
        assert!(topo_sort(&wf).is_err());
    }
}
