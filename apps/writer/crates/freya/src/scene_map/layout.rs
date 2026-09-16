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
    let mut depths: Vec<Option<usize>> = vec![None; blocks.len()];
    for root in (0..blocks.len())
        .filter(|i| blocks[*i].is_default)
        .chain((0..blocks.len()).filter(|i| !blocks[*i].is_default))
    {
        if depths[root].is_some() {
            continue;
        }
        let first_depth = depths.iter().flatten().max().map_or(0, |depth| depth + 1);
        depths[root] = Some(first_depth);
        let mut pending = VecDeque::from([root]);
        while let Some(index) = pending.pop_front() {
            for link in links.iter().filter(|link| link.origin == blocks[index].id) {
                if let Some(next) = blocks.iter().position(|block| block.id == link.destination)
                    && depths[next].is_none()
                {
                    depths[next] = depths[index].map(|depth| depth + 1);
                    pending.push_back(next);
                }
            }
        }
    }
    depths
        .iter()
        .enumerate()
        .map(|(index, depth)| {
            let count = depths.iter().filter(|other| *other == depth).count();
            let column = depths[..index]
                .iter()
                .filter(|other| *other == depth)
                .count();
            let slot = (width - 32.) / count.clamp(1, columns) as f32;
            let preceding_rows: usize = (0..depth.unwrap_or(0))
                .map(|level| {
                    depths
                        .iter()
                        .filter(|d| **d == Some(level))
                        .count()
                        .div_ceil(columns)
                })
                .sum();
            Node {
                x: 8. + (column % columns) as f32 * slot + (slot - (slot - 8.).min(320.)) / 2.,
                y: 12. + (preceding_rows + column / columns) as f32 * (HEIGHT + 100.),
                width: (slot - 8.).min(320.),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
