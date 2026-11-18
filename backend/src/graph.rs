//! Builds and maintains the cross-paper knowledge graph.

use crate::models::*;
use crate::util::{make_id, make_scoped_id};
use std::collections::{HashMap, HashSet};

const METHOD_NAMES: &[&str] = &[
    "NNSplitter", "Magnitude", "LoRO", "TransLinkGuard", "Soter", "TSQP", "ArrowCloak",
    "ArrowMatch", "Collapse", "O_ext", "O_prior", "ShadowNet", "GroupCover", "DarkneTZ",
];

/// Rebuild the whole graph from the paper collection. Deterministic: calling
/// twice with the same papers yields the same nodes/edges.
pub fn rebuild(papers: &[Paper], existing_custom_edges: &[GraphEdge]) -> KnowledgeGraph {
    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut node_index: HashMap<String, usize> = HashMap::new();
    let mut edge_keys: HashSet<String> = HashSet::new();

    for paper in papers {
        let paper_node_id = format!("paper:{}", paper.id);
        add_node(&mut nodes, &mut node_index, GraphNode {
            id: paper_node_id.clone(), kind: NodeKind::Paper,
            label: truncate_label(&paper.title, 58), detail: truncate_label(&paper.summary, 180),
            paper_ids: vec![paper.id.clone()], weight: 6,
        });

        // Concepts are global (shared between papers) so they act as bridges.
        for concept in &paper.concepts {
            let cid = make_id("concept", &concept.name);
            if let Some(idx) = node_index.get(&cid) {
                let node = &mut nodes[*idx];
                if !node.paper_ids.contains(&paper.id) { node.paper_ids.push(paper.id.clone()); node.weight += 1; }
            } else {
                add_node(&mut nodes, &mut node_index, GraphNode {
                    id: cid.clone(), kind: NodeKind::Concept, label: concept.name.clone(),
                    detail: truncate_label(&concept.definition, 170),
                    paper_ids: vec![paper.id.clone()], weight: 2,
                });
            }
            add_edge(&mut edges, &mut edge_keys, GraphEdge {
                id: format!("e|{}|{}|uses", paper_node_id, cid),
                source: paper_node_id.clone(), target: cid, kind: EdgeKind::Uses,
                label: Some("uses".into()), paper_ids: vec![paper.id.clone()],
            });
        }

        for h in &paper.hypotheses {
            let id = h.id.clone();
            add_node(&mut nodes, &mut node_index, GraphNode {
                id: id.clone(), kind: NodeKind::Hypothesis, label: truncate_label(&h.simplified, 52),
                detail: h.statement.clone(), paper_ids: vec![paper.id.clone()],
                weight: 3 + h.evidence_ids.len() as u32,
            });
            add_edge(&mut edges, &mut edge_keys, GraphEdge {
                id: format!("e|{}|{}|proposes", paper_node_id, id),
                source: paper_node_id.clone(), target: id.clone(), kind: EdgeKind::Proposes,
                label: Some("asks".into()), paper_ids: vec![paper.id.clone()],
            });
        }

        for e in &paper.experiments {
            let id = e.id.clone();
            add_node(&mut nodes, &mut node_index, GraphNode {
                id: id.clone(), kind: NodeKind::Experiment, label: truncate_label(&e.name, 52),
                detail: truncate_label(&e.question, 170), paper_ids: vec![paper.id.clone()],
                weight: 3 + e.steps.len() as u32,
            });
            add_edge(&mut edges, &mut edge_keys, GraphEdge {
                id: format!("e|{}|{}|runs", paper_node_id, id),
                source: paper_node_id.clone(), target: id.clone(), kind: EdgeKind::Produces,
                label: Some("runs".into()), paper_ids: vec![paper.id.clone()],
            });
            for hid in &e.hypothesis_ids {
                add_edge(&mut edges, &mut edge_keys, GraphEdge {
                    id: format!("e|{}|{}|tests", id, hid), source: id.clone(), target: hid.clone(),
                    kind: EdgeKind::Tests, label: Some("tests".into()), paper_ids: vec![paper.id.clone()],
                });
            }
            for dataset in &e.setup.datasets {
                let did = make_id("dataset", dataset);
                add_node(&mut nodes, &mut node_index, GraphNode {
                    id: did.clone(), kind: NodeKind::Dataset, label: dataset.clone(), detail: "Evaluation dataset".into(),
                    paper_ids: vec![paper.id.clone()], weight: 1,
                });
                add_edge(&mut edges, &mut edge_keys, GraphEdge {
                    id: format!("e|{}|{}|uses", id, did), source: id.clone(), target: did,
                    kind: EdgeKind::Uses, label: Some("data".into()), paper_ids: vec![paper.id.clone()],
                });
            }
            for metric in e.setup.metrics.iter().take(4) {
                let mid = make_id("metric", metric);
                add_node(&mut nodes, &mut node_index, GraphNode {
                    id: mid.clone(), kind: NodeKind::Metric, label: metric.clone(), detail: "Evaluation metric".into(),
                    paper_ids: vec![paper.id.clone()], weight: 1,
                });
                add_edge(&mut edges, &mut edge_keys, GraphEdge {
                    id: format!("e|{}|{}|uses", id, mid), source: id.clone(), target: mid,
                    kind: EdgeKind::Uses, label: Some("metric".into()), paper_ids: vec![paper.id.clone()],
                });
            }
        }

        for f in &paper.findings {
            let id = f.id.clone();
            add_node(&mut nodes, &mut node_index, GraphNode {
                id: id.clone(), kind: NodeKind::Finding, label: truncate_label(&f.simplified, 52),
                detail: format!("{} — {}", f.claim, f.evidence), paper_ids: vec![paper.id.clone()],
                weight: if f.importance == Importance::High { 4 } else { 2 },
            });
            if let Some(eid) = &f.experiment_id {
                add_edge(&mut edges, &mut edge_keys, GraphEdge {
                    id: format!("e|{}|{}|produces", eid, id), source: eid.clone(), target: id.clone(),
                    kind: EdgeKind::Produces, label: Some("found".into()), paper_ids: vec![paper.id.clone()],
                });
            }
        }

        // Method nodes: named schemes/attacks appearing anywhere in the paper.
        let haystack = paper.all_text();
        for name in METHOD_NAMES {
            if !haystack.to_lowercase().contains(&name.to_lowercase()) { continue; }
            let mid = make_id("method", name);
            if let Some(idx) = node_index.get(&mid) {
                let node = &mut nodes[*idx];
                if !node.paper_ids.contains(&paper.id) { node.paper_ids.push(paper.id.clone()); node.weight += 1; }
            } else {
                add_node(&mut nodes, &mut node_index, GraphNode {
                    id: mid.clone(), kind: NodeKind::Method, label: (*name).into(),
                    detail: "Named method or attack discussed in the collection".into(),
                    paper_ids: vec![paper.id.clone()], weight: 2,
                });
            }
            add_edge(&mut edges, &mut edge_keys, GraphEdge {
                id: format!("e|{}|{}|mentions", paper_node_id, mid),
                source: paper_node_id.clone(), target: mid, kind: EdgeKind::RelatedTo,
                label: Some("discusses".into()), paper_ids: vec![paper.id.clone()],
            });
        }
    }

    // Cross-paper concept bridges: any two papers sharing >= 2 concepts.
    for i in 0..papers.len() {
        for j in (i + 1)..papers.len() {
            let a: HashSet<String> = papers[i].concepts.iter().map(|c| make_id("concept", &c.name)).collect();
            let b: HashSet<String> = papers[j].concepts.iter().map(|c| make_id("concept", &c.name)).collect();
            if a.intersection(&b).count() >= 2 {
                add_edge(&mut edges, &mut edge_keys, GraphEdge {
                    id: format!("e|paper:{}|paper:{}|related", papers[i].id, papers[j].id),
                    source: format!("paper:{}", papers[i].id), target: format!("paper:{}", papers[j].id),
                    kind: EdgeKind::RelatedTo, label: Some("shared concepts".into()),
                    paper_ids: vec![papers[i].id.clone(), papers[j].id.clone()],
                });
            }
        }
    }

    // Citation edges from parsed bibliographies. Resolved references link two
    // paper nodes; unresolved ones get a lightweight reference node so the
    // bibliography is still explorable. `cited_by` is the inverse view exposed
    // by the citations API rather than a duplicate overlapping graph edge.
    for paper in papers {
        let citing_id = format!("paper:{}", paper.id);
        for r in &paper.references {
            if let Some(cited_id) = &r.resolved_paper_id {
                let target = format!("paper:{}", cited_id);
                add_edge(&mut edges, &mut edge_keys, GraphEdge {
                    id: format!("e|{}|{}|cites", citing_id, target),
                    source: citing_id.clone(), target,
                    kind: EdgeKind::Cites, label: Some("cites".into()),
                    paper_ids: vec![paper.id.clone(), cited_id.clone()],
                });
            } else {
                let rid = format!("reference:{}", r.id);
                add_node(&mut nodes, &mut node_index, GraphNode {
                    id: rid.clone(), kind: NodeKind::Reference,
                    label: truncate_label(r.title.as_deref().unwrap_or(&r.raw), 52),
                    detail: truncate_label(&r.raw, 170),
                    paper_ids: vec![paper.id.clone()], weight: 1,
                });
                add_edge(&mut edges, &mut edge_keys, GraphEdge {
                    id: format!("e|{}|{}|cites", citing_id, rid),
                    source: citing_id.clone(), target: rid,
                    kind: EdgeKind::Cites, label: Some("cites".into()),
                    paper_ids: vec![paper.id.clone()],
                });
            }
        }
    }

    for edge in existing_custom_edges {
        add_edge(&mut edges, &mut edge_keys, edge.clone());
    }

    KnowledgeGraph { nodes, edges }
}

