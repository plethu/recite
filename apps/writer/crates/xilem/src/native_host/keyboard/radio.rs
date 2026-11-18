//! Arrow keys move between the choices of a radio group and choose the one they reach, as in the
//! WAI-ARIA radio group pattern; Tab treats the group as one stop (see `tab`). Left and Right do
//! the same in a tab list, whose unselected tabs are not Tab stops (roving focus). Masonry handles
//! neither, so the native host does, after the key reaches it.
use super::*;
use xilem::masonry::{
    accesskit::{Action, ActionRequest, Role, TreeId},
    widgets::RadioButton,
};

#[derive(Default)]
pub(super) struct Arrows {
    pending: Option<(WidgetId, bool)>,
}
impl Arrows {
    /// Claim an arrow key pressed on a radio choice or a tab; `true` when the key is consumed.
    pub fn request(&mut self, root: &RenderRoot, key: &keyboard::Key) -> bool {
        let Some(focused) = root.focused_widget() else {
            return false;
        };
        let Some(widget) = root.get_widget(focused) else {
            return false;
        };
        let radio = widget.downcast::<RadioButton>().is_some();
        let tab = widget.accessibility_role() == Role::Tab;
        let backward = match key {
            keyboard::Key::Named(NamedKey::ArrowLeft) => true,
            keyboard::Key::Named(NamedKey::ArrowRight) => false,
            // A tab list runs along the row; only a radio group takes Up and Down.
            keyboard::Key::Named(NamedKey::ArrowUp) if radio => true,
            keyboard::Key::Named(NamedKey::ArrowDown) if radio => false,
            _ => return false,
        };
        let claimed = radio || tab;
        if claimed {
            self.pending = Some((focused, backward));
        }
        claimed
    }
    pub fn synchronize(&mut self, root: &mut RenderRoot) {
        if let Some((from, backward)) = self.pending.take() {
            step(root, from, backward);
        }
    }
}

/// Focus and choose the next enabled choice or tab after `from`, wrapping at either end.
pub(super) fn step(root: &mut RenderRoot, from: WidgetId, backward: bool) {
    let layer = root.get_layer_root(0);
    let choices = if root
        .get_widget(from)
        .is_some_and(|widget| widget.accessibility_role() == Role::Tab)
    {
        super::tab::tabs(layer, from)
    } else {
        super::tab::choices(layer, from)
    };
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
