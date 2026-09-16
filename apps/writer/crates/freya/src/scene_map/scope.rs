//! Large scenes open around the selected beat; omitted content stays explicit.
use super::{layout::HEIGHT, topology::Topology};
use recite_writer_model::ScriptBlock;
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, PartialEq)]
pub(super) struct Scope {
    pub members: BTreeSet<String>,
    pub bounds: (f32, f32, f32, f32),
}
pub(super) fn nearby(
    blocks: &[ScriptBlock],
    graph: &Topology,
    selected: Option<&str>,
) -> Option<Scope> {
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
    boxes.extend(edges.into_iter().filter_map(|index| {
        let link = &graph.links[index];
        if members.contains(&link.origin) && members.contains(&link.destination) {
            graph.routes[index].as_ref().map(|route| route.bounds)
        } else {
            None
        }
    }));
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
        bounds: (left, top, right - left, bottom - top),
    })
}

#[cfg(test)]
mod tests;