fn add_node(nodes: &mut Vec<GraphNode>, index: &mut HashMap<String, usize>, node: GraphNode) {
    if index.contains_key(&node.id) { return; }
    index.insert(node.id.clone(), nodes.len());
    nodes.push(node);
}

fn add_edge(edges: &mut Vec<GraphEdge>, keys: &mut HashSet<String>, edge: GraphEdge) {
    if !keys.insert(edge.id.clone()) { return; }
    edges.push(edge);
}

fn truncate_label(s: &str, max: usize) -> String {
    if s.chars().count() <= max { return s.to_string(); }
    format!("{}…", s.chars().take(max.saturating_sub(1)).collect::<String>())
}

/// Convenience: a Mermaid representation of the graph, used for export.
pub fn graph_to_mermaid(graph: &KnowledgeGraph) -> String {
    let mut s = String::from("flowchart LR\n");
    for n in graph.nodes.iter().take(80) {
        let shape = match n.kind {
            NodeKind::Paper => (">", "<"), NodeKind::Finding => ("([", "])"), _ => ("[", "]"),
        };
        s.push_str(&format!("  {}{}{}{}\n", n.id.replace([':', '@', '.'], "_"), shape.0, crate::explain::mermaid_escape(&n.label), shape.1));
    }
    for e in graph.edges.iter().take(160) {
        s.push_str(&format!("  {} --> {}\n", e.source.replace([':', '@', '.'], "_"), e.target.replace([':', '@', '.'], "_")));
    }
    s
}

#[allow(dead_code)]
fn scoped(_: &str, raw: &str) -> String { make_scoped_id("x", "p", raw) }
