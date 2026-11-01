//! Spatial selection and reveal rules are shared by native map adapters.
use super::{
    camera::{Camera, Framing},
    layout::{HEIGHT, Node},
};

pub fn nearest(nodes: &[Node], current: usize, direction: (f32, f32)) -> Option<usize> {
    let from = nodes.get(current)?;
    nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != current)
        .filter_map(|(i, node)| {
            let dx = node.x + node.width / 2. - from.x - from.width / 2.;
            let dy = node.y - from.y;
            let forward = dx * direction.0 + dy * direction.1;
            let sideways = (dx * direction.1 - dy * direction.0).abs();
            (forward > 1.).then_some((i, forward.hypot(sideways)))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

pub fn reveal(camera: &mut Camera, node: &Node, size: (f32, f32)) {
    let left = camera.x + node.x * camera.zoom;
    let top = camera.y + node.y * camera.zoom;
    let right = left + node.width * camera.zoom;
    let bottom = top + HEIGHT * camera.zoom;
    let dx = if left < 16. {
        16. - left
    } else if right > size.0 - 16. {
        size.0 - 16. - right
    } else {
        0.
    };
    let dy = if top < 16. {
        16. - top
    } else if bottom > size.1 - 16. {
        size.1 - 16. - bottom
    } else {
        0.
    };
    if dx != 0. || dy != 0. {
        camera.x += dx;
        camera.y += dy;
        camera.framing = Framing::Free;
    }
}

#[cfg(test)]
mod tests;
