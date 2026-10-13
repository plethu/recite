//! Large scenes open around the selected beat; omitted content stays explicit.
use super::{layout::HEIGHT, topology::Topology};
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, PartialEq)]
pub(super) struct Scope {
    pub members: BTreeSet<String>,
    pub node_indices: Vec<usize>,
    pub edge_indices: Vec<usize>,
    pub bounds: (f32, f32, f32, f32),
}
pub(super) fn nearby(graph: &Topology, selected: Option<&str>) -> Option<Scope> {
    let blocks = &graph.blocks;
    if blocks.len() <= 200 {
        return None;
    }
    let start = selected
        .and_then(|id| graph.ids.get(id).copied())
        .unwrap_or(0);
    let mut selected = BTreeSet::from([start]);
    let mut pending = VecDeque::from([start]);
    while let Some(index) = pending.pop_front() {
        for &next in &graph.adjacency[index] {
            if selected.len() < 80 && selected.insert(next) {
                pending.push_back(next);
            }
        }
    }
    let members: BTreeSet<_> = selected.iter().map(|i| blocks[*i].id.clone()).collect();
    let mut boxes: Vec<_> = selected
        .iter()
        .map(|i| {
            let n = &graph.nodes[*i];
            (n.x, n.y, n.width, HEIGHT)
        })
        .collect();
    let edges: BTreeSet<_> = selected
        .iter()
        .flat_map(|i| graph.incident[*i].iter().copied())
        .collect();
    let edge_indices: Vec<_> = edges
        .into_iter()
        .filter(|index| {
            let link = &graph.links[*index];
            members.contains(&link.origin) && members.contains(&link.destination)
        })
        .collect();
    boxes.extend(
        edge_indices
            .iter()
            .filter_map(|index| graph.routes[*index].as_ref().map(|route| route.bounds)),
    );
    let (node_indices, edge_indices) = if graph.ids.len() == blocks.len() {
        (selected.into_iter().collect(), edge_indices)
    } else {
        // Malformed duplicate beat IDs retain the original membership-based
        // rendering, while bounds still describe the BFS neighbourhood above.
        (
            blocks
                .iter()
                .enumerate()
                .filter_map(|(index, block)| members.contains(&block.id).then_some(index))
                .collect(),
            graph
                .links
                .iter()
                .enumerate()
                .filter_map(|(index, link)| {
                    (members.contains(&link.origin) && members.contains(&link.destination))
                        .then_some(index)
                })
                .collect(),
        )
    };
    let left = boxes.iter().map(|b| b.0).fold(f32::INFINITY, f32::min);
    let top = boxes.iter().map(|b| b.1).fold(f32::INFINITY, f32::min);
    let right = boxes
        .iter()
        .map(|b| b.0 + b.2)
        .fold(f32::NEG_INFINITY, f32::max);
    let bottom = boxes
        .iter()
        .map(|b| b.1 + b.3)
        .fold(f32::NEG_INFINITY, f32::max);
    Some(Scope {
        members,
        node_indices,
        edge_indices,
        bounds: (left, top, right - left, bottom - top),
    })
}

#[cfg(test)]
mod tests;
