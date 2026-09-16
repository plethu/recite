//! Spatial navigation and explicit connection navigation share one selected beat.
use super::{
    camera::{Camera, Framing},
    layout::{HEIGHT, Node},
};
use crate::editing::Writer;
use freya::prelude::*;
use recite_writer_model::ScriptBlock;

pub(super) fn nearest(nodes: &[Node], current: usize, direction: (f32, f32)) -> Option<usize> {
    let from = &nodes[current];
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

pub(super) fn reveal(camera: &mut Camera, node: &Node, size: (f32, f32)) {
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

pub(super) fn key(
    event: Event<KeyboardEventData>,
    mut writer: Writer,
    blocks: &[ScriptBlock],
    nodes: &[Node],
) {
    if !event.modifiers.is_empty() {
        return;
    }
    let vim = writer.preferences.peek().config.ui.keymap == recite_config::Keymap::Vim;
    let direction = match event.code {
        Code::ArrowLeft => Some((-1., 0.)),
        Code::ArrowRight => Some((1., 0.)),
        Code::ArrowUp => Some((0., -1.)),
        Code::ArrowDown => Some((0., 1.)),
        Code::KeyH if vim => Some((-1., 0.)),
        Code::KeyL if vim => Some((1., 0.)),
        Code::KeyK if vim => Some((0., -1.)),
        Code::KeyJ if vim => Some((0., 1.)),
        _ => None,
    };
    let current = writer
        .selection
        .peek()
        .as_ref()
        .and_then(|id| blocks.iter().position(|b| &b.id == id))
        .unwrap_or(0);
    if let Some(direction) = direction {
        if let Some(next) = nearest(nodes, current, direction) {
            writer.selection.set(Some(blocks[next].id.clone()));
        }
    } else if event.code == Code::Enter || (vim && event.code == Code::KeyI) {
        if let Some(block) = blocks.get(current) {
            writer.inspect(&block.id);
        }
    } else if event.code == Code::Slash {
        writer.search_focus.request_focus();
    } else {
        return;
    }
    event.stop_propagation();
    event.prevent_default();
}

#[cfg(test)]
mod tests;
