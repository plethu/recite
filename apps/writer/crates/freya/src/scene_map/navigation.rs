//! Spatial navigation and explicit connection navigation share one selected beat.
use super::layout::Node;
use crate::editing::Writer;
use freya::prelude::*;
use recite_writer_model::ScriptBlock;

pub(super) use recite_writer_session::map::navigation::{nearest, reveal};

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
        if vim {
            return;
        }
        writer.search_focus.request_focus();
    } else {
        return;
    }
    event.stop_propagation();
    event.prevent_default();
}
