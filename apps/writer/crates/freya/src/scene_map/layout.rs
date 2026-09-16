//! Deterministic breadth-first placement; return edges never duplicate a node.
use recite_writer_model::{SceneLink, ScriptBlock};
use std::collections::VecDeque;

pub(super) const WIDTH: f32 = 1200.;
pub(super) const HEIGHT: f32 = 156.;
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Node {
    pub x: f32,
    pub y: f32,
    pub width: f32,
}

pub(super) fn layout(blocks: &[ScriptBlock], links: &[SceneLink], width: f32) -> Vec<Node> {
    let columns = ((width - 32.) / 240.).floor().clamp(2., 4.) as usize;
    let ids: std::collections::BTreeMap<_, _> = blocks
        .iter()
        .enumerate()
        .map(|(i, b)| (b.id.as_str(), i))
        .collect();
    let mut adjacency = vec![Vec::new(); blocks.len()];
    for link in links {
        if let (Some(&from), Some(&to)) = (
            ids.get(link.origin.as_str()),
            ids.get(link.destination.as_str()),
        ) {
            adjacency[from].push(to);
        }
    }
    let mut depths = vec![None; blocks.len()];
    let mut next_depth = 0;
    for root in (0..blocks.len())
        .filter(|i| blocks[*i].is_default)
        .chain((0..blocks.len()).filter(|i| !blocks[*i].is_default))
    {
        if depths[root].is_some() {
            continue;
        }
        depths[root] = Some(next_depth);
        let mut pending = VecDeque::from([root]);
        while let Some(index) = pending.pop_front() {
            let depth = depths[index].unwrap_or(0);
            next_depth = next_depth.max(depth + 1);
            for &next in &adjacency[index] {
                if depths[next].is_none() {
                    depths[next] = Some(depth + 1);
                    pending.push_back(next);
                }
            }
        }
    }
    let mut counts = vec![0usize; next_depth];
    for depth in depths.iter().flatten() {
        counts[*depth] += 1;
    }
    let mut rows = vec![0; next_depth];
    for depth in 1..next_depth {
        rows[depth] = rows[depth - 1] + counts[depth - 1].div_ceil(columns);
    }
    let mut used = vec![0usize; next_depth];
    depths
        .iter()
        .map(|depth| {
            let depth = depth.unwrap_or(0);
            let column = used[depth];
            used[depth] += 1;
            let slot = (width - 32.) / counts[depth].clamp(1, columns) as f32;
            Node {
                x: 8. + (column % columns) as f32 * slot + (slot - (slot - 8.).min(320.)) / 2.,
                y: 12. + (rows[depth] + column / columns) as f32 * (HEIGHT + 100.),
                width: (slot - 8.).min(320.),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
