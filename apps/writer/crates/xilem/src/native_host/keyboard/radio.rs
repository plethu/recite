//! Arrow keys move between the choices of a radio group and choose the one they reach, as in the
//! WAI-ARIA radio group pattern; Tab treats the group as one stop (see `tab`). Masonry's radio
//! buttons handle neither, so the native host does, after the key reaches it.
use super::*;
use xilem::masonry::{
    accesskit::{Action, ActionRequest, TreeId},
    widgets::RadioButton,
};

#[derive(Default)]
pub(super) struct Arrows {
    pending: Option<(WidgetId, bool)>,
}
impl Arrows {
    /// Claim an arrow key pressed on a radio choice; `true` when the key is consumed.
    pub fn request(&mut self, root: &RenderRoot, key: &keyboard::Key) -> bool {
        let backward = match key {
            keyboard::Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowUp) => true,
            keyboard::Key::Named(NamedKey::ArrowRight | NamedKey::ArrowDown) => false,
            _ => return false,
        };
        let Some(focused) = root.focused_widget() else {
            return false;
        };
        let radio = root
            .get_widget(focused)
            .is_some_and(|widget| widget.downcast::<RadioButton>().is_some());
        if radio {
            self.pending = Some((focused, backward));
        }
        radio
    }
    pub fn synchronize(&mut self, root: &mut RenderRoot) {
        if let Some((from, backward)) = self.pending.take() {
            step(root, from, backward);
        }
    }
}

/// Focus and choose the next enabled choice after `from`, wrapping at either end.
pub(super) fn step(root: &mut RenderRoot, from: WidgetId, backward: bool) {
    let choices = super::tab::choices(root.get_layer_root(0), from);
    let Some(index) = choices.iter().position(|id| *id == from) else {
        return;
    };
    let count = choices.len();
    let next = choices[if backward {
        (index + count - 1) % count
    } else {
        (index + 1) % count
    }];
    if next == from {
        return;
    }
    root.focus_on(Some(next));
    root.handle_access_event(ActionRequest {
        action: Action::Click,
        target_tree: TreeId::ROOT,
        target_node: next.to_raw().into(),
        data: None,
    });
}

#[cfg(test)]
mod tests;
